## Why

The parity engine must interact with files and devices through a store contract that exposes exact ranges, partial completion, persistence evidence, capability limits, identity observations, and uncertain outcomes. Operation-slot ownership must also be defined before backend I/O or cancellation can be implemented safely.

## What Changes

- Define the random-access store semantic interface and structured completion evidence.
- Define capability and safety profiles for simulation, portable demo, production read-only, and production write-safe operation.
- Define operation-slot tokens, generations, child-operation ownership, duplicate/stale completion handling, and safe reclamation.
- Define bounded admission, buffers, retries, and backpressure expectations.
- Keep concrete filesystems, io_uring, SQLite, and frontend APIs behind replaceable adapters.

## Capabilities

### New Capabilities

- `store-operation-contracts`: Random-access stores, capability evidence, operation slots, uncertain completion, identity observations, and bounded resource semantics.

### Modified Capabilities

None. No existing OpenSpec capabilities are present.

## Impact

- Adds portable store and operation-lifetime contracts used by the simulator and later executors.
- Establishes safe completion ownership before file-backed or Linux I/O is introduced.
- Does not implement a real store, SQLite persistence, ublk, io_uring, or cancellation against a kernel API.
