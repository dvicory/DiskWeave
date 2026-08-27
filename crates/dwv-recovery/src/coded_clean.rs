use crate::{
    ChecksumProfile, ChecksumProfileId, ChecksumSetGeneration, CodedAdmission,
    CodedAuthorityFrontier, CodedCaptureEstablishment, CodedClaimRelease, DurableRecoveryCommit,
    IntegrityExtentId, RecoveryCleanPermit, RecoveryCleanRefusalPermit, RecoveryGeneration,
    RegionId,
};
use dwv_core::{CodedUnitId, TopologyEpoch, TopologySnapshot};
use dwv_store::OperationSlotToken;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

mod operation_map_serde {
    use dwv_store::OperationSlotToken;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
    use std::collections::BTreeMap;

    pub fn serialize<S, V>(
        map: &BTreeMap<OperationSlotToken, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(
        deserializer: D,
    ) -> Result<BTreeMap<OperationSlotToken, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        let entries = Vec::<(OperationSlotToken, V)>::deserialize(deserializer)?;
        let mut map = BTreeMap::new();
        for (operation, value) in entries {
            if map.insert(operation, value).is_some() {
                return Err(D::Error::custom(
                    "duplicate coded capture operation identity",
                ));
            }
        }
        Ok(map)
    }
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct CodedCaptureId(pub u64);

/// Owner-supplied bounded position used to close a capture without retaining
/// an unbounded operation ledger.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct CodedCaptureFrontier {
    recovery_generation: RecoveryGeneration,
    admission_sequence: u64,
}

impl CodedCaptureFrontier {
    fn new(recovery_generation: RecoveryGeneration, admission_sequence: u64) -> Self {
        Self {
            recovery_generation,
            admission_sequence,
        }
    }

    pub const fn recovery_generation(self) -> RecoveryGeneration {
        self.recovery_generation
    }

    pub const fn admission_sequence(self) -> u64 {
        self.admission_sequence
    }
}

/// Owner-approved durable boundary for a mutation admitted after capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CodedCaptureCut {
    topology_epoch: TopologyEpoch,
    recovery_generation: RecoveryGeneration,
    frontier: CodedCaptureFrontier,
}

impl CodedCaptureCut {
    fn new(
        topology_epoch: TopologyEpoch,
        recovery_generation: RecoveryGeneration,
        frontier: CodedCaptureFrontier,
    ) -> Self {
        Self {
            topology_epoch,
            recovery_generation,
            frontier,
        }
    }

    pub const fn topology_epoch(self) -> TopologyEpoch {
        self.topology_epoch
    }

    pub const fn recovery_generation(self) -> RecoveryGeneration {
        self.recovery_generation
    }

    pub const fn frontier(self) -> CodedCaptureFrontier {
        self.frontier
    }
}
/// Owner-approved evidence that exact resolved membership may be forgotten
/// through a bounded frontier. Every current operation identity must be listed
/// in exactly one of `releasable_operations` or `retained_operations`, and
/// every releasable identity must carry an external
/// `ReleaseAllowed(operation-generation)` fact from the lifecycle owner. The
/// coordinator does not infer historicality from capture-local membership.
#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedCaptureRetentionSummary {
    pub capture: CodedCaptureId,
    pub topology_epoch: TopologyEpoch,
    pub frontier: CodedCaptureFrontier,
    pub releasable_operations: BTreeSet<OperationSlotToken>,
    pub retained_operations: BTreeSet<OperationSlotToken>,
    pub release_authorized_operations: BTreeSet<OperationSlotToken>,
}

#[cfg(test)]
impl CodedCaptureRetentionSummary {
    pub fn new(
        capture: CodedCaptureId,
        topology_epoch: TopologyEpoch,
        frontier: CodedCaptureFrontier,
        releasable_operations: impl IntoIterator<Item = OperationSlotToken>,
    ) -> Self {
        Self {
            capture,
            topology_epoch,
            frontier,
            releasable_operations: releasable_operations.into_iter().collect(),
            retained_operations: BTreeSet::new(),
            release_authorized_operations: BTreeSet::new(),
        }
    }

    pub fn with_retained_operations(
        mut self,
        retained_operations: impl IntoIterator<Item = OperationSlotToken>,
    ) -> Self {
        self.retained_operations = retained_operations.into_iter().collect();
        self
    }

    pub fn with_release_authorized_operations(
        mut self,
        operations: impl IntoIterator<Item = OperationSlotToken>,
    ) -> Self {
        self.release_authorized_operations = operations.into_iter().collect();
        self
    }
}

/// Owner-approved evidence that a resolved capture no longer needs exclusion.
/// For `CleanKnown`, `frontier` must be strictly newer than both the captured
/// and retained frontiers; that superseding dirty/recovery boundary ends the
/// capture's future-exclusion responsibility. A `Refused` capture has no
/// future-exclusion obligation.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodedCaptureRetirement {
    pub capture: CodedCaptureId,
    pub topology_epoch: TopologyEpoch,
    pub frontier: CodedCaptureFrontier,
}

#[cfg(test)]
impl CodedCaptureRetirement {
    pub const fn new(
        capture: CodedCaptureId,
        topology_epoch: TopologyEpoch,
        frontier: CodedCaptureFrontier,
    ) -> Self {
        Self {
            capture,
            topology_epoch,
            frontier,
        }
    }
}

/// Exact owner facts captured with a recovery-CLEAN closed-set boundary.
///
/// The coordinator records these facts; it does not derive topology,
/// persistence, dirty-state, checksum, or frontier meaning from private
/// operation state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedCaptureOwnerFacts {
    pub topology: TopologySnapshot,
    pub recovery_generation: RecoveryGeneration,
    pub checksum_profile: ChecksumProfileId,
    pub checksum_set_generation: ChecksumSetGeneration,
    pub scope: ValidatedCodedCaptureScope,
    pub dirty_regions: BTreeSet<RegionId>,
    pub checksum_extents: BTreeSet<IntegrityExtentId>,
    pub lower_frontier: CodedCaptureFrontier,
    pub capture_frontier: CodedCaptureFrontier,
}

impl CodedCaptureOwnerFacts {
    #[allow(clippy::too_many_arguments)]
    fn new(
        topology: TopologySnapshot,
        recovery_generation: RecoveryGeneration,
        checksum_profile: ChecksumProfileId,
        checksum_set_generation: ChecksumSetGeneration,
        scope: ValidatedCodedCaptureScope,
        dirty_regions: impl IntoIterator<Item = RegionId>,
        checksum_extents: impl IntoIterator<Item = IntegrityExtentId>,
        lower_frontier: CodedCaptureFrontier,
        capture_frontier: CodedCaptureFrontier,
    ) -> Self {
        Self {
            topology,
            recovery_generation,
            checksum_profile,
            checksum_set_generation,
            scope,
            dirty_regions: dirty_regions.into_iter().collect(),
            checksum_extents: checksum_extents.into_iter().collect(),
            lower_frontier,
            capture_frontier,
        }
    }

    /// Build the complete coded scope that can invalidate the selected dirty
    /// regions or checksum extents under the captured topology and profiles.
    pub fn selected_invalidation_scope(
        topology: &TopologySnapshot,
        dirty_regions: impl IntoIterator<Item = RegionId>,
        checksum_extents: impl IntoIterator<Item = IntegrityExtentId>,
        dirty_region_bytes: u64,
        checksum_profile: ChecksumProfile,
    ) -> Result<ValidatedCodedCaptureScope, CodedCaptureError> {
        if dirty_region_bytes == 0
            || checksum_profile.extent_size == 0
            || topology.geometry().logical_block_size() == 0
        {
            return Err(CodedCaptureError::InvalidScopeGeometry);
        }
        let dirty_regions = dirty_regions.into_iter().collect::<BTreeSet<_>>();
        let checksum_extents = checksum_extents.into_iter().collect::<BTreeSet<_>>();
        let protected_length = topology.geometry().protected_length();
        let logical_block = u64::from(topology.geometry().logical_block_size());
        let checksum_extents_per_member = protected_length.div_ceil(checksum_profile.extent_size);
        let coding_positions = topology
            .assignments()
            .iter()
            .map(|assignment| u32::from(assignment.coding_position().0))
            .collect::<BTreeSet<_>>();
        let mut units = BTreeSet::new();
        let mut add_range = |offset: u64, length: u64| -> Result<(), CodedCaptureError> {
            let end = offset
                .checked_add(length)
                .map(|end| end.min(protected_length))
                .ok_or(CodedCaptureError::ScopeArithmeticOverflow)?;
            if offset >= end {
                return Err(CodedCaptureError::ScopeIdentityOutOfRange);
            }
            let first = offset / logical_block;
            let last = (end - 1) / logical_block;
            units.extend((first..=last).map(CodedUnitId));
            Ok(())
        };
        for region in &dirty_regions {
            let coding_position = u32::try_from(region.0 >> 32)
                .map_err(|_| CodedCaptureError::ScopeIdentityOutOfRange)?;
            if !coding_positions.contains(&coding_position) {
                return Err(CodedCaptureError::ScopeIdentityOutOfRange);
            }
            let region_index = region.0 & u64::from(u32::MAX);
            let offset = region_index
                .checked_mul(dirty_region_bytes)
                .ok_or(CodedCaptureError::ScopeArithmeticOverflow)?;
            add_range(offset, dirty_region_bytes)?;
        }
        for extent in &checksum_extents {
            if checksum_extents_per_member == 0 {
                return Err(CodedCaptureError::InvalidScopeGeometry);
            }
            let coding_position = extent.0 / checksum_extents_per_member;
            let coding_position = u32::try_from(coding_position)
                .map_err(|_| CodedCaptureError::ScopeIdentityOutOfRange)?;
            if !coding_positions.contains(&coding_position) {
                return Err(CodedCaptureError::ScopeIdentityOutOfRange);
            }
            let extent_index = extent.0 % checksum_extents_per_member;
            let offset = extent_index
                .checked_mul(checksum_profile.extent_size)
                .ok_or(CodedCaptureError::ScopeArithmeticOverflow)?;
            add_range(offset, checksum_profile.extent_size)?;
        }
        if units.is_empty() {
            return Err(CodedCaptureError::EmptyScope);
        }
        Ok(ValidatedCodedCaptureScope::validated(
            units,
            topology.clone(),
            dirty_regions,
            checksum_extents,
            dirty_region_bytes,
            checksum_profile,
        ))
    }
}

/// Exact future-inclusive coded scope computed from owner geometry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedCodedCaptureScope {
    units: BTreeSet<CodedUnitId>,
    binding: Option<CaptureScopeBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CaptureScopeBinding {
    topology: TopologySnapshot,
    dirty_regions: BTreeSet<RegionId>,
    checksum_extents: BTreeSet<IntegrityExtentId>,
    dirty_region_bytes: u64,
    checksum_profile: ChecksumProfile,
}

impl ValidatedCodedCaptureScope {
    fn validated(
        units: BTreeSet<CodedUnitId>,
        topology: TopologySnapshot,
        dirty_regions: BTreeSet<RegionId>,
        checksum_extents: BTreeSet<IntegrityExtentId>,
        dirty_region_bytes: u64,
        checksum_profile: ChecksumProfile,
    ) -> Self {
        Self {
            units,
            binding: Some(CaptureScopeBinding {
                topology,
                dirty_regions,
                checksum_extents,
                dirty_region_bytes,
                checksum_profile,
            }),
        }
    }

    fn from_snapshot_units(units: impl IntoIterator<Item = CodedUnitId>) -> Self {
        Self {
            units: units.into_iter().collect(),
            binding: None,
        }
    }

    #[cfg(test)]
    fn complete(units: impl IntoIterator<Item = CodedUnitId>) -> Self {
        Self::from_snapshot_units(units)
    }

    pub fn units(&self) -> &BTreeSet<CodedUnitId> {
        &self.units
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CodedCapturePhase {
    Open,
    CleanCommitPending,
    CleanCommitUnknown,
    CleanKnown,
    Refused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CodedCaptureMembership {
    Included,
    Later,
    LaterDurableAfterClean,
    LaterDurableStalesClean,
    LaterRejected,
    LaterUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CodedCaptureDecision {
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedCleanCommitObservation {
    Durable,
    Rejected,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedCleanReconciliation {
    Durable,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedLaterCutObservation {
    DurableAfterClean,
    DurableStalesClean,
    Rejected,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedLaterCutReconciliation {
    DurableAfterClean,
    StalesClean,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CodedCaptureSnapshot {
    pub phase: CodedCapturePhase,
    pub capture: CodedCaptureId,
    pub topology: TopologySnapshot,
    pub recovery_generation: RecoveryGeneration,
    pub checksum_profile: ChecksumProfileId,
    pub checksum_set_generation: ChecksumSetGeneration,
    pub scope: BTreeSet<CodedUnitId>,
    pub scope_complete: bool,
    pub scope_validated: bool,
    pub lower_frontier_covered: bool,
    pub dirty_regions: BTreeSet<RegionId>,
    pub checksum_extents: BTreeSet<IntegrityExtentId>,
    pub lower_frontier: CodedCaptureFrontier,
    pub capture_frontier: CodedCaptureFrontier,
    pub retained_frontier: CodedCaptureFrontier,
    #[serde(with = "operation_map_serde")]
    pub membership: BTreeMap<OperationSlotToken, CodedCaptureMembership>,
    /// Exact post-capture cut retained for each unresolved `LaterUnknown`.
    #[serde(with = "operation_map_serde")]
    pub pending_later_cuts: BTreeMap<OperationSlotToken, CodedCaptureCut>,
    /// Exact owner-confirmed cuts retained until their operations are compacted.
    #[serde(default, with = "operation_map_serde")]
    pub resolved_later_cuts: BTreeMap<OperationSlotToken, CodedCaptureCut>,
    /// Newer durable dirty/recovery boundary that can supersede future exclusion.
    #[serde(default)]
    pub retirement_frontier: Option<CodedCaptureFrontier>,
    pub release_authorized_operations: BTreeSet<OperationSlotToken>,
    #[serde(default, with = "operation_map_serde")]
    pub release_frontiers: BTreeMap<OperationSlotToken, CodedAuthorityFrontier>,
    pub decision: Option<CodedCaptureDecision>,
}
impl CodedCaptureSnapshot {
    pub(crate) fn validates_internal_state(&self) -> bool {
        let decision_valid = match self.phase {
            CodedCapturePhase::Open => self.decision != Some(CodedCaptureDecision::Rejected),
            CodedCapturePhase::CleanCommitPending
            | CodedCapturePhase::CleanCommitUnknown
            | CodedCapturePhase::CleanKnown => {
                self.decision == Some(CodedCaptureDecision::Accepted)
            }
            CodedCapturePhase::Refused => true,
        };
        let pending = self
            .membership
            .iter()
            .filter_map(|(operation, membership)| {
                (*membership == CodedCaptureMembership::LaterUnknown).then_some(*operation)
            })
            .collect::<BTreeSet<_>>();
        let resolved = self
            .membership
            .iter()
            .filter_map(|(operation, membership)| {
                matches!(
                    membership,
                    CodedCaptureMembership::LaterDurableAfterClean
                        | CodedCaptureMembership::LaterDurableStalesClean
                )
                .then_some(*operation)
            })
            .collect::<BTreeSet<_>>();
        let cut_valid = |cut: &CodedCaptureCut| {
            cut.topology_epoch == self.topology.topology_epoch()
                && cut.recovery_generation == cut.frontier.recovery_generation
                && cut.frontier > self.capture_frontier
        };
        !self.scope.is_empty()
            && self.scope_complete
            && self.scope_validated
            && self.lower_frontier_covered
            && self.lower_frontier.recovery_generation == self.recovery_generation
            && self.capture_frontier.recovery_generation == self.recovery_generation
            && self.lower_frontier <= self.capture_frontier
            && self.retained_frontier >= self.lower_frontier
            && self
                .retirement_frontier
                .is_none_or(|frontier| frontier > self.capture_frontier)
            && decision_valid
            && pending == self.pending_later_cuts.keys().copied().collect()
            && resolved == self.resolved_later_cuts.keys().copied().collect()
            && self.pending_later_cuts.values().all(cut_valid)
            && self.resolved_later_cuts.values().all(cut_valid)
            && self
                .release_authorized_operations
                .is_subset(&self.membership.keys().copied().collect())
            && self.release_authorized_operations
                == self.release_frontiers.keys().copied().collect()
            && !(self.phase == CodedCapturePhase::CleanKnown
                && self.membership.values().any(|membership| {
                    *membership == CodedCaptureMembership::LaterDurableStalesClean
                }))
            && !(self.phase == CodedCapturePhase::Open
                && self.membership.values().any(|membership| {
                    *membership == CodedCaptureMembership::LaterDurableAfterClean
                }))
    }
}

/// Coordinator-issued exact current-to-proposed durable capture transition.
///
/// This capability is intentionally not deserializable: persisted snapshots
/// are data, while a transition must be issued by the live coordinator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedCaptureUpdate {
    expected: Option<CodedCaptureSnapshot>,
    proposed: CodedCaptureSnapshot,
}

impl CodedCaptureUpdate {
    pub(crate) fn expected(&self) -> Option<&CodedCaptureSnapshot> {
        self.expected.as_ref()
    }

    pub(crate) fn proposed(&self) -> &CodedCaptureSnapshot {
        &self.proposed
    }
}

/// Recovery-inspection-issued exact uncertain-transition resolution.
pub struct CodedCaptureReconciliationReceipt {
    expected_unknown: CodedCaptureSnapshot,
    resolved: CodedCaptureSnapshot,
}

impl CodedCaptureReconciliationReceipt {
    pub(crate) fn new(
        expected_unknown: CodedCaptureSnapshot,
        resolved: CodedCaptureSnapshot,
    ) -> Self {
        Self {
            expected_unknown,
            resolved,
        }
    }

    pub const fn capture(&self) -> CodedCaptureId {
        self.resolved.capture
    }
}

/// One exact owner-evaluated CLEAN transition awaiting its recovery commit.
#[must_use = "prepared CLEAN transitions must be committed or discarded"]
pub struct PreparedCodedCleanCommit {
    capture: CodedCaptureId,
    expected_generation: RecoveryGeneration,
    prior: CodedCaptureSnapshot,
    proposed: CodedCaptureSnapshot,
    proposed_record: CaptureRecord,
}

/// Coordinator-issued proof that one exact durable capture is retirable.
///
/// This capability is intentionally not deserializable or reconstructible from
/// a persisted snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedCaptureRemoval {
    expected: CodedCaptureSnapshot,
}

impl CodedCaptureRemoval {
    pub(crate) fn expected(&self) -> &CodedCaptureSnapshot {
        &self.expected
    }
}

impl PreparedCodedCleanCommit {
    pub fn snapshot(&self) -> &CodedCaptureSnapshot {
        &self.proposed
    }

    pub fn update(&self) -> CodedCaptureUpdate {
        CodedCaptureUpdate {
            expected: Some(self.prior.clone()),
            proposed: self.proposed.clone(),
        }
    }
}

/// One exact later-cut transition awaiting its write-recovery commit.
#[must_use = "prepared later cuts must be committed or discarded"]
pub struct PreparedCodedLaterCut {
    capture: CodedCaptureId,
    expected_generation: RecoveryGeneration,
    prior: CodedCaptureSnapshot,
    proposed: CodedCaptureSnapshot,
    proposed_record: CaptureRecord,
}

impl PreparedCodedLaterCut {
    pub fn snapshot(&self) -> &CodedCaptureSnapshot {
        &self.proposed
    }

    pub fn update(&self) -> CodedCaptureUpdate {
        CodedCaptureUpdate {
            expected: Some(self.prior.clone()),
            proposed: self.proposed.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodedCaptureError {
    EmptyScope,
    IncompleteScope,
    UnvalidatedScope,
    LowerFrontierUncovered,
    InvalidScopeGeometry,
    ScopeIdentityOutOfRange,
    ScopeArithmeticOverflow,
    PreparedTransitionStale(CodedCaptureId),
    DurableReceiptMismatch(CodedCaptureId),
    SnapshotInvalid(CodedCaptureId),
    CaptureFrontierGenerationMismatch(CodedCaptureId),
    RetentionSummaryInvalid(CodedCaptureId),
    RetentionFrontierStale(CodedCaptureId),
    CaptureOperationUnresolved {
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    },
    CaptureNotRetirable(CodedCaptureId),
    RetirementEvidenceInvalid(CodedCaptureId),
    FrontierOrderInvalid(CodedCaptureId),
    CaptureAlreadyExists(CodedCaptureId),
    CaptureNotFound(CodedCaptureId),
    CaptureNotOpen(CodedCaptureId),
    DecisionAlreadyObserved(CodedCaptureId),
    CleanNotEligible(CodedCaptureId),
    CleanCommitNotPending(CodedCaptureId),
    CleanCommitUnknownNotPending(CodedCaptureId),
    LaterOperationNotMember {
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    },
    LaterCutNotPending {
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    },
    LaterCutMismatch {
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    },
    LaterCutOrder(CodedCaptureId),
    LaterCutEvidenceInvalid(CodedCaptureId),
}

impl fmt::Display for CodedCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyScope => formatter.write_str("coded capture scope is empty"),
            Self::IncompleteScope => formatter.write_str("coded capture scope is incomplete"),
            Self::UnvalidatedScope => formatter.write_str("coded capture scope is not validated"),
            Self::LowerFrontierUncovered => {
                formatter.write_str("coded capture lower frontier is not covered")
            }
            Self::InvalidScopeGeometry => {
                formatter.write_str("coded capture scope geometry is invalid")
            }
            Self::ScopeIdentityOutOfRange => {
                formatter.write_str("coded capture scope identity is outside the topology")
            }
            Self::ScopeArithmeticOverflow => {
                formatter.write_str("coded capture scope arithmetic overflowed")
            }
            Self::PreparedTransitionStale(capture) => {
                write!(
                    formatter,
                    "prepared transition for capture {capture:?} is stale"
                )
            }
            Self::DurableReceiptMismatch(capture) => {
                write!(
                    formatter,
                    "durable receipt does not cover capture {capture:?}"
                )
            }
            Self::SnapshotInvalid(capture) => {
                write!(formatter, "durable coded capture {capture:?} is invalid")
            }
            Self::CaptureFrontierGenerationMismatch(capture) => write!(
                formatter,
                "capture frontier generation does not match capture {capture:?} recovery generation"
            ),
            Self::FrontierOrderInvalid(capture) => write!(
                formatter,
                "capture {capture:?} lower frontier is after its capture frontier"
            ),
            Self::RetentionSummaryInvalid(capture) => {
                write!(
                    formatter,
                    "retention summary for capture {capture:?} is invalid"
                )
            }
            Self::RetentionFrontierStale(capture) => {
                write!(
                    formatter,
                    "retention frontier for capture {capture:?} is stale"
                )
            }
            Self::CaptureOperationUnresolved { capture, operation } => write!(
                formatter,
                "operation {operation:?} remains unresolved in capture {capture:?}"
            ),
            Self::CaptureNotRetirable(capture) => {
                write!(
                    formatter,
                    "capture {capture:?} still has live or unresolved obligations"
                )
            }
            Self::RetirementEvidenceInvalid(capture) => {
                write!(
                    formatter,
                    "retirement evidence for capture {capture:?} is invalid"
                )
            }
            Self::CaptureAlreadyExists(capture) => {
                write!(formatter, "capture {capture:?} already exists")
            }
            Self::CaptureNotFound(capture) => {
                write!(formatter, "capture {capture:?} was not found")
            }
            Self::CaptureNotOpen(capture) => write!(formatter, "capture {capture:?} is not open"),
            Self::DecisionAlreadyObserved(capture) => {
                write!(
                    formatter,
                    "capture decision for {capture:?} was already observed"
                )
            }
            Self::CleanNotEligible(capture) => {
                write!(formatter, "capture {capture:?} is not eligible for CLEAN")
            }
            Self::CleanCommitNotPending(capture) => {
                write!(formatter, "capture {capture:?} has no pending CLEAN commit")
            }
            Self::CleanCommitUnknownNotPending(capture) => write!(
                formatter,
                "capture {capture:?} has no unresolved CLEAN commit"
            ),
            Self::LaterOperationNotMember { capture, operation } => write!(
                formatter,
                "operation {operation:?} is not a later member of capture {capture:?}"
            ),
            Self::LaterCutNotPending { capture, operation } => write!(
                formatter,
                "later cut for operation {operation:?} in capture {capture:?} is not unknown"
            ),
            Self::LaterCutMismatch { capture, operation } => write!(
                formatter,
                "later cut for operation {operation:?} does not match the exact unknown observation in capture {capture:?}"
            ),
            Self::LaterCutOrder(capture) => {
                write!(
                    formatter,
                    "later cut ordering is invalid for capture {capture:?}"
                )
            }
            Self::LaterCutEvidenceInvalid(capture) => {
                write!(
                    formatter,
                    "later cut evidence for capture {capture:?} is invalid"
                )
            }
        }
    }
}

impl std::error::Error for CodedCaptureError {}

#[derive(Clone, Debug)]
struct CaptureRecord {
    owner_facts: CodedCaptureOwnerFacts,
    phase: CodedCapturePhase,
    scope: BTreeSet<CodedUnitId>,
    membership: BTreeMap<OperationSlotToken, CodedCaptureMembership>,
    pending_later_cuts: BTreeMap<OperationSlotToken, CodedCaptureCut>,
    resolved_later_cuts: BTreeMap<OperationSlotToken, CodedCaptureCut>,
    release_authorized_operations: BTreeSet<OperationSlotToken>,
    release_frontiers: BTreeMap<OperationSlotToken, CodedAuthorityFrontier>,
    retained_frontier: CodedCaptureFrontier,
    retirement_frontier: Option<CodedCaptureFrontier>,
    decision: Option<CodedCaptureDecision>,
}

impl CaptureRecord {
    fn snapshot(&self, capture: CodedCaptureId) -> CodedCaptureSnapshot {
        CodedCaptureSnapshot {
            phase: self.phase,
            capture,
            topology: self.owner_facts.topology.clone(),
            recovery_generation: self.owner_facts.recovery_generation,
            checksum_profile: self.owner_facts.checksum_profile,
            checksum_set_generation: self.owner_facts.checksum_set_generation,
            scope: self.scope.clone(),
            scope_complete: true,
            scope_validated: true,
            lower_frontier_covered: true,
            dirty_regions: self.owner_facts.dirty_regions.clone(),
            checksum_extents: self.owner_facts.checksum_extents.clone(),
            lower_frontier: self.owner_facts.lower_frontier,
            capture_frontier: self.owner_facts.capture_frontier,
            retained_frontier: self.retained_frontier,
            membership: self.membership.clone(),
            pending_later_cuts: self.pending_later_cuts.clone(),
            resolved_later_cuts: self.resolved_later_cuts.clone(),
            release_authorized_operations: self.release_authorized_operations.clone(),
            release_frontiers: self.release_frontiers.clone(),
            retirement_frontier: self.retirement_frontier,
            decision: self.decision,
        }
    }

    fn intersects(&self, units: &BTreeSet<CodedUnitId>) -> bool {
        self.scope.iter().any(|unit| units.contains(unit))
    }

    fn tracks_future(&self) -> bool {
        !matches!(self.phase, CodedCapturePhase::Refused)
    }

    fn mutation_set_closed(&self) -> bool {
        !self.scope.is_empty()
            && !self.membership.values().any(|membership| {
                matches!(
                    membership,
                    CodedCaptureMembership::LaterDurableStalesClean
                        | CodedCaptureMembership::LaterUnknown
                )
            })
    }

    #[cfg(any(test, feature = "test-support"))]
    fn clean_eligible(&self) -> bool {
        self.decision == Some(CodedCaptureDecision::Accepted) && self.mutation_set_closed()
    }

    fn accepts_later_cut(&self, cut: CodedCaptureCut) -> bool {
        let prior_frontier = self
            .owner_facts
            .capture_frontier
            .max(self.retained_frontier);
        cut.topology_epoch == self.owner_facts.topology.topology_epoch()
            && cut.frontier.recovery_generation == cut.recovery_generation
            && cut.frontier > prior_frontier
    }
    fn can_forget_operation(
        &self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    ) -> Result<(), CodedCaptureError> {
        let Some(membership) = self.membership.get(&operation) else {
            return Ok(());
        };
        let resolved = match membership {
            CodedCaptureMembership::Included => {
                self.decision.is_some() || self.phase == CodedCapturePhase::Refused
            }
            CodedCaptureMembership::LaterDurableAfterClean
            | CodedCaptureMembership::LaterDurableStalesClean
            | CodedCaptureMembership::LaterRejected => true,
            CodedCaptureMembership::Later | CodedCaptureMembership::LaterUnknown => false,
        };
        resolved
            .then_some(())
            .ok_or(CodedCaptureError::CaptureOperationUnresolved { capture, operation })
    }
}
/// dwv:req req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set
#[derive(Clone, Default)]
pub struct CodedCaptureCoordinator {
    captures: BTreeMap<CodedCaptureId, CaptureRecord>,
}

impl CodedCaptureCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn from_snapshots(
        snapshots: impl IntoIterator<Item = CodedCaptureSnapshot>,
    ) -> Result<Self, CodedCaptureError> {
        let mut coordinator = Self::new();
        for snapshot in snapshots {
            let capture = snapshot.capture;
            if !snapshot.scope_complete
                || !snapshot.scope_validated
                || !snapshot.lower_frontier_covered
            {
                return Err(CodedCaptureError::SnapshotInvalid(capture));
            }
            let owner_facts = CodedCaptureOwnerFacts::new(
                snapshot.topology.clone(),
                snapshot.recovery_generation,
                snapshot.checksum_profile,
                snapshot.checksum_set_generation,
                ValidatedCodedCaptureScope::from_snapshot_units(snapshot.scope.iter().copied()),
                snapshot.dirty_regions.iter().copied(),
                snapshot.checksum_extents.iter().copied(),
                snapshot.lower_frontier,
                snapshot.capture_frontier,
            );
            coordinator.insert_capture(capture, owner_facts, [])?;
            coordinator.restore_snapshot_state(snapshot)?;
        }
        Ok(coordinator)
    }
    pub fn from_snapshots_for_owner(
        snapshots: impl IntoIterator<Item = CodedCaptureSnapshot>,
        topology: &TopologySnapshot,
        recovery_generation: RecoveryGeneration,
        checksum_profile: ChecksumProfile,
        checksum_set_generation: ChecksumSetGeneration,
        dirty_region_bytes: u64,
    ) -> Result<Self, CodedCaptureError> {
        let mut coordinator = Self::new();
        for snapshot in snapshots {
            let capture = snapshot.capture;
            let scope = CodedCaptureOwnerFacts::selected_invalidation_scope(
                topology,
                snapshot.dirty_regions.iter().copied(),
                snapshot.checksum_extents.iter().copied(),
                dirty_region_bytes,
                checksum_profile,
            )
            .map_err(|_| CodedCaptureError::SnapshotInvalid(capture))?;
            if !snapshot.scope_complete
                || !snapshot.scope_validated
                || !snapshot.lower_frontier_covered
                || snapshot.topology != *topology
                || snapshot.recovery_generation > recovery_generation
                || snapshot.checksum_profile != checksum_profile.id
                || snapshot.checksum_set_generation != checksum_set_generation
                || snapshot.scope != *scope.units()
                || snapshot.lower_frontier.recovery_generation != snapshot.recovery_generation
                || snapshot.capture_frontier.recovery_generation != snapshot.recovery_generation
                || snapshot.retained_frontier < snapshot.lower_frontier
                || snapshot.retained_frontier.recovery_generation > recovery_generation
            {
                return Err(CodedCaptureError::SnapshotInvalid(capture));
            }
            let owner_facts = CodedCaptureOwnerFacts::new(
                snapshot.topology.clone(),
                snapshot.recovery_generation,
                snapshot.checksum_profile,
                snapshot.checksum_set_generation,
                scope,
                snapshot.dirty_regions.iter().copied(),
                snapshot.checksum_extents.iter().copied(),
                snapshot.lower_frontier,
                snapshot.capture_frontier,
            );
            coordinator.insert_capture(capture, owner_facts, [])?;
            coordinator.restore_snapshot_state(snapshot)?;
        }
        Ok(coordinator)
    }
    fn restore_snapshot_state(
        &mut self,
        snapshot: CodedCaptureSnapshot,
    ) -> Result<(), CodedCaptureError> {
        let capture = snapshot.capture;
        let record = self.record_mut(capture)?;
        if !snapshot.validates_internal_state() {
            return Err(CodedCaptureError::SnapshotInvalid(capture));
        }
        record.phase = snapshot.phase;
        record.membership = snapshot.membership;
        record.pending_later_cuts = snapshot.pending_later_cuts;
        record.resolved_later_cuts = snapshot.resolved_later_cuts;
        record.release_authorized_operations = snapshot.release_authorized_operations;
        record.retained_frontier = snapshot.retained_frontier;
        record.release_frontiers = snapshot.release_frontiers;
        record.retirement_frontier = snapshot.retirement_frontier;
        record.decision = snapshot.decision;
        Ok(())
    }

    pub fn snapshots(&self) -> Vec<CodedCaptureSnapshot> {
        self.captures
            .iter()
            .map(|(capture, record)| record.snapshot(*capture))
            .collect()
    }

    pub fn durable_update(
        &self,
        capture: CodedCaptureId,
        expected: Option<&CodedCaptureSnapshot>,
    ) -> Result<CodedCaptureUpdate, CodedCaptureError> {
        if expected.is_some_and(|snapshot| snapshot.capture != capture) {
            return Err(CodedCaptureError::SnapshotInvalid(capture));
        }
        let proposed = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        Ok(CodedCaptureUpdate {
            expected: expected.cloned(),
            proposed,
        })
    }

    pub fn has_unresolved_reopen_state(&self) -> bool {
        self.captures.values().any(|record| {
            matches!(
                record.phase,
                CodedCapturePhase::Open
                    | CodedCapturePhase::CleanCommitPending
                    | CodedCapturePhase::CleanCommitUnknown
            ) || !record.pending_later_cuts.is_empty()
                || record
                    .membership
                    .keys()
                    .any(|operation| !record.release_authorized_operations.contains(operation))
        })
    }

    /// Retain one exact lifecycle release receipt for every affected capture.
    pub fn observe_release_allowed(&mut self, release: &CodedClaimRelease) -> bool {
        let operation = release.operation();
        let mut observed = false;
        for record in self.captures.values_mut() {
            if record.membership.contains_key(&operation) {
                observed |= record.release_authorized_operations.insert(operation);
                record
                    .release_frontiers
                    .insert(operation, release.frontier());
            }
        }
        observed
    }

    /// Start a bounded capture from capabilities issued by the scope and
    /// coded-admission owners.
    pub fn start_capture(
        &mut self,
        capture: CodedCaptureId,
        recovery_generation: RecoveryGeneration,
        checksum_set_generation: ChecksumSetGeneration,
        scope: ValidatedCodedCaptureScope,
        establishment: CodedCaptureEstablishment,
    ) -> Result<(), CodedCaptureError> {
        let (history, boundary, active_claims) = establishment.into_parts();
        if scope.units.is_empty() {
            return Err(CodedCaptureError::EmptyScope);
        }
        let binding = scope
            .binding
            .as_ref()
            .ok_or(CodedCaptureError::UnvalidatedScope)?;
        let lower_frontier =
            CodedCaptureFrontier::new(recovery_generation, history.through_sequence());
        let capture_frontier =
            CodedCaptureFrontier::new(recovery_generation, boundary.capture_sequence());
        let owner_facts = CodedCaptureOwnerFacts::new(
            binding.topology.clone(),
            recovery_generation,
            binding.checksum_profile.id,
            checksum_set_generation,
            scope.clone(),
            binding.dirty_regions.iter().copied(),
            binding.checksum_extents.iter().copied(),
            lower_frontier,
            capture_frontier,
        );
        self.insert_capture(
            capture,
            owner_facts,
            active_claims
                .into_iter()
                .map(|admission| (admission.operation(), admission.units().clone())),
        )
    }

    fn insert_capture(
        &mut self,
        capture: CodedCaptureId,
        owner_facts: CodedCaptureOwnerFacts,
        active_claims: impl IntoIterator<Item = (OperationSlotToken, BTreeSet<CodedUnitId>)>,
    ) -> Result<(), CodedCaptureError> {
        if self.captures.contains_key(&capture) {
            return Err(CodedCaptureError::CaptureAlreadyExists(capture));
        }
        let scope = owner_facts.scope.clone();
        if scope.units.is_empty() {
            return Err(CodedCaptureError::EmptyScope);
        }
        if owner_facts.capture_frontier.recovery_generation != owner_facts.recovery_generation {
            return Err(CodedCaptureError::CaptureFrontierGenerationMismatch(
                capture,
            ));
        }
        if owner_facts.lower_frontier > owner_facts.capture_frontier {
            return Err(CodedCaptureError::FrontierOrderInvalid(capture));
        }
        let membership = active_claims
            .into_iter()
            .filter_map(|(operation, units)| {
                scope
                    .units
                    .iter()
                    .any(|unit| units.contains(unit))
                    .then_some((operation, CodedCaptureMembership::Included))
            })
            .collect();
        let retained_frontier = owner_facts.lower_frontier;
        self.captures.insert(
            capture,
            CaptureRecord {
                owner_facts,
                phase: CodedCapturePhase::Open,
                scope: scope.units,
                membership,
                pending_later_cuts: BTreeMap::new(),
                resolved_later_cuts: BTreeMap::new(),
                release_authorized_operations: BTreeSet::new(),
                release_frontiers: BTreeMap::new(),
                retained_frontier,
                retirement_frontier: None,
                decision: None,
            },
        );
        Ok(())
    }

    /// Observe a later admitted claim against every active capture.
    pub fn observe_admitted_claim(&mut self, admission: &CodedAdmission) {
        for capture in self.captures.values_mut() {
            if capture.tracks_future() && capture.intersects(admission.units()) {
                capture
                    .membership
                    .insert(admission.operation(), CodedCaptureMembership::Later);
            }
        }
    }

    /// Stage an owner-produced cut for a later admitted operation.
    #[cfg(any(test, feature = "test-support"))]
    pub fn observe_later_admission(
        &mut self,
        capture: CodedCaptureId,
        admission: &CodedAdmission,
        topology_epoch: TopologyEpoch,
        recovery_generation: RecoveryGeneration,
        observation: CodedLaterCutObservation,
    ) -> Result<(), CodedCaptureError> {
        self.apply_later_cut(
            capture,
            admission.operation(),
            CodedCaptureCut::new(
                topology_epoch,
                recovery_generation,
                CodedCaptureFrontier::new(recovery_generation, admission.sequence()),
            ),
            observation,
        )
    }

    /// Return whether every applicable active capture permits the operation's effect.
    pub fn effect_allowed(&self, operation: OperationSlotToken) -> bool {
        // Rejected and uncertain cuts remain blocked; a refused capture stops tracking.
        self.captures.values().all(|capture| {
            if !capture.tracks_future() {
                return true;
            }
            matches!(
                capture.membership.get(&operation),
                None | Some(CodedCaptureMembership::Included)
                    | Some(CodedCaptureMembership::LaterDurableAfterClean)
                    | Some(CodedCaptureMembership::LaterDurableStalesClean)
            )
        })
    }

    /// Retire only a resolved capture whose remaining exclusion duty ended.
    ///
    /// A durable refusal leaves conservative dirty state and has no future
    /// exclusion duty, so its stale in-process membership need not acquire
    /// lifecycle release receipts after a process restart. A known CLEAN still
    /// requires empty membership and a newer durable retirement frontier.
    pub fn retire_resolved_capture(
        &mut self,
        capture: CodedCaptureId,
    ) -> Result<CodedCaptureRemoval, CodedCaptureError> {
        let expected = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        let record = self.record_mut(capture)?;
        let retirable = match record.phase {
            CodedCapturePhase::Refused => true,
            CodedCapturePhase::CleanKnown => {
                record.membership.is_empty()
                    && record.pending_later_cuts.is_empty()
                    && record.retirement_frontier.is_some_and(|frontier| {
                        frontier
                            > record
                                .owner_facts
                                .capture_frontier
                                .max(record.retained_frontier)
                    })
            }
            _ => false,
        };
        if !retirable {
            return Err(CodedCaptureError::CaptureNotRetirable(capture));
        }
        self.captures.remove(&capture);
        Ok(CodedCaptureRemoval { expected })
    }

    /// Forget one lifecycle-released operation under its owner-issued receipt.
    pub fn compact_released_operation(
        &mut self,
        release: &CodedClaimRelease,
        recovery_generation: RecoveryGeneration,
    ) -> Result<Vec<CodedCaptureId>, CodedCaptureError> {
        let operation = release.operation();
        let frontier =
            CodedCaptureFrontier::new(recovery_generation, release.frontier().admission_sequence());
        let captures = self
            .captures
            .iter()
            .filter_map(|(capture, record)| {
                record
                    .membership
                    .contains_key(&operation)
                    .then_some(*capture)
            })
            .collect::<Vec<_>>();
        for capture in &captures {
            let record = self.record_mut(*capture)?;
            if !record.release_authorized_operations.contains(&operation) {
                return Err(CodedCaptureError::RetentionSummaryInvalid(*capture));
            }
            match record.can_forget_operation(*capture, operation) {
                Ok(()) => {
                    record.membership.remove(&operation);
                    record.pending_later_cuts.remove(&operation);
                    record.resolved_later_cuts.remove(&operation);
                    record.release_authorized_operations.remove(&operation);
                    record.release_frontiers.remove(&operation);
                    record.retained_frontier = record.retained_frontier.max(frontier);
                }
                Err(CodedCaptureError::CaptureOperationUnresolved { .. }) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(captures)
    }

    /// Compact retained lifecycle receipts whose outcome is now resolved.
    pub fn compact_authorized_operations(
        &mut self,
        capture: CodedCaptureId,
        recovery_generation: RecoveryGeneration,
    ) -> Result<bool, CodedCaptureError> {
        let operations = self
            .record_mut(capture)?
            .release_frontiers
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let mut changed = false;
        for operation in operations {
            let record = self.record_mut(capture)?;
            if record.can_forget_operation(capture, operation).is_err() {
                continue;
            }
            let authority_frontier = record.release_frontiers[&operation];
            let frontier = CodedCaptureFrontier::new(
                recovery_generation,
                authority_frontier.admission_sequence(),
            );
            record.membership.remove(&operation);
            record.pending_later_cuts.remove(&operation);
            record.resolved_later_cuts.remove(&operation);
            record.release_authorized_operations.remove(&operation);
            record.release_frontiers.remove(&operation);
            record.retained_frontier = record.retained_frontier.max(frontier);
            changed = true;
        }
        Ok(changed)
    }

    #[cfg(test)]
    fn apply_compaction(
        &mut self,
        summary: CodedCaptureRetentionSummary,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(summary.capture)?;
        if summary.topology_epoch != record.owner_facts.topology.topology_epoch()
            || summary.release_authorized_operations != summary.releasable_operations
            || !summary
                .releasable_operations
                .is_subset(&record.release_authorized_operations)
        {
            return Err(CodedCaptureError::RetentionSummaryInvalid(summary.capture));
        }
        if summary.frontier < record.retained_frontier {
            return Err(CodedCaptureError::RetentionFrontierStale(summary.capture));
        }
        if record.membership.keys().any(|operation| {
            !summary.releasable_operations.contains(operation)
                && !summary.retained_operations.contains(operation)
        }) || summary
            .retained_operations
            .iter()
            .any(|operation| !record.membership.contains_key(operation))
        {
            return Err(CodedCaptureError::RetentionSummaryInvalid(summary.capture));
        }
        for operation in &summary.releasable_operations {
            record.can_forget_operation(summary.capture, *operation)?;
        }
        for operation in summary.releasable_operations {
            record.membership.remove(&operation);
            record.release_authorized_operations.remove(&operation);
        }
        record.retained_frontier = summary.frontier;
        Ok(())
    }

    #[cfg(test)]
    fn apply_retirement(
        &mut self,
        retirement: CodedCaptureRetirement,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(retirement.capture)?;
        if retirement.topology_epoch != record.owner_facts.topology.topology_epoch()
            || retirement.frontier < record.retained_frontier
            || (record.phase == CodedCapturePhase::CleanKnown
                && retirement.frontier
                    <= record
                        .owner_facts
                        .capture_frontier
                        .max(record.retained_frontier))
        {
            return Err(CodedCaptureError::RetirementEvidenceInvalid(
                retirement.capture,
            ));
        }
        let retirable = match record.phase {
            CodedCapturePhase::Refused => true,
            CodedCapturePhase::CleanKnown => {
                record.membership.is_empty() && record.pending_later_cuts.is_empty()
            }
            _ => false,
        };
        if !retirable {
            return Err(CodedCaptureError::CaptureNotRetirable(retirement.capture));
        }
        self.captures.remove(&retirement.capture);
        Ok(())
    }

    pub fn capture_snapshot(&self, capture: CodedCaptureId) -> Option<CodedCaptureSnapshot> {
        self.captures
            .get(&capture)
            .map(|record| record.snapshot(capture))
    }

    /// Apply one exact recovery-owner refusal to its selected capture.
    pub fn apply_clean_refusal(
        &mut self,
        capture: CodedCaptureId,
        refusal: &RecoveryCleanRefusalPermit,
    ) -> Result<CodedCaptureUpdate, CodedCaptureError> {
        let prior = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        let record = self.record_mut(capture)?;
        if record.phase != CodedCapturePhase::Open
            || record.decision.is_some()
            || refusal.topology_epoch() != record.owner_facts.topology.topology_epoch()
            || refusal.generation() < record.owner_facts.recovery_generation
            || refusal.regions().iter().copied().collect::<BTreeSet<_>>()
                != record.owner_facts.dirty_regions
            || refusal
                .checksum_extents()
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != record.owner_facts.checksum_extents
        {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        record.decision = Some(CodedCaptureDecision::Rejected);
        record.phase = CodedCapturePhase::Refused;
        Ok(CodedCaptureUpdate {
            expected: Some(prior),
            proposed: record.snapshot(capture),
        })
    }

    /// Prepare one accepted CLEAN result without installing it as durable state.
    pub fn prepare_clean_commit(
        &self,
        capture: CodedCaptureId,
        permit: &RecoveryCleanPermit,
    ) -> Result<PreparedCodedCleanCommit, CodedCaptureError> {
        let prior = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        let mut candidate = self.clone();
        let record = candidate.record_mut(capture)?;
        if record.phase != CodedCapturePhase::Open
            || record.decision.is_some()
            || permit.topology_epoch() != record.owner_facts.topology.topology_epoch()
            || permit.generation() < record.owner_facts.recovery_generation
            || permit.regions().iter().copied().collect::<BTreeSet<_>>()
                != record.owner_facts.dirty_regions
            || permit
                .checksum_extents()
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != record.owner_facts.checksum_extents
            || !record.mutation_set_closed()
        {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        record.decision = Some(CodedCaptureDecision::Accepted);
        record.phase = CodedCapturePhase::CleanKnown;
        let proposed_record = record.clone();
        let proposed = proposed_record.snapshot(capture);
        Ok(PreparedCodedCleanCommit {
            capture,
            expected_generation: permit.generation(),
            prior,
            proposed,
            proposed_record,
        })
    }

    /// Install a prepared CLEAN only under the exact recovery-owner receipt.
    pub fn confirm_clean_commit(
        &mut self,
        prepared: PreparedCodedCleanCommit,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        if self.capture_snapshot(prepared.capture).as_ref() != Some(&prepared.prior) {
            return Err(CodedCaptureError::PreparedTransitionStale(prepared.capture));
        }
        if receipt.expected_generation() != prepared.expected_generation
            || receipt.topology_epoch() != prepared.proposed.topology.topology_epoch()
            || prepared.expected_generation.checked_next() != Some(receipt.generation())
            || !receipt.committed_coded_capture(&prepared.proposed)
        {
            return Err(CodedCaptureError::DurableReceiptMismatch(prepared.capture));
        }
        self.captures
            .insert(prepared.capture, prepared.proposed_record);
        Ok(())
    }

    /// Prepare an owner-derived later cut without installing it as durable state.
    pub fn prepare_later_cut(
        &self,
        capture: CodedCaptureId,
        admission: &CodedAdmission,
        topology_epoch: TopologyEpoch,
        expected_generation: RecoveryGeneration,
    ) -> Result<PreparedCodedLaterCut, CodedCaptureError> {
        let committed_generation = expected_generation
            .checked_next()
            .ok_or(CodedCaptureError::LaterCutEvidenceInvalid(capture))?;
        let prior = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        let mut candidate = self.clone();
        let cut = CodedCaptureCut::new(
            topology_epoch,
            committed_generation,
            CodedCaptureFrontier::new(committed_generation, admission.sequence()),
        );
        let observation = match candidate.record_mut(capture)?.phase {
            CodedCapturePhase::Open => CodedLaterCutObservation::DurableStalesClean,
            CodedCapturePhase::CleanKnown => CodedLaterCutObservation::DurableAfterClean,
            CodedCapturePhase::Refused => {
                return Err(CodedCaptureError::CaptureNotOpen(capture));
            }
            CodedCapturePhase::CleanCommitPending | CodedCapturePhase::CleanCommitUnknown => {
                return Err(CodedCaptureError::LaterCutOrder(capture));
            }
        };
        candidate.apply_later_cut(capture, admission.operation(), cut, observation)?;
        let proposed_record = candidate.record_mut(capture)?.clone();
        let proposed = proposed_record.snapshot(capture);
        Ok(PreparedCodedLaterCut {
            capture,
            expected_generation,
            prior,
            proposed,
            proposed_record,
        })
    }

    /// Install a prepared later cut only under the exact recovery-owner receipt.
    pub fn confirm_later_cut(
        &mut self,
        prepared: PreparedCodedLaterCut,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        if self.capture_snapshot(prepared.capture).as_ref() != Some(&prepared.prior) {
            return Err(CodedCaptureError::PreparedTransitionStale(prepared.capture));
        }
        if receipt.expected_generation() != prepared.expected_generation
            || receipt.topology_epoch() != prepared.proposed.topology.topology_epoch()
            || prepared.expected_generation.checked_next() != Some(receipt.generation())
            || !receipt.committed_coded_capture(&prepared.proposed)
        {
            return Err(CodedCaptureError::DurableReceiptMismatch(prepared.capture));
        }
        self.captures
            .insert(prepared.capture, prepared.proposed_record);
        Ok(())
    }

    /// Apply one exact resolution issued by recovery inspection.
    pub fn apply_reconciliation(
        &mut self,
        receipt: CodedCaptureReconciliationReceipt,
    ) -> Result<(), CodedCaptureError> {
        let capture = receipt.capture();
        if self.capture_snapshot(capture).as_ref() != Some(&receipt.expected_unknown)
            || !receipt.resolved.validates_internal_state()
        {
            return Err(CodedCaptureError::DurableReceiptMismatch(capture));
        }
        self.restore_snapshot_state(receipt.resolved)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn observe_decision(
        &mut self,
        capture: CodedCaptureId,
        decision: CodedCaptureDecision,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.phase != CodedCapturePhase::Open {
            return Err(CodedCaptureError::CaptureNotOpen(capture));
        }
        if record.decision.is_some() {
            return Err(CodedCaptureError::DecisionAlreadyObserved(capture));
        }
        record.decision = Some(decision);
        if decision == CodedCaptureDecision::Rejected {
            record.phase = CodedCapturePhase::Refused;
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn request_clean(&mut self, capture: CodedCaptureId) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.phase != CodedCapturePhase::Open {
            return Err(CodedCaptureError::CaptureNotOpen(capture));
        }
        if !record.clean_eligible() {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        record.phase = CodedCapturePhase::CleanCommitPending;
        Ok(())
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn observe_clean_commit(
        &mut self,
        capture: CodedCaptureId,
        observation: CodedCleanCommitObservation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.phase != CodedCapturePhase::CleanCommitPending {
            return Err(CodedCaptureError::CleanCommitNotPending(capture));
        }
        if observation == CodedCleanCommitObservation::Durable && !record.clean_eligible() {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        record.phase = match observation {
            CodedCleanCommitObservation::Durable => CodedCapturePhase::CleanKnown,
            CodedCleanCommitObservation::Rejected => CodedCapturePhase::Refused,
            CodedCleanCommitObservation::Unknown => CodedCapturePhase::CleanCommitUnknown,
        };
        Ok(())
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn reconcile_clean_commit(
        &mut self,
        capture: CodedCaptureId,
        observation: CodedCleanReconciliation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.phase != CodedCapturePhase::CleanCommitUnknown {
            return Err(CodedCaptureError::CleanCommitUnknownNotPending(capture));
        }
        if observation == CodedCleanReconciliation::Durable && !record.clean_eligible() {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        record.phase = match observation {
            CodedCleanReconciliation::Durable => CodedCapturePhase::CleanKnown,
            CodedCleanReconciliation::Rejected => CodedCapturePhase::Refused,
        };
        Ok(())
    }

    fn apply_later_cut(
        &mut self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        cut: CodedCaptureCut,
        observation: CodedLaterCutObservation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if !record.tracks_future() {
            return Err(CodedCaptureError::CaptureNotOpen(capture));
        }
        if record.membership.get(&operation) != Some(&CodedCaptureMembership::Later) {
            return Err(CodedCaptureError::LaterOperationNotMember { capture, operation });
        }
        if matches!(
            observation,
            CodedLaterCutObservation::DurableAfterClean
                | CodedLaterCutObservation::DurableStalesClean
                | CodedLaterCutObservation::Unknown
        ) && !record.accepts_later_cut(cut)
        {
            return Err(CodedCaptureError::LaterCutEvidenceInvalid(capture));
        }
        if observation == CodedLaterCutObservation::DurableAfterClean
            && record.phase != CodedCapturePhase::CleanKnown
        {
            return Err(CodedCaptureError::LaterCutOrder(capture));
        }
        if observation == CodedLaterCutObservation::DurableStalesClean
            && record.phase == CodedCapturePhase::CleanKnown
        {
            return Err(CodedCaptureError::LaterCutOrder(capture));
        }
        let membership = match observation {
            CodedLaterCutObservation::DurableAfterClean => {
                record.resolved_later_cuts.insert(operation, cut);
                record.retirement_frontier = Some(
                    record
                        .retirement_frontier
                        .map_or(cut.frontier, |frontier| frontier.max(cut.frontier)),
                );
                CodedCaptureMembership::LaterDurableAfterClean
            }
            CodedLaterCutObservation::DurableStalesClean => {
                record.resolved_later_cuts.insert(operation, cut);
                record.retirement_frontier = Some(
                    record
                        .retirement_frontier
                        .map_or(cut.frontier, |frontier| frontier.max(cut.frontier)),
                );
                record.phase = CodedCapturePhase::Refused;
                CodedCaptureMembership::LaterDurableStalesClean
            }
            CodedLaterCutObservation::Rejected => CodedCaptureMembership::LaterRejected,
            CodedLaterCutObservation::Unknown => {
                record.pending_later_cuts.insert(operation, cut);
                CodedCaptureMembership::LaterUnknown
            }
        };
        record.membership.insert(operation, membership);
        Ok(())
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn observe_later_cut(
        &mut self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        cut: CodedCaptureCut,
        observation: CodedLaterCutObservation,
    ) -> Result<(), CodedCaptureError> {
        self.apply_later_cut(capture, operation, cut, observation)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn reconcile_later_cut(
        &mut self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        observation: CodedLaterCutReconciliation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.membership.get(&operation) != Some(&CodedCaptureMembership::LaterUnknown)
            || !record.pending_later_cuts.contains_key(&operation)
        {
            return Err(CodedCaptureError::LaterCutNotPending { capture, operation });
        }
        if observation == CodedLaterCutReconciliation::DurableAfterClean
            && record.phase != CodedCapturePhase::CleanKnown
        {
            return Err(CodedCaptureError::LaterCutOrder(capture));
        }
        if observation == CodedLaterCutReconciliation::StalesClean
            && record.phase == CodedCapturePhase::CleanKnown
        {
            return Err(CodedCaptureError::LaterCutOrder(capture));
        }
        let cut = record
            .pending_later_cuts
            .remove(&operation)
            .ok_or(CodedCaptureError::LaterCutNotPending { capture, operation })?;
        let membership = match observation {
            CodedLaterCutReconciliation::DurableAfterClean => {
                record.resolved_later_cuts.insert(operation, cut);
                record.retirement_frontier = Some(
                    record
                        .retirement_frontier
                        .map_or(cut.frontier, |frontier| frontier.max(cut.frontier)),
                );
                CodedCaptureMembership::LaterDurableAfterClean
            }
            CodedLaterCutReconciliation::StalesClean => {
                record.resolved_later_cuts.insert(operation, cut);
                record.retirement_frontier = Some(
                    record
                        .retirement_frontier
                        .map_or(cut.frontier, |frontier| frontier.max(cut.frontier)),
                );
                record.phase = CodedCapturePhase::Refused;
                CodedCaptureMembership::LaterDurableStalesClean
            }
            CodedLaterCutReconciliation::Rejected => CodedCaptureMembership::LaterRejected,
        };
        record.membership.insert(operation, membership);
        Ok(())
    }

    #[cfg(test)]
    fn start_capture_unchecked(
        &mut self,
        capture: CodedCaptureId,
        owner_facts: CodedCaptureOwnerFacts,
        active_claims: impl IntoIterator<Item = (OperationSlotToken, BTreeSet<CodedUnitId>)>,
    ) -> Result<(), CodedCaptureError> {
        self.insert_capture(capture, owner_facts, active_claims)
    }

    #[cfg(test)]
    fn observe_admitted_claim_unchecked(
        &mut self,
        operation: OperationSlotToken,
        units: &BTreeSet<CodedUnitId>,
    ) {
        for capture in self.captures.values_mut() {
            if capture.tracks_future() && capture.intersects(units) {
                capture
                    .membership
                    .insert(operation, CodedCaptureMembership::Later);
            }
        }
    }

    #[cfg(test)]
    fn compact_capture_unchecked(
        &mut self,
        summary: CodedCaptureRetentionSummary,
    ) -> Result<(), CodedCaptureError> {
        self.apply_compaction(summary)
    }

    #[cfg(test)]
    fn retire_capture_unchecked(
        &mut self,
        retirement: CodedCaptureRetirement,
    ) -> Result<(), CodedCaptureError> {
        self.apply_retirement(retirement)
    }

    #[cfg(test)]
    fn reconcile_later_cut_unchecked(
        &mut self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        cut: CodedCaptureCut,
        observation: CodedLaterCutReconciliation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.pending_later_cuts.get(&operation).copied() != Some(cut) {
            return Err(CodedCaptureError::LaterCutMismatch { capture, operation });
        }
        self.reconcile_later_cut(capture, operation, observation)
    }

    fn record_mut(
        &mut self,
        capture: CodedCaptureId,
    ) -> Result<&mut CaptureRecord, CodedCaptureError> {
        self.captures
            .get_mut(&capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BLAKE3_256_PROFILE, DIRTY_REGION_BYTES};

    fn token(index: u32) -> OperationSlotToken {
        OperationSlotToken::new(index, 1)
    }

    fn release(operation: OperationSlotToken) -> CodedClaimRelease {
        CodedClaimRelease::for_test(operation, u64::from(operation.index) + 1)
    }

    fn units(units: impl IntoIterator<Item = u32>) -> BTreeSet<CodedUnitId> {
        units
            .into_iter()
            .map(|unit| CodedUnitId(u64::from(unit)))
            .collect()
    }
    fn owner_facts(scope: ValidatedCodedCaptureScope) -> CodedCaptureOwnerFacts {
        use dwv_core::{
            ArrayId, AssignmentGeneration, AssignmentInstanceId, CodingPosition, CodingProfile,
            MemberRole, ProtectedGeometry, SlotId, TopologyAssignment, TopologyEpoch,
        };

        let topology = TopologySnapshot::new(
            ArrayId([1; 16]),
            TopologyEpoch(1),
            CodingProfile::new(1, 1).unwrap(),
            ProtectedGeometry::new(4096, 512).unwrap(),
            vec![
                TopologyAssignment::new(
                    SlotId::from_bytes([1; 16]),
                    MemberRole::Data,
                    CodingPosition(0),
                    AssignmentInstanceId::from_bytes([2; 16]),
                    AssignmentGeneration(1),
                ),
                TopologyAssignment::new(
                    SlotId::from_bytes([2; 16]),
                    MemberRole::Parity,
                    CodingPosition(1),
                    AssignmentInstanceId::from_bytes([3; 16]),
                    AssignmentGeneration(1),
                ),
            ],
        )
        .unwrap();
        CodedCaptureOwnerFacts::new(
            topology,
            RecoveryGeneration::ZERO,
            ChecksumProfileId(1),
            ChecksumSetGeneration::INITIAL,
            scope,
            [RegionId(0)],
            [IntegrityExtentId(0)],
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 0),
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 1),
        )
    }

    fn later_cut() -> CodedCaptureCut {
        CodedCaptureCut::new(
            dwv_core::TopologyEpoch(1),
            RecoveryGeneration(1),
            CodedCaptureFrontier::new(RecoveryGeneration(1), 2),
        )
    }

    #[test]
    fn selected_scope_unions_block_dirty_region_and_checksum_extent_geometry() {
        use dwv_core::{
            ArrayId, AssignmentGeneration, AssignmentInstanceId, CodingPosition, CodingProfile,
            MemberRole, ProtectedGeometry, SlotId, TopologyAssignment, TopologyEpoch,
        };

        let topology = TopologySnapshot::new(
            ArrayId([4; 16]),
            TopologyEpoch(1),
            CodingProfile::new(1, 1).unwrap(),
            ProtectedGeometry::new(8 * 1024 * 1024, 512).unwrap(),
            vec![
                TopologyAssignment::new(
                    SlotId::from_bytes([1; 16]),
                    MemberRole::Data,
                    CodingPosition(0),
                    AssignmentInstanceId::from_bytes([2; 16]),
                    AssignmentGeneration(1),
                ),
                TopologyAssignment::new(
                    SlotId::from_bytes([2; 16]),
                    MemberRole::Parity,
                    CodingPosition(1),
                    AssignmentInstanceId::from_bytes([3; 16]),
                    AssignmentGeneration(1),
                ),
            ],
        )
        .unwrap();
        let scope = CodedCaptureOwnerFacts::selected_invalidation_scope(
            &topology,
            [RegionId(1025)],
            [IntegrityExtentId(0)],
            DIRTY_REGION_BYTES,
            BLAKE3_256_PROFILE,
        )
        .unwrap();

        assert_eq!(scope.units().len(), 8_200);
        assert!(scope.units().contains(&CodedUnitId(0)));
        assert!(scope.units().contains(&CodedUnitId(8_191)));
        assert!(!scope.units().contains(&CodedUnitId(8_192)));
        assert!(!scope.units().contains(&CodedUnitId(8_199)));
        assert!(scope.units().contains(&CodedUnitId(8_200)));
        assert!(scope.units().contains(&CodedUnitId(8_207)));
    }

    #[test]
    fn capture_rejects_incoherent_frontier_owner_facts() {
        let capture = CodedCaptureId(8);
        let scope = ValidatedCodedCaptureScope::complete([CodedUnitId(0)]);

        let mut mismatched_generation = owner_facts(scope.clone());
        mismatched_generation.capture_frontier =
            CodedCaptureFrontier::new(RecoveryGeneration(1), 1);
        let mut captures = CodedCaptureCoordinator::new();
        assert_eq!(
            captures.start_capture_unchecked(capture, mismatched_generation, []),
            Err(CodedCaptureError::CaptureFrontierGenerationMismatch(
                capture
            ))
        );

        let mut reversed = owner_facts(scope);
        reversed.lower_frontier = CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 2);
        let mut captures = CodedCaptureCoordinator::new();
        assert_eq!(
            captures.start_capture_unchecked(capture, reversed, []),
            Err(CodedCaptureError::FrontierOrderInvalid(capture))
        );
    }

    #[test]
    fn older_recovery_generation_is_not_a_newer_later_cut_even_with_a_larger_sequence() {
        let capture = CodedCaptureId(9);
        let operation = token(4);
        let mut facts = owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)]));
        facts.recovery_generation = RecoveryGeneration(1);
        facts.lower_frontier = CodedCaptureFrontier::new(RecoveryGeneration(1), 0);
        facts.capture_frontier = CodedCaptureFrontier::new(RecoveryGeneration(1), 1);

        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(capture, facts, [])
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
            .unwrap();
        captures.observe_admitted_claim_unchecked(operation, &units([0]));

        let older_generation = CodedCaptureCut::new(
            dwv_core::TopologyEpoch(1),
            RecoveryGeneration::ZERO,
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, u64::MAX),
        );
        assert_eq!(
            captures.observe_later_cut(
                capture,
                operation,
                older_generation,
                CodedLaterCutObservation::DurableAfterClean,
            ),
            Err(CodedCaptureError::LaterCutEvidenceInvalid(capture))
        );
        assert!(!captures.effect_allowed(operation));
    }

    #[test]
    fn retention_summary_bounds_slot_reuse_and_preserves_unknown_until_reconciled() {
        let capture = CodedCaptureId(10);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
            .unwrap();

        for generation in 1..=32 {
            let operation = OperationSlotToken::new(0, generation);
            let frontier =
                CodedCaptureFrontier::new(RecoveryGeneration::ZERO, u64::from(generation) + 1);
            captures.observe_admitted_claim_unchecked(operation, &units([0]));
            captures
                .observe_later_cut(
                    capture,
                    operation,
                    CodedCaptureCut::new(
                        dwv_core::TopologyEpoch(1),
                        RecoveryGeneration::ZERO,
                        frontier,
                    ),
                    CodedLaterCutObservation::DurableAfterClean,
                )
                .unwrap();
            assert!(captures.observe_release_allowed(&release(operation)));
            captures
                .compact_capture_unchecked(
                    CodedCaptureRetentionSummary::new(
                        capture,
                        dwv_core::TopologyEpoch(1),
                        frontier,
                        [operation],
                    )
                    .with_release_authorized_operations([operation]),
                )
                .unwrap();
            let snapshot = captures.capture_snapshot(capture).unwrap();
            assert!(snapshot.membership.is_empty());
            assert_eq!(snapshot.retained_frontier, frontier);
        }

        let live = OperationSlotToken::new(2, 1);
        captures.observe_admitted_claim_unchecked(live, &units([0]));
        assert_eq!(
            captures.compact_capture_unchecked(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                later_cut().frontier,
                [],
            )),
            Err(CodedCaptureError::RetentionSummaryInvalid(capture))
        );
        captures
            .compact_capture_unchecked(
                CodedCaptureRetentionSummary::new(
                    capture,
                    dwv_core::TopologyEpoch(1),
                    later_cut().frontier,
                    [],
                )
                .with_retained_operations([live]),
            )
            .unwrap();
        assert_eq!(
            captures
                .capture_snapshot(capture)
                .unwrap()
                .membership
                .get(&live),
            Some(&CodedCaptureMembership::Later)
        );
        captures
            .observe_later_cut(
                capture,
                live,
                later_cut(),
                CodedLaterCutObservation::Rejected,
            )
            .unwrap();
        assert!(!captures.effect_allowed(live));
        assert_eq!(
            captures.compact_capture_unchecked(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                later_cut().frontier,
                [live],
            )),
            Err(CodedCaptureError::RetentionSummaryInvalid(capture))
        );
        assert!(!captures.effect_allowed(live));
        assert!(captures.observe_release_allowed(&release(live)));
        captures
            .compact_capture_unchecked(
                CodedCaptureRetentionSummary::new(
                    capture,
                    dwv_core::TopologyEpoch(1),
                    later_cut().frontier,
                    [live],
                )
                .with_release_authorized_operations([live]),
            )
            .unwrap();
        assert!(captures.effect_allowed(live));

        let unknown_cut = CodedCaptureCut::new(
            dwv_core::TopologyEpoch(1),
            RecoveryGeneration(1),
            CodedCaptureFrontier::new(RecoveryGeneration(1), 3),
        );
        let unresolved = OperationSlotToken::new(1, 1);
        captures.observe_admitted_claim_unchecked(unresolved, &units([0]));
        captures
            .observe_later_cut(
                capture,
                unresolved,
                unknown_cut,
                CodedLaterCutObservation::Unknown,
            )
            .unwrap();
        assert_eq!(
            captures.compact_capture_unchecked(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                unknown_cut.frontier,
                [],
            )),
            Err(CodedCaptureError::RetentionSummaryInvalid(capture))
        );
        let retirement_frontier = CodedCaptureFrontier::new(RecoveryGeneration(1), 5);
        let newer_cut = CodedCaptureCut::new(
            dwv_core::TopologyEpoch(1),
            RecoveryGeneration(1),
            CodedCaptureFrontier::new(RecoveryGeneration(1), 4),
        );
        captures
            .compact_capture_unchecked(
                CodedCaptureRetentionSummary::new(
                    capture,
                    dwv_core::TopologyEpoch(1),
                    newer_cut.frontier,
                    [],
                )
                .with_retained_operations([unresolved]),
            )
            .unwrap();
        assert_eq!(
            captures
                .capture_snapshot(capture)
                .unwrap()
                .pending_later_cuts
                .get(&unresolved),
            Some(&unknown_cut)
        );
        assert_eq!(
            captures.reconcile_later_cut_unchecked(
                capture,
                unresolved,
                newer_cut,
                CodedLaterCutReconciliation::Rejected,
            ),
            Err(CodedCaptureError::LaterCutMismatch {
                capture,
                operation: unresolved,
            })
        );
        assert_eq!(
            captures.reconcile_later_cut_unchecked(
                capture,
                unresolved,
                unknown_cut,
                CodedLaterCutReconciliation::Rejected,
            ),
            Ok(())
        );

        assert_eq!(
            captures.compact_capture_unchecked(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 2),
                [],
            )),
            Err(CodedCaptureError::RetentionFrontierStale(capture))
        );
        assert_eq!(
            captures.retire_capture_unchecked(CodedCaptureRetirement::new(
                capture,
                dwv_core::TopologyEpoch(1),
                retirement_frontier,
            )),
            Err(CodedCaptureError::CaptureNotRetirable(capture))
        );
        assert!(captures.observe_release_allowed(&release(unresolved)));

        captures
            .compact_capture_unchecked(
                CodedCaptureRetentionSummary::new(
                    capture,
                    dwv_core::TopologyEpoch(1),
                    newer_cut.frontier,
                    [unresolved],
                )
                .with_release_authorized_operations([unresolved]),
            )
            .unwrap();
        captures
            .retire_capture_unchecked(CodedCaptureRetirement::new(
                capture,
                dwv_core::TopologyEpoch(1),
                retirement_frontier,
            ))
            .unwrap();
        assert!(captures.capture_snapshot(capture).is_none());
    }

    #[test]
    fn capture_rejects_an_empty_internal_scope() {
        let mut captures = CodedCaptureCoordinator::new();
        assert_eq!(
            captures.start_capture_unchecked(
                CodedCaptureId(0),
                owner_facts(ValidatedCodedCaptureScope::complete([])),
                [],
            ),
            Err(CodedCaptureError::EmptyScope)
        );
        assert!(captures.capture_snapshot(CodedCaptureId(0)).is_none());
    }

    #[test]
    fn capture_records_included_and_later_membership() {
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                CodedCaptureId(0),
                owner_facts(ValidatedCodedCaptureScope::complete([
                    CodedUnitId(0),
                    CodedUnitId(1),
                ])),
                [(token(0), units([0]))],
            )
            .unwrap();
        captures.observe_admitted_claim_unchecked(token(1), &units([1]));
        let snapshot = captures.capture_snapshot(CodedCaptureId(0)).unwrap();
        assert_eq!(
            snapshot.membership.get(&token(0)),
            Some(&CodedCaptureMembership::Included)
        );
        assert_eq!(
            snapshot.membership.get(&token(1)),
            Some(&CodedCaptureMembership::Later)
        );
    }

    #[test]
    fn capture_binds_owner_facts_and_requires_a_newer_durable_cut() {
        let capture = CodedCaptureId(7);
        let operation = token(3);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [],
            )
            .unwrap();

        let snapshot = captures.capture_snapshot(capture).unwrap();
        assert_eq!(snapshot.capture, capture);
        assert_eq!(
            snapshot.topology.topology_epoch(),
            dwv_core::TopologyEpoch(1)
        );
        assert_eq!(snapshot.recovery_generation, RecoveryGeneration::ZERO);
        assert_eq!(snapshot.checksum_profile, ChecksumProfileId(1));
        assert_eq!(
            snapshot.checksum_set_generation,
            ChecksumSetGeneration::INITIAL
        );
        assert!(snapshot.dirty_regions.contains(&RegionId(0)));
        assert!(snapshot.checksum_extents.contains(&IntegrityExtentId(0)));
        assert_eq!(
            snapshot.lower_frontier,
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 0)
        );
        assert_eq!(
            snapshot.capture_frontier,
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 1)
        );

        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
            .unwrap();
        captures.observe_admitted_claim_unchecked(operation, &units([0]));

        let stale_cut = CodedCaptureCut::new(
            dwv_core::TopologyEpoch(1),
            RecoveryGeneration::ZERO,
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 1),
        );
        assert_eq!(
            captures.observe_later_cut(
                capture,
                operation,
                stale_cut,
                CodedLaterCutObservation::DurableAfterClean,
            ),
            Err(CodedCaptureError::LaterCutEvidenceInvalid(capture))
        );
        assert!(!captures.effect_allowed(operation));

        captures
            .observe_later_cut(
                capture,
                operation,
                later_cut(),
                CodedLaterCutObservation::DurableAfterClean,
            )
            .unwrap();
        assert!(captures.effect_allowed(operation));
    }

    #[test]
    fn rejected_capture_does_not_classify_future_admissions() {
        let mut captures = CodedCaptureCoordinator::new();
        let capture = CodedCaptureId(0);
        let inherited = token(0);
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [(inherited, units([0]))],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Rejected)
            .unwrap();
        captures.observe_admitted_claim_unchecked(token(1), &units([0]));
        assert!(
            !captures
                .capture_snapshot(capture)
                .unwrap()
                .membership
                .contains_key(&token(1))
        );
        assert!(captures.effect_allowed(token(1)));
        assert_eq!(
            captures
                .capture_snapshot(capture)
                .unwrap()
                .membership
                .get(&inherited),
            Some(&CodedCaptureMembership::Included)
        );
        captures
            .retire_capture_unchecked(CodedCaptureRetirement::new(
                capture,
                dwv_core::TopologyEpoch(1),
                CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 0),
            ))
            .unwrap();
        assert!(captures.capture_snapshot(capture).is_none());
    }
    #[test]
    fn every_applicable_capture_must_satisfy_its_later_cut_before_effect() {
        let mut captures = CodedCaptureCoordinator::new();
        for capture in [CodedCaptureId(0), CodedCaptureId(1)] {
            captures
                .start_capture_unchecked(
                    capture,
                    owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                    [],
                )
                .unwrap();
            captures
                .observe_decision(capture, CodedCaptureDecision::Accepted)
                .unwrap();
            captures.request_clean(capture).unwrap();
            captures
                .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
                .unwrap();
        }

        let later = token(1);
        captures.observe_admitted_claim_unchecked(later, &units([0]));
        assert!(!captures.effect_allowed(later));

        captures
            .observe_later_cut(
                CodedCaptureId(0),
                later,
                later_cut(),
                CodedLaterCutObservation::DurableAfterClean,
            )
            .unwrap();
        assert!(
            !captures.effect_allowed(later),
            "satisfying one of two overlapping applicable capture cuts must not permit the effect"
        );

        captures
            .observe_later_cut(
                CodedCaptureId(1),
                later,
                later_cut(),
                CodedLaterCutObservation::DurableAfterClean,
            )
            .unwrap();
        assert!(captures.effect_allowed(later));
    }

    #[test]
    fn disjoint_active_capture_does_not_gate_later_effect() {
        let mut captures = CodedCaptureCoordinator::new();
        for (capture, unit) in [(CodedCaptureId(0), 0), (CodedCaptureId(1), 1)] {
            captures
                .start_capture_unchecked(
                    capture,
                    owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(unit)])),
                    [],
                )
                .unwrap();
            captures
                .observe_decision(capture, CodedCaptureDecision::Accepted)
                .unwrap();
            captures.request_clean(capture).unwrap();
            captures
                .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
                .unwrap();
        }

        let later = token(1);
        captures.observe_admitted_claim_unchecked(later, &units([0]));
        assert!(!captures.effect_allowed(later));
        assert_eq!(
            captures
                .capture_snapshot(CodedCaptureId(1))
                .unwrap()
                .membership
                .get(&later),
            None,
            "disjoint active capture must not classify or gate the operation"
        );

        captures
            .observe_later_cut(
                CodedCaptureId(0),
                later,
                later_cut(),
                CodedLaterCutObservation::DurableAfterClean,
            )
            .unwrap();
        assert!(
            captures.effect_allowed(later),
            "only applicable capture obligations should gate the effect"
        );
    }

    #[test]
    fn unknown_clean_and_later_cut_remain_distinct() {
        let mut captures = CodedCaptureCoordinator::new();
        let capture = CodedCaptureId(0);
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [(token(0), units([0]))],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Unknown)
            .unwrap();
        captures.observe_admitted_claim_unchecked(token(1), &units([0]));
        assert!(!captures.effect_allowed(token(1)));
        captures
            .observe_later_cut(
                capture,
                token(1),
                later_cut(),
                CodedLaterCutObservation::Unknown,
            )
            .unwrap();
        captures
            .reconcile_later_cut_unchecked(
                capture,
                token(1),
                later_cut(),
                CodedLaterCutReconciliation::Rejected,
            )
            .unwrap();
        assert!(!captures.effect_allowed(token(1)));
        assert_eq!(
            captures.capture_snapshot(capture).unwrap().phase,
            CodedCapturePhase::CleanCommitUnknown
        );
    }
}
