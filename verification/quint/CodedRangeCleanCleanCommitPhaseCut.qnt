/**
 * Verification-only canonical-prefix profile for CLEAN-commit Unknown.
 * No state is assumed: init reaches the complete capture prefix before the
 * Unknown observation, then both authoritative reconciliation outcomes.
 */
module CodedRangeCleanCleanCommitPhaseCut {
  import CodedRangeClean(
    Operations = Set("opA"),
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
    observeCleanCommit("capture0", CleanCommitUnknown),
    reconcileCleanCommit("capture0", CleanReconciledDurable),
    reconcileCleanCommit("capture0", CleanReconciledRejected),
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

  val DurableReconciliationReachable: bool =
    state.captures.get("capture0").phase == CaptureCleanKnown

  val RejectedReconciliationReachable: bool =
    state.captures.get("capture0").phase == CaptureRefused

  run durableCommitPhaseCut =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitUnknown))
      .then(reconcileCleanCommit("capture0", CleanReconciledDurable))
      .expect(DurableReconciliationReachable)

  run rejectedCommitPhaseCut =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitUnknown))
      .then(reconcileCleanCommit("capture0", CleanReconciledRejected))
      .expect(RejectedReconciliationReachable)
}
