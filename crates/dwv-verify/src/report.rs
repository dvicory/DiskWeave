use crate::VerificationIdentity;
use dwv_core::ByteRange;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanMode {
    Exhaustive,
    Sampled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemberRef {
    Data(usize),
    Parity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceStatus {
    CurrentMatch,
    CurrentMismatch,
    Absent,
    Stale,
    Conflicting,
}

impl EvidenceStatus {
    pub const fn is_current_match(self) -> bool {
        matches!(self, Self::CurrentMatch)
    }

    pub const fn is_unavailable(self) -> bool {
        matches!(self, Self::Absent | Self::Stale | Self::Conflicting)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MismatchClass {
    ParityIdentified,
    DataIdentified { slot: usize },
    Ambiguous,
    EvidenceConflict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionDisposition {
    Match { evidence_complete: bool },
    Mismatch(MismatchClass),
    Incomplete { member: MemberRef },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionReport {
    pub range: ByteRange,
    pub disposition: RegionDisposition,
    pub data_evidence: Vec<EvidenceStatus>,
    pub parity_evidence: EvidenceStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub(crate) mode: ScanMode,
    pub(crate) regions: Vec<RegionReport>,
    pub(crate) parity_consistent: bool,
    pub(crate) exhaustive_complete: bool,
    pub(crate) payload_writes: usize,
    pub(crate) clean_authorized: bool,
    pub(crate) binding: VerificationBinding,
}

impl VerificationReport {
    pub const fn mode(&self) -> ScanMode {
        self.mode
    }

    pub fn regions(&self) -> &[RegionReport] {
        &self.regions
    }

    pub const fn parity_consistent(&self) -> bool {
        self.parity_consistent
    }

    pub const fn exhaustive_complete(&self) -> bool {
        self.exhaustive_complete
    }

    pub const fn payload_writes(&self) -> usize {
        self.payload_writes
    }

    pub const fn can_authorize_clean(&self) -> bool {
        self.clean_authorized
    }

    pub fn matching_regions(&self) -> usize {
        self.regions
            .iter()
            .filter(|region| matches!(region.disposition, RegionDisposition::Match { .. }))
            .count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerificationBinding {
    pub run_id: u64,
    pub data_identities: Vec<VerificationIdentity>,
    pub parity_identity: VerificationIdentity,
}
