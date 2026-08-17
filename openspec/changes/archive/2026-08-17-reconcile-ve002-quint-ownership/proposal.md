## Why

The completed VE-002 canary made Quint authoritative for the exact bounded reference transition relation, but the current `explicit-transaction-machine` specification still duplicates that relation in its durable-intent, clean/checkpoint, failure, and trace requirements. It also mixes canonical protocol semantics with the finite bounds of the verification instance. The duplicate wording obscures which product invariants remain independently owned and lets a finite evidence run look like a product limit.

## What Changes

- Retain `transactions-emit-normalized-semantic-actions` for the independent normalized action vocabulary, semantic evidence fields, and backend-fanout boundary.
- Retain `reference-traces-are-deterministic-and-implementation-independent` as the sole OpenSpec declaration of delegated Quint authority and the implementation-comparison boundary.
- Remove the durable-intent, clean/checkpoint, and failure/abandonment requirement blocks because their exact guards, outcomes, action ordering, and release sequencing are delegated to Quint while their independent product invariants already belong to dirty/integrity, recovery-state, store-operation, lifecycle, and healthy-service requirements.
- Remove exact action-order wording from the normalized-action requirement; Quint owns the exact machine relation.
- State separately that the canonical `RecoveryProtocol` module in `verification/quint/RecoveryProtocol.qnt` owns the parameterized protocol semantics, while the separate `RecoveryProtocolAnalysis` module with one obligation, two regions, two stores, and `MaxDepth = 8` is only the finite verification instance used by VE-002 evidence.
- Rewire current and active dependents, evidence mappings, and maintained references from removed requirement IDs to their actual owner requirements or the retained reference-trace requirement.
- Keep the current/proposed distinction: prepare the correction for review, then sync and archive only after the authority boundary is accepted.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `explicit-transaction-machine`: retain only independently meaningful action-normalization and delegated-reference-trace contracts.
- `healthy-portable-io`: remove its dependency on the deleted transaction-outcome requirement while preserving lifecycle, operation-slot, and dirty/restart ownership.

## Impact

This is a semantic ownership and documentation change. It changes no production API, persistent format, model source, or runtime behavior. It removes duplicate OpenSpec semantics, distinguishes protocol authority from evidence-instance bounds, rewires dependent references, and leaves dirty-state, typed-fence, persistence, operation-lifetime, frontend-abandonment, and recovery-authority decisions with their current owners.

