# dirty-integrity-invalidation Specification

## Purpose
This capability provides the portable dirty-region protocol that invalidates affected integrity evidence before data/parity writes and clears recovery state only with matching persistence evidence.
## Requirements
### Requirement: Dirty-region coverage is complete and checked
<!-- dwv:req req.dirty-integrity-invalidation.dirty-region-coverage-is-complete-and-checked -->

One canonical semantic mapping SHALL derive every dirty region intersected by a member byte range from the recovery-owned region geometry. It SHALL return each region exactly once, preserve the member identity without collisions within declared supported bounds, and reject empty ranges, range arithmetic overflow, region-ID representation overflow, or unsupported geometry before the write-recovery record or data/parity write. Service, transaction, persistence-evidence, recovery-`CLEAN`, simulator, and recovery paths SHALL use this mapping rather than a separate hard-coded region size.

#### Scenario: A write crosses a region boundary

- **WHEN** a valid protected write intersects two or more dirty regions
- **THEN** every intersected region is included once in the write-recovery record, transaction ranges, persistence-evidence coverage, and any later matching recovery-`CLEAN` decision

#### Scenario: Region coverage cannot be represented

- **WHEN** the member/range/geometry calculation overflows or two supported inputs would map to the same region identity
- **THEN** the request is rejected before recovery or a data/parity write

### Requirement: Write-recovery record precedes data/parity write
<!-- dwv:req req.dirty-integrity-invalidation.write-recovery-record-precedes-data-parity-write -->

The recovery protocol SHALL durably record every affected region as `DIRTY` and every affected `VALID` checksum extent as `STALE` before the first data/parity write. A rejected, lost, or uncertain write-recovery record commit outcome SHALL prevent the data/parity write.

#### Scenario: Write targets regions currently marked CLEAN

- **WHEN** a transaction targets a clean region and a valid checksum extent
- **THEN** the protocol commits the write-recovery record before the first data/parity write; reads needed by the operation remain governed by the existing transaction and store requirements

#### Scenario: The write-recovery record fails to commit or its outcome is unknown

- **WHEN** the recovery adapter rejects, loses, or cannot classify the write-recovery-record commit
- **THEN** no data/parity write is permitted and the transaction returns a conservative result

### Requirement: Already-dirty writes preserve the invalidation boundary
<!-- dwv:req req.dirty-integrity-invalidation.already-dirty-writes-preserve-the-invalidation-boundary -->

The protocol MAY avoid a redundant write-recovery-record commit only when every affected region is already durably dirty and every affected valid checksum extent is already durably stale under the current topology and generation. Crossing any clean or valid boundary SHALL require a new durable write-recovery record.

#### Scenario: Write remains within an already dirty/stale set

- **WHEN** a second transaction targets only regions and checksum extents already covered by durable dirty/stale state
- **THEN** it may proceed without another identical write-recovery-record commit while retaining the captured generations

#### Scenario: Write crosses a clean or valid boundary

- **WHEN** a transaction expands into any region not covered by dirty state or any checksum extent still `VALID`
- **THEN** it commits a new write-recovery record before the data/parity write

### Requirement: Recovery CLEAN requires persistence evidence
<!-- dwv:req req.dirty-integrity-invalidation.recovery-clean-requires-persistence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:refines req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->

The dirty protocol SHALL select only regions covered by matching recovery authority and owner-admissible persistence evidence for every required store watermark and captured topology, region, checksum, capability, store-incarnation, and recovery generation. Future, stale, partial, omitted-region, or cross-store evidence SHALL fail closed. The dirty protocol owns the exact region subset and generation checks for a recovery-`CLEAN` transition; the recovery-state capability owns which persistence evidence may authorize a `CLEAN` claim.

#### Scenario: All selected regions have covering authority

- **WHEN** every selected region is covered by matching current typed recovery authority and persistence evidence
- **THEN** the protocol may request one durable recovery transaction that clears exactly those regions

#### Scenario: Evidence omits a selected region or generation

- **WHEN** any selected region lacks covering authority or a captured generation changed
- **THEN** recovery-`CLEAN` clearing is refused and dirty state or a state that requires reconciliation remains

### Requirement: Failures and restart are conservative
<!-- dwv:req req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->

Any short, failed, uncertain, abandoned, crashed, or post-write-recovery-record recovery result SHALL preserve dirty or indeterminate evidence for every affected region. This requirement owns the durable dirty/restart consequence after such a result; frontend abandonment meaning, operation-resource lifetime, and transaction-state transitions remain owned by their respective capabilities. Restart SHALL discover durable dirty evidence and SHALL NOT infer a clean state from elapsed time, process success, or a missing action result.

#### Scenario: A data/parity write fails after the write-recovery record is durable

- **WHEN** a data/parity operation is short, failed, or uncertain after the write-recovery record is durable
- **THEN** the affected state remains dirty or indeterminate and no clean completion is reported

#### Scenario: Process exits before recovery state is committed CLEAN

- **WHEN** process state is lost after a data/parity write but before persistence evidence and the recovery-`CLEAN` transition
- **THEN** restart enters recovery or blocked handling with dirty evidence rather than assuming the write was clean

### Requirement: Dirty and integrity/session dimensions remain independent
<!-- dwv:req req.dirty-integrity-invalidation.dirty-and-integrity-session-dimensions-remain-independent -->

The protocol SHALL represent dirty-region state, checksum validity/coverage, parity cleanliness, and session-dirty state as separate dimensions. A clean parity state SHALL NOT establish current checksum coverage, and checksum validity SHALL NOT clear dirty state without the required transaction proof.

#### Scenario: Parity is clean but checksums are stale

- **WHEN** parity state is clean while some checksum extents are stale or absent
- **THEN** the system reports clean parity with incomplete integrity coverage and does not claim all checksums valid

#### Scenario: Session close cannot be proven

- **WHEN** a session persistence boundary or recovery-`CLEAN` transition is missing at shutdown
- **THEN** the session remains dirty even if individual regions were previously cleared

### Requirement: Transitions and evidence are deterministic
<!-- dwv:req req.dirty-integrity-invalidation.transitions-and-evidence-are-deterministic -->

The protocol SHALL emit stable semantic transitions containing transaction identity, topology epoch, affected ranges/extents, captured/current generations, and missing evidence. Replaying the same plan and results SHALL produce the same final state and error class.

#### Scenario: Same schedule is replayed

- **WHEN** the same write-recovery-record, data/parity, persistence-evidence, recovery-`CLEAN`, and failure results are applied twice
- **THEN** the normalized transition trace and recovery state are identical

#### Scenario: Invalid ordering is attempted

- **WHEN** a caller requests a data/parity write before the write-recovery record or recovery-`CLEAN` clearing before covering persistence evidence
- **THEN** the transition is rejected without changing the prior semantic state


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
