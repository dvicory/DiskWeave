//! Deterministic evidence for recovery transitions and conservative failures.

use crate::{IntegrityExtentId, RecoveryGeneration, RegionId};
use dwv_core::TopologyEpoch;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecoveryTransitionId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionOutcome {
    Applied,
    Refused,
    Uncertain,
    Crashed,
    ReconciliationRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionKind {
    WriteRecoveryRecord,
    DataParityWrite,
    Fence,
    RecoveryClean,
    SessionClose,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionEvidence {
    pub transaction_id: RecoveryTransitionId,
    pub sequence: u64,
    pub kind: TransitionKind,
    pub outcome: TransitionOutcome,
    pub topology_epoch: TopologyEpoch,
    pub captured_generation: RecoveryGeneration,
    pub current_generation: RecoveryGeneration,
    pub regions: Vec<RegionId>,
    pub checksum_extents: Vec<IntegrityExtentId>,
    pub missing_evidence: Vec<String>,
}

impl TransitionEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transaction_id: RecoveryTransitionId,
        sequence: u64,
        kind: TransitionKind,
        outcome: TransitionOutcome,
        topology_epoch: TopologyEpoch,
        captured_generation: RecoveryGeneration,
        current_generation: RecoveryGeneration,
        mut regions: Vec<RegionId>,
        mut checksum_extents: Vec<IntegrityExtentId>,
        mut missing_evidence: Vec<String>,
    ) -> Self {
        regions.sort_unstable();
        regions.dedup();
        checksum_extents.sort_unstable();
        checksum_extents.dedup();
        missing_evidence.sort();
        missing_evidence.dedup();
        Self {
            transaction_id,
            sequence,
            kind,
            outcome,
            topology_epoch,
            captured_generation,
            current_generation,
            regions,
            checksum_extents,
            missing_evidence,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct TransitionTrace {
    events: Vec<TransitionEvidence>,
}

impl TransitionTrace {
    pub fn record(&mut self, event: TransitionEvidence) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[TransitionEvidence] {
        &self.events
    }

    pub fn replay(&self) -> Self {
        Self {
            events: self.events.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_normalizes_all_unordered_fields() {
        let event = TransitionEvidence::new(
            RecoveryTransitionId(4),
            2,
            TransitionKind::WriteRecoveryRecord,
            TransitionOutcome::Uncertain,
            TopologyEpoch(3),
            RecoveryGeneration(7),
            RecoveryGeneration(8),
            vec![RegionId(2), RegionId(1), RegionId(2)],
            vec![
                IntegrityExtentId(9),
                IntegrityExtentId(8),
                IntegrityExtentId(9),
            ],
            vec!["z".into(), "a".into(), "a".into()],
        );
        assert_eq!(event.regions, vec![RegionId(1), RegionId(2)]);
        assert_eq!(
            event.checksum_extents,
            vec![IntegrityExtentId(8), IntegrityExtentId(9)]
        );
        assert_eq!(event.missing_evidence, vec!["a", "z"]);
    }

    #[test]
    fn trace_replay_is_exact() {
        let event = TransitionEvidence::new(
            RecoveryTransitionId(1),
            0,
            TransitionKind::RecoveryClean,
            TransitionOutcome::Refused,
            TopologyEpoch(1),
            RecoveryGeneration(2),
            RecoveryGeneration(3),
            vec![],
            vec![],
            vec!["fence".into()],
        );
        let mut trace = TransitionTrace::default();
        trace.record(event);
        assert_eq!(trace, trace.replay());
    }
}
