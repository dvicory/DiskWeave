# healthy-portable-io Specification

## Purpose
This capability provides the first end-to-end portable single-parity path: normalized requests are admitted through stable topology and operation slots, executed against ordinary file-backed members, and completed only with the evidence required by the canonical dirty-region, recovery, transaction, XOR, and store contracts.

## Requirements

### Requirement: Assembly and request admission are bounded and identity-safe
<!-- dwv:req req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.checksum-plane.current-baseline-completion-is-persisted-and-exact -->

The service SHALL assemble exactly one compatible member binding for every required assignment in a validated captured topology and admit each canonical request through a generational operation slot. It SHALL consume, and SHALL NOT reinterpret, the exact snapshot/member-binding valid/invalid result delegated by `req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments`. Local collection, discovery, or vector order SHALL NOT define slot, role, coding position, assignment, or target identity. Before child I/O, the service SHALL resolve the request's stable target slot through its captured topology and fail when the delegated relation rejects the snapshot or bindings. A request whose captured topology epoch is stale SHALL fail before resource reservation or child I/O. Each opened store SHALL continue to be selected by the topology assignment's stable store identity. That authorization remains a non-delegated current obligation because the current assignment type supplies no expected store identity; it lies outside the delegated relation and SHALL NOT be weakened or filled. Service-owned supported-profile, bounded-resource, capability, recovery, and request-admission checks SHALL remain independently required.

When current recovery state carries a mandatory new-checksum-baseline obligation, read/write assembly and request admission SHALL remain blocked until the checksum owner reconstructs complete current persisted coverage. Request validation, admission ownership, child execution, terminal reconciliation, and returned operation evidence SHALL preserve the same canonical request fields without a parallel service request model. A delegated validation rejection or any independent service prerequisite failure SHALL occur before resource reservation, child I/O, or member mutation as required by the owning boundary.

#### Scenario: Healthy topology is assembled

- **WHEN** the delegated topology/member-binding relation accepts, all required data/parity roles have unambiguous owner-qualified identity and compatible service capabilities, and no current admission prerequisite is outstanding
- **THEN** the service enters serving state and accepts normalized requests with a captured topology epoch

#### Scenario: Mandatory post-recovery baseline is incomplete

- **WHEN** current recovery state requires a new checksum baseline and current persisted coverage is absent, partial, invalid, or unsupported
- **THEN** read/write assembly and request admission fail before member mutation

#### Scenario: Mandatory post-recovery baseline is complete

- **WHEN** current recovery state requires a new checksum baseline and the checksum owner reconstructs complete correctly bound current persisted coverage
- **THEN** that baseline prerequisite is satisfied while every other current assembly and admission prerequisite still applies

#### Scenario: Unrelated healthy array has no mandatory baseline

- **WHEN** an otherwise eligible array carries no current new-baseline obligation
- **THEN** the new baseline barrier does not narrow its existing assembly or request admission behavior

#### Scenario: Ambiguous or stale assembly is attempted

- **WHEN** owner-qualified identity evidence is ambiguous, a required role is unavailable, a captured generation is stale, or the delegated topology/member-binding relation otherwise rejects
- **THEN** assembly or request admission fails closed without mutating a member

#### Scenario: Collection order differs from topology order

- **WHEN** member collection order and topology assignment order differ while every supplied semantic value and owner-qualified identity comparison is unchanged
- **THEN** the delegated result is unchanged and the same request slot resolves to the same assignment and member

#### Scenario: A positional binding disagrees with topology

- **WHEN** collection position would select a different member than the topology assignment's stable store identity
- **THEN** assembly or request admission fails before resource reservation, child I/O, or member mutation

#### Scenario: A canonical request completes through the service

- **WHEN** a validated request is admitted, executed, and reconciled
- **THEN** operation evidence identifies the same frontend, request, target slot, topology epoch, operation, range, buffer token, ordering, and durability fields

### Requirement: Publication identity is derived from admitted semantics
<!-- dwv:req req.healthy-portable-io.publication-identity-is-derived-from-admitted-semantics -->
<!-- dwv:requires req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations -->

A service that passes portable admission SHALL expose a deterministic publication identity derived from its exact admitted recovery topology and the complete current identity-observation sets of every bound member in stable semantic assignment order. Each member identity assessment used by publication SHALL be the assessment of that member's complete current observation set, not a synthetic assessment of one selected observation. The identity SHALL change when any array identity, topology epoch, geometry, profile, slot, role, coding position, assignment instance or generation, store identity, assignment evidence summary, member observation source, member observation fingerprint, or member identity assessment changes. It SHALL NOT depend on fixture paths, collection order, SQL layout, runtime handles, or frontend-local rediscovery.

#### Scenario: The same admitted object is derived twice

- **WHEN** topology and all bound member identity observations are semantically identical
- **THEN** the service derives the same publication identity regardless of collection order

#### Scenario: A member observation changes

- **WHEN** one bound member retains its policy label and store identifier but its observed identity fingerprint changes
- **THEN** the publication identity changes and a frontend cannot report the old admitted object as the new one

#### Scenario: One observation in a multi-observation member conflicts

- **WHEN** a bound member has multiple current observations and any observation makes the complete assessment ambiguous, conflicting, or changed
- **THEN** publication identity includes that conservative complete assessment and cannot claim the member merely matches

### Requirement: Healthy reads preserve exact-range evidence
<!-- dwv:req req.healthy-portable-io.healthy-reads-preserve-exact-range-evidence -->

A read SHALL validate the normalized byte range, split it at required boundaries, read the selected data member through the store contract, and return exact completed-range/disposition/persistence evidence. It SHALL not fabricate bytes for short or uncertain reads.

#### Scenario: Aligned healthy read

- **WHEN** a request reads an available data range and the store returns complete bytes
- **THEN** the service returns the exact requested bytes and structured successful completion

#### Scenario: Short or uncertain read

- **WHEN** a child read completes short or with unknown media effect
- **THEN** the service returns structured partial/uncertain evidence and does not report a full successful read

### Requirement: Writes follow the reference transaction and update single XOR parity
<!-- dwv:req req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity -->
<!-- dwv:requires req.dirty-integrity-invalidation.write-recovery-record-precedes-data-parity-write -->
<!-- dwv:requires req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic -->
<!-- dwv:requires req.recovery-state-semantics.data-parity-write-requires-write-recovery-record -->
<!-- dwv:requires req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts -->
<!-- dwv:requires req.xor-reference-model.incremental-updates-and-full-recomputation-are-equivalent -->
<!-- dwv:requires req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence -->

This requirement owns service orchestration and conformance only. A protected write SHALL compose one complete validated semantic mutation unit (which MAY be a request-decomposition unit) with the canonical dirty-region mapping, transaction-owned coded-range authority for every mutation and relevant basis read, the durable write-recovery-record owner, atomic recovery transaction, delegated reference-transaction relation, single-XOR computation, and exact-range store operations. The service SHALL present complete owner-approved admission before dependent basis I/O or protected mutation, preserve each owner's result, and keep relevant basis observations coherent through consumption or discard/reconciliation before release. No protected member data/parity write SHALL occur until both coded/resource admission and write-recovery-record admission succeed. It SHALL NOT restate or reinterpret coded, transaction, dirty, recovery, XOR, store, or release predicates.

#### Scenario: Partial write requires read-modify-write

- **WHEN** a write covers part of a parity extent
- **THEN** the service composes required basis reads, reference-equivalent XOR, and exact writes only after coded/resource and write-recovery owners admit the unit, preserving basis coherence through consumption or discard/reconciliation

#### Scenario: Full overwrite is aligned

- **WHEN** a write fully covers the required data and parity extent
- **THEN** the service may avoid old-data reads only under the XOR contract while preserving every owner-approved coded, transaction, dirty, recovery, persistence, and store boundary

#### Scenario: Basis coherence is required through consumption

- **WHEN** a relevant basis observation remains needed after authority would otherwise release
- **THEN** the service holds it coherently through the consuming mutation or discards/reconciles it before release and never uses a release-then-use gap

### Requirement: Durable completion and recovery CLEAN require persistence evidence
<!-- dwv:req req.healthy-portable-io.durable-completion-and-recovery-clean-require-persistence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->
<!-- dwv:requires req.dirty-integrity-invalidation.recovery-clean-requires-persistence-evidence -->
<!-- dwv:requires req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set -->
<!-- dwv:requires req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts -->

This requirement owns service composition and conformance only. The service SHALL preserve normalized durability intent and compose owner-approved dirty/recovery observations, exact lifecycle dispositions, store watermarks and persistence evidence, dirty/checksum coverage, recovery generation/topology and reopen reconciliation, one capture-wide CLEAN-owner `Accepted`/`Rejected` decision, and the closed mutation set. It may report durable completion or commit recovery state `CLEAN` only when the owning requirements accept current-generation evidence and permit the exact selected clear. Unsupported durability requirements SHALL be rejected or reported at the explicitly established weaker scope. CLEAN-capture uncertainty remains distinct from exact external `ReleaseAllowed` supplied by the canonical lifecycle-release requirement; it neither supplies nor negates that authorization. The service SHALL NOT restate or weaken owner predicates.

#### Scenario: All required owners accept completion evidence

- **WHEN** writes are complete, lifecycle owners provide dispositions, the CLEAN owner accepts the closed set, the durable post-capture cut is satisfied, and store/recovery/dirty owners accept matching evidence
- **THEN** the service may report durable completion and commit `CLEAN` only for the proven selected regions

#### Scenario: An owner rejects completion evidence

- **WHEN** any owner rejects missing, volatile, stale, partial, future, or mismatched evidence, or the CLEAN owner does not accept the capture
- **THEN** the service reports the conservative result and leaves affected state dirty, stale, or indeterminate

#### Scenario: CLEAN disposition is Unknown

- **WHEN** recovery or an adapter reports an unclassifiable `Unknown` commit or reopen disposition
- **THEN** the service preserves dirty/indeterminate state and bounded capture evidence, obtains authoritative reconciliation before `CLEAN` or cleanup, and independently consumes exact external `ReleaseAllowed` when the canonical lifecycle-release owner authorizes operation or coded-claim release

### Requirement: Abandonment, restart, and failure preserve operation safety
<!-- dwv:req req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->

The service SHALL compose frontend delivery interest, operation-slot lifetime, transaction outcome, and durable dirty/restart consequences without redefining them. It SHALL keep admitted work and resources under their owners until required reconciliation and release, preserve every conservative failure result, and never blindly retry an uncertain non-idempotent write.

#### Scenario: Request is abandoned after the write-recovery record is durable

- **WHEN** the frontend abandons delivery interest after a durable write-recovery record but before operation completion
- **THEN** the service continues owned drain and reconciliation, preserves dirty evidence as required, and releases resources only after the operation-lifetime owner permits it

#### Scenario: Service restarts with dirty state

- **WHEN** the process restarts after an incomplete data/parity write or persistence-evidence step
- **THEN** the service follows the durable recovery disposition and never infers a clean state from a missing completion

### Requirement: Portable members remain ordinary and control state is disposable
<!-- dwv:req req.healthy-portable-io.portable-members-remain-ordinary-and-control-state-is-disposable -->

Data and parity member files SHALL contain no required DiskWeave metadata. Loss of disposable control state SHALL not make intact payloads unreadable or authorize unsafe writes; recovery state and topology evidence remain the authority for data/parity writes.

#### Scenario: Control database is removed

- **WHEN** the management/control database is deleted while data and recovery stores remain
- **THEN** management state can be rebuilt without changing payload bytes or silently authorizing a data/parity write

#### Scenario: Backing and export paths alias

- **WHEN** a proposed backing path is also an active exported endpoint
- **THEN** the service rejects the configuration before opening competing access

### Requirement: Generation-qualified release authorization composes owner-approved lifecycle facts
<!-- dwv:req req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->

For one exact generation-bearing admitted operation whose owner-approved admission includes a releasable claim or obligation that another component must be permitted to drop, the healthy service SHALL compose the applicable owner-approved facts into a monotonic semantic authorization `ReleaseAllowed(operation-generation)`. `ReleaseAllowed` is not a generic operation-completion result: routine reads, flushes, and other operations without a releasable claim SHALL NOT establish it merely because their slots terminate. Applicability is an admission/owner scope condition, not a derived eighth lifecycle observation. The authorization MAY be observed repeatedly without changing its meaning and SHALL never apply to a later operation that reuses the slot with a different generation.

`ReleaseAllowed(operation-generation)` SHALL become true only when all of the following seven owner-approved observations hold for that exact generation:

- the operation and its media effect are terminal or authoritatively reconciled under the existing store-operation and recovery semantics;
- every child operation is terminal;
- required child reconciliation is recorded;
- the operation-slot owner has reached its safe generation-qualified `Reclaimable` state;
- the applicable owner-approved semantic transaction release requirement is satisfied, including delegated `RecoveryProtocol.releaseRange` where applicable; an operation outside the authorization scope does not enter this relation;
- any additional recovery-owned reconciliation fact required for that release is authoritative;
- healthy-service composition establishes that no relevant basis observation remains consumable because it was consumed, discarded, or authoritatively reconciled.

`store-operation-contracts` SHALL remain authoritative for child outcomes, operation-slot lifetime, required reconciliation, safe `Reclaimable`, resource accounting, and generation reuse. `explicit-transaction-machine` SHALL remain authoritative for the semantic transaction/release relation. `recovery-state-semantics` SHALL remain authoritative for its reconciliation observations. Healthy-portable-io SHALL own the explicit basis-conformance observation and only the final composition of these seven facts into `ReleaseAllowed`; it SHALL NOT redefine any contributing predicate, infer applicability from termination, or introduce a separate basis-lifecycle owner.

For this requirement's bounded composition surface, the exact state, action, and invariant relation SHALL be delegated to the canonical `models/quint/LifecycleRelease.qnt` module. Its external observations are exactly: terminal or authoritative-reconciled operation/media effect; terminal children; recorded required reconciliation; safe generation-qualified `Reclaimable`; the applicable owner-approved semantic transaction release requirement being satisfied; authoritative recovery-owned reconciliation; and healthy-service basis conformance showing no relevant basis remains consumable. Applicability is external admission scope, not a model observation. The model SHALL distinguish those observations, pure derived claims, `ReleaseAllowed`, cleanup requests, and observed cleanup results. It SHALL represent transaction satisfaction as the owner-approved requirement being satisfied, not raw `RangeReleased` and not a Rust `transaction_required` boolean. It SHALL NOT model CLEAN capture, raw transaction traces, reservation policy, execution mechanics, or generic operation completion. Once established, `ReleaseAllowed` has no normal invalidation or retirement transition; cleanup results cannot revoke it. Separate analysis and mutant modules under `verification/quint/` SHALL provide finite evidence only.

Established authorizations are retained per slot index under an explicit per-slot budget. Admission SHALL consult retention availability before coded admission and refuse new requests that cannot be retained, leaving existing records and owners untouched. Already accepted writes SHALL be preserved through reconciliation waits rather than failed when retention is the only blocker. The exact availability transitions behind this boundary, including discharge-driven sweep, occupied replacement, exhaustion, and exact retirement, are delegated below and SHALL NOT be restated here.

For this requirement's bounded retention-availability subrelation, the exact state, action, and invariant relation SHALL be delegated to the canonical `models/quint/RetentionBudgetAvailability.qnt` module. Its external inputs are exactly: established authorizations as opaque exact-generation records; outstanding-consumer flags as opaque owner observations from admission, coded-capture, and driver owners; and per-slot budgets as parameters. Slot-table occupancy, authorization composition, coded-capture consumer mechanics, budget values, transaction and recovery semantics, WriteDriver continuation, physical stores, and governor capacity policy remain outside the delegated model. The model SHALL distinguish retained entries, consumer flags, admission, refusal, retention, and exhaustion verdicts with exact verdict identity, and exact retirement. It SHALL NOT model authorization composition, slot lifecycle, coded capture internals, u32 range or index bounds, or physical durability. Separate analysis, phase-cut, composition, scenario, and mutant modules under `verification/quint/` SHALL provide finite evidence only.

Reaching the operation-slot owner's safe `Reclaimable` state is a prerequisite for `ReleaseAllowed`, but successful subsequent physical slot or resource reclamation is not. If physical reclamation fails after `ReleaseAllowed` is established, the service SHALL retain the exact generation, canonical request, terminal evidence, and owned resources for retry under the operation-slot owner. The bookkeeping failure SHALL NOT revoke the semantic authorization, permit generation confusion, or authorize reuse of the retained generation's resources. Later cleanup success, topology or recovery-generation changes, unrelated work, and other later observations SHALL NOT make an established authorization false; they may end physical resource retention after the consumer no longer needs it.

Neither a transaction trace `RangeReleased` observation alone nor physical slot reclamation or removal alone SHALL establish `ReleaseAllowed`. Genuine unresolved operation, media-effect, child, reconciliation, recovery, or basis state SHALL withhold the authorization. An unresolved recovery `CLEAN` capture is a separate dirty-integrity concern and SHALL NOT be consulted by this lifecycle authorization; the delegated model represents that independence by containing no CLEAN-capture state.

#### Scenario: All owner predicates permit semantic release for one generation

- **WHEN** operation/media effect is terminal or authoritatively reconciled, every child is terminal, required reconciliation is recorded, the operation-slot owner has reached its safe generation-qualified `Reclaimable` state, the applicable semantic transaction release boundary is complete, required recovery-owned reconciliation facts are authoritative, and healthy-service composition establishes that no relevant basis observation remains consumable
- **THEN** the service may expose `ReleaseAllowed` for that exact operation generation; repeated observations for the same generation are harmless

#### Scenario: Routine termination does not establish release authorization

- **WHEN** a read, flush, or other admitted operation has no releasable claim or obligation and reaches terminal, reconciled, and reclaimable state
- **THEN** the service does not expose `ReleaseAllowed` merely from those completion facts

#### Scenario: Authorization is not revoked by later observations

- **WHEN** `ReleaseAllowed` has been established for an exact generation and later cleanup succeeds or fails, topology or recovery generation changes, unrelated work completes, or another later observation changes
- **THEN** the established authorization remains semantically true for that exact generation; no normal lifecycle action revokes it, and a later generation cannot consume it

#### Scenario: Partial release evidence does not establish authorization

- **WHEN** `RangeReleased` is observed before operation-slot terminalization, required child reconciliation, or healthy-service basis conformance, or physical slot removal is observed without every other required owner fact
- **THEN** the service does not expose `ReleaseAllowed` solely from that observation

#### Scenario: Physical reclamation fails after semantic release authorization

- **WHEN** the operation has reached safe `Reclaimable` state and all other owner predicates establish `ReleaseAllowed`, but subsequent slot or resource reclamation fails
- **THEN** the service retains the exact generation, evidence, and resources for retry, leaves generation reuse blocked, and preserves the already-established semantic authorization without treating the bookkeeping failure as a media-effect or transaction failure

#### Scenario: Unresolved operation or basis state withholds authorization

- **WHEN** operation/media effect, a child, required reconciliation, a required recovery observation, the semantic transaction release boundary, or a relevant basis observation remains unresolved
- **THEN** the service withholds `ReleaseAllowed` and preserves the owner-specific evidence and reconciliation obligation

#### Scenario: A later generation cannot consume an earlier authorization

- **WHEN** a slot is eventually reused with a later generation after the prior operation has been physically reclaimed
- **THEN** an authorization observed for the prior generation has no effect on the later operation

#### Scenario: CLEAN-capture uncertainty is independent

- **WHEN** an otherwise eligible operation has an unresolved dirty-integrity `CLEAN` capture
- **THEN** this lifecycle authorization does not consult, clear, or derive authority from the capture; dirty-integrity retains its independent capture evidence and exclusion obligations

#### Scenario: Saturated consumed budget refuses before admission

- **WHEN** every admissible index holds a full per-slot budget of authorizations with outstanding exact-generation consumers
- **THEN** the new request is refused before coded admission with existing records and owners untouched

#### Scenario: Consumer discharge unblocks the remember path

- **WHEN** a consumer owner discharges its outstanding claim and a new established authorization is remembered for the same index
- **THEN** the service feeds the discharged state into the remember composition, which re-evaluates retention availability for the new authorization; the exact retained set follows the delegated availability relation below

#### Scenario: Exhaustion errors while establishment blocks wait

- **WHEN** either the index budget cannot admit a new authorization at remember time or, separately, retention is blocked at authorization establishment for an accepted write
- **THEN** in the first case the remember path reports a reconciliation-required error with the evidence retained on the driver for retry and the refused entry not retained, while in the second case the accepted driver waits for owner reconciliation with its evidence preserved instead of failing
