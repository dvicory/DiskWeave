use crate::failure::{FailureClass, ServiceError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceState {
    Open,
    Serving,
    Recovering,
    Blocked(FailureClass),
    Closed,
}

impl ServiceState {
    pub const fn accepts_reads(self) -> bool {
        matches!(self, Self::Serving | Self::Recovering)
    }

    pub const fn accepts_writes(self) -> bool {
        matches!(self, Self::Serving)
    }

    pub fn require_reads(self) -> Result<(), ServiceError> {
        if self.accepts_reads() {
            Ok(())
        } else {
            Err(match self {
                Self::Blocked(class) => ServiceError::Blocked(class),
                _ => ServiceError::NotServing,
            })
        }
    }

    pub fn require_writes(self) -> Result<(), ServiceError> {
        if self.accepts_writes() {
            Ok(())
        } else {
            Err(match self {
                Self::Blocked(class) => ServiceError::Blocked(class),
                _ => ServiceError::NotServing,
            })
        }
    }
}
