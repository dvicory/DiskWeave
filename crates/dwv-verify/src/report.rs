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
    pub mode: ScanMode,
    pub regions: Vec<RegionReport>,
    pub parity_consistent: bool,
    pub exhaustive_complete: bool,
    pub payload_writes: usize,
    pub clean_authorized: bool,
}

impl VerificationReport {
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
