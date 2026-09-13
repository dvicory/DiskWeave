## Why

Established release authorizations are retained per slot index under an explicit budget, with refusal-before-admission, discharge-driven sweep, and exact retirement already implemented and adversarially reviewed. No canonical bounded relation states that availability contract, so its exact blocking, sweep, and retirement transitions live only in code and tests. This change names the reviewed `models/quint/RetentionBudgetAvailability.qnt` relation as the sole authority for that finite availability surface without claiming that the model owns authorization composition, slot lifecycle, coded capture mechanics, capacity values, or any surrounding owner.

## What Changes

- Delegate only the bounded retention-budget availability subrelation exposed by `models/quint/RetentionBudgetAvailability.qnt`: exact-token observe, full-and-consumed blocking with refusal that changes nothing, lowest-discharged-first retirement inside every remember, occupied replace without sweep, sweep-and-insert into room, failure-with-sweep exhaustion, and exact retirement with verdict identity.
- Keep accepted-work preservation through reconciliation waits, LifecycleRelease composition, operation-slot lifecycle, coded-capture consumer mechanics, per-slot budget values, slot-table occupancy, transaction/recovery semantics, WriteDriver continuation, physical stores, and governor capacity policy authoritative outside the delegated model. Budgets stay parameters; consumer flags stay opaque owner observations.
- Keep the LifecycleRelease handoff composition, staged phase-cut profiles, scenario profiles, and mutant canaries as evidence-only checks, not delegated semantics.
- No product-code change, no persisted-format change, no operator-facing policy change.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `healthy-portable-io`: add a bounded delegation for the exact retention-budget availability relation while retaining authorization composition, budget values, slot occupancy, and all surrounding owner semantics outside the model. No existing requirement prose is removed.

## Impact

- Affected spec: `healthy-portable-io`.
- Affected verification: Quint model profiles, handoff composition, verification manifest, and maintained portable evidence (at Connect phase).
- No compatibility or persistent-format change.
- OpenSpec synchronization occurs only after delegation artifacts receive adversarial review and strict validation passes. The source-local `dwv:req` marker lands only at the atomic currentization step.
