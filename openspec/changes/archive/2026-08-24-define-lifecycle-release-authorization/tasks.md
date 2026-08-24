## 1. Lifecycle composition

- [x] 1.1 Identify the existing explicit-transaction-machine release observation, recovery reconciliation observations, operation-slot/store child and `Reclaimable` observations, and healthy-service basis-conformance predicate that jointly determine `ReleaseAllowed(operation-generation)`; preserve each owner's authority and do not add a basis-lifecycle owner.
- [x] 1.2 Expose the generation-qualified authorization from healthy-service composition only after terminal or authoritative media-effect state, terminal/reconciled children, required reconciliation, safe `Reclaimable` state, the applicable semantic transaction release boundary, required recovery-owned reconciliation facts, and basis-consumption conformance all hold.
- [x] 1.3 Keep the authorization monotonic and observation-based for one exact generation; repeated observations are harmless and later slot generations cannot consume an earlier authorization.
- [x] 1.4 Separate semantic authorization from physical cleanup: retain exact generation/evidence/resources for retry after reclamation failure without revoking an established authorization or permitting generation reuse.
- [x] 1.5 Preserve conservative withholding for unresolved operation/media-effect, child, reconciliation, recovery, or basis state and keep unresolved CLEAN-capture state outside this lifecycle decision.

## 2. Deterministic evidence

- [x] 2.1 Prove a successful write establishes a generation-qualified authorization after all owner boundaries, while repeated observation remains harmless.
- [x] 2.2 Prove known failed and short operations authorize only after their exact terminal/reconciliation path succeeds.
- [x] 2.3 Prove genuinely uncertain operation/media-effect state withholds authorization until authoritative reconciliation permits it.
- [x] 2.4 Prove stale generations cannot receive or reuse a prior generation's authorization.
- [x] 2.5 Prove an early `RangeReleased` observation and physical slot reclamation/removal alone cannot establish authorization.
- [x] 2.6 Prove terminalization or physical reclamation failure preserves conservative ownership/evidence and does not publish an authorization that its owner predicates have not established.
- [x] 2.7 Prove unresolved CLEAN-capture state is neither consulted nor changed by the lifecycle authorization.

## 3. Verification boundary

- [x] 3.1 Run focused lifecycle tests and conformance evidence for every authorization predicate and negative path.
- [x] 3.2 Review the resulting lifecycle observation against the current healthy-portable-io, transaction, recovery, store, and normalized-lifecycle requirements before any consumer integration.
- [x] 3.3 Leave `CodedRangeClean.qnt`, `RecoveryProtocol.qnt`, coded-clean semantics, Connect, and canonical synchronization unchanged until a later consumer review.

## 4. Lifecycle-release seam repair

- [x] 4.1 Scope `ReleaseAllowed` to operations with an owner-approved releasable claim or obligation; routine reads and flushes SHALL remain outside the relation.
- [x] 4.2 Preserve irrevocable authorization after establishment; remove normal invalidation, retirement, and cleanup-success revocation semantics.
- [x] 4.3 Reduce `LifecycleRelease.qnt` to the seven prerequisite observations, exact-generation identity, and bounded post-authorization cleanup outcomes without CLEAN, raw traces, reservation policy, or execution mechanics.
- [x] 4.4 Add an explicit typed production basis-conformance observation separate from recovery/topology/checksum-generation coherence.
- [x] 4.5 Preserve short/failed authorization certificates and expose authoritative reconciliation for retained unresolved generations through the service seam.
- [x] 4.6 Add exact-generation model-to-Rust projection rows, non-vacuous exhaustive profiles, sampled non-vacuity counts, and revised deterministic/mutant evidence.
- [x] 4.7 Complete fresh adversarial review before any coded-clean or Connect integration.
