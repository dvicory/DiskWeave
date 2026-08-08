//! Bounded checksum extent geometry and target mapping.

use crate::{IntegrityExtentId, ProfileError};
use dwv_core::{ByteRange, CodingPosition, SlotId};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChecksumTarget {
    Data { slot: SlotId },
    Parity { position: CodingPosition },
    Q { position: CodingPosition },
}

impl ChecksumTarget {
    pub const fn data(slot: SlotId) -> Self {
        Self::Data { slot }
    }

    pub const fn parity(position: CodingPosition) -> Self {
        Self::Parity { position }
    }

    pub const fn q(position: CodingPosition) -> Self {
        Self::Q { position }
    }

    pub const fn is_q(self) -> bool {
        matches!(self, Self::Q { .. })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ChecksumExtent {
    pub id: IntegrityExtentId,
    pub target: ChecksumTarget,
    pub range: ByteRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtentError {
    InvalidProfile(ProfileError),
    EmptyRange,
    Unaligned {
        offset: u64,
        length: u64,
        extent_size: u64,
    },
    Overflow,
    TooManyExtents,
}

impl fmt::Display for ExtentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(error) => error.fmt(f),
            Self::EmptyRange => f.write_str("checksum extent range is empty"),
            Self::Unaligned {
                offset,
                length,
                extent_size,
            } => write!(
                f,
                "checksum range offset={offset} length={length} is not aligned to {extent_size}"
            ),
            Self::Overflow => f.write_str("checksum extent id arithmetic overflowed"),
            Self::TooManyExtents => f.write_str("checksum extent count exceeds the bounded limit"),
        }
    }
}

impl std::error::Error for ExtentError {}

impl ChecksumExtent {
    pub fn new(
        id: IntegrityExtentId,
        target: ChecksumTarget,
        range: ByteRange,
        extent_size: u64,
    ) -> Result<Self, ExtentError> {
        if range.is_empty() {
            return Err(ExtentError::EmptyRange);
        }
        if extent_size == 0 || range.offset % extent_size != 0 || range.length > extent_size {
            return Err(ExtentError::Unaligned {
                offset: range.offset,
                length: range.length,
                extent_size,
            });
        }
        Ok(Self { id, target, range })
    }

    pub fn partition(
        target: ChecksumTarget,
        length: u64,
        extent_size: u64,
        first_id: IntegrityExtentId,
    ) -> Result<Vec<Self>, ExtentError> {
        if extent_size == 0 {
            return Err(ExtentError::Unaligned {
                offset: 0,
                length,
                extent_size,
            });
        }
        if length == 0 {
            return Err(ExtentError::EmptyRange);
        }
        let count = length.div_ceil(extent_size);
        if count > 1_048_576 {
            return Err(ExtentError::TooManyExtents);
        }
        let mut extents = Vec::with_capacity(count as usize);
        for index in 0..count {
            let offset = index
                .checked_mul(extent_size)
                .ok_or(ExtentError::Overflow)?;
            let remaining = length - offset;
            let range = ByteRange::new(offset, remaining.min(extent_size))
                .map_err(|_| ExtentError::Overflow)?;
            let id = IntegrityExtentId(first_id.0.checked_add(index).ok_or(ExtentError::Overflow)?);
            extents.push(Self::new(id, target, range, extent_size)?);
        }
        Ok(extents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_core::{CodingPosition, SlotId};

    #[test]
    fn partition_splits_crossing_range_without_merging_targets() {
        let target = ChecksumTarget::data(SlotId([7; 16]));
        let extents = ChecksumExtent::partition(target, 10, 4, IntegrityExtentId(20)).unwrap();

        assert_eq!(extents.len(), 3);
        assert_eq!(extents[0].range, ByteRange::new(0, 4).unwrap());
        assert_eq!(extents[1].range, ByteRange::new(4, 4).unwrap());
        assert_eq!(extents[2].range, ByteRange::new(8, 2).unwrap());
        assert_eq!(extents[0].id, IntegrityExtentId(20));
        assert_eq!(extents[2].id, IntegrityExtentId(22));
        assert!(extents.iter().all(|extent| extent.target == target));
    }

    #[test]
    fn data_parity_and_optional_q_are_distinct_targets() {
        let data = ChecksumTarget::data(SlotId([1; 16]));
        let parity = ChecksumTarget::parity(CodingPosition(1));
        let q = ChecksumTarget::q(CodingPosition(1));
        assert_ne!(data, parity);
        assert_ne!(parity, q);
        assert!(q.is_q());
        assert!(!data.is_q());
    }
}
