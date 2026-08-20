## ADDED Requirements

### Requirement: Recovery CLEAN captures a closed mutation set
<!-- dwv:req req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set -->
<!-- dwv:requires req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts -->
<!-- dwv:requires req.dirty-integrity-invalidation.dirty-region-coverage-is-complete-and-checked -->
<!-- dwv:requires req.dirty-integrity-invalidation.recovery-clean-requires-persistence-evidence -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->
<!-- dwv:requires req.dirty-integrity-invalidation.transitions-and-evidence-are-deterministic -->
<!-- dwv:requires req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->
<!-- dwv:requires req.recovery-state-semantics.topology-snapshots-are-immutable-within-a-transaction -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts -->

The dirty-integrity boundary SHALL own the product meaning of a recovery `CLEAN` capture. A capture SHALL bind a stable identity to the array/topology epoch, coding profile, an externally validated complete coded scope covering every claim capable of invalidating the selected dirty/checksum state regardless of whether a mutation carrying that claim is admitted before, during, or after capture, recovery generation, selected dirty-region and checksum-extents geometry, an owner-approved exact bounded lower-frontier summary, and a capture frontier. The coded owner supplies scope and conflict classification; dirty-integrity SHALL NOT reconstruct coded mappings, broaden the scope, or require an unbounded history.

For an active capture, an admitted mutation whose complete coded claim intersects the capture scope SHALL be classified: at or before the capture frontier it is `Included`; after the frontier it is `Later` and remains outside that `CLEAN`. Lifecycle owners SHALL supply generation-bound `OperationDispositionObserved` facts for included operations. After evaluating those facts with complete scope and selected dirty/checksum state, the CLEAN owner SHALL supply one capture-wide `Accepted` or `Rejected` decision. A generic handoff, requested action, attempted persistence, classification, or model transition is not that decision.

The dirty owner SHALL consume owner-approved coded membership, topology/generation, dirty/checksum coverage, write and data/parity outcomes, store watermarks and persistence evidence, lifecycle dispositions, the capture-wide CLEAN decision, and `Durable`, `Rejected`, or `Unknown` recovery dispositions. A later mutation SHALL affect media only when the captured `CLEAN` is durably committed before its dirty/recovery boundary or the mutation durably establishes a newer exact boundary/frontier/generation that stales/refuses the older `CLEAN`; an owner-approved already-DIRTY boundary is required for that optimization. If a known durable `CLEAN` is followed by an unknown later boundary, the later mutation remains blocked pending authoritative reconciliation.

An unresolved `CLEAN` capture SHALL NOT by itself pin an independently releasable operation or coded claim. Exact external `ReleaseAllowed(operation-generation)` comes from the canonical healthy-portable-io lifecycle-release owner and is consumed here without reconstructing its lifecycle predicates. Release SHALL preserve bounded capture identity, frontier summary, scope, geometry, unresolved commit evidence, and future exclusion. Stale, refused, abandoned, failed, uncertain, crashed, or reopened captures retain conservative dirty/indeterminate consequences; `Unknown` permits neither `CLEAN` nor capture cleanup without authoritative reconciliation.

The bounded capture membership, Included/Later classification, capture-wide decision consumption, durable-cut eligibility, CLEAN/cleanup eligibility, and their supplied-uncertainty relation are delegated to `models/quint/CodedRangeClean.qnt`. Within that bounded relation it is the sole exact state/transition authority; it SHALL NOT decide lifecycle disposition, CLEAN-owner policy, persistence admissibility, geometry construction, resources, sessions, topology mapping, coded authority removal, or product claims. CLEAN cleanup remains distinct from coded-claim removal, and every delegated distinction requires the projection-feasibility seam or named external owner/evidence.

#### Scenario: Scope is future-inclusive and membership is bounded

- **WHEN** a mutation whose complete coded claim is capable of invalidating the selected dirty/checksum state is admitted before, during, or after capture
- **THEN** the capture's complete future-inclusive coded scope already covers that claim; while the capture remains active, the admitted mutation is `Included` at or before the capture frontier or `Later` afterward, and a definitively refused capture acquires no new obligations

#### Scenario: CLEAN requires one capture-wide owner decision

- **WHEN** included lifecycle dispositions and selected dirty/checksum evidence are evaluated for a capture
- **THEN** the CLEAN owner supplies one exact `Accepted` or `Rejected` decision; generic handoff, classification, or model completion cannot clear the selected state

#### Scenario: Both durable-cut orderings protect later media effect

- **WHEN** a later overlapping mutation approaches media effect while a capture is unresolved, known durable, or made stale by a newer boundary
- **THEN** the captured `CLEAN` is durable before the later boundary, or the newer boundary durably stales/refuses the older `CLEAN`; an unknown boundary blocks the mutation pending reconciliation

#### Scenario: CLEAN uncertainty remains conservative through reopen

- **WHEN** selected evidence or a CLEAN commit is stale, missing, rejected, unknown, abandoned, or lost across restart
- **THEN** the service preserves dirty/indeterminate state and bounded capture evidence and obtains authoritative reconciliation before reporting `CLEAN` or cleanup

#### Scenario: Operation release is independent of unresolved capture

- **WHEN** exact external `ReleaseAllowed` is supplied by the canonical lifecycle-release owner while the capture remains unresolved
- **THEN** the operation and coded claim may release, while capture identity/frontier evidence and future-overlap exclusion remain effective
