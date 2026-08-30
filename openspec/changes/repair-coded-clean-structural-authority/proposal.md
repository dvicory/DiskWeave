## Why

The archived `2026-08-29-repair-coded-clean-lifecycle-authority` change synchronized the intended coded-CLEAN contract, but a later exact-snapshot external review found that production APIs still permit partial durable successors and consumer-forged authority. Preserve the canonical contract and historical archive while repairing implementation conformance and restoring the external completion gate.

## What Changes

- Require the recovery-store boundary to reject incomplete or compositionally unsafe coded-CLEAN semantic transition groups before any durable member mutation. `CLEAN`, later-cut, and refused/inherited invalidation-plus-removal each commit only as their complete owner-authorized successor, including every owner-required selected-state effect; omission based on already-conservative state requires exact opaque current owner proof. A transaction containing coded semantic groups contains no ordinary recovery mutation, and multiple coded groups compose only when they name distinct captures and require compatible selected-state postconditions.
- Remove production bypasses that allow prerequisite-only coded release, consumer-issued Included lifecycle evidence, or coded claims not bound to the exact admitted member/range/request, operation identity/generation, topology, and coding profile to stand in for canonical owner authority.
- Make accepted Connect paths use the same geometry, lifecycle, release, and complete recovery-transaction producers as production; narrow evidence to executable correspondence.
- Preserve current future-inclusive scope, capture-wide persistence aggregation, compacted-history quarantine, live `CleanKnown` closure, known-versus-unknown reconciliation, and inherited conservative abandonment behavior.
- Implement finite capture exhaustion as retained nonterminal backpressure. Capacity alone does not enter `Recovering`, discard evidence, duplicate admission, or become a terminal/filesystem-visible failure; the retained operation may progress from its existing admission state after durable capacity release and another drive.
- Record the Round-5 external `NO-GO` as invalidating the prior completion claim. Require a fresh exact-snapshot external `GO` before this corrective change may be synchronized or archived and before the reopened coded-CLEAN Beads may close.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dirty-integrity-invalidation`: Clarify that the complete owner-authorized coded-CLEAN semantic successor is the unit accepted by the durable recovery boundary; partial subsets must be rejected before commit.

## Impact

- Affected implementation: coded authority, coded-CLEAN coordinator and recovery-store transaction validation, healthy portable service integration, Connect correspondence, tests, manifest, and maintained evidence.
- Preserved authority: `openspec/specs/dirty-integrity-invalidation/spec.md` remains current; the archived predecessor remains historical evidence and is not rewritten.
- Workflow gate: `dwv-3vz`, `dwv-nto.9.3`, `dwv-nto.9.4`, and `dwv-nto.9` remain open through implementation and fresh external review. No canonical sync, archive, or Bead closure occurs without that external `GO`.
- Out of scope: scan-independent writable startup, fresh-basis currentization, shutdown, background rollover, platform transport, and long-horizon resource governance.
