/**
 * Deterministic production correspondence paths for the retention-budget
 * availability relation. The wrapper binds the production-shaped domain
 * (two slots, three generations, budget two) with trusted fixture inputs:
 * model generations start at one because slot-table generations start at
 * one; operation (slot, generation) pairs map to real slot-table tokens
 * produced in scripted order; consumer flags map to a fixture outstanding
 * set; per-slot budgets are parameters. Each staged action below is one
 * model transition the Rust bridge driver mirrors against the real
 * ReleaseAuthorizationLedger; probe paths assert rejection purity.
 *
 * Non-claims: same finite-proof ceilings as the delegated relation; the
 * occupied-sweep cap-3 pinpoint stays model-level evidence; probe token
 * values for pure availability checks are fixture-bound, not slot-table
 * produced.
 */
module RetentionBudgetAvailabilityConnect {
  import RetentionBudgetAvailabilityTypes.* from "../../models/quint/RetentionBudgetAvailability"
  import RetentionBudgetAvailability(
    SlotIndices = Set(0, 1),
    Generations = Set(1, 2, 3),
    Operations = Set(
      { slot: 0, generation: 1 },
      { slot: 0, generation: 2 },
      { slot: 0, generation: 3 },
      { slot: 1, generation: 1 },
      { slot: 1, generation: 2 },
      { slot: 1, generation: 3 },
    ),
    CapacityPerSlot = 2,
  ).* from "../../models/quint/RetentionBudgetAvailability"

  var stage: int

  action connectInit: bool = all {
    stage' = 0,
    init,
  }

  action retainS0G1: bool = all {
    stage' = stage + 1,
    retainAuth({ slot: 0, generation: 1 }),
  }
  action retainS0G2: bool = all {
    stage' = stage + 1,
    retainAuth({ slot: 0, generation: 2 }),
  }
  action retainS0G3: bool = all {
    stage' = stage + 1,
    retainAuth({ slot: 0, generation: 3 }),
  }
  action retainS1G1: bool = all {
    stage' = stage + 1,
    retainAuth({ slot: 1, generation: 1 }),
  }
  action retainS1G2: bool = all {
    stage' = stage + 1,
    retainAuth({ slot: 1, generation: 2 }),
  }
  action retainS1G3: bool = all {
    stage' = stage + 1,
    retainAuth({ slot: 1, generation: 3 }),
  }
  action consumeS0G1: bool = all {
    stage' = stage + 1,
    setConsumer({ slot: 0, generation: 1 }),
  }
  action consumeS0G2: bool = all {
    stage' = stage + 1,
    setConsumer({ slot: 0, generation: 2 }),
  }
  action consumeS1G1: bool = all {
    stage' = stage + 1,
    setConsumer({ slot: 1, generation: 1 }),
  }
  action consumeS1G2: bool = all {
    stage' = stage + 1,
    setConsumer({ slot: 1, generation: 2 }),
  }
  action dischargeS0G1: bool = all {
    stage' = stage + 1,
    clearConsumer({ slot: 0, generation: 1 }),
  }
  action requestS0G3: bool = all {
    stage' = stage + 1,
    requestAuth({ slot: 0, generation: 3 }),
  }
  action requestS0G1: bool = all {
    stage' = stage + 1,
    requestAuth({ slot: 0, generation: 1 }),
  }
  action retireS0G1: bool = all {
    stage' = stage + 1,
    retireOp({ slot: 0, generation: 1 }),
  }

  // The model has no invalid transition. This no-op path lets Connect
  // verify that a real unknown-identity probe fails clean without changing
  // ledger state.
  action probeUnknownObserve: bool = all {
    state' = state,
    stage' = stage + 1,
  }
  action retainOccupiedS0G2: bool = all {
    stage' = stage + 1,
    retainAuth({ slot: 0, generation: 2 }),
  }
  action refusalStep: bool = any {
    all { stage == 0, retainS0G1 },
    all { stage == 1, consumeS0G1 },
    all { stage == 2, retainS0G2 },
    all { stage == 3, consumeS0G2 },
    all { stage == 4, requestS0G3 },
  }
  action sweepStep: bool = any {
    all { stage == 0, retainS0G1 },
    all { stage == 1, consumeS0G1 },
    all { stage == 2, retainS0G2 },
    all { stage == 3, consumeS0G2 },
    all { stage == 4, dischargeS0G1 },
    all { stage == 5, retainS0G3 },
  }

  action exhaustedStep: bool = any {
    all { stage == 0, retainS1G1 },
    all { stage == 1, consumeS1G1 },
    all { stage == 2, retainS1G2 },
    all { stage == 3, consumeS1G2 },
    all { stage == 4, retainS1G3 },
  }

  action reuseStep: bool = any {
    all { stage == 0, retainS0G1 },
    all { stage == 1, consumeS0G1 },
    all { stage == 2, retainS0G2 },
    all { stage == 3, consumeS0G2 },
    all { stage == 4, dischargeS0G1 },
    all { stage == 5, retainS0G3 },
    all { stage == 6, requestS0G1 },
  }

  action occupiedStep: bool = any {
    all { stage == 0, retainS0G1 },
    all { stage == 1, consumeS0G1 },
    all { stage == 2, retainS0G2 },
    all { stage == 3, consumeS0G2 },
    all { stage == 4, dischargeS0G1 },
    all { stage == 5, retainOccupiedS0G2 },
  }

  action retireStep: bool = any {
    all { stage == 0, retainS0G1 },
    all { stage == 1, retireS0G1 },
  }

  action identityStep: bool = any {
    all { stage == 0, retainS0G1 },
    all { stage == 1, probeUnknownObserve },
  }
}
