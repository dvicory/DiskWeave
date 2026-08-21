use dwv_core::BlockRequest;
use dwv_store::StoreCompletion;
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
        request: Box<BlockRequest>,
        bytes: Vec<u8>,
        evidence: crate::evidence::CompletionEvidence,
    },
    Invalid {
        request: Option<Box<BlockRequest>>,
        class: FailureClass,
        detail: String,
    },
    Io {
        request: Option<Box<BlockRequest>>,
        class: FailureClass,
        detail: String,
        completion: Option<Box<StoreCompletion>>,
    },
    Terminalization {
        request: Box<BlockRequest>,
        primary: Option<Box<Self>>,
        cleanup: Box<Self>,
    },
}

impl ServiceError {
    pub(crate) fn invalid(class: FailureClass, detail: impl Into<String>) -> Self {
        Self::Invalid {
            request: None,
            class,
            detail: detail.into(),
        }
    }

    pub(crate) fn io(class: FailureClass, detail: impl Into<String>) -> Self {
        Self::Io {
            request: None,
            class,
            detail: detail.into(),
            completion: None,
        }
    }

    pub(crate) fn store_completion(class: FailureClass, completion: StoreCompletion) -> Self {
        Self::Io {
            request: None,
            class,
            detail: format!("store completion: {:?}", completion.disposition),
            completion: Some(Box::new(completion)),
        }
    }

    pub(crate) fn rejected_completion(
        class: FailureClass,
        completion: StoreCompletion,
        detail: impl Into<String>,
    ) -> Self {
        Self::Io {
            request: None,
            class,
            detail: detail.into(),
            completion: Some(Box::new(completion)),
        }
    }

    pub(crate) fn incomplete_read(
        request: BlockRequest,
        bytes: Vec<u8>,
        evidence: crate::evidence::CompletionEvidence,
    ) -> Self {
        Self::IncompleteRead {
            request: Box::new(request),
            bytes,
            evidence,
        }
    }

    pub(crate) fn terminalization(
        request: BlockRequest,
        primary: Option<Self>,
        cleanup: Self,
    ) -> Self {
        Self::Terminalization {
            request: Box::new(request),
            primary: primary.map(Box::new),
            cleanup: Box::new(cleanup),
        }
    }
}

impl ServiceError {
    pub(crate) fn with_request(self, request: BlockRequest) -> Self {
        match self {
            Self::IncompleteRead {
                bytes, evidence, ..
            } => Self::IncompleteRead {
                request: Box::new(request),
                bytes,
                evidence,
            },
            Self::Invalid { class, detail, .. } => Self::Invalid {
                request: Some(Box::new(request)),
                class,
                detail,
            },
            Self::Io {
                class,
                detail,
                completion,
                ..
            } => Self::Io {
                request: Some(Box::new(request)),
                class,
                detail,
                completion,
            },
            Self::Terminalization {
                primary, cleanup, ..
            } => Self::Terminalization {
                request: Box::new(request),
                primary: primary.map(|error| Box::new((*error).with_request(request))),
                cleanup: Box::new((*cleanup).with_request(request)),
            },
            error => error,
        }
    }

    pub fn request(&self) -> Option<BlockRequest> {
        match self {
            Self::IncompleteRead { request, .. } => Some(**request),
            Self::Invalid { request, .. } | Self::Io { request, .. } => request.as_deref().copied(),
            Self::Terminalization { request, .. } => Some(**request),
            Self::NotServing | Self::Blocked(_) => None,
        }
    }

    pub fn primary_error(&self) -> Option<&Self> {
        match self {
            Self::Terminalization { primary, .. } => primary.as_deref(),
            _ => None,
        }
    }

    pub fn cleanup_error(&self) -> Option<&Self> {
        match self {
            Self::Terminalization { cleanup, .. } => Some(cleanup),
            _ => None,
        }
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotServing => write!(formatter, "portable service is not serving"),
            Self::Blocked(class) => write!(formatter, "portable service is blocked: {class:?}"),
            Self::IncompleteRead { evidence, .. } => write!(
                formatter,
                "read completed ranges {:?} of {:?}: {:?}",
                evidence.completed.as_slice(),
                evidence.requested,
                evidence.disposition
            ),
            Self::Invalid { class, detail, .. } => write!(formatter, "{class:?}: {detail}"),
            Self::Io {
                class,
                detail,
                completion: Some(completion),
                ..
            } => write!(
                formatter,
                "{class:?}: {detail}; completed ranges {:?}, persistence {:?}",
                completion.completed.as_slice(),
                completion.persistence
            ),
            Self::Io { class, detail, .. } => write!(formatter, "{class:?}: {detail}"),
            Self::Terminalization {
                primary: Some(primary),
                cleanup,
                ..
            } => write!(
                formatter,
                "operation failed: {primary}; terminalization failed: {cleanup}"
            ),
            Self::Terminalization {
                primary: None,
                cleanup,
                ..
            } => write!(
                formatter,
                "operation completed but terminalization failed: {cleanup}"
            ),
        }
    }
}

impl std::error::Error for ServiceError {}
