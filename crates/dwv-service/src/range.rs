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
