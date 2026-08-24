## Why

Current requirements separately define transaction release, operation-slot reclamation, child and store evidence, recovery reconciliation, and healthy-service failure composition. They do not define the single generation-qualified authorization that a later consumer needs to know that all applicable owners have permitted semantic release. Without that boundary, a consumer could mistake `RangeReleased` or physical slot removal for complete release, or recreate the owner conjunction itself.

## What Changes

- Add one narrowly scoped requirement under the existing `healthy-portable-io` capability to own composition of a generation-qualified `ReleaseAllowed(operation-generation)` authorization only for operations whose admission includes an owner-approved releasable claim or obligation that another component must be permitted to drop; routine reads, flushes, and other operations without that obligation SHALL NOT receive the authorization merely because their slots terminate.
- Require terminal or authoritative reconciliation of operation/media effect, every child, required reconciliation, the applicable transaction semantic release boundary, any required recovery-owned reconciliation facts, and healthy-service basis-coherence conformance before the authorization may become true.
- Make the authorization monotonic and irrevocable for one exact generation: repeated observations are harmless, later cleanup or topology/recovery observations do not revoke it, and an authorization never applies to a later slot reuse. A wrong input is an upstream owner/reconciliation failure, not a normal lifecycle revocation.
- Distinguish semantic authorization from physical resource cleanup. Safe `Reclaimable` state is required; successful subsequent slot/resource reclamation is not. A failed reclamation retains exact generation/evidence/resources for retry without revoking an already-established semantic authorization or permitting generation reuse.
- State that `RangeReleased` alone and slot reclamation/removal alone are insufficient, unresolved operation/media-effect/child/basis state withholds authorization, and unresolved CLEAN-capture state is unrelated.
- Preserve existing owner boundaries: `explicit-transaction-machine` owns the semantic transaction/release relation; `store-operation-contracts` owns child outcomes, operation-slot lifetime, reconciliation, and safe `Reclaimable`; `recovery-state-semantics` owns its reconciliation observations; healthy-portable-io establishes an explicit basis-conformance observation and composes the final authorization without redefining any contributing predicate.
- Add bounded scenarios and implementation/evidence tasks for the lifecycle observation, including authoritative reconciliation of a retained older generation and observability of short/failed certificates; the exact bounded composition relation remains delegated to `models/quint/LifecycleRelease.qnt`; this change does not implement Connect, coded-range/CLEAN coordination, or owner models.

## Capabilities

### New Capabilities

None. The repair belongs to the existing healthy-portable-io service-composition capability.

### Modified Capabilities

- `healthy-portable-io`: define the owner-composed generation-qualified semantic release authorization and its conservative withholding rules.

### Composing Capabilities (unchanged owners)

- `explicit-transaction-machine` continues to own the exact semantic transaction and `releaseRange` relation.
- `recovery-state-semantics` continues to own recovery generations, durable commit observations, and reconciliation.
- `store-operation-contracts` continues to own child outcomes, operation-slot lifetime, safe `Reclaimable` state, resource accounting, and generation reuse.
- `normalized-block-semantics` continues to own canonical request identity and abandonment meaning.
- Healthy-service basis coherence remains a service-owned composition/conformance predicate derived from existing transaction/store/recovery observations; no separate basis-lifecycle owner is created.

## Selection and Independence

The repair closes one missing composition boundary: the portable healthy service must expose whether the existing owners have jointly permitted semantic release for one operation generation. It does not transfer ownership of any contributing predicate and does not create a lifecycle umbrella capability. Coded-clean will later consume this fact; it is not a dependency of the semantic repair.

The authorization is deliberately separate from physical resource cleanup. The operation-slot owner must first establish safe `Reclaimable` state, but a later accounting or resource-release failure is a retryable cleanup failure and does not retroactively revoke semantic authorization. This preserves the distinction between semantic media/claim release and physical bookkeeping while keeping exact generation/evidence/resources addressable.

## Rejected Adjacent Scope

- Rust or Connect implementation of the observation;
- coded-range coordination, recovery-CLEAN closed-set semantics, or `CodedRangeClean.qnt`;
- changes to `RecoveryProtocol.qnt` or the transaction release relation;
- coded-clean rebasing or canonical synchronization;
- startup, shutdown, publication, currentization, or scan-independent work;
- a new umbrella lifecycle capability or generic effect/release framework.

## Impact

The change adds one semantic composition contract, its delegated bounded relation, and bounded acceptance evidence for a future implementation. It leaves all contributing owner requirements and executable models unchanged. Later implementation must expose the composed fact from lifecycle/service ownership so a consumer observes it rather than reconstructing terminalization, reconciliation, transaction/recovery release, operation-slot, or basis policy.