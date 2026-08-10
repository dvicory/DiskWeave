## MODIFIED Requirements

### Requirement: Durable intent precedes every protected home mutation
<!-- dwv:req req.explicit-transaction-machine.durable-intent-precedes-every-protected-home-mutation -->
<!-- dwv:refines req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation -->

The reference machine SHALL emit no protected home-mutation action until it receives a durable invalidation result satisfying the owning dirty/integrity policy for the captured generations. This requirement owns only transaction action emission: a rejected, lost, corrupt, indeterminate, or stale intent result transitions the transaction to blocked or reconciliation-required without emitting a protected mutation.

#### Scenario: Durable invalidation result is accepted

- **WHEN** the owner reports durable dirty and stale intent at the captured recovery generation
- **THEN** the machine may emit its local read, compute, and write actions under the captured topology

#### Scenario: Durable invalidation result is unavailable

- **WHEN** the result is rejected, lost, corrupt, indeterminate, or stale
- **THEN** no protected home-mutation action is emitted and the transaction records the conservative outcome

### Requirement: Clean and checkpoint claims require fence evidence
<!-- dwv:req req.explicit-transaction-machine.clean-and-checkpoint-claims-require-fence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence -->
<!-- dwv:refines req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence -->

The reference machine SHALL emit flush and fence actions after writes and SHALL emit checkpoint, clear, and range-release actions only after all child operations are terminal and the owning recovery and dirty protocols accept the supplied current evidence. This requirement owns action order and release points, not the complete watermark, typed-authority, or exact-region predicates.

#### Scenario: Recovery authority accepts the completed write set

- **WHEN** all writes are terminal and the recovery and dirty protocols accept the current covering evidence
- **THEN** the machine emits checkpoint and clear, then releases its range guard

#### Scenario: Recovery authority rejects the evidence

- **WHEN** the evidence is volatile, incomplete, stale, or otherwise rejected by an owner
- **THEN** checkpoint, clear, and release are not emitted and the transaction remains reconciliation-required

### Requirement: Failure, abandonment, and crash states are conservative
<!-- dwv:req req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->

The reference machine SHALL distinguish failed, uncertain, abandoned-delivery-interest, process-lost, and reconciliation-required transaction outcomes. It SHALL preserve action order and transaction state until backend lifetime and durable recovery consequences are reconciled by their owners. It SHALL not reinterpret frontend abandonment as cancellation, infer clean state after process loss, or emit release before the operation-lifetime owner permits reclamation.

#### Scenario: Delivery interest is abandoned after intent

- **WHEN** the frontend owner reports abandoned delivery interest before transaction checkpoint
- **THEN** the machine suppresses no semantic work, continues drain or reconciliation, and records its local terminal transaction state

#### Scenario: Process loss follows a home write

- **WHEN** process state is lost after a home write but before fence and checkpoint
- **THEN** restart enters the transaction's reconciliation-required path and relies on durable recovery state rather than the missing action result
