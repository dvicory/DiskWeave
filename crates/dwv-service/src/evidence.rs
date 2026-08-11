use dwv_core::{BlockRequest, ByteRange};
use dwv_store::{CompletedRangeSet, CompletionDisposition, PersistenceEvidence};

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

/// dwv:req req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationEvidence {
    pub request: BlockRequest,
    pub completion: CompletionEvidence,
    pub trace: dwv_transaction_ref::Trace,
}
