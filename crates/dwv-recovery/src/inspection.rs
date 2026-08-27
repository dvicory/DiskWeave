use crate::{
    CodedCaptureId, CodedCaptureMembership, CodedCapturePhase, CodedCaptureReconciliationReceipt,
    MemoryRecoveryStore, RecoveryManifest,
};
use dwv_store::OperationSlotToken;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RecoveryFormatLayer {
    Storage,
    Semantic,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RecoveryInspection {
    Absent,
    Supported(Box<RecoveryManifest>),
    CorruptOrUnreadable {
        storage_version: Option<u64>,
        semantic_version: Option<u64>,
    },
    Unsupported {
        layer: RecoveryFormatLayer,
        version: Option<u64>,
        storage_version: Option<u64>,
    },
    MigrationRequired {
        layer: RecoveryFormatLayer,
        from: u64,
        to: u64,
        storage_version: Option<u64>,
    },
    ReconciliationRequired,
}

impl RecoveryInspection {
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Supported(_) => "supported",
            Self::CorruptOrUnreadable { .. } => "corrupt-or-unreadable",
            Self::Unsupported { .. } => "unsupported",
            Self::MigrationRequired { .. } => "migration-required",
            Self::ReconciliationRequired => "reconciliation-required",
        }
    }

    pub fn manifest(&self) -> Option<&RecoveryManifest> {
        match self {
            Self::Supported(manifest) => Some(manifest),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryReconciliation {
    Prior,
    Proposed,
    ReconciliationRequired,
}

/// dwv:req req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations
pub fn reconcile_uncertain_commit(
    prior: &RecoveryManifest,
    proposed: &RecoveryManifest,
    reopened: &RecoveryInspection,
) -> RecoveryReconciliation {
    if MemoryRecoveryStore::from_manifest(prior.clone()).is_err()
        || MemoryRecoveryStore::from_manifest(proposed.clone()).is_err()
    {
        return RecoveryReconciliation::ReconciliationRequired;
    }
    let Some(reopened) = reopened.manifest() else {
        return RecoveryReconciliation::ReconciliationRequired;
    };
    if reopened == prior {
        RecoveryReconciliation::Prior
    } else if reopened == proposed {
        RecoveryReconciliation::Proposed
    } else {
        RecoveryReconciliation::ReconciliationRequired
    }
}

/// Resolve one exact uncertain CLEAN commit from independently reopened state.
pub fn reconcile_coded_clean_attempt(
    prior: &RecoveryManifest,
    proposed: &RecoveryManifest,
    reopened: &RecoveryInspection,
    capture: CodedCaptureId,
) -> Option<CodedCaptureReconciliationReceipt> {
    let outcome = reconcile_uncertain_commit(prior, proposed, reopened);
    let prior_capture = prior
        .snapshot
        .coded_captures
        .iter()
        .find(|snapshot| snapshot.capture == capture)?
        .clone();
    let proposed_capture = proposed
        .snapshot
        .coded_captures
        .iter()
        .find(|snapshot| snapshot.capture == capture)?
        .clone();
    if prior_capture.phase != CodedCapturePhase::CleanCommitPending
        || !matches!(
            proposed_capture.phase,
            CodedCapturePhase::CleanKnown | CodedCapturePhase::Refused
        )
    {
        return None;
    }
    let mut expected_proposed = prior_capture.clone();
    expected_proposed.phase = proposed_capture.phase;
    if expected_proposed != proposed_capture {
        return None;
    }
    let mut expected_unknown = prior_capture;
    expected_unknown.phase = CodedCapturePhase::CleanCommitUnknown;
    let resolved = match outcome {
        RecoveryReconciliation::Proposed => proposed_capture,
        RecoveryReconciliation::Prior => {
            let mut rejected = expected_unknown.clone();
            rejected.phase = CodedCapturePhase::Refused;
            rejected
        }
        RecoveryReconciliation::ReconciliationRequired => return None,
    };
    Some(CodedCaptureReconciliationReceipt::new(
        expected_unknown,
        resolved,
    ))
}

/// Resolve one exact uncertain later cut from independently reopened state.
pub fn reconcile_coded_later_cut_attempt(
    prior: &RecoveryManifest,
    proposed: &RecoveryManifest,
    reopened: &RecoveryInspection,
    capture: CodedCaptureId,
    operation: OperationSlotToken,
) -> Option<CodedCaptureReconciliationReceipt> {
    let outcome = reconcile_uncertain_commit(prior, proposed, reopened);
    let prior_capture = prior
        .snapshot
        .coded_captures
        .iter()
        .find(|snapshot| snapshot.capture == capture)?
        .clone();
    let proposed_capture = proposed
        .snapshot
        .coded_captures
        .iter()
        .find(|snapshot| snapshot.capture == capture)?
        .clone();
    if prior_capture.membership.get(&operation) != Some(&CodedCaptureMembership::Later) {
        return None;
    }
    let membership = *proposed_capture.membership.get(&operation)?;
    if !matches!(
        membership,
        CodedCaptureMembership::LaterDurableAfterClean
            | CodedCaptureMembership::LaterDurableStalesClean
    ) {
        return None;
    }
    let cut = *proposed_capture.resolved_later_cuts.get(&operation)?;
    let cut_frontier = cut.frontier();
    let mut expected_proposed = prior_capture.clone();
    expected_proposed.membership.insert(operation, membership);
    expected_proposed.resolved_later_cuts.insert(operation, cut);
    expected_proposed.retirement_frontier = Some(
        expected_proposed
            .retirement_frontier
            .map_or(cut_frontier, |frontier| frontier.max(cut_frontier)),
    );
    if expected_proposed != proposed_capture {
        return None;
    }
    let mut expected_unknown = prior_capture;
    expected_unknown
        .membership
        .insert(operation, CodedCaptureMembership::LaterUnknown);
    expected_unknown.pending_later_cuts.insert(operation, cut);
    let resolved = match outcome {
        RecoveryReconciliation::Proposed => proposed_capture,
        RecoveryReconciliation::Prior => {
            let mut rejected = expected_unknown.clone();
            rejected.pending_later_cuts.remove(&operation);
            rejected
                .membership
                .insert(operation, CodedCaptureMembership::LaterRejected);
            rejected
        }
        RecoveryReconciliation::ReconciliationRequired => return None,
    };
    Some(CodedCaptureReconciliationReceipt::new(
        expected_unknown,
        resolved,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JobId, RecoveryCursor, RecoveryGeneration, RecoveryMutation, RecoveryStateStore};
    use dwv_core::TopologyEpoch;

    #[test]
    fn uncertain_commit_reopens_to_exact_prior_proposed_or_neither() {
        let mut store = MemoryRecoveryStore::new(TopologyEpoch(1));
        let prior = store.export_manifest(RecoveryGeneration::ZERO).unwrap();
        let mut transaction = store.begin_protocol_txn(RecoveryGeneration::ZERO, TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordMaintenanceCheckpoint {
            job: JobId(1),
            cursor: RecoveryCursor(1),
        });
        let generation = store.commit_durable(transaction).unwrap();
        let proposed = store.export_manifest(generation).unwrap();

        assert_eq!(
            reconcile_uncertain_commit(
                &prior,
                &proposed,
                &RecoveryInspection::Supported(Box::new(prior.clone())),
            ),
            RecoveryReconciliation::Prior
        );
        assert_eq!(
            reconcile_uncertain_commit(
                &prior,
                &proposed,
                &RecoveryInspection::Supported(Box::new(proposed.clone())),
            ),
            RecoveryReconciliation::Proposed
        );

        let mut neither = proposed.clone();
        neither.snapshot.maintenance_checkpoints[0].cursor = RecoveryCursor(2);
        assert_eq!(
            reconcile_uncertain_commit(
                &prior,
                &proposed,
                &RecoveryInspection::Supported(Box::new(neither)),
            ),
            RecoveryReconciliation::ReconciliationRequired
        );
        assert_eq!(
            reconcile_uncertain_commit(
                &prior,
                &proposed,
                &RecoveryInspection::CorruptOrUnreadable {
                    storage_version: None,
                    semantic_version: None,
                },
            ),
            RecoveryReconciliation::ReconciliationRequired
        );
    }
}
