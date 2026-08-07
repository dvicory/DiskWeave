## Why

DiskWeave needs one frontend-neutral request and event vocabulary before Linux ublk, macOS, simulator, or any async runtime can be added. Without it, ordering, flush, FUA, abandonment, and error behavior would be redefined independently by each adapter.

## What Changes

- Define normalized byte-range block requests and frontend lifecycle events.
- Define validation for operations, ranges, buffers, topology epochs, ordering, and durability intent.
- Preserve preflush, FUA, explicit flush, quiescence, loss, recovery, and abandonment semantics without silently weakening them.
- Define deterministic adapter completion and capability behavior for the initial read/write/flush/write-zeroes/discard-disabled profile.
- Keep portable semantics free of ublk, FSKit, io_uring, file descriptors, runtime handles, and frontend tags.

## Capabilities

### New Capabilities

- `normalized-block-semantics`: Frontend-neutral block requests, lifecycle events, validation, ordering, durability intent, and adapter conformance behavior.

### Modified Capabilities

None. No existing OpenSpec capabilities are present.

## Impact

- Adds the first portable semantic library and conformance tests.
- Establishes the request/event seam used by the simulator and later Linux/macOS adapters.
- Does not implement a frontend, raw-store executor, parity math, database, or persistent format.
