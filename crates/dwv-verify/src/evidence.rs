use crate::{error::VerificationError, report::EvidenceStatus};
use dwv_recovery::{
    Blake3Provider, ChecksumProfileId, ChecksumRecord, ChecksumSetGeneration, ChecksumState,
    ContentGeneration, Digest, DigestProvider,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceKind {
    Absent,
    Stale,
    Conflicting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestEvidence {
    Current(Digest),
    Absent,
    Stale,
    Conflicting,
}

impl DigestEvidence {
    pub fn from_record(
        record: Option<&ChecksumRecord>,
        profile: ChecksumProfileId,
        set_generation: ChecksumSetGeneration,
        content_generation: ContentGeneration,
    ) -> Self {
        let Some(record) = record else {
            return Self::Absent;
        };
        match record.state {
            ChecksumState::Absent => Self::Absent,
            ChecksumState::Stale => Self::Stale,
            ChecksumState::Valid => {
                if record.is_valid()
                    && record.profile == profile
                    && record.set_generation == set_generation
                    && record.content_generation == content_generation
                {
                    Self::Current(record.digest.expect("valid checksum has a digest"))
                } else {
                    Self::Conflicting
                }
            }
        }
    }

    pub(crate) fn assess(self, bytes: &[u8]) -> Result<EvidenceStatus, VerificationError> {
        match self {
            Self::Current(expected) => {
                let actual = Blake3Provider
                    .digest(bytes)
                    .map_err(|error| VerificationError::Codec(error.to_string()))?;
                Ok(if actual == expected {
                    EvidenceStatus::CurrentMatch
                } else {
                    EvidenceStatus::CurrentMismatch
                })
            }
            Self::Absent => Ok(EvidenceStatus::Absent),
            Self::Stale => Ok(EvidenceStatus::Stale),
            Self::Conflicting => Ok(EvidenceStatus::Conflicting),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChecksumEvidence {
    pub data: Vec<Vec<DigestEvidence>>,
    pub parity: Vec<DigestEvidence>,
}

impl ChecksumEvidence {
    pub fn new(data: Vec<Vec<DigestEvidence>>, parity: Vec<DigestEvidence>) -> Self {
        Self { data, parity }
    }

    pub(crate) fn validate(
        &self,
        data_count: usize,
        region_count: usize,
    ) -> Result<(), VerificationError> {
        if self.data.len() != data_count
            || self
                .data
                .iter()
                .any(|records| records.len() != region_count)
            || self.parity.len() != region_count
        {
            return Err(VerificationError::InvalidEvidence(format!(
                "expected {data_count} data records and {region_count} regions"
            )));
        }
        Ok(())
    }
}
