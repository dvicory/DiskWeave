## Why

The recovery-state semantics and reference transaction machine define the pieces of a safe write, but they do not yet provide the portable protocol that couples dirty-region intent with checksum invalidation. The handoff makes this ordering the first write-safety gate: no protected home mutation may begin until the affected region is durably dirty and every affected `VALID` checksum is durably stale.

## What Changes

- Add a portable dirty/integrity invalidation capability over the recovery-state and transaction contracts.
- Model first-write, already-dirty, boundary-crossing, checkpoint, flush, crash, and restart transitions with captured region and checksum generations.
- Make recovery-commit failure, uncertain completion, and post-home I/O failure conservative and observable.
- Add simulator schedules proving that no failure path reports false `CLEAN` or preserves a stale `VALID` digest.
- Keep session-level dirty state and fine-grained region state independent; do not introduce a journal or Linux-specific executor.

## Capabilities

### New Capabilities

- `dirty-integrity-invalidation`: Durable dirty-region intent and atomic checksum invalidation for protected mutations.

### Modified Capabilities

None.

## Impact

- Extends `dwv-recovery` with region/checksum-generation transitions and conservative checkpoint evidence.
- Connects `dwv-transaction-ref` action ordering to recovery-state intent and fence results.
- Adds deterministic `dwv-sim` fault schedules and model tests.
- Uses no OS, runtime, database, or third-party storage type in the portable semantic API.
- Implements the handoff Phase 1 OS-010 contract and unlocks OS-011 and OS-013.
