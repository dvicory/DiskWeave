## MODIFIED Requirements

### Requirement: Writes follow the reference transaction and update single XOR parity
<!-- dwv:req req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity -->
<!-- dwv:requires req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation -->
<!-- dwv:requires req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic -->
<!-- dwv:requires req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent -->
<!-- dwv:requires req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.xor-reference-model.incremental-updates-and-full-recomputation-are-equivalent -->
<!-- dwv:requires req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence -->

A protected write SHALL compose the canonical checked dirty-region mapping, durable invalidation owner, atomic recovery transaction, delegated reference-transaction relation, single-XOR computation, and exact-range store operations. This requirement owns service orchestration only. It SHALL preserve each owner's result, emit no protected member mutation before the durable-intent owner succeeds, and SHALL not reinterpret the delegated reference relation or the non-delegated owner predicates.

#### Scenario: Partial write requires read-modify-write

- **WHEN** a write covers part of a parity extent
- **THEN** the service composes the required old data and parity reads, reference-equivalent XOR update, and exact writes under the delegated transaction relation and after the durable-intent owner succeeds

#### Scenario: Full overwrite is aligned

- **WHEN** a write fully covers the required data and parity extent
- **THEN** the service may avoid old-data reads only under the XOR contract while producing parity bytes equal to full recomputation

### Requirement: Abandonment, restart, and failure preserve operation safety
<!-- dwv:req req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->

The service SHALL compose frontend delivery interest, operation-slot lifetime, transaction outcome, and durable dirty/restart consequences without redefining them. It SHALL keep admitted work and resources under their owners until terminal reconciliation, preserve every conservative failure result, and never blindly retry an uncertain non-idempotent write.

#### Scenario: Request is abandoned after intent

- **WHEN** the frontend abandons delivery interest after durable intent but before terminal completion
- **THEN** the service continues owned drain and reconciliation, preserves dirty evidence as required, and releases resources only after the operation-lifetime owner permits it

#### Scenario: Service restarts with dirty state

- **WHEN** the process restarts after an incomplete write or fence
- **THEN** the service follows the durable recovery disposition and never infers clean state from a missing completion
