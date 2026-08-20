/**
 * Verification-only canonical-init prefix profile. Through depth 4 it checks
 * admission, capture, the single external CLEAN decision, commit prefixes,
 * and invariants. CleanCommitUnknown and LaterCutUnknown are both reachable
 * from init at this bound; deeper reconciliation is covered by canonical
 * prefix profiles, not an assumed state.
 */
module CodedRangeCleanUncertaintyExhaustive {
  import CodedRangeClean(
    Operations = Set("opA", "opB"),
    Captures = Set("capture0"),
    CodedUnits = Set("u0")
  ).* from "../../models/quint/CodedRangeClean"

  val ClaimU0: CodedClaim = {
    units: Set("u0"),
    complete: true,
    validated: true,
  }

  val ScopeU0: CaptureScope = {
    units: Set("u0"),
    complete: true,
    validated: true,
    lowerFrontierCovered: true,
  }

  val cleanCommitUnknownAtCompletedBoundReachable: bool =
    state.captures.get("capture0").phase == CaptureCommitUnknown

  val laterCutUnknownAtCompletedBoundReachable: bool =
    state.captures.get("capture0").membership.get("opA") == CaptureLaterUnknown

  val ProfileInvariants: bool =
    TypeInvariant
    and NoHeldCodedOverlap
    and CompleteAdmissionPrecedesEffect
    and CaptureMembershipExhaustive
    and CaptureMembershipExclusive
    and RejectedSatisfactionRefusesCapture
    and NoFalseClean
    and UnknownCleanRequiresReconciliation
    and LaterEffectRequiresDurableCut

  action step: bool = any {
    admitComplete("opA", ClaimU0),
    admitComplete("opB", ClaimU0),
    startCapture("capture0", ScopeU0),
    observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted),
    observeCaptureSatisfaction("capture0", CaptureSatisfactionRejected),
    requestClean("capture0"),
    observeCleanCommit("capture0", CleanCommitUnknown),
    observeCleanCommit("capture0", CleanCommitRejected),
    reconcileCleanCommit("capture0", CleanReconciledDurable),
    reconcileCleanCommit("capture0", CleanReconciledRejected),
    observeLaterCut("capture0", "opA", LaterCutUnknown),
    reconcileLaterCut("capture0", "opA", LaterCutReconciledDurableAfterClean),
    reconcileLaterCut("capture0", "opA", LaterCutReconciledStalesClean),
    reconcileLaterCut("capture0", "opA", LaterCutReconciledRejected),
    permitMutationEffect("opA"),
    permitMutationEffect("opB"),
    removeCodedClaim("opA", ReleaseAllowed),
    removeCodedClaim("opB", ReleaseAllowed),
  }
}
