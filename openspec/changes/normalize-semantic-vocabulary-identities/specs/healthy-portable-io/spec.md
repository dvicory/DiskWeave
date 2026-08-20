## RENAMED Requirements

- FROM: ### Requirement: Durable completion and clean checkpoint require fences
- TO: ### Requirement: Durable completion and recovery CLEAN require persistence evidence

## MODIFIED Requirements

### Requirement: Writes follow the reference transaction and update single XOR parity
<!-- dwv:req req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity -->
<!-- dwv:requires req.dirty-integrity-invalidation.write-recovery-record-precedes-data-parity-write -->
<!-- dwv:requires req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic -->
<!-- dwv:requires req.recovery-state-semantics.data-parity-write-requires-write-recovery-record -->
<!-- dwv:requires req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.xor-reference-model.incremental-updates-and-full-recomputation-are-equivalent -->
<!-- dwv:requires req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence -->

A protected write SHALL compose the canonical checked dirty-region mapping, durable write-recovery-record owner, atomic recovery transaction, delegated reference-transaction relation, single-XOR computation, and exact-range store operations. This requirement owns service orchestration only. It SHALL preserve each owner's result, emit no protected member data/parity write before the write-recovery-record owner succeeds, and SHALL not reinterpret the delegated reference relation or the non-delegated owner predicates.

#### Scenario: Partial write requires read-modify-write

- **WHEN** a write covers part of a parity extent
- **THEN** the service composes the required old data and parity reads, reference-equivalent XOR update, and exact writes under the delegated transaction relation and after the write-recovery-record owner succeeds

#### Scenario: Full overwrite is aligned

- **WHEN** a write fully covers the required data and parity extent
- **THEN** the service may avoid old-data reads only under the XOR contract while producing parity bytes equal to full recomputation

### Requirement: Durable completion and recovery CLEAN require persistence evidence
<!-- dwv:req req.healthy-portable-io.durable-completion-and-recovery-clean-require-persistence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->
<!-- dwv:requires req.dirty-integrity-invalidation.recovery-clean-requires-persistence-evidence -->

The service SHALL preserve normalized request durability intent and compose accepted store watermarks, persistence evidence, and exact dirty-region clear decisions. It may report durable completion and commit recovery state `CLEAN` only when each owner accepts current-generation evidence. Unsupported durability requirements SHALL be rejected or reported at the explicitly established weaker scope; the service SHALL NOT restate or weaken an owner's persistence-evidence predicate.

#### Scenario: All required owners accept completion evidence

- **WHEN** data/parity writes are complete and store, recovery, and dirty owners accept the matching evidence
- **THEN** the service may commit recovery state `CLEAN` only for the proven regions and may report the corresponding durable completion

#### Scenario: An owner rejects completion evidence

- **WHEN** any required owner rejects missing, volatile, stale, partial, future, or mismatched evidence
- **THEN** the service reports the conservative result and leaves affected state dirty, stale, or indeterminate

### Requirement: Abandonment, restart, and failure preserve operation safety
<!-- dwv:req req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->

The service SHALL compose frontend delivery interest, operation-slot lifetime, transaction outcome, and durable dirty/restart consequences without redefining them. It SHALL keep admitted work and resources under their owners until required reconciliation and release, preserve every conservative failure result, and never blindly retry an uncertain non-idempotent write.

#### Scenario: Request is abandoned after intent

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
