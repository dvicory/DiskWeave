/**
 * Negative evidence for the canonical LifecycleRelease composition.
 *
 * Each named mutant is an intentionally false authorization predicate over
 * the canonical state. The run witnesses make the false predicate reachable;
 * bounded checking each mutant invariant must then find a failure.
 */
module LifecycleReleaseMutants {
  import LifecycleRelease(
    OperationGenerations = Set(
      { operation: "slot-a", generation: 0 },
      { operation: "slot-a", generation: 1 },
      { operation: "slot-b", generation: 0 },
      { operation: "slot-b", generation: 1 },
    ),
    ApplicableGenerations = Set(
      { operation: "slot-a", generation: 0 },
      { operation: "slot-a", generation: 1 },
      { operation: "slot-b", generation: 0 },
    ),
  ).* from "../../models/quint/LifecycleRelease"

  pure val A0: OperationGeneration = { operation: "slot-a", generation: 0 }
  pure val A1: OperationGeneration = { operation: "slot-a", generation: 1 }
  pure val B0: OperationGeneration = { operation: "slot-b", generation: 0 }


// Evidence-only proxy: a raw transaction trace observation is not an owner fact.
pure val RawRangeReleased: Set[OperationGeneration] = Set(A0)

  pure def completeExceptTransaction(facts: OwnerFacts): bool =
    (facts.mediaEffect == MediaTerminal or facts.mediaEffect == MediaReconciled)
      and facts.children == ChildrenTerminal
      and facts.requiredReconciliation
      and facts.reclaimable
      and facts.recovery == RecoveryAuthoritative
      and (facts.basis == BasisConsumed
        or facts.basis == BasisDiscarded
        or facts.basis == BasisReconciled)

  pure def completeExceptBasis(facts: OwnerFacts): bool =
    (facts.mediaEffect == MediaTerminal or facts.mediaEffect == MediaReconciled)
      and facts.children == ChildrenTerminal
      and facts.requiredReconciliation
      and facts.reclaimable
      and facts.transaction == TransactionSatisfied
      and facts.recovery == RecoveryAuthoritative

  val missingTransactionAuthorization: Set[OperationGeneration] =
    state.active.filter(id =>
      completeExceptTransaction(state.facts.get(id))
        and state.facts.get(id).transaction != TransactionSatisfied
    )

  val omittedBasisAuthorization: Set[OperationGeneration] =
    state.active.filter(id =>
      completeExceptBasis(state.facts.get(id))
        and state.facts.get(id).basis == BasisUnresolved
    )

  val reclaimableProxyAuthorization: Set[OperationGeneration] =
    state.active.filter(id => state.facts.get(id).reclaimable)
  
  val applicabilityOmittedAuthorization: Set[OperationGeneration] =
    state.active.filter(id => ownerFactsComplete(state.facts.get(id)))

  val ApplicabilityOmittedMutantInvariant: bool =
    applicabilityOmittedAuthorization.forall(id => ApplicableGenerations.contains(id))

  val MissingTransactionMutantInvariant: bool =
    missingTransactionAuthorization.forall(id => ownerFactsComplete(state.facts.get(id)))

  val OmittedBasisEvidenceMutantInvariant: bool =
    omittedBasisAuthorization.forall(id => ownerFactsComplete(state.facts.get(id)))

  val ReclaimableProxyMutantInvariant: bool =
    reclaimableProxyAuthorization.forall(id => ownerFactsComplete(state.facts.get(id)))

  val rangeReleasedProxyAuthorization: Set[OperationGeneration] =
    state.active.intersect(RawRangeReleased)

  val RangeReleasedProxyMutantInvariant: bool =
    rangeReleasedProxyAuthorization.forall(id => ownerFactsComplete(state.facts.get(id)))

  val CleanupSuccessRequiredMutantInvariant: bool =
    state.authorizations.forall(id => state.cleanup.get(id) == CleanupSucceeded)

  val CleanupFailureRevokesAuthorizationMutantInvariant: bool =
    OperationGenerations.forall(id =>
      state.cleanup.get(id) == CleanupFailed implies not(state.authorizations.contains(id))
    )

  val CleanupSuccessErasesAuthorizationMutantInvariant: bool =
    OperationGenerations.forall(id =>
      state.cleanup.get(id) == CleanupSucceeded implies not(state.authorizations.contains(id))
    )

  val UnrelatedAdmissionErasureMutantInvariant: bool =
    state.authorizations.forall(auth =>
      state.active.forall(other =>
        (other.operation != auth.operation) implies not(state.authorizations.contains(auth))
      )
    )

  val LateCleanupAuthorizesNewGenerationMutantInvariant: bool =
    state.cleanup.get(A0) == CleanupSucceeded
      and state.active.contains(A1)
      implies state.authorizations.contains(A1)

  val UnresolvedPrerequisiteAcceptanceMutantInvariant: bool =
    state.active.forall(id => ownerFactsComplete(state.facts.get(id)))

  val ActionImpliesSuccessMutantInvariant: bool =
    state.cleanupRequests.forall(id => state.cleanup.get(id) == CleanupSucceeded)

  
  // Evidence-only mutation: bypass external applicability to test the guard.
  action admitOutOfScope(id: OperationGeneration): bool = all {
    OperationGenerations.contains(id),
    not(ApplicableGenerations.contains(id)),
    not(state.active.contains(id)),
    state' = {
      ...state,
      active: state.active.union(Set(id)),
      facts: state.facts.put(id, emptyFacts),
      cleanup: state.cleanup.put(id, CleanupNotRequested),
      cleanupRequests: state.cleanupRequests.exclude(Set(id)),
    },
  }
  run applicabilityOmittedMutantTest =
    init
      .then(admitOutOfScope({ operation: "slot-b", generation: 1 }))
      .then(observeMediaTerminal({ operation: "slot-b", generation: 1 }))
      .then(observeChildrenTerminal({ operation: "slot-b", generation: 1 }))
      .then(recordRequiredReconciliation({ operation: "slot-b", generation: 1 }))
      .then(observeReclaimable({ operation: "slot-b", generation: 1 }))
      .then(observeTransactionSatisfied({ operation: "slot-b", generation: 1 }))
      .then(observeRecoveryAuthoritative({ operation: "slot-b", generation: 1 }))
      .then(
        observeBasisConformance(
          { operation: "slot-b", generation: 1 },
          BasisConsumed
        )
      )
      .expect(
        applicabilityOmittedAuthorization.contains(
          { operation: "slot-b", generation: 1 }
        )
          and not(ReleaseAllowed.contains({ operation: "slot-b", generation: 1 }))
          and not(ApplicabilityOmittedMutantInvariant)
      )
  run missingTransactionMutantTest =
    init
      .then(enterApplicable(A0))
      .then(observeMediaTerminal(A0))
      .then(observeChildrenTerminal(A0))
      .then(recordRequiredReconciliation(A0))
      .then(observeReclaimable(A0))
      .then(observeRecoveryAuthoritative(A0))
      .then(observeBasisConformance(A0, BasisConsumed))
      .expect(
        missingTransactionAuthorization.contains(A0)
          and not(ReleaseAllowed.contains(A0))
          and not(MissingTransactionMutantInvariant)
      )

  run omittedBasisEvidenceMutantTest =
    init
      .then(enterApplicable(A0))
      .then(observeMediaTerminal(A0))
      .then(observeChildrenTerminal(A0))
      .then(recordRequiredReconciliation(A0))
      .then(observeReclaimable(A0))
      .then(observeTransactionSatisfied(A0))
      .then(observeRecoveryAuthoritative(A0))
      .expect(
        omittedBasisAuthorization.contains(A0)
          and not(ReleaseAllowed.contains(A0))
          and not(OmittedBasisEvidenceMutantInvariant)
      )

  run reclaimableProxyMutantTest =
    init
      .then(enterApplicable(A0))
      .then(observeReclaimable(A0))
      .expect(
        reclaimableProxyAuthorization.contains(A0)
          and not(ReleaseAllowed.contains(A0))
          and not(ReclaimableProxyMutantInvariant)
      )


  run rangeReleasedProxyMutantTest =
    init
      .then(enterApplicable(A0))
      .expect(
        rangeReleasedProxyAuthorization.contains(A0)
          and not(ReleaseAllowed.contains(A0))
          and not(RangeReleasedProxyMutantInvariant)
      )
  run cleanupSuccessRequiredMutantTest =
    init
      .then(enterApplicable(A0))
      .then(observeMediaTerminal(A0))
      .then(observeChildrenTerminal(A0))
      .then(recordRequiredReconciliation(A0))
      .then(observeReclaimable(A0))
      .then(observeTransactionSatisfied(A0))
      .then(observeRecoveryAuthoritative(A0))
      .then(observeBasisConformance(A0, BasisConsumed))
      .then(establishReleaseAllowed(A0))
      .expect(
        ReleaseAllowed.contains(A0)
          and state.cleanup.get(A0) != CleanupSucceeded
          and not(CleanupSuccessRequiredMutantInvariant)
      )

  run cleanupFailureRevocationMutantTest =
    init
      .then(enterApplicable(A0))
      .then(observeMediaTerminal(A0))
      .then(observeChildrenTerminal(A0))
      .then(recordRequiredReconciliation(A0))
      .then(observeReclaimable(A0))
      .then(observeTransactionSatisfied(A0))
      .then(observeRecoveryAuthoritative(A0))
      .then(observeBasisConformance(A0, BasisConsumed))
      .then(establishReleaseAllowed(A0))
      .then(requestCleanup(A0))
      .then(observeCleanupFailed(A0))
      .expect(
        ReleaseAllowed.contains(A0)
          and state.cleanup.get(A0) == CleanupFailed
          and not(CleanupFailureRevokesAuthorizationMutantInvariant)
      )

  run cleanupSuccessErasureMutantTest =
    init
      .then(enterApplicable(A0))
      .then(observeMediaTerminal(A0))
      .then(observeChildrenTerminal(A0))
      .then(recordRequiredReconciliation(A0))
      .then(observeReclaimable(A0))
      .then(observeTransactionSatisfied(A0))
      .then(observeRecoveryAuthoritative(A0))
      .then(observeBasisConformance(A0, BasisConsumed))
      .then(establishReleaseAllowed(A0))
      .then(requestCleanup(A0))
      .then(observeCleanupSucceeded(A0))
      .expect(
        ReleaseAllowed.contains(A0)
          and state.cleanup.get(A0) == CleanupSucceeded
          and not(CleanupSuccessErasesAuthorizationMutantInvariant)
      )

  run unrelatedAdmissionErasureMutantTest =
    init
      .then(enterApplicable(A0))
      .then(observeMediaTerminal(A0))
      .then(observeChildrenTerminal(A0))
      .then(recordRequiredReconciliation(A0))
      .then(observeReclaimable(A0))
      .then(observeTransactionSatisfied(A0))
      .then(observeRecoveryAuthoritative(A0))
      .then(observeBasisConformance(A0, BasisConsumed))
      .then(establishReleaseAllowed(A0))
      .then(enterApplicable(B0))
      .expect(
        state.active.contains(B0)
          and ReleaseAllowed.contains(A0)
          and not(UnrelatedAdmissionErasureMutantInvariant)
      )

  run lateCleanupGenerationConfusionMutantTest =
    init
      .then(enterApplicable(A0))
      .then(observeMediaTerminal(A0))
      .then(observeChildrenTerminal(A0))
      .then(recordRequiredReconciliation(A0))
      .then(observeReclaimable(A0))
      .then(observeTransactionSatisfied(A0))
      .then(observeRecoveryAuthoritative(A0))
      .then(observeBasisConformance(A0, BasisConsumed))
      .then(establishReleaseAllowed(A0))
      .then(requestCleanup(A0))
      .then(observeCleanupSucceeded(A0))
      .then(enterApplicable(A1))
      .expect(
        state.cleanup.get(A0) == CleanupSucceeded
          and state.active.contains(A1)
          and not(ReleaseAllowed.contains(A1))
          and not(LateCleanupAuthorizesNewGenerationMutantInvariant)
      )

  run unresolvedPrerequisiteAcceptanceMutantTest =
    init
      .then(enterApplicable(A0))
      .expect(
        not(ownerFactsComplete(state.facts.get(A0)))
          and not(ReleaseAllowed.contains(A0))
          and not(UnresolvedPrerequisiteAcceptanceMutantInvariant)
      )

  run actionImpliesSuccessMutantTest =
    init
      .then(enterApplicable(A0))
      .then(observeMediaTerminal(A0))
      .then(observeChildrenTerminal(A0))
      .then(recordRequiredReconciliation(A0))
      .then(observeReclaimable(A0))
      .then(observeTransactionSatisfied(A0))
      .then(observeRecoveryAuthoritative(A0))
      .then(observeBasisConformance(A0, BasisConsumed))
      .then(establishReleaseAllowed(A0))
      .then(requestCleanup(A0))
      .expect(
        state.cleanupRequests.contains(A0)
          and state.cleanup.get(A0) == CleanupNotRequested
          and ReleaseAllowed.contains(A0)
          and not(ActionImpliesSuccessMutantInvariant)
      )
}
