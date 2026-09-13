/**
 * Compact retirement-half profile: full nondeterminism over the
 * retention lifecycle actions (retain, retire, consumer set/clear) at
 * depth 4 over the compact domain. Admission interplay lives in the
 * companion refusal-half profile; request actions never mutate ledger
 * state, so request-first orderings add no state beyond the second
 * action's own half.
 */
module RetentionBudgetAvailabilityCompactRetirement {
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

  action compactRetirementInit: bool = all { init }

  action compactRetirementStep: bool = {
    nondet op = Operations.oneOf()
    any {
      retainAuth(op),
      retireOp(op),
      setConsumer(op),
      clearConsumer(op),
    }
  }

  run compactRetirementShapeTest = all {
    SlotIndices == Set(0, 1),
    Generations == Set(0, 1),
    CapacityPerSlot == 1,
  }
}
