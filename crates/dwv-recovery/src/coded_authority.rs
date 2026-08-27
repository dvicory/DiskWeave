use dwv_core::CodedUnitId;
use dwv_store::{OperationReleasePermit, OperationSlotToken};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Exact coded units produced by the topology/geometry mapper.
///
/// This is input, not authority. Admission validates liveness and overlap and
/// returns the owner-issued `CodedAdmission` capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedClaimInput {
    units: BTreeSet<CodedUnitId>,
}

impl CodedClaimInput {
    pub fn mapped(units: impl IntoIterator<Item = CodedUnitId>) -> Self {
        Self {
            units: units.into_iter().collect(),
        }
    }

    pub fn units(&self) -> &BTreeSet<CodedUnitId> {
        &self.units
    }
}

/// A complete, externally validated coded claim accepted by the authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedClaim {
    units: BTreeSet<CodedUnitId>,
}

impl CodedClaim {
    pub fn units(&self) -> &BTreeSet<CodedUnitId> {
        &self.units
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.units.iter().any(|unit| other.units.contains(unit))
    }
}
/// Owner-issued proof that one complete coded claim was admitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedAdmission {
    operation: OperationSlotToken,
    claim: CodedClaim,
    sequence: u64,
}

impl CodedAdmission {
    pub const fn operation(&self) -> OperationSlotToken {
        self.operation
    }

    pub fn units(&self) -> &BTreeSet<CodedUnitId> {
        self.claim.units()
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
}

/// Owner-issued proof that all earlier coded admissions are either represented
/// by the supplied active set or were released under lifecycle authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodedHistoryCoverage {
    through_sequence: u64,
}

impl CodedHistoryCoverage {
    pub(crate) const fn through_sequence(self) -> u64 {
        self.through_sequence
    }
}

/// Owner-issued closed-set boundary. No admission can cross this cut.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodedCaptureBoundary {
    capture_sequence: u64,
}

impl CodedCaptureBoundary {
    pub const fn capture_sequence(self) -> u64 {
        self.capture_sequence
    }
}

/// One atomic owner-issued capture seed binding history, cut, and active claims.
pub struct CodedCaptureEstablishment {
    history: CodedHistoryCoverage,
    boundary: CodedCaptureBoundary,
    active_admissions: Vec<CodedAdmission>,
}

impl CodedCaptureEstablishment {
    pub(crate) fn into_parts(
        self,
    ) -> (
        CodedHistoryCoverage,
        CodedCaptureBoundary,
        Vec<CodedAdmission>,
    ) {
        (self.history, self.boundary, self.active_admissions)
    }
}

/// Owner-issued bound covering every coded admission observed so far.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CodedAuthorityFrontier {
    admission_sequence: u64,
}

impl CodedAuthorityFrontier {
    pub(crate) const fn admission_sequence(self) -> u64 {
        self.admission_sequence
    }
}

/// Non-terminal result of complete coded-claim admission.
///
/// `Contended` leaves the operation pending/backpressured until the held
/// claim clears; it is not a terminal I/O failure.
#[must_use = "coded admission may be contended and must be handled explicitly"]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodedAdmissionOutcome {
    Admitted(CodedAdmission),
    Contended,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedOperationPhase {
    Held,
    EffectPossible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodedEffectPermit {
    operation: OperationSlotToken,
}

impl CodedEffectPermit {
    pub const fn operation(self) -> OperationSlotToken {
        self.operation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodedClaimRelease {
    operation: OperationSlotToken,
    frontier: CodedAuthorityFrontier,
}

impl CodedClaimRelease {
    pub const fn operation(&self) -> OperationSlotToken {
        self.operation
    }

    pub(crate) const fn frontier(&self) -> CodedAuthorityFrontier {
        self.frontier
    }

    #[cfg(test)]
    pub(crate) const fn for_test(operation: OperationSlotToken, admission_sequence: u64) -> Self {
        Self {
            operation,
            frontier: CodedAuthorityFrontier { admission_sequence },
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodedAuthorityError {
    EmptyClaim,
    OperationAlreadyAdmitted(OperationSlotToken),
    OperationNotAdmitted(OperationSlotToken),
    EffectAlreadyPermitted(OperationSlotToken),
    AdmissionSequenceExhausted,
}

impl fmt::Display for CodedAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyClaim => formatter.write_str("coded claim is empty"),

            Self::OperationAlreadyAdmitted(operation) => {
                write!(formatter, "operation {operation:?} is already admitted")
            }
            Self::OperationNotAdmitted(operation) => {
                write!(formatter, "operation {operation:?} is not admitted")
            }
            Self::EffectAlreadyPermitted(operation) => {
                write!(
                    formatter,
                    "effect is already permitted for operation {operation:?}"
                )
            }
            Self::AdmissionSequenceExhausted => {
                formatter.write_str("coded admission sequence is exhausted")
            }
        }
    }
}

impl std::error::Error for CodedAuthorityError {}

/// dwv:req req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts
/// This owner tracks active claims only. Operation-slot lifetime and token reuse
/// are enforced by the service boundary before admission; this lower primitive
/// does not retain released-generation tombstones.
#[derive(Clone, Default)]
pub struct CodedRangeAuthority {
    claims: BTreeMap<OperationSlotToken, CodedClaim>,
    admission_sequences: BTreeMap<OperationSlotToken, u64>,
    effect_possible: BTreeSet<OperationSlotToken>,
    admission_high_water: u64,
}

impl CodedRangeAuthority {
    pub fn new() -> Self {
        Self::default()
    }

    /// Admit one complete claim before any dependent effect.
    pub fn admit(
        &mut self,
        operation: OperationSlotToken,
        input: CodedClaimInput,
    ) -> Result<CodedAdmissionOutcome, CodedAuthorityError> {
        if self.claims.contains_key(&operation) {
            return Err(CodedAuthorityError::OperationAlreadyAdmitted(operation));
        }
        if input.units.is_empty() {
            return Err(CodedAuthorityError::EmptyClaim);
        }

        let claim = CodedClaim { units: input.units };
        if self
            .claims
            .values()
            .any(|held_claim| claim.intersects(held_claim))
        {
            return Ok(CodedAdmissionOutcome::Contended);
        }
        let admission_sequence = self
            .admission_high_water
            .checked_add(1)
            .ok_or(CodedAuthorityError::AdmissionSequenceExhausted)?;
        self.admission_high_water = admission_sequence;
        self.admission_sequences
            .insert(operation, admission_sequence);
        self.claims.insert(operation, claim.clone());
        Ok(CodedAdmissionOutcome::Admitted(CodedAdmission {
            operation,
            claim,
            sequence: admission_sequence,
        }))
    }

    /// Issue permission for the dependent effect only after coded admission.
    pub fn permit_effect(
        &mut self,
        operation: OperationSlotToken,
    ) -> Result<CodedEffectPermit, CodedAuthorityError> {
        if !self.claims.contains_key(&operation) {
            return Err(CodedAuthorityError::OperationNotAdmitted(operation));
        }
        if !self.effect_possible.insert(operation) {
            return Err(CodedAuthorityError::EffectAlreadyPermitted(operation));
        }
        Ok(CodedEffectPermit { operation })
    }

    pub fn operation_phase(&self, operation: OperationSlotToken) -> Option<CodedOperationPhase> {
        self.claims.get(&operation).map(|_| {
            if self.effect_possible.contains(&operation) {
                CodedOperationPhase::EffectPossible
            } else {
                CodedOperationPhase::Held
            }
        })
    }

    pub fn active_claim(&self, operation: OperationSlotToken) -> Option<&CodedClaim> {
        self.claims.get(&operation)
    }

    pub fn active_claims(&self) -> impl Iterator<Item = (OperationSlotToken, &CodedClaim)> + '_ {
        self.claims
            .iter()
            .map(|(operation, claim)| (*operation, claim))
    }
    fn active_admissions(&self) -> Vec<CodedAdmission> {
        self.claims
            .iter()
            .map(|(operation, claim)| CodedAdmission {
                operation: *operation,
                claim: claim.clone(),
                sequence: self.admission_sequences[operation],
            })
            .collect()
    }
    pub fn admission(&self, operation: OperationSlotToken) -> Option<CodedAdmission> {
        let claim = self.claims.get(&operation)?.clone();
        Some(CodedAdmission {
            operation,
            claim,
            sequence: *self.admission_sequences.get(&operation)?,
        })
    }

    /// Establish one atomic owner-produced history, cut, and active set.
    pub fn capture_boundary(&mut self) -> Result<CodedCaptureEstablishment, CodedAuthorityError> {
        let active_admissions = self.active_admissions();
        let through_sequence = self.admission_high_water;
        let capture_sequence = through_sequence
            .checked_add(1)
            .ok_or(CodedAuthorityError::AdmissionSequenceExhausted)?;
        self.admission_high_water = capture_sequence;
        Ok(CodedCaptureEstablishment {
            history: CodedHistoryCoverage { through_sequence },
            boundary: CodedCaptureBoundary { capture_sequence },
            active_admissions,
        })
    }

    pub fn admission_sequence(&self, operation: OperationSlotToken) -> Option<u64> {
        self.admission_sequences.get(&operation).copied()
    }

    pub const fn admission_high_water(&self) -> u64 {
        self.admission_high_water
    }

    pub fn retention_frontier(&self) -> CodedAuthorityFrontier {
        CodedAuthorityFrontier {
            admission_sequence: self.admission_high_water,
        }
    }

    pub fn advance_admission_high_water(&mut self, sequence: u64) {
        self.admission_high_water = self.admission_high_water.max(sequence);
    }

    /// Consume the exact generation-qualified lifecycle release fact.
    ///
    /// The caller supplies only the operation token carried by the canonical
    /// Consume one exact-generation lifecycle release permit.
    pub fn release(
        &mut self,
        permit: OperationReleasePermit,
    ) -> Result<CodedClaimRelease, CodedAuthorityError> {
        self.release_operation(permit.operation())
    }

    #[cfg(test)]
    fn release_unchecked(
        &mut self,
        operation: OperationSlotToken,
    ) -> Result<CodedClaimRelease, CodedAuthorityError> {
        self.release_operation(operation)
    }

    fn release_operation(
        &mut self,
        operation: OperationSlotToken,
    ) -> Result<CodedClaimRelease, CodedAuthorityError> {
        if !self.claims.contains_key(&operation) {
            return Err(CodedAuthorityError::OperationNotAdmitted(operation));
        }
        let admission_sequence = self.admission_sequences[&operation];
        self.claims.remove(&operation);
        self.effect_possible.remove(&operation);
        self.admission_sequences.remove(&operation);
        Ok(CodedClaimRelease {
            operation,
            frontier: CodedAuthorityFrontier { admission_sequence },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(index: u32, generation: u32) -> OperationSlotToken {
        OperationSlotToken::new(index, generation)
    }

    fn claim(units: impl IntoIterator<Item = u32>) -> CodedClaimInput {
        CodedClaimInput::mapped(units.into_iter().map(|unit| CodedUnitId(u64::from(unit))))
    }

    #[test]
    fn coded_overlap_contends_but_disjoint_claims_coexist() {
        let mut authority = CodedRangeAuthority::new();
        assert!(matches!(
            authority.admit(token(0, 1), claim([0])).unwrap(),
            CodedAdmissionOutcome::Admitted(_)
        ));
        assert_eq!(
            authority.admit(token(1, 1), claim([0, 1])).unwrap(),
            CodedAdmissionOutcome::Contended
        );
        assert!(matches!(
            authority.admit(token(2, 1), claim([1])).unwrap(),
            CodedAdmissionOutcome::Admitted(_)
        ));
    }

    #[test]
    fn admitted_witness_binds_operation_claim_and_sequence() {
        let mut authority = CodedRangeAuthority::new();
        let operation = token(0, 1);
        let CodedAdmissionOutcome::Admitted(admission) =
            authority.admit(operation, claim([3])).unwrap()
        else {
            panic!("disjoint claim must admit");
        };
        assert_eq!(admission.operation(), operation);
        assert_eq!(admission.units(), &BTreeSet::from([CodedUnitId(3)]));
        assert_eq!(admission.sequence(), 1);
    }

    #[test]
    fn coded_claim_supports_unit_above_u32_boundary() {
        let unit = CodedUnitId(u64::from(u32::MAX) + 1);
        let mut authority = CodedRangeAuthority::new();
        let operation = token(0, 1);
        assert!(matches!(
            authority
                .admit(operation, CodedClaimInput::mapped([unit]))
                .unwrap(),
            CodedAdmissionOutcome::Admitted(_)
        ));
        assert_eq!(
            authority.active_claim(operation).unwrap().units(),
            &BTreeSet::from([unit])
        );
    }

    #[test]
    fn empty_claim_is_rejected() {
        let mut authority = CodedRangeAuthority::new();
        assert_eq!(
            authority.admit(token(0, 1), CodedClaimInput::mapped([])),
            Err(CodedAuthorityError::EmptyClaim)
        );
    }

    #[test]
    fn duplicate_active_admission_is_rejected() {
        let mut authority = CodedRangeAuthority::new();
        let operation = token(0, 1);
        assert!(matches!(
            authority.admit(operation, claim([0])).unwrap(),
            CodedAdmissionOutcome::Admitted(_)
        ));
        assert_eq!(
            authority.admit(operation, claim([1])),
            Err(CodedAuthorityError::OperationAlreadyAdmitted(operation))
        );
    }

    #[test]
    fn release_is_exact_generation_and_removes_active_claim() {
        let mut authority = CodedRangeAuthority::new();
        let current = token(0, 2);
        let stale = token(0, 1);
        assert!(matches!(
            authority.admit(current, claim([0])).unwrap(),
            CodedAdmissionOutcome::Admitted(_)
        ));
        assert_eq!(
            authority.release_unchecked(stale),
            Err(CodedAuthorityError::OperationNotAdmitted(stale))
        );
        let release = authority.release_unchecked(current).unwrap();
        assert_eq!(release.operation(), current);
        assert_eq!(authority.active_claim(current), None);
    }
}
