/**
 * Deterministic replay hook for the finite CodedRangeClean instance.
 * Seed and ITF paths are evidence configuration, not product semantics.
 */
module CodedRangeCleanReplay {
  import CodedRangeClean(
    Operations = Set("opA", "opB", "opC"),
    Captures = Set("capture0", "capture1"),
    CodedUnits = Set("u0", "u1")
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

  action operationStep: bool = {
    nondet operationId = Operations.oneOf()
    any {
      admitComplete(operationId, ClaimU0),
      permitMutationEffect(operationId),
      removeCodedClaim(operationId, ReleaseAllowed),
    }
  }

  action captureStep: bool = {
    nondet captureId = Captures.oneOf()
    any {
      startCapture(captureId, ScopeU0),
      requestClean(captureId),
      observeCleanCommit(captureId, CleanCommitDurable),
      observeCleanCommit(captureId, CleanCommitRejected),
      observeCleanCommit(captureId, CleanCommitUnknown),
      reconcileCleanCommit(captureId, CleanReconciledDurable),
      reconcileCleanCommit(captureId, CleanReconciledRejected),
    }
  }

  action satisfactionStep: bool = {
    nondet captureId = Captures.oneOf()
    any {
      observeCaptureSatisfaction(captureId, CaptureSatisfactionAccepted),
      observeCaptureSatisfaction(captureId, CaptureSatisfactionRejected),
    }
  }

  action laterCutStep: bool = {
    nondet captureId = Captures.oneOf()
    nondet operationId = Operations.oneOf()
    any {
      observeLaterCut(captureId, operationId, LaterCutDurableAfterClean),
      observeLaterCut(captureId, operationId, LaterCutDurableStalesClean),
      observeLaterCut(captureId, operationId, LaterCutRejected),
      observeLaterCut(captureId, operationId, LaterCutUnknown),
      reconcileLaterCut(captureId, operationId, LaterCutReconciledDurableAfterClean),
      reconcileLaterCut(captureId, operationId, LaterCutReconciledStalesClean),
      reconcileLaterCut(captureId, operationId, LaterCutReconciledRejected),
    }
  }

  // Initialization is setup, not an ordinary replay transition.
  action step: bool = any {
    operationStep,
    captureStep,
    satisfactionStep,
    laterCutStep,
  }

  run replaySeededPath =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .expect(
        state.captures.get("capture0").phase == CaptureCleanKnown
      )
}
