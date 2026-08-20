/**
 * Verification-only exhaustive profile for closed membership, one external
 * CLEAN-owner decision, durable/rejected cleanup outcomes, and later cuts.
 */
module CodedRangeCleanCaptureExhaustive {
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
    observeCleanCommit("capture0", CleanCommitDurable),
    observeCleanCommit("capture0", CleanCommitRejected),
    observeCleanCommit("capture0", CleanCommitUnknown),
    reconcileCleanCommit("capture0", CleanReconciledDurable),
    reconcileCleanCommit("capture0", CleanReconciledRejected),
    observeLaterCut("capture0", "opB", LaterCutDurableAfterClean),
    observeLaterCut("capture0", "opB", LaterCutDurableStalesClean),
    observeLaterCut("capture0", "opB", LaterCutRejected),
    observeLaterCut("capture0", "opB", LaterCutUnknown),
    reconcileLaterCut("capture0", "opB", LaterCutReconciledDurableAfterClean),
    reconcileLaterCut("capture0", "opB", LaterCutReconciledStalesClean),
    reconcileLaterCut("capture0", "opB", LaterCutReconciledRejected),
    permitMutationEffect("opA"),
    permitMutationEffect("opB"),
    removeCodedClaim("opA", ReleaseAllowed),
    removeCodedClaim("opB", ReleaseAllowed),
  }
}
