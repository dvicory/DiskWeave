## Why

The delegated coded-range CLEAN model does not define the distinct full-lifecycle coded release, owner-authorized capture-membership compaction, and phase-specific capture-retirement transitions required by the canonical requirement. After restart, persisted data can be mistaken for live authority, and ordinary successful writes can leave empty `CleanKnown` captures without a bounded retirement path. Those gaps can bypass lifecycle ownership or exhaust finite capture capacity during an indefinitely long healthy workload.

## What Changes

- Require final coded-claim removal to consume the canonical healthy-portable-io owner's exact-generation `ReleaseAllowed`; safe slot `Reclaimable` or a lower-level reclaim witness remains only one prerequisite.
- Keep one coded-geometry owner for complete per-write claims and complete future-inclusive capture scope; persisted scope/proof flags remain data and require fresh owner revalidation after reopen.
- Bind owner-issued compaction authority to the exact capture/topology, durable predecessor, released operation generations and receipts, complete pre-compaction obligations, facts forgotten versus retained, and exact replacement bounded summary/frontier; unresolved/Open/unknown captures retain exact membership and never compact it into history.
- Define capture-specific, predecessor-bound cleanup authority for newly refused, persisted `Refused`, and `CleanKnown` phases; identical geometry never makes authority interchangeable between captures.
- Permit an empty `CleanKnown` capture to retire without an unrelated later write only after the recovery/dirty-integrity owner proves the durable CLEAN result and all release, membership, pending-cut, and retained-history obligations are independently preserved in the exact successor.
- Apply one predecessor-bound prepare → durable commit → install rule to release, compaction, refused cleanup, and `CleanKnown` cleanup. Known non-commit preserves the predecessor; true uncertainty discards process-local authority and reopens to accept only the exact predecessor or exact proposed successor.
- Require focused owner-issued live transition preparations to carry correctness authority across irreversible steps; persisted DTOs, raw identities, phases, booleans, generations, counters, and expected-state equality remain data rather than authority.
- Keep production authority unavailable through ordinary test features; tests use real owner producers or an isolated adapter that cannot become production authority.
- Make the delegated model's release, compaction, and phase-specific cleanup inputs explicit abstractions of those external owner-issued capabilities, and add unresolved-retention/backpressure, geometry-oracle, bounded-retirement, release/compaction/refused/clean-cleanup lost-acknowledgement, stale/cross-binding, and consumer-boundary evidence.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `dirty-integrity-invalidation`: Complete the delegated recovery-CLEAN cleanup lifecycle and define authoritative reopened-capture refusal and retirement.

## Impact

- Delegated model: `models/quint/CodedRangeClean.qnt` and semantic verification profiles.
- Portable implementation: `dwv-recovery` capture capabilities and `dwv-service` lifecycle/reopen coordination.
- Verification evidence: bounded Quint checks, real-owner production correspondence, independent coded-geometry oracle coverage, unresolved-retention and healthy-retirement bounds, transition-ordering checks, known-failure and true lost-acknowledgement tests for every authority-changing durable transition, and stale/cross-capture/cross-revision negatives.
- No persistent compatibility guarantee is added; durable state remains conservative and generation checked.
