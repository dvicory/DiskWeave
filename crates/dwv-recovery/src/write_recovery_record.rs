//! Durable write-recovery record operations over the recovery-state adapter.

use crate::{
    DurableRecoveryCommit, InvalidationTarget, RecoveryCommitObservation, RecoveryError,
    RecoveryGeneration, RecoveryMutation, RecoveryStateStore, WriteRecoveryRecordDecision,
    WriteRecoveryRecordEvidence, assess_write_recovery_record,
};
use dwv_core::TopologyEpoch;

pub struct WriteRecoveryRecordCommit<'a, S: RecoveryStateStore + ?Sized> {
    store: &'a mut S,
    topology_epoch: TopologyEpoch,
    expected_generation: RecoveryGeneration,
    target: InvalidationTarget,
    additional_mutations: Vec<RecoveryMutation>,
}

pub struct WriteRecoveryRecordCommitResult {
    pub evidence: WriteRecoveryRecordEvidence,
    pub receipt: Option<DurableRecoveryCommit>,
}

impl<'a, S: RecoveryStateStore + ?Sized> WriteRecoveryRecordCommit<'a, S> {
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
            additional_mutations: Vec::new(),
        }
    }

    pub fn with_mutations(mut self, mutations: impl IntoIterator<Item = RecoveryMutation>) -> Self {
        self.additional_mutations.extend(mutations);
        self
    }

    pub fn assess(&self) -> Result<WriteRecoveryRecordDecision, RecoveryError> {
        let snapshot = self.store.load_assembly_snapshot()?;
        assess_write_recovery_record(
            &snapshot,
            self.topology_epoch,
            self.expected_generation,
            self.target.clone(),
        )
    }

    pub fn commit(self) -> Result<WriteRecoveryRecordEvidence, RecoveryError> {
        self.commit_with_receipt().map(|result| result.evidence)
    }

    pub fn commit_with_receipt(self) -> Result<WriteRecoveryRecordCommitResult, RecoveryError> {
        let decision = self.assess()?;
        let already_covered = matches!(&decision, WriteRecoveryRecordDecision::AlreadyCovered(_));
        if self.additional_mutations.is_empty()
            && let WriteRecoveryRecordDecision::AlreadyCovered(coverage) = decision
        {
            return Ok(WriteRecoveryRecordCommitResult {
                evidence: WriteRecoveryRecordEvidence::already_covered(
                    coverage.topology_epoch,
                    coverage.generation,
                    coverage.target,
                ),
                receipt: None,
            });
        }

        let target = self.target;
        let mut txn = self
            .store
            .begin_protocol_txn(self.expected_generation, self.topology_epoch);
        if !already_covered {
            for region in &target.regions {
                txn.mark_region_dirty(*region, self.expected_generation);
            }
            for extent in &target.checksum_extents {
                txn.mark_integrity_stale(*extent, self.expected_generation);
            }
        }
        for mutation in self.additional_mutations {
            txn.push(mutation);
        }
        let receipt = self.store.commit_durable_receipt(txn)?;
        let committed_generation = receipt.generation();
        Ok(WriteRecoveryRecordCommitResult {
            evidence: WriteRecoveryRecordEvidence {
                topology_epoch: self.topology_epoch,
                captured_generation: self.expected_generation,
                committed_generation,
                target,
                durable: true,
            },
            receipt: Some(receipt),
        })
    }

    pub fn commit_observed(
        self,
        observation: RecoveryCommitObservation,
    ) -> Result<WriteRecoveryRecordEvidence, RecoveryError> {
        if !observation.is_durable() {
            return Err(RecoveryError::CommitNotDurable(observation));
        }
        self.commit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryRecoveryStore, RegionId};

    #[test]
    fn write_recovery_record_commit_is_atomic_and_repeated_record_is_idempotent() {
        let mut store = MemoryRecoveryStore::new(TopologyEpoch(3));
        let target = InvalidationTarget::new(vec![RegionId(1)], vec![]);
        let evidence = WriteRecoveryRecordCommit::new(
            &mut store,
            TopologyEpoch(3),
            RecoveryGeneration(0),
            target.clone(),
        )
        .commit()
        .unwrap();
        assert_eq!(evidence.committed_generation, RecoveryGeneration(1));
        assert!(evidence.durable);
        let repeated = WriteRecoveryRecordCommit::new(
            &mut store,
            TopologyEpoch(3),
            RecoveryGeneration(1),
            target,
        )
        .commit()
        .unwrap();
        assert_eq!(repeated.committed_generation, RecoveryGeneration(1));
        assert_eq!(store.snapshot().generation, RecoveryGeneration(1));
    }

    #[test]
    fn rejected_observation_does_not_mutate_state() {
        let mut store = MemoryRecoveryStore::new(TopologyEpoch(3));
        let target = InvalidationTarget::new(vec![RegionId(1)], vec![]);
        let result = WriteRecoveryRecordCommit::new(
            &mut store,
            TopologyEpoch(3),
            RecoveryGeneration(0),
            target,
        )
        .commit_observed(RecoveryCommitObservation::Lost);
        assert!(matches!(result, Err(RecoveryError::CommitNotDurable(_))));
        assert_eq!(store.snapshot().generation, RecoveryGeneration(0));
        assert!(store.snapshot().dirty_regions.is_empty());
    }
}
