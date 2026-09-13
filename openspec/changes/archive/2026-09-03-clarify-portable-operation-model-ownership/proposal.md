## Why

`PortableOperationExecutionCore.qnt` is already the sole delegated authority for retained-child correlation through `Reclaimable`, but the evidence-only `PortableOperationExecution.qnt` still lives beside canonical models and reimplements child, transaction, and release state. Its location and duplicated transitions can be mistaken for a second semantic owner and can drift from `PortableOperationExecutionCore.qnt`, `RecoveryProtocol.qnt`, and `LifecycleRelease.qnt`.

## What Changes

- Keep `models/quint/PortableOperationExecutionCore.qnt` as the sole delegated retained-child correlation authority for `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`, with the existing fixed six-child domain and `Reclaimable` boundary unchanged.
- Move or replace the evidence-only correspondence wrapper as `verification/quint/PortableOperationExecution.qnt`; remove the copy under `models/quint` in a clean cutover.
- Replace duplicated child-correlation, write/transaction, and release-authorization state machines with qualified composition or projections of `PortableOperationExecutionCore.qnt`, `RecoveryProtocol.qnt`, and `LifecycleRelease.qnt` where those owners already define the required relation. Retain only evidence-local driver sequencing and cross-owner correspondence state that has no canonical owner.
- Parameterize finite fixture operation and child identity values as trusted model inputs. The composed evidence checks exact correlation but does not create topology, store, request, persistence, recovery, transaction, or lifecycle authority.
- Update every analysis, Connect, mutant, evidence, manifest, and maintained reference to the relocated composition while preserving or strengthening the existing driver, fan-in, bounded, mutation, and production-correspondence checks.
- Review the affected owner/dependent set and the active `define-portable-writable-session-lifecycle`, `add-portable-shutdown-claim-release`, and `add-scan-independent-writable-startup` changes without importing their proposed semantics into this repair.
- Preserve product and operator behavior, persisted formats, scheduling policy, physical-store behavior, and all existing verification claim ceilings.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `store-operation-contracts`: clarify the existing operation-slot requirement's sole delegated model owner, evidence-only composition boundary, trusted identity inputs, and non-authoritative wrapper location without changing required product behavior.

## Impact

- Planning target: `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations` retains its stable identity and behavior.
- Future model cutover: retain `models/quint/PortableOperationExecutionCore.qnt`; replace `models/quint/PortableOperationExecution.qnt` with `verification/quint/PortableOperationExecution.qnt` and update `PortableOperationExecutionAnalysis.qnt` and `PortableOperationExecutionFanInAnalysis.qnt` imports.
- Future verification/evidence review: retain the core analysis, six mutation canaries, deterministic six-child Connect paths, Rust Connect correspondence, verification manifest entry, and `docs/verification/retained-operation-execution-core.md`, updating paths and claims only where the ownership-preserving cutover requires it.
- No implementation, canonical spec, current model, verification asset, evidence, maintained documentation, Bead, or active-change artifact is changed by this planning proposal.
