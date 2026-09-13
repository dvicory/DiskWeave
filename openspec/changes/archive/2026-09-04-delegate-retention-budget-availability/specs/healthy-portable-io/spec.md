## MODIFIED Requirements

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
