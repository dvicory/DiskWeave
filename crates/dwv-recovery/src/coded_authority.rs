use crate::{ChecksumProfileId, CodedGeometryOwner, InvalidationTarget};
use dwv_core::{
    AssignmentGeneration, AssignmentInstanceId, BlockRequest, ByteRange, CodedUnitId,
    CodingPosition, CodingProfile,
};
use dwv_lifecycle_authority::{
    LifecycleAuthorityOwner, LifecycleAuthorityVerifier, ReleaseAuthorization,
};
use dwv_store::{OperationReleasePermit, OperationSlotToken};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_CODED_GEOMETRY_AUTHORITY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CodedGeometryAuthorityId(u64);

impl CodedGeometryAuthorityId {
    pub(crate) fn new() -> Self {
        Self(
            NEXT_CODED_GEOMETRY_AUTHORITY_ID
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                    current.checked_add(1)
                })
                .expect("coded geometry authority identity exhausted"),
        )
    }

    #[cfg(test)]
    const fn for_test() -> Self {
        Self(0)
    }
}

/// Exact mutation-bound coded units issued by the topology/geometry owner.
///
/// Fields are private so production callers cannot replay arbitrary mapped
/// units as authority for another operation or request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedClaimInput {
    binding: CodedClaimBinding,
    units: BTreeSet<CodedUnitId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CodedClaimBinding {
    geometry_authority: CodedGeometryAuthorityId,
    operation: OperationSlotToken,
    request: BlockRequest,
    assignment_instance: AssignmentInstanceId,
    assignment_generation: AssignmentGeneration,
    coding_position: CodingPosition,
    coding_profile: CodingProfile,
    checksum_profile: ChecksumProfileId,
    normalized_ranges: Vec<ByteRange>,
    invalidation_target: InvalidationTarget,
}

impl CodedClaimInput {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn issued(
        geometry_authority: CodedGeometryAuthorityId,
        operation: OperationSlotToken,
        request: BlockRequest,
        assignment_instance: AssignmentInstanceId,
        assignment_generation: AssignmentGeneration,
        coding_position: CodingPosition,
        coding_profile: CodingProfile,
        checksum_profile: ChecksumProfileId,
        normalized_ranges: Vec<ByteRange>,
        invalidation_target: InvalidationTarget,
        units: BTreeSet<CodedUnitId>,
    ) -> Self {
        Self {
            binding: CodedClaimBinding {
                geometry_authority,
                operation,
                request,
                assignment_instance,
                assignment_generation,
                coding_position,
                coding_profile,
                checksum_profile,
                normalized_ranges,
                invalidation_target,
            },
            units,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn matches_owner_context(
        &self,
        geometry_authority: CodedGeometryAuthorityId,
        operation: OperationSlotToken,
        request: &BlockRequest,
        assignment_instance: AssignmentInstanceId,
        assignment_generation: AssignmentGeneration,
        coding_position: CodingPosition,
        coding_profile: CodingProfile,
        checksum_profile: ChecksumProfileId,
        normalized_ranges: &[ByteRange],
        invalidation_target: &InvalidationTarget,
        units: &BTreeSet<CodedUnitId>,
    ) -> bool {
        self.binding.geometry_authority == geometry_authority
            && self.binding.operation == operation
            && self.binding.request == *request
            && self.binding.assignment_instance == assignment_instance
            && self.binding.assignment_generation == assignment_generation
            && self.binding.coding_position == coding_position
            && self.binding.coding_profile == coding_profile
            && self.binding.checksum_profile == checksum_profile
            && self.binding.normalized_ranges == normalized_ranges
            && self.units == *units
            && self.binding.invalidation_target == *invalidation_target
    }

    pub(crate) fn normalized_ranges(&self) -> &[ByteRange] {
        &self.binding.normalized_ranges
    }

    #[cfg(test)]
    fn mapped_for_test(
        operation: OperationSlotToken,
        units: impl IntoIterator<Item = CodedUnitId>,
    ) -> Self {
        use dwv_core::{
            BlockOp, BufferToken, DurabilityIntent, FenceDomain, FrontendId, OrderingIntent,
            RequestId, SlotId, SubmissionSequence, TopologyEpoch,
        };

        Self::issued(
            CodedGeometryAuthorityId::for_test(),
            operation,
            BlockRequest::new(
                RequestId(u64::from(operation.index)),
                FrontendId(1),
                SlotId::from_bytes([1; 16]),
                TopologyEpoch(1),
                BlockOp::Write,
                ByteRange::new(0, 1).unwrap(),
                Some(BufferToken::new(1, 1)),
                OrderingIntent {
                    submission_sequence: SubmissionSequence(0),
                    preflush: false,
                    fence_domain: FenceDomain(1),
                },
                DurabilityIntent::Ordinary,
            ),
            AssignmentInstanceId::from_bytes([2; 16]),
            AssignmentGeneration(1),
            CodingPosition(0),
            CodingProfile::new(1, 1).unwrap(),
            ChecksumProfileId(1),
            vec![ByteRange::new(0, 1).unwrap()],
            InvalidationTarget::new(Vec::new(), Vec::new()),
            units.into_iter().collect(),
        )
    }

    pub fn units(&self) -> &BTreeSet<CodedUnitId> {
        &self.units
    }
}
static NEXT_CODED_ADMISSION_AUTHORITY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CodedAdmissionAuthorityId(u64);

impl CodedAdmissionAuthorityId {
    pub(crate) fn new() -> Self {
        Self(
            NEXT_CODED_ADMISSION_AUTHORITY_ID
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                    current.checked_add(1)
                })
                .expect("coded admission authority identity exhausted"),
        )
    }
}

/// A complete, externally validated coded claim accepted by the authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedClaim {
    binding: CodedClaimBinding,
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
    authority: CodedAdmissionAuthorityId,
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

    pub const fn invalidation_target(&self) -> &InvalidationTarget {
        &self.claim.binding.invalidation_target
    }
    pub(crate) const fn belongs_to(&self, authority: CodedAdmissionAuthorityId) -> bool {
        self.authority.0 == authority.0
    }

    #[cfg(test)]
    pub(crate) fn for_test(
        authority: CodedAdmissionAuthorityId,
        operation: OperationSlotToken,
        units: impl IntoIterator<Item = CodedUnitId>,
        invalidation_target: InvalidationTarget,
        sequence: u64,
    ) -> Self {
        let mut input = CodedClaimInput::mapped_for_test(operation, units);
        input.binding.invalidation_target = invalidation_target;
        Self {
            authority,
            operation,
            claim: CodedClaim {
                binding: input.binding,
                units: input.units,
            },
            sequence,
        }
    }
}

/// Owner-issued proof that all earlier coded admissions are either represented
/// by the supplied active set or were released under lifecycle authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CodedHistoryCoverage {
    through_sequence: u64,
}

impl CodedHistoryCoverage {
    pub(crate) const fn through_sequence(self) -> u64 {
        self.through_sequence
    }
}

/// Owner-issued closed-set boundary. No admission can cross this cut.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CodedCaptureBoundary {
    capture_sequence: u64,
}

impl CodedCaptureBoundary {
    pub(crate) const fn capture_sequence(self) -> u64 {
        self.capture_sequence
    }
}

/// One atomic owner-issued capture seed binding history, cut, and active claims.
pub(crate) struct CodedCaptureEstablishment {
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
/// The admitted path owns its capability inline; boxing would allocate on
/// every successful admission to optimize the uncommon contended result.
#[allow(clippy::large_enum_variant)]
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
    ForeignGeometryAuthority,
    ClaimOperationMismatch {
        expected: OperationSlotToken,
        actual: OperationSlotToken,
    },
    ClaimRequestMismatch,
    ReleaseAuthorizationMismatch {
        expected: OperationSlotToken,
        actual: OperationSlotToken,
    },
    ReleaseAuthorityDomainMismatch(OperationSlotToken),
    OperationAlreadyAdmitted(OperationSlotToken),
    OperationNotAdmitted(OperationSlotToken),
    EffectAlreadyPermitted(OperationSlotToken),
    AdmissionSequenceExhausted,
    StateRevisionExhausted,
}

impl fmt::Display for CodedAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyClaim => formatter.write_str("coded claim is empty"),
            Self::ForeignGeometryAuthority => {
                formatter.write_str("coded claim is foreign to this geometry authority")
            }
            Self::ClaimOperationMismatch { expected, actual } => {
                write!(
                    formatter,
                    "coded claim is bound to operation {actual:?}, not {expected:?}"
                )
            }
            Self::ClaimRequestMismatch => {
                formatter.write_str("coded claim is not bound to the admitted request")
            }
            Self::ReleaseAuthorizationMismatch { expected, actual } => {
                write!(
                    formatter,
                    "coded release authorization is bound to operation {actual:?}, not {expected:?}"
                )
            }
            Self::ReleaseAuthorityDomainMismatch(operation) => {
                write!(
                    formatter,
                    "coded release authorization is foreign to operation {operation:?}"
                )
            }

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
            Self::StateRevisionExhausted => {
                formatter.write_str("coded authority state revision is exhausted")
            }
        }
    }
}

impl std::error::Error for CodedAuthorityError {}

/// dwv:req req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts
/// This owner tracks active claims only. Operation-slot lifetime and token reuse
/// are enforced by the service boundary before admission; this lower primitive
/// does not retain released-generation tombstones.
///
/// Raw lifecycle and slot prerequisites cannot invoke release:
///
/// ```compile_fail
/// use dwv_lifecycle_authority::ReleaseAuthorization;
/// use dwv_recovery::CodedRangeAuthority;
/// use dwv_store::OperationReleasePermit;
///
/// fn release(
///     authority: &mut CodedRangeAuthority,
///     authorization: &ReleaseAuthorization,
///     permit: OperationReleasePermit,
/// ) {
///     authority.release(authorization, permit);
/// }
/// ```
pub struct CodedRangeAuthority {
    admission_authority: CodedAdmissionAuthorityId,
    geometry_authority: CodedGeometryAuthorityId,
    lifecycle_authority: LifecycleAuthorityVerifier,
    claims: BTreeMap<OperationSlotToken, CodedClaim>,
    admission_sequences: BTreeMap<OperationSlotToken, u64>,
    effect_possible: BTreeSet<OperationSlotToken>,
    admission_high_water: u64,
    state_revision: u64,
}

impl CodedRangeAuthority {
    pub fn new(geometry: &CodedGeometryOwner) -> Self {
        let (_, lifecycle_authority) = LifecycleAuthorityOwner::new();
        Self::new_with_authorities(
            lifecycle_authority,
            CodedAdmissionAuthorityId::new(),
            geometry.authority(),
        )
    }

    #[cfg(test)]
    fn new_for_test() -> Self {
        let (_, lifecycle_authority) = LifecycleAuthorityOwner::new();
        Self::new_with_authorities(
            lifecycle_authority,
            CodedAdmissionAuthorityId::new(),
            CodedGeometryAuthorityId::for_test(),
        )
    }

    pub(crate) fn new_with_authorities(
        lifecycle_authority: LifecycleAuthorityVerifier,
        admission_authority: CodedAdmissionAuthorityId,
        geometry_authority: CodedGeometryAuthorityId,
    ) -> Self {
        Self {
            geometry_authority,
            admission_authority,
            lifecycle_authority,
            claims: BTreeMap::new(),
            admission_sequences: BTreeMap::new(),
            effect_possible: BTreeSet::new(),
            admission_high_water: 0,
            state_revision: 0,
        }
    }
    pub(crate) fn fork_candidate(&self) -> Self {
        Self {
            admission_authority: self.admission_authority,
            geometry_authority: self.geometry_authority,
            lifecycle_authority: self.lifecycle_authority.clone(),
            claims: self.claims.clone(),
            admission_sequences: self.admission_sequences.clone(),
            effect_possible: self.effect_possible.clone(),
            admission_high_water: self.admission_high_water,
            state_revision: self.state_revision,
        }
    }

    pub(crate) const fn state_revision(&self) -> u64 {
        self.state_revision
    }

    pub(crate) const fn belongs_to_same_owner(&self, other: &Self) -> bool {
        self.admission_authority.0 == other.admission_authority.0
            && self.geometry_authority.0 == other.geometry_authority.0
    }

    fn next_state_revision(&self) -> Result<u64, CodedAuthorityError> {
        self.state_revision
            .checked_add(1)
            .ok_or(CodedAuthorityError::StateRevisionExhausted)
    }

    /// Admit one complete claim before any dependent effect.
    pub fn admit(
        &mut self,
        operation: OperationSlotToken,
        request: BlockRequest,
        input: CodedClaimInput,
    ) -> Result<CodedAdmissionOutcome, CodedAuthorityError> {
        if input.binding.geometry_authority != self.geometry_authority {
            return Err(CodedAuthorityError::ForeignGeometryAuthority);
        }
        if self.claims.contains_key(&operation) {
            return Err(CodedAuthorityError::OperationAlreadyAdmitted(operation));
        }
        if input.binding.operation != operation {
            return Err(CodedAuthorityError::ClaimOperationMismatch {
                expected: operation,
                actual: input.binding.operation,
            });
        }
        if input.binding.request != request {
            return Err(CodedAuthorityError::ClaimRequestMismatch);
        }
        if input.units.is_empty() {
            return Err(CodedAuthorityError::EmptyClaim);
        }

        let claim = CodedClaim {
            binding: input.binding,
            units: input.units,
        };
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
        let state_revision = self.next_state_revision()?;
        self.admission_high_water = admission_sequence;
        self.state_revision = state_revision;
        self.admission_sequences
            .insert(operation, admission_sequence);
        self.claims.insert(operation, claim.clone());
        Ok(CodedAdmissionOutcome::Admitted(CodedAdmission {
            authority: self.admission_authority,
            operation,
            claim,
            sequence: admission_sequence,
        }))
    }

    /// Issue permission for the dependent effect only after coded admission.
    pub(crate) fn permit_effect(
        &mut self,
        operation: OperationSlotToken,
    ) -> Result<CodedEffectPermit, CodedAuthorityError> {
        if !self.claims.contains_key(&operation) {
            return Err(CodedAuthorityError::OperationNotAdmitted(operation));
        }
        let state_revision = self.next_state_revision()?;
        if !self.effect_possible.insert(operation) {
            return Err(CodedAuthorityError::EffectAlreadyPermitted(operation));
        }
        self.state_revision = state_revision;
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
                authority: self.admission_authority,
                operation: *operation,
                claim: claim.clone(),
                sequence: self.admission_sequences[operation],
            })
            .collect()
    }
    pub fn admission(&self, operation: OperationSlotToken) -> Option<CodedAdmission> {
        let claim = self.claims.get(&operation)?.clone();
        Some(CodedAdmission {
            authority: self.admission_authority,
            operation,
            claim,
            sequence: *self.admission_sequences.get(&operation)?,
        })
    }

    /// Establish one atomic owner-produced history, cut, and active set.
    pub(crate) fn capture_boundary(&mut self) -> Option<CodedCaptureEstablishment> {
        let active_admissions = self.active_admissions();
        let through_sequence = self.admission_high_water;
        let capture_sequence = through_sequence.checked_add(1)?;
        let state_revision = self.state_revision.checked_add(1)?;
        self.admission_high_water = capture_sequence;
        self.state_revision = state_revision;
        Some(CodedCaptureEstablishment {
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

    /// Consume the exact generation-qualified lifecycle release capability and
    /// its lower-level slot-release prerequisite.
    pub(crate) fn release(
        &mut self,
        authorization: &ReleaseAuthorization,
        permit: OperationReleasePermit,
    ) -> Result<CodedClaimRelease, CodedAuthorityError> {
        let operation = permit.operation();
        if !self.lifecycle_authority.accepts_release(authorization) {
            return Err(CodedAuthorityError::ReleaseAuthorityDomainMismatch(
                operation,
            ));
        }
        if authorization.operation() != operation {
            return Err(CodedAuthorityError::ReleaseAuthorizationMismatch {
                expected: operation,
                actual: authorization.operation(),
            });
        }
        self.release_operation(operation)
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
        let state_revision = self.next_state_revision()?;
        let admission_sequence = self.admission_sequences[&operation];
        self.claims.remove(&operation);
        self.effect_possible.remove(&operation);
        self.admission_sequences.remove(&operation);
        self.state_revision = state_revision;
        Ok(CodedClaimRelease {
            operation,
            frontier: CodedAuthorityFrontier { admission_sequence },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CodedCaptureCoordinator, CodedCaptureError, CodedCaptureId, RecoveryGeneration};
    use dwv_core::TopologyEpoch;

    fn token(index: u32, generation: u32) -> OperationSlotToken {
        OperationSlotToken::new(index, generation)
    }

    fn claim(
        operation: OperationSlotToken,
        units: impl IntoIterator<Item = u32>,
    ) -> CodedClaimInput {
        CodedClaimInput::mapped_for_test(
            operation,
            units.into_iter().map(|unit| CodedUnitId(u64::from(unit))),
        )
    }

    fn admit(
        authority: &mut CodedRangeAuthority,
        operation: OperationSlotToken,
        input: CodedClaimInput,
    ) -> Result<CodedAdmissionOutcome, CodedAuthorityError> {
        let request = input.binding.request;
        authority.admit(operation, request, input)
    }

    #[test]
    fn coded_overlap_contends_but_disjoint_claims_coexist() {
        let mut authority = CodedRangeAuthority::new_for_test();
        assert!(matches!(
            admit(&mut authority, token(0, 1), claim(token(0, 1), [0]),).unwrap(),
            CodedAdmissionOutcome::Admitted(_)
        ));
        assert_eq!(
            admit(&mut authority, token(1, 1), claim(token(1, 1), [0, 1]),).unwrap(),
            CodedAdmissionOutcome::Contended
        );
        assert!(matches!(
            admit(&mut authority, token(2, 1), claim(token(2, 1), [1]),).unwrap(),
            CodedAdmissionOutcome::Admitted(_)
        ));
    }

    #[test]
    fn admitted_witness_binds_operation_claim_and_sequence() {
        let mut authority = CodedRangeAuthority::new_for_test();
        let operation = token(0, 1);
        let CodedAdmissionOutcome::Admitted(admission) =
            admit(&mut authority, operation, claim(operation, [3])).unwrap()
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
        let mut authority = CodedRangeAuthority::new_for_test();
        let operation = token(0, 1);
        assert!(matches!(
            admit(
                &mut authority,
                operation,
                CodedClaimInput::mapped_for_test(operation, [unit]),
            )
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
        let mut authority = CodedRangeAuthority::new_for_test();
        let operation = token(0, 1);
        assert_eq!(
            admit(
                &mut authority,
                operation,
                CodedClaimInput::mapped_for_test(operation, []),
            ),
            Err(CodedAuthorityError::EmptyClaim)
        );
    }

    #[test]
    fn duplicate_active_admission_is_rejected() {
        let mut authority = CodedRangeAuthority::new_for_test();
        let operation = token(0, 1);
        assert!(matches!(
            admit(&mut authority, operation, claim(operation, [0])).unwrap(),
            CodedAdmissionOutcome::Admitted(_)
        ));
        assert_eq!(
            admit(&mut authority, operation, claim(operation, [1])),
            Err(CodedAuthorityError::OperationAlreadyAdmitted(operation))
        );
    }

    #[test]
    fn release_is_exact_generation_and_removes_active_claim() {
        let mut authority = CodedRangeAuthority::new_for_test();
        let current = token(0, 2);
        let stale = token(0, 1);
        assert!(matches!(
            admit(&mut authority, current, claim(current, [0])).unwrap(),
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

    #[test]
    fn mutation_bound_claim_cannot_be_replayed_for_another_operation() {
        let mut authority = CodedRangeAuthority::new_for_test();
        let issued_for = token(0, 1);
        let replayed_as = token(1, 1);
        assert_eq!(
            admit(&mut authority, replayed_as, claim(issued_for, [0])),
            Err(CodedAuthorityError::ClaimOperationMismatch {
                expected: replayed_as,
                actual: issued_for,
            })
        );
        assert!(authority.active_claim(replayed_as).is_none());
    }

    #[test]
    fn mutation_bound_claim_cannot_be_replayed_for_another_request() {
        let mut authority = CodedRangeAuthority::new_for_test();
        let operation = token(0, 1);
        let input = claim(operation, [0]);
        let mut other_request = input.binding.request;
        other_request.request_id = dwv_core::RequestId(999);
        assert_eq!(
            authority.admit(operation, other_request, input),
            Err(CodedAuthorityError::ClaimRequestMismatch)
        );
        assert!(authority.active_claim(operation).is_none());
    }
    #[test]
    fn foreign_admission_cannot_enter_another_capture_authority_graph() {
        let operation = token(0, 1);
        let mut foreign_range = CodedRangeAuthority::new_for_test();
        let mut input = claim(operation, [0]);
        input.binding.invalidation_target =
            InvalidationTarget::new(vec![crate::RegionId(7)], vec![crate::IntegrityExtentId(9)]);
        let CodedAdmissionOutcome::Admitted(admission) =
            admit(&mut foreign_range, operation, input).unwrap()
        else {
            panic!("foreign claim unexpectedly contended");
        };

        let (_, lifecycle_authority) = LifecycleAuthorityOwner::new();
        let captures = CodedCaptureCoordinator::new_with_authorities(
            lifecycle_authority,
            CodedAdmissionAuthorityId::new(),
        );
        assert!(matches!(
            captures.prepare_admission_observation(&admission, RecoveryGeneration::ZERO),
            Err(CodedCaptureError::ForeignAdmissionAuthority)
        ));
        assert!(matches!(
            captures.prepare_later_cut(
                CodedCaptureId(1),
                &admission,
                TopologyEpoch(1),
                RecoveryGeneration::ZERO,
            ),
            Err(CodedCaptureError::ForeignAdmissionAuthority)
        ));
    }
}
