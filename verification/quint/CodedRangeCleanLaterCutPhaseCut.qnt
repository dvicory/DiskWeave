/**
 * Verification-only canonical-prefix profile for a later-boundary Unknown
 * after an already durable CLEAN. init/action transitions establish the full
 * prefix; no post-Unknown state is assumed. Durable-after-CLEAN and rejected
 * reconciliation both preserve the already durable CLEAN.
 */
module CodedRangeCleanLaterCutPhaseCut {
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

  action phaseStep: bool = any {
    admitComplete("opA", ClaimU0),
    startCapture("capture0", ScopeU0),
    observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted),
    requestClean("capture0"),
    observeCleanCommit("capture0", CleanCommitDurable),
    removeCodedClaim("opA", ReleaseAllowed),
    admitComplete("opB", ClaimU0),
    observeLaterCut("capture0", "opB", LaterCutUnknown),
    reconcileLaterCut("capture0", "opB", LaterCutReconciledDurableAfterClean),
    reconcileLaterCut("capture0", "opB", LaterCutReconciledRejected),
    permitMutationEffect("opB"),
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

  val DurableAfterCleanReconciliationReachable: bool =
    state.captures.get("capture0").phase == CaptureCleanKnown
    and state.captures.get("capture0").membership.get("opB")
      == CaptureLaterDurableAfterClean

  val RejectedReconciliationReachable: bool =
    state.captures.get("capture0").phase == CaptureCleanKnown
    and state.captures.get("capture0").membership.get("opB")
      == CaptureLaterRejected

  run durableAfterCleanLaterCutPhaseCut =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutUnknown))
      .then(reconcileLaterCut(
        "capture0",
        "opB",
        LaterCutReconciledDurableAfterClean,
      ))
      .then(permitMutationEffect("opB"))
      .expect(DurableAfterCleanReconciliationReachable)

  run rejectedLaterCutPhaseCut =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutUnknown))
      .then(reconcileLaterCut(
        "capture0",
        "opB",
        LaterCutReconciledRejected,
      ))
      .expect(RejectedReconciliationReachable)
}
