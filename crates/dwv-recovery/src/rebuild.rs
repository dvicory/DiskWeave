//! Typed semantic authority for offline replacement rebuilds.
//!
//! This module records only portable, bounded recovery facts.  It deliberately
//! does not contain paths, database rows, runtime handles, or claims about how
//! replacement bytes were produced.  A later rebuild engine may issue a chunk
//! receipt only after readback, equation verification, and a durability fence.

use crate::{Digest, RecoveryGeneration, TopologySnapshot};
use dwv_core::{
    AssignmentGeneration, AssignmentInstanceId, CodingPosition, MemberRole, SlotId,
    TopologyAssignment, TopologyEpoch,
};
use dwv_store::{StoreFenceRef, StoreId};
use std::fmt;

pub const REBUILD_ID_BYTES: usize = 16;
pub const REBUILD_TARGET_IDENTITY_BYTES: usize = 16;

/// Stable, fixed-width identity for one offline rebuild attempt.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RebuildId([u8; REBUILD_ID_BYTES]);

impl RebuildId {
    pub const fn from_bytes(bytes: [u8; REBUILD_ID_BYTES]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; REBUILD_ID_BYTES] {
        self.0
    }
}

/// Portable observed identity of the separately opened rebuild target.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RebuildTargetIdentity([u8; REBUILD_TARGET_IDENTITY_BYTES]);

impl RebuildTargetIdentity {
    pub const fn from_bytes(bytes: [u8; REBUILD_TARGET_IDENTITY_BYTES]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; REBUILD_TARGET_IDENTITY_BYTES] {
        self.0
    }
}

/// First byte that has not yet been verified and durably checkpointed.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RebuildCursor(u64);

impl RebuildCursor {
    pub const ZERO: Self = Self(0);

    pub const fn offset(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RebuildLifecycle {
    Prepared,
    Materializing,
    AwaitingFinalVerification,
    Verified,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
struct RebuildBinding {
    source_topology: TopologySnapshot,
    source_recovery_generation: RecoveryGeneration,
    missing_slot: SlotId,
    missing_coding_position: CodingPosition,
    replacement_assignment: AssignmentInstanceId,
    replacement_store: StoreId,
    replacement_identity: RebuildTargetIdentity,
}

/// Durable semantic state for one separately targeted offline rebuild.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RebuildState {
    id: RebuildId,
    binding: RebuildBinding,
    cursor: RebuildCursor,
    lifecycle: RebuildLifecycle,
    last_replacement_fence: Option<StoreFenceRef>,
    verified_digest: Option<Digest>,
}

impl RebuildState {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        id: RebuildId,
        source_topology: TopologySnapshot,
        source_recovery_generation: RecoveryGeneration,
        missing_slot: SlotId,
        missing_coding_position: CodingPosition,
        replacement_assignment: AssignmentInstanceId,
        replacement_store: StoreId,
        replacement_identity: RebuildTargetIdentity,
    ) -> Result<Self, RebuildError> {
        if source_topology.profile().parity_slots() != 1 {
            return Err(RebuildError::UnsupportedProfile);
        }
        let missing = source_topology
            .assignments()
            .iter()
            .find(|assignment| assignment.slot_id() == missing_slot)
            .ok_or(RebuildError::MissingSourceSlot)?;
        if missing.role() != MemberRole::Data {
            return Err(RebuildError::MissingMemberNotData);
        }
        if missing.coding_position() != missing_coding_position {
            return Err(RebuildError::MissingCodingPositionMismatch);
        }
        if source_topology
            .assignments()
            .iter()
            .any(|assignment| assignment.assignment_instance() == replacement_assignment)
        {
            return Err(RebuildError::ReplacementAssignmentAliasesSource);
        }
        if source_topology
            .assignments()
            .iter()
            .any(|assignment| assignment.store_id() == replacement_store)
        {
            return Err(RebuildError::ReplacementStoreAliasesSource);
        }

        Ok(Self {
            id,
            binding: RebuildBinding {
                source_topology,
                source_recovery_generation,
                missing_slot,
                missing_coding_position,
                replacement_assignment,
                replacement_store,
                replacement_identity,
            },
            cursor: RebuildCursor::ZERO,
            lifecycle: RebuildLifecycle::Prepared,
            last_replacement_fence: None,
            verified_digest: None,
        })
    }

    pub const fn id(&self) -> RebuildId {
        self.id
    }

    pub const fn source_topology(&self) -> &TopologySnapshot {
        &self.binding.source_topology
    }

    pub const fn source_recovery_generation(&self) -> RecoveryGeneration {
        self.binding.source_recovery_generation
    }

    pub const fn missing_slot(&self) -> SlotId {
        self.binding.missing_slot
    }

    pub const fn missing_coding_position(&self) -> CodingPosition {
        self.binding.missing_coding_position
    }

    pub const fn replacement_assignment(&self) -> AssignmentInstanceId {
        self.binding.replacement_assignment
    }

    pub const fn replacement_store(&self) -> StoreId {
        self.binding.replacement_store
    }

    pub const fn replacement_identity(&self) -> RebuildTargetIdentity {
        self.binding.replacement_identity
    }

    pub const fn geometry(&self) -> dwv_core::ProtectedGeometry {
        self.binding.source_topology.geometry()
    }

    pub const fn cursor(&self) -> RebuildCursor {
        self.cursor
    }

    pub const fn lifecycle(&self) -> RebuildLifecycle {
        self.lifecycle
    }

    pub const fn last_replacement_fence(&self) -> Option<StoreFenceRef> {
        self.last_replacement_fence
    }

    pub const fn verified_digest(&self) -> Option<Digest> {
        self.verified_digest
    }

    /// Construct an opaque receipt for a chunk whose replacement bytes have
    /// already been read back, equation-verified, and durably fenced.
    pub fn durable_chunk_receipt(
        &self,
        expected_recovery_generation: RecoveryGeneration,
        first_unprocessed_byte: u64,
        replacement_fence: StoreFenceRef,
    ) -> Result<RebuildChunkReceipt, RebuildError> {
        if !matches!(
            self.lifecycle,
            RebuildLifecycle::Prepared | RebuildLifecycle::Materializing
        ) {
            return Err(RebuildError::LifecycleRefusesAdvance);
        }
        if expected_recovery_generation < self.binding.source_recovery_generation {
            return Err(RebuildError::RecoveryGenerationPrecedesSource);
        }
        if first_unprocessed_byte <= self.cursor.0 {
            return Err(RebuildError::CursorNotForward);
        }
        if first_unprocessed_byte > self.geometry().protected_length() {
            return Err(RebuildError::CursorOutsideGeometry);
        }
        if !first_unprocessed_byte.is_multiple_of(self.geometry().logical_block_size().into()) {
            return Err(RebuildError::CursorUnaligned);
        }
        if replacement_fence.store_id != self.binding.replacement_store {
            return Err(RebuildError::FenceStoreMismatch);
        }
        if replacement_fence.topology_epoch != self.binding.source_topology.topology_epoch() {
            return Err(RebuildError::FenceTopologyMismatch);
        }
        if self
            .last_replacement_fence
            .is_some_and(|prior| replacement_fence.through < prior.through)
        {
            return Err(RebuildError::FenceRegressed);
        }

        Ok(RebuildChunkReceipt {
            rebuild_id: self.id,
            binding: self.binding.clone(),
            expected_recovery_generation,
            from: self.cursor,
            through: RebuildCursor(first_unprocessed_byte),
            replacement_fence,
        })
    }

    pub(crate) fn validate_source(
        &self,
        active_topology: Option<&TopologySnapshot>,
        current_generation: RecoveryGeneration,
    ) -> Result<(), RebuildError> {
        if active_topology != Some(&self.binding.source_topology) {
            return Err(RebuildError::SourceTopologyNotActive);
        }
        if current_generation != self.binding.source_recovery_generation {
            return Err(RebuildError::SourceRecoveryGenerationMismatch);
        }
        Ok(())
    }

    pub(crate) fn validate_restored(
        &self,
        active_topology: Option<&TopologySnapshot>,
        current_generation: RecoveryGeneration,
    ) -> Result<(), RebuildError> {
        if current_generation < self.binding.source_recovery_generation {
            return Err(RebuildError::RecoveryGenerationPrecedesSource);
        }
        if !matches!(self.lifecycle, RebuildLifecycle::Verified)
            && active_topology != Some(&self.binding.source_topology)
        {
            return Err(RebuildError::SourceTopologyNotActive);
        }
        if self.cursor.0 > self.geometry().protected_length()
            || !self
                .cursor
                .0
                .is_multiple_of(self.geometry().logical_block_size().into())
        {
            return Err(RebuildError::InvalidPersistedState);
        }
        let lifecycle_valid = match self.lifecycle {
            RebuildLifecycle::Prepared => {
                self.cursor == RebuildCursor::ZERO
                    && self.last_replacement_fence.is_none()
                    && self.verified_digest.is_none()
            }
            RebuildLifecycle::Materializing => {
                self.cursor > RebuildCursor::ZERO
                    && self.cursor.0 < self.geometry().protected_length()
                    && self.last_replacement_fence.is_some()
                    && self.verified_digest.is_none()
            }
            RebuildLifecycle::AwaitingFinalVerification => {
                self.cursor.0 == self.geometry().protected_length()
                    && self.last_replacement_fence.is_some()
                    && self.verified_digest.is_none()
            }
            RebuildLifecycle::Verified => {
                self.cursor.0 == self.geometry().protected_length()
                    && self.last_replacement_fence.is_some()
                    && self.verified_digest.is_some()
            }
            RebuildLifecycle::Blocked => true,
        };
        if !lifecycle_valid {
            return Err(RebuildError::InvalidPersistedState);
        }
        if let Some(fence) = self.last_replacement_fence
            && (fence.store_id != self.binding.replacement_store
                || fence.topology_epoch != self.binding.source_topology.topology_epoch())
        {
            return Err(RebuildError::InvalidPersistedState);
        }
        Ok(())
    }

    pub(crate) fn apply_chunk_receipt(
        &mut self,
        receipt: RebuildChunkReceipt,
        current_generation: RecoveryGeneration,
    ) -> Result<(), RebuildError> {
        if receipt.rebuild_id != self.id || receipt.binding != self.binding {
            return Err(RebuildError::BindingMismatch);
        }
        if receipt.expected_recovery_generation != current_generation {
            return Err(RebuildError::ReceiptGenerationMismatch);
        }
        if receipt.from != self.cursor {
            return Err(RebuildError::CursorMismatch);
        }
        if !matches!(
            self.lifecycle,
            RebuildLifecycle::Prepared | RebuildLifecycle::Materializing
        ) {
            return Err(RebuildError::LifecycleRefusesAdvance);
        }
        if receipt.through.0 <= self.cursor.0
            || receipt.through.0 > self.geometry().protected_length()
        {
            return Err(RebuildError::CursorOutsideGeometry);
        }
        if receipt.replacement_fence.store_id != self.binding.replacement_store {
            return Err(RebuildError::FenceStoreMismatch);
        }
        if receipt.replacement_fence.topology_epoch != self.binding.source_topology.topology_epoch()
        {
            return Err(RebuildError::FenceTopologyMismatch);
        }
        if self
            .last_replacement_fence
            .is_some_and(|prior| receipt.replacement_fence.through < prior.through)
        {
            return Err(RebuildError::FenceRegressed);
        }

        self.cursor = receipt.through;
        self.last_replacement_fence = Some(receipt.replacement_fence);
        self.lifecycle = if self.cursor.0 == self.geometry().protected_length() {
            RebuildLifecycle::AwaitingFinalVerification
        } else {
            RebuildLifecycle::Materializing
        };
        Ok(())
    }

    pub fn final_verification_receipt(
        &self,
        expected_recovery_generation: RecoveryGeneration,
        replacement_digest: Digest,
    ) -> Result<RebuildCompletionReceipt, RebuildError> {
        if self.lifecycle != RebuildLifecycle::AwaitingFinalVerification
            || self.cursor.0 != self.geometry().protected_length()
        {
            return Err(RebuildError::FinalVerificationIncomplete);
        }
        Ok(RebuildCompletionReceipt {
            rebuild_id: self.id,
            binding: self.binding.clone(),
            expected_recovery_generation,
            replacement_digest,
        })
    }

    pub(crate) fn apply_completion_receipt(
        &mut self,
        receipt: RebuildCompletionReceipt,
        current_generation: RecoveryGeneration,
    ) -> Result<(), RebuildError> {
        if receipt.rebuild_id != self.id || receipt.binding != self.binding {
            return Err(RebuildError::BindingMismatch);
        }
        if receipt.expected_recovery_generation != current_generation {
            return Err(RebuildError::ReceiptGenerationMismatch);
        }
        if self.lifecycle != RebuildLifecycle::AwaitingFinalVerification
            || self.cursor.0 != self.geometry().protected_length()
        {
            return Err(RebuildError::FinalVerificationIncomplete);
        }
        self.lifecycle = RebuildLifecycle::Verified;
        self.verified_digest = Some(receipt.replacement_digest);
        Ok(())
    }

    pub fn prepared_replacement_topology(
        &self,
        topology_epoch: TopologyEpoch,
        assignment_generation: AssignmentGeneration,
    ) -> Result<TopologySnapshot, RebuildError> {
        if self.lifecycle != RebuildLifecycle::Verified {
            return Err(RebuildError::FinalVerificationIncomplete);
        }
        if !topology_epoch.is_after(self.binding.source_topology.topology_epoch()) {
            return Err(RebuildError::ReplacementEpochNotNewer);
        }
        let missing = self
            .binding
            .source_topology
            .assignments()
            .iter()
            .find(|assignment| assignment.slot_id() == self.binding.missing_slot)
            .ok_or(RebuildError::MissingSourceSlot)?;
        if assignment_generation <= missing.assignment_generation() {
            return Err(RebuildError::ReplacementGenerationNotNewer);
        }

        let mut assignments = Vec::with_capacity(self.binding.source_topology.assignments().len());
        let mut stores = Vec::with_capacity(self.binding.source_topology.assignments().len());
        for assignment in self.binding.source_topology.assignments() {
            if assignment.slot_id() == self.binding.missing_slot {
                assignments.push(TopologyAssignment::new(
                    assignment.slot_id(),
                    assignment.role(),
                    assignment.coding_position(),
                    self.binding.replacement_assignment,
                    assignment_generation,
                ));
                stores.push(self.binding.replacement_store);
            } else {
                assignments.push(
                    TopologyAssignment::new(
                        assignment.slot_id(),
                        assignment.role(),
                        assignment.coding_position(),
                        assignment.assignment_instance(),
                        assignment.assignment_generation(),
                    )
                    .with_evidence(assignment.evidence()),
                );
                stores.push(assignment.store_id());
            }
        }
        let core = dwv_core::TopologySnapshot::new(
            self.binding.source_topology.array_id(),
            topology_epoch,
            self.binding.source_topology.profile(),
            self.binding.source_topology.geometry(),
            assignments,
        )
        .map_err(|_| RebuildError::InvalidReplacementTopology)?;
        TopologySnapshot::from_core(core, stores)
            .map_err(|_| RebuildError::InvalidReplacementTopology)
    }
}

/// Opaque-by-field durable chunk evidence consumed by recovery state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebuildChunkReceipt {
    rebuild_id: RebuildId,
    binding: RebuildBinding,
    expected_recovery_generation: RecoveryGeneration,
    from: RebuildCursor,
    through: RebuildCursor,
    replacement_fence: StoreFenceRef,
}

impl RebuildChunkReceipt {
    pub const fn rebuild_id(&self) -> RebuildId {
        self.rebuild_id
    }

    pub const fn from(&self) -> RebuildCursor {
        self.from
    }

    pub const fn through(&self) -> RebuildCursor {
        self.through
    }

    pub const fn replacement_fence(&self) -> StoreFenceRef {
        self.replacement_fence
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebuildCompletionReceipt {
    rebuild_id: RebuildId,
    binding: RebuildBinding,
    expected_recovery_generation: RecoveryGeneration,
    replacement_digest: Digest,
}

impl RebuildCompletionReceipt {
    pub const fn rebuild_id(&self) -> RebuildId {
        self.rebuild_id
    }

    pub const fn replacement_digest(&self) -> Digest {
        self.replacement_digest
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RebuildError {
    UnsupportedProfile,
    MissingSourceSlot,
    MissingMemberNotData,
    MissingCodingPositionMismatch,
    ReplacementAssignmentAliasesSource,
    ReplacementStoreAliasesSource,
    DuplicateRebuildId,
    SourceTopologyNotActive,
    SourceRecoveryGenerationMismatch,
    RebuildNotFound,
    RecoveryGenerationPrecedesSource,
    ReceiptGenerationMismatch,
    BindingMismatch,
    CursorNotForward,
    CursorOutsideGeometry,
    CursorUnaligned,
    CursorMismatch,
    LifecycleRefusesAdvance,
    FenceStoreMismatch,
    FenceTopologyMismatch,
    FenceRegressed,
    InvalidPersistedState,
    FinalVerificationIncomplete,
    ReplacementEpochNotNewer,
    ReplacementGenerationNotNewer,
    InvalidReplacementTopology,
}

impl fmt::Display for RebuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "offline rebuild state error: {self:?}")
    }
}

impl std::error::Error for RebuildError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MemoryRecoveryStore, RecoveryError, RecoveryExportLimits, RecoveryMutation,
        RecoveryRecordKind, RecoveryStateStore,
    };
    use dwv_core::{
        ArrayId, AssignmentGeneration, CodingProfile, ProtectedGeometry, TopologyAssignment,
        TopologyEpoch,
    };
    use dwv_store::{CapabilityEvidenceId, FenceId, StoreWriteWatermark};

    const REBUILD: RebuildId = RebuildId::from_bytes([9; REBUILD_ID_BYTES]);
    const MISSING_SLOT: SlotId = SlotId([1; 16]);
    const REPLACEMENT_ASSIGNMENT: AssignmentInstanceId = AssignmentInstanceId([21; 16]);
    const REPLACEMENT_IDENTITY: RebuildTargetIdentity =
        RebuildTargetIdentity::from_bytes([31; REBUILD_TARGET_IDENTITY_BYTES]);

    fn topology(array_byte: u8) -> TopologySnapshot {
        let core = dwv_core::TopologySnapshot::new(
            ArrayId([array_byte; 16]),
            TopologyEpoch(7),
            CodingProfile::new(2, 1).unwrap(),
            ProtectedGeometry::new(16, 4).unwrap(),
            vec![
                TopologyAssignment::new(
                    MISSING_SLOT,
                    MemberRole::Data,
                    CodingPosition(0),
                    AssignmentInstanceId([11; 16]),
                    AssignmentGeneration(3),
                ),
                TopologyAssignment::new(
                    SlotId([2; 16]),
                    MemberRole::Data,
                    CodingPosition(1),
                    AssignmentInstanceId([12; 16]),
                    AssignmentGeneration(3),
                ),
                TopologyAssignment::new(
                    SlotId([3; 16]),
                    MemberRole::Parity,
                    CodingPosition(2),
                    AssignmentInstanceId([13; 16]),
                    AssignmentGeneration(3),
                ),
            ],
        )
        .unwrap();
        TopologySnapshot::from_core(core, vec![StoreId(10), StoreId(11), StoreId(12)]).unwrap()
    }

    fn prepared_state(source: TopologySnapshot, replacement_store: StoreId) -> RebuildState {
        RebuildState::prepare(
            REBUILD,
            source,
            RecoveryGeneration::ZERO,
            MISSING_SLOT,
            CodingPosition(0),
            REPLACEMENT_ASSIGNMENT,
            replacement_store,
            REPLACEMENT_IDENTITY,
        )
        .unwrap()
    }

    fn store_with_rebuild(replacement_store: StoreId) -> MemoryRecoveryStore {
        let topology = topology(1);
        let rebuild = prepared_state(topology.clone(), replacement_store);
        let mut store = MemoryRecoveryStore::with_active_topology(topology);
        let mut transaction = store.begin_protocol_txn(RecoveryGeneration::ZERO, TopologyEpoch(7));
        transaction.begin_offline_rebuild(rebuild);
        assert_eq!(
            store.commit_durable(transaction).unwrap(),
            RecoveryGeneration(1)
        );
        store
    }

    fn fence(store_id: StoreId, through: u64) -> StoreFenceRef {
        StoreFenceRef {
            fence_id: FenceId(through),
            store_id,
            topology_epoch: TopologyEpoch(7),
            through: StoreWriteWatermark(through),
            capability_evidence_id: CapabilityEvidenceId(4),
        }
    }

    #[test]
    fn cursor_advancement_is_atomic_and_receipt_driven() {
        let mut store = store_with_rebuild(StoreId(90));
        let receipt = store.snapshot().rebuilds[0]
            .durable_chunk_receipt(RecoveryGeneration(1), 4, fence(StoreId(90), 1))
            .unwrap();
        let before = store.snapshot().clone();
        let mut invalid = store.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(7));
        invalid.push(RecoveryMutation::AdvanceOfflineRebuild {
            receipt: receipt.clone(),
        });
        invalid.push(RecoveryMutation::AdvanceOfflineRebuild {
            receipt: receipt.clone(),
        });
        assert!(matches!(
            store.commit_durable(invalid),
            Err(RecoveryError::Rebuild(RebuildError::CursorMismatch))
        ));
        assert_eq!(store.snapshot(), &before);

        let mut valid = store.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(7));
        valid.advance_offline_rebuild(receipt);
        assert_eq!(store.commit_durable(valid).unwrap(), RecoveryGeneration(2));
        assert_eq!(store.snapshot().rebuilds[0].cursor().offset(), 4);
        assert_eq!(
            store.snapshot().rebuilds[0].lifecycle(),
            RebuildLifecycle::Materializing
        );
    }

    #[test]
    fn stale_receipt_generation_is_refused_without_advancement() {
        let mut store = store_with_rebuild(StoreId(90));
        let receipt = store.snapshot().rebuilds[0]
            .durable_chunk_receipt(RecoveryGeneration(1), 4, fence(StoreId(90), 1))
            .unwrap();
        let unrelated = store.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(7));
        assert_eq!(
            store.commit_durable(unrelated).unwrap(),
            RecoveryGeneration(2)
        );
        let before = store.snapshot().clone();
        let mut transaction = store.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(7));
        transaction.advance_offline_rebuild(receipt);
        assert!(matches!(
            store.commit_durable(transaction),
            Err(RecoveryError::Rebuild(
                RebuildError::ReceiptGenerationMismatch
            ))
        ));
        assert_eq!(store.snapshot(), &before);
    }

    #[test]
    fn source_topology_and_replacement_binding_mismatches_are_refused() {
        let foreign = prepared_state(topology(2), StoreId(90));
        let local_topology = topology(1);
        let mut local = MemoryRecoveryStore::with_active_topology(local_topology);
        let mut start = local.begin_protocol_txn(RecoveryGeneration::ZERO, TopologyEpoch(7));
        start.begin_offline_rebuild(foreign);
        assert!(matches!(
            local.commit_durable(start),
            Err(RecoveryError::Rebuild(
                RebuildError::SourceTopologyNotActive
            ))
        ));
        assert!(local.snapshot().rebuilds.is_empty());

        let source = store_with_rebuild(StoreId(90));
        let receipt = source.snapshot().rebuilds[0]
            .durable_chunk_receipt(RecoveryGeneration(1), 4, fence(StoreId(90), 1))
            .unwrap();
        let mut different_target = store_with_rebuild(StoreId(91));
        let before = different_target.snapshot().clone();
        let mut advance =
            different_target.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(7));
        advance.advance_offline_rebuild(receipt);
        assert!(matches!(
            different_target.commit_durable(advance),
            Err(RecoveryError::Rebuild(RebuildError::BindingMismatch))
        ));
        assert_eq!(different_target.snapshot(), &before);
    }

    #[test]
    fn rebuild_export_is_bounded() {
        let store = store_with_rebuild(StoreId(90));
        assert!(matches!(
            store.export_manifest_with_limits(
                RecoveryGeneration(1),
                RecoveryExportLimits {
                    max_rebuilds: 0,
                    ..RecoveryExportLimits::default()
                }
            ),
            Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::OfflineRebuild
            ))
        ));
    }

    #[test]
    fn interrupted_manifest_round_trip_resumes_from_first_unprocessed_byte() {
        let mut store = store_with_rebuild(StoreId(90));
        let first = store.snapshot().rebuilds[0]
            .durable_chunk_receipt(RecoveryGeneration(1), 4, fence(StoreId(90), 1))
            .unwrap();
        let mut advance = store.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(7));
        advance.advance_offline_rebuild(first);
        store.commit_durable(advance).unwrap();

        let manifest = store.export_manifest(RecoveryGeneration(2)).unwrap();
        let mut resumed = MemoryRecoveryStore::from_manifest(manifest.clone()).unwrap();
        assert_eq!(resumed.snapshot(), &manifest.snapshot);
        assert_eq!(resumed.snapshot().rebuilds[0].cursor().offset(), 4);

        let next = resumed.snapshot().rebuilds[0]
            .durable_chunk_receipt(RecoveryGeneration(2), 8, fence(StoreId(90), 2))
            .unwrap();
        let mut advance = resumed.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(7));
        advance.advance_offline_rebuild(next);
        resumed.commit_durable(advance).unwrap();
        assert_eq!(resumed.snapshot().rebuilds[0].cursor().offset(), 8);
    }

    #[test]
    fn final_verification_prepares_but_does_not_publish_replacement_topology() {
        let mut store = store_with_rebuild(StoreId(90));
        for (through, watermark) in [(4, 1), (8, 2), (12, 3), (16, 4)] {
            let generation = store.snapshot().generation;
            let receipt = store.snapshot().rebuilds[0]
                .durable_chunk_receipt(generation, through, fence(StoreId(90), watermark))
                .unwrap();
            let mut advance = store.begin_protocol_txn(generation, TopologyEpoch(7));
            advance.advance_offline_rebuild(receipt);
            store.commit_durable(advance).unwrap();
        }
        assert_eq!(
            store.snapshot().rebuilds[0].lifecycle(),
            RebuildLifecycle::AwaitingFinalVerification
        );
        assert!(matches!(
            store.snapshot().rebuilds[0]
                .prepared_replacement_topology(TopologyEpoch(8), AssignmentGeneration(4),),
            Err(RebuildError::FinalVerificationIncomplete)
        ));

        let generation = store.snapshot().generation;
        let completion = store.snapshot().rebuilds[0]
            .final_verification_receipt(generation, [7; 32])
            .unwrap();
        assert_eq!(completion.replacement_digest(), [7; 32]);
        let mut complete = store.begin_protocol_txn(generation, TopologyEpoch(7));
        complete.complete_offline_rebuild(completion);
        store.commit_durable(complete).unwrap();
        assert_eq!(
            store.snapshot().rebuilds[0].lifecycle(),
            RebuildLifecycle::Verified
        );
        assert_eq!(
            store.snapshot().rebuilds[0].verified_digest(),
            Some([7; 32])
        );

        let prepared = store.snapshot().rebuilds[0]
            .prepared_replacement_topology(TopologyEpoch(8), AssignmentGeneration(4))
            .unwrap();
        let replacement = prepared
            .assignments()
            .iter()
            .find(|assignment| assignment.slot_id() == MISSING_SLOT)
            .unwrap();
        assert_eq!(replacement.coding_position(), CodingPosition(0));
        assert_eq!(replacement.assignment_instance(), REPLACEMENT_ASSIGNMENT);
        assert_eq!(replacement.assignment_generation(), AssignmentGeneration(4));
        assert_eq!(replacement.store_id(), StoreId(90));
        assert_eq!(prepared.topology_epoch(), TopologyEpoch(8));
        assert_eq!(store.snapshot().topology_epoch, TopologyEpoch(7));
        assert!(store.snapshot().pending_topology.is_none());
    }
}
