/**
 * Two-generation canonical-prefix profile for exact-generation isolation.
 * Both generations are externally in-scope; the phase cut constrains only
 * verification reachability and does not import physical slot-reuse policy.
 */
module LifecycleReleaseTwoGenerationPrefix {
  import LifecycleRelease(
    OperationGenerations = Set(
      { operation: "slot-a", generation: 0 },
      { operation: "slot-a", generation: 1 },
    ),
    ApplicableGenerations = Set(
      { operation: "slot-a", generation: 0 },
      { operation: "slot-a", generation: 1 },
    ),
  ).* from "../../models/quint/LifecycleRelease"

  pure val A0: OperationGeneration = { operation: "slot-a", generation: 0 }
  pure val A1: OperationGeneration = { operation: "slot-a", generation: 1 }

  var phase: int

  action profileInit: bool = all {
    phase' = 0,
    init,
  }

  action profileStep: bool = any {
    all { phase == 0, phase' = 1, enterApplicable(A0) },
    all { phase == 1, phase' = 2, observeMediaTerminal(A0) },
    all { phase == 2, phase' = 3, observeChildrenTerminal(A0) },
    all { phase == 3, phase' = 4, recordRequiredReconciliation(A0) },
    all { phase == 4, phase' = 5, observeReclaimable(A0) },
    all { phase == 5, phase' = 6, observeTransactionSatisfied(A0) },
    all { phase == 6, phase' = 7, observeRecoveryAuthoritative(A0) },
    all { phase == 7, phase' = 8, observeBasisConformance(A0, BasisConsumed) },
    all { phase == 8, phase' = 9, establishReleaseAllowed(A0) },
    all { phase == 9, phase' = 10, enterApplicable(A1) },
    all { phase == 10, phase' = 11, observeMediaTerminal(A1) },
    all { phase == 11, phase' = 12, observeChildrenTerminal(A1) },
    all { phase == 12, phase' = 13, recordRequiredReconciliation(A1) },
    all { phase == 13, phase' = 14, observeReclaimable(A1) },
    all { phase == 14, phase' = 15, observeTransactionSatisfied(A1) },
    all { phase == 15, phase' = 16, observeRecoveryAuthoritative(A1) },
    all { phase == 16, phase' = 17, observeBasisConformance(A1, BasisConsumed) },
    all { phase == 17, phase' = 18, establishReleaseAllowed(A1) },
  }

  val ProfileReleaseAllowed: Set[OperationGeneration] = ReleaseAllowed

  run authorizationSurvivesSecondGeneration =
    profileInit
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .expect(
        ProfileReleaseAllowed.contains(A0)
          and state.active.contains(A1)
          and not(ProfileReleaseAllowed.contains(A1))
          and GenerationIsolationReachable
      )
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .then(profileStep)
      .expect(
        ProfileReleaseAllowed.contains(A0)
          and ProfileReleaseAllowed.contains(A1)
          and AuthorizationUsesExactIdentity
      )
}
