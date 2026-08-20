//! Dirty-region and integrity-boundary decisions.

use crate::{
    IntegrityExtentId, IntegrityState, RecoveryError, RecoveryGeneration, RecoverySnapshot,
    RegionId, RegionState,
};
use dwv_core::TopologyEpoch;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidationTarget {
    pub regions: Vec<RegionId>,
    pub checksum_extents: Vec<IntegrityExtentId>,
}

impl InvalidationTarget {
    pub fn new(mut regions: Vec<RegionId>, mut checksum_extents: Vec<IntegrityExtentId>) -> Self {
        regions.sort_unstable();
        regions.dedup();
        checksum_extents.sort_unstable();
        checksum_extents.dedup();
        Self {
            regions,
            checksum_extents,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty() && self.checksum_extents.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteRecoveryRecordBoundary {
    CleanRegion,
    ValidChecksum,
    UnknownRegion,
    AbsentChecksum,
    TopologyChanged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteRecoveryRecordCoverage {
    pub topology_epoch: TopologyEpoch,
    pub generation: RecoveryGeneration,
    pub target: InvalidationTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteRecoveryRecordDecision {
    AlreadyCovered(WriteRecoveryRecordCoverage),
    RequiresDurableCommit {
        coverage: WriteRecoveryRecordCoverage,
        boundaries: Vec<WriteRecoveryRecordBoundary>,
    },
}

impl WriteRecoveryRecordDecision {
    pub const fn requires_commit(&self) -> bool {
        matches!(self, Self::RequiresDurableCommit { .. })
    }

    pub fn coverage(&self) -> &WriteRecoveryRecordCoverage {
        match self {
            Self::AlreadyCovered(coverage) | Self::RequiresDurableCommit { coverage, .. } => {
                coverage
            }
        }
    }
}

/// Determine whether an identical durable write-recovery record is already sufficient.
pub fn assess_write_recovery_record(
    snapshot: &RecoverySnapshot,
    topology_epoch: TopologyEpoch,
    generation: RecoveryGeneration,
    target: InvalidationTarget,
) -> Result<WriteRecoveryRecordDecision, RecoveryError> {
    if snapshot.topology_epoch != topology_epoch {
        return Err(RecoveryError::TopologyMismatch {
            expected: topology_epoch,
            actual: snapshot.topology_epoch,
        });
    }

    let coverage = WriteRecoveryRecordCoverage {
        topology_epoch,
        generation,
        target,
    };
    let mut boundaries = Vec::new();

    for region in &coverage.target.regions {
        match snapshot
            .dirty_regions
            .iter()
            .find(|record| record.region == *region)
            .map(|record| record.state)
        {
            Some(RegionState::Dirty { dirty_since }) if dirty_since <= generation => {}
            Some(RegionState::Indeterminate) => {
                boundaries.push(WriteRecoveryRecordBoundary::UnknownRegion)
            }
            _ => boundaries.push(WriteRecoveryRecordBoundary::CleanRegion),
        }
    }

    for extent in &coverage.target.checksum_extents {
        match snapshot
            .integrity_records
            .iter()
            .find(|record| record.extent == *extent)
            .map(|record| &record.state)
        {
            Some(IntegrityState::Stale { stale_generation }) if *stale_generation <= generation => {
            }
            Some(IntegrityState::Absent) | None => {
                boundaries.push(WriteRecoveryRecordBoundary::AbsentChecksum)
            }
            _ => boundaries.push(WriteRecoveryRecordBoundary::ValidChecksum),
        }
    }

    if boundaries.is_empty() {
        Ok(WriteRecoveryRecordDecision::AlreadyCovered(coverage))
    } else {
        Ok(WriteRecoveryRecordDecision::RequiresDurableCommit {
            coverage,
            boundaries,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteRecoveryRecordEvidence {
    pub topology_epoch: TopologyEpoch,
    pub captured_generation: RecoveryGeneration,
    pub committed_generation: RecoveryGeneration,
    pub target: InvalidationTarget,
    pub durable: bool,
}

impl WriteRecoveryRecordEvidence {
    pub const fn already_covered(
        topology_epoch: TopologyEpoch,
        generation: RecoveryGeneration,
        target: InvalidationTarget,
    ) -> Self {
        Self {
            topology_epoch,
            captured_generation: generation,
            committed_generation: generation,
            target,
            durable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryRecoveryStore, RecoveryStateStore};
    use dwv_core::TopologyEpoch;

    #[test]
    fn clean_or_valid_boundaries_require_a_new_write_recovery_record() {
        let snapshot = MemoryRecoveryStore::new(TopologyEpoch(4))
            .snapshot()
            .clone();
        let decision = assess_write_recovery_record(
            &snapshot,
            TopologyEpoch(4),
            RecoveryGeneration(0),
            InvalidationTarget::new(vec![RegionId(2)], vec![IntegrityExtentId(8)]),
        )
        .unwrap();
        assert!(decision.requires_commit());
        assert_eq!(decision.coverage().target.regions, vec![RegionId(2)]);
        assert_eq!(
            decision.coverage().target.checksum_extents,
            vec![IntegrityExtentId(8)]
        );
    }

    #[test]
    fn duplicate_targets_are_normalized_before_assessment() {
        let mut store = MemoryRecoveryStore::new(TopologyEpoch(4));
        let target =
            InvalidationTarget::new(vec![RegionId(2), RegionId(2)], vec![IntegrityExtentId(8)]);
        let mut txn = store.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(4));
        txn.mark_region_dirty(RegionId(2), RecoveryGeneration(0));
        txn.mark_integrity_stale(IntegrityExtentId(8), RecoveryGeneration(0));
        store.commit_durable(txn).unwrap();
        let decision = assess_write_recovery_record(
            store.snapshot(),
            TopologyEpoch(4),
            RecoveryGeneration(1),
            target,
        )
        .unwrap();
        assert!(matches!(
            decision,
            WriteRecoveryRecordDecision::AlreadyCovered(_)
        ));
        assert_eq!(store.snapshot().integrity_records.len(), 1);
    }
}
