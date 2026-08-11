## Context

See `proposal.md`. Canonical owners already require conservative recovery observations, independent authority, stable assignment identity, complete member observations, and admitted publication identity. The defect is a conforming implementation gap plus overclaimed milestone/evidence prose, not a new recovery authority model.

## Goals / Non-Goals

**Goals:**

- Preserve uncertainty at each existing semantic boundary.
- Make the smallest clean cutover across every caller and evidence artifact.
- Leave a focused adversarial regression for each C1-C9 finding that is executable.

**Non-Goals:**

- Define or implement explicit new-lineage adoption.
- Define or implement prior-lineage recovery after DiskWeave lost custody.
- Broaden the one-data/one-parity Linux profile, add read-only publication, or claim raw-device/power-loss durability.

## Decisions

### Reconcile inspection without mutating semantic state

SQLite inspection will interpret an existing commit-intent sidecar before returning manifest-derived state. It will compare the current complete manifest digest to the exact prior/proposed intent digests using the current read-only schema path. Exact prior or proposed state may be reported; corrupt intent, unreadable state, or neither digest becomes `ReconciliationRequired`. Inspection does not initialize, migrate, write a manifest, or remove the sidecar. Writable open retains the existing reconciliation cleanup after it acquires the claim.

### Remove the unrepresentable authority transition

The metadata-loss matrix will retain stable case IDs but make every current case that could create fresh state from uncertified all-data/parity agreement refuse with an authority-unavailable action. The matrix version changes because rendered plans change. Certified receipt cases remain unavailable as already specified. No replacement authorization token or compatibility shim is added.

### Bind by stable semantics once

Service assembly will build opened-member bindings by matching each topology assignment's `StoreId`, then validate every assignment field. Publication identity will use those admitted bindings, assess each member from its complete observation set, and hash assignments in stable semantic order. No parallel identity model is introduced.

### Scope discovery by array before full identity

Publication identity will expose its admitted `ArrayId`. Linux publication metadata will persist both array identity and the complete publication identity. Discovery will ignore other arrays, return the exact match, and report reconciliation-required if the requested array has any live stale/conflicting identity. Multiple exact publications for the requested array are also reconciliation-required.

### Preserve history and correct maintained claims

Archived OpenSpec changes remain immutable historical records. The new change and architecture decision record will name the superseded attempted transition and the fail-closed v0.9 boundary. M9 and its verification record will state only the delivered observation, verification, refusal, baseline/admission behavior for already-authoritative state, and actual Linux publication evidence.

## Risks / Trade-offs

- Previously rendered matrix plans and proposal IDs change; this is an intentional early-development clean break.
- A read-only inspection may leave a conclusively reconciled sidecar for the next writable open to remove; this preserves observational inspection at the cost of harmless repeated comparison.
- Array-scoped discovery requires publication metadata from this cutover. Old metadata without array identity is not treated as a match.
