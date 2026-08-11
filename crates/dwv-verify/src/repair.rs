use crate::{
    error::VerificationError,
    report::{MismatchClass, RegionDisposition, VerificationBinding, VerificationReport},
    scan::{ScanConfig, read_member_region},
    store::VerificationStore,
};
use dwv_codec::{Geometry, ParityCodec, XorReference};
use dwv_core::ByteRange;
use dwv_recovery::{Blake3Provider, Digest, DigestProvider};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairTarget {
    Data { slot: usize },
    Parity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairCandidate {
    range: ByteRange,
    target: RepairTarget,
    classification: MismatchClass,
    binding: VerificationBinding,
}

impl RepairCandidate {
    pub const fn range(&self) -> ByteRange {
        self.range
    }

    pub const fn target(&self) -> RepairTarget {
        self.target
    }

    pub const fn classification(&self) -> MismatchClass {
        self.classification
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RepairRefusal {
    pub range: ByteRange,
    pub classification: MismatchClass,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairPlan {
    pub candidates: Vec<RepairCandidate>,
    pub refused: Vec<RepairRefusal>,
}

pub fn plan_repairs(report: &VerificationReport) -> RepairPlan {
    let mut plan = RepairPlan {
        candidates: Vec::new(),
        refused: Vec::new(),
    };
    for region in &report.regions {
        let RegionDisposition::Mismatch(classification) = region.disposition else {
            continue;
        };
        match classification {
            MismatchClass::ParityIdentified => plan.candidates.push(RepairCandidate {
                range: region.range,
                target: RepairTarget::Parity,
                classification,
                binding: report.binding.clone(),
            }),
            MismatchClass::DataIdentified { slot } => {
                plan.candidates.push(RepairCandidate {
                    range: region.range,
                    target: RepairTarget::Data { slot },
                    classification,
                    binding: report.binding.clone(),
                });
            }
            MismatchClass::Ambiguous | MismatchClass::EvidenceConflict => {
                plan.refused.push(RepairRefusal {
                    range: region.range,
                    classification,
                });
            }
        }
    }
    plan
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairOutcome {
    range: ByteRange,
    target: RepairTarget,
    digest: Digest,
    target_identity: crate::VerificationIdentity,
    binding: VerificationBinding,
}

impl RepairOutcome {
    pub const fn range(&self) -> ByteRange {
        self.range
    }

    pub const fn target(&self) -> RepairTarget {
        self.target
    }

    pub const fn digest(&self) -> Digest {
        self.digest
    }

    pub const fn target_identity(&self) -> crate::VerificationIdentity {
        self.target_identity
    }

    /// Returns true only for a read-back-verified repair produced from this
    /// exact verification run and expected mismatch.
    pub fn verifies(
        &self,
        report: &VerificationReport,
        range: ByteRange,
        target: RepairTarget,
    ) -> bool {
        self.binding == report.binding && self.range == range && self.target == target
    }
}

pub fn apply_repair<S: VerificationStore, T: VerificationStore>(
    config: &ScanConfig,
    data: &mut [S],
    parity: &mut S,
    target: &mut T,
    candidate: RepairCandidate,
) -> Result<RepairOutcome, VerificationError> {
    validate_candidate(config, data.len(), &candidate)?;
    if candidate.binding.data_identities
        != data
            .iter()
            .map(VerificationStore::identity)
            .collect::<Vec<_>>()
        || candidate.binding.parity_identity != parity.identity()
    {
        return Err(VerificationError::Repair(
            "repair sources differ from the verification run".to_owned(),
        ));
    }
    let target_identity = target.identity();
    if data.iter().any(|store| store.identity() == target_identity)
        || parity.identity() == target_identity
    {
        return Err(VerificationError::Repair(
            "repair target aliases a source member".to_owned(),
        ));
    }
    let local_geometry = local_geometry(data.len(), candidate.range.length)?;
    let codec_range = dwv_codec::ByteRange::new(0, candidate.range.length)
        .map_err(|error| VerificationError::Codec(error.to_string()))?;

    let bytes = match candidate.target {
        RepairTarget::Parity => {
            let mut data_bytes = Vec::with_capacity(data.len());
            for (slot, store) in data.iter_mut().enumerate() {
                data_bytes.push(
                    read_member_region(store, &config.geometry, slot, candidate.range).map_err(
                        |failure| {
                            VerificationError::Store(format!(
                                "read {:?}: {}",
                                failure.member, failure.message
                            ))
                        },
                    )?,
                );
            }
            let references = data_bytes.iter().map(Vec::as_slice).collect::<Vec<_>>();
            XorReference
                .compute_parity(&local_geometry, &references)
                .map_err(|error| VerificationError::Codec(error.to_string()))?
        }
        RepairTarget::Data { slot } => {
            let parity_bytes = read_exact(parity, candidate.range)?;
            let mut survivors = Vec::with_capacity(data.len());
            for (survivor_slot, store) in data.iter_mut().enumerate() {
                if survivor_slot == slot {
                    survivors.push(None);
                } else {
                    let bytes =
                        read_member_region(store, &config.geometry, survivor_slot, candidate.range)
                            .map_err(|failure| {
                                VerificationError::Store(format!(
                                    "read {:?}: {}",
                                    failure.member, failure.message
                                ))
                            })?;
                    survivors.push(Some(bytes));
                }
            }
            let references = survivors
                .iter()
                .map(|bytes| bytes.as_deref())
                .collect::<Vec<_>>();
            XorReference
                .reconstruct(
                    &local_geometry,
                    &parity_bytes,
                    slot,
                    codec_range,
                    &references,
                )
                .map_err(|error| VerificationError::Codec(error.to_string()))?
        }
    };

    target
        .write_exact(candidate.range, &bytes)
        .map_err(|error| VerificationError::Store(error.to_string()))?;
    let readback = read_exact(target, candidate.range)?;
    if readback != bytes {
        return Err(VerificationError::Repair(
            "repair target readback differs from reconstructed bytes".to_owned(),
        ));
    }

    match candidate.target {
        RepairTarget::Parity => {
            let mut data_bytes = Vec::with_capacity(data.len());
            for (slot, store) in data.iter_mut().enumerate() {
                data_bytes.push(
                    read_member_region(store, &config.geometry, slot, candidate.range).map_err(
                        |failure| {
                            VerificationError::Store(format!(
                                "read {:?}: {}",
                                failure.member, failure.message
                            ))
                        },
                    )?,
                );
            }
            let references = data_bytes.iter().map(Vec::as_slice).collect::<Vec<_>>();
            let expected = XorReference
                .compute_parity(&local_geometry, &references)
                .map_err(|error| VerificationError::Codec(error.to_string()))?;
            if expected != readback {
                return Err(VerificationError::Repair(
                    "repaired parity does not satisfy the data equation".to_owned(),
                ));
            }
        }
        RepairTarget::Data { slot } => {
            let parity_bytes = read_exact(parity, candidate.range)?;
            let mut data_bytes = Vec::with_capacity(data.len());
            for (data_slot, store) in data.iter_mut().enumerate() {
                if data_slot == slot {
                    data_bytes.push(readback.clone());
                } else {
                    data_bytes.push(
                        read_member_region(store, &config.geometry, data_slot, candidate.range)
                            .map_err(|failure| {
                                VerificationError::Store(format!(
                                    "read {:?}: {}",
                                    failure.member, failure.message
                                ))
                            })?,
                    );
                }
            }
            let references = data_bytes.iter().map(Vec::as_slice).collect::<Vec<_>>();
            let expected = XorReference
                .compute_parity(&local_geometry, &references)
                .map_err(|error| VerificationError::Codec(error.to_string()))?;
            if expected != parity_bytes {
                return Err(VerificationError::Repair(
                    "repaired data does not satisfy the parity equation".to_owned(),
                ));
            }
        }
    }

    let digest = Blake3Provider
        .digest(&readback)
        .map_err(|error| VerificationError::Codec(error.to_string()))?;
    Ok(RepairOutcome {
        range: candidate.range,
        target: candidate.target,
        digest,
        target_identity,
        binding: candidate.binding,
    })
}

fn validate_candidate(
    config: &ScanConfig,
    data_count: usize,
    candidate: &RepairCandidate,
) -> Result<(), VerificationError> {
    if candidate.range.is_empty()
        || candidate.range.end() > config.geometry.parity_length()
        || candidate.range.length > config.region_size
    {
        return Err(VerificationError::InvalidConfig(
            "repair range is outside the bounded scan geometry".to_owned(),
        ));
    }
    match (candidate.target, candidate.classification) {
        (RepairTarget::Parity, MismatchClass::ParityIdentified) => {}
        (RepairTarget::Data { slot }, MismatchClass::DataIdentified { slot: classified })
            if slot == classified && slot < data_count => {}
        _ => {
            return Err(VerificationError::Repair(
                "repair target does not match an identified classification".to_owned(),
            ));
        }
    }
    Ok(())
}

fn local_geometry(data_count: usize, length: u64) -> Result<Geometry, VerificationError> {
    Geometry::new(vec![length; data_count], length)
        .map_err(|error| VerificationError::Codec(error.to_string()))
}

fn read_exact<S: VerificationStore>(
    store: &mut S,
    range: ByteRange,
) -> Result<Vec<u8>, VerificationError> {
    let bytes = store
        .read_exact(range)
        .map_err(|error| VerificationError::Store(error.to_string()))?;
    let expected = usize::try_from(range.length).map_err(|_| {
        VerificationError::InvalidConfig("range length does not fit memory".to_owned())
    })?;
    if bytes.len() != expected {
        return Err(VerificationError::Store(format!(
            "short read: expected {expected}, got {}",
            bytes.len()
        )));
    }
    Ok(bytes)
}
