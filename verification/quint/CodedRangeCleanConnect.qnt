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
    all { pathPhase == 2, pathPhase' = 3, acceptCapture },
    all { pathPhase == 3, pathPhase' = 4, admitOpC },
    all { pathPhase == 4, pathPhase' = 5, requestCleanPath },
    all { pathPhase == 5, pathPhase' = 6, cleanDurable },
    all { pathPhase == 6, pathPhase' = 7, laterAfterClean },
    all { pathPhase == 7, pathPhase' = 8, permitOpC },
    all { pathPhase == 8, pathPhase' = 9, removeOpA },
  }

  action unknownCleanStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, acceptCapture },
    all { pathPhase == 3, pathPhase' = 4, requestCleanPath },
    all { pathPhase == 4, pathPhase' = 5, cleanUnknown },
    all { pathPhase == 5, pathPhase' = 6, removeOpA },
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
  }

  action staleLaterCutStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, acceptCapture },
    all { pathPhase == 3, pathPhase' = 4, admitOpC },
    all { pathPhase == 4, pathPhase' = 5, laterStalesClean },
    all { pathPhase == 5, pathPhase' = 6, permitOpC },
  }

  action laterRejectedStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, acceptCapture },
    all { pathPhase == 3, pathPhase' = 4, admitOpC },
    all { pathPhase == 4, pathPhase' = 5, laterRejectedObservation },
    all { pathPhase == 5, pathPhase' = 6, probeEffectAfterLaterRejection },

  }
  action preCleanUnknownStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, acceptCapture },
    all { pathPhase == 3, pathPhase' = 4, admitOpC },
    all { pathPhase == 4, pathPhase' = 5, laterUnknown },
    all { pathPhase == 5, pathPhase' = 6, probeCleanBeforeLaterReconciliation },
    all { pathPhase == 6, pathPhase' = 7, laterStalesCleanReconciled },
  }

  action nonTransitiveStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, admitOpC },
    all { pathPhase == 2, pathPhase' = 3, probeNonTransitive },
  }

  action cleanReconcileStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, acceptCapture },
    all { pathPhase == 3, pathPhase' = 4, requestCleanPath },
    all { pathPhase == 4, pathPhase' = 5, cleanUnknown },
    all { pathPhase == 5, pathPhase' = 6, cleanReconciledDurable },
    all { pathPhase == 6, pathPhase' = 7, removeOpA },
  }

  action cleanReconcileRejectedStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, acceptCapture },
    all { pathPhase == 3, pathPhase' = 4, requestCleanPath },
    all { pathPhase == 4, pathPhase' = 5, cleanUnknown },
    all { pathPhase == 5, pathPhase' = 6, cleanReconciledRejected },
  }

  action cleanRejectedStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, acceptCapture },
    all { pathPhase == 3, pathPhase' = 4, requestCleanPath },
    all { pathPhase == 4, pathPhase' = 5, cleanRejected },
  }

  action noCleanStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, probeCleanBeforeDecision },
  }

  action unknownAfterCleanReconcileStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, admitOpA },
    all { pathPhase == 1, pathPhase' = 2, startCapturePath },
    all { pathPhase == 2, pathPhase' = 3, acceptCapture },
    all { pathPhase == 3, pathPhase' = 4, requestCleanPath },
    all { pathPhase == 4, pathPhase' = 5, cleanDurable },
    all { pathPhase == 5, pathPhase' = 6, removeOpA },
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

  action removeOpC: bool = all {
    removeCodedClaim("opC", ReleaseAllowed),
  }
}
