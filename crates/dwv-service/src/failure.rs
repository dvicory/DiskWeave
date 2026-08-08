use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureClass {
    Admission,
    Alias,
    Capability,
    Identity,
    InvalidRequest,
    Range,
    StoreRead,
    StoreWrite,
    Fence,
    Recovery,
    StaleTopology,
    StaleSlot,
    ReconciliationRequired,
}

#[derive(Debug)]
pub enum ServiceError {
    NotServing,
    Blocked(FailureClass),
    IncompleteRead {
        bytes: Vec<u8>,
        evidence: crate::evidence::CompletionEvidence,
    },
    Invalid {
        class: FailureClass,
        detail: String,
    },
    Io {
        class: FailureClass,
        detail: String,
    },
}

impl ServiceError {
    pub(crate) fn invalid(class: FailureClass, detail: impl Into<String>) -> Self {
        Self::Invalid {
            class,
            detail: detail.into(),
        }
    }

    pub(crate) fn io(class: FailureClass, detail: impl Into<String>) -> Self {
        Self::Io {
            class,
            detail: detail.into(),
        }
    }

    pub(crate) fn incomplete_read(
        bytes: Vec<u8>,
        evidence: crate::evidence::CompletionEvidence,
    ) -> Self {
        Self::IncompleteRead { bytes, evidence }
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotServing => write!(formatter, "portable service is not serving"),
            Self::Blocked(class) => write!(formatter, "portable service is blocked: {class:?}"),
            Self::IncompleteRead { evidence, .. } => write!(
                formatter,
                "read completed only {}/{} bytes: {:?}",
                evidence.completed, evidence.requested.length, evidence.disposition
            ),
            Self::Invalid { class, detail } => write!(formatter, "{class:?}: {detail}"),
            Self::Io { class, detail } => write!(formatter, "{class:?}: {detail}"),
        }
    }
}

impl std::error::Error for ServiceError {}
