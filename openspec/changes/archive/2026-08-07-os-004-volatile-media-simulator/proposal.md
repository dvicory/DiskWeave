## Why

The portable contracts now describe requests, stores, completion evidence, operation lifetimes, and reference parity math, but they need an executable fault model before transaction and recovery work can claim crash safety. OS-004 adds a deterministic byte-level simulator that separates durable media, acknowledged volatile writes, pending operations, and completion delivery.

## What Changes

- Add a dependency-free `dwv-sim` library behind the portable workspace boundary.
- Model deterministic read, write, flush, FUA-like write, write-zeroes, discard, short, torn, failed, uncertain, duplicate, disappearance, daemon-crash, controller-reset, latent-corruption, and power-loss outcomes.
- Model recovery-state commit failures and torn parity-envelope copies as semantic state faults without depending on SQLite or an envelope implementation.
- Add schedules that can be serialized as seed-free reproducers and greedily minimized.
- Expose durable media, recovery-state, parity-envelope snapshots, delivery evidence, and bounded schedule coverage for invariant checks and later transaction-machine work.

## Capabilities

### New Capabilities

- `volatile-media-simulator`: deterministic media state, fault schedules, crash/power-loss transitions, and minimized reproducers.

### Modified Capabilities

None. The simulator consumes the completed OS-001, OS-002, and OS-003 contracts.

## Impact

- Adds portable executable evidence for crash and power-loss semantics.
- Does not implement a transaction machine, SQLite, a real filesystem, or hardware durability claims; recovery state and parity-envelope copies are semantic fault-model fixtures only.
- Does not change any data-member or persistent on-disk format.
