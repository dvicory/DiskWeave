//! Portable logical topology semantics.
//!
//! These values describe the recorded array, not the device enumeration that
//! happened to produce an observation.  The types intentionally contain no
//! paths, operating-system handles, persistence rows, or data-member bytes.

use std::fmt;

pub const MAX_TOPOLOGY_ASSIGNMENTS: usize = 256;
pub const MAX_EVIDENCE_SOURCES: u8 = 32;

macro_rules! byte_id {
    ($name:ident) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            serde::Deserialize,
            serde::Serialize,
        )]
        pub struct $name(pub [u8; 16]);

        impl $name {
            pub const fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(bytes)
            }

            pub const fn as_bytes(self) -> [u8; 16] {
                self.0
            }
        }
    };
}

byte_id!(ArrayId);
byte_id!(AssignmentInstanceId);

/// Compatibility name for callers that describe an assignment instance as an
/// assignment ID.
pub type AssignmentId = AssignmentInstanceId;

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct TopologyEpoch(pub u64);

impl TopologyEpoch {
    pub const fn next(self) -> Result<Self, EpochError> {
        match self.0.checked_add(1) {
            Some(next) => Ok(Self(next)),
            None => Err(EpochError::Exhausted),
        }
    }

    pub const fn is_after(self, previous: Self) -> bool {
        self.0 > previous.0
    }
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct SlotId(pub [u8; 16]);

impl SlotId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct CodingPosition(pub u16);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct AssignmentGeneration(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TopologyGeneration(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RoleId(pub u8);

impl RoleId {
    pub const DATA: Self = Self(0);
    pub const PARITY: Self = Self(1);
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub enum MemberRole {
    Data,
    Parity,
}

impl MemberRole {
    pub const fn id(self) -> RoleId {
        match self {
            Self::Data => RoleId::DATA,
            Self::Parity => RoleId::PARITY,
        }
    }
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct CodingProfile {
    data_slots: u16,
    parity_slots: u16,
}

impl CodingProfile {
    pub const fn new(data_slots: u16, parity_slots: u16) -> Result<Self, CodingProfileError> {
        if data_slots == 0 || parity_slots == 0 {
            return Err(CodingProfileError::EmptyRole);
        }
        if data_slots.checked_add(parity_slots).is_none() {
            return Err(CodingProfileError::TooManyPositions);
        }
        Ok(Self {
            data_slots,
            parity_slots,
        })
    }

    pub const fn data_slots(self) -> u16 {
        self.data_slots
    }

    pub const fn parity_slots(self) -> u16 {
        self.parity_slots
    }

    pub const fn total_slots(self) -> u16 {
        self.data_slots + self.parity_slots
    }

    pub const fn accepts(self, role: MemberRole, position: CodingPosition) -> bool {
        match role {
            MemberRole::Data => position.0 < self.data_slots,
            MemberRole::Parity => position.0 >= self.data_slots && position.0 < self.total_slots(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ProtectedGeometry {
    protected_length: u64,
    logical_block_size: u32,
    parity_length: u64,
}

impl ProtectedGeometry {
    pub fn new(protected_length: u64, logical_block_size: u32) -> Result<Self, GeometryError> {
        Self::with_parity_length(protected_length, logical_block_size, protected_length)
    }

    pub fn with_parity_length(
        protected_length: u64,
        logical_block_size: u32,
        parity_length: u64,
    ) -> Result<Self, GeometryError> {
        if protected_length == 0 || parity_length < protected_length {
            return Err(GeometryError::InvalidLength {
                protected_length,
                parity_length,
            });
        }
        if logical_block_size == 0
            || !protected_length.is_multiple_of(logical_block_size as u64)
            || !parity_length.is_multiple_of(logical_block_size as u64)
        {
            return Err(GeometryError::Unaligned {
                protected_length,
                logical_block_size,
                parity_length,
            });
        }
        Ok(Self {
            protected_length,
            logical_block_size,
            parity_length,
        })
    }

    pub const fn protected_length(self) -> u64 {
        self.protected_length
    }

    pub const fn logical_block_size(self) -> u32 {
        self.logical_block_size
    }

    pub const fn parity_length(self) -> u64 {
        self.parity_length
    }
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub enum EvidenceConfidence {
    None,
    Low,
    Medium,
    High,
    Attested,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AssignmentEvidence {
    observed_sources: u8,
    stable_sources: u8,
    conflict_count: u8,
    confidence: EvidenceConfidence,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EvidenceAction {
    NoAction,
    PrepareTopology,
    ReadOnlyInspection,
    Blocked,
}

/// A bounded, portable summary suitable for semantic recovery reports.
///
/// It deliberately omits raw fingerprints, paths, runtime resources, SQL
/// identifiers, and data-member metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SemanticEvidenceReport {
    version: u16,
    topology_epoch: TopologyEpoch,
    candidate_count: u16,
    observation_count: u16,
    stable_source_count: u16,
    conflict_count: u16,
    confidence: EvidenceConfidence,
    action: EvidenceAction,
}

impl SemanticEvidenceReport {
    pub fn new(
        topology_epoch: TopologyEpoch,
        candidate_count: usize,
        observation_count: usize,
        stable_source_count: usize,
        conflict_count: usize,
        confidence: EvidenceConfidence,
        action: EvidenceAction,
    ) -> Self {
        Self {
            version: 1,
            topology_epoch,
            candidate_count: candidate_count.min(u16::MAX as usize) as u16,
            observation_count: observation_count.min(u16::MAX as usize) as u16,
            stable_source_count: stable_source_count.min(u16::MAX as usize) as u16,
            conflict_count: conflict_count.min(u16::MAX as usize) as u16,
            confidence,
            action,
        }
    }

    pub const fn version(self) -> u16 {
        self.version
    }
    pub const fn topology_epoch(self) -> TopologyEpoch {
        self.topology_epoch
    }
    pub const fn candidate_count(self) -> u16 {
        self.candidate_count
    }
    pub const fn observation_count(self) -> u16 {
        self.observation_count
    }
    pub const fn stable_source_count(self) -> u16 {
        self.stable_source_count
    }
    pub const fn conflict_count(self) -> u16 {
        self.conflict_count
    }
    pub const fn confidence(self) -> EvidenceConfidence {
        self.confidence
    }
    pub const fn action(self) -> EvidenceAction {
        self.action
    }
}

impl AssignmentEvidence {
    pub fn new(
        observed_sources: u8,
        stable_sources: u8,
        conflict_count: u8,
        confidence: EvidenceConfidence,
    ) -> Self {
        Self {
            observed_sources: if observed_sources > MAX_EVIDENCE_SOURCES {
                MAX_EVIDENCE_SOURCES
            } else {
                observed_sources
            },
            stable_sources: if stable_sources > MAX_EVIDENCE_SOURCES {
                MAX_EVIDENCE_SOURCES
            } else {
                stable_sources
            },
            conflict_count,
            confidence,
        }
    }

    pub fn empty() -> Self {
        Self::new(0, 0, 0, EvidenceConfidence::None)
    }

    pub const fn observed_sources(self) -> u8 {
        self.observed_sources
    }

    pub const fn stable_sources(self) -> u8 {
        self.stable_sources
    }

    pub const fn conflict_count(self) -> u8 {
        self.conflict_count
    }

    pub const fn confidence(self) -> EvidenceConfidence {
        self.confidence
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TopologyAssignment {
    slot_id: SlotId,
    role: MemberRole,
    coding_position: CodingPosition,
    assignment_instance: AssignmentInstanceId,
    assignment_generation: AssignmentGeneration,
    evidence: AssignmentEvidence,
}

impl TopologyAssignment {
    pub fn new(
        slot_id: SlotId,
        role: MemberRole,
        coding_position: CodingPosition,
        assignment_instance: AssignmentInstanceId,
        assignment_generation: AssignmentGeneration,
    ) -> Self {
        Self {
            slot_id,
            role,
            coding_position,
            assignment_instance,
            assignment_generation,
            evidence: AssignmentEvidence::empty(),
        }
    }

    pub const fn with_evidence(mut self, evidence: AssignmentEvidence) -> Self {
        self.evidence = evidence;
        self
    }

    pub const fn slot_id(&self) -> SlotId {
        self.slot_id
    }

    pub const fn role(&self) -> MemberRole {
        self.role
    }

    pub const fn coding_position(&self) -> CodingPosition {
        self.coding_position
    }

    pub const fn assignment_instance(&self) -> AssignmentInstanceId {
        self.assignment_instance
    }

    pub const fn assignment_generation(&self) -> AssignmentGeneration {
        self.assignment_generation
    }

    pub const fn evidence(&self) -> AssignmentEvidence {
        self.evidence
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopologySnapshot {
    array_id: ArrayId,
    topology_epoch: TopologyEpoch,
    profile: CodingProfile,
    geometry: ProtectedGeometry,
    assignments: Vec<TopologyAssignment>,
}

impl TopologySnapshot {
    pub fn new(
        array_id: ArrayId,
        topology_epoch: TopologyEpoch,
        profile: CodingProfile,
        geometry: ProtectedGeometry,
        assignments: Vec<TopologyAssignment>,
    ) -> Result<Self, TopologyValidationError> {
        let snapshot = Self {
            array_id,
            topology_epoch,
            profile,
            geometry,
            assignments,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), TopologyValidationError> {
        if self.assignments.len() > MAX_TOPOLOGY_ASSIGNMENTS {
            return Err(TopologyValidationError::TooManyAssignments {
                count: self.assignments.len(),
            });
        }
        if self.assignments.len() != usize::from(self.profile.total_slots()) {
            return Err(TopologyValidationError::AssignmentCount {
                expected: usize::from(self.profile.total_slots()),
                actual: self.assignments.len(),
            });
        }

        for (index, assignment) in self.assignments.iter().enumerate() {
            if !self
                .profile
                .accepts(assignment.role, assignment.coding_position)
            {
                return Err(TopologyValidationError::RoleCodingMismatch {
                    role: assignment.role,
                    position: assignment.coding_position,
                });
            }
            for prior in &self.assignments[..index] {
                if prior.slot_id == assignment.slot_id {
                    return Err(TopologyValidationError::DuplicateSlot {
                        slot_id: assignment.slot_id,
                    });
                }
                if prior.coding_position == assignment.coding_position {
                    return Err(TopologyValidationError::DuplicateCodingPosition {
                        position: assignment.coding_position,
                    });
                }
                if prior.assignment_instance == assignment.assignment_instance {
                    return Err(TopologyValidationError::DuplicateAssignmentInstance {
                        assignment_instance: assignment.assignment_instance,
                    });
                }
            }
        }
        Ok(())
    }

    pub const fn array_id(&self) -> ArrayId {
        self.array_id
    }

    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch
    }

    pub const fn profile(&self) -> CodingProfile {
        self.profile
    }

    pub const fn geometry(&self) -> ProtectedGeometry {
        self.geometry
    }

    pub fn assignments(&self) -> &[TopologyAssignment] {
        &self.assignments
    }

    pub fn assignment_for_slot(&self, slot_id: SlotId) -> Option<&TopologyAssignment> {
        self.assignments
            .iter()
            .find(|assignment| assignment.slot_id == slot_id)
    }

    pub fn assignment_for_position(&self, position: CodingPosition) -> Option<&TopologyAssignment> {
        self.assignments
            .iter()
            .find(|assignment| assignment.coding_position == position)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionStage {
    Prepared,
    Verified,
    Committed,
    Published,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedTransition {
    base: TopologySnapshot,
    candidate: TopologySnapshot,
}

impl PreparedTransition {
    pub fn verify(self) -> Result<VerifiedTransition, TransitionError> {
        self.verify_with(|_| true)
    }

    pub fn verify_with(
        self,
        verification: impl FnOnce(&TopologySnapshot) -> bool,
    ) -> Result<VerifiedTransition, TransitionError> {
        self.candidate
            .validate()
            .map_err(TransitionError::InvalidCandidate)?;
        if !verification(&self.candidate) {
            return Err(TransitionError::VerificationFailed);
        }
        Ok(VerifiedTransition {
            base: self.base,
            candidate: self.candidate,
        })
    }

    pub const fn stage(&self) -> TransitionStage {
        TransitionStage::Prepared
    }

    pub const fn base(&self) -> &TopologySnapshot {
        &self.base
    }

    pub const fn candidate(&self) -> &TopologySnapshot {
        &self.candidate
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedTransition {
    base: TopologySnapshot,
    candidate: TopologySnapshot,
}

impl VerifiedTransition {
    pub const fn stage(&self) -> TransitionStage {
        TransitionStage::Verified
    }

    pub const fn candidate(&self) -> &TopologySnapshot {
        &self.candidate
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedTransition {
    base: TopologySnapshot,
    candidate: TopologySnapshot,
    generation: TopologyGeneration,
}

impl CommittedTransition {
    pub const fn stage(&self) -> TransitionStage {
        TransitionStage::Committed
    }

    pub const fn generation(&self) -> TopologyGeneration {
        self.generation
    }

    pub const fn candidate(&self) -> &TopologySnapshot {
        &self.candidate
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopologyAuthority {
    active: TopologySnapshot,
    next_generation: TopologyGeneration,
    committed: Option<CommittedTransition>,
}

impl TopologyAuthority {
    pub fn new(active: TopologySnapshot) -> Result<Self, TopologyValidationError> {
        active.validate()?;
        Ok(Self {
            active,
            next_generation: TopologyGeneration(1),
            committed: None,
        })
    }

    pub const fn active(&self) -> &TopologySnapshot {
        &self.active
    }

    pub const fn committed(&self) -> Option<&CommittedTransition> {
        self.committed.as_ref()
    }

    pub fn prepare(
        &self,
        candidate: TopologySnapshot,
    ) -> Result<PreparedTransition, TransitionError> {
        candidate
            .validate()
            .map_err(TransitionError::InvalidCandidate)?;
        if candidate.array_id != self.active.array_id {
            return Err(TransitionError::ArrayMismatch);
        }
        if !candidate
            .topology_epoch
            .is_after(self.active.topology_epoch)
        {
            return Err(TransitionError::EpochNotMonotonic {
                active: self.active.topology_epoch,
                candidate: candidate.topology_epoch,
            });
        }
        if candidate.profile != self.active.profile {
            return Err(TransitionError::ProfileChanged);
        }
        Ok(PreparedTransition {
            base: self.active.clone(),
            candidate,
        })
    }

    pub fn commit(
        &mut self,
        transition: VerifiedTransition,
    ) -> Result<CommittedTransition, TransitionError> {
        if self.committed.is_some() {
            return Err(TransitionError::CommitAlreadyPending);
        }
        if transition.base != self.active {
            return Err(TransitionError::StaleBase);
        }
        let committed = CommittedTransition {
            base: transition.base,
            candidate: transition.candidate,
            generation: self.next_generation,
        };
        self.next_generation = TopologyGeneration(
            self.next_generation
                .0
                .checked_add(1)
                .ok_or(TransitionError::GenerationExhausted)?,
        );
        self.committed = Some(committed.clone());
        Ok(committed)
    }

    pub fn publish(&mut self, transition: CommittedTransition) -> Result<(), TransitionError> {
        if self.committed.as_ref() != Some(&transition) {
            return Err(TransitionError::UnknownCommit);
        }
        self.active = transition.candidate;
        self.committed = None;
        Ok(())
    }

    /// Reconcile a durably recorded commit after publication was interrupted.
    pub fn reconcile_committed(&mut self) -> Result<&TopologySnapshot, TransitionError> {
        let transition = self
            .committed
            .take()
            .ok_or(TransitionError::NoCommitToReconcile)?;
        self.active = transition.candidate;
        Ok(&self.active)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EpochError {
    Exhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodingProfileError {
    EmptyRole,
    TooManyPositions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryError {
    InvalidLength {
        protected_length: u64,
        parity_length: u64,
    },
    Unaligned {
        protected_length: u64,
        logical_block_size: u32,
        parity_length: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopologyValidationError {
    TooManyAssignments {
        count: usize,
    },
    AssignmentCount {
        expected: usize,
        actual: usize,
    },
    DuplicateSlot {
        slot_id: SlotId,
    },
    DuplicateCodingPosition {
        position: CodingPosition,
    },
    DuplicateAssignmentInstance {
        assignment_instance: AssignmentInstanceId,
    },
    RoleCodingMismatch {
        role: MemberRole,
        position: CodingPosition,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionError {
    InvalidCandidate(TopologyValidationError),
    ArrayMismatch,
    EpochNotMonotonic {
        active: TopologyEpoch,
        candidate: TopologyEpoch,
    },
    ProfileChanged,
    VerificationFailed,
    StaleBase,
    CommitAlreadyPending,
    GenerationExhausted,
    UnknownCommit,
    NoCommitToReconcile,
}

impl fmt::Display for TopologyValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyAssignments { count } => {
                write!(formatter, "too many assignments: {count}")
            }
            Self::AssignmentCount { expected, actual } => {
                write!(formatter, "expected {expected} assignments, got {actual}")
            }
            Self::DuplicateSlot { .. } => write!(formatter, "duplicate logical slot"),
            Self::DuplicateCodingPosition { position } => {
                write!(formatter, "duplicate coding position {}", position.0)
            }
            Self::DuplicateAssignmentInstance { .. } => {
                write!(formatter, "duplicate assignment instance")
            }
            Self::RoleCodingMismatch { role, position } => {
                write!(
                    formatter,
                    "role {role:?} cannot use coding position {}",
                    position.0
                )
            }
        }
    }
}

impl std::error::Error for TopologyValidationError {}

impl fmt::Display for TransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCandidate(error) => write!(formatter, "invalid candidate: {error}"),
            Self::ArrayMismatch => write!(formatter, "candidate belongs to another array"),
            Self::EpochNotMonotonic { active, candidate } => write!(
                formatter,
                "candidate epoch {} is not after active epoch {}",
                candidate.0, active.0
            ),
            Self::ProfileChanged => write!(formatter, "coding profile changes are not staged here"),
            Self::VerificationFailed => write!(formatter, "candidate verification failed"),
            Self::StaleBase => write!(formatter, "transition was prepared from a stale topology"),
            Self::CommitAlreadyPending => write!(formatter, "another topology commit is pending"),
            Self::GenerationExhausted => write!(formatter, "topology generation exhausted"),
            Self::UnknownCommit => write!(formatter, "unknown topology commit"),
            Self::NoCommitToReconcile => write!(formatter, "no committed topology to reconcile"),
        }
    }
}

impl std::error::Error for TransitionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_geometry_accepts_only_valid_capacity_and_alignment() {
        let block_sizes = [1_u32, 2, 4, 512];
        for block_size in block_sizes {
            for protected_blocks in 0_u64..=4 {
                for parity_blocks in 0_u64..=5 {
                    let protected = protected_blocks * u64::from(block_size);
                    let parity = parity_blocks * u64::from(block_size);
                    let expected = protected != 0 && parity >= protected;
                    assert_eq!(
                        ProtectedGeometry::with_parity_length(protected, block_size, parity)
                            .is_ok(),
                        expected,
                        "protected={protected} block={block_size} parity={parity}"
                    );
                }
            }
        }
    }

    fn snapshot(epoch: u64) -> TopologySnapshot {
        let profile = CodingProfile::new(2, 1).unwrap();
        let geometry = ProtectedGeometry::new(4096, 512).unwrap();
        let assignments = [
            (MemberRole::Data, 0, 1),
            (MemberRole::Data, 1, 2),
            (MemberRole::Parity, 2, 3),
        ]
        .into_iter()
        .map(|(role, position, id)| {
            TopologyAssignment::new(
                SlotId([id; 16]),
                role,
                CodingPosition(position),
                AssignmentInstanceId([id + 10; 16]),
                AssignmentGeneration(1),
            )
        })
        .collect();
        TopologySnapshot::new(
            ArrayId([9; 16]),
            TopologyEpoch(epoch),
            profile,
            geometry,
            assignments,
        )
        .unwrap()
    }

    #[test]
    fn topology_validation_is_independent_of_discovery_order() {
        let first = snapshot(1);
        let mut reordered = first.assignments().to_vec();
        reordered.reverse();
        let second = TopologySnapshot::new(
            first.array_id(),
            first.topology_epoch(),
            first.profile(),
            first.geometry(),
            reordered,
        )
        .unwrap();
        assert_eq!(
            first.assignment_for_position(CodingPosition(1)),
            second.assignment_for_position(CodingPosition(1))
        );
    }
    #[test]
    fn seeded_topology_candidates_reject_duplicate_assignments() {
        for (line, seed_text) in include_str!("../../../verification/corpus/topology-seeds.txt")
            .lines()
            .enumerate()
        {
            let seed = u64::from_str_radix(seed_text.trim(), 16)
                .unwrap_or_else(|_| panic!("invalid topology seed on line {}", line + 1));
            let base = snapshot(1);
            let mut reordered = base.assignments().to_vec();
            let rotation = (seed as usize) % reordered.len();
            reordered.rotate_left(rotation);
            let candidate = TopologySnapshot::new(
                base.array_id(),
                TopologyEpoch(2),
                base.profile(),
                base.geometry(),
                reordered,
            )
            .unwrap();
            assert_eq!(candidate.assignments().len(), 3);

            let mut duplicate = candidate.assignments().to_vec();
            duplicate[0] = duplicate[1].clone();
            assert!(matches!(
                TopologySnapshot::new(
                    base.array_id(),
                    TopologyEpoch(2),
                    base.profile(),
                    base.geometry(),
                    duplicate,
                ),
                Err(TopologyValidationError::DuplicateSlot { .. })
            ));
        }
    }

    #[test]
    fn failed_verification_does_not_change_active_topology() {
        let old = snapshot(1);
        let authority = TopologyAuthority::new(old.clone()).unwrap();
        let candidate = snapshot(2);
        let prepared = authority.prepare(candidate).unwrap();
        assert_eq!(
            prepared.verify_with(|_| false),
            Err(TransitionError::VerificationFailed)
        );
        assert_eq!(authority.active(), &old);
        assert!(authority.committed().is_none());
    }

    #[test]
    fn durable_commit_waits_for_publication_and_can_reconcile() {
        let old = snapshot(1);
        let mut authority = TopologyAuthority::new(old).unwrap();
        let verified = authority.prepare(snapshot(2)).unwrap().verify().unwrap();
        let committed = authority.commit(verified).unwrap();
        assert_eq!(authority.active().topology_epoch(), TopologyEpoch(1));
        assert_eq!(committed.stage(), TransitionStage::Committed);
        assert_eq!(
            authority.reconcile_committed().unwrap().topology_epoch(),
            TopologyEpoch(2)
        );
    }
}

#[cfg(kani)]
mod kani_verification {
    use super::ProtectedGeometry;
    use crate::{ByteRange, RangeError};

    #[kani::proof]
    fn byte_range_constructor_matches_checked_add() {
        let offset: u64 = kani::any();
        let length: u64 = kani::any();
        let checked = offset.checked_add(length);
        let constructed = ByteRange::new(offset, length);

        match (checked, constructed) {
            (Some(expected_end), Ok(range)) => assert_eq!(range.end(), expected_end),
            (None, Err(RangeError::Overflow { .. })) => {}
            (Some(_), Err(_)) | (None, Ok(_)) => assert!(false),
        }
    }

    fn verify_geometry_for_block(
        logical_block_size: u32,
        protected_length: u64,
        parity_length: u64,
    ) {
        let block_size = u64::from(logical_block_size);
        let valid = protected_length > 0
            && parity_length >= protected_length
            && protected_length.is_multiple_of(block_size)
            && parity_length.is_multiple_of(block_size);
        let result = ProtectedGeometry::with_parity_length(
            protected_length,
            logical_block_size,
            parity_length,
        );

        kani::cover!(valid);
        kani::cover!(!valid);
        match (valid, result) {
            (true, Ok(geometry)) => {
                assert_eq!(geometry.protected_length(), protected_length);
                assert_eq!(geometry.logical_block_size(), logical_block_size);
                assert_eq!(geometry.parity_length(), parity_length);
            }
            (false, Err(_)) => {}
            (true, Err(_)) | (false, Ok(_)) => assert!(false),
        }
    }

    #[kani::proof]
    fn geometry_512_acceptance_is_exact_and_reachable() {
        let protected_length: u64 = kani::any();
        let parity_length: u64 = kani::any();
        verify_geometry_for_block(512, protected_length, parity_length);
    }

    #[kani::proof]
    fn geometry_4096_acceptance_is_exact_and_reachable() {
        let protected_length: u64 = kani::any();
        let parity_length: u64 = kani::any();
        verify_geometry_for_block(4096, protected_length, parity_length);
    }
}
