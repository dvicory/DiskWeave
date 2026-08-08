## Why

OS-015 can recover metadata when all single-parity members survive, but the handoff's Phase 1 gate also requires safe reads and a replacement image when one data member is a known erasure. OS-016 supplies that portable/macOS-capable path without weakening the rule that dirty, unknown, ambiguous, or beyond-tolerance ranges must fail closed.

## What Changes

- Add evidence-gated single-XOR degraded reads for one known missing data slot, with exact reconstruction and explicit degraded telemetry.
- Require unambiguous validated topology, parity-clean or replay-proven range evidence, acceptable surviving integrity evidence, and a stable read-only generation before decoding.
- Add an offline rebuild engine that writes only a separate replacement target, verifies readback and the parity equation, and checkpoints deterministic progress in recovery state.
- Resume an interrupted rebuild from its durable cursor without rewriting already checkpointed ranges, then require a complete final verification before topology promotion can be requested.
- Preserve stable slot/coding position while assigning a new assignment instance for the replacement; keep topology publication as a separate recovery transaction.
- Add macOS regular/sparse-file fixtures proving the replacement byte-equals the reference image and remains directly readable after DiskWeave stops.
- Refuse degraded writes, online rebuild races, P/Q decoding, two-erasure recovery, Gate H certificate shortcuts, and Linux frontend claims in this change.

## Capabilities

### New Capabilities

- `degraded-read-offline-rebuild`: Known-erasure single-XOR reads and separate-target, resumable, verified offline replacement rebuild.

### Modified Capabilities

- `recovery-state-semantics`: Make rebuild identity, source topology/generation, deterministic cursor, completion evidence, and replacement promotion prerequisites explicit durable semantic state.

## Impact

- Extends portable recovery semantics with a typed rebuild checkpoint rather than an opaque maintenance cursor.
- Extends `dwv-service`/`dwv-verify` with a read-only known-erasure decode boundary and offline rebuild orchestration over existing exact-range store traits.
- Adds macOS regular-file acceptance tests using existing workspace crates and the in-tree single-XOR reference codec.
- Does not add a third-party codec/runtime/database dependency or change a data-member byte format.
