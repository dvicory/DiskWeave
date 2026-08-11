//! Portable recovery-state semantics.
//!
//! This crate is the authority boundary for dirty intent, integrity evidence,
//! topology epochs, and clean checkpoints.  The reference implementation is
//! in memory so its transaction rules can be exercised without selecting a
//! SQLite layout or durability configuration.

use dwv_core::{FenceDomain, TopologyEpoch};
use dwv_store::{StoreFenceRef, StoreId};
use std::fmt;

mod baseline;
mod checkpoint;
mod extent;
mod generation;
mod inspection;
mod intent;
mod invalidation;
mod job;
mod metadata_loss;
mod migration;
mod profile;
mod provider;
mod rebuild;
mod record;
mod transition;

pub use baseline::{
    ChecksumBaseline, ChecksumBaselineInvalidReason, ChecksumBaselineProvenance,
    ChecksumBaselineStatus, assess_checksum_baseline, expected_checksum_extents,
    new_checksum_baseline, pending_checksum_baseline_extents,
};
pub use checkpoint::{
    CheckpointDecision, CheckpointRefusal, CheckpointRequest, RequiredFence, evaluate_checkpoint,
    fence_ref,
};
pub use extent::{ChecksumExtent, ChecksumTarget, ExtentError};
pub use generation::{GenerationCapture, RecoveryGeneration};
pub use inspection::{
    RecoveryFormatLayer, RecoveryInspection, RecoveryReconciliation, reconcile_uncertain_commit,
};
pub use intent::IntentCommit;
pub use invalidation::{
    IntentBoundary, IntentCoverage, IntentDecision, IntentEvidence, InvalidationTarget,
    assess_intent,
};
pub use job::{
    ChecksumAuthority, ChecksumJob, ChecksumJobKey, ChecksumJobResult, ChecksumQueue,
    CommitOutcome, JobError, ReadEvidence,
};
pub use metadata_loss::{
    BaselineDisposition, EvidenceRequirement, METADATA_LOSS_MATRIX_VERSION, MetadataLossAction,
    MetadataLossAudit, MetadataLossAuthorization, MetadataLossCase, MetadataLossDisposition,
    MetadataLossError, MetadataLossPlan, MetadataLossVerification, PayloadWritePolicy,
    create_fresh_manifest, render_metadata_loss_matrix,
};
pub use migration::{MigrationError, MigrationOutcome, ProfileMigration};
pub use profile::{
    BLAKE3_256_PROFILE, ChecksumProfile, ChecksumProfileId, ChecksumSet, ChecksumSetGeneration,
    ChecksumSetState, ProfileError,
};
pub use provider::{Blake3Provider, DigestProvider, ProviderError};
pub use rebuild::{
    REBUILD_ID_BYTES, REBUILD_TARGET_IDENTITY_BYTES, RebuildChunkReceipt, RebuildCompletionReceipt,
    RebuildCursor, RebuildError, RebuildId, RebuildLifecycle, RebuildState, RebuildTargetIdentity,
};
pub use record::{ChecksumRecord, ChecksumState, ContentGeneration, Digest, FenceEvidence};
pub use transition::{
    RecoveryTransitionId, TransitionEvidence, TransitionKind, TransitionOutcome, TransitionTrace,
};

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct SessionId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RegionId(pub u64);

pub const DIRTY_REGION_BYTES: u64 = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionMappingError {
    EmptyRange,
    InvalidRegionBytes,
    RegionIdOverflow,
}

impl fmt::Display for RegionMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRange => {
                write!(formatter, "dirty-region mapping requires a non-empty range")
            }
            Self::InvalidRegionBytes => write!(formatter, "dirty-region size must be non-zero"),
            Self::RegionIdOverflow => {
                write!(formatter, "dirty-region identity is not representable")
            }
        }
    }
}

impl std::error::Error for RegionMappingError {}

pub fn dirty_regions_for_range(
    member_index: u32,
    range: dwv_core::ByteRange,
    region_bytes: u64,
) -> Result<Vec<RegionId>, RegionMappingError> {
    if range.is_empty() {
        return Err(RegionMappingError::EmptyRange);
    }
    if region_bytes == 0 {
        return Err(RegionMappingError::InvalidRegionBytes);
    }

    let first = range.offset / region_bytes;
    let last = range
        .end()
        .checked_sub(1)
        .ok_or(RegionMappingError::RegionIdOverflow)?
        / region_bytes;
    let first = u32::try_from(first).map_err(|_| RegionMappingError::RegionIdOverflow)?;
    let last = u32::try_from(last).map_err(|_| RegionMappingError::RegionIdOverflow)?;
    Ok((first..=last)
        .map(|region| RegionId((u64::from(member_index) << 32) | u64::from(region)))
        .collect())
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct IntegrityExtentId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct JobId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RecoveryCursor(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RecoverySchemaVersion(pub u16);

pub const CURRENT_RECOVERY_SCHEMA: RecoverySchemaVersion = RecoverySchemaVersion(4);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryRecordKind {
    SnapshotHeader,
    Topology,
    DirtyRegion,
    IntegrityEvidence,
    ChecksumBaseline,
    StoreFence,
    WritableSession,
    MaintenanceCheckpoint,
    MigrationState,
    MetadataLossAudit,
    OfflineRebuild,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverySchemaDescriptor {
    pub version: RecoverySchemaVersion,
    pub records: Vec<RecoveryRecordKind>,
}

pub fn current_recovery_schema() -> RecoverySchemaDescriptor {
    RecoverySchemaDescriptor {
        version: CURRENT_RECOVERY_SCHEMA,
        records: vec![
            RecoveryRecordKind::SnapshotHeader,
            RecoveryRecordKind::Topology,
            RecoveryRecordKind::DirtyRegion,
            RecoveryRecordKind::IntegrityEvidence,
            RecoveryRecordKind::StoreFence,
            RecoveryRecordKind::WritableSession,
            RecoveryRecordKind::MaintenanceCheckpoint,
            RecoveryRecordKind::MigrationState,
            RecoveryRecordKind::MetadataLossAudit,
            RecoveryRecordKind::OfflineRebuild,
            RecoveryRecordKind::ChecksumBaseline,
        ],
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryMigrationStep {
    InitializeSemanticSchemaV1,
    AddMetadataLossAuditV2,
    AddOfflineRebuildV3,
    AddChecksumBaselineV4,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryMigrationPlan {
    pub from: RecoverySchemaVersion,
    pub to: RecoverySchemaVersion,
    pub steps: Vec<RecoveryMigrationStep>,
}

impl RecoveryMigrationPlan {
    pub fn plan(
        from: RecoverySchemaVersion,
        to: RecoverySchemaVersion,
    ) -> Result<Self, RecoveryError> {
        let steps = match (from, to) {
            (from, to) if from == to && from.0 <= CURRENT_RECOVERY_SCHEMA.0 => Vec::new(),
            (RecoverySchemaVersion(0), RecoverySchemaVersion(1)) => {
                vec![RecoveryMigrationStep::InitializeSemanticSchemaV1]
            }
            (RecoverySchemaVersion(0), RecoverySchemaVersion(2)) => vec![
                RecoveryMigrationStep::InitializeSemanticSchemaV1,
                RecoveryMigrationStep::AddMetadataLossAuditV2,
            ],
            (RecoverySchemaVersion(0), RecoverySchemaVersion(3)) => vec![
                RecoveryMigrationStep::InitializeSemanticSchemaV1,
                RecoveryMigrationStep::AddMetadataLossAuditV2,
                RecoveryMigrationStep::AddOfflineRebuildV3,
            ],
            (RecoverySchemaVersion(0), CURRENT_RECOVERY_SCHEMA) => vec![
                RecoveryMigrationStep::InitializeSemanticSchemaV1,
                RecoveryMigrationStep::AddMetadataLossAuditV2,
                RecoveryMigrationStep::AddOfflineRebuildV3,
                RecoveryMigrationStep::AddChecksumBaselineV4,
            ],
            (RecoverySchemaVersion(1), RecoverySchemaVersion(2)) => {
                vec![RecoveryMigrationStep::AddMetadataLossAuditV2]
            }
            (RecoverySchemaVersion(1), RecoverySchemaVersion(3)) => vec![
                RecoveryMigrationStep::AddMetadataLossAuditV2,
                RecoveryMigrationStep::AddOfflineRebuildV3,
            ],
            (RecoverySchemaVersion(1), CURRENT_RECOVERY_SCHEMA) => vec![
                RecoveryMigrationStep::AddMetadataLossAuditV2,
                RecoveryMigrationStep::AddOfflineRebuildV3,
                RecoveryMigrationStep::AddChecksumBaselineV4,
            ],
            (RecoverySchemaVersion(2), RecoverySchemaVersion(3)) => {
                vec![RecoveryMigrationStep::AddOfflineRebuildV3]
            }
            (RecoverySchemaVersion(2), CURRENT_RECOVERY_SCHEMA) => vec![
                RecoveryMigrationStep::AddOfflineRebuildV3,
                RecoveryMigrationStep::AddChecksumBaselineV4,
            ],
            (RecoverySchemaVersion(3), CURRENT_RECOVERY_SCHEMA) => {
                vec![RecoveryMigrationStep::AddChecksumBaselineV4]
            }
            _ => {
                return Err(RecoveryError::UnsupportedSchemaMigration { from, to });
            }
        };
        Ok(Self { from, to, steps })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RecoveryStoreHealth {
    Healthy,
    Missing,
    Corrupt,
    Stale,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryWritePolicy {
    Allowed,
    Blocked(RecoveryStoreHealth),
}

impl RecoveryStoreHealth {
    pub fn write_policy(self) -> RecoveryWritePolicy {
        match self {
            Self::Healthy => RecoveryWritePolicy::Allowed,
            health => RecoveryWritePolicy::Blocked(health),
        }
    }

    pub fn disposition(self) -> RecoveryDisposition {
        match self {
            Self::Healthy => RecoveryDisposition::Proceed,
            Self::Missing | Self::Corrupt => RecoveryDisposition::RebuildFromData,
            Self::Stale => RecoveryDisposition::ReconcileReadOnly,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryDisposition {
    Proceed,
    ReconcileReadOnly,
    RebuildFromData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryCommitObservation {
    Durable,
    Rejected,
    Lost,
    Corrupt,
}

impl RecoveryCommitObservation {
    pub fn is_durable(self) -> bool {
        matches!(self, Self::Durable)
    }

    pub fn disposition(self) -> RecoveryDisposition {
        match self {
            Self::Durable => RecoveryDisposition::Proceed,
            Self::Rejected | Self::Lost => RecoveryDisposition::ReconcileReadOnly,
            Self::Corrupt => RecoveryDisposition::RebuildFromData,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RegionState {
    Clean,
    Dirty { dirty_since: RecoveryGeneration },
    Indeterminate,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DirtyRegionRecord {
    pub region: RegionId,
    pub state: RegionState,
    pub last_clean_fence: Option<FenceCertificate>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ChecksumEvidenceBinding {
    pub extent: ChecksumExtent,
    pub profile: ChecksumProfileId,
    pub set_generation: ChecksumSetGeneration,
    pub topology_epoch: TopologyEpoch,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum IntegrityState {
    Absent,
    Stale {
        stale_generation: RecoveryGeneration,
    },
    Valid {
        binding: ChecksumEvidenceBinding,
        content_generation: RecoveryGeneration,
        durable_fence: StoreFenceRef,
        digest: Vec<u8>,
        verified_at: RecoveryGeneration,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct IntegrityRecord {
    pub extent: IntegrityExtentId,
    pub state: IntegrityState,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct FenceCertificate {
    pub topology_epoch: TopologyEpoch,
    pub fence_domain: FenceDomain,
    pub stores: Vec<StoreFenceRef>,
    pub captured_region_generations: Vec<(RegionId, RecoveryGeneration)>,
    pub captured_integrity_generations: Vec<(IntegrityExtentId, RecoveryGeneration)>,
}

impl FenceCertificate {
    pub fn new(
        topology_epoch: TopologyEpoch,
        fence_domain: FenceDomain,
        stores: Vec<StoreFenceRef>,
        captured_region_generations: Vec<(RegionId, RecoveryGeneration)>,
    ) -> Self {
        Self {
            topology_epoch,
            fence_domain,
            stores,
            captured_region_generations,
            captured_integrity_generations: Vec::new(),
        }
    }

    pub fn with_integrity_extent(
        mut self,
        extent: IntegrityExtentId,
        generation: RecoveryGeneration,
    ) -> Self {
        self.captured_integrity_generations
            .push((extent, generation));
        self
    }

    pub(crate) fn covers_region(&self, region: RegionId, generation: RecoveryGeneration) -> bool {
        self.captured_region_generations
            .iter()
            .any(|(captured_region, captured_generation)| {
                *captured_region == region && *captured_generation >= generation
            })
    }

    pub(crate) fn contains_store_fence(&self, fence: StoreFenceRef) -> bool {
        self.stores.contains(&fence)
    }

    pub(crate) fn covers_integrity_extent(
        &self,
        extent: IntegrityExtentId,
        generation: RecoveryGeneration,
    ) -> bool {
        self.captured_integrity_generations
            .iter()
            .any(|(captured_extent, captured_generation)| {
                *captured_extent == extent && *captured_generation >= generation
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WritableSession {
    pub session_id: SessionId,
    pub topology_epoch: TopologyEpoch,
    pub dirty_envelope_generation: u64,
    pub closed: bool,
    pub global_fence: Option<FenceCertificate>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StoreAssignment {
    assignment: dwv_core::TopologyAssignment,
    store_id: StoreId,
}

impl StoreAssignment {
    pub const fn slot_id(&self) -> dwv_core::SlotId {
        self.assignment.slot_id()
    }

    pub const fn role(&self) -> dwv_core::MemberRole {
        self.assignment.role()
    }

    pub const fn coding_position(&self) -> dwv_core::CodingPosition {
        self.assignment.coding_position()
    }

    pub const fn assignment_instance(&self) -> dwv_core::AssignmentInstanceId {
        self.assignment.assignment_instance()
    }

    pub const fn assignment_generation(&self) -> dwv_core::AssignmentGeneration {
        self.assignment.assignment_generation()
    }

    pub const fn evidence(&self) -> dwv_core::AssignmentEvidence {
        self.assignment.evidence()
    }

    pub const fn store_id(&self) -> StoreId {
        self.store_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TopologySnapshot {
    array_id: dwv_core::ArrayId,
    topology_epoch: TopologyEpoch,
    profile: dwv_core::CodingProfile,
    geometry: dwv_core::ProtectedGeometry,
    assignments: Vec<StoreAssignment>,
}

impl TopologySnapshot {
    /// dwv:req req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch
    pub fn from_core(
        topology: dwv_core::TopologySnapshot,
        store_ids: Vec<StoreId>,
    ) -> Result<Self, RecoveryTopologyError> {
        topology
            .validate()
            .map_err(RecoveryTopologyError::InvalidCoreTopology)?;
        if topology.assignments().len() != store_ids.len() {
            return Err(RecoveryTopologyError::StoreCount {
                expected: topology.assignments().len(),
                actual: store_ids.len(),
            });
        }
        for (index, store_id) in store_ids.iter().enumerate() {
            if store_ids[..index].contains(store_id) {
                return Err(RecoveryTopologyError::DuplicateStore(*store_id));
            }
        }
        let mut assignments = topology
            .assignments()
            .iter()
            .cloned()
            .zip(store_ids)
            .map(|(assignment, store_id)| StoreAssignment {
                assignment,
                store_id,
            })
            .collect::<Vec<_>>();
        assignments.sort_unstable_by(|left, right| {
            left.slot_id().as_bytes().cmp(&right.slot_id().as_bytes())
        });
        Ok(Self {
            array_id: topology.array_id(),
            topology_epoch: topology.topology_epoch(),
            profile: topology.profile(),
            geometry: topology.geometry(),
            assignments,
        })
    }

    pub const fn array_id(&self) -> dwv_core::ArrayId {
        self.array_id
    }

    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch
    }

    pub const fn profile(&self) -> dwv_core::CodingProfile {
        self.profile
    }

    pub const fn geometry(&self) -> dwv_core::ProtectedGeometry {
        self.geometry
    }

    pub fn assignments(&self) -> &[StoreAssignment] {
        &self.assignments
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryTopologyError {
    InvalidCoreTopology(dwv_core::TopologyValidationError),
    StoreCount { expected: usize, actual: usize },
    DuplicateStore(StoreId),
}

impl fmt::Display for RecoveryTopologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCoreTopology(error) => write!(formatter, "invalid core topology: {error}"),
            Self::StoreCount { expected, actual } => {
                write!(
                    formatter,
                    "topology has {expected} assignments but {actual} stores"
                )
            }
            Self::DuplicateStore(store_id) => {
                write!(formatter, "store {store_id:?} appears more than once")
            }
        }
    }
}

impl std::error::Error for RecoveryTopologyError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MaintenanceCheckpoint {
    pub job: JobId,
    pub cursor: RecoveryCursor,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RecoverySnapshot {
    pub generation: RecoveryGeneration,
    pub topology_epoch: TopologyEpoch,
    pub active_topology: Option<TopologySnapshot>,
    pub pending_topology: Option<TopologySnapshot>,
    pub writable_session: Option<WritableSession>,
    pub dirty_regions: Vec<DirtyRegionRecord>,
    pub integrity_records: Vec<IntegrityRecord>,
    pub checksum_baseline: Option<ChecksumBaseline>,
    pub fences: Vec<FenceCertificate>,
    pub maintenance_checkpoints: Vec<MaintenanceCheckpoint>,
    pub metadata_loss_audit: Option<MetadataLossAudit>,
    pub rebuilds: Vec<RebuildState>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RecoveryManifest {
    pub schema: RecoverySchemaVersion,
    pub snapshot: RecoverySnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryExportLimits {
    pub max_dirty_regions: usize,
    pub max_integrity_records: usize,
    pub max_fences: usize,
    pub max_maintenance_checkpoints: usize,
    pub max_topology_assignments: usize,
    pub max_stores_per_fence: usize,
    pub max_region_captures_per_fence: usize,
    pub max_integrity_captures_per_fence: usize,
    pub max_digest_bytes: usize,
    pub max_rebuilds: usize,
}

impl Default for RecoveryExportLimits {
    fn default() -> Self {
        Self {
            max_dirty_regions: 16 * 1024,
            max_integrity_records: 16 * 1024,
            max_fences: 16 * 1024,
            max_maintenance_checkpoints: 1024,
            max_topology_assignments: 1024,
            max_stores_per_fence: 1024,
            max_region_captures_per_fence: 16 * 1024,
            max_integrity_captures_per_fence: 16 * 1024,
            max_digest_bytes: 1024,
            max_rebuilds: 64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryError {
    Unhealthy(RecoveryStoreHealth),
    GenerationExhausted,
    GenerationMismatch {
        expected: RecoveryGeneration,
        actual: RecoveryGeneration,
    },
    TopologyMismatch {
        expected: TopologyEpoch,
        actual: TopologyEpoch,
    },
    InvalidTransition(TransitionError),
    CommitNotDurable(RecoveryCommitObservation),
    ExportGenerationMismatch {
        requested: RecoveryGeneration,
        actual: RecoveryGeneration,
    },
    UnsupportedSchemaMigration {
        from: RecoverySchemaVersion,
        to: RecoverySchemaVersion,
    },
    ExportLimitExceeded(RecoveryRecordKind),
    ChecksumBaseline(ExtentError),
    Rebuild(RebuildError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionError {
    SessionAlreadyOpen,
    SessionMissing,
    SessionIdMismatch,
    RegionNotDirty,
    RegionAlreadyUnknown,
    FenceEmpty,
    FenceTopologyMismatch,
    FenceCoverageMissing,
    IntegrityFenceMissing,
    IntegrityGenerationStale,
    PendingTopologyMissing,
    SessionOpen,
    CheckpointGenerationBehind,
    IntegrityCoverageMissing,
    IntegrityContentGenerationFuture,
    IntegrityBindingMismatch,
}

impl fmt::Display for RecoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unhealthy(health) => write!(formatter, "recovery state is {health:?}"),
            Self::GenerationExhausted => write!(formatter, "recovery generation exhausted"),
            Self::GenerationMismatch { expected, actual } => {
                write!(
                    formatter,
                    "recovery generation mismatch: expected {expected:?}, got {actual:?}"
                )
            }
            Self::TopologyMismatch { expected, actual } => {
                write!(
                    formatter,
                    "topology epoch mismatch: expected {expected:?}, got {actual:?}"
                )
            }
            Self::InvalidTransition(error) => {
                write!(formatter, "invalid recovery transition: {error:?}")
            }
            Self::CommitNotDurable(observation) => {
                write!(
                    formatter,
                    "recovery commit was not durable: {observation:?}"
                )
            }
            Self::ExportGenerationMismatch { requested, actual } => {
                write!(
                    formatter,
                    "manifest generation {requested:?} is not current {actual:?}"
                )
            }
            Self::UnsupportedSchemaMigration { from, to } => {
                write!(
                    formatter,
                    "unsupported recovery schema migration {from:?} -> {to:?}"
                )
            }
            Self::ExportLimitExceeded(kind) => {
                write!(formatter, "recovery export limit exceeded for {kind:?}")
            }
            Self::Rebuild(error) => error.fmt(formatter),
            Self::ChecksumBaseline(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RecoveryError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryMutation {
    BeginWritableSession {
        session_id: SessionId,
        topology_epoch: TopologyEpoch,
        dirty_envelope_generation: u64,
    },
    MarkRegionDirty {
        region: RegionId,
        mutation_generation: RecoveryGeneration,
    },
    MarkIntegrityStale {
        extent: IntegrityExtentId,
        stale_generation: RecoveryGeneration,
    },
    RecordHomeFence {
        fence: FenceCertificate,
    },
    CloseWritableSession {
        session_id: SessionId,
        global_fence: FenceCertificate,
    },
    MarkRegionClean {
        region: RegionId,
        through_generation: RecoveryGeneration,
    },
    InstallIntegrityDigest {
        record: IntegrityRecord,
    },
    PrepareTopology {
        topology: TopologySnapshot,
    },
    CommitTopology {
        topology_epoch: TopologyEpoch,
    },
    RecordMaintenanceCheckpoint {
        job: JobId,
        cursor: RecoveryCursor,
    },
    BeginOfflineRebuild {
        rebuild: RebuildState,
    },
    AdvanceOfflineRebuild {
        receipt: RebuildChunkReceipt,
    },
    CompleteOfflineRebuild {
        receipt: RebuildCompletionReceipt,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryTxn {
    pub expected_generation: RecoveryGeneration,
    pub expected_topology_epoch: TopologyEpoch,
    pub mutations: Vec<RecoveryMutation>,
}

impl RecoveryTxn {
    pub fn new(
        expected_generation: RecoveryGeneration,
        expected_topology_epoch: TopologyEpoch,
    ) -> Self {
        Self {
            expected_generation,
            expected_topology_epoch,
            mutations: Vec::new(),
        }
    }

    pub fn push(&mut self, mutation: RecoveryMutation) -> &mut Self {
        self.mutations.push(mutation);
        self
    }

    pub fn mark_region_dirty(
        &mut self,
        region: RegionId,
        mutation_generation: RecoveryGeneration,
    ) -> &mut Self {
        self.push(RecoveryMutation::MarkRegionDirty {
            region,
            mutation_generation,
        })
    }

    pub fn mark_integrity_stale(
        &mut self,
        extent: IntegrityExtentId,
        stale_generation: RecoveryGeneration,
    ) -> &mut Self {
        self.push(RecoveryMutation::MarkIntegrityStale {
            extent,
            stale_generation,
        })
    }

    pub fn begin_offline_rebuild(&mut self, rebuild: RebuildState) -> &mut Self {
        self.push(RecoveryMutation::BeginOfflineRebuild { rebuild })
    }

    pub fn advance_offline_rebuild(&mut self, receipt: RebuildChunkReceipt) -> &mut Self {
        self.push(RecoveryMutation::AdvanceOfflineRebuild { receipt })
    }

    pub fn complete_offline_rebuild(&mut self, receipt: RebuildCompletionReceipt) -> &mut Self {
        self.push(RecoveryMutation::CompleteOfflineRebuild { receipt })
    }
}

/// dwv:req req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
pub trait RecoveryStateStore {
    fn load_assembly_snapshot(&self) -> Result<RecoverySnapshot, RecoveryError>;
    fn verify_integrity(&self) -> RecoveryStoreHealth;
    fn begin_protocol_txn(
        &self,
        expected: RecoveryGeneration,
        topology_epoch: TopologyEpoch,
    ) -> RecoveryTxn;
    fn commit_durable(&mut self, txn: RecoveryTxn) -> Result<RecoveryGeneration, RecoveryError>;
    fn commit_observed(
        &mut self,
        txn: RecoveryTxn,
        observation: RecoveryCommitObservation,
    ) -> Result<RecoveryGeneration, RecoveryError> {
        if !observation.is_durable() {
            return Err(RecoveryError::CommitNotDurable(observation));
        }
        self.commit_durable(txn)
    }
    fn export_manifest(
        &self,
        generation: RecoveryGeneration,
    ) -> Result<RecoveryManifest, RecoveryError>;
}

#[derive(Clone)]
pub struct MemoryRecoveryStore {
    snapshot: RecoverySnapshot,
    health: RecoveryStoreHealth,
}

impl MemoryRecoveryStore {
    pub fn new(topology_epoch: TopologyEpoch) -> Self {
        Self {
            snapshot: RecoverySnapshot {
                generation: RecoveryGeneration(0),
                topology_epoch,
                active_topology: None,
                pending_topology: None,
                writable_session: None,
                dirty_regions: Vec::new(),
                integrity_records: Vec::new(),
                checksum_baseline: None,
                fences: Vec::new(),
                maintenance_checkpoints: Vec::new(),
                metadata_loss_audit: None,
                rebuilds: Vec::new(),
            },
            health: RecoveryStoreHealth::Healthy,
        }
    }

    pub fn snapshot(&self) -> &RecoverySnapshot {
        &self.snapshot
    }

    pub(crate) fn with_active_topology(topology: TopologySnapshot) -> Self {
        let mut store = Self::new(topology.topology_epoch());
        store.snapshot.active_topology = Some(topology);
        store
    }

    pub(crate) fn set_checksum_baseline(&mut self, baseline: ChecksumBaseline) {
        self.snapshot.checksum_baseline = Some(baseline);
    }
    pub(crate) fn set_metadata_loss_audit(&mut self, audit: MetadataLossAudit) {
        self.snapshot.metadata_loss_audit = Some(audit);
    }

    pub fn set_health(&mut self, health: RecoveryStoreHealth) {
        self.health = health;
    }

    pub fn health(&self) -> RecoveryStoreHealth {
        self.health
    }

    pub fn write_policy(&self) -> RecoveryWritePolicy {
        self.health.write_policy()
    }

    pub fn disposition(&self) -> RecoveryDisposition {
        self.health.disposition()
    }

    fn apply_mutation(
        snapshot: &mut RecoverySnapshot,
        mutation: RecoveryMutation,
        expected_topology_epoch: TopologyEpoch,
    ) -> Result<(), RecoveryError> {
        match mutation {
            RecoveryMutation::BeginWritableSession {
                session_id,
                topology_epoch,
                dirty_envelope_generation,
            } => {
                if topology_epoch != expected_topology_epoch {
                    return Err(RecoveryError::TopologyMismatch {
                        expected: expected_topology_epoch,
                        actual: topology_epoch,
                    });
                }
                if snapshot
                    .writable_session
                    .as_ref()
                    .is_some_and(|session| !session.closed)
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::SessionAlreadyOpen,
                    ));
                }
                snapshot.writable_session = Some(WritableSession {
                    session_id,
                    topology_epoch,
                    dirty_envelope_generation,
                    closed: false,
                    global_fence: None,
                });
            }
            RecoveryMutation::MarkRegionDirty {
                region,
                mutation_generation,
            } => {
                upsert_region(
                    snapshot,
                    region,
                    RegionState::Dirty {
                        dirty_since: mutation_generation,
                    },
                );
            }
            RecoveryMutation::MarkIntegrityStale {
                extent,
                stale_generation,
            } => {
                let record = snapshot
                    .integrity_records
                    .iter_mut()
                    .find(|record| record.extent == extent);
                match record {
                    Some(record) => {
                        record.state = match record.state {
                            IntegrityState::Stale {
                                stale_generation: current,
                            } => IntegrityState::Stale {
                                stale_generation: current.max(stale_generation),
                            },
                            _ => IntegrityState::Stale { stale_generation },
                        };
                    }
                    None => snapshot.integrity_records.push(IntegrityRecord {
                        extent,
                        state: IntegrityState::Stale { stale_generation },
                    }),
                }
            }
            RecoveryMutation::RecordHomeFence { fence } => {
                validate_fence(&fence, expected_topology_epoch)?;
                if fence.stores.is_empty() {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::FenceEmpty,
                    ));
                }
                snapshot.fences.push(fence);
            }
            RecoveryMutation::CloseWritableSession {
                session_id,
                global_fence,
            } => {
                validate_fence(&global_fence, expected_topology_epoch)?;
                let session =
                    snapshot
                        .writable_session
                        .as_mut()
                        .ok_or(RecoveryError::InvalidTransition(
                            TransitionError::SessionMissing,
                        ))?;
                if session.session_id != session_id {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::SessionIdMismatch,
                    ));
                }
                snapshot.fences.push(global_fence.clone());
                session.closed = true;
                session.global_fence = Some(global_fence);
            }
            RecoveryMutation::MarkRegionClean {
                region,
                through_generation,
            } => {
                let record = snapshot
                    .dirty_regions
                    .iter_mut()
                    .find(|record| record.region == region)
                    .ok_or(RecoveryError::InvalidTransition(
                        TransitionError::RegionNotDirty,
                    ))?;
                let dirty_since = match record.state {
                    RegionState::Dirty { dirty_since } => dirty_since,
                    RegionState::Clean | RegionState::Indeterminate => {
                        return Err(RecoveryError::InvalidTransition(
                            TransitionError::RegionNotDirty,
                        ));
                    }
                };
                let fence = snapshot
                    .fences
                    .iter()
                    .find(|fence| {
                        fence.topology_epoch == expected_topology_epoch
                            && fence.covers_region(region, dirty_since)
                    })
                    .cloned()
                    .ok_or(RecoveryError::InvalidTransition(
                        TransitionError::FenceCoverageMissing,
                    ))?;
                if through_generation < dirty_since {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::CheckpointGenerationBehind,
                    ));
                }
                record.state = RegionState::Clean;
                record.last_clean_fence = Some(fence);
            }
            RecoveryMutation::InstallIntegrityDigest { record } => {
                let (binding, content_generation, durable_fence, digest, verified_at) =
                    match record.state {
                        IntegrityState::Valid {
                            binding,
                            content_generation,
                            durable_fence,
                            digest,
                            verified_at,
                        } => (
                            binding,
                            content_generation,
                            durable_fence,
                            digest,
                            verified_at,
                        ),
                        _ => {
                            return Err(RecoveryError::InvalidTransition(
                                TransitionError::IntegrityFenceMissing,
                            ));
                        }
                    };
                if binding.extent.id != record.extent
                    || binding.topology_epoch != expected_topology_epoch
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::IntegrityBindingMismatch,
                    ));
                }
                validate_store_fence(&durable_fence, expected_topology_epoch)?;
                let has_fence = snapshot
                    .fences
                    .iter()
                    .any(|certificate| certificate.contains_store_fence(durable_fence));
                if !has_fence {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::IntegrityFenceMissing,
                    ));
                }
                let has_coverage = snapshot.fences.iter().any(|certificate| {
                    certificate.contains_store_fence(durable_fence)
                        && certificate.covers_integrity_extent(record.extent, content_generation)
                });
                if !has_coverage {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::IntegrityCoverageMissing,
                    ));
                }
                let next_generation = snapshot
                    .generation
                    .0
                    .checked_add(1)
                    .map(RecoveryGeneration)
                    .ok_or(RecoveryError::GenerationExhausted)?;
                if content_generation > next_generation || verified_at > next_generation {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::IntegrityContentGenerationFuture,
                    ));
                }
                if let Some(existing) = snapshot
                    .integrity_records
                    .iter()
                    .find(|existing| existing.extent == record.extent)
                    && let IntegrityState::Stale { stale_generation } = existing.state
                    && content_generation < stale_generation
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::IntegrityGenerationStale,
                    ));
                }
                let target = snapshot
                    .integrity_records
                    .iter_mut()
                    .find(|existing| existing.extent == record.extent);
                let value = IntegrityRecord {
                    extent: record.extent,
                    state: IntegrityState::Valid {
                        binding,
                        content_generation,
                        durable_fence,
                        digest,
                        verified_at,
                    },
                };
                match target {
                    Some(target) => *target = value,
                    None => snapshot.integrity_records.push(value),
                }
            }
            RecoveryMutation::PrepareTopology { topology } => {
                if topology.topology_epoch() <= snapshot.topology_epoch {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::FenceTopologyMismatch,
                    ));
                }
                snapshot.pending_topology = Some(topology);
            }
            RecoveryMutation::CommitTopology { topology_epoch } => {
                let pending =
                    snapshot
                        .pending_topology
                        .as_ref()
                        .ok_or(RecoveryError::InvalidTransition(
                            TransitionError::PendingTopologyMissing,
                        ))?;
                if pending.topology_epoch() != topology_epoch {
                    return Err(RecoveryError::TopologyMismatch {
                        expected: topology_epoch,
                        actual: pending.topology_epoch(),
                    });
                }
                if snapshot
                    .writable_session
                    .as_ref()
                    .is_some_and(|session| !session.closed)
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::SessionOpen,
                    ));
                }
                snapshot.active_topology = snapshot.pending_topology.take();
                snapshot.topology_epoch = topology_epoch;
            }
            RecoveryMutation::RecordMaintenanceCheckpoint { job, cursor } => {
                if let Some(checkpoint) = snapshot
                    .maintenance_checkpoints
                    .iter_mut()
                    .find(|checkpoint| checkpoint.job == job)
                {
                    checkpoint.cursor = cursor;
                } else {
                    snapshot
                        .maintenance_checkpoints
                        .push(MaintenanceCheckpoint { job, cursor });
                }
            }
            RecoveryMutation::BeginOfflineRebuild { rebuild } => {
                if snapshot
                    .rebuilds
                    .iter()
                    .any(|existing| existing.id() == rebuild.id())
                {
                    return Err(RecoveryError::Rebuild(RebuildError::DuplicateRebuildId));
                }
                rebuild
                    .validate_source(snapshot.active_topology.as_ref(), snapshot.generation)
                    .map_err(RecoveryError::Rebuild)?;
                snapshot.rebuilds.push(rebuild);
            }
            RecoveryMutation::AdvanceOfflineRebuild { receipt } => {
                let rebuild = snapshot
                    .rebuilds
                    .iter_mut()
                    .find(|rebuild| rebuild.id() == receipt.rebuild_id())
                    .ok_or(RecoveryError::Rebuild(RebuildError::RebuildNotFound))?;
                rebuild
                    .apply_chunk_receipt(receipt, snapshot.generation)
                    .map_err(RecoveryError::Rebuild)?;
            }
            RecoveryMutation::CompleteOfflineRebuild { receipt } => {
                let rebuild = snapshot
                    .rebuilds
                    .iter_mut()
                    .find(|rebuild| rebuild.id() == receipt.rebuild_id())
                    .ok_or(RecoveryError::Rebuild(RebuildError::RebuildNotFound))?;
                rebuild
                    .apply_completion_receipt(receipt, snapshot.generation)
                    .map_err(RecoveryError::Rebuild)?;
            }
        }
        Ok(())
    }
}

impl RecoveryStateStore for MemoryRecoveryStore {
    fn load_assembly_snapshot(&self) -> Result<RecoverySnapshot, RecoveryError> {
        if self.health != RecoveryStoreHealth::Healthy {
            return Err(RecoveryError::Unhealthy(self.health));
        }
        Ok(self.snapshot.clone())
    }

    fn verify_integrity(&self) -> RecoveryStoreHealth {
        self.health
    }

    fn begin_protocol_txn(
        &self,
        expected: RecoveryGeneration,
        topology_epoch: TopologyEpoch,
    ) -> RecoveryTxn {
        RecoveryTxn::new(expected, topology_epoch)
    }

    fn commit_durable(&mut self, txn: RecoveryTxn) -> Result<RecoveryGeneration, RecoveryError> {
        if self.health != RecoveryStoreHealth::Healthy {
            return Err(RecoveryError::Unhealthy(self.health));
        }
        if txn.expected_generation != self.snapshot.generation {
            return Err(RecoveryError::GenerationMismatch {
                expected: txn.expected_generation,
                actual: self.snapshot.generation,
            });
        }
        if txn.expected_topology_epoch != self.snapshot.topology_epoch {
            return Err(RecoveryError::TopologyMismatch {
                expected: txn.expected_topology_epoch,
                actual: self.snapshot.topology_epoch,
            });
        }

        let mut candidate = self.snapshot.clone();
        for mutation in txn.mutations {
            Self::apply_mutation(&mut candidate, mutation, txn.expected_topology_epoch)?;
        }
        candidate.generation = candidate
            .generation
            .checked_next()
            .ok_or(RecoveryError::GenerationExhausted)?;
        self.snapshot = candidate;
        Ok(self.snapshot.generation)
    }

    fn export_manifest(
        &self,
        generation: RecoveryGeneration,
    ) -> Result<RecoveryManifest, RecoveryError> {
        self.export_manifest_with_limits(generation, RecoveryExportLimits::default())
    }
}

impl MemoryRecoveryStore {
    pub fn from_manifest(manifest: RecoveryManifest) -> Result<Self, RecoveryError> {
        if manifest.schema != CURRENT_RECOVERY_SCHEMA {
            return Err(RecoveryError::UnsupportedSchemaMigration {
                from: manifest.schema,
                to: CURRENT_RECOVERY_SCHEMA,
            });
        }
        if manifest.snapshot.topology_epoch
            != manifest.snapshot.active_topology.as_ref().map_or(
                manifest.snapshot.topology_epoch,
                TopologySnapshot::topology_epoch,
            )
        {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceTopologyMismatch,
            ));
        }
        for (index, rebuild) in manifest.snapshot.rebuilds.iter().enumerate() {
            if manifest.snapshot.rebuilds[..index]
                .iter()
                .any(|prior| prior.id() == rebuild.id())
            {
                return Err(RecoveryError::Rebuild(RebuildError::DuplicateRebuildId));
            }
            rebuild
                .validate_restored(
                    manifest.snapshot.active_topology.as_ref(),
                    manifest.snapshot.generation,
                )
                .map_err(RecoveryError::Rebuild)?;
        }
        validate_export_limits(&manifest.snapshot, RecoveryExportLimits::default())?;
        Ok(Self {
            snapshot: manifest.snapshot,
            health: RecoveryStoreHealth::Healthy,
        })
    }

    pub fn export_manifest_with_limits(
        &self,
        generation: RecoveryGeneration,
        limits: RecoveryExportLimits,
    ) -> Result<RecoveryManifest, RecoveryError> {
        if self.health != RecoveryStoreHealth::Healthy {
            return Err(RecoveryError::Unhealthy(self.health));
        }
        if generation != self.snapshot.generation {
            return Err(RecoveryError::ExportGenerationMismatch {
                requested: generation,
                actual: self.snapshot.generation,
            });
        }
        validate_export_limits(&self.snapshot, limits)?;
        Ok(RecoveryManifest {
            schema: CURRENT_RECOVERY_SCHEMA,
            snapshot: self.snapshot.clone(),
        })
    }
}

fn upsert_region(snapshot: &mut RecoverySnapshot, region: RegionId, state: RegionState) {
    if let Some(record) = snapshot
        .dirty_regions
        .iter_mut()
        .find(|record| record.region == region)
    {
        record.state = state;
    } else {
        snapshot.dirty_regions.push(DirtyRegionRecord {
            region,
            state,
            last_clean_fence: None,
        });
    }
}

fn validate_export_limits(
    snapshot: &RecoverySnapshot,
    limits: RecoveryExportLimits,
) -> Result<(), RecoveryError> {
    if snapshot.dirty_regions.len() > limits.max_dirty_regions {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::DirtyRegion,
        ));
    }
    if snapshot.integrity_records.len() > limits.max_integrity_records {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::IntegrityEvidence,
        ));
    }
    if snapshot
        .checksum_baseline
        .as_ref()
        .is_some_and(|baseline| baseline.expected_extents.len() > limits.max_integrity_records)
    {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::ChecksumBaseline,
        ));
    }
    if snapshot.fences.len() > limits.max_fences {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::StoreFence,
        ));
    }
    for fence in &snapshot.fences {
        if fence.stores.len() > limits.max_stores_per_fence
            || fence.captured_region_generations.len() > limits.max_region_captures_per_fence
            || fence.captured_integrity_generations.len() > limits.max_integrity_captures_per_fence
        {
            return Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::StoreFence,
            ));
        }
    }
    if snapshot.maintenance_checkpoints.len() > limits.max_maintenance_checkpoints {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::MaintenanceCheckpoint,
        ));
    }
    if snapshot.rebuilds.len() > limits.max_rebuilds {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::OfflineRebuild,
        ));
    }
    for topology in [
        snapshot.active_topology.as_ref(),
        snapshot.pending_topology.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if topology.assignments().len() > limits.max_topology_assignments {
            return Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::Topology,
            ));
        }
    }
    for rebuild in &snapshot.rebuilds {
        if rebuild.source_topology().assignments().len() > limits.max_topology_assignments {
            return Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::OfflineRebuild,
            ));
        }
    }
    for record in &snapshot.integrity_records {
        if let IntegrityState::Valid { digest, .. } = &record.state
            && digest.len() > limits.max_digest_bytes
        {
            return Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::IntegrityEvidence,
            ));
        }
    }
    Ok(())
}

fn validate_store_fence(
    fence: &StoreFenceRef,
    expected_topology_epoch: TopologyEpoch,
) -> Result<(), RecoveryError> {
    if fence.topology_epoch != expected_topology_epoch {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceTopologyMismatch,
        ));
    }
    Ok(())
}

fn validate_fence(
    certificate: &FenceCertificate,
    expected_topology_epoch: TopologyEpoch,
) -> Result<(), RecoveryError> {
    if certificate.topology_epoch != expected_topology_epoch {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceTopologyMismatch,
        ));
    }
    if certificate.stores.is_empty() {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceEmpty,
        ));
    }
    for fence in &certificate.stores {
        validate_store_fence(fence, expected_topology_epoch)?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryResetBoundary {
    Process,
    VirtualMachine,
    PowerLoss,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryFaultPoint {
    BeforeDirtyIntentCommit,
    AfterDirtyIntentBeforeHomeMutation,
    AfterHomeMutationBeforeFence,
    AfterFenceBeforeCheckpoint,
    MissingState,
    CorruptState,
    CorruptJournal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverySimulationCase {
    pub case_id: &'static str,
    pub reset: RecoveryResetBoundary,
    pub fault: RecoveryFaultPoint,
    pub observation: RecoveryCommitObservation,
    pub expected_health: RecoveryStoreHealth,
    pub expected_disposition: RecoveryDisposition,
    pub permits_home_mutation: bool,
}

pub fn recovery_simulation_cases() -> Vec<RecoverySimulationCase> {
    vec![
        RecoverySimulationCase {
            case_id: "intent-commit-rejected-before-home",
            reset: RecoveryResetBoundary::Process,
            fault: RecoveryFaultPoint::BeforeDirtyIntentCommit,
            observation: RecoveryCommitObservation::Rejected,
            expected_health: RecoveryStoreHealth::Healthy,
            expected_disposition: RecoveryDisposition::ReconcileReadOnly,
            permits_home_mutation: false,
        },
        RecoverySimulationCase {
            case_id: "intent-ack-lost-before-home",
            reset: RecoveryResetBoundary::PowerLoss,
            fault: RecoveryFaultPoint::BeforeDirtyIntentCommit,
            observation: RecoveryCommitObservation::Lost,
            expected_health: RecoveryStoreHealth::Stale,
            expected_disposition: RecoveryDisposition::ReconcileReadOnly,
            permits_home_mutation: false,
        },
        RecoverySimulationCase {
            case_id: "home-write-before-fence",
            reset: RecoveryResetBoundary::PowerLoss,
            fault: RecoveryFaultPoint::AfterHomeMutationBeforeFence,
            observation: RecoveryCommitObservation::Lost,
            expected_health: RecoveryStoreHealth::Stale,
            expected_disposition: RecoveryDisposition::ReconcileReadOnly,
            permits_home_mutation: false,
        },
        RecoverySimulationCase {
            case_id: "fence-before-checkpoint",
            reset: RecoveryResetBoundary::Process,
            fault: RecoveryFaultPoint::AfterFenceBeforeCheckpoint,
            observation: RecoveryCommitObservation::Lost,
            expected_health: RecoveryStoreHealth::Stale,
            expected_disposition: RecoveryDisposition::ReconcileReadOnly,
            permits_home_mutation: false,
        },
        RecoverySimulationCase {
            case_id: "missing-state-after-reset",
            reset: RecoveryResetBoundary::VirtualMachine,
            fault: RecoveryFaultPoint::MissingState,
            observation: RecoveryCommitObservation::Corrupt,
            expected_health: RecoveryStoreHealth::Missing,
            expected_disposition: RecoveryDisposition::RebuildFromData,
            permits_home_mutation: false,
        },
        RecoverySimulationCase {
            case_id: "corrupt-main-state-after-reset",
            reset: RecoveryResetBoundary::VirtualMachine,
            fault: RecoveryFaultPoint::CorruptState,
            observation: RecoveryCommitObservation::Corrupt,
            expected_health: RecoveryStoreHealth::Corrupt,
            expected_disposition: RecoveryDisposition::RebuildFromData,
            permits_home_mutation: false,
        },
        RecoverySimulationCase {
            case_id: "corrupt-journal-after-power-loss",
            reset: RecoveryResetBoundary::PowerLoss,
            fault: RecoveryFaultPoint::CorruptJournal,
            observation: RecoveryCommitObservation::Corrupt,
            expected_health: RecoveryStoreHealth::Corrupt,
            expected_disposition: RecoveryDisposition::RebuildFromData,
            permits_home_mutation: false,
        },
        RecoverySimulationCase {
            case_id: "durable-intent-after-process-reset",
            reset: RecoveryResetBoundary::Process,
            fault: RecoveryFaultPoint::AfterDirtyIntentBeforeHomeMutation,
            observation: RecoveryCommitObservation::Durable,
            expected_health: RecoveryStoreHealth::Healthy,
            expected_disposition: RecoveryDisposition::Proceed,
            permits_home_mutation: true,
        },
    ]
}

#[cfg(kani)]
mod kani_verification {
    use super::*;
    use dwv_core::ByteRange;

    #[kani::proof]
    #[kani::unwind(5)]
    fn dirty_region_mapping_covers_every_intersection_once() {
        const REGION_BYTES: u64 = 4096;
        let member: u8 = kani::any();
        let start: u8 = kani::any();
        let length: u8 = kani::any();
        kani::assume(member < 4);
        kani::assume(start < 4);
        kani::assume(length > 0 && u16::from(start) + u16::from(length) <= 4);

        let regions = dirty_regions_for_range(
            u32::from(member),
            ByteRange::new(
                u64::from(start) * REGION_BYTES,
                u64::from(length) * REGION_BYTES,
            )
            .unwrap(),
            REGION_BYTES,
        )
        .unwrap();
        assert_eq!(regions.len(), usize::from(length));
        for (index, region) in regions.iter().enumerate() {
            assert_eq!(
                *region,
                RegionId(
                    (u64::from(member) << 32) | (u64::from(start) + u64::try_from(index).unwrap())
                )
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_core::ByteRange;

    fn store() -> MemoryRecoveryStore {
        MemoryRecoveryStore::new(TopologyEpoch(1))
    }

    #[test]
    fn dirty_region_mapping_is_complete_unique_and_checked() {
        assert_eq!(
            dirty_regions_for_range(7, ByteRange::new(4095, 4098).unwrap(), 4096).unwrap(),
            vec![
                RegionId(7_u64 << 32),
                RegionId((7_u64 << 32) | 1),
                RegionId((7_u64 << 32) | 2),
            ]
        );
        assert!(matches!(
            dirty_regions_for_range(0, ByteRange::empty(), 4096),
            Err(RegionMappingError::EmptyRange)
        ));
        assert!(matches!(
            dirty_regions_for_range(0, ByteRange::new(u64::MAX - 1, 1).unwrap(), 1),
            Err(RegionMappingError::RegionIdOverflow)
        ));
        assert_ne!(
            dirty_regions_for_range(1, ByteRange::new(0, 1).unwrap(), 4096).unwrap(),
            dirty_regions_for_range(2, ByteRange::new(0, 1).unwrap(), 4096).unwrap()
        );
    }

    fn fence(region: RegionId, generation: RecoveryGeneration) -> FenceCertificate {
        FenceCertificate::new(
            TopologyEpoch(1),
            FenceDomain(1),
            vec![StoreFenceRef {
                fence_id: dwv_store::FenceId(1),
                store_id: StoreId(2),
                store_incarnation: dwv_store::StoreIncarnationId(0),
                topology_epoch: TopologyEpoch(1),
                through: dwv_store::StoreWriteWatermark(10),
                capability_evidence_id: dwv_store::CapabilityEvidenceId(3),
            }],
            vec![(region, generation)],
        )
        .with_integrity_extent(IntegrityExtentId(9), generation)
    }

    #[test]
    fn generation_checked_commit_is_atomic() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        transaction.push(RecoveryMutation::MarkIntegrityStale {
            extent: IntegrityExtentId(1),
            stale_generation: RecoveryGeneration(1),
        });
        assert_eq!(
            recovery.commit_durable(transaction).unwrap(),
            RecoveryGeneration(1)
        );

        let before = recovery.snapshot().clone();
        let mut stale = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        stale.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(2),
            mutation_generation: RecoveryGeneration(2),
        });
        assert!(matches!(
            recovery.commit_durable(stale),
            Err(RecoveryError::GenerationMismatch { .. })
        ));
        assert_eq!(&before, recovery.snapshot());
    }

    #[test]
    fn dirty_region_cannot_be_clean_without_covering_fence() {
        let mut recovery = store();
        let mut dirty = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        dirty.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(7),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(dirty).unwrap();

        let mut clean = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        clean.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(7),
            through_generation: RecoveryGeneration(2),
        });
        assert!(matches!(
            recovery.commit_durable(clean),
            Err(RecoveryError::InvalidTransition(
                TransitionError::FenceCoverageMissing
            ))
        ));
        assert!(matches!(
            recovery.snapshot().dirty_regions[0].state,
            RegionState::Dirty { .. }
        ));
    }

    #[test]
    fn fence_allows_clean_checkpoint_and_valid_digest() {
        let mut recovery = store();
        let mut intent = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        intent.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(7),
            mutation_generation: RecoveryGeneration(1),
        });
        intent.push(RecoveryMutation::MarkIntegrityStale {
            extent: IntegrityExtentId(9),
            stale_generation: RecoveryGeneration(1),
        });
        intent.push(RecoveryMutation::RecordHomeFence {
            fence: fence(RegionId(7), RecoveryGeneration(1)),
        });
        recovery.commit_durable(intent).unwrap();

        let store_fence = recovery.snapshot().fences[0].stores[0];
        let mut checkpoint = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        checkpoint.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(7),
            through_generation: RecoveryGeneration(2),
        });
        checkpoint.push(RecoveryMutation::InstallIntegrityDigest {
            record: IntegrityRecord {
                extent: IntegrityExtentId(9),
                state: IntegrityState::Valid {
                    binding: ChecksumEvidenceBinding {
                        extent: ChecksumExtent {
                            id: IntegrityExtentId(9),
                            target: ChecksumTarget::data(dwv_core::SlotId([1; 16])),
                            range: dwv_core::ByteRange::new(0, 1).unwrap(),
                        },
                        profile: BLAKE3_256_PROFILE.id,
                        set_generation: ChecksumSetGeneration::INITIAL,
                        topology_epoch: TopologyEpoch(1),
                    },
                    content_generation: RecoveryGeneration(1),
                    durable_fence: store_fence,
                    digest: vec![1, 2, 3],
                    verified_at: RecoveryGeneration(2),
                },
            },
        });
        recovery.commit_durable(checkpoint).unwrap();
        assert_eq!(
            recovery.snapshot().dirty_regions[0].state,
            RegionState::Clean
        );
        assert!(matches!(
            recovery.snapshot().integrity_records[0].state,
            IntegrityState::Valid { .. }
        ));
    }

    #[test]
    fn topology_prepare_and_commit_change_epoch_explicitly() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        let core_topology = dwv_core::TopologySnapshot::new(
            dwv_core::ArrayId([1; 16]),
            TopologyEpoch(2),
            dwv_core::CodingProfile::new(1, 1).unwrap(),
            dwv_core::ProtectedGeometry::new(64, 1).unwrap(),
            vec![
                dwv_core::TopologyAssignment::new(
                    dwv_core::SlotId([1; 16]),
                    dwv_core::MemberRole::Data,
                    dwv_core::CodingPosition(0),
                    dwv_core::AssignmentInstanceId([11; 16]),
                    dwv_core::AssignmentGeneration(4),
                ),
                dwv_core::TopologyAssignment::new(
                    dwv_core::SlotId([2; 16]),
                    dwv_core::MemberRole::Parity,
                    dwv_core::CodingPosition(1),
                    dwv_core::AssignmentInstanceId([12; 16]),
                    dwv_core::AssignmentGeneration(4),
                ),
            ],
        )
        .unwrap();
        transaction.push(RecoveryMutation::PrepareTopology {
            topology: TopologySnapshot::from_core(core_topology, vec![StoreId(2), StoreId(3)])
                .unwrap(),
        });
        transaction.push(RecoveryMutation::CommitTopology {
            topology_epoch: TopologyEpoch(2),
        });
        recovery.commit_durable(transaction).unwrap();
        assert_eq!(recovery.snapshot().topology_epoch, TopologyEpoch(2));
        assert_eq!(
            recovery
                .snapshot()
                .active_topology
                .as_ref()
                .unwrap()
                .topology_epoch(),
            TopologyEpoch(2)
        );

        let stale_epoch = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        assert!(matches!(
            recovery.commit_durable(stale_epoch),
            Err(RecoveryError::TopologyMismatch {
                expected: TopologyEpoch(1),
                actual: TopologyEpoch(2),
            })
        ));
    }

    #[test]
    fn open_session_blocks_topology_publication_and_close_records_fence() {
        let mut recovery = store();
        let mut begin = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        begin.push(RecoveryMutation::BeginWritableSession {
            session_id: SessionId(8),
            topology_epoch: TopologyEpoch(1),
            dirty_envelope_generation: 3,
        });
        recovery.commit_durable(begin).unwrap();

        let mut close = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        close.push(RecoveryMutation::CloseWritableSession {
            session_id: SessionId(8),
            global_fence: fence(RegionId(7), RecoveryGeneration(1)),
        });
        recovery.commit_durable(close).unwrap();
        assert!(
            recovery
                .snapshot()
                .writable_session
                .as_ref()
                .unwrap()
                .closed
        );
        assert_eq!(recovery.snapshot().fences.len(), 1);
    }

    #[test]
    fn semantic_export_and_health_are_conservative() {
        let mut recovery = store();
        let manifest = recovery.export_manifest(RecoveryGeneration(0)).unwrap();
        assert_eq!(manifest.snapshot, *recovery.snapshot());
        recovery.set_health(RecoveryStoreHealth::Corrupt);
        assert_eq!(recovery.verify_integrity(), RecoveryStoreHealth::Corrupt);
        assert!(matches!(
            recovery.load_assembly_snapshot(),
            Err(RecoveryError::Unhealthy(RecoveryStoreHealth::Corrupt))
        ));
        assert!(matches!(
            recovery.export_manifest(RecoveryGeneration(0)),
            Err(RecoveryError::Unhealthy(RecoveryStoreHealth::Corrupt))
        ));
    }

    #[test]
    fn semantic_schema_migration_and_export_are_versioned_and_bounded() {
        assert_eq!(current_recovery_schema().version, CURRENT_RECOVERY_SCHEMA);
        assert_eq!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(0), CURRENT_RECOVERY_SCHEMA)
                .unwrap()
                .steps,
            vec![
                RecoveryMigrationStep::InitializeSemanticSchemaV1,
                RecoveryMigrationStep::AddMetadataLossAuditV2,
                RecoveryMigrationStep::AddOfflineRebuildV3,
                RecoveryMigrationStep::AddChecksumBaselineV4,
            ]
        );
        assert_eq!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(1), CURRENT_RECOVERY_SCHEMA)
                .unwrap()
                .steps,
            vec![
                RecoveryMigrationStep::AddMetadataLossAuditV2,
                RecoveryMigrationStep::AddOfflineRebuildV3,
                RecoveryMigrationStep::AddChecksumBaselineV4,
            ]
        );
        assert_eq!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(2), CURRENT_RECOVERY_SCHEMA)
                .unwrap()
                .steps,
            vec![
                RecoveryMigrationStep::AddOfflineRebuildV3,
                RecoveryMigrationStep::AddChecksumBaselineV4,
            ]
        );
        assert_eq!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(3), CURRENT_RECOVERY_SCHEMA)
                .unwrap()
                .steps,
            vec![RecoveryMigrationStep::AddChecksumBaselineV4]
        );
        assert!(
            current_recovery_schema()
                .records
                .contains(&RecoveryRecordKind::MetadataLossAudit)
        );
        assert!(
            current_recovery_schema()
                .records
                .contains(&RecoveryRecordKind::ChecksumBaseline)
        );
        assert!(matches!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(7), CURRENT_RECOVERY_SCHEMA),
            Err(RecoveryError::UnsupportedSchemaMigration { .. })
        ));
        assert!(matches!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(7), RecoverySchemaVersion(7)),
            Err(RecoveryError::UnsupportedSchemaMigration { .. })
        ));

        let recovery = store();
        let manifest = recovery.export_manifest(RecoveryGeneration(0)).unwrap();
        assert_eq!(manifest.schema, CURRENT_RECOVERY_SCHEMA);
        assert!(
            recovery
                .export_manifest_with_limits(
                    RecoveryGeneration(0),
                    RecoveryExportLimits {
                        max_dirty_regions: 0,
                        ..RecoveryExportLimits::default()
                    }
                )
                .is_ok()
        );
    }

    #[test]
    fn export_limits_reject_unbounded_semantic_state() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();
        assert!(matches!(
            recovery.export_manifest_with_limits(
                RecoveryGeneration(1),
                RecoveryExportLimits {
                    max_dirty_regions: 0,
                    ..RecoveryExportLimits::default()
                }
            ),
            Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::DirtyRegion
            ))
        ));
    }

    #[test]
    fn non_durable_commit_observation_has_no_side_effect() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(4),
            mutation_generation: RecoveryGeneration(1),
        });
        assert!(matches!(
            recovery.commit_observed(transaction, RecoveryCommitObservation::Lost),
            Err(RecoveryError::CommitNotDurable(
                RecoveryCommitObservation::Lost
            ))
        ));
        assert_eq!(recovery.snapshot().generation, RecoveryGeneration(0));
        assert!(recovery.snapshot().dirty_regions.is_empty());
    }

    #[test]
    fn missing_and_corrupt_state_are_not_writable() {
        let mut recovery = store();
        recovery.set_health(RecoveryStoreHealth::Missing);
        assert_eq!(
            recovery.write_policy(),
            RecoveryWritePolicy::Blocked(RecoveryStoreHealth::Missing)
        );
        assert_eq!(recovery.disposition(), RecoveryDisposition::RebuildFromData);
        recovery.set_health(RecoveryStoreHealth::Corrupt);
        assert_eq!(recovery.disposition(), RecoveryDisposition::RebuildFromData);
    }

    #[test]
    fn recovery_simulation_fixtures_cover_conservative_failures() {
        let cases = recovery_simulation_cases();
        assert!(cases.iter().any(|case| {
            case.fault == RecoveryFaultPoint::AfterHomeMutationBeforeFence
                && !case.permits_home_mutation
        }));
        assert!(cases.iter().any(|case| {
            case.fault == RecoveryFaultPoint::CorruptJournal
                && case.expected_health == RecoveryStoreHealth::Corrupt
        }));
    }

    #[test]
    fn integrity_digest_requires_extent_coverage_and_current_generation() {
        let mut recovery = store();
        let mut intent = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        intent.push(RecoveryMutation::MarkIntegrityStale {
            extent: IntegrityExtentId(9),
            stale_generation: RecoveryGeneration(1),
        });
        intent.push(RecoveryMutation::RecordHomeFence {
            fence: FenceCertificate::new(
                TopologyEpoch(1),
                FenceDomain(1),
                vec![StoreFenceRef {
                    fence_id: dwv_store::FenceId(1),
                    store_id: StoreId(2),
                    store_incarnation: dwv_store::StoreIncarnationId(0),
                    topology_epoch: TopologyEpoch(1),
                    through: dwv_store::StoreWriteWatermark(10),
                    capability_evidence_id: dwv_store::CapabilityEvidenceId(3),
                }],
                vec![],
            ),
        });
        recovery.commit_durable(intent).unwrap();
        let store_fence = recovery.snapshot().fences[0].stores[0];
        let mut digest = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        digest.push(RecoveryMutation::InstallIntegrityDigest {
            record: IntegrityRecord {
                extent: IntegrityExtentId(9),
                state: IntegrityState::Valid {
                    binding: ChecksumEvidenceBinding {
                        extent: ChecksumExtent {
                            id: IntegrityExtentId(9),
                            target: ChecksumTarget::data(dwv_core::SlotId([1; 16])),
                            range: dwv_core::ByteRange::new(0, 1).unwrap(),
                        },
                        profile: BLAKE3_256_PROFILE.id,
                        set_generation: ChecksumSetGeneration::INITIAL,
                        topology_epoch: TopologyEpoch(1),
                    },
                    content_generation: RecoveryGeneration(2),
                    durable_fence: store_fence,
                    digest: vec![1],
                    verified_at: RecoveryGeneration(2),
                },
            },
        });
        assert!(matches!(
            recovery.commit_durable(digest),
            Err(RecoveryError::InvalidTransition(
                TransitionError::IntegrityCoverageMissing
            ))
        ));
    }

    #[test]
    fn dirty_integrity_intent_survives_semantic_export() {
        let mut recovery = MemoryRecoveryStore::new(TopologyEpoch(1));
        let evidence = IntentCommit::new(
            &mut recovery,
            TopologyEpoch(1),
            RecoveryGeneration(0),
            InvalidationTarget::new(vec![RegionId(5)], vec![IntegrityExtentId(6)]),
        )
        .commit()
        .unwrap();
        let manifest = recovery
            .export_manifest(evidence.committed_generation)
            .unwrap();
        assert_eq!(manifest.snapshot.generation, evidence.committed_generation);
        assert!(matches!(
            manifest.snapshot.dirty_regions[0].state,
            RegionState::Dirty { .. }
        ));
        assert!(matches!(
            manifest.snapshot.integrity_records[0].state,
            IntegrityState::Stale { .. }
        ));
    }
}
