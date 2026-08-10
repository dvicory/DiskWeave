use blake3::Hash;
use dwv_codec::{Geometry, compute_parity};
use dwv_core::{
    ArrayId, AssignmentGeneration, AssignmentInstanceId, BlockOp, BlockRequest, BufferToken,
    ByteRange, CodingPosition, CodingProfile, DurabilityIntent, FenceDomain, FrontendId,
    MemberRole, OrderingIntent, ProtectedGeometry, RequestId, SlotId, SubmissionSequence,
    TopologyAssignment, TopologyEpoch, TopologySnapshot,
};
use dwv_format::{
    ByteRange as EnvelopeRange, COPY_BYTES, DIRTY_REGION_BYTES, EnvelopeRecord, MigrationState,
    Profile, ProfileConfig, SessionState, assess_copies, compare_profiles, encode_copy,
};
use dwv_recovery::{
    ChecksumSetGeneration, MemoryRecoveryStore, RebuildId, RebuildLifecycle, RebuildState,
    RebuildTargetIdentity, RecoveryGeneration, RecoveryManifest, RecoveryMutation,
    RecoveryStateStore, TopologySnapshot as RecoveryTopologySnapshot,
};
use dwv_recovery_sqlite::SqliteRecoveryStore;
use dwv_service::{
    FileRebuildStore, HealthyPortableService, MemberBinding, ServiceConfig,
    commit_verified_rebuild_chunk, commit_verified_rebuild_completion,
};
use dwv_sim::{Schedule, ScheduleStep, Simulator, SimulatorConfig};
use dwv_store::StoreId;
use dwv_store::{
    ChildOperationId, CompletionDisposition, OperationSlotToken, StoreWriteWatermark, WriteIntent,
};
use dwv_store_file::{FileStore, FileStoreConfig, FileSyncMode};
use dwv_trace::{
    ChecksumOutcome, PayloadPattern, RepairDecision, Trace, TraceError, TraceEventKind,
    TraceFixture, TraceModelState, TraceOutcome, materialize_pattern,
};
use dwv_verify::{
    ChecksumEvidence, DigestEvidence, RebuildBinding, RebuildTarget, ReconstructionRangeEvidence,
    ReconstructionSourceState, RepairTarget, ScanConfig, ScrubContext, VerificationStore,
    apply_scrub, authorize_known_erasure, execute_rebuild_chunk, plan_rebuild_ranges, plan_scrub,
    read_known_erasure, verify_complete_rebuild, verify_exhaustive,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

const CONTRACT: &str = "dwv.cli.v0";
const MANIFEST_FILE: &str = "fixture.json";
const MARKER_FILE: &str = ".dwv-demo";
const PLAN_FILE: &str = "rebuild-plan.json";
const SCRUB_PLAN_FILE: &str = "scrub-plan.json";
const BLOCK: u32 = 512;
const DEFAULT_SIZE: u64 = 16 * 1024;
const MAX_SIZE: u64 = 64 * 1024 * 1024;
const EPOCH: u64 = 16;
const CHUNK: u64 = 4096;
const ARRAY_ID: [u8; 16] = [0xd0; 16];
const REBUILD_ID: RebuildId = RebuildId::from_bytes([0xd1; 16]);
const MISSING_SLOT: [u8; 16] = [1; 16];
const REPLACEMENT_ASSIGNMENT: [u8; 16] = [0x21; 16];
const TRACE_MINIMIZED_FILE: &str = "trace-minimized.json";
const REPLACEMENT_STORE: StoreId = StoreId(20);

#[derive(Debug)]
pub struct DemoError {
    pub code: u8,
    pub class: &'static str,
    pub message: String,
}

impl DemoError {
    fn new(code: u8, class: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            class,
            message: message.into(),
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(2, "usage", message)
    }

    fn blocked(message: impl Into<String>) -> Self {
        Self::new(3, "blocked", message)
    }

    fn refused(message: impl Into<String>) -> Self {
        Self::new(4, "refused", message)
    }

    fn failed(message: impl Into<String>) -> Self {
        Self::new(5, "operation-failed", message)
    }
}

impl fmt::Display for DemoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.class, self.message)
    }
}

impl std::error::Error for DemoError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FixtureManifest {
    schema: u32,
    contract: String,
    protected_length: u64,
    logical_block_size: u32,
    topology_epoch: u64,
    array_id: [u8; 16],
    data_files: [String; 2],
    data_identities: [[u8; 16]; 2],
    envelope_files: [String; 2],
    envelope_profile: u8,
    parity_file: String,
    parity_identity: [u8; 16],
    reference_file: String,
    reference_data1_file: String,
    reference_identity: [u8; 16],
    replacement_file: String,
    replacement_identity: [u8; 16],
    recovery_file: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ScrubPlanFile {
    schema: u32,
    contract: String,
    topology_epoch: u64,
    recovery_generation: u64,
    checksum_set_generation: u64,
    data_identities: Vec<[u8; 16]>,
    parity_identity: [u8; 16],
    replacement_identity: [u8; 16],
    range_offset: u64,
    range_length: u64,
    target: ScrubTargetFile,
    confirmation: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum ScrubTargetFile {
    Data { slot: usize },
    Parity,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RebuildPlan {
    schema: u32,
    contract: String,
    rebuild_id: [u8; 16],
    missing_slot: [u8; 16],
    replacement_store: u64,
    replacement_file: String,
    replacement_identity: [u8; 16],
    topology_epoch: u64,
    source_recovery_generation: u64,
    protected_length: u64,
    chunk_size: u64,
    confirmation: String,
}

pub fn init(root: &Path, requested_size: Option<u64>) -> Result<Value, DemoError> {
    let size = requested_size.unwrap_or(DEFAULT_SIZE);
    validate_size(size)?;
    if root.exists() {
        let empty = root.is_dir()
            && fs::read_dir(root)
                .map_err(|error| DemoError::refused(error.to_string()))?
                .next()
                .is_none();
        if !empty {
            return Err(DemoError::refused(format!(
                "fixture root must be a missing or empty directory: {}",
                root.display()
            )));
        }
    } else {
        fs::create_dir_all(root).map_err(|error| DemoError::failed(error.to_string()))?;
    }
    fs::write(root.join(MARKER_FILE), b"diskweave disposable macOS demo\n")
        .map_err(|error| DemoError::failed(error.to_string()))?;

    let data0 = deterministic_bytes(size, 7, 13);
    let data1 = deterministic_bytes(size, 11, 29);
    let geometry = codec_geometry(size)?;
    let parity = compute_parity(&geometry, &[&data0, &data1])
        .map_err(|error| DemoError::failed(format!("parity initialization failed: {error}")))?;
    let mut manifest = FixtureManifest {
        schema: 1,
        contract: CONTRACT.to_owned(),
        protected_length: size,
        logical_block_size: BLOCK,
        topology_epoch: EPOCH,
        array_id: ARRAY_ID,
        data_files: ["data0.raw".to_owned(), "data1.raw".to_owned()],
        data_identities: [[0; 16]; 2],
        envelope_files: [
            "parity-envelope-a.bin".to_owned(),
            "parity-envelope-b.bin".to_owned(),
        ],
        envelope_profile: 1,
        parity_file: "parity.raw".to_owned(),
        parity_identity: [0; 16],
        reference_file: "reference.raw".to_owned(),
        reference_data1_file: "reference-data1.raw".to_owned(),
        reference_identity: [0; 16],
        replacement_file: "replacement.raw".to_owned(),
        replacement_identity: [0; 16],
        recovery_file: "recovery.sqlite3".to_owned(),
    };

    let envelope_layout = ProfileConfig::new(
        Profile::RedundantEnvelope,
        envelope_extent_length(size)?,
        size,
    )
    .layout()
    .map_err(|error| DemoError::failed(error.to_string()))?;
    let mut envelope = EnvelopeRecord {
        profile: Profile::RedundantEnvelope,
        compatible_features: 0,
        required_features: 0,
        array_id: ARRAY_ID,
        parity_device_id: [0x12; 16],
        parity_role: 0,
        codec_profile: 1,
        payload: EnvelopeRange::new(
            envelope_layout.payload.offset,
            envelope_layout.payload.length,
        )
        .ok_or_else(|| DemoError::failed("envelope payload range overflow"))?,
        logical_length: size,
        logical_block_size: BLOCK,
        stripe_width: 2,
        topology_generation: EPOCH,
        session_generation: 1,
        last_global_clean_checkpoint: 0,
        last_full_verified_checkpoint: None,
        session_state: SessionState::Closed,
        migration_state: MigrationState::Stable,
        copy_generation: 1,
        copy_slot: 0,
        bitmap_region_bytes: 0,
        bitmap: Vec::new(),
    };
    let envelope_a =
        encode_copy(&envelope).map_err(|error| DemoError::failed(error.to_string()))?;
    envelope.copy_slot = 1;
    let envelope_b =
        encode_copy(&envelope).map_err(|error| DemoError::failed(error.to_string()))?;

    fs::write(root.join(&manifest.data_files[0]), &data0)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    fs::write(root.join(&manifest.data_files[1]), &data1)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    fs::write(root.join(&manifest.parity_file), &parity)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    fs::write(root.join(&manifest.envelope_files[0]), &envelope_a)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    fs::write(root.join(&manifest.envelope_files[1]), &envelope_b)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    fs::write(root.join(&manifest.reference_file), &data0)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    fs::write(root.join(&manifest.reference_data1_file), &data1)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    create_empty_replacement(root, &manifest)?;
    manifest.data_identities = [
        file_identity(&open_store(
            root,
            &manifest.data_files[0],
            &manifest,
            StoreId(10),
            false,
        )?),
        file_identity(&open_store(
            root,
            &manifest.data_files[1],
            &manifest,
            StoreId(11),
            false,
        )?),
    ];
    manifest.parity_identity = file_identity(&open_store(
        root,
        &manifest.parity_file,
        &manifest,
        StoreId(12),
        false,
    )?);
    manifest.reference_identity = file_identity(&open_store(
        root,
        &manifest.reference_file,
        &manifest,
        StoreId(30),
        false,
    )?);
    manifest.replacement_identity = file_identity(&open_store(
        root,
        &manifest.replacement_file,
        &manifest,
        REPLACEMENT_STORE,
        false,
    )?);

    let recovery_manifest = initial_recovery_manifest(&manifest)?;
    let recovery_path = root.join(&manifest.recovery_file);
    let recovery = SqliteRecoveryStore::create_new(&recovery_path, recovery_manifest)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    drop(recovery);
    write_manifest(root, &manifest)?;

    Ok(json!({
        "fixture": "initialized",
        "contract": CONTRACT,
        "protected_length": size,
        "logical_block_size": BLOCK,
        "data_files": manifest.data_files,
        "envelope_files": manifest.envelope_files,
        "parity_file": manifest.parity_file,
        "envelope_profile": manifest.envelope_profile,
        "reference_file": manifest.reference_file,
        "replacement_file": manifest.replacement_file,
        "recovery_file": manifest.recovery_file,
    }))
}

pub fn run(root: &Path) -> Result<Value, DemoError> {
    let (_, manifest) = load_fixture(root)?;
    let healthy = healthy_cycle(root, &manifest)?;
    let rebuild = rebuild_fixture(root, &manifest)?;
    Ok(json!({
        "workflow": "complete",
        "healthy": healthy,
        "rebuild": rebuild,
        "claim_boundary": [
            "portable file-backed macOS behavior",
            "single-XOR known-erasure reconstruction",
            "offline separate-target rebuild"
        ],
        "unsupported": [
            "live FSKit/DiskImages bridge",
            "Linux frontend",
            "physical power-loss durability",
            "P/Q and degraded writes"
        ]
    }))
}

pub fn status(root: &Path) -> Result<Value, DemoError> {
    let (_, manifest) = load_fixture(root)?;
    status_value(root, &manifest)
}

pub fn inspect(root: &Path) -> Result<Value, DemoError> {
    let (_, manifest) = load_fixture(root)?;
    let mut result = status_value(root, &manifest)?;
    let envelope = inspect_envelope(root, &manifest)?;
    result["inspection"] = json!({
        "manifest_schema": manifest.schema,
        "contract": manifest.contract,
        "data_members": manifest.data_files,
        "parity_member": manifest.parity_file,
        "recovery_state": manifest.recovery_file,
        "data_payload_metadata": "none-required",
        "parity_envelope": envelope,
        "identity_evidence": {
            "data": [
                hex_bytes(&manifest.data_identities[0]),
                hex_bytes(&manifest.data_identities[1])
            ],
            "parity": hex_bytes(&manifest.parity_identity),
            "reference": hex_bytes(&manifest.reference_identity),
            "replacement": hex_bytes(&manifest.replacement_identity)
        },
    });
    Ok(result)
}

fn inspect_envelope(root: &Path, manifest: &FixtureManifest) -> Result<Value, DemoError> {
    if manifest.envelope_profile != 1 {
        return Err(DemoError::blocked(
            "fixture envelope profile is unsupported",
        ));
    }
    let first = fs::read(root.join(&manifest.envelope_files[0]))
        .map_err(|error| DemoError::blocked(error.to_string()))?;
    let second = fs::read(root.join(&manifest.envelope_files[1]))
        .map_err(|error| DemoError::blocked(error.to_string()))?;
    let assessment = assess_copies(Some(&first), Some(&second), Some(manifest.topology_epoch));
    let extent_length = envelope_extent_length(manifest.protected_length)?;
    let layout = ProfileConfig::new(
        Profile::RedundantEnvelope,
        extent_length,
        manifest.protected_length,
    )
    .layout()
    .map_err(|error| DemoError::failed(error.to_string()))?;
    let profile_comparison: Vec<Value> = compare_profiles(extent_length, manifest.protected_length)
        .into_iter()
        .map(|evaluation| match evaluation.layout {
            Ok(layout) => json!({
                "profile": format!("{:?}", evaluation.profile),
                "accepted": true,
                "metadata_bytes": layout.metadata_bytes,
                "payload_offset": layout.payload.offset,
                "payload_length": layout.payload.length,
            }),
            Err(error) => json!({
                "profile": format!("{:?}", evaluation.profile),
                "accepted": false,
                "error": error.to_string(),
            }),
        })
        .collect();
    Ok(json!({
        "profile_comparison": profile_comparison,
        "profile": format!("{:?}", Profile::RedundantEnvelope),
        "copy_lengths": [first.len(), second.len()],
        "metadata_bytes": layout.metadata_bytes,
        "payload_offset": layout.payload.offset,
        "payload_length": layout.payload.length,
        "assessment": format!("{:?}", assessment.state),
        "evidence_strength": format!("{:?}", assessment.strength),
        "reason": format!("{:?}", assessment.reason),
        "clean_recovery_authorized": assessment.clean_recovery_authorized,
    }))
}

fn envelope_extent_length(protected_length: u64) -> Result<u64, DemoError> {
    let regions = protected_length
        .checked_add(DIRTY_REGION_BYTES - 1)
        .ok_or_else(|| DemoError::failed("envelope bitmap region overflow"))?
        / DIRTY_REGION_BYTES;
    let bitmap_bytes = regions
        .checked_add(7)
        .ok_or_else(|| DemoError::failed("envelope bitmap length overflow"))?
        / 8;
    protected_length
        .checked_add((COPY_BYTES as u64) * 2)
        .and_then(|length| length.checked_add(bitmap_bytes))
        .ok_or_else(|| DemoError::failed("envelope extent length overflow"))
}
pub fn capabilities(root: &Path) -> Result<Value, DemoError> {
    let (_, manifest) = load_fixture(root)?;
    let mut entries = Vec::new();
    for (relative, store_id) in [
        (&manifest.data_files[0], StoreId(10)),
        (&manifest.data_files[1], StoreId(11)),
        (&manifest.parity_file, StoreId(12)),
        (&manifest.replacement_file, REPLACEMENT_STORE),
    ] {
        let store = open_store(root, relative, &manifest, store_id, false)?;
        let report = store.capabilities_report();
        entries.push(json!({
            "path": relative,
            "identity": hex_bytes(&file_identity(&store)),
            "capabilities": format!("{:?}", report.capabilities),
        }));
    }
    Ok(json!({
        "capability_boundary": "file-backed portable demo",
        "members": entries,
    }))
}

pub fn verify(root: &Path) -> Result<Value, DemoError> {
    let (_, manifest) = load_fixture(root)?;
    verify_value(root, &manifest)
}

pub fn plan(root: &Path, plan_path: &Path) -> Result<Value, DemoError> {
    let (_, manifest) = load_fixture(root)?;
    let recovery = open_recovery(root, &manifest)?;
    let snapshot = recovery
        .load_assembly_snapshot()
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let target = open_rebuild_target(root, &manifest, false)?;
    let plan = RebuildPlan {
        schema: 1,
        contract: CONTRACT.to_owned(),
        rebuild_id: REBUILD_ID.as_bytes(),
        missing_slot: MISSING_SLOT,
        replacement_store: REPLACEMENT_STORE.0,
        replacement_file: manifest.replacement_file.clone(),
        replacement_identity: target.identity().0,
        topology_epoch: snapshot.topology_epoch.0,
        source_recovery_generation: snapshot.generation.0,
        protected_length: manifest.protected_length,
        chunk_size: CHUNK.min(manifest.protected_length),
        confirmation: String::new(),
    };
    drop(target);
    drop(recovery);
    let plan = with_confirmation(plan)?;
    let plan_path = plan_path
        .to_str()
        .ok_or_else(|| DemoError::usage("plan path must be valid UTF-8"))?;
    let plan_path = owned_path(root, plan_path, false)?;
    write_plan(&plan_path, &plan)?;
    serde_json::to_value(plan).map_err(|error| DemoError::failed(error.to_string()))
}

pub fn execute(
    root: &Path,
    plan_path: &Path,
    confirmation: &str,
    stop_after: Option<u64>,
) -> Result<Value, DemoError> {
    let (_, manifest) = load_fixture(root)?;
    let plan_relative = plan_path
        .to_str()
        .ok_or_else(|| DemoError::usage("plan path must be valid UTF-8"))?;
    let plan_path = owned_path(root, plan_relative, true)?;
    let plan: RebuildPlan = serde_json::from_slice(
        &fs::read(&plan_path).map_err(|error| DemoError::failed(error.to_string()))?,
    )
    .map_err(|error| DemoError::refused(format!("invalid rebuild plan: {error}")))?;
    let expected = with_confirmation(RebuildPlan {
        confirmation: String::new(),
        ..plan.clone()
    })?;
    if confirmation != expected.confirmation || plan.confirmation != expected.confirmation {
        return Err(DemoError::refused(
            "confirmation does not match the canonical rebuild plan",
        ));
    }
    validate_plan(&manifest, &plan)?;
    let result = rebuild_fixture_with_plan(root, &manifest, &plan, stop_after)?;
    Ok(json!({
        "plan": "accepted",
        "workflow": result,
    }))
}

fn healthy_request(
    request_id: u64,
    slot_id: SlotId,
    op: BlockOp,
    range: ByteRange,
    durability: DurabilityIntent,
) -> BlockRequest {
    BlockRequest::new(
        RequestId(request_id),
        FrontendId(1),
        slot_id,
        TopologyEpoch(EPOCH),
        op,
        range,
        matches!(op, BlockOp::Read | BlockOp::Write).then_some(BufferToken::new(1, 1)),
        OrderingIntent {
            submission_sequence: SubmissionSequence(request_id),
            preflush: false,
            fence_domain: FenceDomain(1),
        },
        durability,
    )
}

fn healthy_cycle(root: &Path, manifest: &FixtureManifest) -> Result<Value, DemoError> {
    let topology = core_topology(manifest)?;
    let epoch = topology.topology_epoch();
    let recovery = open_recovery(root, manifest)?;
    let member = |slot_id, store_id, store| {
        MemberBinding::new(
            topology
                .assignment_for_slot(slot_id)
                .expect("demo topology contains the requested assignment"),
            epoch,
            store_id,
            store,
        )
    };
    let members = vec![
        member(
            SlotId::from_bytes([1; 16]),
            StoreId(10),
            open_store(root, &manifest.data_files[0], manifest, StoreId(10), true)?,
        ),
        member(
            SlotId::from_bytes([2; 16]),
            StoreId(11),
            open_store(root, &manifest.data_files[1], manifest, StoreId(11), true)?,
        ),
        member(
            SlotId::from_bytes([3; 16]),
            StoreId(12),
            open_store(root, &manifest.parity_file, manifest, StoreId(12), true)?,
        ),
    ];
    let config = ServiceConfig {
        maximum_transfer: Some(manifest.protected_length),
        ..ServiceConfig::default()
    };
    let mut service = HealthyPortableService::open(topology, members, recovery, config)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let range = ByteRange::new(0, u64::from(BLOCK))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let bytes = vec![0xa5; usize::try_from(range.length).unwrap()];
    service
        .write(
            healthy_request(
                101,
                SlotId::from_bytes([2; 16]),
                BlockOp::Write,
                range,
                DurabilityIntent::Ordinary,
            ),
            &bytes,
        )
        .map_err(|error| DemoError::failed(error.to_string()))?;
    service
        .flush(healthy_request(
            102,
            SlotId::from_bytes([1; 16]),
            BlockOp::Flush,
            ByteRange::empty(),
            DurabilityIntent::ExplicitFlush,
        ))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let (read_back, _) = service
        .read(healthy_request(
            103,
            SlotId::from_bytes([2; 16]),
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        ))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    if read_back != bytes {
        return Err(DemoError::failed("healthy read did not match the write"));
    }
    let generation = service
        .recovery()
        .load_assembly_snapshot()
        .map_err(|error| DemoError::failed(error.to_string()))?
        .generation
        .0;
    drop(service);

    let mut reopened = open_store(root, &manifest.data_files[1], manifest, StoreId(11), false)?;
    let reopened_bytes = reopened
        .read_bytes(range)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    drop(reopened);
    if reopened_bytes != bytes {
        return Err(DemoError::failed(
            "reopened member did not preserve the write",
        ));
    }
    Ok(json!({
        "write": "success",
        "flush": "durable-host-file-fence",
        "read": "matched",
        "reopened": "matched",
        "recovery_generation": generation,
    }))
}

fn rebuild_fixture(root: &Path, manifest: &FixtureManifest) -> Result<Value, DemoError> {
    let plan = plan(root, Path::new(PLAN_FILE))?;
    let plan: RebuildPlan =
        serde_json::from_value(plan).map_err(|error| DemoError::failed(error.to_string()))?;
    rebuild_fixture_with_plan(root, manifest, &plan, None)
}

fn rebuild_fixture_with_plan(
    root: &Path,
    manifest: &FixtureManifest,
    plan: &RebuildPlan,
    stop_after: Option<u64>,
) -> Result<Value, DemoError> {
    let parity_before = fs::read(root.join(&manifest.parity_file))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let reference = fs::read(root.join(&manifest.reference_file))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let topology = core_topology(manifest)?;
    let recovery_topology = recovery_topology(&topology)?;
    let geometry = codec_geometry(manifest.protected_length)?;
    let mut recovery = open_recovery(root, manifest)?;
    let mut target = open_rebuild_target(root, manifest, true)?;
    if target.identity().0 != plan.replacement_identity {
        return Err(DemoError::refused(
            "replacement identity differs from the plan",
        ));
    }
    let before = recovery
        .load_assembly_snapshot()
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let existing = before
        .rebuilds
        .iter()
        .find(|rebuild| rebuild.id() == REBUILD_ID)
        .cloned();
    let cursor = if let Some(rebuild) = existing.as_ref() {
        if rebuild.source_recovery_generation().0 != plan.source_recovery_generation
            || rebuild.replacement_identity().as_bytes() != plan.replacement_identity
            || rebuild.replacement_store() != REPLACEMENT_STORE
        {
            return Err(DemoError::refused(
                "persisted rebuild does not match the plan",
            ));
        }
        if rebuild.lifecycle() == RebuildLifecycle::Verified {
            return finish_rebuild_report(
                root,
                manifest,
                reference,
                parity_before,
                "already-verified",
            );
        }
        dwv_service::validate_rebuild_resume(&recovery, REBUILD_ID, &mut target)
            .map_err(|error| DemoError::refused(error.to_string()))?
            .offset()
    } else {
        if before.generation.0 != plan.source_recovery_generation
            || before.topology_epoch.0 != plan.topology_epoch
        {
            return Err(DemoError::refused(
                "recovery state is stale relative to the plan",
            ));
        }
        let rebuild = RebuildState::prepare(
            REBUILD_ID,
            recovery_topology.clone(),
            before.generation,
            dwv_core::SlotId(MISSING_SLOT),
            CodingPosition(0),
            AssignmentInstanceId(REPLACEMENT_ASSIGNMENT),
            REPLACEMENT_STORE,
            RebuildTargetIdentity::from_bytes(plan.replacement_identity),
        )
        .map_err(|error| DemoError::failed(error.to_string()))?;
        let mut transaction = recovery.begin_protocol_txn(before.generation, before.topology_epoch);
        transaction.begin_offline_rebuild(rebuild);
        recovery
            .commit_durable(transaction)
            .map_err(|error| DemoError::failed(error.to_string()))?;
        0
    };

    let survivor = FileRebuildStore::new(
        StoreId(11),
        open_store(root, &manifest.data_files[1], manifest, StoreId(11), false)?,
    );
    let mut data = vec![None, Some(survivor)];
    let mut parity = FileRebuildStore::new(
        StoreId(12),
        open_store(root, &manifest.parity_file, manifest, StoreId(12), false)?,
    );
    let degraded_range = ByteRange::new(0, CHUNK.min(manifest.protected_length))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let source_states = [
        ReconstructionSourceState::Missing,
        ReconstructionSourceState::Available,
        ReconstructionSourceState::Available,
    ];
    let refs = data.iter().map(Option::as_ref).collect::<Vec<_>>();
    let authorization = authorize_known_erasure(
        &topology,
        recovery
            .load_assembly_snapshot()
            .map_err(|error| DemoError::failed(error.to_string()))?
            .generation,
        geometry.clone(),
        degraded_range,
        ReconstructionRangeEvidence::ParityClean,
        true,
        &refs,
        &parity,
        &source_states,
    )
    .map_err(|error| DemoError::failed(error.to_string()))?;
    let degraded = read_known_erasure(
        &authorization,
        TopologyEpoch(EPOCH),
        authorization.recovery_generation(),
        &mut data,
        &mut parity,
    )
    .map_err(|error| DemoError::failed(error.to_string()))?;
    if degraded.bytes != reference[..usize::try_from(degraded_range.length).unwrap()] {
        return Err(DemoError::failed(
            "degraded read did not match the reference",
        ));
    }

    let mut completed_chunks = 0_u64;
    if stop_after == Some(0) {
        return checkpoint_report(target, parity, data, recovery, completed_chunks);
    }
    for range in plan_rebuild_ranges(
        manifest.protected_length,
        plan.chunk_size,
        cursor,
        manifest.logical_block_size,
    )
    .map_err(|error| DemoError::failed(error.to_string()))?
    {
        let snapshot = recovery
            .load_assembly_snapshot()
            .map_err(|error| DemoError::failed(error.to_string()))?;
        let rebuild = snapshot
            .rebuilds
            .iter()
            .find(|rebuild| rebuild.id() == REBUILD_ID)
            .ok_or_else(|| DemoError::failed("rebuild state disappeared"))?;
        let binding = RebuildBinding::from_recovery(rebuild);
        let refs = data.iter().map(Option::as_ref).collect::<Vec<_>>();
        let authorization = authorize_known_erasure(
            &topology,
            snapshot.generation,
            geometry.clone(),
            range,
            ReconstructionRangeEvidence::ParityClean,
            true,
            &refs,
            &parity,
            &source_states,
        )
        .map_err(|error| DemoError::failed(error.to_string()))?;
        let receipt = execute_rebuild_chunk(
            binding,
            &authorization,
            TopologyEpoch(EPOCH),
            snapshot.generation,
            &mut data,
            &mut parity,
            &mut target,
        )
        .map_err(|error| DemoError::failed(error.to_string()))?;
        commit_verified_rebuild_chunk(&mut recovery, REBUILD_ID, &receipt, &mut target)
            .map_err(|error| DemoError::failed(error.to_string()))?;
        completed_chunks += 1;
        if stop_after.is_some_and(|limit| completed_chunks >= limit) {
            return checkpoint_report(target, parity, data, recovery, completed_chunks);
        }
    }

    let snapshot = recovery
        .load_assembly_snapshot()
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let rebuild = snapshot
        .rebuilds
        .iter()
        .find(|rebuild| rebuild.id() == REBUILD_ID)
        .ok_or_else(|| DemoError::failed("rebuild state disappeared before final verification"))?;
    let binding = RebuildBinding::from_recovery(rebuild);
    let refs = data.iter().map(Option::as_ref).collect::<Vec<_>>();
    let full_range = ByteRange::new(0, manifest.protected_length)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let authorization = authorize_known_erasure(
        &topology,
        snapshot.generation,
        geometry,
        full_range,
        ReconstructionRangeEvidence::ParityClean,
        true,
        &refs,
        &parity,
        &source_states,
    )
    .map_err(|error| DemoError::failed(error.to_string()))?;
    let receipt = verify_complete_rebuild(
        binding,
        &authorization,
        TopologyEpoch(EPOCH),
        snapshot.generation,
        &mut data,
        &mut parity,
        &mut target,
    )
    .map_err(|error| DemoError::failed(error.to_string()))?;
    commit_verified_rebuild_completion(&mut recovery, REBUILD_ID, &receipt, &mut target)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    drop(target);
    drop(parity);
    drop(data);
    drop(recovery);
    finish_rebuild_report(
        root,
        manifest,
        reference,
        parity_before,
        &completed_chunks.to_string(),
    )
}

fn checkpoint_report(
    target: FileRebuildStore,
    parity: FileRebuildStore,
    data: Vec<Option<FileRebuildStore>>,
    recovery: SqliteRecoveryStore,
    completed_chunks: u64,
) -> Result<Value, DemoError> {
    drop(target);
    drop(parity);
    drop(data);
    drop(recovery);
    Ok(json!({
        "checkpoint": "durable",
        "completed_chunks": completed_chunks,
        "resumable": true,
        "final_verification": "pending",
    }))
}

fn finish_rebuild_report(
    root: &Path,
    manifest: &FixtureManifest,
    reference: Vec<u8>,
    parity_before: Vec<u8>,
    chunks: &str,
) -> Result<Value, DemoError> {
    let replacement = fs::read(root.join(&manifest.replacement_file))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let parity_after = fs::read(root.join(&manifest.parity_file))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    if replacement != reference {
        return Err(DemoError::failed(
            "replacement does not match the preserved reference",
        ));
    }
    if parity_before != parity_after {
        return Err(DemoError::failed(
            "parity changed during separate-target rebuild",
        ));
    }
    Ok(json!({
        "degraded_read": "matched-reference",
        "chunks": chunks,
        "final_verification": "passed",
        "replacement": "byte-equal-reference",
        "source_parity_preserved": true,
        "direct_read_after_close": true,
    }))
}

fn status_value(root: &Path, manifest: &FixtureManifest) -> Result<Value, DemoError> {
    let recovery = open_recovery(root, manifest)?;
    let snapshot = recovery
        .load_assembly_snapshot()
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let mut files = Vec::new();
    for name in manifest.data_files.iter().chain([
        &manifest.envelope_files[0],
        &manifest.envelope_files[1],
        &manifest.parity_file,
        &manifest.reference_file,
        &manifest.reference_data1_file,
        &manifest.replacement_file,
    ]) {
        let path = owned_path(root, name, false)?;
        let metadata = fs::metadata(&path).map_err(|error| DemoError::failed(error.to_string()))?;
        files.push(json!({ "path": name, "length": metadata.len() }));
    }
    Ok(json!({
        "contract": CONTRACT,
        "fixture": "owned",
        "protected_length": manifest.protected_length,
        "logical_block_size": manifest.logical_block_size,
        "topology_epoch": snapshot.topology_epoch.0,
        "recovery_generation": snapshot.generation.0,
        "recovery_health": format!("{:?}", recovery.verify_integrity()),
        "rebuilds": snapshot.rebuilds.len(),
        "files": files,
    }))
}

fn verify_value(root: &Path, manifest: &FixtureManifest) -> Result<Value, DemoError> {
    let geometry = codec_geometry(manifest.protected_length)?;
    let mut data0 = open_store(root, &manifest.data_files[0], manifest, StoreId(10), false)?;
    let mut data1 = open_store(root, &manifest.data_files[1], manifest, StoreId(11), false)?;
    let mut parity = open_store(root, &manifest.parity_file, manifest, StoreId(12), false)?;
    let range = ByteRange::new(0, manifest.protected_length)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let data0_bytes = data0
        .read_bytes(range)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let data1_bytes = data1
        .read_bytes(range)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let parity_bytes = parity
        .read_bytes(range)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let expected = compute_parity(&geometry, &[&data0_bytes, &data1_bytes])
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let reference_equal = fs::read(root.join(&manifest.reference_file))
        .map_err(|error| DemoError::failed(error.to_string()))?
        == data0_bytes;
    Ok(json!({
        "parity": if expected == parity_bytes { "clean" } else { "mismatch" },
        "reference": if reference_equal { "matched" } else { "mismatch" },
        "writes": 0,
        "clean_certification": "exhaustive-equation-scan",
    }))
}

pub fn scrub(root: &Path) -> Result<Value, DemoError> {
    let (_, manifest) = load_fixture(root)?;
    let (config, plan, _data, _parity, _replacement) = build_scrub_plan(root, &manifest, false)?;
    let report = scrub_report(&plan);
    let plan_file = if plan.repairs().candidates.len() == 1 {
        let plan_file = scrub_plan_file(&manifest, &plan)?;
        write_scrub_plan(root, &plan_file)?;
        json!(SCRUB_PLAN_FILE)
    } else {
        Value::Null
    };
    Ok(json!({
        "workflow": "scrub",
        "mode": format!("{:?}", config.region_count().map(|_| plan.report().mode()).unwrap()),
        "report": report,
        "repair_plan": plan_file,
        "payload_writes": 0,
        "claim_boundary": [
            "portable exhaustive checksum/parity evidence",
            "separate-target repair plan",
            "no automatic repair from parity disagreement alone"
        ],
    }))
}

pub fn repair(root: &Path, plan_path: &Path, confirmation: &str) -> Result<Value, DemoError> {
    let (_, manifest) = load_fixture(root)?;
    let plan_path = owned_path(root, plan_path.to_str().unwrap_or(SCRUB_PLAN_FILE), true)?;
    let plan_file: ScrubPlanFile = serde_json::from_slice(
        &fs::read(&plan_path).map_err(|error| DemoError::failed(error.to_string()))?,
    )
    .map_err(|error| DemoError::refused(format!("invalid scrub plan: {error}")))?;
    let expected = with_scrub_confirmation(ScrubPlanFile {
        confirmation: String::new(),
        ..plan_file.clone()
    })?
    .confirmation;
    if confirmation != expected || plan_file.confirmation != expected {
        return Err(DemoError::refused(
            "confirmation does not match the canonical scrub plan",
        ));
    }
    let (config, plan, mut data, mut parity, mut replacement) =
        build_scrub_plan(root, &manifest, true)?;
    validate_scrub_plan(&manifest, &plan_file, &plan, &data, &parity, &replacement)?;
    let context = plan.context();
    let outcomes = apply_scrub(
        &config,
        &plan,
        &mut data,
        &mut parity,
        &mut replacement,
        context,
    )
    .map_err(|error| DemoError::failed(error.to_string()))?;
    let fence = replacement
        .flush_rebuild()
        .map_err(|error| DemoError::failed(error.to_string()))?;
    Ok(json!({
        "workflow": "repair",
        "outcome": "verified-separate-target",
        "repairs": outcomes.iter().map(|outcome| json!({
            "range": {
                "offset": outcome.range().offset,
                "length": outcome.range().length,
            },
            "target": format!("{:?}", outcome.target()),
            "digest": hex_bytes(&outcome.digest()),
            "target_identity": hex_bytes(&(outcome.target_identity().0)),
        })).collect::<Vec<_>>(),
        "durable_fence": format!("{fence:?}"),
        "source_media_preserved": true,
        "integrity_publication": "not asserted without a protected-member mapping",
        "claim_boundary": [
            "portable separate-target readback and parity verification",
            "durable replacement fence evidence",
            "no in-place source-member mutation"
        ],
    }))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ReplaySummary {
    payload_digest: String,
    parity_digest: String,
    model: TraceModelState,
}

#[derive(Clone, Copy)]
struct TraceIo {
    offset: u64,
    length: u64,
    pattern: PayloadPattern,
    through: u64,
}

pub fn trace_export(root: &Path, trace_path: &Path) -> Result<Value, DemoError> {
    let (root, manifest) = load_fixture(root)?;
    let write_length = CHUNK.min(manifest.protected_length);
    let mut trace = Trace::new(TraceFixture {
        size: manifest.protected_length,
        seed: 7,
        write_length,
    })
    .map_err(trace_error)?;
    push_trace(
        &mut trace,
        TraceEventKind::Open {
            size: manifest.protected_length,
        },
    )?;
    push_trace(
        &mut trace,
        TraceEventKind::Write {
            store: 0,
            offset: 0,
            length: write_length,
            pattern: PayloadPattern::Counter { seed: 17 },
        },
    )?;
    push_trace(
        &mut trace,
        TraceEventKind::Flush {
            store: 0,
            through: 1,
        },
    )?;
    push_trace(
        &mut trace,
        TraceEventKind::Read {
            store: 0,
            offset: 0,
            length: write_length,
        },
    )?;
    push_trace(&mut trace, TraceEventKind::RecoveryIntent { generation: 1 })?;
    push_trace(
        &mut trace,
        TraceEventKind::Checkpoint {
            generation: 1,
            durable: true,
        },
    )?;
    push_trace(
        &mut trace,
        TraceEventKind::Checksum {
            region: 0,
            outcome: ChecksumOutcome::CurrentMatch,
        },
    )?;
    push_trace(
        &mut trace,
        TraceEventKind::DegradedRead {
            slot: 1,
            offset: 0,
            length: BLOCK.min(write_length as u32) as u64,
        },
    )?;
    push_trace(
        &mut trace,
        TraceEventKind::RebuildChunk {
            offset: 0,
            length: write_length,
            verified: true,
        },
    )?;
    push_trace(
        &mut trace,
        TraceEventKind::RepairDecision {
            target: REPLACEMENT_STORE.0 as u8,
            decision: RepairDecision::Verified,
        },
    )?;
    push_trace(
        &mut trace,
        TraceEventKind::Outcome {
            outcome: TraceOutcome::Success,
        },
    )?;
    let bytes = trace.to_json().map_err(trace_error)?;
    let path = owned_trace_path(&root, trace_path, false)?;
    fs::write(&path, bytes).map_err(|error| DemoError::failed(error.to_string()))?;
    Ok(json!({
        "workflow": "trace-export",
        "trace": trace_path.to_string_lossy(),
        "schema_version": trace.schema,
        "events": trace.events.len(),
        "claim_boundary": trace_claim_boundary(),
    }))
}

pub fn trace_render(root: &Path, trace_path: &Path) -> Result<Value, DemoError> {
    let (root, _) = load_fixture(root)?;
    let trace = read_trace(&root, trace_path)?;
    let rendered = trace
        .events
        .iter()
        .map(|event| format!("{:04} {:?}", event.sequence, event.kind))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(json!({
        "workflow": "trace-render",
        "trace": trace_path.to_string_lossy(),
        "summary": trace.summary(),
        "model": trace.model_state().map_err(trace_error)?,
        "rendered": rendered,
        "claim_boundary": trace_claim_boundary(),
    }))
}

pub fn trace_replay(root: &Path, trace_path: &Path) -> Result<Value, DemoError> {
    let (root, manifest) = load_fixture(root)?;
    let trace = read_trace(&root, trace_path)?;
    if trace.fixture.size != manifest.protected_length {
        return Err(DemoError::refused(
            "trace fixture size differs from the current demo fixture",
        ));
    }
    let source_path = root.join(&manifest.data_files[0]);
    let source_before =
        fs::read(&source_path).map_err(|error| DemoError::failed(error.to_string()))?;
    let parity = fs::read(root.join(&manifest.parity_file))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let simulator = replay_in_simulator(&trace, &parity)?;
    let file_backed = replay_in_file_store(&root, &manifest, &trace)?;
    let source_after =
        fs::read(&source_path).map_err(|error| DemoError::failed(error.to_string()))?;
    if source_before != source_after {
        return Err(DemoError::failed(
            "trace replay mutated the caller fixture source",
        ));
    }
    if simulator != file_backed {
        let sequence = trace
            .events
            .iter()
            .position(|event| matches!(event.kind, TraceEventKind::Write { .. }))
            .unwrap_or(0);
        let minimized = trace.minimize_prefix(sequence).map_err(trace_error)?;
        let minimized_path = owned_path(&root, TRACE_MINIMIZED_FILE, false)?;
        fs::write(&minimized_path, minimized.to_json().map_err(trace_error)?)
            .map_err(|error| DemoError::failed(error.to_string()))?;
        return Err(DemoError::refused(format!(
            "trace replay mismatch at event {sequence}; minimized trace written to {TRACE_MINIMIZED_FILE}"
        )));
    }
    Ok(json!({
        "workflow": "trace-replay",
        "trace": trace_path.to_string_lossy(),
        "equivalent": true,
        "event_count": trace.events.len(),
        "simulator": simulator,
        "file_backed": file_backed,
        "claim_boundary": trace_claim_boundary(),
    }))
}

fn trace_claim_boundary() -> [&'static str; 5] {
    [
        "portable normalized semantic events",
        "deterministic volatile-media simulator replay",
        "copied ordinary-file replay through FileStore",
        "no raw payloads, paths, or backend handles in trace bytes",
        "no Linux, live bridge, or physical durability claim",
    ]
}

fn push_trace(trace: &mut Trace, event: TraceEventKind) -> Result<(), DemoError> {
    trace.push(event).map_err(trace_error)
}

fn trace_error(error: TraceError) -> DemoError {
    DemoError::refused(error.to_string())
}

fn owned_trace_path(root: &Path, path: &Path, must_exist: bool) -> Result<PathBuf, DemoError> {
    let relative = path
        .to_str()
        .ok_or_else(|| DemoError::usage("trace path must be valid UTF-8"))?;
    owned_path(root, relative, must_exist)
}

fn read_trace(root: &Path, trace_path: &Path) -> Result<Trace, DemoError> {
    let path = owned_trace_path(root, trace_path, true)?;
    let bytes = fs::read(&path).map_err(|error| DemoError::failed(error.to_string()))?;
    Trace::from_json(&bytes).map_err(trace_error)
}

fn trace_io(trace: &Trace) -> Result<TraceIo, DemoError> {
    let mut write = None;
    let mut read = None;
    let mut through = None;
    for event in &trace.events {
        match &event.kind {
            TraceEventKind::Write {
                store,
                offset,
                length,
                pattern,
            } if *store == 0 => {
                if write.replace((*offset, *length, *pattern)).is_some() {
                    return Err(DemoError::refused("trace contains multiple writes"));
                }
            }
            TraceEventKind::Read {
                store,
                offset,
                length,
            } if *store == 0 => {
                if read.replace((*offset, *length)).is_some() {
                    return Err(DemoError::refused("trace contains multiple reads"));
                }
            }
            TraceEventKind::Flush {
                store,
                through: watermark,
            } if *store == 0 && through.replace(*watermark).is_some() => {
                return Err(DemoError::refused("trace contains multiple flushes"));
            }
            TraceEventKind::Flush { store, .. } if *store == 0 => {}
            _ => {}
        }
    }
    let (offset, length, pattern) =
        write.ok_or_else(|| DemoError::refused("trace has no supported write"))?;
    let (read_offset, read_length) =
        read.ok_or_else(|| DemoError::refused("trace has no supported read"))?;
    if (offset, length) != (read_offset, read_length) {
        return Err(DemoError::refused(
            "trace read range does not match its write range",
        ));
    }
    Ok(TraceIo {
        offset,
        length,
        pattern,
        through: through.ok_or_else(|| DemoError::refused("trace has no supported flush"))?,
    })
}

fn child_operation(index: u32) -> ChildOperationId {
    ChildOperationId {
        slot: OperationSlotToken::new(0, 1),
        index,
    }
}

fn replay_in_simulator(trace: &Trace, parity: &[u8]) -> Result<ReplaySummary, DemoError> {
    let io = trace_io(trace)?;
    let initial = deterministic_bytes(trace.fixture.size, trace.fixture.seed as u8, 13);
    let bytes = materialize_pattern(io.pattern, io.length).map_err(trace_error)?;
    let mut simulator = Simulator::new(
        initial,
        SimulatorConfig::for_size(trace.fixture.size as usize),
    )
    .map_err(|error| DemoError::failed(error.to_string()))?;
    let range = ByteRange::new(io.offset, io.length)
        .map_err(|error| DemoError::refused(error.to_string()))?;
    let schedule = Schedule::new(vec![
        ScheduleStep::SubmitWrite {
            operation_id: child_operation(1),
            offset: io.offset,
            bytes,
            intent: WriteIntent::Ordinary,
        },
        ScheduleStep::Deliver { pending_index: 0 },
        ScheduleStep::SubmitFlush {
            operation_id: child_operation(2),
            through: StoreWriteWatermark(io.through),
        },
        ScheduleStep::Deliver { pending_index: 0 },
        ScheduleStep::SubmitRead {
            operation_id: child_operation(3),
            offset: io.offset,
            length: io.length,
        },
        ScheduleStep::Deliver { pending_index: 0 },
    ]);
    let simulation = simulator
        .run(&schedule)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let snapshot = simulation.final_snapshot;
    let start = usize::try_from(range.offset)
        .map_err(|_| DemoError::failed("simulator range does not fit usize"))?;
    let end = start
        .checked_add(usize::try_from(range.length).unwrap())
        .ok_or_else(|| DemoError::failed("simulator range overflows usize"))?;
    let expected = materialize_pattern(io.pattern, io.length).map_err(trace_error)?;
    if snapshot.durable_media.get(start..end) != Some(expected.as_slice()) {
        return Err(DemoError::failed(
            "simulator replay readback differs from the symbolic pattern",
        ));
    }
    Ok(ReplaySummary {
        payload_digest: hex_bytes(blake3::hash(&snapshot.durable_media).as_bytes()),
        parity_digest: hex_bytes(blake3::hash(parity).as_bytes()),
        model: trace.model_state().map_err(trace_error)?,
    })
}

fn replay_in_file_store(
    root: &Path,
    manifest: &FixtureManifest,
    trace: &Trace,
) -> Result<ReplaySummary, DemoError> {
    let io = trace_io(trace)?;
    let source = root.join(&manifest.data_files[0]);
    let parity = fs::read(root.join(&manifest.parity_file))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| DemoError::failed(error.to_string()))?
        .as_nanos();
    let replay_root =
        std::env::temp_dir().join(format!("dwv-trace-replay-{}-{nonce}", std::process::id()));
    fs::create_dir(&replay_root).map_err(|error| DemoError::failed(error.to_string()))?;
    fs::copy(&source, replay_root.join(&manifest.data_files[0]))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let result = (|| {
        let mut store = open_store(
            &replay_root,
            &manifest.data_files[0],
            manifest,
            StoreId(10),
            true,
        )?;
        let range = ByteRange::new(io.offset, io.length)
            .map_err(|error| DemoError::refused(error.to_string()))?;
        let bytes = materialize_pattern(io.pattern, io.length).map_err(trace_error)?;
        let write = store.write_bytes(child_operation(1), range, &bytes, WriteIntent::Ordinary);
        require_success(&write, "file-backed write")?;
        let flush = store.flush_file(child_operation(2), StoreWriteWatermark(io.through));
        require_success(&flush, "file-backed flush")?;
        let read = store
            .read_bytes(range)
            .map_err(|error| DemoError::failed(error.to_string()))?;
        if read != bytes {
            return Err(DemoError::failed(
                "file-backed replay readback differs from the symbolic pattern",
            ));
        }
        let payload = store
            .read_bytes(
                ByteRange::new(0, trace.fixture.size)
                    .map_err(|error| DemoError::failed(error.to_string()))?,
            )
            .map_err(|error| DemoError::failed(error.to_string()))?;
        Ok(ReplaySummary {
            payload_digest: hex_bytes(blake3::hash(&payload).as_bytes()),
            parity_digest: hex_bytes(blake3::hash(&parity).as_bytes()),
            model: trace.model_state().map_err(trace_error)?,
        })
    })();
    let _ = fs::remove_dir_all(&replay_root);
    result
}

fn require_success(
    completion: &dwv_store::StoreCompletion,
    operation: &str,
) -> Result<(), DemoError> {
    if !matches!(&completion.disposition, CompletionDisposition::Success)
        || !completion
            .completed
            .covers(completion.requested)
            .map_err(|error| DemoError::failed(error.to_string()))?
    {
        return Err(DemoError::failed(format!(
            "{operation} did not complete durably: {:?}",
            completion.disposition
        )));
    }
    Ok(())
}

fn build_scrub_plan(
    root: &Path,
    manifest: &FixtureManifest,
    writable_target: bool,
) -> Result<
    (
        ScanConfig,
        dwv_verify::ScrubPlan,
        Vec<FileRebuildStore>,
        FileRebuildStore,
        FileRebuildStore,
    ),
    DemoError,
> {
    let geometry = codec_geometry(manifest.protected_length)?;
    let config = ScanConfig::new(geometry.clone(), manifest.protected_length)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let mut data = vec![
        FileRebuildStore::new(
            StoreId(10),
            open_store(root, &manifest.data_files[0], manifest, StoreId(10), false)?,
        ),
        FileRebuildStore::new(
            StoreId(11),
            open_store(root, &manifest.data_files[1], manifest, StoreId(11), false)?,
        ),
    ];
    let mut parity = FileRebuildStore::new(
        StoreId(12),
        open_store(root, &manifest.parity_file, manifest, StoreId(12), false)?,
    );
    let replacement = open_rebuild_target(root, manifest, writable_target)?;
    let reference0 = fs::read(root.join(&manifest.reference_file))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let reference1 = fs::read(root.join(&manifest.reference_data1_file))
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let expected_length = usize::try_from(manifest.protected_length)
        .map_err(|_| DemoError::failed("protected length does not fit memory"))?;
    if reference0.len() != expected_length || reference1.len() != expected_length {
        return Err(DemoError::blocked("checksum reference length is invalid"));
    }
    let expected_parity = compute_parity(&geometry, &[&reference0, &reference1])
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let evidence = ChecksumEvidence::new(
        vec![
            vec![DigestEvidence::Current(
                *blake3::hash(&reference0).as_bytes(),
            )],
            vec![DigestEvidence::Current(
                *blake3::hash(&reference1).as_bytes(),
            )],
        ],
        vec![DigestEvidence::Current(
            *blake3::hash(&expected_parity).as_bytes(),
        )],
    );
    let report = verify_exhaustive(&mut data, &mut parity, &config, &evidence)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let recovery = open_recovery(root, manifest)?;
    let snapshot = recovery
        .load_assembly_snapshot()
        .map_err(|error| DemoError::failed(error.to_string()))?;
    let context = ScrubContext {
        topology_epoch: snapshot.topology_epoch,
        recovery_generation: snapshot.generation,
        checksum_set_generation: ChecksumSetGeneration::INITIAL,
    };
    let plan = plan_scrub(report, &data, &parity, &replacement, context);
    Ok((config, plan, data, parity, replacement))
}

fn scrub_report(plan: &dwv_verify::ScrubPlan) -> Value {
    let regions = plan
        .report()
        .regions()
        .iter()
        .map(|region| {
            json!({
                "range": {
                    "offset": region.range.offset,
                    "length": region.range.length,
                },
                "disposition": format!("{:?}", region.disposition),
                "data_evidence": region.data_evidence.iter().map(|status| format!("{status:?}")).collect::<Vec<_>>(),
                "parity_evidence": format!("{:?}", region.parity_evidence),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "mode": format!("{:?}", plan.report().mode()),
        "exhaustive_complete": plan.report().exhaustive_complete(),
        "parity_consistent": plan.report().parity_consistent(),
        "regions": regions,
        "repair_candidates": plan.repairs().candidates.len(),
        "repair_refusals": plan.repairs().refused.len(),
    })
}

fn scrub_plan_file(
    manifest: &FixtureManifest,
    plan: &dwv_verify::ScrubPlan,
) -> Result<ScrubPlanFile, DemoError> {
    let candidate = plan
        .repairs()
        .candidates
        .first()
        .ok_or_else(|| DemoError::refused("scrub did not identify a repair candidate"))?;
    let target = match candidate.target() {
        RepairTarget::Data { slot } => ScrubTargetFile::Data { slot },
        RepairTarget::Parity => ScrubTargetFile::Parity,
    };
    with_scrub_confirmation(ScrubPlanFile {
        schema: 1,
        contract: CONTRACT.to_owned(),
        topology_epoch: plan.context().topology_epoch.0,
        recovery_generation: plan.context().recovery_generation.0,
        checksum_set_generation: plan.context().checksum_set_generation.0,
        data_identities: manifest.data_identities.to_vec(),
        parity_identity: manifest.parity_identity,
        replacement_identity: manifest.replacement_identity,
        range_offset: candidate.range().offset,
        range_length: candidate.range().length,
        target,
        confirmation: String::new(),
    })
}

fn validate_scrub_plan(
    manifest: &FixtureManifest,
    saved: &ScrubPlanFile,
    plan: &dwv_verify::ScrubPlan,
    data: &[FileRebuildStore],
    parity: &FileRebuildStore,
    replacement: &FileRebuildStore,
) -> Result<(), DemoError> {
    let candidate = plan
        .repairs()
        .candidates
        .first()
        .ok_or_else(|| DemoError::refused("scrub plan no longer has a repair candidate"))?;
    let current_data = data
        .iter()
        .map(|store| store.identity().0)
        .collect::<Vec<_>>();
    let current_target = match candidate.target() {
        RepairTarget::Data { slot } => ScrubTargetFile::Data { slot },
        RepairTarget::Parity => ScrubTargetFile::Parity,
    };
    if saved.schema != 1
        || saved.contract != CONTRACT
        || saved.topology_epoch != plan.context().topology_epoch.0
        || saved.recovery_generation != plan.context().recovery_generation.0
        || saved.checksum_set_generation != plan.context().checksum_set_generation.0
        || saved.data_identities != current_data
        || saved.parity_identity != parity.identity().0
        || saved.replacement_identity != replacement.identity().0
        || saved.replacement_identity != manifest.replacement_identity
        || saved.range_offset != candidate.range().offset
        || saved.range_length != candidate.range().length
        || saved.target != current_target
    {
        return Err(DemoError::refused(
            "scrub plan is stale or no longer matches the fixture",
        ));
    }
    Ok(())
}

fn write_scrub_plan(root: &Path, plan: &ScrubPlanFile) -> Result<(), DemoError> {
    let bytes =
        serde_json::to_vec_pretty(plan).map_err(|error| DemoError::failed(error.to_string()))?;
    fs::write(root.join(SCRUB_PLAN_FILE), bytes)
        .map_err(|error| DemoError::failed(error.to_string()))
}

fn with_scrub_confirmation(mut plan: ScrubPlanFile) -> Result<ScrubPlanFile, DemoError> {
    plan.confirmation.clear();
    let canonical =
        serde_json::to_vec(&plan).map_err(|error| DemoError::failed(error.to_string()))?;
    plan.confirmation = hex(&blake3::hash(&canonical));
    Ok(plan)
}

fn initial_recovery_manifest(manifest: &FixtureManifest) -> Result<RecoveryManifest, DemoError> {
    let topology = core_topology(manifest)?;
    let recovery_topology = recovery_topology(&topology)?;
    let mut recovery = MemoryRecoveryStore::new(TopologyEpoch(0));
    let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration::ZERO, TopologyEpoch(0));
    transaction.push(RecoveryMutation::PrepareTopology {
        topology: recovery_topology,
    });
    transaction.push(RecoveryMutation::CommitTopology {
        topology_epoch: TopologyEpoch(EPOCH),
    });
    let generation = recovery
        .commit_durable(transaction)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    recovery
        .export_manifest(generation)
        .map_err(|error| DemoError::failed(error.to_string()))
}

fn core_topology(manifest: &FixtureManifest) -> Result<TopologySnapshot, DemoError> {
    TopologySnapshot::new(
        ArrayId(manifest.array_id),
        TopologyEpoch(manifest.topology_epoch),
        CodingProfile::new(2, 1).map_err(|error| DemoError::failed(format!("{error:?}")))?,
        ProtectedGeometry::new(manifest.protected_length, manifest.logical_block_size)
            .map_err(|error| DemoError::failed(format!("{error:?}")))?,
        vec![
            TopologyAssignment::new(
                dwv_core::SlotId([1; 16]),
                MemberRole::Data,
                CodingPosition(0),
                AssignmentInstanceId([11; 16]),
                AssignmentGeneration(1),
            ),
            TopologyAssignment::new(
                dwv_core::SlotId([2; 16]),
                MemberRole::Data,
                CodingPosition(1),
                AssignmentInstanceId([12; 16]),
                AssignmentGeneration(1),
            ),
            TopologyAssignment::new(
                dwv_core::SlotId([3; 16]),
                MemberRole::Parity,
                CodingPosition(2),
                AssignmentInstanceId([13; 16]),
                AssignmentGeneration(1),
            ),
        ],
    )
    .map_err(|error| DemoError::failed(error.to_string()))
}

fn recovery_topology(topology: &TopologySnapshot) -> Result<RecoveryTopologySnapshot, DemoError> {
    RecoveryTopologySnapshot::from_core(
        topology.clone(),
        vec![StoreId(10), StoreId(11), StoreId(12)],
    )
    .map_err(|error| DemoError::failed(error.to_string()))
}

fn open_recovery(
    root: &Path,
    manifest: &FixtureManifest,
) -> Result<SqliteRecoveryStore, DemoError> {
    SqliteRecoveryStore::open(root.join(&manifest.recovery_file))
        .map_err(|error| DemoError::failed(error.to_string()))
}

fn open_store(
    root: &Path,
    relative: &str,
    manifest: &FixtureManifest,
    store_id: StoreId,
    writable: bool,
) -> Result<FileStore, DemoError> {
    let path = owned_path(root, relative, true)?;
    FileStore::open(
        FileStoreConfig::new(path, manifest.protected_length, manifest.logical_block_size)
            .maximum_transfer(manifest.protected_length)
            .writable(writable)
            .sync_mode(FileSyncMode::SyncAll)
            .store_id(store_id)
            .topology_epoch(TopologyEpoch(manifest.topology_epoch)),
    )
    .map_err(|error| DemoError::failed(error.to_string()))
}
fn file_identity(store: &FileStore) -> [u8; 16] {
    store
        .capabilities_report()
        .identity
        .observations
        .first()
        .map(|observation| observation.fingerprint)
        .unwrap_or([0; 16])
}

fn create_empty_replacement(root: &Path, manifest: &FixtureManifest) -> Result<(), DemoError> {
    let path = owned_path(root, &manifest.replacement_file, false)?;
    let store = FileStore::open(
        FileStoreConfig::new(path, manifest.protected_length, manifest.logical_block_size)
            .maximum_transfer(manifest.protected_length)
            .writable(true)
            .create_new(true)
            .sparse(true)
            .sync_mode(FileSyncMode::SyncAll)
            .store_id(REPLACEMENT_STORE)
            .topology_epoch(TopologyEpoch(manifest.topology_epoch)),
    )
    .map_err(|error| DemoError::failed(error.to_string()))?;
    drop(store);
    Ok(())
}

fn open_rebuild_target(
    root: &Path,
    manifest: &FixtureManifest,
    writable: bool,
) -> Result<FileRebuildStore, DemoError> {
    Ok(FileRebuildStore::new(
        REPLACEMENT_STORE,
        open_store(
            root,
            &manifest.replacement_file,
            manifest,
            REPLACEMENT_STORE,
            writable,
        )?,
    ))
}

fn codec_geometry(length: u64) -> Result<Geometry, DemoError> {
    Geometry::new(vec![length, length], length)
        .map_err(|error| DemoError::failed(error.to_string()))
}

fn deterministic_bytes(length: u64, start: u8, step: u8) -> Vec<u8> {
    (0..length)
        .map(|index| start.wrapping_add((index as u8).wrapping_mul(step)))
        .collect()
}

fn validate_size(size: u64) -> Result<(), DemoError> {
    if size < u64::from(BLOCK) || size > MAX_SIZE || !size.is_multiple_of(u64::from(BLOCK)) {
        return Err(DemoError::usage(format!(
            "size must be block-aligned between {} and {} bytes",
            BLOCK, MAX_SIZE
        )));
    }
    Ok(())
}

fn load_fixture(root: &Path) -> Result<(PathBuf, FixtureManifest), DemoError> {
    let root = fs::canonicalize(root)
        .map_err(|error| DemoError::blocked(format!("fixture root is unavailable: {error}")))?;
    let marker = fs::read_to_string(root.join(MARKER_FILE))
        .map_err(|error| DemoError::blocked(format!("fixture marker is unavailable: {error}")))?;
    if !marker.starts_with("diskweave disposable macOS demo") {
        return Err(DemoError::blocked("fixture marker is not recognized"));
    }
    let manifest: FixtureManifest = serde_json::from_slice(
        &fs::read(root.join(MANIFEST_FILE))
            .map_err(|error| DemoError::blocked(error.to_string()))?,
    )
    .map_err(|error| DemoError::blocked(format!("fixture manifest is invalid: {error}")))?;
    if manifest.schema != 1 || manifest.contract != CONTRACT {
        return Err(DemoError::blocked(
            "fixture contract version is unsupported",
        ));
    }
    validate_size(manifest.protected_length)?;
    for path in manifest.data_files.iter().chain([
        &manifest.envelope_files[0],
        &manifest.envelope_files[1],
        &manifest.parity_file,
        &manifest.reference_file,
        &manifest.reference_data1_file,
        &manifest.replacement_file,
        &manifest.recovery_file,
    ]) {
        owned_path(&root, path, true)?;
    }
    validate_identities(&root, &manifest)?;

    Ok((root, manifest))
}
fn validate_identities(root: &Path, manifest: &FixtureManifest) -> Result<(), DemoError> {
    for (relative, expected, store_id) in [
        (
            &manifest.data_files[0],
            manifest.data_identities[0],
            StoreId(10),
        ),
        (
            &manifest.data_files[1],
            manifest.data_identities[1],
            StoreId(11),
        ),
        (&manifest.parity_file, manifest.parity_identity, StoreId(12)),
        (
            &manifest.reference_file,
            manifest.reference_identity,
            StoreId(30),
        ),
        (
            &manifest.replacement_file,
            manifest.replacement_identity,
            REPLACEMENT_STORE,
        ),
    ] {
        let store = open_store(root, relative, manifest, store_id, false)?;
        if file_identity(&store) != expected {
            return Err(DemoError::refused(format!(
                "fixture identity changed for {relative}"
            )));
        }
    }
    Ok(())
}

fn owned_path(root: &Path, relative: &str, must_exist: bool) -> Result<PathBuf, DemoError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(DemoError::refused(format!(
            "fixture path is not relative and owned: {relative}"
        )));
    }
    let root = fs::canonicalize(root).map_err(|error| DemoError::blocked(error.to_string()))?;
    let joined = root.join(path);
    if must_exist {
        let canonical = fs::canonicalize(&joined).map_err(|error| {
            DemoError::blocked(format!("fixture entry is unavailable: {error}"))
        })?;
        if !canonical.starts_with(&root) {
            return Err(DemoError::refused("fixture entry escapes its owned root"));
        }
        Ok(canonical)
    } else {
        let parent = joined
            .parent()
            .ok_or_else(|| DemoError::refused("fixture entry has no parent"))?;
        let parent =
            fs::canonicalize(parent).map_err(|error| DemoError::blocked(error.to_string()))?;
        if !parent.starts_with(&root) {
            return Err(DemoError::refused(
                "fixture destination escapes its owned root",
            ));
        }
        Ok(joined)
    }
}

fn write_manifest(root: &Path, manifest: &FixtureManifest) -> Result<(), DemoError> {
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|error| DemoError::failed(error.to_string()))?;
    fs::write(root.join(MANIFEST_FILE), bytes).map_err(|error| DemoError::failed(error.to_string()))
}

fn write_plan(path: &Path, plan: &RebuildPlan) -> Result<(), DemoError> {
    let bytes =
        serde_json::to_vec_pretty(plan).map_err(|error| DemoError::failed(error.to_string()))?;
    fs::write(path, bytes).map_err(|error| DemoError::failed(error.to_string()))
}

fn with_confirmation(mut plan: RebuildPlan) -> Result<RebuildPlan, DemoError> {
    plan.confirmation.clear();
    let canonical =
        serde_json::to_vec(&plan).map_err(|error| DemoError::failed(error.to_string()))?;
    plan.confirmation = hex(&blake3::hash(&canonical));
    Ok(plan)
}

fn validate_plan(manifest: &FixtureManifest, plan: &RebuildPlan) -> Result<(), DemoError> {
    if plan.schema != 1
        || plan.contract != CONTRACT
        || plan.rebuild_id != REBUILD_ID.as_bytes()
        || plan.missing_slot != MISSING_SLOT
        || plan.replacement_store != REPLACEMENT_STORE.0
        || plan.replacement_file != manifest.replacement_file
        || plan.topology_epoch != manifest.topology_epoch
        || plan.protected_length != manifest.protected_length
        || plan.chunk_size == 0
        || !plan.chunk_size.is_multiple_of(u64::from(BLOCK))
    {
        return Err(DemoError::refused(
            "rebuild plan does not match the fixture",
        ));
    }
    Ok(())
}

fn hex(hash: &Hash) -> String {
    hex_bytes(hash.as_bytes())
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
