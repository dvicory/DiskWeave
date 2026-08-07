## 1. Simulator contract

- [x] 1.1 Add the dependency-free `dwv-sim` workspace package and consume only portable core/store contracts.
- [x] 1.2 Define durable, volatile, pending, delivery, availability, and deterministic fault-model state.
- [x] 1.3 Define exact-range read/write/flush/FUA-like submissions and structured completion evidence.

## 2. Schedules and faults

- [x] 2.1 Implement explicit schedule steps for submission, delivery permutation, duplicate delivery, disappearance/reappearance, daemon crash, and power loss.
- [x] 2.2 Implement versioned reproducer serialization and deterministic greedy schedule minimization.
- [x] 2.3 Expose durable snapshots, delivery traces, and executable media safety invariants.

## 3. Evidence

- [x] 3.1 Add deterministic tests for ordinary-vs-durable writes, short/failed/uncertain completion, crash-vs-power-loss behavior, delivery permutation, disappearance, duplicate delivery, and reproducer round trips.
- [x] 3.2 Run `openspec validate os-004-volatile-media-simulator --json`, workspace tests, and the OpenSpec apply instructions; resolve failures.
- [x] 3.3 Mark OS-004 complete only after minimized regression schedules replay deterministically and record OS-005, OS-008, and OS-010 as successors.

## 4. Handoff Phase 0 fault-model acceptance

- [x] 4.1 Extend the normalized schedule and pending-operation model with write-zeroes, discard, torn effects, latent corruption, controller reset, and explicit backend-failure evidence.
- [x] 4.2 Add durable recovery-state and two-copy parity-envelope semantic fixtures with failure, uncertainty, and tear behavior.
- [x] 4.3 Add deterministic bounded one-range schedule enumeration and invariant/replay coverage across the expanded fault model.
- [x] 4.4 Update OS-004 requirements/design to reflect the implemented handoff scope and run focused simulator/OpenSpec validation tests.
