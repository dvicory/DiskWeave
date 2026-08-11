## Why

Adversarial review found that the M9 implementation still converts uncertainty into stronger claims at several correctness boundaries: unresolved SQLite commits can be observed before reconciliation, declarative policy plus parity agreement can still expose an executable fresh-state path, member bindings and publication identity depend on incomplete or collection-ordered evidence, and Linux endpoint discovery is not array-scoped. The milestone and verification record consequently overstate executable recovery and publication evidence.

## What Changes

- Make recovery inspection surface unresolved commit intent as reconciliation-required before exposing any manifest-derived classification, and preserve that classification through the operator result boundary.
- Refuse the current all-metadata-lost/all-data-present transition because neither explicit new-lineage adoption nor prior-lineage recovery has authority-reestablishment semantics in v0.8.
- Bind opened stores to exact topology assignments by stable store identity, assess publication identity from complete admitted member observations, and make the fingerprint independent of collection order.
- Scope live ublk discovery by admitted array publication identity and reject stale/different publications for the same array.
- Correct M9, verification, evidence, and traceability claims to the behavior actually proved.
- Record the lost-custody authority model as a v0.9 architecture boundary without selecting either explicit new-lineage adoption or prior-lineage recovery.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `recovery-state-semantics`: require reconciliation of durable unresolved commit intent before read-only inspection interprets candidate state.
- `metadata-loss-recovery`: make the current lost-custody all-data-present path explicitly non-executable until a future authority model is selected.
- `operator-recovery`: preserve reconciliation-required outcomes and refuse apply when current authority cannot be re-established.
- `anchorless-topology-identity`: require store/member bindings and topology-derived identities to remain stable under collection reordering.
- `healthy-portable-io`: derive publication identity and member assessments from exact admitted assignments and complete observation sets.
- `linux-ublk-frontend`: make live endpoint discovery array-scoped and reject conflicting or stale publications.

## Impact

Affected code: `crates/dwv-recovery-sqlite`, `crates/dwv-recovery`, `crates/dwv-service`, `crates/dwv-frontend-ublk`, `src/operator.rs`, and adversarial tests. Affected maintained records: canonical delta specs, M9, the M9 verification record, verification manifest/evidence, and one architecture decision record. No persistent-format compatibility is preserved; early-development clean cutover applies.