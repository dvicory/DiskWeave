use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationError {
    InvalidConfig(String),
    InvalidEvidence(String),
    Codec(String),
    Store(String),
    Repair(String),
}

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => {
                write!(formatter, "invalid verification config: {message}")
            }
            Self::InvalidEvidence(message) => {
                write!(formatter, "invalid verification evidence: {message}")
            }
            Self::Codec(message) => write!(formatter, "verification codec failure: {message}"),
            Self::Store(message) => write!(formatter, "verification store failure: {message}"),
            Self::Repair(message) => write!(formatter, "repair verification failure: {message}"),
        }
    }
}

impl std::error::Error for VerificationError {}
