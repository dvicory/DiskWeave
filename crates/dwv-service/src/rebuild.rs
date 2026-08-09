//! File-backed adapter for portable degraded-read and offline-rebuild engines.

use dwv_core::ByteRange;
use dwv_recovery::{
    Blake3Provider, DigestProvider, RebuildCursor, RebuildError as RecoveryRebuildError, RebuildId,
    RecoveryError, RecoveryGeneration, RecoveryStateStore,
};
use dwv_store::{
    ChildOperationId, CompletionDisposition, OperationSlotToken, PersistenceEvidence,
    StoreFenceRef, StoreId, StoreWriteWatermark, WriteIntent,
};
use dwv_store_file::FileStore;
use dwv_verify::{
    RebuildBinding, RebuildChunkReceipt as VerifiedChunkReceipt, RebuildTarget,
    RebuildVerificationReceipt, VerificationIdentity, VerificationStore, VerificationStoreError,
};
use std::fmt;

pub struct FileRebuildStore {
    store_id: StoreId,
    store: FileStore,
    operation_index: u32,
    last_write_watermark: Option<StoreWriteWatermark>,
    last_fence: Option<StoreFenceRef>,
}

impl FileRebuildStore {
    pub fn new(store_id: StoreId, store: FileStore) -> Self {
        Self {
            store_id,
            store,
            operation_index: 0,
            last_write_watermark: None,
            last_fence: None,
        }
    }

    pub const fn store_id(&self) -> StoreId {
        self.store_id
    }

    pub const fn last_fence(&self) -> Option<StoreFenceRef> {
        self.last_fence
    }

    pub const fn file_store(&self) -> &FileStore {
        &self.store
    }

    pub fn file_store_mut(&mut self) -> &mut FileStore {
        &mut self.store
    }

    pub fn into_inner(self) -> FileStore {
        self.store
    }

    fn next_operation(&mut self) -> ChildOperationId {
        let index = self.operation_index;
        self.operation_index = self.operation_index.wrapping_add(1);
        ChildOperationId {
            slot: OperationSlotToken::new(0, 1),
            index,
        }
    }

    fn resume_after_fence(
        &mut self,
        fence: Option<StoreFenceRef>,
    ) -> Result<(), OfflineRebuildCommitError> {
        let Some(fence) = fence else {
            return Ok(());
        };
        let next_index = u32::try_from(fence.through.0)
            .map_err(|_| OfflineRebuildCommitError::ReceiptBindingMismatch)?;
        self.operation_index = self.operation_index.max(next_index);
        self.last_fence = Some(fence);
        Ok(())
    }
}

impl VerificationStore for FileRebuildStore {
    fn identity(&self) -> VerificationIdentity {
        VerificationIdentity(
            self.store
                .capabilities_report()
                .identity
                .observations
                .first()
                .map(|observation| observation.fingerprint)
                .unwrap_or([0; 16]),
        )
    }

    fn read_exact(&mut self, range: ByteRange) -> Result<Vec<u8>, VerificationStoreError> {
        self.store
            .read_bytes(range)
            .map_err(|error| VerificationStoreError::new(error.to_string()))
    }

    fn write_exact(
        &mut self,
        range: ByteRange,
        bytes: &[u8],
    ) -> Result<(), VerificationStoreError> {
        let operation = self.next_operation();
        let completion = self
            .store
            .write_bytes(operation, range, bytes, WriteIntent::Ordinary);
        if matches!(completion.disposition, CompletionDisposition::Success)
            && completion.completed.covers(range).unwrap_or(false)
        {
            self.last_write_watermark = Some(completion.write_watermark.ok_or_else(|| {
                VerificationStoreError::new("replacement write lacks a store watermark")
            })?);
            Ok(())
        } else {
            Err(VerificationStoreError::new(format!(
                "replacement write did not complete exactly: {:?}",
                completion.disposition
            )))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OfflineRebuildCommitError {
    Recovery(RecoveryError),
    Rebuild(RecoveryRebuildError),
    RebuildNotFound,
    ReceiptBindingMismatch,
    MissingDurableFence,
}

impl fmt::Display for OfflineRebuildCommitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "offline rebuild commit refused: {self:?}")
    }
}

impl std::error::Error for OfflineRebuildCommitError {}

impl From<RecoveryError> for OfflineRebuildCommitError {
    fn from(error: RecoveryError) -> Self {
        Self::Recovery(error)
    }
}

impl From<RecoveryRebuildError> for OfflineRebuildCommitError {
    fn from(error: RecoveryRebuildError) -> Self {
        Self::Rebuild(error)
    }
}

/// Advances the durable recovery cursor only after validating the opaque
/// verifier receipt against the live target identity and its latest fence.
pub fn commit_verified_rebuild_chunk<R: RecoveryStateStore>(
    recovery: &mut R,
    rebuild_id: RebuildId,
    receipt: &VerifiedChunkReceipt,
    target: &mut FileRebuildStore,
) -> Result<RecoveryGeneration, OfflineRebuildCommitError> {
    let snapshot = recovery.load_assembly_snapshot()?;
    let rebuild = snapshot
        .rebuilds
        .iter()
        .find(|rebuild| rebuild.id() == rebuild_id)
        .ok_or(OfflineRebuildCommitError::RebuildNotFound)?;
    let fence = target
        .last_fence()
        .ok_or(OfflineRebuildCommitError::MissingDurableFence)?;
    let binding_matches = receipt.rebuild() == RebuildBinding::from_recovery(rebuild)
        && receipt.array_id() == rebuild.source_topology().array_id()
        && receipt.topology_epoch() == rebuild.source_topology().topology_epoch()
        && receipt.recovery_generation() == snapshot.generation
        && receipt.stable_slot() == rebuild.missing_slot()
        && receipt.coding_position() == rebuild.missing_coding_position()
        && receipt.replacement_identity() == target.identity()
        && rebuild.replacement_identity().as_bytes() == target.identity().0
        && receipt.range().offset == rebuild.cursor().offset()
        && receipt.durable_through() == receipt.range().end()
        && target.store_id() == rebuild.replacement_store()
        && receipt.durable_fence() == fence
        && fence.store_id == rebuild.replacement_store()
        && fence.topology_epoch == rebuild.source_topology().topology_epoch();
    if !binding_matches {
        return Err(OfflineRebuildCommitError::ReceiptBindingMismatch);
    }

    let durable_bytes = target
        .read_exact(receipt.range())
        .map_err(|_| OfflineRebuildCommitError::ReceiptBindingMismatch)?;
    let durable_digest = Blake3Provider
        .digest(&durable_bytes)
        .map_err(|_| OfflineRebuildCommitError::ReceiptBindingMismatch)?;
    if durable_digest != receipt.digest() {
        return Err(OfflineRebuildCommitError::ReceiptBindingMismatch);
    }

    let durable =
        rebuild.durable_chunk_receipt(snapshot.generation, receipt.durable_through(), fence)?;
    let mut txn = recovery.begin_protocol_txn(snapshot.generation, snapshot.topology_epoch);
    txn.advance_offline_rebuild(durable);
    recovery.commit_durable(txn).map_err(Into::into)
}

/// Marks a rebuild verified only after the full-range verifier receipt is
/// rebound to the current durable recovery state and replacement identity.
pub fn commit_verified_rebuild_completion<R: RecoveryStateStore>(
    recovery: &mut R,
    rebuild_id: RebuildId,
    receipt: &RebuildVerificationReceipt,
    target: &mut FileRebuildStore,
) -> Result<RecoveryGeneration, OfflineRebuildCommitError> {
    let snapshot = recovery.load_assembly_snapshot()?;
    let rebuild = snapshot
        .rebuilds
        .iter()
        .find(|rebuild| rebuild.id() == rebuild_id)
        .ok_or(OfflineRebuildCommitError::RebuildNotFound)?;
    let binding_matches = receipt.rebuild() == RebuildBinding::from_recovery(rebuild)
        && receipt.array_id() == rebuild.source_topology().array_id()
        && receipt.topology_epoch() == rebuild.source_topology().topology_epoch()
        && receipt.recovery_generation() == snapshot.generation
        && receipt.stable_slot() == rebuild.missing_slot()
        && receipt.coding_position() == rebuild.missing_coding_position()
        && receipt.replacement_identity() == target.identity()
        && rebuild.replacement_identity().as_bytes() == target.identity().0
        && receipt.protected_length() == rebuild.geometry().protected_length()
        && target.store_id() == rebuild.replacement_store();
    if !binding_matches {
        return Err(OfflineRebuildCommitError::ReceiptBindingMismatch);
    }

    let final_bytes = target
        .read_exact(
            ByteRange::new(0, receipt.protected_length())
                .map_err(|_| OfflineRebuildCommitError::ReceiptBindingMismatch)?,
        )
        .map_err(|_| OfflineRebuildCommitError::ReceiptBindingMismatch)?;
    let final_digest = Blake3Provider
        .digest(&final_bytes)
        .map_err(|_| OfflineRebuildCommitError::ReceiptBindingMismatch)?;
    if final_digest != receipt.digest() {
        return Err(OfflineRebuildCommitError::ReceiptBindingMismatch);
    }

    let completion = rebuild.final_verification_receipt(snapshot.generation, receipt.digest())?;
    let mut txn = recovery.begin_protocol_txn(snapshot.generation, snapshot.topology_epoch);
    txn.complete_offline_rebuild(completion);
    recovery.commit_durable(txn).map_err(Into::into)
}

/// Validates the persisted replacement binding before any resumed payload I/O.
pub fn validate_rebuild_resume<R: RecoveryStateStore>(
    recovery: &R,
    rebuild_id: RebuildId,
    target: &mut FileRebuildStore,
) -> Result<RebuildCursor, OfflineRebuildCommitError> {
    let snapshot = recovery.load_assembly_snapshot()?;
    let rebuild = snapshot
        .rebuilds
        .iter()
        .find(|rebuild| rebuild.id() == rebuild_id)
        .ok_or(OfflineRebuildCommitError::RebuildNotFound)?;
    if target.store_id() != rebuild.replacement_store()
        || target.identity().0 != rebuild.replacement_identity().as_bytes()
        || rebuild.source_topology().topology_epoch() != snapshot.topology_epoch
        || snapshot.active_topology.as_ref() != Some(rebuild.source_topology())
        || snapshot.generation < rebuild.source_recovery_generation()
    {
        return Err(OfflineRebuildCommitError::ReceiptBindingMismatch);
    }
    target.resume_after_fence(rebuild.last_replacement_fence())?;
    Ok(rebuild.cursor())
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::*;
    use crate::{FileRebuildSource, authorize_file_known_erasure};
    use dwv_codec::{Geometry, compute_parity};
    use dwv_core::{
        ArrayId, AssignmentGeneration, AssignmentInstanceId, CodingPosition, CodingProfile,
        MemberRole, ProtectedGeometry, SlotId, TopologyAssignment, TopologyEpoch, TopologySnapshot,
    };
    use dwv_recovery::{
        MemoryRecoveryStore, RebuildLifecycle, RebuildState, RebuildTargetIdentity,
        RecoveryMutation, RecoveryStateStore,
    };
    use dwv_store_file::{FileStoreConfig, FileSyncMode};
    use dwv_verify::{
        execute_rebuild_chunk, plan_rebuild_ranges, read_known_erasure, verify_complete_rebuild,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    const LENGTH: u64 = 4096;
    const BLOCK: u32 = 512;
    const CHUNK: u64 = 1024;
    const EPOCH: TopologyEpoch = TopologyEpoch(7);
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
            let path =
                std::env::temp_dir().join(format!("dwv-os016-{}-{nonce}", std::process::id()));
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

    fn open_source(
        path: &Path,
        store_id: StoreId,
        assignment_instance: AssignmentInstanceId,
    ) -> FileRebuildSource {
        FileRebuildSource::open(
            FileStoreConfig::new(path, LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .writable(false)
                .store_id(store_id)
                .topology_epoch(EPOCH),
            store_id,
            assignment_instance,
        )
        .expect("source file should open")
    }

    fn open_target(path: &Path, create: bool) -> FileRebuildStore {
        let store = FileStore::open(
            FileStoreConfig::new(path, LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .create_new(create)
                .sparse(true)
                .sync_mode(FileSyncMode::CallerFlush)
                .store_id(REPLACEMENT_STORE)
                .topology_epoch(EPOCH),
        )
        .expect("replacement file should open");
        FileRebuildStore::new(REPLACEMENT_STORE, store)
    }

    fn topology() -> TopologySnapshot {
        TopologySnapshot::new(
            ArrayId([7; 16]),
            EPOCH,
            CodingProfile::new(2, 1).unwrap(),
            ProtectedGeometry::new(LENGTH, BLOCK).unwrap(),
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
        recovery: &MemoryRecoveryStore,
        topology: &TopologySnapshot,
        range: ByteRange,
        data: &[Option<FileRebuildSource>],
        parity: &FileRebuildSource,
    ) -> dwv_verify::KnownErasureAuthorization {
        let data_refs = data.iter().map(Option::as_ref).collect::<Vec<_>>();
        authorize_file_known_erasure(
            recovery,
            topology,
            Geometry::new(vec![LENGTH, LENGTH], LENGTH).unwrap(),
            range,
            &data_refs,
            parity,
        )
        .expect("recovery state should authorize a clean single erasure")
    }

    #[test]
    fn regular_file_degraded_read_and_sparse_rebuild_resume_to_direct_equality() {
        let directory = FixtureDirectory::new();
        let missing_path = directory.join("missing-reference.bin");
        let survivor_path = directory.join("survivor.bin");
        let parity_path = directory.join("parity.bin");
        let replacement_path = directory.join("replacement.bin");
        let wrong_replacement_path = directory.join("wrong-replacement.bin");
        let missing = (0..LENGTH)
            .map(|index| ((index * 17 + 3) % 251) as u8)
            .collect::<Vec<_>>();
        let survivor = (0..LENGTH)
            .map(|index| ((index * 29 + 5) % 253) as u8)
            .collect::<Vec<_>>();
        let parity_bytes = compute_parity(
            &Geometry::new(vec![LENGTH, LENGTH], LENGTH).unwrap(),
            &[&missing, &survivor],
        )
        .unwrap();
        fs::write(&missing_path, &missing).unwrap();
        fs::write(&survivor_path, &survivor).unwrap();
        fs::write(&parity_path, &parity_bytes).unwrap();

        let topology = topology();
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
        let mut target = open_target(&replacement_path, true);
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

        let mut data = vec![
            None,
            Some(open_source(
                &survivor_path,
                StoreId(11),
                AssignmentInstanceId([12; 16]),
            )),
        ];
        let mut parity = open_source(&parity_path, StoreId(12), AssignmentInstanceId([13; 16]));

        let middle = ByteRange::new(512, 1024).unwrap();
        let degraded = read_known_erasure(
            &authorize(&recovery, &topology, middle, &data, &parity),
            EPOCH,
            recovery.snapshot().generation,
            &mut data,
            &mut parity,
        )
        .unwrap();
        assert_eq!(degraded.bytes, missing[512..1536]);

        for range in plan_rebuild_ranges(LENGTH, CHUNK, 0, BLOCK)
            .unwrap()
            .take(2)
        {
            let generation = recovery.snapshot().generation;
            let authorization = authorize(&recovery, &topology, range, &data, &parity);
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
            commit_verified_rebuild_chunk(&mut recovery, REBUILD_ID, &receipt, &mut target)
                .unwrap();
        }
        let checkpointed = target
            .read_exact(ByteRange::new(0, 2 * CHUNK).unwrap())
            .unwrap();
        assert_eq!(checkpointed, missing[..(2 * CHUNK) as usize]);
        let manifest = recovery
            .export_manifest(recovery.snapshot().generation)
            .unwrap();
        drop(target);
        drop(parity);
        drop(data);

        let mut recovery = MemoryRecoveryStore::from_manifest(manifest).unwrap();
        assert_eq!(recovery.snapshot().rebuilds[0].cursor().offset(), 2 * CHUNK);
        fs::write(&wrong_replacement_path, vec![0_u8; LENGTH as usize]).unwrap();
        let mut wrong_target = open_target(&wrong_replacement_path, false);
        assert_eq!(
            validate_rebuild_resume(&recovery, REBUILD_ID, &mut wrong_target),
            Err(OfflineRebuildCommitError::ReceiptBindingMismatch)
        );
        assert_eq!(recovery.snapshot().rebuilds[0].cursor().offset(), 2 * CHUNK);
        drop(wrong_target);
        let mut data = vec![
            None,
            Some(open_source(
                &survivor_path,
                StoreId(11),
                AssignmentInstanceId([12; 16]),
            )),
        ];
        let mut parity = open_source(&parity_path, StoreId(12), AssignmentInstanceId([13; 16]));
        let mut target = open_target(&replacement_path, false);
        assert_eq!(
            validate_rebuild_resume(&recovery, REBUILD_ID, &mut target).unwrap(),
            recovery.snapshot().rebuilds[0].cursor()
        );
        assert_eq!(
            target
                .read_exact(ByteRange::new(0, 2 * CHUNK).unwrap())
                .unwrap(),
            checkpointed
        );

        for range in plan_rebuild_ranges(LENGTH, CHUNK, 2 * CHUNK, BLOCK).unwrap() {
            let generation = recovery.snapshot().generation;
            let authorization = authorize(&recovery, &topology, range, &data, &parity);
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
            commit_verified_rebuild_chunk(&mut recovery, REBUILD_ID, &receipt, &mut target)
                .unwrap();
        }

        let generation = recovery.snapshot().generation;
        let full = authorize(
            &recovery,
            &topology,
            ByteRange::new(0, LENGTH).unwrap(),
            &data,
            &parity,
        );
        let verified = verify_complete_rebuild(
            RebuildBinding::from_recovery(&recovery.snapshot().rebuilds[0]),
            &full,
            EPOCH,
            generation,
            &mut data,
            &mut parity,
            &mut target,
        )
        .unwrap();
        commit_verified_rebuild_completion(&mut recovery, REBUILD_ID, &verified, &mut target)
            .unwrap();
        let completed = &recovery.snapshot().rebuilds[0];
        assert_eq!(completed.lifecycle(), RebuildLifecycle::Verified);
        let prepared = completed
            .prepared_replacement_topology(EPOCH.next().unwrap(), AssignmentGeneration(2))
            .unwrap();
        assert_eq!(
            prepared
                .assignments()
                .iter()
                .find(|assignment| assignment.slot_id() == MISSING_SLOT)
                .unwrap()
                .store_id(),
            REPLACEMENT_STORE
        );
        assert_eq!(recovery.snapshot().topology_epoch, EPOCH);

        drop(target);
        drop(parity);
        drop(data);
        assert_eq!(fs::read(&replacement_path).unwrap(), missing);
        assert_eq!(fs::read(&missing_path).unwrap(), missing);
        assert_eq!(fs::read(&survivor_path).unwrap(), survivor);
        assert_eq!(fs::read(&parity_path).unwrap(), parity_bytes);
    }
}

impl RebuildTarget for FileRebuildStore {
    fn flush_rebuild(&mut self) -> Result<StoreFenceRef, VerificationStoreError> {
        let through = self
            .last_write_watermark
            .ok_or_else(|| VerificationStoreError::new("no replacement write to flush"))?;
        let operation = self.next_operation();
        let completion = self.store.flush_file(operation, through);
        match (completion.disposition, completion.persistence) {
            (CompletionDisposition::Success, PersistenceEvidence::DurableByFence { fence })
                if fence.store_id == self.store_id && fence.through >= through =>
            {
                self.last_fence = Some(fence);
                Ok(fence)
            }
            (disposition, persistence) => Err(VerificationStoreError::new(format!(
                "replacement flush lacks durable fence: {disposition:?} {persistence:?}"
            ))),
        }
    }
}
