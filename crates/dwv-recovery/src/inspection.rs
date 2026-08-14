use crate::{MemoryRecoveryStore, RecoveryManifest};

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
