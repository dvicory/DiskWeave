## Why

The canonical XOR and parity-verification requirements define exact equation semantics, but no independently invocable tool can verify a bounded data/parity set without production-daemon state. A small read-only equation verifier is useful on its own for offline diagnosis and preserves the stronger boundary that an equation match is not current, historical, repair, or stable-format authority.

## What Changes

- Add one standalone, read-only independent equation-verification capability for explicitly supplied data/parity payloads and bounded protected geometry.
- Reuse the canonical XOR geometry and parity-verification owners; the new boundary owns only input binding, bounded reads, report rendering, and experimental claim scope.
- Report per-range `matched`, `mismatched`, `incomplete`, or `invalid/unsupported` dispositions with exact bounded evidence and deterministic process status.
- Preserve source bytes and all recovery state; the tool performs no repair, parity build, migration, publication, recovery, or claim promotion.
- Make human and structured output equivalent and state that file-backed or model evidence does not certify production durability or a stable persistent format.
- Keep the slice independent of C8a/U26a recovery-state inspection, U11 portable shutdown, production service/operator/frontend private state, and every broader U26b tool such as candidate export, recovery-plan explanation, migration, or damaged/unknown drills.

## Capabilities

### New Capabilities

- `independent-parity-verification`: Standalone bounded equation verification over explicitly bound payloads without production private state or mutation.

### Modified Capabilities

None. Existing XOR, parity-verification, evidence, and recovery requirements remain canonical owners; the new capability is an adapter/tool boundary and does not redefine them.

## Impact

The change adds one independent tool contract and focused evidence for bounded input validation, exact range reads, equation results, source non-mutation, renderer equivalence, and evidence-tier limits. It does not depend on independent recovery-state inspection (C8a/U26a), portable shutdown (U11), startup/currentization, history/retention, repair execution, migration, stable-format claims, or implementation results from another campaign item.
