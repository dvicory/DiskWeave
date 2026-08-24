/**
 * Two-operation canonical-prefix profile for authorization independence.
 * Both exact generations are externally in-scope; no global operation order
 * or physical slot-reuse policy belongs to this relation.
 */
module LifecycleReleaseIndependentPrefix {
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
    all { phase == 9, phase' = 10, enterApplicable(B0) },
    all { phase == 10, phase' = 11, observeMediaTerminal(B0) },
    all { phase == 11, phase' = 12, observeChildrenTerminal(B0) },
    all { phase == 12, phase' = 13, recordRequiredReconciliation(B0) },
  }

  val ProfileReleaseAllowed: Set[OperationGeneration] = ReleaseAllowed

  run authorizationSurvivesUnrelatedProgress =
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
      .then(profileStep)
      .then(profileStep)
      .expect(
        ProfileReleaseAllowed.contains(A0)
          and state.active.contains(B0)
          and not(ProfileReleaseAllowed.contains(B0))
          and UnrelatedOperationReachable
      )
}
