/**
 * Verification-only canonical-prefix composition profile. It covers durable
 * CLEAN, external removal, a later durable cut, and rejected uncertainty.
 */
module CodedRangeCleanCompositionExhaustive {
  import CodedRangeClean(
    Operations = Set("opA", "opB"),
    Captures = Set("capture0"),
    CodedUnits = Set("u0", "u1")
  ).* from "../../models/quint/CodedRangeClean"

  val ClaimU0: CodedClaim = {
    units: Set("u0"),
    complete: true,
    validated: true,
  }

  val ClaimU1: CodedClaim = {
    units: Set("u1"),
    complete: true,
    validated: true,
  }

  val ScopeU0U1: CaptureScope = {
    units: Set("u0", "u1"),
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
    admitComplete("opB", ClaimU1),
    startCapture("capture0", ScopeU0U1),
    observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted),
    observeCaptureSatisfaction("capture0", CaptureSatisfactionRejected),
    requestClean("capture0"),
    observeCleanCommit("capture0", CleanCommitDurable),
    observeCleanCommit("capture0", CleanCommitUnknown),
    observeCleanCommit("capture0", CleanCommitRejected),
    reconcileCleanCommit("capture0", CleanReconciledDurable),
    reconcileCleanCommit("capture0", CleanReconciledRejected),
    observeLaterCut("capture0", "opB", LaterCutDurableAfterClean),
    observeLaterCut("capture0", "opB", LaterCutDurableStalesClean),
    observeLaterCut("capture0", "opB", LaterCutUnknown),
    reconcileLaterCut("capture0", "opB", LaterCutReconciledDurableAfterClean),
    reconcileLaterCut("capture0", "opB", LaterCutReconciledStalesClean),
    reconcileLaterCut("capture0", "opB", LaterCutReconciledRejected),
    permitMutationEffect("opB"),
    removeCodedClaim("opA", ReleaseAllowed),
    removeCodedClaim("opB", ReleaseAllowed),
  }

  val DurableCompositionReachable: bool =
    state.captures.get("capture0").phase == CaptureCleanKnown
    and state.operations.get("opB").phase == EffectPossible

  val RejectedCompositionReachable: bool =
    state.captures.get("capture0").phase == CaptureRefused
    and state.operations.get("opB").phase == EffectPossible

  run compositionDurablePath =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0U1))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU1))
      .then(observeLaterCut("capture0", "opB", LaterCutDurableAfterClean))
      .then(permitMutationEffect("opB"))
      .expect(DurableCompositionReachable)

  run compositionRejectedPath =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0U1))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitUnknown))
      .then(reconcileCleanCommit("capture0", CleanReconciledRejected))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU1))
      .then(permitMutationEffect("opB"))
      .expect(RejectedCompositionReachable)
}
