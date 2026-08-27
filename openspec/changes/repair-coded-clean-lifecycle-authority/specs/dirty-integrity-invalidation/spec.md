## MODIFIED Requirements

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

The dirty-integrity boundary SHALL own the product meaning of a recovery `CLEAN` capture. The canonical coded-range owner SHALL supply, from one mapping relation over the admitted topology and coding profile, both each admitted mutation's complete coded claim and the complete future-inclusive coded scope covering every claim capable of invalidating selected dirty/checksum state. A capture SHALL bind stable capture identity, array/topology epoch, coding profile, that owner-approved scope, recovery generation, exact selected dirty-region and checksum-extent geometry, an owner-approved bounded lower-frontier summary, and a capture frontier. Dirty-integrity SHALL NOT reconstruct coded mappings, implement a second scope algorithm, broaden owner output, or require unbounded history.

Durable capture data SHALL retain the identities, geometry, coded units, membership and pending/resolved history, bounded summaries/frontiers, durable evidence references or receipts, and exact snapshot/revision needed for owner revalidation. A serialized assertion such as `scope_complete`, `scope_validated`, or `lower_frontier_covered` is data, not live authority. After reopen, the coded-range owner SHALL revalidate scope from durable topology/coding identity and selected geometry; the dirty-integrity capture-retention owner SHALL revalidate membership and lower-frontier/history coverage from underlying durable evidence; and recovery-state owners SHALL revalidate topology, generation, predecessor revision, and commit observations. Missing, stale, contradictory, or unavailable underlying facts SHALL withhold fresh authority and preserve conservative dirty/indeterminate state and capture exclusion.

For an active capture, an admitted mutation whose complete coded claim intersects the capture scope SHALL be classified: at or before the capture frontier it is `Included`; after the frontier it is `Later` and remains outside that `CLEAN`. Lifecycle owners SHALL supply generation-bound `OperationDispositionObserved` facts for included operations. After evaluating those facts with complete scope and selected dirty/checksum state, the CLEAN owner SHALL supply one capture-wide `Accepted` or `Rejected` decision. A generic handoff, requested action, attempted persistence, classification, or model transition is not that decision.

The dirty owner SHALL consume owner-approved coded membership, topology/generation, dirty/checksum coverage, write and data/parity outcomes, store watermarks and persistence evidence, lifecycle dispositions, the capture-wide CLEAN decision, and `Durable`, `Rejected`, or `Unknown` recovery dispositions. A later mutation SHALL affect media only when the captured `CLEAN` is durably committed before its dirty/recovery boundary or the mutation durably establishes a newer exact boundary/frontier/generation that stales/refuses the older `CLEAN`; an owner-approved already-DIRTY boundary is required for that optimization. If a known durable `CLEAN` is followed by an unknown later boundary, the later mutation remains blocked pending authoritative reconciliation.

An unresolved `CLEAN` capture SHALL NOT by itself pin an independently releasable operation or coded claim. Final coded-claim removal SHALL consume the canonical healthy-portable-io lifecycle owner's full exact-generation `ReleaseAllowed(operation-generation)` after all of its owner-approved predicates hold. Safe slot `Reclaimable`, `OperationReleasePermit`, a range-release observation, or another lower-level reclaimable-slot witness is only a contributing prerequisite and SHALL NOT independently authorize coded release. The coded-CLEAN consumer SHALL NOT construct `ReleaseAllowed`, invoke a raw issuer, or construct the trusted lifecycle observations from which the owner derives it.

Operation release, durable capture-membership compaction, and capture retirement SHALL be distinct transitions. Successful release SHALL remove only the exact generation's coded authority and durably retain every required release receipt before the delegated relation observes `Released`; it SHALL NOT erase capture membership or retire a capture.

The dirty-integrity capture-retention owner SHALL be the only owner that authorizes forgetting capture membership/history. Its opaque compaction preparation SHALL bind the exact capture, topology and coding identity, durable predecessor snapshot/revision, released operation generations and every required durable release receipt, complete pre-compaction membership and pending/resolved history obligations, every fact forgotten versus retained, the exact replacement bounded summary/frontier, and the proposed successor. The service MAY carry that preparation through one atomic generation-checked recovery transaction and confirm it from the exact durable receipt, but SHALL NOT invent a summary from recovery generation, coded-admission high-water mark, one released sequence, or other consumer arithmetic. Known rejection preserves the exact prior state; unknown acknowledgement follows recovery-state reopen semantics.

Capture retirement SHALL consume a phase-specific opaque capability issued by the dirty-integrity capture-lifecycle owner and bound to capture identity, exact live predecessor snapshot/revision, current topology, current recovery generation, exact selected dirty/checksum geometry, and exact proposed successor. A generic production authority that accepts a capture identity, inspects phase, and grants itself removal SHALL NOT exist. Identical geometry SHALL NOT make cleanup authority interchangeable between captures.

For a newly refused inherited capture, `RecoveryCleanRefusalPermit` remains CLEAN-policy evidence and SHALL NOT itself be capture-removal authority; the capture-lifecycle owner SHALL combine it with the exact capture and predecessor bindings to derive one atomic refusal-and-removal proposal. Persisted `Refused` phase is data and SHALL be revalidated under current recovery and dirty-integrity authority before cleanup. A definitive refusal ends future-exclusion duty and may retire despite inherited membership without reconstructing or synthesizing release authorization; dirty/indeterminate state remains.

An empty `CleanKnown` capture SHALL be retirable without requiring an unrelated later mutation or strictly newer dirty boundary. The capture-lifecycle owner may issue exact clean-cleanup authority only when the capture-wide accepted decision and `CLEAN` commit are durably known, the durable recovery successor independently retains the CLEAN/protection result and evidence needed for future claims, every included or later operation has a durable resolved disposition and required release receipt, owner-authorized compaction has removed all membership, no pending or unknown later-cut/CLEAN/history obligation remains, and the exact bounded retained-history summary covers every forgotten fact. The CLEAN transaction alone SHALL NOT authorize immediate deletion, but after those closure facts become durable no later write is required. Under continued healthy owner execution, every ordinary successful write SHALL make its capture retirable through release, compaction, and cleanup even when its region is never touched again; sequential disjoint writes SHALL NOT accumulate settled captures without bound.

After restart, current refusal evidence and exact capture-specific cleanup authority may propose one inherited capture's refusal and removal atomically. A known rejected or failed transaction leaves the exact prior capture authoritative. After a true may-have-committed unknown, lost, or corrupt acknowledgement, the service SHALL install neither outcome from process-local belief, release process-local authority, and reopen or inspect under recovery-state semantics. Reconciliation may accept only the exact durable prior capture or exact atomically refused-and-removed successor. Neither outcome may claim `CLEAN`, synthesize operation release, clear dirty state, or validate integrity.

The bounded capture membership, Included/Later classification, capture-wide decision consumption, durable-cut eligibility, successfully linearized full-lifecycle release, owner-authorized durable membership compaction, phase-specific capture cleanup eligibility, and their named external authorization relation are delegated to `models/quint/CodedRangeClean.qnt`. Within that bounded relation it is the sole exact state/transition authority. Model `Released` SHALL abstract only the complete successful production release linearization; model compaction and refused/clean cleanup authorizations SHALL abstract only their corresponding opaque external owner capabilities. Model scope-proof booleans are live external-authority projections, not persisted proof. The model SHALL NOT produce authorization, decide lifecycle predicates or CLEAN policy, reproduce geometry construction or recovery-store mechanics, or own adapter commit uncertainty. Recovery-state semantics retain known-failure and unknown-outcome reconciliation.

#### Scenario: One coded owner supplies claims and scope

- **WHEN** a write claim and a future-inclusive scope are required for one topology/coding profile
- **THEN** the canonical coded-range owner supplies both from one mapping relation, and dirty-integrity classifies their overlap without reconstructing mapping

#### Scenario: Persisted proof flags do not rehydrate authority

- **WHEN** a capture reopens with serialized completeness, validation, or lower-frontier flags
- **THEN** fresh authority is issued only after the named owners revalidate the underlying durable identities, geometry, evidence, membership, and summaries; failure preserves conservative capture state

#### Scenario: Scope is future-inclusive and membership is bounded

- **WHEN** a mutation whose complete coded claim is capable of invalidating selected dirty/checksum state is admitted before, during, or after capture
- **THEN** the owner-approved future-inclusive scope already covers that claim; while active, the mutation is `Included` at or before the frontier or `Later` afterward, and a definitively refused capture acquires no new obligations

#### Scenario: CLEAN requires one capture-wide owner decision

- **WHEN** included lifecycle dispositions and selected dirty/checksum evidence are evaluated
- **THEN** the CLEAN owner supplies one exact `Accepted` or `Rejected` decision; generic handoff, classification, or model completion cannot clear selected state

#### Scenario: Both durable-cut orderings protect later media effect

- **WHEN** a later overlapping mutation approaches media effect while a capture is unresolved, known durable, or made stale by a newer boundary
- **THEN** the captured `CLEAN` is durable before the later boundary, or the newer boundary durably stales/refuses it; an unknown boundary blocks mutation pending reconciliation

#### Scenario: CLEAN uncertainty remains conservative through reopen

- **WHEN** selected evidence or a CLEAN commit is stale, missing, rejected, unknown, abandoned, or lost across restart
- **THEN** the service preserves dirty/indeterminate state and bounded capture evidence and obtains authoritative reconciliation before reporting `CLEAN` or cleanup

#### Scenario: Operation release is independent of unresolved capture

- **WHEN** exact external `ReleaseAllowed` is supplied by the canonical lifecycle-release owner while the capture remains unresolved
- **THEN** the operation and coded claim may release, while capture identity/frontier evidence and future-overlap exclusion remain effective

#### Scenario: Full lifecycle release authority is required

- **WHEN** a coded operation reaches safe `Reclaimable` or presents a lower-level reclaim witness but one or more canonical lifecycle predicates remain unsatisfied
- **THEN** coded authority remains held because only exact-generation `ReleaseAllowed` authorizes final coded-claim removal

#### Scenario: Released membership compacts under exact owner authority

- **WHEN** exact generations are released and their receipts are durable while a resolved capture still names them
- **THEN** only owner-issued compaction preparation bound to the exact predecessor, all forgotten/retained facts, and replacement summary may remove that membership

#### Scenario: Compaction authority is stale or incomplete

- **WHEN** compaction preparation names another capture/topology/revision/generation, lacks a required receipt, omits a history obligation, or substitutes consumer arithmetic
- **THEN** compaction is rejected without changing durable or in-memory state

#### Scenario: Cleanup authority is capture-specific and phase-specific

- **WHEN** two captures have identical selected geometry or one capture changes predecessor revision
- **THEN** neither refused nor clean cleanup authority for one exact capture predecessor can remove the other capture or revised predecessor

#### Scenario: Last healthy write does not pin an empty CLEAN capture

- **WHEN** a last write to a never-again-touched region has durable `CLEAN`, full lifecycle release, owner-authorized empty membership, no pending/unknown obligations, and independently retained CLEAN/history evidence
- **THEN** the owner may issue exact `CleanKnown` cleanup authority without a later write or dirty boundary, so the settled capture can retire

#### Scenario: Reopened capture is definitively refused

- **WHEN** exact current refusal evidence is combined with capture-specific predecessor-bound cleanup authority and the atomic refusal-and-removal transaction commits durably
- **THEN** the capture is removed, conservative dirty/indeterminate state remains, no inherited operation release is synthesized, and reopen does not reconstruct it

#### Scenario: Reopened refusal is known not to commit

- **WHEN** the refusal-and-removal transaction is authoritatively rejected or fails before commit
- **THEN** the exact prior inherited capture remains authoritative and process-local state does not install the proposal

#### Scenario: Reopened refusal outcome may have committed

- **WHEN** acknowledgement of the atomic refusal-and-removal transaction is lost, corrupt, or otherwise genuinely uncertain
- **THEN** the service installs neither outcome from process-local belief and reopen reconciles only the exact durable prior or exact atomically refused-and-removed successor
