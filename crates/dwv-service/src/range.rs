use crate::failure::{FailureClass, ServiceError};
use dwv_core::{ByteRange, ProtectedGeometry};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangePlan {
    pub ranges: Vec<ByteRange>,
}

pub fn split_range(
    range: ByteRange,
    geometry: ProtectedGeometry,
    maximum_transfer: u64,
) -> Result<RangePlan, ServiceError> {
    if range.is_empty() || range.end() > geometry.protected_length() {
        return Err(ServiceError::invalid(
            FailureClass::Range,
            "range is empty or outside protected geometry",
        ));
    }
    let block = u64::from(geometry.logical_block_size());
    if !range.offset.is_multiple_of(block) || !range.length.is_multiple_of(block) {
        return Err(ServiceError::invalid(
            FailureClass::Range,
            "range is not logically block aligned",
        ));
    }
    if maximum_transfer == 0 || !maximum_transfer.is_multiple_of(block) {
        return Err(ServiceError::invalid(
            FailureClass::Range,
            "maximum transfer is not a non-zero block multiple",
        ));
    }
    let mut ranges = Vec::new();
    let mut offset = range.offset;
    let mut remaining = range.length;
    while remaining != 0 {
        let length = remaining.min(maximum_transfer);
        ranges.push(
            ByteRange::new(offset, length)
                .map_err(|error| ServiceError::io(FailureClass::Range, error.to_string()))?,
        );
        offset += length;
        remaining -= length;
    }
    Ok(RangePlan { ranges })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_only_at_transfer_and_preserves_exact_coverage() {
        let geometry = ProtectedGeometry::new(4096, 512).unwrap();
        let range = ByteRange::new(512, 2048).unwrap();
        let plan = split_range(range, geometry, 1024).unwrap();
        assert_eq!(
            plan.ranges,
            vec![
                ByteRange::new(512, 1024).unwrap(),
                ByteRange::new(1536, 1024).unwrap()
            ]
        );
        assert_eq!(plan.ranges.first().unwrap().offset, range.offset);
        assert_eq!(plan.ranges.last().unwrap().end(), range.end());
        assert_eq!(
            plan.ranges.iter().map(|part| part.length).sum::<u64>(),
            range.length
        );
    }

    #[test]
    fn rejects_unaligned_and_out_of_geometry_ranges() {
        let geometry = ProtectedGeometry::new(4096, 512).unwrap();
        assert!(split_range(ByteRange::new(1, 512).unwrap(), geometry, 1024).is_err());
        assert!(split_range(ByteRange::new(3584, 1024).unwrap(), geometry, 1024).is_err());
    }

    #[test]
    fn bounded_split_ranges_preserve_aligned_coverage() {
        for block_size in [1_u32, 2, 4, 8] {
            for geometry_blocks in 1_u64..=8 {
                let geometry =
                    ProtectedGeometry::new(geometry_blocks * u64::from(block_size), block_size)
                        .unwrap();
                for transfer_blocks in 1_u64..=geometry_blocks {
                    let maximum_transfer = transfer_blocks * u64::from(block_size);
                    for start_block in 0..geometry_blocks {
                        for length_blocks in 1..=(geometry_blocks - start_block) {
                            let range = ByteRange::new(
                                start_block * u64::from(block_size),
                                length_blocks * u64::from(block_size),
                            )
                            .unwrap();
                            let plan = split_range(range, geometry, maximum_transfer).unwrap();
                            assert!(!plan.ranges.is_empty());

                            let mut cursor = range.offset;
                            for part in &plan.ranges {
                                assert_eq!(part.offset, cursor);
                                assert!(part.length <= maximum_transfer);
                                assert_eq!(part.offset % u64::from(block_size), 0);
                                assert_eq!(part.length % u64::from(block_size), 0);
                                cursor = part.end();
                            }
                            assert_eq!(cursor, range.end());
                            assert_eq!(
                                plan.ranges.iter().map(|part| part.length).sum::<u64>(),
                                range.length
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(kani)]
mod kani_verification {
    use super::*;

    // Fixed-array arithmetic model for split_range. Production Vec allocation
    // and error formatting remain outside this proof and are covered by the
    // ordinary/property test layers.
    fn split_range_math(range: ByteRange, maximum_transfer: u64) -> ([ByteRange; 8], usize) {
        let empty = ByteRange::new(0, 0).unwrap();
        let mut parts = [empty; 8];
        let mut offset = range.offset;
        let mut remaining = range.length;
        let mut count = 0;
        while remaining != 0 {
            assert!(count < parts.len());
            let length = remaining.min(maximum_transfer);
            parts[count] = ByteRange::new(offset, length).unwrap();
            offset += length;
            remaining -= length;
            count += 1;
        }
        (parts, count)
    }

    #[kani::proof]
    #[kani::unwind(9)]
    fn split_range_math_preserves_aligned_coverage() {
        const BLOCK: u64 = 512;
        const BLOCKS: u64 = 8;

        let start_block: u8 = kani::any();
        let length_blocks: u8 = kani::any();
        let transfer_blocks: u8 = kani::any();
        kani::assume(start_block <= BLOCKS as u8 - 1);
        kani::assume(length_blocks >= 1);
        kani::assume(u16::from(start_block) + u16::from(length_blocks) <= BLOCKS as u16);
        kani::assume(transfer_blocks >= 1 && transfer_blocks <= BLOCKS as u8);
        kani::cover!(start_block == 0 && length_blocks == 1 && transfer_blocks == 1);
        kani::cover!(start_block == 7 && length_blocks == 1 && transfer_blocks == 8);

        let range = ByteRange::new(
            u64::from(start_block) * BLOCK,
            u64::from(length_blocks) * BLOCK,
        )
        .unwrap();
        let maximum_transfer = u64::from(transfer_blocks) * BLOCK;
        let (parts, count) = split_range_math(range, maximum_transfer);
        let mut cursor = range.offset;
        let mut total = 0_u64;

        for (index, part) in parts.iter().enumerate() {
            if index < count {
                assert_eq!(part.offset, cursor);
                assert!(part.length > 0);
                assert!(part.length <= maximum_transfer);
                assert_eq!(part.length % BLOCK, 0);
                cursor = part.end();
                total += part.length;
            } else {
                assert_eq!(*part, ByteRange::new(0, 0).unwrap());
            }
        }
        assert_eq!(cursor, range.end());
        assert_eq!(total, range.length);
    }
}
