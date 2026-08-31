/**
 * Evidence-only larger finite profile for the composed CodedRangeClean
 * relation. Three operation identities, two captures, two coded units, and
 * all finite inputs are checker bounds only.
 */
module CodedRangeCleanAnalysis {
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

  val ClaimU0U1: CodedClaim = {
    units: Set("u0", "u1"),
    complete: true,
    validated: true,
  }

  val ScopeU0: CaptureScope = {
    units: Set("u0"),
    complete: true,
    validated: true,
    lowerFrontierCovered: true,
  }

  val ScopeU0U1: CaptureScope = {
    units: Set("u0", "u1"),
    complete: true,
    validated: true,
    lowerFrontierCovered: true,
  }

  val completeClaimAdmissionReachable: bool =
    state.operations.get("opA").phase == Held

  val overlapConflictReachable: bool =
    state.operations.get("opA").phase == Held
    and state.operations.get("opB").phase == Unadmitted

  val disjointClaimsCoexistReachable: bool =
    state.operations.get("opA").phase == Held
    and state.operations.get("opC").phase == Held

  val captureIncludedReachable: bool =
    state.captures.get("capture0").membership.get("opA") == CaptureIncluded

  val captureLaterReachable: bool =
    captureMembershipIsLater(state.captures.get("capture0"), "opB")

  val captureSatisfactionAcceptedReachable: bool =
    state.captures.get("capture0").satisfaction == CaptureSatisfactionAccepted

  val captureSatisfactionRejectedReachable: bool =
    state.captures.get("capture0").satisfaction == CaptureSatisfactionRejected

  val effectPossibleReachable: bool =
    state.operations.get("opA").phase == EffectPossible

  val releasedReachable: bool =
    state.operations.get("opA").phase == Released


  val cleanCommitPendingReachable: bool =
    state.captures.get("capture0").phase == CaptureCommitPending

  val cleanKnownReachable: bool =
    state.captures.get("capture0").phase == CaptureCleanKnown

  val cleanRefusedReachable: bool =
    state.captures.get("capture0").phase == CaptureRefused

  val cleanUnknownReachable: bool =
    state.captures.get("capture0").phase == CaptureCommitUnknown
  val cleanReconciledReachable: bool =
    state.captures.get("capture0").phase == CaptureCleanKnown

  val laterCutAfterCleanReachable: bool =
    state.captures.get("capture0").membership.get("opB")
      == CaptureLaterDurableAfterClean
    and state.captures.get("capture0").phase == CaptureCleanKnown

  val laterCutStalesCleanReachable: bool =
    state.captures.get("capture0").membership.get("opB")
      == CaptureLaterDurableStalesClean
    and state.captures.get("capture0").phase == CaptureRefused

  val laterCutRejectedReachable: bool =
    state.captures.get("capture0").membership.get("opB")
      == CaptureLaterRejected

  val laterCutUnknownReachable: bool =
    state.captures.get("capture0").membership.get("opB")
      == CaptureLaterUnknown
    and state.captures.get("capture0").phase != CaptureRefused

  val laterCutReconciledRejectedReachable: bool =
    state.captures.get("capture0").membership.get("opB")
      == CaptureLaterRejected

  val laterCutReconciledAfterCleanReachable: bool =
    state.captures.get("capture0").membership.get("opB")
      == CaptureLaterDurableAfterClean
    and state.captures.get("capture0").phase == CaptureCleanKnown

  val conservativeDirtyReachable: bool = cleanRefusedReachable

  val releasedHistoryOmittedReachable: bool =
    state.captures.get("capture0").phase == CaptureOpen
    and state.captures.get("capture0").membership.get("opA") == CaptureNotMember

  val activeHistoryRetainedReachable: bool = captureIncludedReachable

  val externalRemovalUnderCleanUnknownReachable: bool =
    state.operations.get("opA").phase == Released
    and state.captures.get("capture0").phase == CaptureCommitUnknown

  val AllBoundedInvariants: bool =
    TypeInvariant
    and NoHeldCodedOverlap
    and CompleteAdmissionPrecedesEffect
    and CaptureMembershipExhaustive
    and CaptureMembershipExclusive
    and RejectedSatisfactionRefusesCapture
    and NoFalseClean
    and UnknownCleanRequiresReconciliation
    and LaterEffectRequiresDurableCut

  action operationStep: bool = {
    nondet operationId = Operations.oneOf()
    any {
      admitComplete(operationId, ClaimU0),
      admitComplete(operationId, ClaimU1),
      admitComplete(operationId, ClaimU0U1),
      permitMutationEffect(operationId),
      removeCodedClaim(operationId, ReleaseAllowed),
    }
  }

  action captureStep: bool = {
    nondet captureId = Captures.oneOf()
    any {
      startCapture(captureId, ScopeU0),
      startCapture(captureId, ScopeU0U1),
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

  // init is setup, not an ordinary analysis transition.
  action step: bool = any {
    operationStep,
    captureStep,
    satisfactionStep,
    laterCutStep,
  }

  val opAIncludedInCapture0: bool =
    state.captures.get("capture0").membership.get("opA") == CaptureIncluded

  run boundedAssumptionsTest = all {
    Operations == Set("opA", "opB", "opC"),
    Captures == Set("capture0", "capture1"),
    CodedUnits == Set("u0", "u1"),
    ClaimU0.units.subseteq(CodedUnits),
    ClaimU1.units.subseteq(CodedUnits),
    ClaimU0U1.units.subseteq(CodedUnits),
    ScopeU0.units.subseteq(CodedUnits),
    ScopeU0U1.units.subseteq(CodedUnits),
  }

  run disjointClaimsCanCoexist =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(admitComplete("opC", ClaimU1))
      .expect(disjointClaimsCoexistReachable)

  run sameCodedClaimConflicts =
    init
      .then(admitComplete("opA", ClaimU0))
      .expect(overlapConflictReachable)
      .then(admitComplete("opB", ClaimU0).fail())

  run nonTransitiveOverlapLeavesDisjointClaimsAvailable =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(admitComplete("opC", ClaimU1))
      .expect(disjointClaimsCoexistReachable)
      .then(admitComplete("opB", ClaimU0U1).fail())

  run incompleteClaimCannotReachEffect =
    init
      .then(admitComplete(
        "opA",
        { units: Set("u0"), complete: false, validated: true },
      ).fail())

  run unadmittedClaimCannotReachEffect =
    init
      .then(permitMutationEffect("opA").fail())

  run captureMembershipIsIncludedThenLater =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .expect(captureIncludedReachable)
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .expect(
        state.captures.get("capture0").membership.get("opA") == CaptureIncluded
        and state.captures.get("capture0").membership.get("opB") == CaptureLater
      )

  run includedOperationCanCommitAndClean =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .expect(cleanKnownReachable)

  run missingCleanDecisionCannotClean =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(requestClean("capture0").fail())

  run cleanOwnerDecisionEnablesClean =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .expect(cleanKnownReachable)

  run rejectedCleanDecisionRefusesCapture =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionRejected))
      .expect(cleanRefusedReachable)

  run unknownCleanDoesNotNegateExternalRelease =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitUnknown))
      .expect(cleanUnknownReachable)
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .expect(externalRemovalUnderCleanUnknownReachable)

  run unknownCleanCanReconcileDurably =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitUnknown))
      .then(reconcileCleanCommit("capture0", CleanReconciledDurable))
      .expect(cleanReconciledReachable)

  run laterMutationUsesDurableAfterCleanCut =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutDurableAfterClean))
      .then(permitMutationEffect("opB"))
      .expect(laterCutAfterCleanReachable)

  run unknownLaterCutAfterDurableCleanReconcilesDurably =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutUnknown))
      .expect(laterCutUnknownReachable)
      .expect(NoFalseClean)
      .expect(
        state.operations.get("opB").phase == Held
        and not(allLaterCutsSatisfied(state.captures, "opB"))
      )
      .then(reconcileLaterCut("capture0", "opB", LaterCutReconciledDurableAfterClean))
      .expect(laterCutReconciledAfterCleanReachable)
      .then(permitMutationEffect("opB"))

  run unknownLaterCutBeforeDurableCleanBlocksCleanCommit =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutUnknown))
      .then(observeCleanCommit("capture0", CleanCommitDurable).fail())

  run unknownLaterCutAfterDurableCleanReconcilesRejected =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutUnknown))
      .then(reconcileLaterCut("capture0", "opB", LaterCutReconciledRejected))
      .expect(laterCutReconciledRejectedReachable)
      .then(permitMutationEffect("opB").fail())

  run unknownLaterCutAfterDurableCleanCannotReconcileStale =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutUnknown))
      .then(reconcileLaterCut("capture0", "opB", LaterCutReconciledStalesClean).fail())

  run staleLaterCutAfterDurableCleanIsRejected =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction("capture0", CaptureSatisfactionAccepted))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitDurable))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutDurableStalesClean).fail())

  run newerDurableBoundaryStalesOlderClean =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutUnknown))
      .then(reconcileLaterCut("capture0", "opB", LaterCutReconciledStalesClean))
      .expect(laterCutStalesCleanReachable)

  run resolvingOneCaptureDoesNotDischargeAnother =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(startCapture("capture1", ScopeU0))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutUnknown))
      .then(observeLaterCut("capture1", "opB", LaterCutUnknown))
      .then(reconcileLaterCut("capture0", "opB", LaterCutReconciledStalesClean))
      .expect(
        state.captures.get("capture0").phase == CaptureRefused
        and state.captures.get("capture1").membership.get("opB") == CaptureLaterUnknown
      )
      .expect(
        state.operations.get("opB").phase == Held
        and not(allLaterCutsSatisfied(state.captures, "opB"))
      )
      .then(reconcileLaterCut("capture1", "opB", LaterCutReconciledStalesClean))
      .then(permitMutationEffect("opB"))

  run unknownLaterCutBeforeCleanCannotReconcileAfterClean =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutUnknown))
      .then(reconcileLaterCut(
        "capture0",
        "opB",
        LaterCutReconciledDurableAfterClean,
      ).fail())

  run rejectedLaterCutRefusesEffect =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opB", ClaimU0))
      .then(observeLaterCut("capture0", "opB", LaterCutRejected))
      .then(permitMutationEffect("opB").fail())


  run removalNeedsExactExternalAuthorization =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(removeCodedClaim("opA", ReleaseNotAuthorized).fail())

  run releasedHistoryNeedNotRemainEnumerable =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(startCapture("capture0", ScopeU0))
      .expect(releasedHistoryOmittedReachable)
      .then(admitComplete("opB", ClaimU0))
      .expect(captureLaterReachable)

  run activeHistoryRemainsEnumerable =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .expect(activeHistoryRetainedReachable)

  run openCaptureRetainsReleasedMembership =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .expect(all {
        opAIncludedInCapture0,
        UnresolvedCaptureMembershipNotCompactable,
      })
      .then(compactReleasedMembership(
        "capture0",
        "opA",
        MembershipCompactionAllowed,
      ).fail())

  run unknownCaptureRetainsReleasedMembership =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(startCapture("capture0", ScopeU0))
      .then(observeCaptureSatisfaction(
        "capture0",
        CaptureSatisfactionAccepted,
      ))
      .then(requestClean("capture0"))
      .then(observeCleanCommit("capture0", CleanCommitUnknown))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .expect(all {
        opAIncludedInCapture0,
        UnresolvedCaptureMembershipNotCompactable,
      })
      .then(compactReleasedMembership(
        "capture0",
        "opA",
        MembershipCompactionAllowed,
      ).fail())
}
