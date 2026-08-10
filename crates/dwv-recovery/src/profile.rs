//! Stable checksum profile and set identities.

use std::fmt;

/// Stable identifier for the semantic digest profile, not a provider version.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct ChecksumProfileId(pub u32);

/// Monotonic identity for a parallel checksum set.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct ChecksumSetGeneration(pub u64);

impl ChecksumSetGeneration {
    pub const INITIAL: Self = Self(1);

    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct ChecksumProfile {
    pub id: ChecksumProfileId,
    pub extent_size: u64,
    pub digest_size: u8,
}

pub const BLAKE3_256_PROFILE: ChecksumProfile = ChecksumProfile {
    id: ChecksumProfileId(1),
    extent_size: 4 * 1024 * 1024,
    digest_size: 32,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileError {
    ZeroExtentSize,
    ZeroDigestSize,
    ExtentTooLarge(u64),
    DigestSizeMismatch { expected: u8, actual: u8 },
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExtentSize => f.write_str("checksum extent size must be non-zero"),
            Self::ZeroDigestSize => f.write_str("checksum digest size must be non-zero"),
            Self::ExtentTooLarge(size) => write!(f, "checksum extent size is too large: {size}"),
            Self::DigestSizeMismatch { expected, actual } => {
                write!(f, "checksum digest size {actual} does not match {expected}")
            }
        }
    }
}

impl std::error::Error for ProfileError {}

impl ChecksumProfile {
    pub const fn validate(self) -> Result<(), ProfileError> {
        if self.extent_size == 0 {
            return Err(ProfileError::ZeroExtentSize);
        }
        if self.digest_size == 0 {
            return Err(ProfileError::ZeroDigestSize);
        }
        // This bound protects callers from converting a semantic extent into
        // an unbounded allocation.  The provisional profile is well below it.
        if self.extent_size > 64 * 1024 * 1024 {
            return Err(ProfileError::ExtentTooLarge(self.extent_size));
        }
        Ok(())
    }

    pub fn is_supported(self) -> bool {
        self.id == BLAKE3_256_PROFILE.id
            && self.extent_size == BLAKE3_256_PROFILE.extent_size
            && self.digest_size == BLAKE3_256_PROFILE.digest_size
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum ChecksumSetState {
    Building,
    Active,
    Interrupted,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ChecksumSet {
    pub generation: ChecksumSetGeneration,
    pub profile: ChecksumProfile,
    pub state: ChecksumSetState,
}

impl ChecksumSet {
    pub fn new(
        generation: ChecksumSetGeneration,
        profile: ChecksumProfile,
    ) -> Result<Self, ProfileError> {
        profile.validate()?;
        Ok(Self {
            generation,
            profile,
            state: ChecksumSetState::Building,
        })
    }
}
