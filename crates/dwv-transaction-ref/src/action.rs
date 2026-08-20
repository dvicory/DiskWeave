use crate::error::ErrorClass;
use dwv_core::{ByteRange, FenceDomain, TopologyEpoch};
use dwv_recovery::{
    FenceCertificate, IntegrityExtentId, RecoveryGeneration, RegionId, SessionId,
    WriteRecoveryRecordEvidence,
};
use dwv_store::{StoreFenceRef, StoreId, StoreWriteWatermark};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ParityRange {
    pub region: RegionId,
    pub store: StoreId,
    pub range: ByteRange,
}

impl ParityRange {
    pub const fn new(region: RegionId, store: StoreId, range: ByteRange) -> Self {
        Self {
            region,
            store,
            range,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PlannedRead {
    pub store: StoreId,
    pub range: ByteRange,
}

impl PlannedRead {
    pub const fn new(store: StoreId, range: ByteRange) -> Self {
        Self { store, range }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PlannedWrite {
    pub store: StoreId,
    pub range: ByteRange,
}

impl PlannedWrite {
    pub const fn new(store: StoreId, range: ByteRange) -> Self {
        Self { store, range }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StoreWatermark {
    pub store: StoreId,
    pub through: StoreWriteWatermark,
    pub incarnation: dwv_store::StoreIncarnationId,
}

impl StoreWatermark {
    pub const fn new(store: StoreId, through: StoreWriteWatermark) -> Self {
        Self {
            store,
            through,
            incarnation: dwv_store::StoreIncarnationId(0),
        }
    }

    pub const fn for_incarnation(
        store: StoreId,
        incarnation: dwv_store::StoreIncarnationId,
        through: StoreWriteWatermark,
    ) -> Self {
        Self {
            store,
            through,
            incarnation,
        }
    }
}

pub type StoreWatermarks = Vec<StoreWatermark>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ParityComputationPlan {
    pub topology_epoch: TopologyEpoch,
    pub range: ByteRange,
    pub input_count: u16,
}

impl ParityComputationPlan {
    pub const fn new(topology_epoch: TopologyEpoch, range: ByteRange, input_count: u16) -> Self {
        Self {
            topology_epoch,
            range,
            input_count,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RangeGuardToken(pub u64);

impl RangeGuardToken {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CommittedRecoveryGeneration {
    pub generation: RecoveryGeneration,
    pub topology_epoch: TopologyEpoch,
}

impl CommittedRecoveryGeneration {
    pub const fn new(generation: RecoveryGeneration, topology_epoch: TopologyEpoch) -> Self {
        Self {
            generation,
            topology_epoch,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WriteRecoveryRecordRequirement {
    CommitRequired,
    AlreadyCovered {
        durable_generation: RecoveryGeneration,
    },
}

impl WriteRecoveryRecordRequirement {
    pub const fn requires_commit(self) -> bool {
        matches!(self, Self::CommitRequired)
    }

    pub const fn generation(self, captured: RecoveryGeneration) -> RecoveryGeneration {
        match self {
            Self::CommitRequired => captured,
            Self::AlreadyCovered { durable_generation } => durable_generation,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionAction {
    AcquireRange {
        ranges: Vec<ParityRange>,
    },
    PersistDirtyAndInvalidateIntegrity {
        dirty_regions: Vec<RegionId>,
        checksum_extents: Vec<IntegrityExtentId>,
    },
    ReadSet {
        reads: Vec<PlannedRead>,
    },
    ComputeParity {
        plan: ParityComputationPlan,
    },
    WriteSet {
        writes: Vec<PlannedWrite>,
    },
    FlushSet {
        stores: Vec<StoreId>,
        through: StoreWatermarks,
    },
    CommitRecoveryClean {
        certificate: FenceCertificate,
    },
    ReleaseRange,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ActionKind {
    AcquireRange,
    PersistDirtyAndInvalidateIntegrity,
    ReadSet,
    ComputeParity,
    WriteSet,
    FlushSet,
    CommitRecoveryClean,
    ReleaseRange,
}

impl TransactionAction {
    pub const fn kind(&self) -> ActionKind {
        match self {
            Self::AcquireRange { .. } => ActionKind::AcquireRange,
            Self::PersistDirtyAndInvalidateIntegrity { .. } => {
                ActionKind::PersistDirtyAndInvalidateIntegrity
            }
            Self::ReadSet { .. } => ActionKind::ReadSet,
            Self::ComputeParity { .. } => ActionKind::ComputeParity,
            Self::WriteSet { .. } => ActionKind::WriteSet,
            Self::FlushSet { .. } => ActionKind::FlushSet,
            Self::CommitRecoveryClean { .. } => ActionKind::CommitRecoveryClean,
            Self::ReleaseRange => ActionKind::ReleaseRange,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResultKind {
    RangeAcquired,
    WriteRecoveryRecordDurable,
    ReadSetComplete,
    ParityComputed,
    WriteSetComplete,
    FlushSetComplete,
    RecoveryCleanCommitted,
    RangeReleased,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticIoResult {
    Complete,
    Failed,
    Uncertain,
}

impl SemanticIoResult {
    pub const fn complete() -> Self {
        Self::Complete
    }

    pub const fn failed() -> Self {
        Self::Failed
    }

    pub const fn uncertain() -> Self {
        Self::Uncertain
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputationResult {
    Complete,
    Failed,
}

impl ComputationResult {
    pub const fn complete() -> Self {
        Self::Complete
    }

    pub const fn failed() -> Self {
        Self::Failed
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionPersistenceEvidence {
    pub certificate: FenceCertificate,
    pub durable: bool,
    pub uncertain: bool,
}

impl TransactionPersistenceEvidence {
    pub fn durable(certificate: FenceCertificate) -> Self {
        Self {
            certificate,
            durable: true,
            uncertain: false,
        }
    }

    pub fn volatile(certificate: FenceCertificate) -> Self {
        Self {
            certificate,
            durable: false,
            uncertain: false,
        }
    }

    pub fn uncertain(certificate: FenceCertificate) -> Self {
        Self {
            certificate,
            durable: false,
            uncertain: true,
        }
    }

    pub const fn is_durable(&self) -> bool {
        self.durable && !self.uncertain
    }

    pub fn covers(
        &self,
        topology_epoch: TopologyEpoch,
        fence_domain: FenceDomain,
        stores: &[StoreWatermark],
        regions: &[RegionId],
        extents: &[IntegrityExtentId],
        generation: RecoveryGeneration,
    ) -> bool {
        if !self.is_durable()
            || self.certificate.topology_epoch != topology_epoch
            || self.certificate.fence_domain != fence_domain
        {
            return false;
        }

        let stores_covered = stores.iter().all(|required| {
            self.certificate.stores.iter().any(|observed| {
                observed.store_id == required.store
                    && observed.store_incarnation == required.incarnation
                    && observed.topology_epoch == topology_epoch
                    && observed.through.0 >= required.through.0
            })
        });
        let regions_covered = regions.iter().all(|region| {
            self.certificate
                .captured_region_generations
                .iter()
                .any(|(captured, observed)| *captured == *region && observed.0 >= generation.0)
        });
        let extents_covered = extents.iter().all(|extent| {
            self.certificate
                .captured_integrity_generations
                .iter()
                .any(|(captured, observed)| *captured == *extent && observed.0 >= generation.0)
        });
        stores_covered && regions_covered && extents_covered
    }

    pub fn store_fences(&self) -> &[StoreFenceRef] {
        &self.certificate.stores
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticFailure {
    pub class: ErrorClass,
}

impl SemanticFailure {
    pub const fn new(class: ErrorClass) -> Self {
        Self { class }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionResult {
    RangeAcquired(RangeGuardToken),
    WriteRecoveryRecordDurable(CommittedRecoveryGeneration),
    WriteRecoveryRecordDurableWithEvidence(WriteRecoveryRecordEvidence),
    ReadSetComplete(SemanticIoResult),
    ParityComputed(ComputationResult),
    WriteSetComplete(SemanticIoResult),
    FlushSetComplete(TransactionPersistenceEvidence),
    RecoveryCleanCommitted(CommittedRecoveryGeneration),
    RangeReleased,
    Failed(SemanticFailure),
}

impl ActionResult {
    pub const fn kind(&self) -> ResultKind {
        match self {
            Self::RangeAcquired(_) => ResultKind::RangeAcquired,
            Self::WriteRecoveryRecordDurable(_) => ResultKind::WriteRecoveryRecordDurable,
            Self::WriteRecoveryRecordDurableWithEvidence(_) => {
                ResultKind::WriteRecoveryRecordDurable
            }
            Self::ReadSetComplete(_) => ResultKind::ReadSetComplete,
            Self::ParityComputed(_) => ResultKind::ParityComputed,
            Self::WriteSetComplete(_) => ResultKind::WriteSetComplete,
            Self::FlushSetComplete(_) => ResultKind::FlushSetComplete,
            Self::RecoveryCleanCommitted(_) => ResultKind::RecoveryCleanCommitted,
            Self::RangeReleased => ResultKind::RangeReleased,
            Self::Failed(_) => ResultKind::Failed,
        }
    }

    pub const fn failure(class: ErrorClass) -> Self {
        Self::Failed(SemanticFailure::new(class))
    }
}

/// The semantic session identity is carried in the plan, not in backend I/O.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransactionIdentity {
    pub session: SessionId,
    pub topology_epoch: TopologyEpoch,
    pub recovery_generation: RecoveryGeneration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_multi_store_fence_is_rejected() {
        let epoch = TopologyEpoch(3);
        let generation = RecoveryGeneration(4);
        let stores = [
            StoreWatermark::for_incarnation(
                StoreId(1),
                dwv_store::StoreIncarnationId(11),
                StoreWriteWatermark(7),
            ),
            StoreWatermark::for_incarnation(
                StoreId(2),
                dwv_store::StoreIncarnationId(22),
                StoreWriteWatermark(8),
            ),
        ];
        let certificate = FenceCertificate::new(
            epoch,
            FenceDomain(5),
            vec![StoreFenceRef {
                fence_id: dwv_store::FenceId(1),
                store_id: StoreId(1),
                store_incarnation: dwv_store::StoreIncarnationId(11),
                topology_epoch: epoch,
                through: StoreWriteWatermark(7),
                capability_evidence_id: dwv_store::CapabilityEvidenceId(1),
            }],
            vec![(RegionId(7), generation), (RegionId(8), generation)],
        )
        .with_integrity_extent(IntegrityExtentId(9), generation);
        assert!(
            !TransactionPersistenceEvidence::durable(certificate).covers(
                epoch,
                FenceDomain(5),
                &stores,
                &[RegionId(7), RegionId(8)],
                &[IntegrityExtentId(9)],
                generation,
            )
        );
    }
}

#[cfg(kani)]
mod kani_verification {
    use super::*;

    #[kani::proof]
    fn fence_coverage_requires_every_store_region_and_incarnation() {
        let first_required: u8 = kani::any();
        let second_required: u8 = kani::any();
        let first_observed: u8 = kani::any();
        let second_observed: u8 = kani::any();
        let wrong_second_incarnation: bool = kani::any();
        let epoch = TopologyEpoch(3);
        let generation = RecoveryGeneration(4);
        let stores = [
            StoreWatermark::for_incarnation(
                StoreId(1),
                dwv_store::StoreIncarnationId(11),
                StoreWriteWatermark(u64::from(first_required)),
            ),
            StoreWatermark::for_incarnation(
                StoreId(2),
                dwv_store::StoreIncarnationId(22),
                StoreWriteWatermark(u64::from(second_required)),
            ),
        ];
        let certificate =
            FenceCertificate::new(
                epoch,
                FenceDomain(5),
                vec![
                    StoreFenceRef {
                        fence_id: dwv_store::FenceId(1),
                        store_id: StoreId(1),
                        store_incarnation: dwv_store::StoreIncarnationId(11),
                        topology_epoch: epoch,
                        through: StoreWriteWatermark(u64::from(first_observed)),
                        capability_evidence_id: dwv_store::CapabilityEvidenceId(1),
                    },
                    StoreFenceRef {
                        fence_id: dwv_store::FenceId(2),
                        store_id: StoreId(2),
                        store_incarnation: dwv_store::StoreIncarnationId(
                            if wrong_second_incarnation { 23 } else { 22 },
                        ),
                        topology_epoch: epoch,
                        through: StoreWriteWatermark(u64::from(second_observed)),
                        capability_evidence_id: dwv_store::CapabilityEvidenceId(2),
                    },
                ],
                vec![(RegionId(7), generation), (RegionId(8), generation)],
            )
            .with_integrity_extent(IntegrityExtentId(9), generation)
            .with_integrity_extent(IntegrityExtentId(10), generation);
        let covered = TransactionPersistenceEvidence::durable(certificate).covers(
            epoch,
            FenceDomain(5),
            &stores,
            &[RegionId(7), RegionId(8)],
            &[IntegrityExtentId(9), IntegrityExtentId(10)],
            generation,
        );
        assert_eq!(
            covered,
            !wrong_second_incarnation
                && first_observed >= first_required
                && second_observed >= second_required
        );
    }
}
