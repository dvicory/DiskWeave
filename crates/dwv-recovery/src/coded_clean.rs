use dwv_core::CodedUnitId;
use dwv_store::OperationSlotToken;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CodedCaptureId(pub u64);

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
    pub scope: BTreeSet<CodedUnitId>,
    pub membership: BTreeMap<OperationSlotToken, CodedCaptureMembership>,
    pub decision: Option<CodedCaptureDecision>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodedCaptureError {
    EmptyScope,
    IncompleteScope,
    UnvalidatedScope,
    LowerFrontierUncovered,
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
    LaterCutOrder(CodedCaptureId),
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
            Self::LaterCutOrder(capture) => {
                write!(
                    formatter,
                    "later cut ordering is invalid for capture {capture:?}"
                )
            }
        }
    }
}

impl std::error::Error for CodedCaptureError {}

#[derive(Clone, Debug)]
struct CaptureRecord {
    phase: CodedCapturePhase,
    scope: BTreeSet<CodedUnitId>,
    membership: BTreeMap<OperationSlotToken, CodedCaptureMembership>,
    decision: Option<CodedCaptureDecision>,
}

impl CaptureRecord {
    fn snapshot(&self) -> CodedCaptureSnapshot {
        CodedCaptureSnapshot {
            phase: self.phase,
            scope: self.scope.clone(),
            membership: self.membership.clone(),
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

    /// Start a bounded capture from an externally validated complete scope.
    pub fn start_capture(
        &mut self,
        capture: CodedCaptureId,
        scope: CodedCaptureScopeInput,
        active_claims: impl IntoIterator<Item = (OperationSlotToken, BTreeSet<CodedUnitId>)>,
    ) -> Result<(), CodedCaptureError> {
        if self.captures.contains_key(&capture) {
            return Err(CodedCaptureError::CaptureAlreadyExists(capture));
        }
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
        self.captures.insert(
            capture,
            CaptureRecord {
                phase: CodedCapturePhase::Open,
                scope: scope.units,
                membership,
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

    /// Return whether all active captures permit the operation's effect.
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

    pub fn capture_snapshot(&self, capture: CodedCaptureId) -> Option<CodedCaptureSnapshot> {
        self.captures.get(&capture).map(CaptureRecord::snapshot)
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
        observation: CodedLaterCutObservation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if !record.tracks_future() {
            return Err(CodedCaptureError::CaptureNotOpen(capture));
        }
        if record.membership.get(&operation) != Some(&CodedCaptureMembership::Later) {
            return Err(CodedCaptureError::LaterOperationNotMember { capture, operation });
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
        record.membership.insert(
            operation,
            match observation {
                CodedLaterCutObservation::DurableAfterClean => {
                    CodedCaptureMembership::LaterDurableAfterClean
                }
                CodedLaterCutObservation::DurableStalesClean => {
                    record.phase = CodedCapturePhase::Refused;
                    CodedCaptureMembership::LaterDurableStalesClean
                }
                CodedLaterCutObservation::Rejected => CodedCaptureMembership::LaterRejected,
                CodedLaterCutObservation::Unknown => CodedCaptureMembership::LaterUnknown,
            },
        );
        Ok(())
    }

    pub fn reconcile_later_cut(
        &mut self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        observation: CodedLaterCutReconciliation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.membership.get(&operation) != Some(&CodedCaptureMembership::LaterUnknown) {
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
        units.into_iter().map(CodedUnitId).collect()
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
                captures.start_capture(CodedCaptureId(0), scope, []),
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
                CodedCaptureScopeInput::complete([CodedUnitId(0), CodedUnitId(1)]),
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
    fn rejected_capture_does_not_classify_future_admissions() {
        let mut captures = CodedCaptureCoordinator::new();
        let capture = CodedCaptureId(0);
        captures
            .start_capture(
                capture,
                CodedCaptureScopeInput::complete([CodedUnitId(0)]),
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
    }

    #[test]
    fn unknown_clean_and_later_cut_remain_distinct() {
        let mut captures = CodedCaptureCoordinator::new();
        let capture = CodedCaptureId(0);
        captures
            .start_capture(
                capture,
                CodedCaptureScopeInput::complete([CodedUnitId(0)]),
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
            .observe_later_cut(capture, token(1), CodedLaterCutObservation::Unknown)
            .unwrap();
        captures
            .reconcile_later_cut(capture, token(1), CodedLaterCutReconciliation::Rejected)
            .unwrap();
        assert!(!captures.effect_allowed(token(1)));
        assert_eq!(
            captures.capture_snapshot(capture).unwrap().phase,
            CodedCapturePhase::CleanCommitUnknown
        );
    }
}
