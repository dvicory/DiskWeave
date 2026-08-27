use dwv_core::{BlockRequest, ByteRange};
use dwv_store::{
    CompletedRangeSet, CompletionDisposition, OperationSlotToken, PersistenceEvidence,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistenceClaim {
    VolatileOrUnknown,
    HostFenceOnly,
}

impl From<PersistenceEvidence> for PersistenceClaim {
    fn from(value: PersistenceEvidence) -> Self {
        if value.is_durable() {
            Self::HostFenceOnly
        } else {
            Self::VolatileOrUnknown
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionEvidence {
    pub requested: ByteRange,
    pub completed: CompletedRangeSet,
    pub disposition: CompletionDisposition,
    pub persistence: PersistenceClaim,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BasisConformance {
    Unresolved,
    Consumed,
    Discarded,
    Reconciled,
}

impl BasisConformance {
    pub(crate) const fn is_conformant(self) -> bool {
        !matches!(self, Self::Unresolved)
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OperationEffect {
    Unresolved,
    Terminal,
    AuthoritativelyReconciled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct OperationEffectObservation {
    pub(crate) operation: OperationSlotToken,
    pub(crate) effect: OperationEffect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecoveryReconciliation {
    Unresolved,
    Authoritative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseScope {
    Unknown,
    Outside,
    InScope,
}

impl ReleaseScope {
    pub(crate) const fn is_in_scope(self) -> bool {
        matches!(self, Self::InScope)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseRequirement {
    NotApplicable,
    TransactionUnresolved,
    TransactionSatisfied,
}

impl ReleaseRequirement {
    pub(crate) const fn is_satisfied(self) -> bool {
        matches!(self, Self::TransactionSatisfied)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseReconciliation {
    pub(crate) operation: OperationSlotToken,
    pub(crate) operation_effect: OperationEffect,
    pub(crate) requirement: ReleaseRequirement,
    pub(crate) recovery: RecoveryReconciliation,
    pub(crate) basis: BasisConformance,
}

/// Immutable service-level proof that the lifecycle owner found all seven
/// required observations for one exact operation generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseAuthorization {
    operation: OperationSlotToken,
}

impl ReleaseAuthorization {
    pub(crate) const fn issue_from_lifecycle_owner(operation: OperationSlotToken) -> Self {
        Self { operation }
    }

    pub const fn operation(&self) -> OperationSlotToken {
        self.operation
    }
}

/// dwv:req req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationEvidence {
    pub request: BlockRequest,
    pub completion: CompletionEvidence,
    pub trace: dwv_transaction_ref::Trace,
    pub release_authorization: Option<ReleaseAuthorization>,
}
