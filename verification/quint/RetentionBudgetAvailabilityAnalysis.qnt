/**
 * Scenario evidence for the retention-budget availability relation.
 * Production-shaped instance: two slot indices, three generations, budget
 * two. Exhaustive bounded checks live in the RefusalCycle and
 * RetirementCycle phase-cut profiles; this module holds deterministic
 * scenario traces via `quint test` plus full-step sampled runs.
 *
 * Every remember drops the lowest discharged entry at its index first,
 * then replaces, sweeps-and-inserts, or fails-with-sweep. Every verdict
 * scenario asserts both the outcome and the exact outcomeOp identity, so
 * a mislabeled verdict breaks the suite.
 */
module RetentionBudgetAvailabilityAnalysis {
  import RetentionBudgetAvailabilityTypes.* from "../../models/quint/RetentionBudgetAvailability"
  import RetentionBudgetAvailability(
    SlotIndices = Set(0, 1),
    Generations = Set(0, 1, 2),
    Operations = Set(
      { slot: 0, generation: 0 },
      { slot: 0, generation: 1 },
      { slot: 0, generation: 2 },
      { slot: 1, generation: 0 },
      { slot: 1, generation: 1 },
      { slot: 1, generation: 2 },
    ),
    CapacityPerSlot = 2,
  ).* from "../../models/quint/RetentionBudgetAvailability"

  action analysisInit: bool = all { init }
  action analysisStep: bool = all { step }

  run productionShapeTest = all {
    SlotIndices == Set(0, 1),
    Generations == Set(0, 1, 2),
    CapacityPerSlot == 2,
  }

  run admitWhenFree =
    init
      .then(requestAuth({ slot: 0, generation: 0 }))
      .expect(
        state.outcome == OutcomeAdmitted
          and state.outcomeOp == ForOperation({ slot: 0, generation: 0 })
      )

  run eagerDropOnRoom =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(retainAuth({ slot: 0, generation: 1 }))
      .expect(
        state.outcome == OutcomeRetained
          and state.outcomeOp == ForOperation({ slot: 0, generation: 1 })
          and bucketHas(state, { slot: 0, generation: 1 })
          and not(bucketHas(state, { slot: 0, generation: 0 }))
      )

  // Purity pin: refusal changes nothing, so the exact bucket and consumer
  // maps are asserted, not just the verdict. Any refusal-path mutation
  // breaks this scenario.
  run refuseConsumedSaturation =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(setConsumer({ slot: 0, generation: 0 }))
      .then(retainAuth({ slot: 0, generation: 1 }))
      .then(setConsumer({ slot: 0, generation: 1 }))
      .then(requestAuth({ slot: 0, generation: 2 }))
      .expect(
        state.outcome == OutcomeRefused
          and state.outcomeOp == ForOperation({ slot: 0, generation: 2 })
          and state.retained.get(0) == Set(0, 1)
          and state.consumers.get(0) == Set(0, 1)
      )

  run freeIndexAdmitsDespiteSaturation =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(setConsumer({ slot: 0, generation: 0 }))
      .then(retainAuth({ slot: 0, generation: 1 }))
      .then(setConsumer({ slot: 0, generation: 1 }))
      .then(requestAuth({ slot: 0, generation: 2 }))
      .then(requestAuth({ slot: 1, generation: 0 }))
      .expect(
        state.outcome == OutcomeAdmitted
          and state.outcomeOp == ForOperation({ slot: 1, generation: 0 })
          and state.retained.get(0) == Set(0, 1)
          and state.consumers.get(0) == Set(0, 1)
          and bucketSize(state, 1) == 0
      )

  run dischargeSweepRetains =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(setConsumer({ slot: 0, generation: 0 }))
      .then(retainAuth({ slot: 0, generation: 1 }))
      .then(setConsumer({ slot: 0, generation: 1 }))
      .then(clearConsumer({ slot: 0, generation: 0 }))
      .then(retainAuth({ slot: 0, generation: 2 }))
      .expect(
        state.outcome == OutcomeRetained
          and state.outcomeOp == ForOperation({ slot: 0, generation: 2 })
          and bucketHas(state, { slot: 0, generation: 1 })
          and bucketHas(state, { slot: 0, generation: 2 })
          and not(bucketHas(state, { slot: 0, generation: 0 }))
      )

  // Purity pin: exhaustion refuses the new entry (with the documented
  // failure sweep, a no-op here since nothing is discharged) while the
  // exact bucket and consumer maps are asserted unchanged.
  run exhaustedBeforeDischarge =
    init
      .then(retainAuth({ slot: 1, generation: 0 }))
      .then(setConsumer({ slot: 1, generation: 0 }))
      .then(retainAuth({ slot: 1, generation: 1 }))
      .then(setConsumer({ slot: 1, generation: 1 }))
      .then(retainAuth({ slot: 1, generation: 2 }))
      .expect(
        state.outcome == OutcomeExhausted
          and state.outcomeOp == ForOperation({ slot: 1, generation: 2 })
          and state.retained.get(1) == Set(0, 1)
          and state.consumers.get(1) == Set(0, 1)
          and not(bucketHas(state, { slot: 1, generation: 2 }))
      )

  // Whole-ledger purity: refusal must leave every bucket on every index
  // untouched, not just the verdict. Any refusal-path mutation on any
  // index breaks this scenario.
  run refusalLeavesWholeLedgerUntouched =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(setConsumer({ slot: 0, generation: 0 }))
      .then(retainAuth({ slot: 0, generation: 1 }))
      .then(setConsumer({ slot: 0, generation: 1 }))
      .then(requestAuth({ slot: 0, generation: 2 }))
      .expect(
        state.outcome == OutcomeRefused
          and state.outcomeOp == ForOperation({ slot: 0, generation: 2 })
          and state.retained.get(0) == Set(0, 1)
          and state.consumers.get(0) == Set(0, 1)
          and bucketSize(state, 1) == 0
          and state.consumers.get(1).size() == 0
      )

  // Whole-ledger purity for the exhausted path: the refused entry is
  // absent and both indices' maps are asserted exactly.
  run exhaustionLeavesWholeLedgerUntouched =
    init
      .then(retainAuth({ slot: 1, generation: 0 }))
      .then(setConsumer({ slot: 1, generation: 0 }))
      .then(retainAuth({ slot: 1, generation: 1 }))
      .then(setConsumer({ slot: 1, generation: 1 }))
      .then(retainAuth({ slot: 1, generation: 2 }))
      .expect(
        state.outcome == OutcomeExhausted
          and state.outcomeOp == ForOperation({ slot: 1, generation: 2 })
          and state.retained.get(1) == Set(0, 1)
          and state.consumers.get(1) == Set(0, 1)
          and bucketSize(state, 0) == 0
          and state.consumers.get(0).size() == 0
      )

  run replaceWithoutGrowth =
    init
      .then(retainAuth({ slot: 1, generation: 0 }))
      .then(setConsumer({ slot: 1, generation: 0 }))
      .then(retainAuth({ slot: 1, generation: 1 }))
      .then(setConsumer({ slot: 1, generation: 1 }))
      .then(retainAuth({ slot: 1, generation: 0 }))
      .expect(
        state.outcome == OutcomeRetained
          and state.outcomeOp == ForOperation({ slot: 1, generation: 0 })
          and bucketSize(state, 1) == 2
      )

  run retireOnlyWhenDischarged =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(retireOp({ slot: 0, generation: 0 }))
      .expect(bucketSize(state, 0) == 0)

  run generationReuseAfterSweep =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(setConsumer({ slot: 0, generation: 0 }))
      .then(retainAuth({ slot: 0, generation: 1 }))
      .then(setConsumer({ slot: 0, generation: 1 }))
      .then(clearConsumer({ slot: 0, generation: 0 }))
      .then(retainAuth({ slot: 0, generation: 2 }))
      .then(requestAuth({ slot: 0, generation: 0 }))
      .expect(
        bucketHas(state, { slot: 0, generation: 1 })
          and bucketHas(state, { slot: 0, generation: 2 })
          and not(bucketHas(state, { slot: 0, generation: 0 }))
          and state.outcome == OutcomeAdmitted
          and state.outcomeOp == ForOperation({ slot: 0, generation: 0 })
      )
}

/**
 * Occupied-sweep pinpoint: at budget three, an occupied remember drops
 * exactly the lowest discharged entry and keeps every other discharged
 * entry, while a vacant remember sweeps them all. Needs three
 * same-index entries, so no smaller profile can distinguish it.
 */
module RetentionBudgetAvailabilityOccupiedSweep {
  import RetentionBudgetAvailabilityTypes.* from "../../models/quint/RetentionBudgetAvailability"
  import RetentionBudgetAvailability(
    SlotIndices = Set(0),
    Generations = Set(0, 1, 2),
    Operations = Set(
      { slot: 0, generation: 0 },
      { slot: 0, generation: 1 },
      { slot: 0, generation: 2 },
    ),
    CapacityPerSlot = 3,
  ).* from "../../models/quint/RetentionBudgetAvailability"

  var phase: int

  action occupiedInit: bool = all {
    phase' = 0,
    init,
  }

  action occupiedStep: bool = any {
    all { phase == 0, phase' = 1, retainAuth({ slot: 0, generation: 0 }) },
    all { phase == 1, phase' = 2, setConsumer({ slot: 0, generation: 0 }) },
    all { phase == 2, phase' = 3, retainAuth({ slot: 0, generation: 1 }) },
    all { phase == 3, phase' = 4, setConsumer({ slot: 0, generation: 1 }) },
    all { phase == 4, phase' = 5, retainAuth({ slot: 0, generation: 2 }) },
    all { phase == 5, phase' = 6, setConsumer({ slot: 0, generation: 2 }) },
    all { phase == 6, phase' = 7, clearConsumer({ slot: 0, generation: 0 }) },
    all { phase == 7, phase' = 8, clearConsumer({ slot: 0, generation: 1 }) },
    all { phase == 8, phase' = 9, retainAuth({ slot: 0, generation: 2 }) },
  }

  run occupiedKeepsOtherDischarged =
    occupiedInit
      .then(occupiedStep)
      .then(occupiedStep)
      .then(occupiedStep)
      .then(occupiedStep)
      .then(occupiedStep)
      .then(occupiedStep)
      .then(occupiedStep)
      .then(occupiedStep)
      .then(occupiedStep)
      .expect(
        state.outcome == OutcomeRetained
          and state.outcomeOp == ForOperation({ slot: 0, generation: 2 })
          and bucketHas(state, { slot: 0, generation: 1 })
          and bucketHas(state, { slot: 0, generation: 2 })
          and not(bucketHas(state, { slot: 0, generation: 0 }))
      )

  // Sibling-generation isolation canary: a retire that clears the whole
  // bucket instead of one exact entry orphans the surviving entries'
  // consumer flags. Only expressible where one index holds several
  // generations, so it lives here rather than with the cap-1 canaries.
  action retireAllMutant(op: Operation): bool = all {
    Operations.contains(op),
    bucketHas(state, op),
    state' = {
      ...state,
      retained: state.retained.put(op.slot, state.retained.get(op.slot).filter(g => false)),
      outcome: OutcomeNone,
      outcomeOp: NoOutcome,
    },
  }

  run retireAllOrphansConsumersMutantTest =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(setConsumer({ slot: 0, generation: 0 }))
      .then(retainAuth({ slot: 0, generation: 1 }))
      .then(setConsumer({ slot: 0, generation: 1 }))
      .then(retainAuth({ slot: 0, generation: 2 }))
      .then(setConsumer({ slot: 0, generation: 2 }))
      .then(clearConsumer({ slot: 0, generation: 1 }))
      .then(retireAllMutant({ slot: 0, generation: 1 }))
      .expect(not(ConsumerEntriesRetained))
}
