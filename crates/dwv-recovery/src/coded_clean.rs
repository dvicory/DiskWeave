use crate::{
    ChecksumProfileId, ChecksumSetGeneration, IntegrityExtentId, RecoveryGeneration, RegionId,
};
use dwv_core::{CodedUnitId, TopologyEpoch, TopologySnapshot};
use dwv_store::OperationSlotToken;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

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
    pub recovery_generation: RecoveryGeneration,
    pub admission_sequence: u64,
}

impl CodedCaptureFrontier {
    pub const fn new(recovery_generation: RecoveryGeneration, admission_sequence: u64) -> Self {
        Self {
            recovery_generation,
            admission_sequence,
        }
    }
}
/// Owner-approved durable boundary for a mutation admitted after capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodedCaptureCut {
    pub topology_epoch: TopologyEpoch,
    pub recovery_generation: RecoveryGeneration,
    pub frontier: CodedCaptureFrontier,
}

impl CodedCaptureCut {
    pub const fn new(
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
}
/// Owner-approved evidence that exact resolved membership may be forgotten
/// through a bounded frontier. Every current operation identity must be listed
/// in exactly one of `releasable_operations` or `retained_operations`, and
/// every releasable identity must carry an external
/// `ReleaseAllowed(operation-generation)` fact from the lifecycle owner. The
/// coordinator does not infer historicality from capture-local membership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedCaptureRetentionSummary {
    pub capture: CodedCaptureId,
    pub topology_epoch: TopologyEpoch,
    pub frontier: CodedCaptureFrontier,
    pub releasable_operations: BTreeSet<OperationSlotToken>,
    pub retained_operations: BTreeSet<OperationSlotToken>,
    pub release_authorized_operations: BTreeSet<OperationSlotToken>,
}

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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodedCaptureRetirement {
    pub capture: CodedCaptureId,
    pub topology_epoch: TopologyEpoch,
    pub frontier: CodedCaptureFrontier,
}

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
    pub scope: CodedCaptureScopeInput,
    pub dirty_regions: BTreeSet<RegionId>,
    pub checksum_extents: BTreeSet<IntegrityExtentId>,
    pub lower_frontier: CodedCaptureFrontier,
    pub capture_frontier: CodedCaptureFrontier,
}

impl CodedCaptureOwnerFacts {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        topology: TopologySnapshot,
        recovery_generation: RecoveryGeneration,
        checksum_profile: ChecksumProfileId,
        checksum_set_generation: ChecksumSetGeneration,
        scope: CodedCaptureScopeInput,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedCaptureScopeInput {
    units: BTreeSet<CodedUnitId>,
    complete: bool,
    validated: bool,
    lower_frontier_covered: bool,
}

impl CodedCaptureScopeInput {
    pub fn new(
        units: impl IntoIterator<Item = CodedUnitId>,
        complete: bool,
        validated: bool,
        lower_frontier_covered: bool,
    ) -> Self {
        Self {
            units: units.into_iter().collect(),
            complete,
            validated,
            lower_frontier_covered,
        }
    }

    pub fn complete(units: impl IntoIterator<Item = CodedUnitId>) -> Self {
        Self::new(units, true, true, true)
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

    pub const fn lower_frontier_covered(&self) -> bool {
        self.lower_frontier_covered
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedCapturePhase {
    Open,
    CleanCommitPending,
    CleanCommitUnknown,
    CleanKnown,
    Refused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedCaptureMembership {
    Included,
    Later,
    LaterDurableAfterClean,
    LaterDurableStalesClean,
    LaterRejected,
    LaterUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub membership: BTreeMap<OperationSlotToken, CodedCaptureMembership>,
    /// Exact post-capture cut retained for each unresolved `LaterUnknown`.
    pub pending_later_cuts: BTreeMap<OperationSlotToken, CodedCaptureCut>,
    pub decision: Option<CodedCaptureDecision>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodedCaptureError {
    EmptyScope,
    IncompleteScope,
    UnvalidatedScope,
    LowerFrontierUncovered,
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
    retained_frontier: CodedCaptureFrontier,
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
            scope_complete: self.owner_facts.scope.is_complete(),
            scope_validated: self.owner_facts.scope.is_validated(),
            lower_frontier_covered: self.owner_facts.scope.lower_frontier_covered(),
            dirty_regions: self.owner_facts.dirty_regions.clone(),
            checksum_extents: self.owner_facts.checksum_extents.clone(),
            lower_frontier: self.owner_facts.lower_frontier,
            capture_frontier: self.owner_facts.capture_frontier,
            retained_frontier: self.retained_frontier,
            membership: self.membership.clone(),
            pending_later_cuts: self.pending_later_cuts.clone(),
            decision: self.decision,
        }
    }

    fn intersects(&self, units: &BTreeSet<CodedUnitId>) -> bool {
        self.scope.iter().any(|unit| units.contains(unit))
    }

    fn tracks_future(&self) -> bool {
        !matches!(self.phase, CodedCapturePhase::Refused)
    }

    fn clean_eligible(&self) -> bool {
        !self.scope.is_empty()
            && self.decision == Some(CodedCaptureDecision::Accepted)
            && !self.membership.values().any(|membership| {
                matches!(
                    membership,
                    CodedCaptureMembership::LaterDurableStalesClean
                        | CodedCaptureMembership::LaterUnknown
                )
            })
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
            CodedCaptureMembership::Included => self.decision.is_some(),
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
#[derive(Default)]
pub struct CodedCaptureCoordinator {
    captures: BTreeMap<CodedCaptureId, CaptureRecord>,
}

impl CodedCaptureCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a bounded capture from exact owner facts and a complete scope.
    pub fn start_capture(
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
        if !scope.complete {
            return Err(CodedCaptureError::IncompleteScope);
        }
        if !scope.validated {
            return Err(CodedCaptureError::UnvalidatedScope);
        }
        if !scope.lower_frontier_covered {
            return Err(CodedCaptureError::LowerFrontierUncovered);
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
                retained_frontier,
                decision: None,
            },
        );
        Ok(())
    }

    /// Observe a later admitted claim against every active capture.
    pub fn observe_admitted_claim(
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

    /// Consume an owner-approved summary and forget only its exact resolved
    /// operation identities. Live or unresolved membership remains retained.
    pub fn compact_capture(
        &mut self,
        summary: CodedCaptureRetentionSummary,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(summary.capture)?;
        if summary.topology_epoch != record.owner_facts.topology.topology_epoch()
            || summary.release_authorized_operations != summary.releasable_operations
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
        }
        record.retained_frontier = summary.frontier;
        Ok(())
    }

    /// Retire a refused or fully resolved capture after owner-approved
    /// retention evidence has removed every exact obligation.
    pub fn retire_capture(
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
        if !matches!(
            record.phase,
            CodedCapturePhase::Refused | CodedCapturePhase::CleanKnown
        ) || !record.membership.is_empty()
            || !record.pending_later_cuts.is_empty()
        {
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

    pub fn observe_later_cut(
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
                CodedCaptureMembership::LaterDurableAfterClean
            }
            CodedLaterCutObservation::DurableStalesClean => {
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

    pub fn reconcile_later_cut(
        &mut self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        cut: CodedCaptureCut,
        observation: CodedLaterCutReconciliation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.membership.get(&operation) != Some(&CodedCaptureMembership::LaterUnknown) {
            return Err(CodedCaptureError::LaterCutNotPending { capture, operation });
        }
        if record.pending_later_cuts.get(&operation).copied() != Some(cut) {
            return Err(CodedCaptureError::LaterCutMismatch { capture, operation });
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
        record.pending_later_cuts.remove(&operation);
        record.membership.insert(
            operation,
            match observation {
                CodedLaterCutReconciliation::DurableAfterClean => {
                    CodedCaptureMembership::LaterDurableAfterClean
                }
                CodedLaterCutReconciliation::StalesClean => {
                    record.phase = CodedCapturePhase::Refused;
                    CodedCaptureMembership::LaterDurableStalesClean
                }
                CodedLaterCutReconciliation::Rejected => CodedCaptureMembership::LaterRejected,
            },
        );
        Ok(())
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

    fn token(index: u32) -> OperationSlotToken {
        OperationSlotToken::new(index, 1)
    }

    fn units(units: impl IntoIterator<Item = u32>) -> BTreeSet<CodedUnitId> {
        units
            .into_iter()
            .map(|unit| CodedUnitId(u64::from(unit)))
            .collect()
    }
    fn owner_facts(scope: CodedCaptureScopeInput) -> CodedCaptureOwnerFacts {
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
    fn capture_rejects_incoherent_frontier_owner_facts() {
        let capture = CodedCaptureId(8);
        let scope = CodedCaptureScopeInput::complete([CodedUnitId(0)]);

        let mut mismatched_generation = owner_facts(scope.clone());
        mismatched_generation.capture_frontier =
            CodedCaptureFrontier::new(RecoveryGeneration(1), 1);
        let mut captures = CodedCaptureCoordinator::new();
        assert_eq!(
            captures.start_capture(capture, mismatched_generation, []),
            Err(CodedCaptureError::CaptureFrontierGenerationMismatch(
                capture
            ))
        );

        let mut reversed = owner_facts(scope);
        reversed.lower_frontier = CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 2);
        let mut captures = CodedCaptureCoordinator::new();
        assert_eq!(
            captures.start_capture(capture, reversed, []),
            Err(CodedCaptureError::FrontierOrderInvalid(capture))
        );
    }

    #[test]
    fn older_recovery_generation_is_not_a_newer_later_cut_even_with_a_larger_sequence() {
        let capture = CodedCaptureId(9);
        let operation = token(4);
        let mut facts = owner_facts(CodedCaptureScopeInput::complete([CodedUnitId(0)]));
        facts.recovery_generation = RecoveryGeneration(1);
        facts.lower_frontier = CodedCaptureFrontier::new(RecoveryGeneration(1), 0);
        facts.capture_frontier = CodedCaptureFrontier::new(RecoveryGeneration(1), 1);

        let mut captures = CodedCaptureCoordinator::new();
        captures.start_capture(capture, facts, []).unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
            .unwrap();
        captures.observe_admitted_claim(operation, &units([0]));

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
            .start_capture(
                capture,
                owner_facts(CodedCaptureScopeInput::complete([CodedUnitId(0)])),
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
            captures.observe_admitted_claim(operation, &units([0]));
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
            captures
                .compact_capture(
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
        captures.observe_admitted_claim(live, &units([0]));
        assert_eq!(
            captures.compact_capture(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                later_cut().frontier,
                [],
            )),
            Err(CodedCaptureError::RetentionSummaryInvalid(capture))
        );
        captures
            .compact_capture(
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
            captures.compact_capture(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                later_cut().frontier,
                [live],
            )),
            Err(CodedCaptureError::RetentionSummaryInvalid(capture))
        );
        assert!(!captures.effect_allowed(live));
        captures
            .compact_capture(
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
        captures.observe_admitted_claim(unresolved, &units([0]));
        captures
            .observe_later_cut(
                capture,
                unresolved,
                unknown_cut,
                CodedLaterCutObservation::Unknown,
            )
            .unwrap();
        assert_eq!(
            captures.compact_capture(CodedCaptureRetentionSummary::new(
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
            .compact_capture(
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
            captures.reconcile_later_cut(
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
            captures.reconcile_later_cut(
                capture,
                unresolved,
                unknown_cut,
                CodedLaterCutReconciliation::Rejected,
            ),
            Ok(())
        );

        assert_eq!(
            captures.compact_capture(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 2),
                [],
            )),
            Err(CodedCaptureError::RetentionFrontierStale(capture))
        );
        assert_eq!(
            captures.retire_capture(CodedCaptureRetirement::new(
                capture,
                dwv_core::TopologyEpoch(1),
                retirement_frontier,
            )),
            Err(CodedCaptureError::CaptureNotRetirable(capture))
        );

        captures
            .compact_capture(
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
            .retire_capture(CodedCaptureRetirement::new(
                capture,
                dwv_core::TopologyEpoch(1),
                retirement_frontier,
            ))
            .unwrap();
        assert!(captures.capture_snapshot(capture).is_none());
    }

    #[test]
    fn capture_requires_a_complete_validated_covered_scope() {
        let invalid_scopes = [
            (
                CodedCaptureScopeInput::new([], true, true, true),
                CodedCaptureError::EmptyScope,
            ),
            (
                CodedCaptureScopeInput::new([CodedUnitId(0)], false, true, true),
                CodedCaptureError::IncompleteScope,
            ),
            (
                CodedCaptureScopeInput::new([CodedUnitId(0)], true, false, true),
                CodedCaptureError::UnvalidatedScope,
            ),
            (
                CodedCaptureScopeInput::new([CodedUnitId(0)], true, true, false),
                CodedCaptureError::LowerFrontierUncovered,
            ),
        ];
        for (scope, expected) in invalid_scopes {
            let mut captures = CodedCaptureCoordinator::new();
            assert_eq!(
                captures.start_capture(CodedCaptureId(0), owner_facts(scope), []),
                Err(expected)
            );
            assert!(captures.capture_snapshot(CodedCaptureId(0)).is_none());
        }
    }

    #[test]
    fn capture_records_included_and_later_membership() {
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture(
                CodedCaptureId(0),
                owner_facts(CodedCaptureScopeInput::complete([
                    CodedUnitId(0),
                    CodedUnitId(1),
                ])),
                [(token(0), units([0]))],
            )
            .unwrap();
        captures.observe_admitted_claim(token(1), &units([1]));
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
            .start_capture(
                capture,
                owner_facts(CodedCaptureScopeInput::complete([CodedUnitId(0)])),
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
        captures.observe_admitted_claim(operation, &units([0]));

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
        captures
            .start_capture(
                capture,
                owner_facts(CodedCaptureScopeInput::complete([CodedUnitId(0)])),
                [],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Rejected)
            .unwrap();
        captures.observe_admitted_claim(token(0), &units([0]));
        assert!(
            captures
                .capture_snapshot(capture)
                .unwrap()
                .membership
                .is_empty()
        );
        assert!(captures.effect_allowed(token(0)));
        captures
            .retire_capture(CodedCaptureRetirement::new(
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
                .start_capture(
                    capture,
                    owner_facts(CodedCaptureScopeInput::complete([CodedUnitId(0)])),
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
        captures.observe_admitted_claim(later, &units([0]));
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
                .start_capture(
                    capture,
                    owner_facts(CodedCaptureScopeInput::complete([CodedUnitId(unit)])),
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
        captures.observe_admitted_claim(later, &units([0]));
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
            .start_capture(
                capture,
                owner_facts(CodedCaptureScopeInput::complete([CodedUnitId(0)])),
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
        captures.observe_admitted_claim(token(1), &units([0]));
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
            .reconcile_later_cut(
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
