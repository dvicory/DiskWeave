use dwv_core::ByteRange;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationStoreError {
    message: String,
}

impl VerificationStoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for VerificationStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for VerificationStoreError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VerificationIdentity(pub [u8; 16]);

/// Minimal adapter seam for verification and separate-target repair.
pub trait VerificationStore {
    fn identity(&self) -> VerificationIdentity;

    fn read_exact(&mut self, range: ByteRange) -> Result<Vec<u8>, VerificationStoreError>;

    fn write_exact(&mut self, range: ByteRange, bytes: &[u8])
    -> Result<(), VerificationStoreError>;
}
