/**
 * Bounded Connect profiles for the production coded-authority and CLEAN
 * capture seams. Claims, scope, CLEAN decisions, and commit/cut outcomes are
 * typed external owner inputs; the driver must observe coordinator results.
 */
module CodedRangeCleanConnect {
  import CodedRangeClean(
    Operations = Set("opA", "opB", "opC"),
    Captures = Set("capture0"),
    CodedUnits = Set("u0", "u1"),
  ).* from "../../models/quint/CodedRangeClean"

  pure val ClaimU0: CodedClaim = {
    units: Set("u0"),
    complete: true,
    validated: true,
  }

  pure val ClaimU1: CodedClaim = {
    units: Set("u1"),
    complete: true,
    validated: true,
  }

  pure val ClaimU01: CodedClaim = {
    units: Set("u0", "u1"),
    complete: true,
    validated: true,
  }

  pure val ScopeU01: CaptureScope = {
    units: Set("u0", "u1"),
    complete: true,
    validated: true,
    lowerFrontierCovered: true,
  }

  var pathPhase: int

  action connectInit: bool = all {
    pathPhase' = 0,
    init,
  }


  // Negative probes report rejected implementation facts without changing the
  // delegated relation state.
  action probeConflict: bool = all { state' = state }
  action probeIncompleteClaim: bool = all { state' = state }
  action probeUnauthorizedRemoval: bool = all { state' = state }
  action probeStaleGeneration: bool = all { state' = state }
  action probeEffectBeforeLaterCut: bool = all { state' = state }
  action probeEffectAfterLaterRejection: bool = all { state' = state }
  action probeRejectedFutureAdmission: bool = all { state' = state }
  action probeCleanBeforeLaterReconciliation: bool = all { state' = state }
  action probeNonTransitive: bool = all { state' = state }
  action probeUnvalidatedClaim: bool = all { state' = state }
  action probeCleanBeforeDecision: bool = all { state' = state }

  action admissionStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, admitOpC },
    all { pathPhase == 2, pathPhase' = 3, permitOpA },
    all { pathPhase == 3, pathPhase' = 4, permitOpC },
    all { pathPhase == 4, pathPhase' = 5, removeOpA },
    all { pathPhase == 5, pathPhase' = 6, removeOpC },
  }

  action wideClaimStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpB },
    all { pathPhase == 1, pathPhase' = 2, permitOpB },
    all { pathPhase == 2, pathPhase' = 3, removeOpB },
  }

  action captureStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, removeOpA },
    all { pathPhase == 3, pathPhase' = 4, acceptCapture },
    all { pathPhase == 4, pathPhase' = 5, admitOpC },
    all { pathPhase == 5, pathPhase' = 6, requestCleanPath },
    all { pathPhase == 6, pathPhase' = 7, cleanDurable },
    all { pathPhase == 7, pathPhase' = 8, laterAfterClean },
    all { pathPhase == 8, pathPhase' = 9, permitOpC },
    all { pathPhase == 9, pathPhase' = 10, compactOpA },
  }

  action unknownCleanStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, removeOpA },
    all { pathPhase == 3, pathPhase' = 4, acceptCapture },
    all { pathPhase == 4, pathPhase' = 5, requestCleanPath },
    all { pathPhase == 5, pathPhase' = 6, cleanUnknown },
    all { pathPhase == 6, pathPhase' = 7, admitOpC },
    all { pathPhase == 7, pathPhase' = 8, laterUnknown },
    all { pathPhase == 8, pathPhase' = 9, probeEffectBeforeLaterCut },
    all { pathPhase == 9, pathPhase' = 10, laterRejected },
    all { pathPhase == 10, pathPhase' = 11, probeEffectAfterLaterRejection },
  }

  action rejectedCaptureStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, startCapturePath },
    all { pathPhase == 1, pathPhase' = 2, rejectCapture },
    all { pathPhase == 2, pathPhase' = 3, probeRejectedFutureAdmission },
    all { pathPhase == 3, pathPhase' = 4, cleanupRefusedCapturePath },
  }

  // A quiescent last write reaches clean retirement without a later mutation.
  action cleanRetirementStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, removeOpA },
    all { pathPhase == 3, pathPhase' = 4, acceptCapture },
    all { pathPhase == 4, pathPhase' = 5, requestCleanPath },
    all { pathPhase == 5, pathPhase' = 6, cleanDurable },
    all { pathPhase == 6, pathPhase' = 7, compactOpA },
    all { pathPhase == 7, pathPhase' = 8, cleanupCleanCapturePath },
  }

  action staleLaterCutStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, removeOpA },
    all { pathPhase == 3, pathPhase' = 4, acceptCapture },
    all { pathPhase == 4, pathPhase' = 5, admitOpC },
    all { pathPhase == 5, pathPhase' = 6, laterStalesClean },
    all { pathPhase == 6, pathPhase' = 7, permitOpC },
  }

  action laterRejectedStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, removeOpA },
    all { pathPhase == 3, pathPhase' = 4, acceptCapture },
    all { pathPhase == 4, pathPhase' = 5, admitOpC },
    all { pathPhase == 5, pathPhase' = 6, laterRejectedObservation },
    all { pathPhase == 6, pathPhase' = 7, probeEffectAfterLaterRejection },
  }

  action preCleanUnknownStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, removeOpA },
    all { pathPhase == 3, pathPhase' = 4, acceptCapture },
    all { pathPhase == 4, pathPhase' = 5, admitOpC },
    all { pathPhase == 5, pathPhase' = 6, laterUnknown },
    all { pathPhase == 6, pathPhase' = 7, probeCleanBeforeLaterReconciliation },
    all { pathPhase == 7, pathPhase' = 8, laterStalesCleanReconciled },
  }

  action nonTransitiveStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, admitOpC },
    all { pathPhase == 2, pathPhase' = 3, probeNonTransitive },
  }

  action cleanReconcileStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, removeOpA },
    all { pathPhase == 3, pathPhase' = 4, acceptCapture },
    all { pathPhase == 4, pathPhase' = 5, requestCleanPath },
    all { pathPhase == 5, pathPhase' = 6, cleanUnknown },
    all { pathPhase == 6, pathPhase' = 7, cleanReconciledDurable },
    all { pathPhase == 7, pathPhase' = 8, compactOpA },
  }

  action cleanReconcileRejectedStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, removeOpA },
    all { pathPhase == 3, pathPhase' = 4, acceptCapture },
    all { pathPhase == 4, pathPhase' = 5, requestCleanPath },
    all { pathPhase == 5, pathPhase' = 6, cleanUnknown },
    all { pathPhase == 6, pathPhase' = 7, cleanReconciledRejected },
  }

  action cleanRejectedStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, removeOpA },
    all { pathPhase == 3, pathPhase' = 4, acceptCapture },
    all { pathPhase == 4, pathPhase' = 5, requestCleanPath },
    all { pathPhase == 5, pathPhase' = 6, cleanRejected },
  }

  action noCleanStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, probeCleanBeforeDecision },
  }

  action unknownAfterCleanReconcileStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, removeOpA },
    all { pathPhase == 3, pathPhase' = 4, acceptCapture },
    all { pathPhase == 4, pathPhase' = 5, requestCleanPath },
    all { pathPhase == 5, pathPhase' = 6, cleanDurable },
    all { pathPhase == 6, pathPhase' = 7, admitOpC },
    all { pathPhase == 7, pathPhase' = 8, laterUnknown },
    all { pathPhase == 8, pathPhase' = 9, laterAfterCleanReconciled },
    all { pathPhase == 9, pathPhase' = 10, permitOpC },
  }

  action negativeStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, probeIncompleteClaim },
    all { pathPhase == 1, pathPhase' = 2, probeUnvalidatedClaim },
    all { pathPhase == 2, pathPhase' = 3, admitOpA },
    all { pathPhase == 3, pathPhase' = 4, probeConflict },
    all { pathPhase == 4, pathPhase' = 5, probeUnauthorizedRemoval },
    all { pathPhase == 5, pathPhase' = 6, probeStaleGeneration },
  }

  // Sampled operation/capture coverage deliberately excludes CLEAN and later
  // cuts; those owner-ordering paths retain their explicit profiles below.
  // The runner's explicit max-steps bound limits sampled evidence; pathPhase
  // remains only a trace counter and does not constrain this action.
  action sampledStep: bool = {
    nondet choice = 0.to(9).oneOf()
    any {
      all { pathPhase' = pathPhase + 1, choice == 0, admitOpA },
      all { pathPhase' = pathPhase + 1, choice == 1, admitOpB },
      all { pathPhase' = pathPhase + 1, choice == 2, admitOpC },
      all { pathPhase' = pathPhase + 1, choice == 3, startCapturePath },
      all { pathPhase' = pathPhase + 1, choice == 4, permitOpA },
      all { pathPhase' = pathPhase + 1, choice == 5, permitOpB },
      all { pathPhase' = pathPhase + 1, choice == 6, permitOpC },
      all { pathPhase' = pathPhase + 1, choice == 7, removeOpA },
      all { pathPhase' = pathPhase + 1, choice == 8, removeOpB },
      all { pathPhase' = pathPhase + 1, choice == 9, removeOpC },
    }
  }


  action admitOpA: bool = all {
    admitComplete("opA", ClaimU0),
  }

  action admitOpB: bool = all {
    admitComplete("opB", ClaimU01),
  }

  action admitOpC: bool = all {
    admitComplete("opC", ClaimU1),
  }

  action startCapturePath: bool = all {
    startCapture("capture0", ScopeU01),
  }


  action acceptCapture: bool = all {
    observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted),
  }

  action rejectCapture: bool = all {
    observeCaptureSatisfaction("capture0", CaptureSatisfactionRejected),
  }

  action requestCleanPath: bool = all {
    requestClean("capture0"),
  }

  action cleanDurable: bool = all {
    observeCleanCommit("capture0", CleanCommitDurable),
  }

  action cleanUnknown: bool = all {
    observeCleanCommit("capture0", CleanCommitUnknown),
  }
  action cleanRejected: bool = all {
    observeCleanCommit("capture0", CleanCommitRejected),
  }

  action cleanReconciledDurable: bool = all {
    reconcileCleanCommit("capture0", CleanReconciledDurable),
  }
  action cleanReconciledRejected: bool = all {
    reconcileCleanCommit("capture0", CleanReconciledRejected),
  }

  action laterAfterClean: bool = all {
    observeLaterCut("capture0", "opC", LaterCutDurableAfterClean),
  }

  action laterStalesClean: bool = all {
    observeLaterCut("capture0", "opC", LaterCutDurableStalesClean),
  }
  action permitOpB: bool = all {
    permitMutationEffect("opB"),
  }

  action laterUnknown: bool = all {
    observeLaterCut("capture0", "opC", LaterCutUnknown),
  }

  action laterRejected: bool = all {
    reconcileLaterCut("capture0", "opC", LaterCutReconciledRejected),
  }
  action laterRejectedObservation: bool = all {
    observeLaterCut("capture0", "opC", LaterCutRejected),
  }

  action removeOpB: bool = all {
    removeCodedClaim("opB", ReleaseAllowed),
  }
  action laterStalesCleanReconciled: bool = all {
    reconcileLaterCut("capture0", "opC", LaterCutReconciledStalesClean),
  }

  action permitOpA: bool = all {
    permitMutationEffect("opA"),
  }

  action laterAfterCleanReconciled: bool = all {
    reconcileLaterCut("capture0", "opC", LaterCutReconciledDurableAfterClean),
  }
  action permitOpC: bool = all {
    permitMutationEffect("opC"),
  }

  action removeOpA: bool = all {
    removeCodedClaim("opA", ReleaseAllowed),
  }

  action compactOpA: bool = all {
    compactReleasedMembership(
      "capture0",
      "opA",
      MembershipCompactionAllowed,
    ),
  }

  action cleanupRefusedCapturePath: bool = all {
    cleanupResolvedCapture("capture0", RefusedCaptureCleanupAllowed),
  }

  action cleanupCleanCapturePath: bool = all {
    cleanupResolvedCapture("capture0", CleanKnownCaptureCleanupAllowed),
  }

  val cleanRetirementWithoutLaterMutationReached: bool =
    pathPhase == 8
    and state.operations.get("opA").phase == Released
    and state.captures.get("capture0").phase == CaptureAbsent

  action removeOpC: bool = all {
    removeCodedClaim("opC", ReleaseAllowed),
  }
}
