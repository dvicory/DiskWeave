/**
 * Compact refusal-half profile: full nondeterminism over the admission
 * pressure actions (request, retain, consumer set/clear) at depth 4 over
 * the compact domain. Retirement interplay lives in the companion
 * retirement-half profile; the retire-then-request ordering is pinned
 * deterministically on the isolation staged path because request actions
 * never mutate ledger state.
 */
module RetentionBudgetAvailabilityCompactRefusal {
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

  action compactRefusalInit: bool = all { init }

  action compactRefusalStep: bool = {
    nondet op = Operations.oneOf()
    any {
      requestAuth(op),
      retainAuth(op),
      setConsumer(op),
      clearConsumer(op),
    }
  }

  run compactRefusalShapeTest = all {
    SlotIndices == Set(0, 1),
    Generations == Set(0, 1),
    CapacityPerSlot == 1,
  }
}
