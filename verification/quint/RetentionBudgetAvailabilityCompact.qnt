/**
 * Compact full-interleaving profile for the retention-budget availability
 * relation. Two slot indices, two generations, budget one, unrestricted
 * step: exhaustive bounded verification over every interleaving to shallow
 * depth. Deep paths live in the staged RefusalCycle and RetirementCycle
 * profiles; production-shaped scenarios and sampled runs live in
 * RetentionBudgetAvailabilityAnalysis.
 */
module RetentionBudgetAvailabilityCompact {
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

  action compactInit: bool = all { init }
  action compactStep: bool = all { step }

  run compactShapeTest = all {
    SlotIndices == Set(0, 1),
    Generations == Set(0, 1),
    CapacityPerSlot == 1,
  }
}
