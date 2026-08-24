/**
 * Verification-only LifecycleRelease projection profiles.
 *
 * The imported LifecycleRelease relation remains canonical. These profiles
 * add only deterministic paths and negative probes needed by the Rust
 * correspondence driver; the extra actions leave the canonical state alone.
 * Production slot reuse and abstract same-operation generation coexistence
 * are not part of this projection.
 */
module LifecycleReleaseConnect {
  import LifecycleRelease(
    OperationGenerations = Set(
      { operation: "slot-a", generation: 0 },
      { operation: "slot-b", generation: 0 },
    ),
    ApplicableGenerations = Set(
      { operation: "slot-a", generation: 0 },
      { operation: "slot-b", generation: 0 },
    ),
  ).* from "../../models/quint/LifecycleRelease"

  pure val A0: OperationGeneration = { operation: "slot-a", generation: 0 }
  pure val B0: OperationGeneration = { operation: "slot-b", generation: 0 }

  var pathPhase: int

  action connectInit: bool = all {
    pathPhase' = 0,
    init,
  }

  // These probes exercise verification-only negative paths. They do not alter
  // LifecycleRelease state because they report rejected implementation facts.
  action probeNoAuthorizationBeforeApplicability: bool = all {
    state' = state,
  }

  action probeOutOfScopeOperation: bool = all {
    state' = state,
  }

  action probeStaleGenerationRejection: bool = all {
    state' = state,
  }

  action attemptIncompleteAuthorization(id: OperationGeneration): bool = all {
    state.active.contains(id),
    not(ownerFactsComplete(state.facts.get(id))),
    state' = state,
  }

  action attemptUnresolvedRecovery(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state.facts.get(id).recovery == RecoveryUnresolved,
    state' = state,
  }

  action attemptUnresolvedBasis(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state.facts.get(id).basis == BasisUnresolved,
    state' = state,
  }

  action successStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, enterApplicable(A0) },
    all { pathPhase == 1, pathPhase' = 2, observeChildrenTerminal(A0) },
    all { pathPhase == 2, pathPhase' = 3, observeMediaTerminal(A0) },
    all { pathPhase == 3, pathPhase' = 4, recordRequiredReconciliation(A0) },
    all { pathPhase == 4, pathPhase' = 5, observeReclaimable(A0) },
    all { pathPhase == 5, pathPhase' = 6, observeTransactionSatisfied(A0) },
    all { pathPhase == 6, pathPhase' = 7, observeRecoveryAuthoritative(A0) },
    all { pathPhase == 7, pathPhase' = 8, observeBasisConformance(A0, BasisConsumed) },
    all { pathPhase == 8, pathPhase' = 9, establishReleaseAllowed(A0) },
    all { pathPhase == 9, pathPhase' = 10, observeReleaseAllowed(A0) },
    all { pathPhase == 10, pathPhase' = 11, requestCleanup(A0) },
    all { pathPhase == 11, pathPhase' = 12, observeCleanupSucceeded(A0) },
  }

  action failureStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, enterApplicable(B0) },
    all { pathPhase == 1, pathPhase' = 2, observeMediaReconciled(B0) },
    all { pathPhase == 2, pathPhase' = 3, observeChildrenTerminal(B0) },
    all { pathPhase == 3, pathPhase' = 4, recordRequiredReconciliation(B0) },
    all { pathPhase == 4, pathPhase' = 5, observeReclaimable(B0) },
    all { pathPhase == 5, pathPhase' = 6, observeTransactionSatisfied(B0) },
    all { pathPhase == 6, pathPhase' = 7, observeRecoveryAuthoritative(B0) },
    all { pathPhase == 7, pathPhase' = 8, observeBasisConformance(B0, BasisReconciled) },
    all { pathPhase == 8, pathPhase' = 9, establishReleaseAllowed(B0) },
    all { pathPhase == 9, pathPhase' = 10, observeReleaseAllowed(B0) },
    all { pathPhase == 10, pathPhase' = 11, requestCleanup(B0) },
    all { pathPhase == 11, pathPhase' = 12, observeCleanupFailed(B0) },
  }

  action negativeStep: bool = any {
    all { pathPhase == 0, pathPhase' = 1, probeNoAuthorizationBeforeApplicability },
    all { pathPhase == 1, pathPhase' = 2, probeOutOfScopeOperation },
    all { pathPhase == 2, pathPhase' = 3, probeStaleGenerationRejection },
    all { pathPhase == 3, pathPhase' = 4, enterApplicable(A0) },
    all { pathPhase == 4, pathPhase' = 5, attemptIncompleteAuthorization(A0) },
    all { pathPhase == 5, pathPhase' = 6, observeChildrenTerminal(A0) },
    all { pathPhase == 6, pathPhase' = 7, observeMediaTerminal(A0) },
    all { pathPhase == 7, pathPhase' = 8, recordRequiredReconciliation(A0) },
    all { pathPhase == 8, pathPhase' = 9, observeReclaimable(A0) },
    all { pathPhase == 9, pathPhase' = 10, observeTransactionSatisfied(A0) },
    all { pathPhase == 10, pathPhase' = 11, attemptUnresolvedRecovery(A0) },
    all { pathPhase == 11, pathPhase' = 12, observeRecoveryAuthoritative(A0) },
    all { pathPhase == 12, pathPhase' = 13, attemptUnresolvedBasis(A0) },
    all { pathPhase == 13, pathPhase' = 14, observeBasisConformance(A0, BasisConsumed) },
    all { pathPhase == 14, pathPhase' = 15, establishReleaseAllowed(A0) },
    all { pathPhase == 15, pathPhase' = 16, observeReleaseAllowed(A0) },
    all { pathPhase == 16, pathPhase' = 17, requestCleanup(A0) },
    all { pathPhase == 17, pathPhase' = 18, observeCleanupSucceeded(A0) },
  }
}
