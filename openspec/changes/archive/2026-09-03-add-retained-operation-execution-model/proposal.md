## Why

The portable write path already retains a service driver after submission, accepts physical work through an exact proof boundary, and delivers normalized results later. Existing Rust tests cover individual cases, but no canonical bounded relation states the combined driver-local and operation-slot ownership transitions or checks their implementation correspondence.

This change makes that bounded relation explicit and reviewable without claiming that the model owns topology, persistence, recovery, resource capacity, frontend behavior, or physical durability.

## What Changes

- Add a reviewed Quint relation for one generation-qualified retained operation with driver-local work, slot-owned child submission, exact identity correlation, delayed and reordered child delivery, conservative dispositions, abandonment, and reconciliation.
- Delegate only the finite child-correlation and reconciliation-through-`Reclaimable` relation exposed by `models/quint/PortableOperationExecutionCore.qnt`; keep `models/quint/PortableOperationExecution.qnt` as a full correspondence wrapper whose fixed protected-write action order, `TransactionMachine` semantics, explicit transaction/lifecycle release observations, post-`Reclaimable` cleanup, and all surrounding operation-slot, recovery, persistence, resource, and frontend requirements remain authoritative outside the delegated core.
- Add compact core analysis profiles plus evidence-only driver/slot-fan-in profiles, deterministic through-`Reclaimable` scenario checks, and a production `quint-connect` correspondence harness. The fixed driver order and release observations remain outside the production Connect claim.
- Record implementation-conformance, model bounds, and non-claims in the maintained verification manifest and portable verification evidence.
- Do not change persisted formats, physical store behavior, scheduling policy, or operator-facing recovery policy.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `store-operation-contracts`: add a bounded delegation for the exact retained-operation child-correlation relation while retaining request, buffer, frontend-tag, watermark, drain, resource, generation-reuse, persistence, and physical-store ownership outside the model. No existing requirement prose is removed because those non-modeled dimensions remain canonical.

## Impact

- Affected spec: `store-operation-contracts`.
- Affected verification: Quint model profiles, `dwv-service` Connect tests, verification manifest, and maintained portable evidence.
- No compatibility or persistent-format change.
- OpenSpec synchronization occurs only after model, delegation artifacts, and implementation receive adversarial review and focused verification.
