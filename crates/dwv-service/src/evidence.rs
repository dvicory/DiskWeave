use dwv_core::ByteRange;
use dwv_store::{CompletionDisposition, PersistenceEvidence};

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
    pub completed: u64,
    pub disposition: CompletionDisposition,
    pub persistence: PersistenceClaim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationEvidence {
    pub completion: CompletionEvidence,
    pub trace: dwv_transaction_ref::Trace,
}
