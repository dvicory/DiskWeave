//! Deterministic, portable identity evidence assessment.
//!
//! Discovery adapters can translate hardware, bridge, filesystem, or file
//! identity into these values.  The assessor never opens a path, probes a
//! device, consults a database, or treats enumeration order as identity.

use dwv_core::{
    ArrayId, AssignmentGeneration, AssignmentInstanceId, CodingPosition, EvidenceAction,
    EvidenceConfidence, MemberRole, SemanticEvidenceReport, SlotId, TopologyEpoch,
};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum IdentitySourceKind {
    StableDeviceId,
    Serial,
    FilesystemId,
    WorldWideName,
    FileId,
    Capacity,
    Geometry,
    OperatorAttestation,
    Path,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentitySourceSet {
    pub stable_device_id: bool,
    pub serial: bool,
    pub filesystem_id: bool,
    pub world_wide_name: bool,
    pub file_id: bool,
    pub path: bool,
}

impl IdentitySourceSet {
    pub const fn none() -> Self {
        Self {
            stable_device_id: false,
            serial: false,
            filesystem_id: false,
            world_wide_name: false,
            file_id: false,
            path: false,
        }
    }

    pub const fn has_stable_source(self) -> bool {
        self.stable_device_id || self.serial || self.world_wide_name || self.file_id
    }
}

/// Legacy observation shape retained for existing store adapters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityObservation {
    pub source: IdentitySourceKind,
    pub fingerprint: [u8; 16],
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CandidateId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityProvenance {
    Hardware,
    Bridge,
    Filesystem,
    File,
    Operator,
    Synthetic,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum IdentityStability {
    Stable,
    Session,
    Ephemeral,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum IdentityConfidence {
    None,
    Low,
    Medium,
    High,
    Attested,
}

impl IdentityConfidence {
    const fn as_core(self) -> EvidenceConfidence {
        match self {
            Self::None => EvidenceConfidence::None,
            Self::Low => EvidenceConfidence::Low,
            Self::Medium => EvidenceConfidence::Medium,
            Self::High => EvidenceConfidence::High,
            Self::Attested => EvidenceConfidence::Attested,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityEvidence {
    pub source: IdentitySourceKind,
    pub fingerprint: [u8; 16],
    pub provenance: IdentityProvenance,
    pub stability: IdentityStability,
    pub confidence: IdentityConfidence,
}

impl IdentityEvidence {
    pub const fn new(
        source: IdentitySourceKind,
        fingerprint: [u8; 16],
        provenance: IdentityProvenance,
        stability: IdentityStability,
        confidence: IdentityConfidence,
    ) -> Self {
        Self {
            source,
            fingerprint,
            provenance,
            stability,
            confidence,
        }
    }

    pub const fn stable(
        source: IdentitySourceKind,
        fingerprint: [u8; 16],
        provenance: IdentityProvenance,
    ) -> Self {
        Self::new(
            source,
            fingerprint,
            provenance,
            IdentityStability::Stable,
            IdentityConfidence::High,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateGeometry {
    pub capacity: Option<u64>,
    pub protected_length: Option<u64>,
    pub logical_block_size: Option<u32>,
}

impl CandidateGeometry {
    pub const fn unknown() -> Self {
        Self {
            capacity: None,
            protected_length: None,
            logical_block_size: None,
        }
    }

    pub const fn new(capacity: u64, protected_length: u64, logical_block_size: u32) -> Self {
        Self {
            capacity: Some(capacity),
            protected_length: Some(protected_length),
            logical_block_size: Some(logical_block_size),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityCandidate {
    pub candidate_id: CandidateId,
    pub claimed_array: Option<ArrayId>,
    pub claimed_slot: Option<SlotId>,
    pub claimed_role: Option<MemberRole>,
    pub claimed_coding_position: Option<CodingPosition>,
    pub assignment_instance: Option<AssignmentInstanceId>,
    pub assignment_generation: Option<AssignmentGeneration>,
    pub geometry: CandidateGeometry,
    pub observations: Vec<IdentityEvidence>,
    pub operator_attested: bool,
}

impl IdentityCandidate {
    pub fn new(candidate_id: CandidateId, observations: Vec<IdentityEvidence>) -> Self {
        Self {
            candidate_id,
            claimed_array: None,
            claimed_slot: None,
            claimed_role: None,
            claimed_coding_position: None,
            assignment_instance: None,
            assignment_generation: None,
            geometry: CandidateGeometry::unknown(),
            observations,
            operator_attested: false,
        }
    }

    pub fn with_slot(mut self, slot: SlotId) -> Self {
        self.claimed_slot = Some(slot);
        self
    }

    pub fn with_geometry(mut self, geometry: CandidateGeometry) -> Self {
        self.geometry = geometry;
        self
    }

    pub fn attested(mut self) -> Self {
        self.operator_attested = true;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityProfile {
    pub array_id: ArrayId,
    pub topology_epoch: TopologyEpoch,
    pub slot_id: SlotId,
    pub role: MemberRole,
    pub coding_position: CodingPosition,
    pub assignment_instance: AssignmentInstanceId,
    pub assignment_generation: AssignmentGeneration,
    pub geometry: CandidateGeometry,
    pub observations: Vec<IdentityEvidence>,
}

impl IdentityProfile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        array_id: ArrayId,
        topology_epoch: TopologyEpoch,
        slot_id: SlotId,
        role: MemberRole,
        coding_position: CodingPosition,
        assignment_instance: AssignmentInstanceId,
        assignment_generation: AssignmentGeneration,
        geometry: CandidateGeometry,
        observations: Vec<IdentityEvidence>,
    ) -> Self {
        Self {
            array_id,
            topology_epoch,
            slot_id,
            role,
            coding_position,
            assignment_instance,
            assignment_generation,
            geometry,
            observations,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityAssessment {
    Confirmed,
    Match,
    Changed,
    Clone,
    Ambiguous,
    Conflicting,
    InsufficientEvidence,
    NewDevice,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityComparison {
    Unchanged,
    Changed,
    Ambiguous,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssemblyBlockReason {
    NoCandidate,
    MultipleCandidates,
    InsufficientEvidence,
    CloneConflict,
    ConflictingAssignment,
    GeometryChanged,
    RoleOrPositionMismatch,
    NewUnassignedDevice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssemblyDecision {
    Writable {
        candidate: CandidateId,
        confidence: IdentityConfidence,
    },
    PrepareReplacement {
        candidate: CandidateId,
        confidence: IdentityConfidence,
    },
    ReadOnly {
        reason: AssemblyBlockReason,
    },
    Blocked {
        reason: AssemblyBlockReason,
    },
}

impl AssemblyDecision {
    pub const fn is_writable(self) -> bool {
        matches!(self, Self::Writable { .. })
    }

    pub const fn is_read_only(self) -> bool {
        matches!(self, Self::ReadOnly { .. })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateAssessment {
    pub candidate: CandidateId,
    pub assessment: IdentityAssessment,
    pub confidence: IdentityConfidence,
    pub geometry_compatible: bool,
    pub stable_observations: usize,
    pub missing_stable_observations: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityResolution {
    pub assessment: IdentityAssessment,
    pub assembly: AssemblyDecision,
    pub candidates: Vec<CandidateAssessment>,
    pub report: SemanticEvidenceReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityObservationSet {
    pub observations: Vec<IdentityObservation>,
    pub assessment: IdentityAssessment,
}

impl IdentityObservationSet {
    pub fn new(mut observations: Vec<IdentityObservation>, assessment: IdentityAssessment) -> Self {
        observations.sort_by_key(|observation| (observation.source, observation.fingerprint));
        Self {
            observations,
            assessment,
        }
    }

    pub fn compare(&self, observed: &Self) -> IdentityComparison {
        if !matches!(
            self.assessment,
            IdentityAssessment::Confirmed | IdentityAssessment::Match
        ) || !matches!(
            observed.assessment,
            IdentityAssessment::Confirmed | IdentityAssessment::Match
        ) {
            return IdentityComparison::Ambiguous;
        }
        if self.observations == observed.observations {
            IdentityComparison::Unchanged
        } else {
            IdentityComparison::Changed
        }
    }
}

pub struct IdentityAssessor;

impl IdentityAssessor {
    pub fn assess(
        profile: &IdentityProfile,
        candidates: &[IdentityCandidate],
    ) -> IdentityResolution {
        assess_identity(profile, candidates)
    }
}

/// dwv:req req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations
pub fn assess_identity(
    profile: &IdentityProfile,
    candidates: &[IdentityCandidate],
) -> IdentityResolution {
    // A complete supplied candidate set is assessed completely. Discovery
    // resource policy belongs upstream in enumeration, where incompleteness
    // is explicit rather than silent.
    let mut ordered: Vec<&IdentityCandidate> = candidates.iter().collect();
    ordered.sort_by_key(|candidate| candidate.candidate_id);
    let mut assessments = Vec::with_capacity(ordered.len());

    for candidate in ordered.iter() {
        assessments.push(assess_candidate(profile, candidate));
    }

    let matches: Vec<_> = assessments
        .iter()
        .filter(|assessment| {
            matches!(
                assessment.assessment,
                IdentityAssessment::Match | IdentityAssessment::Confirmed
            )
        })
        .collect();
    let has_clone = assessments
        .iter()
        .any(|assessment| assessment.assessment == IdentityAssessment::Clone);
    let has_conflict = assessments
        .iter()
        .any(|assessment| assessment.assessment == IdentityAssessment::Conflicting);
    let has_changed = assessments
        .iter()
        .any(|assessment| assessment.assessment == IdentityAssessment::Changed);

    let (assessment, assembly, action, confidence) = if matches.len() > 1 || has_clone {
        (
            IdentityAssessment::Clone,
            AssemblyDecision::Blocked {
                reason: AssemblyBlockReason::CloneConflict,
            },
            EvidenceAction::Blocked,
            IdentityConfidence::None,
        )
    } else if has_conflict {
        (
            IdentityAssessment::Conflicting,
            AssemblyDecision::Blocked {
                reason: AssemblyBlockReason::ConflictingAssignment,
            },
            EvidenceAction::Blocked,
            IdentityConfidence::None,
        )
    } else if let Some(found) = matches.first() {
        if has_changed || !found.geometry_compatible {
            (
                IdentityAssessment::Changed,
                AssemblyDecision::PrepareReplacement {
                    candidate: found.candidate,
                    confidence: found.confidence,
                },
                EvidenceAction::ReadOnlyInspection,
                found.confidence,
            )
        } else {
            (
                IdentityAssessment::Match,
                AssemblyDecision::Writable {
                    candidate: found.candidate,
                    confidence: found.confidence,
                },
                EvidenceAction::PrepareTopology,
                found.confidence,
            )
        }
    } else if let Some(changed) = assessments
        .iter()
        .find(|assessment| assessment.assessment == IdentityAssessment::Changed)
    {
        (
            IdentityAssessment::Changed,
            AssemblyDecision::PrepareReplacement {
                candidate: changed.candidate,
                confidence: changed.confidence,
            },
            EvidenceAction::ReadOnlyInspection,
            changed.confidence,
        )
    } else if assessments
        .iter()
        .any(|assessment| assessment.assessment == IdentityAssessment::InsufficientEvidence)
    {
        (
            IdentityAssessment::InsufficientEvidence,
            AssemblyDecision::ReadOnly {
                reason: AssemblyBlockReason::InsufficientEvidence,
            },
            EvidenceAction::ReadOnlyInspection,
            IdentityConfidence::None,
        )
    } else {
        (
            IdentityAssessment::NewDevice,
            AssemblyDecision::ReadOnly {
                reason: AssemblyBlockReason::NewUnassignedDevice,
            },
            EvidenceAction::ReadOnlyInspection,
            IdentityConfidence::None,
        )
    };

    let stable_sources = assessments
        .iter()
        .map(|assessment| assessment.stable_observations)
        .sum::<usize>();
    let missing_sources = assessments
        .iter()
        .map(|assessment| assessment.missing_stable_observations)
        .sum::<usize>();
    let report = SemanticEvidenceReport::new(
        profile.topology_epoch,
        candidates.len(),
        profile.observations.len()
            + candidates
                .iter()
                .map(|candidate| candidate.observations.len())
                .sum::<usize>(),
        stable_sources,
        usize::from(has_clone) + usize::from(has_conflict),
        confidence.as_core(),
        action,
    );
    let _ = missing_sources;

    IdentityResolution {
        assessment,
        assembly,
        candidates: assessments,
        report,
    }
}

fn assess_candidate(
    profile: &IdentityProfile,
    candidate: &IdentityCandidate,
) -> CandidateAssessment {
    let role_position_matches = candidate
        .claimed_array
        .is_none_or(|array| array == profile.array_id)
        && candidate
            .claimed_slot
            .is_none_or(|slot| slot == profile.slot_id)
        && candidate
            .claimed_role
            .is_none_or(|role| role == profile.role)
        && candidate
            .claimed_coding_position
            .is_none_or(|position| position == profile.coding_position);
    if !role_position_matches {
        return CandidateAssessment {
            candidate: candidate.candidate_id,
            assessment: IdentityAssessment::Conflicting,
            confidence: IdentityConfidence::None,
            geometry_compatible: false,
            stable_observations: 0,
            missing_stable_observations: 1,
        };
    }

    let mut stable_matches = 0usize;
    let mut stable_conflicts = 0usize;
    let mut filesystem_matches = 0usize;
    let mut independent_difference = false;
    for expected in &profile.observations {
        if !is_stable(expected) {
            continue;
        }
        match candidate
            .observations
            .iter()
            .find(|observed| observed.source == expected.source)
        {
            Some(observed) if observed.fingerprint == expected.fingerprint => {
                stable_matches = stable_matches.saturating_add(1);
                if expected.source == IdentitySourceKind::FilesystemId {
                    filesystem_matches = filesystem_matches.saturating_add(1);
                }
            }
            Some(observed) => {
                stable_conflicts = stable_conflicts.saturating_add(1);
                if expected.source != IdentitySourceKind::FilesystemId
                    || observed.source != IdentitySourceKind::FilesystemId
                {
                    independent_difference = true;
                }
            }
            None => {}
        }
    }
    let expected_stable = profile
        .observations
        .iter()
        .filter(|observation| is_stable(observation))
        .count();
    let missing = expected_stable
        .saturating_sub(stable_matches)
        .saturating_sub(stable_conflicts);
    let geometry_compatible = geometry_matches(profile.geometry, candidate.geometry);
    let assignment_changed = candidate
        .assignment_instance
        .is_some_and(|instance| instance != profile.assignment_instance);
    let has_attested_bridge_evidence = candidate.operator_attested
        && candidate
            .observations
            .iter()
            .any(|observation| observation.source == IdentitySourceKind::FileId)
        && geometry_compatible;

    let (assessment, confidence) =
        if stable_conflicts > 0 && (filesystem_matches > 0 || independent_difference) {
            (IdentityAssessment::Clone, IdentityConfidence::None)
        } else if stable_matches > 0 && (assignment_changed || !geometry_compatible) {
            (IdentityAssessment::Changed, IdentityConfidence::High)
        } else if stable_matches == expected_stable && expected_stable > 0 {
            (
                IdentityAssessment::Match,
                if stable_matches > 1 {
                    IdentityConfidence::High
                } else {
                    IdentityConfidence::Medium
                },
            )
        } else if has_attested_bridge_evidence {
            (IdentityAssessment::Match, IdentityConfidence::Attested)
        } else if candidate
            .observations
            .iter()
            .all(|observation| observation.source == IdentitySourceKind::Path)
            && !candidate.observations.is_empty()
        {
            (
                IdentityAssessment::InsufficientEvidence,
                IdentityConfidence::None,
            )
        } else if stable_matches == 0 && stable_conflicts == 0 {
            (IdentityAssessment::NewDevice, IdentityConfidence::Low)
        } else {
            (
                IdentityAssessment::InsufficientEvidence,
                IdentityConfidence::Low,
            )
        };

    CandidateAssessment {
        candidate: candidate.candidate_id,
        assessment,
        confidence,
        geometry_compatible,
        stable_observations: stable_matches,
        missing_stable_observations: missing,
    }
}

fn is_stable(observation: &IdentityEvidence) -> bool {
    matches!(observation.stability, IdentityStability::Stable)
        && !matches!(
            observation.source,
            IdentitySourceKind::Path | IdentitySourceKind::Capacity | IdentitySourceKind::Geometry
        )
}

fn geometry_matches(expected: CandidateGeometry, observed: CandidateGeometry) -> bool {
    expected
        .capacity
        .is_none_or(|value| observed.capacity == Some(value))
        && expected
            .protected_length
            .is_none_or(|value| observed.protected_length == Some(value))
        && expected
            .logical_block_size
            .is_none_or(|value| observed.logical_block_size == Some(value))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityEvidenceReport {
    pub version: u16,
    pub topology_epoch: TopologyEpoch,
    pub candidate_count: u16,
    pub observation_count: u16,
    pub assessments: Vec<CandidateAssessment>,
    pub assessment: IdentityAssessment,
    pub assembly: AssemblyDecision,
}

impl IdentityResolution {
    pub fn evidence_report(&self) -> IdentityEvidenceReport {
        IdentityEvidenceReport {
            version: self.report.version(),
            topology_epoch: self.report.topology_epoch(),
            candidate_count: self.report.candidate_count(),
            observation_count: self.report.observation_count(),
            assessments: self.candidates.clone(),
            assessment: self.assessment,
            assembly: self.assembly,
        }
    }
}

pub mod fixtures {
    use super::*;

    fn profile() -> IdentityProfile {
        IdentityProfile::new(
            ArrayId([1; 16]),
            TopologyEpoch(4),
            SlotId([2; 16]),
            MemberRole::Data,
            CodingPosition(0),
            AssignmentInstanceId([3; 16]),
            AssignmentGeneration(1),
            CandidateGeometry::new(4096, 4096, 512),
            vec![IdentityEvidence::stable(
                IdentitySourceKind::Serial,
                [7; 16],
                IdentityProvenance::Hardware,
            )],
        )
    }

    pub fn confident_match() -> IdentityResolution {
        let candidate = IdentityCandidate::new(
            CandidateId(1),
            vec![IdentityEvidence::stable(
                IdentitySourceKind::Serial,
                [7; 16],
                IdentityProvenance::Hardware,
            )],
        )
        .with_geometry(CandidateGeometry::new(4096, 4096, 512));
        assess_identity(&profile(), &[candidate])
    }

    pub fn cloned_filesystem_identifier() -> IdentityResolution {
        let profile = IdentityProfile::new(
            ArrayId([1; 16]),
            TopologyEpoch(4),
            SlotId([2; 16]),
            MemberRole::Data,
            CodingPosition(0),
            AssignmentInstanceId([3; 16]),
            AssignmentGeneration(1),
            CandidateGeometry::new(4096, 4096, 512),
            vec![
                IdentityEvidence::stable(
                    IdentitySourceKind::FilesystemId,
                    [8; 16],
                    IdentityProvenance::Filesystem,
                ),
                IdentityEvidence::stable(
                    IdentitySourceKind::Serial,
                    [7; 16],
                    IdentityProvenance::Hardware,
                ),
            ],
        );
        let first = IdentityCandidate::new(
            CandidateId(1),
            vec![
                IdentityEvidence::stable(
                    IdentitySourceKind::FilesystemId,
                    [8; 16],
                    IdentityProvenance::Filesystem,
                ),
                IdentityEvidence::stable(
                    IdentitySourceKind::Serial,
                    [7; 16],
                    IdentityProvenance::Hardware,
                ),
            ],
        )
        .with_geometry(CandidateGeometry::new(4096, 4096, 512));
        let second = IdentityCandidate::new(
            CandidateId(2),
            vec![
                IdentityEvidence::stable(
                    IdentitySourceKind::FilesystemId,
                    [8; 16],
                    IdentityProvenance::Filesystem,
                ),
                IdentityEvidence::stable(
                    IdentitySourceKind::Serial,
                    [9; 16],
                    IdentityProvenance::Hardware,
                ),
            ],
        )
        .with_geometry(CandidateGeometry::new(4096, 4096, 512));
        assess_identity(&profile, &[second, first])
    }

    pub fn usb_bridge_attested() -> IdentityResolution {
        let mut expected = profile();
        expected.observations = vec![IdentityEvidence::stable(
            IdentitySourceKind::FileId,
            [4; 16],
            IdentityProvenance::File,
        )];
        let candidate = IdentityCandidate::new(
            CandidateId(3),
            vec![IdentityEvidence::stable(
                IdentitySourceKind::FileId,
                [4; 16],
                IdentityProvenance::Bridge,
            )],
        )
        .with_geometry(CandidateGeometry::new(4096, 4096, 512))
        .attested();
        assess_identity(&expected, &[candidate])
    }

    pub fn changed_geometry() -> IdentityResolution {
        let candidate = IdentityCandidate::new(
            CandidateId(4),
            vec![IdentityEvidence::stable(
                IdentitySourceKind::Serial,
                [7; 16],
                IdentityProvenance::Hardware,
            )],
        )
        .with_geometry(CandidateGeometry::new(8192, 4096, 512));
        assess_identity(&profile(), &[candidate])
    }

    pub fn replacement() -> IdentityResolution {
        changed_geometry()
    }

    pub fn file_identity() -> IdentityResolution {
        usb_bridge_attested()
    }

    pub fn ambiguous_candidates() -> IdentityResolution {
        cloned_filesystem_identifier()
    }

    pub fn reordered_discovery() -> IdentityResolution {
        confident_match()
    }
}

impl fmt::Display for IdentityAssessment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures_cover_fail_closed_identity_cases() {
        assert!(fixtures::confident_match().assembly.is_writable());
        assert!(
            !fixtures::cloned_filesystem_identifier()
                .assembly
                .is_writable()
        );
        assert_eq!(
            fixtures::usb_bridge_attested().assessment,
            IdentityAssessment::Match
        );
        assert_eq!(
            fixtures::changed_geometry().assessment,
            IdentityAssessment::Changed
        );
    }

    #[test]
    fn candidate_order_does_not_change_resolution() {
        let profile = IdentityProfile::new(
            ArrayId([1; 16]),
            TopologyEpoch(1),
            SlotId([2; 16]),
            MemberRole::Data,
            CodingPosition(0),
            AssignmentInstanceId([3; 16]),
            AssignmentGeneration(1),
            CandidateGeometry::new(4, 4, 1),
            vec![IdentityEvidence::stable(
                IdentitySourceKind::Serial,
                [1; 16],
                IdentityProvenance::Hardware,
            )],
        );
        let a = IdentityCandidate::new(
            CandidateId(2),
            vec![IdentityEvidence::stable(
                IdentitySourceKind::Serial,
                [1; 16],
                IdentityProvenance::Hardware,
            )],
        )
        .with_geometry(CandidateGeometry::new(4, 4, 1));
        let b = IdentityCandidate::new(
            CandidateId(1),
            vec![IdentityEvidence::stable(
                IdentitySourceKind::Serial,
                [1; 16],
                IdentityProvenance::Hardware,
            )],
        )
        .with_geometry(CandidateGeometry::new(4, 4, 1));
        let candidates = vec![a, b];
        let expected = assess_identity(&profile, &candidates);
        for shift in 0..candidates.len() {
            let mut reordered = candidates.clone();
            reordered.rotate_left(shift);
            assert_eq!(expected, assess_identity(&profile, &reordered));
            reordered.reverse();
            assert_eq!(expected, assess_identity(&profile, &reordered));
        }
    }

    #[test]
    fn path_only_identity_is_never_writable() {
        let mut profile = fixtures::confident_match();
        assert!(profile.assembly.is_writable());
        let expected = IdentityProfile::new(
            ArrayId([1; 16]),
            TopologyEpoch(1),
            SlotId([2; 16]),
            MemberRole::Data,
            CodingPosition(0),
            AssignmentInstanceId([3; 16]),
            AssignmentGeneration(1),
            CandidateGeometry::unknown(),
            vec![IdentityEvidence::new(
                IdentitySourceKind::Path,
                [1; 16],
                IdentityProvenance::Synthetic,
                IdentityStability::Session,
                IdentityConfidence::High,
            )],
        );
        let observed = IdentityCandidate::new(
            CandidateId(1),
            vec![IdentityEvidence::new(
                IdentitySourceKind::Path,
                [1; 16],
                IdentityProvenance::Synthetic,
                IdentityStability::Session,
                IdentityConfidence::High,
            )],
        );
        profile = assess_identity(&expected, &[observed]);
        assert!(!profile.assembly.is_writable());
        assert_eq!(profile.assessment, IdentityAssessment::InsufficientEvidence);
    }

    #[test]
    fn explicit_fixture_matrix_is_fail_closed() {
        assert_eq!(
            fixtures::replacement().assessment,
            IdentityAssessment::Changed
        );
        assert_eq!(
            fixtures::file_identity().assessment,
            IdentityAssessment::Match
        );
        assert_eq!(
            fixtures::ambiguous_candidates().assessment,
            IdentityAssessment::Clone
        );
        assert_eq!(
            fixtures::reordered_discovery().assessment,
            IdentityAssessment::Match
        );
    }

    fn serial_profile() -> IdentityProfile {
        IdentityProfile::new(
            ArrayId([1; 16]),
            TopologyEpoch(1),
            SlotId([2; 16]),
            MemberRole::Data,
            CodingPosition(0),
            AssignmentInstanceId([3; 16]),
            AssignmentGeneration(1),
            CandidateGeometry::new(4, 4, 1),
            vec![IdentityEvidence::stable(
                IdentitySourceKind::Serial,
                [1; 16],
                IdentityProvenance::Hardware,
            )],
        )
    }

    fn serial_match(id: u64) -> IdentityCandidate {
        IdentityCandidate::new(
            CandidateId(id),
            vec![IdentityEvidence::stable(
                IdentitySourceKind::Serial,
                [1; 16],
                IdentityProvenance::Hardware,
            )],
        )
        .with_geometry(CandidateGeometry::new(4, 4, 1))
    }

    // Regression: the assessor evaluates every supplied candidate, so a match
    // anywhere in discovery order resolves. Observation vectors are likewise
    // unbounded at this layer; discovery policy bounds them upstream.
    #[test]
    fn supplied_match_is_assessed_regardless_of_position() {
        let profile = serial_profile();
        // Control: a match in an early position resolves writable.
        let mut in_bound: Vec<IdentityCandidate> = (1..=31)
            .map(|id| IdentityCandidate::new(CandidateId(id), vec![]))
            .collect();
        in_bound.push(serial_match(32));
        assert!(assess_identity(&profile, &in_bound).assembly.is_writable());
        // Regression: the identical match at position 33 is assessed too.
        let mut past_bound: Vec<IdentityCandidate> = (1..=32)
            .map(|id| IdentityCandidate::new(CandidateId(id), vec![]))
            .collect();
        past_bound.push(serial_match(33));
        let resolution = assess_identity(&profile, &past_bound);
        assert_eq!(
            resolution.candidates.len(),
            past_bound.len(),
            "every supplied candidate is assessed"
        );
        // If any repair marks a candidate writable, it must be the assessed
        // match, not a filler from the prefix.
        if let AssemblyDecision::Writable { candidate, .. } = resolution.assembly {
            assert_eq!(
                candidate,
                CandidateId(33),
                "writability must track the assessed match"
            );
        }
        // Negative property: the outcome must stop being indistinguishable
        // from the no-match case.
        let silently_dropped = resolution.assessment == IdentityAssessment::NewDevice
            && matches!(resolution.assembly, AssemblyDecision::ReadOnly { .. });
        if silently_dropped {
            panic!("a supplied match must not resolve as an unassigned device");
        }
    }
}
