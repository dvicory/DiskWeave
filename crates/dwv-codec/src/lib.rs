//! Reference single-parity semantics for DiskWeave.
//!
//! The implementation is intentionally byte-oriented and dependency-free.
//! Runtime dispatch, vector width, device I/O, and persistent format choices
//! are outside this crate.

use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ByteRange {
    offset: u64,
    length: u64,
}

impl ByteRange {
    pub const fn new(offset: u64, length: u64) -> Result<Self, CodecError> {
        if offset.checked_add(length).is_none() {
            return Err(CodecError::RangeOverflow { offset, length });
        }
        Ok(Self { offset, length })
    }

    pub const fn end(self) -> u64 {
        self.offset + self.length
    }

    pub const fn offset(self) -> u64 {
        self.offset
    }

    pub const fn length(self) -> u64 {
        self.length
    }

    fn length_as_usize(self) -> Result<usize, CodecError> {
        usize::try_from(self.length).map_err(|_| CodecError::LengthTooLarge(self.length))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Geometry {
    data_lengths: Vec<u64>,
    parity_length: u64,
}

impl Geometry {
    pub fn new(data_lengths: Vec<u64>, parity_length: u64) -> Result<Self, CodecError> {
        if data_lengths.is_empty() {
            return Err(CodecError::NoDataSlots);
        }
        let largest = data_lengths.iter().copied().max().unwrap_or_default();
        if parity_length < largest {
            return Err(CodecError::ParityCapacityTooSmall {
                largest_data_length: largest,
                parity_length,
            });
        }
        usize::try_from(parity_length).map_err(|_| CodecError::LengthTooLarge(parity_length))?;
        for length in &data_lengths {
            usize::try_from(*length).map_err(|_| CodecError::LengthTooLarge(*length))?;
        }
        Ok(Self {
            data_lengths,
            parity_length,
        })
    }

    pub const fn data_count(&self) -> usize {
        self.data_lengths.len()
    }

    pub fn data_lengths(&self) -> &[u64] {
        &self.data_lengths
    }

    pub const fn parity_length(&self) -> u64 {
        self.parity_length
    }

    pub fn data_length(&self, slot: usize) -> Option<u64> {
        self.data_lengths.get(slot).copied()
    }

    fn data_len(&self, slot: usize) -> Result<usize, CodecError> {
        let length = self
            .data_lengths
            .get(slot)
            .copied()
            .ok_or(CodecError::SlotOutOfRange { slot })?;
        usize::try_from(length).map_err(|_| CodecError::LengthTooLarge(length))
    }

    pub fn range(&self, slot: usize, range: ByteRange) -> Result<(), CodecError> {
        let length = self
            .data_lengths
            .get(slot)
            .copied()
            .ok_or(CodecError::SlotOutOfRange { slot })?;
        let end = range.end();
        if end > length {
            return Err(CodecError::RangeOutsideSlot {
                slot,
                range,
                protected_length: length,
            });
        }
        if end > self.parity_length {
            return Err(CodecError::RangeOutsideParity {
                range,
                parity_length: self.parity_length,
            });
        }
        Ok(())
    }

    fn parity_range(&self, range: ByteRange) -> Result<(), CodecError> {
        if range.end() > self.parity_length {
            return Err(CodecError::RangeOutsideParity {
                range,
                parity_length: self.parity_length,
            });
        }
        Ok(())
    }

    fn parity_bounds(&self, range: ByteRange) -> Result<(usize, usize), CodecError> {
        self.parity_range(range)?;
        let offset = usize::try_from(range.offset())
            .map_err(|_| CodecError::LengthTooLarge(range.offset()))?;
        let length = range.length_as_usize()?;
        let end = offset
            .checked_add(length)
            .ok_or(CodecError::RangeOverflow {
                offset: range.offset(),
                length: range.length(),
            })?;
        Ok((offset, end))
    }

    fn survivor_length(&self, slot: usize, range: ByteRange) -> Result<usize, CodecError> {
        let slot_length = self
            .data_lengths
            .get(slot)
            .copied()
            .ok_or(CodecError::SlotOutOfRange { slot })?;
        self.parity_range(range)?;
        let start = range.offset().min(slot_length);
        let end = range.end().min(slot_length);
        usize::try_from(end - start).map_err(|_| CodecError::LengthTooLarge(end - start))
    }

    fn parity_len(&self) -> Result<usize, CodecError> {
        usize::try_from(self.parity_length)
            .map_err(|_| CodecError::LengthTooLarge(self.parity_length))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodecError {
    NoDataSlots,
    ParityCapacityTooSmall {
        largest_data_length: u64,
        parity_length: u64,
    },
    LengthTooLarge(u64),
    RangeOverflow {
        offset: u64,
        length: u64,
    },
    SlotOutOfRange {
        slot: usize,
    },
    RangeOutsideSlot {
        slot: usize,
        range: ByteRange,
        protected_length: u64,
    },
    RangeOutsideParity {
        range: ByteRange,
        parity_length: u64,
    },
    DataInputTooShort {
        slot: usize,
        required: u64,
        actual: usize,
    },
    DataInputTooLong {
        slot: usize,
        expected: u64,
        actual: usize,
    },
    ParityInputTooShort {
        required: u64,
        actual: usize,
    },
    BufferLengthMismatch {
        expected: usize,
        old_actual: usize,
        new_actual: usize,
    },
    SurvivorCountMismatch {
        expected: usize,
        actual: usize,
    },
    MissingSlotHasSurvivor {
        slot: usize,
    },
    MultipleErasures,
    SurvivorInputTooShort {
        slot: usize,
        required: usize,
        actual: usize,
    },
    SurvivorInputTooLong {
        slot: usize,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDataSlots => write!(formatter, "at least one data slot is required"),
            Self::ParityCapacityTooSmall {
                largest_data_length,
                parity_length,
            } => write!(
                formatter,
                "parity length {parity_length} is smaller than data length {largest_data_length}"
            ),
            Self::LengthTooLarge(length) => {
                write!(formatter, "length {length} cannot fit in memory")
            }
            Self::RangeOverflow { offset, length } => {
                write!(
                    formatter,
                    "range overflows: offset={offset} length={length}"
                )
            }
            Self::SlotOutOfRange { slot } => write!(formatter, "slot {slot} is out of range"),
            Self::RangeOutsideSlot {
                slot,
                range,
                protected_length,
            } => write!(
                formatter,
                "range {range:?} exceeds slot {slot} length {protected_length}"
            ),
            Self::RangeOutsideParity {
                range,
                parity_length,
            } => write!(
                formatter,
                "range {range:?} exceeds parity length {parity_length}"
            ),
            Self::DataInputTooShort {
                slot,
                required,
                actual,
            } => write!(
                formatter,
                "slot {slot} requires {required} bytes but has {actual}"
            ),
            Self::DataInputTooLong {
                slot,
                expected,
                actual,
            } => write!(
                formatter,
                "slot {slot} permits exactly {expected} protected bytes but has {actual}"
            ),
            Self::ParityInputTooShort { required, actual } => {
                write!(
                    formatter,
                    "parity requires {required} bytes but has {actual}"
                )
            }
            Self::BufferLengthMismatch {
                expected,
                old_actual,
                new_actual,
            } => write!(
                formatter,
                "update buffers must each have {expected} bytes; old={old_actual} new={new_actual}"
            ),
            Self::SurvivorCountMismatch { expected, actual } => {
                write!(formatter, "expected {expected} survivors but got {actual}")
            }
            Self::MissingSlotHasSurvivor { slot } => {
                write!(formatter, "missing slot {slot} has a survivor buffer")
            }
            Self::MultipleErasures => {
                write!(formatter, "single parity cannot solve multiple erasures")
            }
            Self::SurvivorInputTooShort {
                slot,
                required,
                actual,
            } => write!(
                formatter,
                "survivor slot {slot} requires {required} bytes but has {actual}"
            ),
            Self::SurvivorInputTooLong {
                slot,
                expected,
                actual,
            } => write!(
                formatter,
                "survivor slot {slot} permits exactly {expected} bytes for the requested range but has {actual}"
            ),
        }
    }
}

impl std::error::Error for CodecError {}

/// Semantic seam for future optimized or P/Q implementations.
pub trait ParityCodec {
    fn compute_parity(&self, geometry: &Geometry, data: &[&[u8]]) -> Result<Vec<u8>, CodecError>;

    fn update_parity(
        &self,
        geometry: &Geometry,
        parity: &mut [u8],
        slot: usize,
        range: ByteRange,
        old_data: &[u8],
        new_data: &[u8],
    ) -> Result<(), CodecError>;

    fn reconstruct(
        &self,
        geometry: &Geometry,
        parity: &[u8],
        missing_slot: usize,
        range: ByteRange,
        survivors: &[Option<&[u8]>],
    ) -> Result<Vec<u8>, CodecError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct XorReference;

impl XorReference {
    fn validate_data_inputs(geometry: &Geometry, data: &[&[u8]]) -> Result<usize, CodecError> {
        if data.len() != geometry.data_count() {
            return Err(CodecError::SurvivorCountMismatch {
                expected: geometry.data_count(),
                actual: data.len(),
            });
        }
        for (slot, input) in data.iter().enumerate() {
            let required = geometry.data_lengths[slot];
            let expected = geometry.data_len(slot)?;
            if input.len() < expected {
                return Err(CodecError::DataInputTooShort {
                    slot,
                    required,
                    actual: input.len(),
                });
            }
            if input.len() > expected {
                return Err(CodecError::DataInputTooLong {
                    slot,
                    expected: required,
                    actual: input.len(),
                });
            }
        }
        geometry.parity_len()
    }

    fn validate_parity_input(geometry: &Geometry, parity: &[u8]) -> Result<(), CodecError> {
        let required = geometry.parity_len()?;
        if parity.len() < required {
            return Err(CodecError::ParityInputTooShort {
                required: geometry.parity_length,
                actual: parity.len(),
            });
        }
        Ok(())
    }
}

impl ParityCodec for XorReference {
    fn compute_parity(&self, geometry: &Geometry, data: &[&[u8]]) -> Result<Vec<u8>, CodecError> {
        let parity_length = Self::validate_data_inputs(geometry, data)?;
        let mut parity = vec![0; parity_length];
        for (slot, input) in data.iter().enumerate() {
            let protected_length = geometry.data_len(slot)?;
            for index in 0..protected_length {
                parity[index] ^= input[index];
            }
        }
        Ok(parity)
    }

    fn update_parity(
        &self,
        geometry: &Geometry,
        parity: &mut [u8],
        slot: usize,
        range: ByteRange,
        old_data: &[u8],
        new_data: &[u8],
    ) -> Result<(), CodecError> {
        geometry.range(slot, range)?;
        let (offset, end) = geometry.parity_bounds(range)?;
        Self::validate_parity_input(geometry, parity)?;
        let expected = range.length_as_usize()?;
        if old_data.len() != expected || new_data.len() != expected {
            return Err(CodecError::BufferLengthMismatch {
                expected,
                old_actual: old_data.len(),
                new_actual: new_data.len(),
            });
        }
        for ((destination, old), new) in parity[offset..end]
            .iter_mut()
            .zip(old_data.iter())
            .zip(new_data.iter())
        {
            *destination ^= old ^ new;
        }
        Ok(())
    }

    fn reconstruct(
        &self,
        geometry: &Geometry,
        parity: &[u8],
        missing_slot: usize,
        range: ByteRange,
        survivors: &[Option<&[u8]>],
    ) -> Result<Vec<u8>, CodecError> {
        // Survivor buffers begin at range.offset. A shorter protected member
        // contributes only its intersection with the requested range; its
        // remaining bytes are the explicit logical-zero extension.
        geometry.range(missing_slot, range)?;
        Self::validate_parity_input(geometry, parity)?;
        if survivors.len() != geometry.data_count() {
            return Err(CodecError::SurvivorCountMismatch {
                expected: geometry.data_count(),
                actual: survivors.len(),
            });
        }

        if survivors[missing_slot].is_some() {
            return Err(CodecError::MissingSlotHasSurvivor { slot: missing_slot });
        }
        let (offset, end) = geometry.parity_bounds(range)?;
        let expected = range.length_as_usize()?;
        let mut missing = vec![0; expected];
        missing.copy_from_slice(&parity[offset..end]);

        for (slot, survivor) in survivors.iter().enumerate() {
            if slot == missing_slot {
                continue;
            }
            let Some(survivor) = survivor else {
                return Err(CodecError::MultipleErasures);
            };
            let required = geometry.survivor_length(slot, range)?;
            if survivor.len() < required {
                return Err(CodecError::SurvivorInputTooShort {
                    slot,
                    required,
                    actual: survivor.len(),
                });
            }
            if survivor.len() > required {
                return Err(CodecError::SurvivorInputTooLong {
                    slot,
                    expected: required,
                    actual: survivor.len(),
                });
            }
            for (destination, source) in missing
                .iter_mut()
                .take(required)
                .zip(survivor.iter().copied())
            {
                *destination ^= source;
            }
        }
        Ok(missing)
    }
}

pub fn compute_parity(geometry: &Geometry, data: &[&[u8]]) -> Result<Vec<u8>, CodecError> {
    XorReference.compute_parity(geometry, data)
}

pub fn update_parity(
    geometry: &Geometry,
    parity: &mut [u8],
    slot: usize,
    range: ByteRange,
    old_data: &[u8],
    new_data: &[u8],
) -> Result<(), CodecError> {
    XorReference.update_parity(geometry, parity, slot, range, old_data, new_data)
}

pub fn reconstruct(
    geometry: &Geometry,
    parity: &[u8],
    missing_slot: usize,
    range: ByteRange,
    survivors: &[Option<&[u8]>],
) -> Result<Vec<u8>, CodecError> {
    XorReference.reconstruct(geometry, parity, missing_slot, range, survivors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector_geometry() -> Geometry {
        Geometry::new(vec![4, 2, 3], 4).expect("golden geometry should be valid")
    }

    #[test]
    fn golden_vector_covers_zero_extension() {
        let geometry = vector_geometry();
        let data = [vec![1, 2, 3, 4], vec![4, 5], vec![10, 20, 30]];
        let references: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
        assert_eq!(
            compute_parity(&geometry, &references).expect("parity should compute"),
            vec![15, 19, 29, 4]
        );
    }

    #[test]
    fn golden_incremental_update_matches_recomputation() {
        let geometry = vector_geometry();
        let old_data = [vec![1, 2, 3, 4], vec![4, 5], vec![10, 20, 30]];
        let mut new_data = old_data.clone();
        new_data[0][1..3].copy_from_slice(&[8, 9]);
        let old_refs: Vec<&[u8]> = old_data.iter().map(Vec::as_slice).collect();
        let new_refs: Vec<&[u8]> = new_data.iter().map(Vec::as_slice).collect();
        let mut parity = compute_parity(&geometry, &old_refs).expect("old parity should compute");
        update_parity(
            &geometry,
            &mut parity,
            0,
            ByteRange::new(1, 2).unwrap(),
            &[2, 3],
            &[8, 9],
        )
        .expect("incremental update should succeed");
        assert_eq!(parity, compute_parity(&geometry, &new_refs).unwrap());
    }

    #[test]
    fn golden_reconstruction_matches_missing_member() {
        let geometry = vector_geometry();
        let data = [vec![1, 2, 3, 4], vec![4, 5], vec![10, 20, 30]];
        let references: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
        let parity = compute_parity(&geometry, &references).unwrap();
        let survivors = [Some(&data[0][..2]), None, Some(&data[2][..2])];
        assert_eq!(
            reconstruct(
                &geometry,
                &parity,
                1,
                ByteRange::new(0, 2).unwrap(),
                &survivors,
            )
            .unwrap(),
            data[1][..2]
        );
    }

    #[test]
    fn randomized_updates_match_full_recomputation() {
        let mut seed = 0xD15C_A11E_u64;
        for case in 0..128 {
            let lengths = vec![
                1 + (next(&mut seed) % 31),
                1 + (next(&mut seed) % 31),
                1 + (next(&mut seed) % 31),
                1 + (next(&mut seed) % 31),
            ];
            let parity_length = *lengths.iter().max().unwrap();
            let geometry = Geometry::new(lengths.clone(), parity_length).unwrap();
            let mut old_data: Vec<Vec<u8>> = lengths
                .iter()
                .map(|length| (0..*length).map(|_| next(&mut seed) as u8).collect())
                .collect();
            let slot = case % old_data.len();
            let max_offset = old_data[slot].len() - 1;
            let offset = (next(&mut seed) as usize) % old_data[slot].len();
            let length = 1 + ((next(&mut seed) as usize) % (max_offset + 1 - offset));
            let old = old_data[slot][offset..offset + length].to_vec();
            let new: Vec<u8> = (0..length).map(|_| next(&mut seed) as u8).collect();
            old_data[slot][offset..offset + length].copy_from_slice(&new);
            let old_refs: Vec<&[u8]> = old_data
                .iter()
                .enumerate()
                .map(|(index, data)| {
                    if index == slot {
                        let mut restored = data.clone();
                        restored[offset..offset + length].copy_from_slice(&old);
                        Box::leak(restored.into_boxed_slice()) as &[u8]
                    } else {
                        data.as_slice()
                    }
                })
                .collect();
            let new_refs: Vec<&[u8]> = old_data.iter().map(Vec::as_slice).collect();
            let mut updated = compute_parity(&geometry, &old_refs).unwrap();
            update_parity(
                &geometry,
                &mut updated,
                slot,
                ByteRange::new(offset as u64, length as u64).unwrap(),
                &old,
                &new,
            )
            .unwrap();
            assert_eq!(updated, compute_parity(&geometry, &new_refs).unwrap());
        }
    }

    #[test]
    fn randomized_single_erasure_reconstructs() {
        let mut seed = 0xC0DEC0DEu64;
        for _ in 0..128 {
            let lengths = vec![
                1 + (next(&mut seed) % 31),
                1 + (next(&mut seed) % 31),
                1 + (next(&mut seed) % 31),
            ];
            let geometry = Geometry::new(lengths.clone(), *lengths.iter().max().unwrap()).unwrap();
            let data: Vec<Vec<u8>> = lengths
                .iter()
                .map(|length| (0..*length).map(|_| next(&mut seed) as u8).collect())
                .collect();
            let references: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
            let parity = compute_parity(&geometry, &references).unwrap();
            for missing in 0..data.len() {
                let missing_length = data[missing].len();
                let offset = (next(&mut seed) as usize) % (missing_length + 1);
                let range_length = missing_length - offset;
                let range_end = offset + range_length;
                let survivors: Vec<Option<&[u8]>> = data
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        (index != missing)
                            .then_some(&value[offset.min(value.len())..range_end.min(value.len())])
                    })
                    .collect();
                assert_eq!(
                    reconstruct(
                        &geometry,
                        &parity,
                        missing,
                        ByteRange::new(offset as u64, range_length as u64).unwrap(),
                        &survivors,
                    )
                    .unwrap(),
                    data[missing][offset..range_end]
                );
            }
        }
    }

    #[test]
    fn zero_length_members_and_tails_are_explicit() {
        let geometry = Geometry::new(vec![0, 2, 4], 4).unwrap();
        assert_eq!(geometry.data_lengths(), &[0, 2, 4]);
        assert_eq!(geometry.parity_length(), 4);
        let data = [vec![], vec![1, 2], vec![3, 4, 5, 6]];
        let references: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
        assert_eq!(
            compute_parity(&geometry, &references).unwrap(),
            vec![2, 6, 5, 6]
        );
    }

    #[test]
    fn public_range_accessors_expose_only_checked_values() {
        let range = ByteRange::new(7, 5).unwrap();
        assert_eq!(range.offset(), 7);
        assert_eq!(range.length(), 5);
        assert_eq!(range.end(), 12);

        assert_eq!(ByteRange::new(u64::MAX, 0).unwrap().end(), u64::MAX);
        assert!(matches!(
            ByteRange::new(u64::MAX, 1),
            Err(CodecError::RangeOverflow { .. })
        ));
    }

    #[test]
    fn invalid_geometry_and_ranges_are_rejected() {
        assert_eq!(Geometry::new(Vec::new(), 1), Err(CodecError::NoDataSlots));
        assert!(matches!(
            Geometry::new(vec![5], 4),
            Err(CodecError::ParityCapacityTooSmall { .. })
        ));
        assert!(matches!(
            ByteRange::new(u64::MAX, 1),
            Err(CodecError::RangeOverflow { .. })
        ));
        let geometry = Geometry::new(vec![4], 4).unwrap();
        assert!(matches!(
            geometry.range(1, ByteRange::new(0, 1).unwrap()),
            Err(CodecError::SlotOutOfRange { .. })
        ));
        assert!(matches!(
            geometry.range(0, ByteRange::new(3, 2).unwrap()),
            Err(CodecError::RangeOutsideSlot { .. })
        ));
    }

    #[test]
    fn invalid_inputs_never_silently_truncate() {
        let geometry = vector_geometry();
        let data = [vec![1, 2, 3, 4], vec![4], vec![10, 20, 30]];
        let references: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
        assert!(matches!(
            compute_parity(&geometry, &references),
            Err(CodecError::DataInputTooShort { slot: 1, .. })
        ));

        let overlong = [vec![1, 2, 3, 4], vec![4, 5, 99], vec![10, 20, 30]];
        let overlong_references: Vec<&[u8]> = overlong.iter().map(Vec::as_slice).collect();
        assert!(matches!(
            compute_parity(&geometry, &overlong_references),
            Err(CodecError::DataInputTooLong { slot: 1, .. })
        ));

        let parity = vec![0; 4];
        let survivors = [None, None, Some(&data[2][..2])];
        assert_eq!(
            reconstruct(
                &geometry,
                &parity,
                0,
                ByteRange::new(0, 2).unwrap(),
                &survivors,
            ),
            Err(CodecError::MultipleErasures)
        );

        let survivors = [None, Some(&[4, 5, 99][..]), Some(&[10, 20, 30][..])];
        assert!(matches!(
            reconstruct(
                &geometry,
                &parity,
                0,
                ByteRange::new(0, 4).unwrap(),
                &survivors,
            ),
            Err(CodecError::SurvivorInputTooLong { slot: 1, .. })
        ));
    }

    #[test]
    fn bounded_geometry_and_parity_exhaustive() {
        for lengths in [[0_u64, 1, 2], [1, 2, 3], [2, 0, 3]] {
            let parity_length = lengths.iter().copied().max().unwrap();
            let geometry = Geometry::new(lengths.to_vec(), parity_length).unwrap();
            let total_length = lengths.iter().sum::<u64>() as usize;
            let vector_count = 3_usize.pow(total_length as u32);

            for old_ordinal in 0..vector_count {
                let old_data = bounded_vectors(&lengths, old_ordinal);
                let old_references: Vec<&[u8]> = old_data.iter().map(Vec::as_slice).collect();
                let expected_old = explicit_parity(&old_data, parity_length);
                assert_eq!(
                    compute_parity(&geometry, &old_references).unwrap(),
                    expected_old
                );

                for slot in 0..lengths.len() {
                    for offset in 0..=lengths[slot] {
                        for range_length in 0..=(lengths[slot] - offset) {
                            let range = ByteRange::new(offset, range_length).unwrap();
                            let old =
                                &old_data[slot][offset as usize..(offset + range_length) as usize];
                            let new_count = 3_usize.pow(range_length as u32);
                            for new_ordinal in 0..new_count {
                                let new = bounded_bytes(range_length as usize, new_ordinal);
                                let mut new_data = old_data.clone();
                                new_data[slot][offset as usize..(offset + range_length) as usize]
                                    .copy_from_slice(&new);
                                let mut updated = expected_old.clone();
                                update_parity(&geometry, &mut updated, slot, range, old, &new)
                                    .unwrap();
                                assert_eq!(updated, explicit_parity(&new_data, parity_length));
                            }
                        }
                    }
                }

                for missing_slot in 0..lengths.len() {
                    for offset in 0..=lengths[missing_slot] {
                        for range_length in 0..=(lengths[missing_slot] - offset) {
                            let range = ByteRange::new(offset, range_length).unwrap();
                            let survivors: Vec<Option<&[u8]>> = old_data
                                .iter()
                                .enumerate()
                                .map(|(slot, data)| {
                                    if slot == missing_slot {
                                        None
                                    } else {
                                        let start = offset.min(data.len() as u64) as usize;
                                        let end =
                                            (offset + range_length).min(data.len() as u64) as usize;
                                        Some(&data[start..end])
                                    }
                                })
                                .collect();
                            assert_eq!(
                                reconstruct(
                                    &geometry,
                                    &expected_old,
                                    missing_slot,
                                    range,
                                    &survivors,
                                )
                                .unwrap(),
                                old_data[missing_slot]
                                    [offset as usize..(offset + range_length) as usize]
                            );
                        }
                    }
                }
            }
        }
    }

    fn bounded_vectors(lengths: &[u64], mut ordinal: usize) -> Vec<Vec<u8>> {
        const VALUES: [u8; 3] = [0, 1, u8::MAX];
        lengths
            .iter()
            .map(|length| {
                (0..*length)
                    .map(|_| {
                        let value = VALUES[ordinal % VALUES.len()];
                        ordinal /= VALUES.len();
                        value
                    })
                    .collect()
            })
            .collect()
    }

    fn bounded_bytes(length: usize, mut ordinal: usize) -> Vec<u8> {
        const VALUES: [u8; 3] = [0, 1, u8::MAX];
        (0..length)
            .map(|_| {
                let value = VALUES[ordinal % VALUES.len()];
                ordinal /= VALUES.len();
                value
            })
            .collect()
    }

    fn explicit_parity(data: &[Vec<u8>], parity_length: u64) -> Vec<u8> {
        let mut parity = vec![0; parity_length as usize];
        for member in data {
            for (index, byte) in member.iter().copied().enumerate() {
                parity[index] ^= byte;
            }
        }
        parity
    }

    #[test]
    fn future_codec_profiles_use_a_separate_seam() {
        fn accepts_codec<T: ParityCodec>(_codec: T) {}
        accepts_codec(XorReference);
    }

    fn next(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *seed
    }
}

#[cfg(kani)]
mod kani_verification {
    use super::*;

    fn fixed_geometry() -> Geometry {
        Geometry::new(vec![2, 2, 2], 2).unwrap()
    }

    #[kani::proof]
    fn full_parity_matches_explicit_xor() {
        let data = [
            [kani::any::<u8>(), kani::any::<u8>()],
            [kani::any::<u8>(), kani::any::<u8>()],
            [kani::any::<u8>(), kani::any::<u8>()],
        ];
        let references: [&[u8]; 3] = [&data[0], &data[1], &data[2]];
        let parity = compute_parity(&fixed_geometry(), &references).unwrap();

        assert_eq!(parity[0], data[0][0] ^ data[1][0] ^ data[2][0]);
        assert_eq!(parity[1], data[0][1] ^ data[1][1] ^ data[2][1]);
    }

    #[kani::proof]
    fn incremental_update_matches_full_recomputation() {
        let old = [
            [kani::any::<u8>(), kani::any::<u8>()],
            [kani::any::<u8>(), kani::any::<u8>()],
            [kani::any::<u8>(), kani::any::<u8>()],
        ];
        let new = [kani::any::<u8>(), kani::any::<u8>()];
        let geometry = fixed_geometry();
        let old_references: [&[u8]; 3] = [&old[0], &old[1], &old[2]];
        let new_references: [&[u8]; 3] = [&old[0], &new, &old[2]];
        let mut updated = compute_parity(&geometry, &old_references).unwrap();

        update_parity(
            &geometry,
            &mut updated,
            1,
            ByteRange::new(0, 2).unwrap(),
            &old[1],
            &new,
        )
        .unwrap();

        assert_eq!(updated, compute_parity(&geometry, &new_references).unwrap());
    }

    #[kani::proof]
    fn fixed_single_erasure_reconstructs_exactly() {
        let data = [
            [kani::any::<u8>(), kani::any::<u8>()],
            [kani::any::<u8>(), kani::any::<u8>()],
            [kani::any::<u8>(), kani::any::<u8>()],
        ];
        let geometry = fixed_geometry();
        let references: [&[u8]; 3] = [&data[0], &data[1], &data[2]];
        let parity = compute_parity(&geometry, &references).unwrap();
        let survivors = [Some(&data[0][..]), None, Some(&data[2][..])];
        let reconstructed = reconstruct(
            &geometry,
            &parity,
            1,
            ByteRange::new(0, 2).unwrap(),
            &survivors,
        )
        .unwrap();

        assert_eq!(reconstructed.as_slice(), &data[1]);
    }
}
