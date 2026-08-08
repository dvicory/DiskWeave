## Why

DiskWeave already has an explicit transaction machine that encodes the safety
oracle, while the architecture provisionally prefers `procmachines` for a
procedural async implementation. Selecting either path without an executable
comparison would turn a convenience preference into an unmeasured correctness
and maintenance dependency.

## What Changes

- Add an isolated `dwv-transaction-proc` comparison implementation using the
  pinned `procmachines` dependency.
- Drive one representative dirty-region parity write through both the explicit
  reference machine and the procedural candidate using the same semantic plan,
  action vocabulary, results, and deterministic fault schedules.
- Normalize traces so allowed batching or driver differences do not obscure
  semantic equivalence.
- Compare success, I/O failure and uncertainty, completion ordering,
  abandonment, daemon crash, power-loss persistence boundaries, duplicate
  delivery, and stale-token behavior.
- Measure allocations, synchronization operations, CPU/request, latency,
  memory, and dependency/audit surface at the comparison boundary.
- Record a reversible ADR decision: select `procmachines` only if correctness
  equivalence and cost criteria pass; otherwise retain the explicit machine.

## Capabilities

### New Capabilities

- `transaction-engine-evidence-comparison`: Evidence-backed comparison of the
  explicit transaction machine and the provisional procedural adapter.

### Modified Capabilities

<!-- No existing requirement changes. The comparison adds evidence before any
     production engine selection or public semantic-contract migration. -->

## Impact

- Adds the isolated `dwv-transaction-proc` workspace crate and a locked
  `procmachines` dependency.
- Adds comparison helpers/tests and a transaction-engine ADR/verification
  record.
- Does not replace `dwv-transaction-ref`, change the service engine, expose
  `procmachines` types, or add a runtime/kernel/filesystem dependency to the
  portable semantic core.
