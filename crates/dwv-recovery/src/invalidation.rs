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
pub enum IntentBoundary {
    CleanRegion,
    ValidChecksum,
    UnknownRegion,
    AbsentChecksum,
    TopologyChanged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentCoverage {
    pub topology_epoch: TopologyEpoch,
    pub generation: RecoveryGeneration,
    pub target: InvalidationTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntentDecision {
    AlreadyCovered(IntentCoverage),
    RequiresDurableCommit {
        coverage: IntentCoverage,
        boundaries: Vec<IntentBoundary>,
    },
}

impl IntentDecision {
    pub const fn requires_commit(&self) -> bool {
        matches!(self, Self::RequiresDurableCommit { .. })
    }

    pub fn coverage(&self) -> &IntentCoverage {
        match self {
            Self::AlreadyCovered(coverage) | Self::RequiresDurableCommit { coverage, .. } => {
                coverage
            }
        }
    }
}

/// Determine whether an identical durable intent is already sufficient.
pub fn assess_intent(
    snapshot: &RecoverySnapshot,
    topology_epoch: TopologyEpoch,
    generation: RecoveryGeneration,
    target: InvalidationTarget,
) -> Result<IntentDecision, RecoveryError> {
    if snapshot.topology_epoch != topology_epoch {
        return Err(RecoveryError::TopologyMismatch {
            expected: topology_epoch,
            actual: snapshot.topology_epoch,
        });
    }

    let coverage = IntentCoverage {
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
            Some(RegionState::Indeterminate) => boundaries.push(IntentBoundary::UnknownRegion),
            _ => boundaries.push(IntentBoundary::CleanRegion),
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
            Some(IntegrityState::Absent) | None => boundaries.push(IntentBoundary::AbsentChecksum),
            _ => boundaries.push(IntentBoundary::ValidChecksum),
        }
    }

    if boundaries.is_empty() {
        Ok(IntentDecision::AlreadyCovered(coverage))
    } else {
        Ok(IntentDecision::RequiresDurableCommit {
            coverage,
            boundaries,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentEvidence {
    pub topology_epoch: TopologyEpoch,
    pub captured_generation: RecoveryGeneration,
    pub committed_generation: RecoveryGeneration,
    pub target: InvalidationTarget,
    pub durable: bool,
}

impl IntentEvidence {
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
    fn clean_or_valid_boundaries_require_a_new_intent() {
        let snapshot = MemoryRecoveryStore::new(TopologyEpoch(4))
            .snapshot()
            .clone();
        let decision = assess_intent(
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
        let decision = assess_intent(
            store.snapshot(),
            TopologyEpoch(4),
            RecoveryGeneration(1),
            target,
        )
        .unwrap();
        assert!(matches!(decision, IntentDecision::AlreadyCovered(_)));
        assert_eq!(store.snapshot().integrity_records.len(), 1);
    }
}
