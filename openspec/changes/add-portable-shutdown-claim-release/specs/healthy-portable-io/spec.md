## ADDED Requirements

### Requirement: Portable shutdown preserves operation ownership and claim-release ordering
<!-- dwv:req req.healthy-portable-io.portable-shutdown-preserves-operation-ownership-and-claim-release-ordering -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->
<!-- dwv:requires req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts -->
<!-- External target prerequisite: req.recovery-state-semantics.durable-writable-session-lifecycle-binds-authority-and-close-evidence from define-portable-writable-session-lifecycle; add the normal owner edge only after that change completes its canonical transition and before implementation, validation, or canonical sync. -->

A portable service shutdown SHALL stop new admissions and quiesce frontends and namespace writers, then establish a shutdown close frontier. It SHALL require every operation generation still owned at that frontier to reach the operation-slot owner's safe `Reclaimable` state with every child terminal and required reconciliation recorded, and it SHALL require each such operation's applicable operation or media effect to be terminal or authoritatively reconciled under its existing owner. It SHALL then obtain only owner-approved recovery `CLEAN` and durable session-close evidence, whose owners consume but SHALL NOT manufacture those operation-slot or effect facts; withdraw every exported writable endpoint; and release each store, recovery, or operation-associated claim only after its independent owner permits release and no writable alias can remain. The service SHALL preserve frontend abandonment, operation-slot lifetime, operation/media-effect, transaction, dirty/restart, store-watermark, persistence-evidence, and generation-qualified release-authorization semantics owned by their respective requirements rather than redefining them.

A shutdown SHALL report a clean result only when every operation generation still owned at the shutdown close frontier satisfies those exact generation-qualified operation-slot and effect predicates, while already released mutation history is covered by the closed-mutation-set owner's exact bounded lower-frontier/coverage evidence rather than an unbounded retained operation-slot ledger, required recovery-`CLEAN` and durable session-close evidence is accepted for the exact affected scope and generations, every exported endpoint is withdrawn, and each released claim satisfies its independent owner predicate. Missing, stale, partial, volatile, mismatched, or unavailable evidence SHALL produce failure or reconciliation-required state and SHALL NOT produce a clean-close claim. A forced or process-lost shutdown MAY leave dirty or indeterminate durable state, but SHALL NOT write or report a clean close certificate merely because a timeout or process exit occurred.

The shutdown contract SHALL make no claim about custody continuity, current or historical protection, lineage, recovery authority, payload integrity, or stable persistent-format status. It SHALL not authorize post-gap publication, currentization, recovery mutation, destructive rebaseline, retention, or historical recovery.

#### Scenario: Clean portable shutdown completes

- **WHEN** admission is closed, frontends and namespace writers are quiesced, the shutdown close frontier is established, every operation generation still owned at that frontier is safely `Reclaimable` with every child terminal and required reconciliation recorded, each such operation's applicable operation or media effect is terminal or authoritatively reconciled, already released mutation history is covered by the closed-mutation-set owner's exact bounded lower-frontier/coverage evidence, exact recovery-`CLEAN` and durable session-close evidence is accepted without manufacturing operation-slot or effect facts, exported writable endpoints are withdrawn, each applicable claim-release owner permits release, and no writable alias remains
- **THEN** the service releases only those independently authorized claims and reports a bounded clean shutdown without adding a stronger recovery or protection claim

#### Scenario: An admitted operation is abandoned during shutdown

- **WHEN** a frontend withdraws completion interest after admission but before the owned operation generation reaches safe `Reclaimable` or its applicable operation/media effect is terminal or authoritatively reconciled
- **THEN** shutdown retains the operation slot, backend resources, durability obligations, and conservative dirty or indeterminate consequences until every child is terminal, required reconciliation is recorded, safe `Reclaimable` is reached, the applicable effect is settled by its owner, and every independent release owner permits release; it does not report a clean result early

#### Scenario: Required recovery or close evidence is unavailable

- **WHEN** an operation generation has not reached safe `Reclaimable`, a child is non-terminal, required reconciliation is unrecorded, an applicable operation/media effect is unresolved, or exact store-watermark, persistence, recovery-`CLEAN`, or durable session-close evidence is missing, stale, partial, volatile, mismatched, or otherwise rejected by its owner
- **THEN** shutdown reports failure or reconciliation-required state, keeps the affected claim or recovery consequence conservative, and does not report a clean recovery state or clean close

#### Scenario: Endpoint withdrawal cannot be proven

- **WHEN** an exported writable endpoint cannot be withdrawn or a writable alias may remain
- **THEN** shutdown refuses to release the related claim and reports failure or reconciliation-required state without claiming that the service is cleanly stopped

#### Scenario: Shutdown is forced or the owner process is lost

- **WHEN** a forced stop or process loss occurs before the portable shutdown sequence establishes clean evidence
- **THEN** the service does not manufacture a clean close certificate; durable dirty or indeterminate state, stale endpoint evidence, and any required reconciliation remain explicit inputs to the next safe action
