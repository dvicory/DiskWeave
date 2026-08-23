use dwv_core::CodedUnitId;
use dwv_store::OperationSlotToken;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// External mapping input for one complete coded parity/codeword claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedClaimInput {
    units: BTreeSet<CodedUnitId>,
    complete: bool,
    validated: bool,
}

impl CodedClaimInput {
    pub fn new(
        units: impl IntoIterator<Item = CodedUnitId>,
        complete: bool,
        validated: bool,
    ) -> Self {
        Self {
            units: units.into_iter().collect(),
            complete,
            validated,
        }
    }

    pub fn complete(units: impl IntoIterator<Item = CodedUnitId>) -> Self {
        Self::new(units, true, true)
    }

    pub fn units(&self) -> &BTreeSet<CodedUnitId> {
        &self.units
    }

    pub const fn is_complete(&self) -> bool {
        self.complete
    }

    pub const fn is_validated(&self) -> bool {
        self.validated
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
/// Non-terminal result of complete coded-claim admission.
///
/// `Contended` leaves the operation pending/backpressured until the held
/// claim clears; it is not a terminal I/O failure.
#[must_use = "coded admission may be contended and must be handled explicitly"]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedAdmissionOutcome {
    Admitted,
    Contended,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedOperationPhase {
    Held,
    EffectPossible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodedClaimRelease {
    pub operation: OperationSlotToken,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodedAuthorityError {
    EmptyClaim,
    IncompleteClaim,
    UnvalidatedClaim,
    OperationAlreadyAdmitted(OperationSlotToken),
    OperationNotAdmitted(OperationSlotToken),
    EffectAlreadyPermitted(OperationSlotToken),
    ReleaseAuthorizationMissing(OperationSlotToken),
    ReleaseAuthorizationMismatch {
        operation: OperationSlotToken,
        authorized: OperationSlotToken,
    },
}

impl fmt::Display for CodedAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyClaim => formatter.write_str("coded claim is empty"),
            Self::IncompleteClaim => formatter.write_str("coded claim is incomplete"),
            Self::UnvalidatedClaim => formatter.write_str("coded claim is not validated"),
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
            Self::ReleaseAuthorizationMissing(operation) => write!(
                formatter,
                "release authorization is missing for operation {operation:?}"
            ),
            Self::ReleaseAuthorizationMismatch {
                operation,
                authorized,
            } => write!(
                formatter,
                "release authorization {authorized:?} does not match operation {operation:?}"
            ),
        }
    }
}

impl std::error::Error for CodedAuthorityError {}

/// dwv:req req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts
/// This owner tracks active claims only. Operation-slot lifetime and token reuse
/// are enforced by the service boundary before admission; this lower primitive
/// does not retain released-generation tombstones.
#[derive(Default)]
pub struct CodedRangeAuthority {
    claims: BTreeMap<OperationSlotToken, CodedClaim>,
    effect_possible: BTreeSet<OperationSlotToken>,
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
        if !input.complete {
            return Err(CodedAuthorityError::IncompleteClaim);
        }
        if !input.validated {
            return Err(CodedAuthorityError::UnvalidatedClaim);
        }
        let claim = CodedClaim { units: input.units };
        if self
            .claims
            .values()
            .any(|held_claim| claim.intersects(held_claim))
        {
            return Ok(CodedAdmissionOutcome::Contended);
        }
        self.claims.insert(operation, claim);
        Ok(CodedAdmissionOutcome::Admitted)
    }

    /// Permit the dependent effect only after coded admission.
    pub fn permit_effect(
        &mut self,
        operation: OperationSlotToken,
    ) -> Result<(), CodedAuthorityError> {
        if !self.claims.contains_key(&operation) {
            return Err(CodedAuthorityError::OperationNotAdmitted(operation));
        }
        if !self.effect_possible.insert(operation) {
            return Err(CodedAuthorityError::EffectAlreadyPermitted(operation));
        }
        Ok(())
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

    /// Consume the exact generation-qualified lifecycle release fact.
    ///
    /// The caller supplies only the operation token carried by the canonical
    /// `ReleaseAuthorization`; lifecycle predicates remain outside this owner.
    pub fn release(
        &mut self,
        operation: OperationSlotToken,
        release_allowed_operation: Option<OperationSlotToken>,
    ) -> Result<CodedClaimRelease, CodedAuthorityError> {
        if !self.claims.contains_key(&operation) {
            return Err(CodedAuthorityError::OperationNotAdmitted(operation));
        }
        let Some(authorized) = release_allowed_operation else {
            return Err(CodedAuthorityError::ReleaseAuthorizationMissing(operation));
        };
        if authorized != operation {
            return Err(CodedAuthorityError::ReleaseAuthorizationMismatch {
                operation,
                authorized,
            });
        }
        self.claims.remove(&operation);
        self.effect_possible.remove(&operation);
        Ok(CodedClaimRelease { operation })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(index: u32, generation: u32) -> OperationSlotToken {
        OperationSlotToken::new(index, generation)
    }

    fn claim(units: impl IntoIterator<Item = u32>) -> CodedClaimInput {
        CodedClaimInput::complete(units.into_iter().map(CodedUnitId))
    }

    #[test]
    fn coded_overlap_contends_but_disjoint_claims_coexist() {
        let mut authority = CodedRangeAuthority::new();
        assert_eq!(
            authority.admit(token(0, 1), claim([0])).unwrap(),
            CodedAdmissionOutcome::Admitted
        );
        assert!(matches!(
            authority.admit(token(1, 1), claim([0, 1])).unwrap(),
            CodedAdmissionOutcome::Contended
        ));
        assert_eq!(
            authority.admit(token(2, 1), claim([1])).unwrap(),
            CodedAdmissionOutcome::Admitted
        );
    }

    #[test]
    fn incomplete_or_unvalidated_claims_never_enter_authority() {
        let mut authority = CodedRangeAuthority::new();
        assert_eq!(
            authority.admit(
                token(0, 1),
                CodedClaimInput::new([CodedUnitId(0)], false, true),
            ),
            Err(CodedAuthorityError::IncompleteClaim)
        );
        assert_eq!(
            authority.admit(
                token(0, 1),
                CodedClaimInput::new([CodedUnitId(0)], true, false),
            ),
            Err(CodedAuthorityError::UnvalidatedClaim)
        );
        assert_eq!(authority.operation_phase(token(0, 1)), None);
    }
    #[test]
    fn empty_claim_is_rejected() {
        let mut authority = CodedRangeAuthority::new();
        assert_eq!(
            authority.admit(token(0, 1), CodedClaimInput::complete([])),
            Err(CodedAuthorityError::EmptyClaim)
        );
    }

    #[test]
    fn duplicate_active_admission_is_rejected() {
        let mut authority = CodedRangeAuthority::new();
        let operation = token(0, 1);
        assert_eq!(
            authority.admit(operation, claim([0])).unwrap(),
            CodedAdmissionOutcome::Admitted
        );
        assert_eq!(
            authority.admit(operation, claim([1])),
            Err(CodedAuthorityError::OperationAlreadyAdmitted(operation))
        );
    }

    #[test]
    fn release_requires_exact_generation_and_removes_active_claim() {
        let mut authority = CodedRangeAuthority::new();
        let current = token(0, 2);
        assert_eq!(
            authority.admit(current, claim([0])).unwrap(),
            CodedAdmissionOutcome::Admitted
        );
        assert_eq!(
            authority.release(current, Some(token(0, 1))),
            Err(CodedAuthorityError::ReleaseAuthorizationMismatch {
                operation: current,
                authorized: token(0, 1),
            })
        );
        authority.release(current, Some(current)).unwrap();
        assert_eq!(authority.active_claim(current), None);
    }
}
