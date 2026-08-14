## Why

DiskWeave already owns bounded, non-mutating recovery inspection semantics, but it has no independently invocable operator tool that proves a documented recovery artifact can be inspected without the production service or its private state. A small read-only inspection capability advances C8/U26 while preserving the experimental-format and non-authorization boundaries.

## What Changes

- Add an independent command that inspects one recovery-state artifact through the portable observational boundary without acquiring writable ownership, initializing, migrating, repairing, or changing the artifact.
- Preserve distinct absent, supported, corrupt-or-unreadable, unsupported, migration-required, and reconciliation-required outcomes.
- Emit one bounded, versioned semantic result in human and structured forms; supported state includes the portable recovery manifest, while failures retain their exact conservative classification and available format-layer/version facts.
- Make the command independent of production service, operator workflow, and frontend private state.
- State explicitly that successful inspection authorizes no writable interpretation, publication, payload mutation, recovery mutation, repair, migration, parity claim, lineage claim, or stable-format claim.

## Capabilities

### New Capabilities

- `independent-recovery-inspection`: Independently inspect a recovery-state artifact and report bounded portable semantics without mutation or stronger authority.

### Modified Capabilities

None.

## Impact

- Implementation will add a small executable surface over the existing recovery semantic interface and read-only adapter; its dependency graph must exclude production service, operator, and frontend crates.
- The result contract requires deterministic human and versioned structured renderings with bounded output and deterministic outcome-to-process-status mapping.
- Focused evidence must cover every inspection disposition, unchanged artifacts, bounded hostile inputs/output, equivalent human/structured meaning, and the production-private-state dependency boundary.
- Equation verification, parity construction, candidate export, recovery-plan explanation, migration execution, damaged-state repair, and stable-format graduation remain out of scope.
