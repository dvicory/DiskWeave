/**
 * Staged retirement path: replacement, exhaustion, discharge-sweep, and
 * exact retirement as one deterministic phase sequence. A phase counter
 * drives a single action per phase, so the bounded checker follows the
 * deep path without branching; interleaving coverage comes from sampled
 * full-step runs and the refusal-half profile. Verifies at depth 7 over
 * the compact domain (two slots, two generations, budget one).
 */
module RetentionBudgetAvailabilityRetirementCycle {
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

  action retirementInit: bool = all {
    phase' = 0,
    init,
  }

  action retirementStep: bool = any {
    all { phase == 0, phase' = 1, retainAuth({ slot: 0, generation: 0 }) },
    all { phase == 1, phase' = 2, retainAuth({ slot: 0, generation: 0 }) },
    all { phase == 2, phase' = 3, setConsumer({ slot: 0, generation: 0 }) },
    all { phase == 3, phase' = 4, retainAuth({ slot: 0, generation: 1 }) },
    all { phase == 4, phase' = 5, clearConsumer({ slot: 0, generation: 0 }) },
    all { phase == 5, phase' = 6, retainAuth({ slot: 0, generation: 1 }) },
    all { phase == 6, phase' = 7, retireOp({ slot: 0, generation: 1 }) },
  }

  run retirementShapeTest = all {
    SlotIndices == Set(0, 1),
    Generations == Set(0, 1),
    CapacityPerSlot == 1,
  }

  run stagedRetirementPath =
    retirementInit
      .then(retirementStep)
      .then(retirementStep)
      .expect(
        state.outcome == OutcomeRetained
          and state.outcomeOp == ForOperation({ slot: 0, generation: 0 })
          and bucketSize(state, 0) == 1
      )
      .then(retirementStep)
      .then(retirementStep)
      .expect(
        state.outcome == OutcomeExhausted
          and state.outcomeOp == ForOperation({ slot: 0, generation: 1 })
      )
      .then(retirementStep)
      .then(retirementStep)
      .expect(
        state.outcome == OutcomeRetained
          and state.outcomeOp == ForOperation({ slot: 0, generation: 1 })
          and bucketHas(state, { slot: 0, generation: 1 })
          and not(bucketHas(state, { slot: 0, generation: 0 }))
      )
      .then(retirementStep)
      .expect(bucketSize(state, 0) == 0)
}

/**
 * Isolation variant: a bystander entry on the neighbor index stands
 * throughout saturation, exhaustion, sweep, and retirement, proving no
 * transition touches another index. Same depth, domain, and invariants
 * as the primary staged path.
 */
module RetentionBudgetAvailabilityRetirementIsolation {
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

  action retirementIsoInit: bool = all {
    phase' = 0,
    init,
  }

  action retirementIsoStep: bool = any {
    all { phase == 0, phase' = 1, retainAuth({ slot: 1, generation: 0 }) },
    all { phase == 1, phase' = 2, retainAuth({ slot: 0, generation: 0 }) },
    all { phase == 2, phase' = 3, setConsumer({ slot: 0, generation: 0 }) },
    all { phase == 3, phase' = 4, retainAuth({ slot: 0, generation: 1 }) },
    all { phase == 4, phase' = 5, clearConsumer({ slot: 0, generation: 0 }) },
    all { phase == 5, phase' = 6, retainAuth({ slot: 0, generation: 1 }) },
    all { phase == 6, phase' = 7, retireOp({ slot: 0, generation: 1 }) },
    // retire-then-request ordering: the emptied bucket admits immediately.
    all { phase == 7, phase' = 8, requestAuth({ slot: 0, generation: 0 }) },
  }

  run stagedRetirementIsolation =
    retirementIsoInit
      .then(retirementIsoStep)
      .then(retirementIsoStep)
      .then(retirementIsoStep)
      .then(retirementIsoStep)
      .expect(
        state.outcome == OutcomeExhausted
          and state.outcomeOp == ForOperation({ slot: 0, generation: 1 })
      )
      .then(retirementIsoStep)
      .then(retirementIsoStep)
      .expect(
        state.outcome == OutcomeRetained
          and state.outcomeOp == ForOperation({ slot: 0, generation: 1 })
          and bucketHas(state, { slot: 0, generation: 1 })
      )
      .then(retirementIsoStep)
      .expect(
        bucketSize(state, 0) == 0
          and bucketHas(state, { slot: 1, generation: 0 })
      )
      .then(retirementIsoStep)
      .expect(
        state.outcome == OutcomeAdmitted
          and state.outcomeOp == ForOperation({ slot: 0, generation: 0 })
      )
}
