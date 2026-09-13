/**
 * Staged refusal path: saturation, refusal, discharge, and sweep-retry as
 * one deterministic phase sequence. A phase counter drives a single
 * action per phase, so the bounded checker follows the deep path without
 * branching; interleaving coverage comes from sampled full-step runs and
 * the retirement-half profile. Verifies at depth 7 over the compact
 * domain (two slots, two generations, budget one).
 */
module RetentionBudgetAvailabilityRefusalCycle {
  import RetentionBudgetAvailabilityTypes.* from "../../models/quint/RetentionBudgetAvailability"
  import RetentionBudgetAvailability(
    SlotIndices = Set(0, 1),
    Generations = Set(0, 1),
    Operations = Set(
      { slot: 0, generation: 0 },
      { slot: 0, generation: 1 },
      { slot: 1, generation: 0 },
      { slot: 1, generation: 1 },
    ),
    CapacityPerSlot = 1,
  ).* from "../../models/quint/RetentionBudgetAvailability"

  var phase: int

  action refusalInit: bool = all {
    phase' = 0,
    init,
  }

  action refusalStep: bool = any {
    all { phase == 0, phase' = 1, retainAuth({ slot: 0, generation: 0 }) },
    all { phase == 1, phase' = 2, setConsumer({ slot: 0, generation: 0 }) },
    all { phase == 2, phase' = 3, requestAuth({ slot: 0, generation: 1 }) },
    all { phase == 3, phase' = 4, clearConsumer({ slot: 0, generation: 0 }) },
    all { phase == 4, phase' = 5, retainAuth({ slot: 0, generation: 1 }) },
    all { phase == 5, phase' = 6, requestAuth({ slot: 1, generation: 0 }) },
    all { phase == 6, phase' = 7, retireOp({ slot: 0, generation: 1 }) },
  }

  run refusalShapeTest = all {
    SlotIndices == Set(0, 1),
    Generations == Set(0, 1),
    CapacityPerSlot == 1,
  }

  run stagedRefusalPath =
    refusalInit
      .then(refusalStep)
      .then(refusalStep)
      .then(refusalStep)
      .expect(
        state.outcome == OutcomeRefused
          and state.outcomeOp == ForOperation({ slot: 0, generation: 1 })
      )
      .then(refusalStep)
      .then(refusalStep)
      .expect(
        state.outcome == OutcomeRetained
          and state.outcomeOp == ForOperation({ slot: 0, generation: 1 })
          and bucketHas(state, { slot: 0, generation: 1 })
          and not(bucketHas(state, { slot: 0, generation: 0 }))
      )
      .then(refusalStep)
      .expect(
        state.outcome == OutcomeAdmitted
          and state.outcomeOp == ForOperation({ slot: 1, generation: 0 })
      )
      .then(refusalStep)
      .expect(bucketSize(state, 0) == 0)
}

/**
 * Cross-traffic variant: the other index saturates first, proving one
 * index's saturation, refusal, and sweep path never disturb its neighbor.
 * Same depth, domain, and invariants as the primary staged path.
 */
module RetentionBudgetAvailabilityRefusalCrossTraffic {
  import RetentionBudgetAvailabilityTypes.* from "../../models/quint/RetentionBudgetAvailability"
  import RetentionBudgetAvailability(
    SlotIndices = Set(0, 1),
    Generations = Set(0, 1),
    Operations = Set(
      { slot: 0, generation: 0 },
      { slot: 0, generation: 1 },
      { slot: 1, generation: 0 },
      { slot: 1, generation: 1 },
    ),
    CapacityPerSlot = 1,
  ).* from "../../models/quint/RetentionBudgetAvailability"

  var phase: int

  action refusalCrossInit: bool = all {
    phase' = 0,
    init,
  }

  action refusalCrossStep: bool = any {
    all { phase == 0, phase' = 1, retainAuth({ slot: 1, generation: 0 }) },
    all { phase == 1, phase' = 2, setConsumer({ slot: 1, generation: 0 }) },
    all { phase == 2, phase' = 3, retainAuth({ slot: 0, generation: 0 }) },
    all { phase == 3, phase' = 4, setConsumer({ slot: 0, generation: 0 }) },
    all { phase == 4, phase' = 5, requestAuth({ slot: 0, generation: 1 }) },
    all { phase == 5, phase' = 6, clearConsumer({ slot: 1, generation: 0 }) },
    all { phase == 6, phase' = 7, retainAuth({ slot: 1, generation: 1 }) },
  }

  run stagedRefusalCrossTraffic =
    refusalCrossInit
      .then(refusalCrossStep)
      .then(refusalCrossStep)
      .then(refusalCrossStep)
      .then(refusalCrossStep)
      .then(refusalCrossStep)
      .expect(
        state.outcome == OutcomeRefused
          and state.outcomeOp == ForOperation({ slot: 0, generation: 1 })
      )
      .then(refusalCrossStep)
      .then(refusalCrossStep)
      .expect(
        state.outcome == OutcomeRetained
          and state.outcomeOp == ForOperation({ slot: 1, generation: 1 })
          and bucketHas(state, { slot: 1, generation: 1 })
          and bucketHas(state, { slot: 0, generation: 0 })
          and not(bucketHas(state, { slot: 1, generation: 0 }))
      )
}
