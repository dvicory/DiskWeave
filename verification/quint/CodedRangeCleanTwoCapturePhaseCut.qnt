/**
 * Deliberately small bounded phase-cut profile for independent capture
 * gates. It checks one fixed 14-step phase cut with two overlapping
 * active captures; it does not widen the canonical model or claim arbitrary
 * capture counts.
 */
module CodedRangeCleanTwoCapturePhaseCut {
  import CodedRangeClean(
    Operations = Set("opA", "opB"),
    Captures = Set("capture0", "capture1"),
    CodedUnits = Set("u0")
  ).* from "../../models/quint/CodedRangeClean"

  var phase: int

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

  action profileInit: bool = all {
    phase' = 0,
    init,
  }

  action step: bool = any {
    all { phase == 0, phase' = 1, admitComplete("opA", ClaimU0) },
    all { phase == 1, phase' = 2, startCapture("capture0", ScopeU0) },
    all { phase == 2, phase' = 3, startCapture("capture1", ScopeU0) },
    all { phase == 3, phase' = 4, observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted) },
    all { phase == 4, phase' = 5, observeCaptureSatisfaction("capture1", CaptureSatisfactionAccepted) },
    all { phase == 5, phase' = 6, requestClean("capture0") },
    all { phase == 6, phase' = 7, requestClean("capture1") },
    all { phase == 7, phase' = 8, observeCleanCommit("capture0", CleanCommitDurable) },
    all { phase == 8, phase' = 9, observeCleanCommit("capture1", CleanCommitDurable) },
    all { phase == 9, phase' = 10, removeCodedClaim("opA", ReleaseAllowed) },
    all { phase == 10, phase' = 11, admitComplete("opB", ClaimU0) },
    all { phase == 11, phase' = 12, observeLaterCut("capture0", "opB", LaterCutDurableAfterClean) },
    all { phase == 12, phase' = 13, observeLaterCut("capture1", "opB", LaterCutDurableAfterClean) },
    all { phase == 13, phase' = 14, permitMutationEffect("opB") },
  }

  val BothApplicableCaptureCutsGateEffect: bool =
    state.operations.get("opB").phase != EffectPossible
    or not(
      captureTracksFutureAdmission(state.captures.get("capture0"))
      and captureMembershipIsLater(state.captures.get("capture0"), "opB")
      and captureTracksFutureAdmission(state.captures.get("capture1"))
      and captureMembershipIsLater(state.captures.get("capture1"), "opB")
    )
    or (
      captureLaterCutSatisfiesEffect(
        state.captures.get("capture0").membership.get("opB")
      )
      and captureLaterCutSatisfiesEffect(
        state.captures.get("capture1").membership.get("opB")
      )
    )

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
    and BothApplicableCaptureCutsGateEffect

  run bothApplicableCaptureCutsPermit =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(startCapture("capture1", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(observeCaptureSatisfaction("capture1", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(requestClean("capture1"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(observeCleanCommit("capture1", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutDurableAfterClean))
      .then(observeLaterCut("capture1", "opB", LaterCutDurableAfterClean))
      .then(permitMutationEffect("opB"))
      .expect(
        state.operations.get("opB").phase == EffectPossible
        and state.captures.get("capture0").membership.get("opB")
          == CaptureLaterDurableAfterClean
        and state.captures.get("capture1").membership.get("opB")
          == CaptureLaterDurableAfterClean
      )

  run noCaptureCutDoesNotPermit =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(startCapture("capture1", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(observeCaptureSatisfaction("capture1", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(requestClean("capture1"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(observeCleanCommit("capture1", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(permitMutationEffect("opB").fail())

  run oneCaptureCutDoesNotPermit =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(startCapture("capture1", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(observeCaptureSatisfaction("capture1", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(requestClean("capture1"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(observeCleanCommit("capture1", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .expect(
        state.operations.get("opB").phase == Held
        and state.captures.get("capture0").membership.get("opB")
          == CaptureLater
        and state.captures.get("capture1").membership.get("opB")
          == CaptureLater
      )
      .then(observeLaterCut("capture0", "opB", LaterCutDurableAfterClean))
      .expect(
        state.operations.get("opB").phase == Held
        and state.captures.get("capture0").membership.get("opB")
          == CaptureLaterDurableAfterClean
        and state.captures.get("capture1").membership.get("opB")
          == CaptureLater
      )
      .then(permitMutationEffect("opB").fail())
}
