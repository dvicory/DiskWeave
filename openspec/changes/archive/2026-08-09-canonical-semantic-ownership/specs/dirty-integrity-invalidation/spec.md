## MODIFIED Requirements

### Requirement: Checkpoint and clear require fence evidence
<!-- dwv:req req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:refines req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence -->

The dirty protocol SHALL select only regions covered by matching typed recovery authority for every required store watermark and captured topology, region, checksum, capability, store-incarnation, and recovery generation. Future, stale, partial, omitted-region, or cross-store evidence SHALL fail closed. The dirty protocol owns the exact region subset and generation checks for clear; the recovery-state capability owns which typed fence evidence may authorize a clean claim.

#### Scenario: All selected regions have covering authority

- **WHEN** every selected region is covered by matching current typed recovery authority
- **THEN** the protocol may request one durable recovery transaction that clears exactly those regions

#### Scenario: Evidence omits a selected region or generation

- **WHEN** any selected region lacks covering authority or a captured generation changed
- **THEN** clear is refused and dirty or reconciliation-required state remains

### Requirement: Failures and restart are conservative
<!-- dwv:req req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->

Any short, failed, uncertain, abandoned, crashed, or post-intent recovery result SHALL preserve dirty or indeterminate evidence for every affected region. This requirement owns the durable dirty/restart consequence after such a result; frontend abandonment meaning, operation-resource lifetime, and transaction-state transitions remain owned by their respective capabilities. Restart SHALL discover durable dirty evidence and SHALL NOT infer clean state from elapsed time, process success, or a missing action result.

#### Scenario: Home write fails after intent

- **WHEN** a data or parity home operation is short, failed, or uncertain after intent is durable
- **THEN** the affected state remains dirty or indeterminate and no clean completion is reported

#### Scenario: Process loss occurs before checkpoint

- **WHEN** process state is lost after a home mutation but before fence and checkpoint
- **THEN** restart enters recovery or blocked handling with dirty evidence rather than assuming the write was clean
