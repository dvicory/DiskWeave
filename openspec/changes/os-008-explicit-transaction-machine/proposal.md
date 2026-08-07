## Why

The recovery and simulator contracts exist, but there is no small executable oracle that orders range ownership, durable dirty/integrity intent, home I/O, fences, checkpoints, abandonment, and release. Without that machine, later implementations can accidentally make a home write before its recovery intent or report clean after an incomplete fence.

## What Changes

- Add a dependency-free `dwv-transaction-ref` explicit reference machine.
- Define normalized semantic actions and results independent of backend child I/O, SQLite, or runtime futures.
- Enforce the first safe write sequence: acquire, persist dirty/invalidate intent, read/compute/write, flush/fence, checkpoint/clear, release.
- Model failed/uncertain recovery commits, backend failures, abandonment, daemon crash, and reconciliation-required terminal states.
- Emit deterministic action traces and test every legal transition plus forbidden orderings.

## Capabilities

### New Capabilities

- `explicit-transaction-machine`: Auditable transaction ordering and normalized action traces.

### Modified Capabilities

None.

## Impact

- Adds `dwv-transaction-ref` over `dwv-core`, `dwv-codec`, `dwv-recovery`, and `dwv-store` semantic values.
- Does not select `procmachines`; OS-009 may compare it later against this oracle.
- Does not perform device I/O or claim physical durability.
- Unlocks dirty/integrity ordering and healthy portable I/O.
