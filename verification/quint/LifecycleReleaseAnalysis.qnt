/**
 * Finite evidence instance for the canonical LifecycleRelease composition.
 * Concrete identities, scenarios, and checker bounds are evidence;
 * the imported module owns the bounded relation and invariants.
 */
module LifecycleReleaseAnalysis {
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
  pure val B1: OperationGeneration = { operation: "slot-b", generation: 1 }

  run boundedDomainTest = all {
    OperationGenerations.size() == 4,
    ApplicableGenerations.size() == 3,
    ApplicableGenerations.subseteq(OperationGenerations),
    OperationGenerations.contains(A0),
    OperationGenerations.contains(A1),
    OperationGenerations.contains(B0),
    OperationGenerations.contains(B1),
    ApplicableGenerations.contains(A0),
    ApplicableGenerations.contains(A1),
    ApplicableGenerations.contains(B0),
    not(ApplicableGenerations.contains(B1)),
  }

  run canonicalPrefixAuthorizationTest =
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
          and AuthorizationReachable
          and AuthorizationRequiresApplicability
          and AuthorizationRequiresAllOwnerFacts
      )
      .then(observeReleaseAllowed(A0))
      .then(observeReleaseAllowed(A0))
      .expect(ReleaseAllowed.contains(A0))

  run incompleteFactsWithholdTest =
    init
      .then(enterApplicable(A0))
      .expect(not(ReleaseAllowed.contains(A0)))
      .expect(
        not(ReleaseAllowed.contains(A0))
          and not(ReleaseCandidates.contains(A0))
          and MissingOrIncompleteEvidenceWithholds
      )

  run failedShortNeedsAuthoritativeReconciliationTest =
    init
      .then(enterApplicable(A1))
      .then(observeChildrenTerminal(A1))
      .then(recordRequiredReconciliation(A1))
      .then(observeReclaimable(A1))
      .then(observeTransactionSatisfied(A1))
      .then(observeRecoveryAuthoritative(A1))
      .then(observeBasisConformance(A1, BasisDiscarded))
      .expect(not(ReleaseAllowed.contains(A1)))
      .then(observeMediaReconciled(A1))
      .then(establishReleaseAllowed(A1))
      .expect(
        ReleaseAllowed.contains(A1)
          and AuthoritativeReconciliationReachable
          and AuthorizationRequiresAllOwnerFacts
      )

  run basisConformanceIsRequiredTest =
    init
      .then(enterApplicable(A0))
      .then(observeMediaTerminal(A0))
      .then(observeChildrenTerminal(A0))
      .then(recordRequiredReconciliation(A0))
      .then(observeReclaimable(A0))
      .then(observeTransactionSatisfied(A0))
      .then(observeRecoveryAuthoritative(A0))
      .expect(not(ReleaseAllowed.contains(A0)))
      .then(observeBasisConformance(A0, BasisReconciled))
      .then(establishReleaseAllowed(A0))
      .expect(ReleaseAllowed.contains(A0))

  run cleanupRequestIsPendingTest =
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
          and CleanupRequestPreservesAuthorization
      )

  run cleanupFailureRetainsAuthorizationTest =
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
        state.cleanup.get(A0) == CleanupFailed
          and state.active.contains(A0)
          and ReleaseAllowed.contains(A0)
          and CleanupFailurePreservesAuthorization
      )
      .expect(not(state.active.contains(A1)))
      .then(requestCleanup(A0))
      .expect(
        state.cleanupRequests.contains(A0)
          and ReleaseAllowed.contains(A0)
          and state.active.contains(A0)
      )
      .then(observeCleanupSucceeded(A0))
      .expect(
        state.cleanup.get(A0) == CleanupSucceeded
          and not(state.active.contains(A0))
          and ReleaseAllowed.contains(A0)
          and CleanupSuccessPreservesAuthorization
      )
      .then(enterApplicable(A1))
      .expect(
        state.active.contains(A1)
          and ReleaseAllowed.contains(A0)
          and not(ReleaseAllowed.contains(A1))
          and GenerationIsolationReachable
      )

  run canonicalPrefixCleanupTest =
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
        state.cleanup.get(A0) == CleanupSucceeded
          and not(state.active.contains(A0))
          and ReleaseAllowed.contains(A0)
          and CleanupSuccessReachable
          and CleanupSuccessPreservesAuthorization
      )

  run unrelatedAdmissionPreservesAuthorizationTest =
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
          and UnrelatedOperationReachable
      )

  run exactGenerationIsolationTest =
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
      .then(enterApplicable(A1))
      .expect(
        ReleaseAllowed.contains(A0)
          and not(ReleaseAllowed.contains(A1))
          and state.active.contains(A0)
          and state.active.contains(A1)
          and GenerationIsolationReachable
      )
      .then(observeMediaTerminal(A1))
      .then(observeChildrenTerminal(A1))
      .then(recordRequiredReconciliation(A1))
      .then(observeReclaimable(A1))
      .then(observeTransactionSatisfied(A1))
      .then(observeRecoveryAuthoritative(A1))
      .then(observeBasisConformance(A1, BasisConsumed))
      .then(establishReleaseAllowed(A1))
      .expect(
        ReleaseAllowed.contains(A0)
          and ReleaseAllowed.contains(A1)
          and AuthorizationUsesExactIdentity
      )

  run nonVacuousAuthorizationAndCleanupTest =
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
      .expect(AuthorizationReachable and CleanupRequestReachable)
      .then(observeCleanupFailed(A0))
      .expect(CleanupFailureReachable and ReleaseAllowed.contains(A0))
      .then(requestCleanup(A0))
      .then(observeCleanupSucceeded(A0))
      .expect(CleanupSuccessReachable and ReleaseAllowed.contains(A0))
}
