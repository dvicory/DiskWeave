//! Replaceable digest implementations behind the portable checksum seam.

use crate::{BLAKE3_256_PROFILE, ChecksumProfile, Digest};
use std::fmt;

/// A digest provider is an implementation detail of a semantic checksum profile.
/// Provider or crate identity is deliberately not persisted in checksum records.
pub trait DigestProvider {
    fn profile(&self) -> ChecksumProfile;
    fn digest(&self, bytes: &[u8]) -> Result<Digest, ProviderError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Blake3Provider;

impl DigestProvider for Blake3Provider {
    fn profile(&self) -> ChecksumProfile {
        BLAKE3_256_PROFILE
    }

    fn digest(&self, bytes: &[u8]) -> Result<Digest, ProviderError> {
        Ok(*blake3::hash(bytes).as_bytes())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderError {
    UnsupportedProfile(ChecksumProfile),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProfile(profile) => {
                write!(formatter, "no digest provider for profile {:?}", profile.id)
            }
        }
    }
}

impl std::error::Error for ProviderError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_has_fixed_profile_and_digest_width() {
        let provider = Blake3Provider;
        assert_eq!(provider.profile(), BLAKE3_256_PROFILE);
        assert_eq!(provider.digest(b"diskweave").unwrap().len(), 32);
        assert_eq!(provider.digest(b"diskweave"), provider.digest(b"diskweave"));
        assert_ne!(provider.digest(b"diskweave"), provider.digest(b"DiskWeave"));
    }

    #[test]
    fn deterministic_vectors_cover_empty_partial_full_and_zero_extents() {
        let provider = Blake3Provider;
        let full = vec![0xa5; BLAKE3_256_PROFILE.extent_size as usize];
        let zero = vec![0; BLAKE3_256_PROFILE.extent_size as usize];
        assert_eq!(
            provider.digest(b"").unwrap(),
            [
                0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40, 0x4d, 0xea, 0x36, 0xdc,
                0xc9, 0x49, 0x9b, 0xcb, 0x25, 0xc9, 0xad, 0xc1, 0x12, 0xb7, 0xcc, 0x9a, 0x93, 0xca,
                0xe4, 0x1f, 0x32, 0x62,
            ]
        );
        assert_eq!(
            provider.digest(b"partial extent").unwrap(),
            [
                0x3f, 0x9d, 0xa5, 0xde, 0xd8, 0x40, 0x00, 0x50, 0x14, 0xfc, 0xf6, 0xcf, 0xf5, 0x0a,
                0x36, 0xee, 0x00, 0xcb, 0x99, 0x47, 0x7a, 0x12, 0xbe, 0xb8, 0x06, 0xe5, 0xd6, 0xe6,
                0xf9, 0x54, 0x86, 0xcb,
            ]
        );
        assert_ne!(
            provider.digest(&full).unwrap(),
            provider.digest(&zero).unwrap()
        );
        assert_eq!(
            provider.digest(&zero).unwrap(),
            provider.digest(&zero).unwrap()
        );
    }
}
