## Why

Parity can reconstruct a known erasure but cannot identify arbitrary silent corruption. The handoff therefore promotes checksums to an independent integrity plane: data, P, and Q extents have generational evidence, and a digest is valid only when it covers fenced durable bytes of the named generation.

## What Changes

- Add a portable checksum-extent and checksum-record model for data and parity targets.
- Define profile IDs, digest state, target content generations, and migration seams without freezing a final format.
- Add asynchronous revalidation with generation checks, stale-result rejection, and a safe full-overwrite optimization.
- Integrate checksum validity with OS-010 invalidation while keeping parity cleanliness independent.
- Add deterministic concurrent-write/hash, crash, power-loss, metadata-loss, and benchmark fixtures.

## Capabilities

### New Capabilities

- `checksum-plane`: Generational data/parity checksum records and asynchronous revalidation.

### Modified Capabilities

- None. The checksum capability consumes OS-010's invalidation contract; it does not modify an archived capability in this change.

## Impact

- Adds checksum semantic modules to `dwv-recovery` or a portable integrity crate, with no frontend/runtime types.
- Adds a digest-provider seam; BLAKE3-256 and approximately 4 MiB extents remain the handoff’s provisional P-007 choice until benchmark evidence.
- Extends OS-010 and later healthy I/O with checksum invalidation/revalidation actions.
- Does not implement scrub/repair (OS-014/017), parity envelope format, or Linux-specific hashing executors.
