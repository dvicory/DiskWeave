#![cfg(target_os = "macos")]

use dwv_codec::{Geometry, compute_parity};
use dwv_core::{
    ArrayId, AssignmentGeneration, AssignmentInstanceId, ByteRange, CodingPosition, CodingProfile,
    MemberRole, ProtectedGeometry, SlotId, TopologyAssignment, TopologyEpoch, TopologySnapshot,
};
use dwv_recovery::{
    MemoryRecoveryStore, RebuildId, RebuildLifecycle, RebuildState, RebuildTargetIdentity,
    RecoveryGeneration, RecoveryMutation, RecoveryStateStore,
};
use dwv_service::{
    FileRebuildStore, commit_verified_rebuild_chunk, commit_verified_rebuild_completion,
};
use dwv_store::StoreId;
use dwv_store_file::{FileStore, FileStoreConfig, FileSyncMode};
use dwv_verify::{
    KnownErasureAuthorization, RebuildBinding, ReconstructionRangeEvidence,
    ReconstructionSourceState, VerificationStore, authorize_known_erasure, execute_rebuild_chunk,
    plan_rebuild_ranges, verify_complete_rebuild,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const BLOCK: u32 = 512;
const CHUNK: u64 = 1024 * 1024;
const EPOCH: TopologyEpoch = TopologyEpoch(16);
const REBUILD_ID: RebuildId = RebuildId::from_bytes([0x16; 16]);
const MISSING_SLOT: SlotId = SlotId([1; 16]);
const REPLACEMENT_STORE: StoreId = StoreId(20);

struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should follow Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dwv-os016-apfs-rebuild-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("fixture directory should be unique");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn open_source(path: &Path, store_id: StoreId, length: u64) -> FileRebuildStore {
    FileRebuildStore::new(
        store_id,
        FileStore::open(
            FileStoreConfig::new(path, length, BLOCK)
                .maximum_transfer(length)
                .writable(false)
                .store_id(store_id)
                .topology_epoch(EPOCH),
        )
        .expect("source image should open"),
    )
}

fn topology(length: u64) -> TopologySnapshot {
    TopologySnapshot::new(
        ArrayId([16; 16]),
        EPOCH,
        CodingProfile::new(2, 1).unwrap(),
        ProtectedGeometry::new(length, BLOCK).unwrap(),
        vec![
            TopologyAssignment::new(
                MISSING_SLOT,
                MemberRole::Data,
                CodingPosition(0),
                AssignmentInstanceId([11; 16]),
                AssignmentGeneration(1),
            ),
            TopologyAssignment::new(
                SlotId([2; 16]),
                MemberRole::Data,
                CodingPosition(1),
                AssignmentInstanceId([12; 16]),
                AssignmentGeneration(1),
            ),
            TopologyAssignment::new(
                SlotId([3; 16]),
                MemberRole::Parity,
                CodingPosition(2),
                AssignmentInstanceId([13; 16]),
                AssignmentGeneration(1),
            ),
        ],
    )
    .unwrap()
}

fn authorize(
    topology: &TopologySnapshot,
    generation: RecoveryGeneration,
    geometry: &Geometry,
    range: ByteRange,
    data: &[Option<FileRebuildStore>],
    parity: &FileRebuildStore,
) -> KnownErasureAuthorization {
    let data_refs = data.iter().map(Option::as_ref).collect::<Vec<_>>();
    authorize_known_erasure(
        topology,
        generation,
        geometry.clone(),
        range,
        ReconstructionRangeEvidence::ParityClean,
        true,
        &data_refs,
        parity,
        &[
            ReconstructionSourceState::Missing,
            ReconstructionSourceState::Available,
            ReconstructionSourceState::Available,
        ],
    )
    .expect("clean APFS-image range should authorize")
}

#[test]
#[ignore = "set DWV_OS016_APFS_REFERENCE and DWV_OS016_APFS_REPLACEMENT"]
fn rebuilt_apfs_disk_image_is_ready_for_independent_attachment() {
    let reference_path = PathBuf::from(
        std::env::var_os("DWV_OS016_APFS_REFERENCE")
            .expect("DWV_OS016_APFS_REFERENCE must name a detached APFS image"),
    );
    let replacement_path = PathBuf::from(
        std::env::var_os("DWV_OS016_APFS_REPLACEMENT")
            .expect("DWV_OS016_APFS_REPLACEMENT must name an absent output"),
    );
    assert!(!replacement_path.exists(), "replacement must be a new path");

    let reference = fs::read(&reference_path).expect("reference image should be readable");
    let length = u64::try_from(reference.len()).unwrap();
    assert!(length > 0 && length.is_multiple_of(u64::from(BLOCK)));
    let survivor = (0..reference.len())
        .map(|index| ((index * 29 + 5) % 253) as u8)
        .collect::<Vec<_>>();
    let geometry = Geometry::new(vec![length, length], length).unwrap();
    let parity_bytes = compute_parity(&geometry, &[&reference, &survivor]).unwrap();
    let fixture = FixtureDirectory::new();
    let survivor_path = fixture.join("survivor.bin");
    let parity_path = fixture.join("parity.bin");
    fs::write(&survivor_path, &survivor).unwrap();
    fs::write(&parity_path, &parity_bytes).unwrap();

    let topology = topology(length);
    let recovery_topology = dwv_recovery::TopologySnapshot::from_core(
        topology.clone(),
        vec![StoreId(10), StoreId(11), StoreId(12)],
    )
    .unwrap();
    let mut recovery = MemoryRecoveryStore::new(TopologyEpoch(0));
    let mut topology_txn = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(0));
    topology_txn.push(RecoveryMutation::PrepareTopology {
        topology: recovery_topology.clone(),
    });
    topology_txn.push(RecoveryMutation::CommitTopology {
        topology_epoch: EPOCH,
    });
    let source_generation = recovery.commit_durable(topology_txn).unwrap();

    let mut target = FileRebuildStore::new(
        REPLACEMENT_STORE,
        FileStore::open(
            FileStoreConfig::new(&replacement_path, length, BLOCK)
                .maximum_transfer(length)
                .create(true)
                .sparse(true)
                .sync_mode(FileSyncMode::CallerFlush)
                .store_id(REPLACEMENT_STORE)
                .topology_epoch(EPOCH),
        )
        .expect("new replacement should open"),
    );
    let rebuild = RebuildState::prepare(
        REBUILD_ID,
        recovery_topology,
        source_generation,
        MISSING_SLOT,
        CodingPosition(0),
        AssignmentInstanceId([21; 16]),
        REPLACEMENT_STORE,
        RebuildTargetIdentity::from_bytes(target.identity().0),
    )
    .unwrap();
    let mut begin = recovery.begin_protocol_txn(source_generation, EPOCH);
    begin.begin_offline_rebuild(rebuild);
    recovery.commit_durable(begin).unwrap();

    let mut data = vec![None, Some(open_source(&survivor_path, StoreId(11), length))];
    let mut parity = open_source(&parity_path, StoreId(12), length);
    for range in plan_rebuild_ranges(length, CHUNK, 0, BLOCK).unwrap() {
        let generation = recovery.snapshot().generation;
        let authorization = authorize(&topology, generation, &geometry, range, &data, &parity);
        let receipt = execute_rebuild_chunk(
            RebuildBinding::from_recovery(&recovery.snapshot().rebuilds[0]),
            &authorization,
            EPOCH,
            generation,
            &mut data,
            &mut parity,
            &mut target,
        )
        .unwrap();
        commit_verified_rebuild_chunk(&mut recovery, REBUILD_ID, &receipt, &mut target).unwrap();
    }

    let generation = recovery.snapshot().generation;
    let full = authorize(
        &topology,
        generation,
        &geometry,
        ByteRange::new(0, length).unwrap(),
        &data,
        &parity,
    );
    let receipt = verify_complete_rebuild(
        RebuildBinding::from_recovery(&recovery.snapshot().rebuilds[0]),
        &full,
        EPOCH,
        generation,
        &mut data,
        &mut parity,
        &mut target,
    )
    .unwrap();
    commit_verified_rebuild_completion(&mut recovery, REBUILD_ID, &receipt, &mut target).unwrap();
    assert_eq!(
        recovery.snapshot().rebuilds[0].lifecycle(),
        RebuildLifecycle::Verified
    );

    drop(target);
    drop(parity);
    drop(data);
    assert_eq!(fs::read(&replacement_path).unwrap(), reference);
    assert_eq!(fs::read(&survivor_path).unwrap(), survivor);
    assert_eq!(fs::read(&parity_path).unwrap(), parity_bytes);
}
