//! Durable intent operations over the recovery-state adapter.

use crate::{
    IntentDecision, IntentEvidence, InvalidationTarget, RecoveryCommitObservation, RecoveryError,
    RecoveryGeneration, RecoveryStateStore, assess_intent,
};
use dwv_core::TopologyEpoch;

pub struct IntentCommit<'a, S: RecoveryStateStore + ?Sized> {
    store: &'a mut S,
    topology_epoch: TopologyEpoch,
    expected_generation: RecoveryGeneration,
    target: InvalidationTarget,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryRecoveryStore, RegionId};

    #[test]
    fn intent_commit_is_atomic_and_repeated_intent_is_idempotent() {
        let mut store = MemoryRecoveryStore::new(TopologyEpoch(3));
        let target = InvalidationTarget::new(vec![RegionId(1)], vec![]);
        let evidence = IntentCommit::new(
            &mut store,
            TopologyEpoch(3),
            RecoveryGeneration(0),
            target.clone(),
        )
        .commit()
        .unwrap();
        assert_eq!(evidence.committed_generation, RecoveryGeneration(1));
        assert!(evidence.durable);
        let repeated =
            IntentCommit::new(&mut store, TopologyEpoch(3), RecoveryGeneration(1), target)
                .commit()
                .unwrap();
        assert_eq!(repeated.committed_generation, RecoveryGeneration(1));
        assert_eq!(store.snapshot().generation, RecoveryGeneration(1));
    }

    #[test]
    fn rejected_observation_does_not_mutate_state() {
        let mut store = MemoryRecoveryStore::new(TopologyEpoch(3));
        let target = InvalidationTarget::new(vec![RegionId(1)], vec![]);
        let result = IntentCommit::new(&mut store, TopologyEpoch(3), RecoveryGeneration(0), target)
            .commit_observed(RecoveryCommitObservation::Lost);
        assert!(matches!(result, Err(RecoveryError::CommitNotDurable(_))));
        assert_eq!(store.snapshot().generation, RecoveryGeneration(0));
        assert!(store.snapshot().dirty_regions.is_empty());
    }
}

impl<'a, S: RecoveryStateStore + ?Sized> IntentCommit<'a, S> {
    pub fn new(
        store: &'a mut S,
        topology_epoch: TopologyEpoch,
        expected_generation: RecoveryGeneration,
        target: InvalidationTarget,
    ) -> Self {
        Self {
            store,
            topology_epoch,
            expected_generation,
            target,
        }
    }

    pub fn assess(&self) -> Result<IntentDecision, RecoveryError> {
        let snapshot = self.store.load_assembly_snapshot()?;
        assess_intent(
            &snapshot,
            self.topology_epoch,
            self.expected_generation,
            self.target.clone(),
        )
    }

    pub fn commit(self) -> Result<IntentEvidence, RecoveryError> {
        let decision = self.assess()?;
        if let IntentDecision::AlreadyCovered(coverage) = decision {
            return Ok(IntentEvidence::already_covered(
                coverage.topology_epoch,
                coverage.generation,
                coverage.target,
            ));
        }

        let target = self.target;
        let mut txn = self
            .store
            .begin_protocol_txn(self.expected_generation, self.topology_epoch);
        for region in &target.regions {
            txn.mark_region_dirty(*region, self.expected_generation);
        }
        for extent in &target.checksum_extents {
            txn.mark_integrity_stale(*extent, self.expected_generation);
        }
        let committed_generation = self.store.commit_durable(txn)?;
        Ok(IntentEvidence {
            topology_epoch: self.topology_epoch,
            captured_generation: self.expected_generation,
            committed_generation,
            target,
            durable: true,
        })
    }

    pub fn commit_observed(
        self,
        observation: RecoveryCommitObservation,
    ) -> Result<IntentEvidence, RecoveryError> {
        if !observation.is_durable() {
            return Err(RecoveryError::CommitNotDurable(observation));
        }
        self.commit()
    }
}
