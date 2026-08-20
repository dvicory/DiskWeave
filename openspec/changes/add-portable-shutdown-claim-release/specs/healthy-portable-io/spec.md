## ADDED Requirements

### Requirement: Portable shutdown preserves operation ownership and claim-release ordering
<!-- dwv:req req.healthy-portable-io.portable-shutdown-preserves-operation-ownership-and-claim-release-ordering -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->

A portable service shutdown SHALL stop new admissions, quiesce frontends and namespace writers, drain or durably hand off every admitted operation, reconcile indeterminate completions where possible, obtain only owner-approved recovery CLEAN and close-session evidence, withdraw every exported writable endpoint, and release store or recovery claims only after no writable alias can remain. The service SHALL preserve frontend abandonment, operation-slot lifetime, transaction, dirty/restart, store-watermark, and persistence-evidence semantics owned by their respective requirements rather than redefining them.

A shutdown SHALL report a clean result only when all required admitted work is terminal or durably handed off, required recovery CLEAN and close-session evidence is accepted for the exact affected scope and generations, every exported endpoint is withdrawn, and claim release is safe. Missing, stale, partial, volatile, mismatched, or unavailable evidence SHALL produce failure or reconciliation-required state and SHALL NOT produce a clean-close claim. A forced or process-lost shutdown MAY leave dirty or indeterminate durable state, but SHALL NOT write or report a clean close certificate merely because a timeout or process exit occurred.

The shutdown contract SHALL make no claim about custody continuity, current or historical protection, lineage, recovery authority, payload integrity, or stable persistent-format status. It SHALL not authorize post-gap publication, currentization, recovery mutation, destructive rebaseline, retention, or historical recovery.

#### Scenario: Clean portable shutdown completes

- **WHEN** admission is closed, frontends and namespace writers are quiesced, all admitted operations are terminal or durably handed off, exact recovery CLEAN and close-session evidence is accepted, exported writable endpoints are withdrawn, and no writable alias remains
- **THEN** the service releases its claims and reports a bounded clean shutdown without adding a stronger recovery or protection claim

#### Scenario: An admitted operation is abandoned during shutdown

- **WHEN** a frontend withdraws completion interest after admission but before the owned operation reaches terminal or reconciliation state
- **THEN** shutdown retains the operation slot, backend resources, durability obligations, and conservative dirty or indeterminate consequences until the owning lifecycle permits release, and it does not report a clean result early

#### Scenario: Required recovery or close evidence is unavailable

- **WHEN** drain, reconciliation, exact store-watermark evidence, persistence evidence, or close-session evidence is missing, stale, partial, volatile, mismatched, or otherwise rejected by its owner
- **THEN** shutdown reports failure or reconciliation-required state, keeps the affected claim or recovery consequence conservative, and does not report a clean recovery state or clean close

#### Scenario: Endpoint withdrawal cannot be proven

- **WHEN** an exported writable endpoint cannot be withdrawn or a writable alias may remain
- **THEN** shutdown refuses to release the related claim and reports failure or reconciliation-required state without claiming that the service is cleanly stopped

#### Scenario: Shutdown is forced or the owner process is lost

- **WHEN** a forced stop or process loss occurs before the portable shutdown sequence establishes clean evidence
- **THEN** the service does not manufacture a clean close certificate; durable dirty or indeterminate state, stale endpoint evidence, and any required reconciliation remain explicit inputs to the next safe action
