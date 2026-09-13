/**
 * Evidence-only handoff composition: LifecycleRelease establishes,
 * the retention budget retains.
 *
 * Both owners keep their state machines; this wrapper holds only a phase
 * cursor and a fixed establish-then-retain script with trusted fixture
 * inputs. It proves the boundary agreement neither model can state alone:
 * every retained (slot, generation) was established for the matching
 * operation generation, and an established authorization that arrives at a
 * saturated consumed budget fails clean (Exhausted, state unchanged) rather
 * than lost or duplicated.
 *
 * Fixture mapping: operation "slot-a" generation g <-> slot 0 generation g.
 * Coded-capture consumer mechanics stay opaque: setConsumer stands in for
 * the recorded outstanding certificate. Outcome discipline: release-only
 * phases stutter the whole budget state, so the last budget verdict
 * (always gen0's Retained here) persists while the release pipeline
 * advances. That stutter is exact, not stale: the ledger it annotates is
 * unchanged, so every verdict-validity predicate still reads the ledger
 * the verdict was made against. Read outcome only alongside its outcomeOp.
 *
 * Non-claims: same finite-proof ceilings as both imported relations; no
 * canonical authority; no upstream truth for the fixture mapping.
 */
module RetentionBudgetAvailabilityComposition {
  import RetentionBudgetAvailabilityTypes.* from "../../models/quint/RetentionBudgetAvailability"
  import LifecycleRelease(
    OperationGenerations = Set(
      { operation: "slot-a", generation: 0 },
      { operation: "slot-a", generation: 1 },
    ),
    ApplicableGenerations = Set(
      { operation: "slot-a", generation: 0 },
      { operation: "slot-a", generation: 1 },
    ),
  ) as release from "../../models/quint/LifecycleRelease"
  import RetentionBudgetAvailability(
    SlotIndices = Set(0),
    Generations = Set(0, 1),
    Operations = Set(
      { slot: 0, generation: 0 },
      { slot: 0, generation: 1 },
    ),
    CapacityPerSlot = 1,
  ) as budget from "../../models/quint/RetentionBudgetAvailability"

  var phase: int

  action compositionInit: bool = all {
    phase' = 0,
    release::init,
    budget::init,
  }

  action compositionStep: bool = any {
    all { phase == 0, phase' = 1, release::enterApplicable({ operation: "slot-a", generation: 0 }), budget::state' = budget::state },
    all { phase == 1, phase' = 2, release::observeMediaTerminal({ operation: "slot-a", generation: 0 }), budget::state' = budget::state },
    all { phase == 2, phase' = 3, release::observeChildrenTerminal({ operation: "slot-a", generation: 0 }), budget::state' = budget::state },
    all { phase == 3, phase' = 4, release::recordRequiredReconciliation({ operation: "slot-a", generation: 0 }), budget::state' = budget::state },
    all { phase == 4, phase' = 5, release::observeReclaimable({ operation: "slot-a", generation: 0 }), budget::state' = budget::state },
    all { phase == 5, phase' = 6, release::observeTransactionSatisfied({ operation: "slot-a", generation: 0 }), budget::state' = budget::state },
    all { phase == 6, phase' = 7, release::observeRecoveryAuthoritative({ operation: "slot-a", generation: 0 }), budget::state' = budget::state },
    all { phase == 7, phase' = 8, release::observeBasisConformance({ operation: "slot-a", generation: 0 }, release::BasisConsumed), budget::state' = budget::state },
    all { phase == 8, phase' = 9, release::establishReleaseAllowed({ operation: "slot-a", generation: 0 }), budget::state' = budget::state },
    all { phase == 9, phase' = 10, release::state' = release::state, budget::retainAuth({ slot: 0, generation: 0 }) },
    all { phase == 10, phase' = 11, release::enterApplicable({ operation: "slot-a", generation: 1 }), budget::state' = budget::state },
    all { phase == 11, phase' = 12, release::observeMediaTerminal({ operation: "slot-a", generation: 1 }), budget::state' = budget::state },
    all { phase == 12, phase' = 13, release::observeChildrenTerminal({ operation: "slot-a", generation: 1 }), budget::state' = budget::state },
    all { phase == 13, phase' = 14, release::recordRequiredReconciliation({ operation: "slot-a", generation: 1 }), budget::state' = budget::state },
    all { phase == 14, phase' = 15, release::observeReclaimable({ operation: "slot-a", generation: 1 }), budget::state' = budget::state },
    all { phase == 15, phase' = 16, release::observeTransactionSatisfied({ operation: "slot-a", generation: 1 }), budget::state' = budget::state },
    all { phase == 16, phase' = 17, release::observeRecoveryAuthoritative({ operation: "slot-a", generation: 1 }), budget::state' = budget::state },
    all { phase == 17, phase' = 18, release::observeBasisConformance({ operation: "slot-a", generation: 1 }, release::BasisConsumed), budget::state' = budget::state },
    all { phase == 18, phase' = 19, release::establishReleaseAllowed({ operation: "slot-a", generation: 1 }), budget::state' = budget::state },
    all { phase == 19, phase' = 20, release::state' = release::state, budget::setConsumer({ slot: 0, generation: 0 }) },
    all { phase == 20, phase' = 21, release::state' = release::state, budget::retainAuth({ slot: 0, generation: 1 }) },
    all { phase == 21, phase' = 22, release::state' = release::state, budget::requestAuth({ slot: 0, generation: 1 }) },
  }

  val EveryRetainedWasEstablished: bool =
    budget::state.retained.get(0).forall(g =>
      release::state.authorizations.contains({ operation: "slot-a", generation: g }))

  val BudgetInvariants: bool = budget::AvailabilityInvariants
  run compositionShapeTest =
    compositionInit
      .expect(phase == 0)

  run establishedThenRetainedThenExhausted =
    compositionInit
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .expect(
        release::state.authorizations.contains({ operation: "slot-a", generation: 0 })
          and budget::state.outcome == OutcomeRetained
          and budget::state.outcomeOp == ForOperation({ slot: 0, generation: 0 })
      )
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .then(compositionStep)
      .expect(
        release::state.authorizations.contains({ operation: "slot-a", generation: 1 })
          and budget::state.outcome == OutcomeRetained
          and budget::state.outcomeOp == ForOperation({ slot: 0, generation: 0 })
      )
      .then(compositionStep)
      .then(compositionStep)
      .expect(
        budget::state.outcome == OutcomeExhausted
          and budget::state.outcomeOp == ForOperation({ slot: 0, generation: 1 })
          and budget::bucketHas(budget::state, { slot: 0, generation: 0 })
          and not(budget::bucketHas(budget::state, { slot: 0, generation: 1 }))
      )
      .then(compositionStep)
      .expect(
        budget::state.outcome == OutcomeRefused
          and budget::state.outcomeOp == ForOperation({ slot: 0, generation: 1 })
      )
}
