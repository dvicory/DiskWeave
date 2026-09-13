/**
 * Negative evidence for the retention-budget availability relation.
 *
 * Each action injects one defect. Its deterministic witness must violate the
 * named safety predicate, proving that the corresponding oracle is effective.
 */
module RetentionBudgetAvailabilityMutants {
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

  action sweepDropsConsumedMutant(op: Operation): bool = all {
    Operations.contains(op),
    state' = {
      ...state,
      retained: state.retained.put(op.slot, Set(op.generation)),
      outcome: OutcomeRetained,
      outcomeOp: ForOperation(op),
    },
  }

  action refuseDespiteSpaceMutant(op: Operation): bool = all {
    Operations.contains(op),
    state' = {
      ...state,
      outcome: OutcomeRefused,
      outcomeOp: ForOperation(op),
    },
  }

  action retireWithConsumerMutant(op: Operation): bool = all {
    Operations.contains(op),
    bucketHas(state, op),
    state' = {
      ...state,
      retained: state.retained.put(
        op.slot,
        state.retained.get(op.slot).filter(g => g != op.generation)),
      outcome: OutcomeNone,
      outcomeOp: NoOutcome,
    },
  }

  pure def mutantBlocked(s: BudgetState, op: Operation): bool =
    bucketSize(s, op.slot) > 0
      and bucketSize(s, op.slot) >= CapacityPerSlot
      and bucketAllConsumed(s, op.slot)

  action tokenBlindRefusalMutant(op: Operation): bool = all {
    Operations.contains(op),
    state' = {
      ...state,
      outcome: if (mutantBlocked(state, op)) OutcomeRefused else OutcomeAdmitted,
      outcomeOp: ForOperation(op),
    },
  }

  run sweepDropsConsumedMutantTest =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(setConsumer({ slot: 0, generation: 0 }))
      .then(sweepDropsConsumedMutant({ slot: 0, generation: 1 }))
      .expect(not(ConsumerEntriesRetained))

  run refuseDespiteSpaceMutantTest =
    init
      .then(refuseDespiteSpaceMutant({ slot: 0, generation: 0 }))
      .expect(not(RefusalValid))

  run retireWithConsumerMutantTest =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(setConsumer({ slot: 0, generation: 0 }))
      .then(retireWithConsumerMutant({ slot: 0, generation: 0 }))
      .expect(not(ConsumerEntriesRetained))

  run tokenBlindRefusalMutantTest =
    init
      .then(retainAuth({ slot: 0, generation: 0 }))
      .then(setConsumer({ slot: 0, generation: 0 }))
      .then(tokenBlindRefusalMutant({ slot: 0, generation: 0 }))
      .expect(not(RefusalValid))
}
