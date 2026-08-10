## MODIFIED Requirements

### Requirement: Assembly and request admission are bounded and identity-safe
<!-- dwv:req req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->

The service SHALL assemble exactly one compatible member binding for every required assignment in a validated captured topology and admit each canonical request through a generational operation slot. Local collection, discovery, or vector order SHALL NOT define slot, role, coding position, assignment, or target identity. Before child I/O, the service SHALL resolve the request's stable target slot through its captured topology and reject missing, extra, duplicate, aliased, role/position-mismatched, assignment-mismatched, or stale bindings and requests.

#### Scenario: Healthy topology is assembled

- **WHEN** all required data/parity roles have unambiguous identity and compatible capabilities
- **THEN** the service enters serving state and accepts normalized requests with a captured topology epoch

#### Scenario: Ambiguous or stale assembly is attempted

- **WHEN** identity evidence is ambiguous, a required role is unavailable, or a captured generation is stale
- **THEN** assembly or request admission fails closed without mutating a member

#### Scenario: Collection order differs from topology order

- **WHEN** member collection order and topology assignment order differ while every binding retains the same stable slot, role, coding position, assignment instance, and generation
- **THEN** the same request slot resolves to the same assignment and member

#### Scenario: A positional binding disagrees with topology

- **WHEN** collection position would select a different member than the request slot's captured assignment
- **THEN** assembly or request admission fails before resource reservation, child I/O, or member mutation

### Requirement: Writes follow the reference transaction and update single XOR parity
<!-- dwv:req req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity -->
<!-- dwv:requires req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation -->
<!-- dwv:requires req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic -->
<!-- dwv:requires req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent -->
<!-- dwv:requires req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions -->
<!-- dwv:requires req.xor-reference-model.incremental-updates-and-full-recomputation-are-equivalent -->
<!-- dwv:requires req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence -->

A protected write SHALL compose the canonical checked dirty-region mapping, durable invalidation owner, atomic recovery transaction, reference transaction actions, single-XOR computation, and exact-range store operations. This requirement owns service orchestration only. It SHALL preserve each owner's result and SHALL emit no protected member mutation before the durable-intent owner succeeds.

#### Scenario: Partial write requires read-modify-write

- **WHEN** a write covers part of a parity extent
- **THEN** the service composes the required old data and parity reads, reference-equivalent XOR update, and exact writes after the durable-intent owner succeeds

#### Scenario: Full overwrite is aligned

- **WHEN** a write fully covers the required data and parity extent
- **THEN** the service may avoid old-data reads only under the XOR contract while producing parity bytes equal to full recomputation

### Requirement: Durable completion and clean checkpoint require fences
<!-- dwv:req req.healthy-portable-io.durable-completion-and-clean-checkpoint-require-fences -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence -->
<!-- dwv:requires req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence -->

The service SHALL preserve normalized durability intent and compose accepted store watermarks, typed recovery authority, and exact dirty-region clear decisions. It may report durable completion and request a clean checkpoint only when each owner accepts the evidence at current generations. Unsupported durability requirements SHALL be rejected or reported at the explicitly established weaker scope; the service SHALL NOT restate or weaken an owner's fence predicate.

#### Scenario: All required owners accept completion evidence

- **WHEN** data and parity writes are terminal and store, recovery, and dirty owners accept the matching evidence
- **THEN** recovery may checkpoint only the proven regions and the service may report the corresponding durable completion

#### Scenario: An owner rejects completion evidence

- **WHEN** any required owner rejects missing, volatile, stale, partial, future, or mismatched evidence
- **THEN** the service reports the conservative result and leaves affected state dirty, stale, or uncertain

### Requirement: Abandonment, restart, and failure preserve operation safety
<!-- dwv:req req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->

The service SHALL compose frontend delivery interest, operation-slot lifetime, transaction outcome, and durable dirty/restart consequences without redefining them. It SHALL keep admitted work and resources under their owners until terminal reconciliation, preserve every conservative failure result, and never blindly retry an uncertain non-idempotent write.

#### Scenario: Request is abandoned after intent

- **WHEN** the frontend abandons delivery interest after durable intent but before terminal completion
- **THEN** the service continues owned drain and reconciliation, preserves dirty evidence as required, and releases resources only after the operation-lifetime owner permits it

#### Scenario: Service restarts with dirty state

- **WHEN** the process restarts after an incomplete write or fence
- **THEN** the service follows the durable recovery disposition and never infers clean state from a missing completion
