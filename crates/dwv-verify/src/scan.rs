use crate::{
    error::VerificationError,
    evidence::ChecksumEvidence,
    report::{
        EvidenceStatus, MemberRef, MismatchClass, RegionDisposition, RegionReport, ScanMode,
        VerificationReport,
    },
    store::VerificationStore,
};
use dwv_codec::{Geometry, ParityCodec, XorReference};
use dwv_core::ByteRange;
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_MAX_REGIONS: usize = 1_048_576;
static NEXT_VERIFICATION_RUN_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct ScanConfig {
    pub geometry: Geometry,
    pub region_size: u64,
    pub max_regions: usize,
}

impl ScanConfig {
    pub fn new(geometry: Geometry, region_size: u64) -> Result<Self, VerificationError> {
        let config = Self {
            geometry,
            region_size,
            max_regions: DEFAULT_MAX_REGIONS,
        };
        config.validate()?;
        Ok(config)
    }

    pub const fn with_max_regions(mut self, max_regions: usize) -> Self {
        self.max_regions = max_regions;
        self
    }

    pub fn validate(&self) -> Result<(), VerificationError> {
        if self.region_size == 0 {
            return Err(VerificationError::InvalidConfig(
                "region size must be non-zero".to_owned(),
            ));
        }
        if self.geometry.parity_length() == 0 {
            return Err(VerificationError::InvalidConfig(
                "protected length must be non-zero".to_owned(),
            ));
        }
        if self.max_regions == 0 {
            return Err(VerificationError::InvalidConfig(
                "maximum region count must be non-zero".to_owned(),
            ));
        }
        let count = self.region_count()?;
        if count > self.max_regions {
            return Err(VerificationError::InvalidConfig(format!(
                "scan needs {count} regions but the bound is {}",
                self.max_regions
            )));
        }
        Ok(())
    }

    pub fn region_count(&self) -> Result<usize, VerificationError> {
        let count = self.geometry.parity_length().div_ceil(self.region_size);
        usize::try_from(count).map_err(|_| {
            VerificationError::InvalidConfig("region count does not fit usize".to_owned())
        })
    }

    pub fn region_ranges(&self) -> Result<Vec<ByteRange>, VerificationError> {
        self.validate()?;
        let mut ranges = Vec::with_capacity(self.region_count()?);
        let mut offset = 0_u64;
        while offset < self.geometry.parity_length() {
            let length = self.region_size.min(self.geometry.parity_length() - offset);
            ranges.push(
                ByteRange::new(offset, length)
                    .map_err(|error| VerificationError::InvalidConfig(error.to_string()))?,
            );
            offset += length;
        }
        Ok(ranges)
    }
}

pub fn verify_exhaustive<S: VerificationStore>(
    data: &mut [S],
    parity: &mut S,
    config: &ScanConfig,
    evidence: &ChecksumEvidence,
) -> Result<VerificationReport, VerificationError> {
    let ranges = config.region_ranges()?;
    verify_ranges(
        data,
        parity,
        config,
        evidence,
        ScanMode::Exhaustive,
        &ranges,
    )
}

pub fn verify_sampled<S: VerificationStore>(
    data: &mut [S],
    parity: &mut S,
    config: &ScanConfig,
    evidence: &ChecksumEvidence,
    ranges: &[ByteRange],
) -> Result<VerificationReport, VerificationError> {
    config.validate()?;
    if ranges.is_empty() {
        return Err(VerificationError::InvalidConfig(
            "sample must contain at least one range".to_owned(),
        ));
    }
    if ranges.len() > config.max_regions {
        return Err(VerificationError::InvalidConfig(format!(
            "sample needs {} regions but the bound is {}",
            ranges.len(),
            config.max_regions
        )));
    }
    for range in ranges {
        validate_scan_range(config, *range)?;
    }
    verify_ranges(data, parity, config, evidence, ScanMode::Sampled, ranges)
}

fn verify_ranges<S: VerificationStore>(
    data: &mut [S],
    parity: &mut S,
    config: &ScanConfig,
    evidence: &ChecksumEvidence,
    mode: ScanMode,
    ranges: &[ByteRange],
) -> Result<VerificationReport, VerificationError> {
    if data.len() != config.geometry.data_count() {
        return Err(VerificationError::InvalidConfig(format!(
            "geometry expects {} data members, got {}",
            config.geometry.data_count(),
            data.len()
        )));
    }
    validate_identities(data, parity)?;
    evidence.validate(data.len(), ranges.len())?;
    let run_id = NEXT_VERIFICATION_RUN_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| {
            VerificationError::InvalidConfig("verification run IDs exhausted".to_owned())
        })?;
    let data_identities = data.iter().map(VerificationStore::identity).collect();
    let parity_identity = parity.identity();

    let mut regions = Vec::with_capacity(ranges.len());
    for (region_index, range) in ranges.iter().copied().enumerate() {
        let (data_bytes, parity_bytes) = match read_region(data, parity, &config.geometry, range) {
            Ok(value) => value,
            Err(failure) => {
                regions.push(RegionReport {
                    range,
                    disposition: RegionDisposition::Incomplete {
                        member: failure.member,
                    },
                    data_evidence: Vec::new(),
                    parity_evidence: EvidenceStatus::Absent,
                });
                continue;
            }
        };
        let data_evidence = assess_data_evidence(evidence, region_index, &data_bytes)?;
        let parity_evidence = evidence.parity[region_index].assess(&parity_bytes)?;
        let expected_parity = expected_parity(&data_bytes, range.length)?;
        let disposition = if expected_parity == parity_bytes {
            RegionDisposition::Match {
                evidence_complete: data_evidence.iter().all(|status| status.is_current_match())
                    && parity_evidence.is_current_match(),
            }
        } else {
            RegionDisposition::Mismatch(classify_mismatch(&data_evidence, parity_evidence))
        };
        regions.push(RegionReport {
            range,
            disposition,
            data_evidence,
            parity_evidence,
        });
    }

    let parity_consistent = regions
        .iter()
        .all(|region| matches!(region.disposition, RegionDisposition::Match { .. }));
    let exhaustive_complete = mode == ScanMode::Exhaustive
        && parity_consistent
        && regions.len() == config.region_count()?;
    Ok(VerificationReport {
        mode,
        regions,
        parity_consistent,
        exhaustive_complete,
        payload_writes: 0,
        clean_authorized: false,
        binding: crate::report::VerificationBinding {
            run_id,
            data_identities,
            parity_identity,
        },
    })
}

fn assess_data_evidence(
    evidence: &ChecksumEvidence,
    region_index: usize,
    data_bytes: &[Vec<u8>],
) -> Result<Vec<EvidenceStatus>, VerificationError> {
    evidence
        .data
        .iter()
        .zip(data_bytes)
        .map(|(records, bytes)| records[region_index].assess(bytes))
        .collect()
}

fn classify_mismatch(data: &[EvidenceStatus], parity: EvidenceStatus) -> MismatchClass {
    let invalid_data = data
        .iter()
        .enumerate()
        .filter_map(|(slot, status)| (*status == EvidenceStatus::CurrentMismatch).then_some(slot))
        .collect::<Vec<_>>();
    let all_data_match = data.iter().all(|status| status.is_current_match());
    let all_data_available = data.iter().all(|status| !status.is_unavailable());

    if all_data_match && parity == EvidenceStatus::CurrentMismatch {
        return MismatchClass::ParityIdentified;
    }
    if invalid_data.len() == 1 && all_data_available && parity == EvidenceStatus::CurrentMatch {
        return MismatchClass::DataIdentified {
            slot: invalid_data[0],
        };
    }
    if data.iter().all(|status| status.is_current_match()) && parity == EvidenceStatus::CurrentMatch
    {
        return MismatchClass::EvidenceConflict;
    }
    MismatchClass::Ambiguous
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReadFailure {
    pub member: MemberRef,
    pub message: String,
}

pub(crate) fn read_member_region<S: VerificationStore>(
    store: &mut S,
    geometry: &Geometry,
    slot: usize,
    range: ByteRange,
) -> Result<Vec<u8>, ReadFailure> {
    let member_length = geometry.data_length(slot).ok_or_else(|| ReadFailure {
        member: MemberRef::Data(slot),
        message: "data slot is outside geometry".to_owned(),
    })?;
    let output_length = usize::try_from(range.length).map_err(|_| ReadFailure {
        member: MemberRef::Data(slot),
        message: "range length does not fit memory".to_owned(),
    })?;
    if range.offset >= member_length {
        return Ok(vec![0; output_length]);
    }
    let read_end = range.end().min(member_length);
    let read_range =
        ByteRange::new(range.offset, read_end - range.offset).map_err(|error| ReadFailure {
            member: MemberRef::Data(slot),
            message: error.to_string(),
        })?;
    let bytes = store.read_exact(read_range).map_err(|error| ReadFailure {
        member: MemberRef::Data(slot),
        message: error.to_string(),
    })?;
    let expected = usize::try_from(read_range.length).map_err(|_| ReadFailure {
        member: MemberRef::Data(slot),
        message: "read length does not fit memory".to_owned(),
    })?;
    if bytes.len() != expected {
        return Err(ReadFailure {
            member: MemberRef::Data(slot),
            message: format!("short read: expected {expected}, got {}", bytes.len()),
        });
    }
    let mut output = vec![0; output_length];
    output[..bytes.len()].copy_from_slice(&bytes);
    Ok(output)
}

fn read_region<S: VerificationStore>(
    data: &mut [S],
    parity: &mut S,
    geometry: &Geometry,
    range: ByteRange,
) -> Result<(Vec<Vec<u8>>, Vec<u8>), ReadFailure> {
    let mut data_bytes = Vec::with_capacity(data.len());
    for (slot, store) in data.iter_mut().enumerate() {
        data_bytes.push(read_member_region(store, geometry, slot, range)?);
    }
    let parity_bytes = parity.read_exact(range).map_err(|error| ReadFailure {
        member: MemberRef::Parity,
        message: error.to_string(),
    })?;
    let expected = usize::try_from(range.length).map_err(|_| ReadFailure {
        member: MemberRef::Parity,
        message: "range length does not fit memory".to_owned(),
    })?;
    if parity_bytes.len() != expected {
        return Err(ReadFailure {
            member: MemberRef::Parity,
            message: format!(
                "short read: expected {expected}, got {}",
                parity_bytes.len()
            ),
        });
    }
    Ok((data_bytes, parity_bytes))
}

fn expected_parity(data: &[Vec<u8>], length: u64) -> Result<Vec<u8>, VerificationError> {
    let geometry = Geometry::new(vec![length; data.len()], length)
        .map_err(|error| VerificationError::Codec(error.to_string()))?;
    let references = data.iter().map(Vec::as_slice).collect::<Vec<_>>();
    XorReference
        .compute_parity(&geometry, &references)
        .map_err(|error| VerificationError::Codec(error.to_string()))
}

fn validate_scan_range(config: &ScanConfig, range: ByteRange) -> Result<(), VerificationError> {
    if range.is_empty() || range.end() > config.geometry.parity_length() {
        return Err(VerificationError::InvalidConfig(format!(
            "sample range {range:?} is outside protected geometry"
        )));
    }
    Ok(())
}

fn validate_identities<S: VerificationStore>(
    data: &[S],
    parity: &S,
) -> Result<(), VerificationError> {
    let mut identities = data
        .iter()
        .map(VerificationStore::identity)
        .collect::<Vec<_>>();
    identities.push(parity.identity());
    for (index, identity) in identities.iter().enumerate() {
        if identities
            .iter()
            .skip(index + 1)
            .any(|other| other == identity)
        {
            return Err(VerificationError::InvalidConfig(
                "data and parity stores have aliased or ambiguous identity".to_owned(),
            ));
        }
    }
    Ok(())
}
