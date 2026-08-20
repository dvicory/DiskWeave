/**
 * Deliberately bad transition variants for compact CodedRangeClean ownership.
 * Each run is expected to fail its named invariant; a failed expectation is a
 * green negative-evidence result.
 */
module CodedRangeCleanMutants {
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

  val ClaimU1: CodedClaim = {
    units: Set("u1"),
    complete: true,
    validated: true,
  }

  val ScopeU0: CaptureScope = {
    units: Set("u0"),
    complete: true,
    validated: true,
    lowerFrontierCovered: true,
  }

  pure def held(claim: CodedClaim): OperationState = {
    phase: Held,
    claim: claim,
  }

  action mutantPerMemberConflict: bool = all {
    state.operations.get("opA").phase == Held,
    state.operations.get("opB").phase == Unadmitted,
    state' = {
      ...state,
      operations: state.operations.put("opB", held(ClaimU0)),
    },
  }

  action mutantConnectedComponentSerialization: bool = all {
    state.operations.get("opA").phase == Held,
    state.operations.get("opC").phase == Held,
    state' = {
      ...state,
      operations: state.operations.put("opC", {
        ...state.operations.get("opC"),
        phase: Unadmitted,
      }),
    },
  }

  action mutantPartialAdmissionPermitsEffect: bool = all {
    state.operations.get("opA").phase == Unadmitted,
    state' = {
      ...state,
      operations: state.operations.put("opA", {
        phase: EffectPossible,
        claim: ClaimU0,
      }),
    },
  }

  action mutantAdmissionBelongsToNeitherCaptureSide: bool = all {
    state.operations.get("opA").phase == Held,
    state.captures.get("capture0").phase == CaptureOpen,
    state' = {
      ...state,
      operations: state.operations.put("opB", held(ClaimU0)),
    },
  }

  action mutantIncludedOperationDisappears: bool = all {
    state.captures.get("capture0").membership.get("opA") == CaptureIncluded,
    state' = {
      ...state,
      captures: state.captures.put("capture0", {
        ...state.captures.get("capture0"),
        membership: state.captures.get("capture0").membership.put("opA", CaptureNotMember),
      }),
    },
  }

  action mutantLaterEffectWithoutDurableCut: bool = all {
    state.captures.get("capture0").membership.get("opB") == CaptureLater,
    state' = {
      ...state,
      operations: state.operations.put("opB", {
        ...state.operations.get("opB"),
        phase: EffectPossible,
      }),
    },
  }

  action mutantNoDecisionClean: bool = all {
    state.captures.get("capture0").phase == CaptureOpen,
    state' = {
      ...state,
      captures: state.captures.put("capture0", {
        ...state.captures.get("capture0"),
        phase: CaptureCleanKnown,
      }),
    },
  }

  action mutantRejectedSatisfactionStaysOpen: bool = all {
    state.captures.get("capture0").phase == CaptureOpen,
    state' = {
      ...state,
      captures: state.captures.put("capture0", {
        ...state.captures.get("capture0"),
        satisfaction: CaptureSatisfactionRejected,
      }),
    },
  }

  action mutantUnknownCleanBecomesKnown: bool = all {
    state.captures.get("capture0").phase == CaptureCommitUnknown,
    state' = {
      ...state,
      captures: state.captures.put("capture0", {
        ...state.captures.get("capture0"),
        phase: CaptureCleanKnown,
      }),
    },
  }

  action mutantUnknownLaterCutAllowsEffect: bool = all {
    state.captures.get("capture0").membership.get("opB") == CaptureLaterUnknown,
    state.operations.get("opB").phase == Held,
    state' = {
      ...state,
      operations: state.operations.put("opB", {
        ...state.operations.get("opB"),
        phase: EffectPossible,
      }),
    },
  }

  action mutantStaleCaptureCommits: bool = all {
    state.captures.get("capture0").phase == CaptureRefused,
    state' = {
      ...state,
      captures: state.captures.put("capture0", {
        ...state.captures.get("capture0"),
        phase: CaptureCleanKnown,
        membership: state.captures.get("capture0").membership.put("opB", CaptureLaterDurableStalesClean),
      }),
    },
  }

  action mutantRemoveWithoutExternalReleaseAllowed: bool = all {
    state.operations.get("opA").phase == Held,
    state' = {
      ...state,
      operations: state.operations.put("opA", {
        ...state.operations.get("opA"),
        phase: Released,
      }),
    },
  }

  run perMemberInsteadOfCodedConflictFails =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(mutantPerMemberConflict)
      .expect(NoHeldCodedOverlap)

  run connectedComponentSerializationKillsDisjointWitness =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(admitComplete("opC", ClaimU1))
      .then(mutantConnectedComponentSerialization)
    .expect(state.operations.get("opC").phase == Held)

  run partialAdmissionBeforeEffectFails =
    init
      .then(mutantPartialAdmissionPermitsEffect)
      .expect(state.operations.get("opA").phase == Held)

  run unclassifiedAdmissionFailsExhaustiveness =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(mutantAdmissionBelongsToNeitherCaptureSide)
      .expect(CaptureMembershipExhaustive)

  run disappearingIncludedOperationFailsRetention =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(mutantIncludedOperationDisappears)
      .expect(CaptureMembershipExhaustive)

  run laterEffectWithoutCutFails =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(mutantLaterEffectWithoutDurableCut)
      .expect(LaterEffectRequiresDurableCut)

  run unsatisfiedIncludedWorkCannotBeClean =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(mutantNoDecisionClean)
      .expect(NoFalseClean)

  run rejectedSatisfactionCannotRemainActive =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(mutantRejectedSatisfactionStaysOpen)
      .expect(RejectedSatisfactionRefusesCapture)

  run unknownCleanCannotBecomeKnown =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitUnknown))
      .then(mutantUnknownCleanBecomesKnown)
      .expect(state.captures.get("capture0").phase == CaptureCommitUnknown)

  run unknownLaterCutCannotAllowEffect =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutUnknown))
      .then(mutantUnknownLaterCutAllowsEffect)
      .expect(LaterEffectRequiresDurableCut)

  run staleCaptureCannotRemainClean =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutDurableStalesClean))
      .then(mutantStaleCaptureCommits)
      .expect(NoFalseClean)

  run removalWithoutExternalReleaseAllowedFails =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(mutantRemoveWithoutExternalReleaseAllowed)
      .expect(state.operations.get("opA").phase == Held)
}
