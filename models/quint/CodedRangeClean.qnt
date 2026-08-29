/**
 * Bounded coded-range conflict and recovery-CLEAN closure relation.
 *
 * The relation receives complete live owner-approved claims/scopes, one
 * external CLEAN-owner decision per capture, exact CLEAN-commit/cut
 * observations, canonical full-lifecycle ReleaseAllowed, owner-issued
 * membership-compaction authorization, and phase-specific capture-cleanup
 * authorization. It derives only coded overlap, active/unreconciled capture
 * membership, durable post-capture-cut eligibility, CLEAN eligibility, and
 * transitions conditional on those external capabilities. Lifecycle
 * terminality, authority production, basis coherence, topology, persistence,
 * recovery correctness, and physical mechanisms remain external.
 *
 * Finite operation/capture/input sets are analysis parameters, not production
 * cardinality, locks, counters, or frontier mechanisms.
 */
// dwv:req req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts
// dwv:req req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set
module CodedRangeClean {

  type CodedClaim = {
    units: Set[str],
    complete: bool,
    validated: bool,
  }

  type CaptureScope = {
    units: Set[str],
    // Finite projections of fresh coded-owner scope authority. These values
    // are action inputs, never persisted proof or rehydrated authority.
    complete: bool,
    validated: bool,
    // Fresh dirty-integrity-owner lower-frontier authority; not retained.
    lowerFrontierCovered: bool,
  }

  type OperationPhase =
    Unadmitted
    | Held
    | EffectPossible
    | Released

  // The map key is one generation-qualified production operation-slot /
  // assignment identity. Released keys never return to Unadmitted.
  type OperationState = {
    phase: OperationPhase,
    claim: CodedClaim,
  }

  // One exact CLEAN-owner decision applies to the closed capture membership.
  type CaptureSatisfactionObservation =
    CaptureSatisfactionNotObserved
    | CaptureSatisfactionAccepted
    | CaptureSatisfactionRejected

  // Exact canonical full-lifecycle ReleaseAllowed. Safe Reclaimable or any
  // lower-level reclaim witness is only one external prerequisite and cannot
  // inhabit this type. The authorization is consumed, never inferred/stored.
  type ReleaseAuthorization =
    ReleaseNotAuthorized
    | ReleaseAllowed

  // Finite abstraction of exact dirty-integrity-owner compaction authority.
  // Production binds capture/predecessor, receipts, forgotten/retained facts,
  // and replacement summary; capture state alone never supplies it.
  type MembershipCompactionAuthorization =
    MembershipCompactionNotAuthorized
    | MembershipCompactionAllowed

  // Finite abstractions of distinct capture-specific production capabilities.
  // Refused and CleanKnown authorization are not interchangeable, and neither
  // is inferred from phase alone.
  type CaptureCleanupAuthorization =
    CaptureCleanupNotAuthorized
    | RefusedCaptureCleanupAllowed
    | CleanKnownCaptureCleanupAllowed

  // Commit observations are action inputs; CaptureCommitUnknown is the only
  // stored uncertainty status. Durable and rejected outcomes become
  // CleanKnown and CaptureRefused respectively.
  type CleanCommitObservation =
    CleanCommitNotObserved
    | CleanCommitDurable
    | CleanCommitRejected
    | CleanCommitUnknown

  type CleanReconciliationObservation =
    CleanReconciledDurable
    | CleanReconciledRejected

  type LaterCutObservation =
    LaterCutNotObserved
    | LaterCutDurableAfterClean
    | LaterCutDurableStalesClean
    | LaterCutRejected
    | LaterCutUnknown

  // A durable cut after an already durable CLEAN preserves it. The stale
  // reconciliation is the separate before-CLEAN ordering.
  type LaterCutReconciliationObservation =
    LaterCutReconciledDurableAfterClean
    | LaterCutReconciledStalesClean
    | LaterCutReconciledRejected

  type CapturePhase =
    CaptureAbsent
    | CaptureOpen
    | CaptureCommitPending
    | CaptureCommitUnknown
    | CaptureCleanKnown
    | CaptureRefused

  // One per-operation status combines Included/Later membership and later-cut
  // observation. This prevents duplicate sets from disagreeing.
  type CaptureOperationStatus =
    CaptureNotMember
    | CaptureIncluded
    | CaptureLater
    | CaptureLaterDurableAfterClean
    | CaptureLaterDurableStalesClean
    | CaptureLaterRejected
    | CaptureLaterUnknown

  type CaptureState = {
    phase: CapturePhase,
    scope: Set[str],
    membership: str -> CaptureOperationStatus,
    // One external accepted/rejected decision for the whole closed set.
    satisfaction: CaptureSatisfactionObservation,
  }

  const Operations: Set[str]
  const Captures: Set[str]
  const CodedUnits: Set[str]

  type ModelState = {
    operations: str -> OperationState,
    captures: str -> CaptureState,
  }

  var state: ModelState

  pure def emptyClaim: CodedClaim = {
    units: Set(),
    complete: false,
    validated: false,
  }

  pure def initialOperation: OperationState = {
    phase: Unadmitted,
    claim: emptyClaim,
  }

  pure def initialMembership: str -> CaptureOperationStatus =
    Operations.mapBy(operationId => CaptureNotMember)

  pure def initialCapture: CaptureState = {
    phase: CaptureAbsent,
    scope: Set(),
    membership: initialMembership,
    satisfaction: CaptureSatisfactionNotObserved,
  }

  pure def claimOverlap(left: CodedClaim, right: CodedClaim): bool =
    left.complete
    and left.validated
    and right.complete
    and right.validated
    and left.units.intersect(right.units) != Set()

  pure def claimIntersectsScope(claim: CodedClaim, scope: Set[str]): bool =
    claim.complete
    and claim.validated
    and claim.units.intersect(scope) != Set()

  pure def operationHoldsAuthority(operation: OperationState): bool =
    operation.phase == Held
    or operation.phase == EffectPossible

  pure def captureMembershipIsIncluded(
    capture: CaptureState,
    operationId: str,
  ): bool =
    capture.membership.get(operationId) == CaptureIncluded

  pure def captureMembershipIsLater(
    capture: CaptureState,
    operationId: str,
  ): bool =
    capture.membership.get(operationId) == CaptureLater
    or capture.membership.get(operationId) == CaptureLaterDurableAfterClean
    or capture.membership.get(operationId) == CaptureLaterDurableStalesClean
    or capture.membership.get(operationId) == CaptureLaterRejected
    or capture.membership.get(operationId) == CaptureLaterUnknown

  pure def captureLaterCutSatisfiesEffect(status: CaptureOperationStatus): bool =
    status == CaptureLaterDurableAfterClean
    or status == CaptureLaterDurableStalesClean

  pure def captureMembershipHasUnknown(capture: CaptureState): bool =
    Operations.exists(operationId =>
      capture.membership.get(operationId) == CaptureLaterUnknown
    )

  pure def captureMembershipHasStale(capture: CaptureState): bool =
    Operations.exists(operationId =>
      capture.membership.get(operationId) == CaptureLaterDurableStalesClean
    )

  pure def captureTracksFutureAdmission(capture: CaptureState): bool =
    capture.phase != CaptureAbsent
    and capture.phase != CaptureRefused

  pure def intersectingAdmittedOperations(
    operations: str -> OperationState,
    scope: Set[str],
  ): Set[str] =
    Operations.filter(operationId =>
      operationHoldsAuthority(operations.get(operationId))
      and claimIntersectsScope(operations.get(operationId).claim, scope)
    )

  pure def capturesAfterAdmission(
    captures: str -> CaptureState,
    operationId: str,
    claim: CodedClaim,
  ): str -> CaptureState =
    Captures.mapBy(captureId => {
      val capture = captures.get(captureId)
      if (
        captureTracksFutureAdmission(capture)
        and claimIntersectsScope(claim, capture.scope)
      ) {
        {
          ...capture,
          membership: capture.membership.put(operationId, CaptureLater),
        }
      } else capture
    })

  pure def noHeldOverlap(
    operations: str -> OperationState,
    operationId: str,
    claim: CodedClaim,
  ): bool =
    Operations.forall(otherId => {
      val other = operations.get(otherId)
      otherId == operationId
      or not(
        operationHoldsAuthority(other)
        and claimOverlap(other.claim, claim)
      )
    })

  pure def allLaterCutsSatisfied(
    captures: str -> CaptureState,
    operationId: str,
  ): bool =
    Captures.forall(captureId => {
      val capture = captures.get(captureId)
      not(
        captureTracksFutureAdmission(capture)
        and captureMembershipIsLater(capture, operationId)
      )
      or captureLaterCutSatisfiesEffect(
        capture.membership.get(operationId),
      )
    })

  // Consumes the one external CLEAN-owner decision; no dirty/checksum or
  // lifecycle policy is derived and closed membership remains model-validated.
  pure def captureCleanEligible(
    captures: str -> CaptureState,
    captureId: str,
  ): bool = {
    val capture = captures.get(captureId)
    capture.scope != Set()
    and capture.satisfaction == CaptureSatisfactionAccepted
    and not(captureMembershipHasStale(capture))
    and not(captureMembershipHasUnknown(capture))
    and capture.membership.keys() == Operations
  }

  pure def captureAfterSatisfaction(
    capture: CaptureState,
    observation: CaptureSatisfactionObservation,
  ): CaptureState = {
    {
      ...capture,
      satisfaction: observation,
      phase: if (observation == CaptureSatisfactionRejected)
        CaptureRefused
      else CaptureOpen,
    }
  }

  pure def captureAfterLaterCut(
    capture: CaptureState,
    operationId: str,
    observation: LaterCutObservation,
  ): CaptureState = {
    val status = if (observation == LaterCutDurableAfterClean)
      CaptureLaterDurableAfterClean
    else if (observation == LaterCutDurableStalesClean)
      CaptureLaterDurableStalesClean
    else if (observation == LaterCutRejected)
      CaptureLaterRejected
    else if (observation == LaterCutUnknown)
      CaptureLaterUnknown
    else CaptureLater
    val updated = {
      ...capture,
      membership: capture.membership.put(operationId, status),
    }
    {
      ...updated,
      phase: if (observation == LaterCutDurableStalesClean)
        CaptureRefused
      else capture.phase,
    }
  }

  pure def captureAfterLaterCutReconciliation(
    capture: CaptureState,
    operationId: str,
    observation: LaterCutReconciliationObservation,
  ): CaptureState = {
    val status = if (observation == LaterCutReconciledDurableAfterClean)
      CaptureLaterDurableAfterClean
    else if (observation == LaterCutReconciledStalesClean)
      CaptureLaterDurableStalesClean
    else CaptureLaterRejected
    val updated = {
      ...capture,
      membership: capture.membership.put(operationId, status),
    }
    {
      ...updated,
      phase: if (observation == LaterCutReconciledStalesClean)
        CaptureRefused
      else capture.phase,
    }
  }

  pure def captureMembershipMayResolve(capture: CaptureState): bool =
    capture.phase == CaptureRefused
    or capture.phase == CaptureCleanKnown

  // Open, commit-pending, and commit-unknown captures keep exact membership.
  // Finite production capacity backpressures rather than weakening this guard.

  pure def captureMembershipMayCompact(
    capture: CaptureState,
    operationId: str,
  ): bool = {
    val status = capture.membership.get(operationId)
    captureMembershipMayResolve(capture)
    and status != CaptureNotMember
    and status != CaptureLater
    and status != CaptureLaterUnknown
  }

  pure def captureMembershipEmpty(capture: CaptureState): bool =
    Operations.forall(operationId =>
      capture.membership.get(operationId) == CaptureNotMember
    )


  pure def captureAfterMembershipCompaction(
    capture: CaptureState,
    operationId: str,
  ): CaptureState = {
    {
      ...capture,
      membership: capture.membership.put(operationId, CaptureNotMember),
    }
  }
  action init: bool = all {
    state' = {
      operations: Operations.mapBy(operationId => initialOperation),
      captures: Captures.mapBy(captureId => initialCapture),
    },
  }

  action admitComplete(operationId: str, claim: CodedClaim): bool = all {
    state.operations.get(operationId).phase == Unadmitted,
    claim.complete,
    claim.validated,
    claim.units != Set(),
    claim.units.subseteq(CodedUnits),
    noHeldOverlap(state.operations, operationId, claim),
    state' = {
      ...state,
      operations: state.operations.put(operationId, {
        phase: Held,
        claim: claim,
      }),
      captures: capturesAfterAdmission(
        state.captures,
        operationId,
        claim,
      ),
    },
  }

  action startCapture(captureId: str, scope: CaptureScope): bool = all {
    state.captures.get(captureId).phase == CaptureAbsent,
    scope.complete,
    scope.validated,
    scope.units != Set(),
    scope.units.subseteq(CodedUnits),
    scope.lowerFrontierCovered,
    state' = {
      ...state,
      captures: state.captures.put(captureId, {
        phase: CaptureOpen,
        scope: scope.units,
        membership: Operations.mapBy(operationId =>
          if (intersectingAdmittedOperations(state.operations, scope.units).contains(operationId))
            CaptureIncluded
          else CaptureNotMember
        ),
        satisfaction: CaptureSatisfactionNotObserved,
      }),
    },
  }

  action permitMutationEffect(operationId: str): bool = all {
    state.operations.get(operationId).phase == Held,
    allLaterCutsSatisfied(state.captures, operationId),
    state' = {
      ...state,
      operations: state.operations.put(operationId, {
        ...state.operations.get(operationId),
        phase: EffectPossible,
      }),
    },
  }

  // One CLEAN-owner decision covers the closed capture membership.
  action observeCaptureSatisfaction(
    captureId: str,
    observation: CaptureSatisfactionObservation,
  ): bool = all {
    state.captures.get(captureId).phase == CaptureOpen,
    state.captures.get(captureId).satisfaction == CaptureSatisfactionNotObserved,
    observation != CaptureSatisfactionNotObserved,
    state' = {
      ...state,
      captures: state.captures.put(captureId, captureAfterSatisfaction(state.captures.get(captureId), observation)),
    },
  }

  // ReleaseAllowed means the complete production release plus durable receipt
  // definitely committed or authoritative reopen proved that exact successor.
  // Failed or uncertain process-local preparations never enter this relation.
  action removeCodedClaim(
    operationId: str,
    authorization: ReleaseAuthorization,
  ): bool = all {
    state.operations.get(operationId).phase != Unadmitted,
    state.operations.get(operationId).phase != Released,
    authorization == ReleaseAllowed,
    state' = {
      ...state,
      operations: state.operations.put(operationId, {
        ...state.operations.get(operationId),
        phase: Released,
      }),
    },
  }

  // Definite full lifecycle release and its durable receipt precede separately
  // owner-authorized compaction. Unresolved captures retain exact membership.
  action compactReleasedMembership(
    captureId: str,
    operationId: str,
    authorization: MembershipCompactionAuthorization,
  ): bool = all {
    authorization == MembershipCompactionAllowed,
    state.operations.get(operationId).phase == Released,
    captureMembershipMayCompact(state.captures.get(captureId), operationId),
    state' = {
      ...state,
      captures: state.captures.put(captureId, captureAfterMembershipCompaction(state.captures.get(captureId), operationId)),
    },
  }

  // Capture retirement consumes a phase-specific external owner capability
  // only after definite commit or authoritative reconciliation. CleanKnown
  // retirement needs no later mutation once production has durably closed and
  // independently retained every release/history/CLEAN obligation.
  action cleanupResolvedCapture(
    captureId: str,
    authorization: CaptureCleanupAuthorization,
  ): bool = all {
    (
      state.captures.get(captureId).phase == CaptureRefused
      and authorization == RefusedCaptureCleanupAllowed
    )
    or (
      state.captures.get(captureId).phase == CaptureCleanKnown
      and captureMembershipEmpty(state.captures.get(captureId))
      and authorization == CleanKnownCaptureCleanupAllowed
    ),
    state' = {
      ...state,
      captures: state.captures.put(captureId, initialCapture),
    },
  }

  action requestClean(captureId: str): bool = all {
    state.captures.get(captureId).phase == CaptureOpen,
    captureCleanEligible(state.captures, captureId),
    state' = {
      ...state,
      captures: state.captures.put(captureId, {
        ...state.captures.get(captureId),
        phase: CaptureCommitPending,
      }),
    },
  }

  action observeCleanCommit(
    captureId: str,
    observation: CleanCommitObservation,
  ): bool = all {
    state.captures.get(captureId).phase == CaptureCommitPending,
    observation != CleanCommitNotObserved,
    (
      observation != CleanCommitDurable
      or captureCleanEligible(state.captures, captureId)
    ),
    state' = {
      ...state,
      captures: state.captures.put(captureId, {
        ...state.captures.get(captureId),
        phase: if (observation == CleanCommitDurable)
          CaptureCleanKnown
        else if (observation == CleanCommitUnknown)
          CaptureCommitUnknown
        else CaptureRefused,
      }),
    },
  }

  action reconcileCleanCommit(
    captureId: str,
    observation: CleanReconciliationObservation,
  ): bool = all {
    state.captures.get(captureId).phase == CaptureCommitUnknown,
    (
      observation != CleanReconciledDurable
      or captureCleanEligible(state.captures, captureId)
    ),
    state' = {
      ...state,
      captures: state.captures.put(captureId, {
        ...state.captures.get(captureId),
        phase: if (observation == CleanReconciledDurable)
          CaptureCleanKnown
        else CaptureRefused,
      }),
    },
  }

  action observeLaterCut(
    captureId: str,
    operationId: str,
    observation: LaterCutObservation,
  ): bool = all {
    state.captures.get(captureId).phase != CaptureAbsent,
    state.captures.get(captureId).phase != CaptureRefused,
    captureMembershipIsLater(state.captures.get(captureId), operationId),
    state.operations.get(operationId).phase != Unadmitted,
    observation != LaterCutNotObserved,
    state.captures.get(captureId).membership.get(operationId) == CaptureLater,
    (
      observation != LaterCutDurableAfterClean
      or state.captures.get(captureId).phase == CaptureCleanKnown
    ),
    (
      observation != LaterCutDurableStalesClean
      or state.captures.get(captureId).phase != CaptureCleanKnown
    ),
    state' = {
      ...state,
      captures: state.captures.put(captureId, captureAfterLaterCut(state.captures.get(captureId), operationId, observation)),
    },
  }

  action reconcileLaterCut(
    captureId: str,
    operationId: str,
    observation: LaterCutReconciliationObservation,
  ): bool = all {
    state.captures.get(captureId).membership.get(operationId) == CaptureLaterUnknown,
    state.operations.get(operationId).phase != Unadmitted,
    (
      observation != LaterCutReconciledDurableAfterClean
      or state.captures.get(captureId).phase == CaptureCleanKnown
    ),
    (
      observation != LaterCutReconciledStalesClean
      or state.captures.get(captureId).phase != CaptureCleanKnown
    ),
    state' = {
      ...state,
      captures: state.captures.put(captureId, captureAfterLaterCutReconciliation(state.captures.get(captureId), operationId, observation)),
    },
  }

  val TypeInvariant: bool =
    Operations.forall(operationId => {
      val operation = state.operations.get(operationId)
      operation.claim.units.subseteq(CodedUnits)
      and (
        operation.phase != Unadmitted implies (
          operation.claim.complete
          and operation.claim.validated
        )
      )
    })
    and Captures.forall(captureId => {
      val capture = state.captures.get(captureId)
      capture.scope.subseteq(CodedUnits)
      and capture.membership.keys() == Operations
    })

  val AbsentCaptureHasNoMembership: bool =
    Captures.forall(captureId => {
      val capture = state.captures.get(captureId)
      capture.phase == CaptureAbsent implies captureMembershipEmpty(capture)
    })

  val UnresolvedCaptureMembershipNotCompactable: bool =
    Captures.forall(captureId => {
      val capture = state.captures.get(captureId)
      not(captureMembershipMayResolve(capture)) implies
        Operations.forall(operationId =>
          not(captureMembershipMayCompact(capture, operationId))
        )
    })

  val NoHeldCodedOverlap: bool =
    Operations.forall(operationId => {
      val operation = state.operations.get(operationId)
      Operations.forall(otherId => {
        val other = state.operations.get(otherId)
        operationId == otherId
        or not(
          operationHoldsAuthority(operation)
          and operationHoldsAuthority(other)
          and claimOverlap(operation.claim, other.claim)
        )
      })
    })

  val CompleteAdmissionPrecedesEffect: bool =
    Operations.forall(operationId => {
      val operation = state.operations.get(operationId)
      operation.phase == EffectPossible implies (
        operation.claim.complete
        and operation.claim.validated
        and allLaterCutsSatisfied(state.captures, operationId)
      )
    })

  val CaptureMembershipExhaustive: bool =
    Captures.forall(captureId => {
      val capture = state.captures.get(captureId)
      (
        capture.phase != CaptureAbsent
        and capture.phase != CaptureRefused
      ) implies Operations.forall(operationId => {
        val operation = state.operations.get(operationId)
        not(
          operationHoldsAuthority(operation)
          and claimIntersectsScope(operation.claim, capture.scope)
        )
        or capture.membership.get(operationId) != CaptureNotMember
      })
    })

  val CaptureMembershipExclusive: bool =
    Captures.forall(captureId => {
      val capture = state.captures.get(captureId)
      Operations.forall(operationId => {
        val status = capture.membership.get(operationId)
        not(status == CaptureIncluded and captureMembershipIsLater(capture, operationId))
      })
    })

  val RejectedSatisfactionRefusesCapture: bool =
    Captures.forall(captureId => {
      val capture = state.captures.get(captureId)
      capture.satisfaction == CaptureSatisfactionRejected implies
        capture.phase == CaptureRefused
    })

  val NoFalseClean: bool =
    Captures.forall(captureId => {
      val capture = state.captures.get(captureId)
      capture.phase == CaptureCleanKnown implies (
        capture.satisfaction == CaptureSatisfactionAccepted
        and not(captureMembershipHasStale(capture))
      )
    })

  val UnknownCleanRequiresReconciliation: bool =
    Captures.forall(captureId => {
      val capture = state.captures.get(captureId)
      capture.phase == CaptureCommitUnknown implies
        capture.satisfaction == CaptureSatisfactionAccepted
    })

  val LaterEffectRequiresDurableCut: bool =
    Operations.forall(operationId => {
      val operation = state.operations.get(operationId)
      operation.phase == EffectPossible implies
        allLaterCutsSatisfied(state.captures, operationId)
    })
}
