## ADDED Requirements

### Requirement: Durable writable-session lifecycle binds authority and close evidence
<!-- dwv:req req.recovery-state-semantics.durable-writable-session-lifecycle-binds-authority-and-close-evidence -->
<!-- dwv:refines req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->
<!-- dwv:requires req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->
<!-- dwv:requires req.recovery-state-semantics.writable-recovery-ownership-is-crash-releasing -->
<!-- dwv:requires req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:requires req.store-operation-contracts.capability-evidence-determines-the-allowed-safety-profile -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->
<!-- dwv:requires req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit -->
<!-- dwv:requires req.dirty-integrity-invalidation.dirty-and-integrity-session-dimensions-remain-independent -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->
<!-- External target prerequisite: req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set from define-coded-range-clean-coordination; restore the normal owner edge after integration and before implementation, validation, or canonical sync. -->

The recovery-state owner SHALL define one durable writable-session lifecycle for each admitted writable session. A successful begin SHALL capture a fresh session identity; the accepted array and topology identity; the topology epoch; the complete required assignment set with each assignment's stable slot, role, coding position, assignment instance and generation, store identity and incarnation, and protected geometry; the recovery generation; the session or dirty-envelope generation; each bound store's ordering-domain identity; the accepted capability/profile binding; and any optional baseline watermark that is already current and owner-approved. Begin SHALL NOT claim future write watermarks or fence certificates that do not yet exist. It SHALL consume current owner-approved identity, topology, recovery-health, capability, and store-claim evidence without acquiring or releasing those claims. The begin transition SHALL be durably observed before writable endpoint publication, writable request admission, or protected payload mutation.

A begin precondition rejection, including a generation, topology, identity, capability, binding, or claim mismatch, SHALL create no session transition and SHALL leave the exact prior recovery snapshot authoritative. A begin commit observed as durable SHALL make the proposed session authority authoritative. A lost, corrupt, or otherwise unclassifiable begin observation SHALL leave prior-versus-proposed authority unresolved, publish no writable service, and require reconciliation rather than inferring that the session is open or closed.

A clean close SHALL be a durable recovery fact for the same session identity and captured authority. It SHALL require owner-approved evidence that every operation generation still owned when the close frontier is established has reached the operation-slot owner's safe `Reclaimable` state, including terminal children and recorded required reconciliation, and that its applicable operation or media effect is terminal or authoritatively reconciled under its existing owner. Already released mutation history SHALL NOT require an unbounded retained operation-slot ledger; the closed-mutation-set owner SHALL represent already-accounted history only through its exact bounded lower-frontier/coverage evidence. Neither that closed-set evidence nor recovery `CLEAN` or session-close acceptance SHALL establish terminal children, recorded required reconciliation, safe `Reclaimable`, or an operation/media-effect disposition for an operation still owned at the frontier. The exact closed mutation set SHALL have matching persistence and fence evidence for every participating store, ordering domain, topology, and relevant content/recovery generation, and the recovery-state, dirty/integrity, and closed-mutation-set owners SHALL accept the required recovery-`CLEAN` transition for that same set. A generic durable-handoff observation SHALL NOT substitute for those independently owned facts. The session owner SHALL consume those decisions without defining operation terminality, child reconciliation, the closed-set relation, its admission frontier, historical compaction, superseding per-write evidence, or weakening any owner predicate. A clean-close result SHALL require accepted recovery `CLEAN`, the applicable representation-owner clean/closed transition after that evidence or atomically with it, and a durable close observation.

A close precondition rejection, including a session, binding, generation, or owner-evidence mismatch, SHALL create no clean-close transition and SHALL leave the exact prior session and recovery state authoritative. A close commit observed as durable SHALL make only the proposed durable session transition authoritative; it SHALL be reported as cleanly closed only when the recovery-`CLEAN` and representation-owner evidence ordering above is also accepted. A lost, corrupt, or otherwise unclassifiable close observation SHALL leave prior-versus-proposed session authority unresolved and SHALL produce no clean-close; it SHALL NOT by itself authorize or revoke independently owned recovery-`CLEAN` or claim-release decisions, and explicit reconciliation SHALL be required.

Service stop, endpoint withdrawal, durable session close, and store or recovery claim release SHALL remain distinct facts. A service MAY stop or an endpoint MAY be withdrawn while clean close remains unproved; neither event SHALL prove the other or release a claim by implication. A clean session close SHALL NOT itself withdraw an endpoint or release a store or recovery claim. Claim acquisition, alias prevention, and crash-release remain owned by the applicable ownership and store requirements.

#### Scenario: Durable begin precedes writable publication

- **WHEN** a service has current authority and complete writable bindings but the durable begin observation has not succeeded
- **THEN** writable endpoint publication, writable request admission, and protected payload mutation are refused

#### Scenario: Begin precondition is rejected

- **WHEN** begin is rejected because a captured generation, topology, identity, capability, binding, or claim precondition is known not to hold
- **THEN** no session transition is recorded, the exact prior recovery snapshot remains authoritative, and the caller receives a definite refusal rather than a reconciliation-required result

#### Scenario: Begin commit outcome is uncertain

- **WHEN** the begin transition may have committed but its durable observation is lost, corrupt, or otherwise unclassifiable
- **THEN** the service publishes no writable endpoint, treats neither prior nor proposed session authority or claim continuity as proved, and requires recovery reconciliation before a new begin or writable admission

#### Scenario: Clean close has exact owner-approved evidence

- **WHEN** every operation generation still owned at the close frontier has reached the operation-slot owner's safe `Reclaimable` state with terminal children and recorded required reconciliation, each such operation's applicable operation or media-effect evidence is terminal or authoritatively reconciled, already released mutation history is covered by the closed-set owner's exact bounded lower-frontier/coverage evidence, matching store and fence evidence covers the exact closed set, recovery `CLEAN` and the applicable clean/closed representation transition are accepted in the required order or atomically without manufacturing operation-slot or effect facts, and the durable close transition is observed
- **THEN** the session may be reported as cleanly closed while endpoint withdrawal and claim release remain separately evaluated

#### Scenario: Close precondition is rejected

- **WHEN** close is rejected because a session, binding, generation, or owner-evidence precondition is known not to hold
- **THEN** no clean-close transition is recorded, the exact prior session and recovery state remain authoritative, and the result is definite refusal or incomplete closure rather than uncertainty

#### Scenario: Close commit outcome is uncertain

- **WHEN** the close transition may have committed but its durable observation is lost, corrupt, or otherwise unclassifiable
- **THEN** prior-versus-proposed session authority remains unresolved, no clean-close result is reported, and the lifecycle result neither authorizes nor revokes independently owned recovery-`CLEAN` or claim-release decisions; explicit reconciliation is required

#### Scenario: Stop or endpoint withdrawal occurs without clean close

- **WHEN** service stop or endpoint withdrawal succeeds but clean-close evidence is unavailable
- **THEN** the result identifies the completed lifecycle event without asserting clean session closure, recovery `CLEAN`, or safe claim release

#### Scenario: Forced stop or process loss interrupts the session

- **WHEN** forced stop or process loss occurs before all close evidence is durable
- **THEN** no clean-close claim is created; crash-releasing claims may follow their ownership contract, while the durable session and dirty or indeterminate recovery facts remain explicit inputs to reconciliation

#### Scenario: Restart observes a previously closed session

- **WHEN** restart finds a durably closed session with matching authority and accepted close evidence
- **THEN** it may use that fact as prior lifecycle evidence but SHALL establish a new durable begin bound to newly current authority before writable publication

#### Scenario: Restart observes an open or uncertain session

- **WHEN** restart finds an open, partially closed, or uncertain session or a claim or binding mismatch
- **THEN** writable admission is refused until the owning recovery and store authorities reconcile the session, and restart SHALL NOT infer clean closure from process exit, endpoint removal, unmount success, timeout, or flush completion
