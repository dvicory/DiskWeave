## MODIFIED Requirements

### Requirement: Operation slots own backend lifetimes and generations
<!-- dwv:req req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->

Every admitted logical operation SHALL reserve a generation-bearing operation slot before backend submission. The slot SHALL retain the exact canonical normalized request, own its frontend buffer token, frontend tags, child-operation identities, submitted watermarks, drain state, and terminal evidence, and expose the canonical request alongside terminal state. It SHALL become reclaimable only after all children are terminal and required reconciliation is recorded. Frontend abandonment SHALL remove completion-delivery interest only and SHALL NOT release the slot or any owned resource early.

For the bounded retained-operation child-correlation subrelation through the slot owner's `Reclaimable` state, the exact state relationship and transitions represented in `models/quint/PortableOperationExecutionCore.qnt` SHALL remain the sole model authority for registered/accepted/terminal/refused slot children; planned/emitted/accepted/executed/delivered/refused driver work; exact operation-generation, store, store-incarnation, topology, range, and action identity correlation; delayed, partial, out-of-order, and duplicate result observation; conservative short, failed, and uncertain dispositions including refusal of every not-yet-accepted sibling after a non-success completion; one-way abandonment with accepted-work retention; and complete reconciliation, where retrying the recorded outcome is idempotent and replacing it is rejected. The delegated production domain SHALL remain the fixed six-child protected-write identity set; the two-child analysis profiles and deterministic Connect traces SHALL remain finite evidence, not arbitrary-width concurrency or liveness proof. Operation-generation and per-child store, store-incarnation, topology, range, and action values supplied to the relation SHALL be trusted model inputs from their existing owners; the relation SHALL own their exact correlation only and SHALL NOT select, discover, validate, or redefine those upstream facts.

`verification/quint/PortableOperationExecution.qnt` SHALL be evidence-only composition. For child correlation through `Reclaimable`, write and transaction lifecycle, and `ReleaseAllowed`, it SHALL use qualified imports or projections of the existing delegated owners `models/quint/PortableOperationExecutionCore.qnt`, `models/quint/RecoveryProtocol.qnt`, and `models/quint/LifecycleRelease.qnt` rather than define shadow copies of their state machines. Its fixed protected-write driver order, fan-in entry, cross-owner observations, and any post-`Reclaimable` cleanup SHALL remain implementation-correspondence evidence or existing-owner interactions, not delegated product semantics. The evidence composition MAY retain only evidence-local sequencing and correspondence state needed to drive those owner relations. It SHALL NOT own or redefine canonical request fields, frontend buffers or tags, resource accounting, operation-slot generation reuse, stale-slot lookup, drain-required or drain-complete policy, submitted watermarks, persistence-evidence admissibility, physical store behavior, recovery authority, transaction semantics, lifecycle `ReleaseAllowed` composition, frontend abandonment, or any other surrounding operation-slot or service semantics.

`verification/quint/PortableOperationExecution.qnt` SHALL supply `RecoveryProtocol` only owner-admitted current-write observations. Because that owner model does not decide whether a concrete result belongs to a prior write after release and reuse, the evidence composition SHALL require a separately supplied trusted current-write-to-operation-generation correlation fact from the applicable current owner before projecting transaction satisfaction into `LifecycleRelease`; it SHALL NOT manufacture that binding.

#### Scenario: A slot is reused after a terminal operation

- **WHEN** all child operations are terminal and reconciliation is complete
- **THEN** the slot may be reclaimed and a later reservation receives a generation that prevents stale completions from addressing its resources

#### Scenario: A stale completion arrives

- **WHEN** completion references a released slot generation
- **THEN** the adapter rejects it before resource lookup and never applies it to reused memory

#### Scenario: A child operation completes twice

- **WHEN** a terminal child receives a duplicate completion
- **THEN** the duplicate is recorded and ignored without changing the semantic result or reclaiming resources early

#### Scenario: A non-success completion refuses unaccepted siblings

- **WHEN** a child completes with short, failed, or uncertain disposition
- **THEN** every sibling not yet accepted is refused before reconciliation, while accepted siblings remain owned and the slot remains unreclaimable until their terminal evidence is recorded

#### Scenario: Reconciliation retries the recorded outcome

- **WHEN** an owner retries reconciliation after the first reconciliation attempt has already recorded an outcome
- **THEN** the same outcome is accepted as an idempotent no-op, while a different outcome is rejected without changing the recorded evidence or reclaimability

#### Scenario: Canonical identity reaches terminal evidence

- **WHEN** an admitted operation reaches completion, failure, uncertainty, or reconciled terminal state
- **THEN** its slot evidence exposes the exact frontend, request, target slot, topology epoch, operation, range, buffer token, submission sequence, ordering intent, and durability intent admitted for that operation

#### Scenario: Completion interest is abandoned while a child remains active

- **WHEN** the frontend abandons completion delivery after backend submission but before every child is terminal
- **THEN** completion delivery is suppressed while the slot, canonical request, buffer, child identity, and reconciliation obligations remain owned until safe reclamation

#### Scenario: Correspondence evidence joins existing model owners

- **WHEN** portable-operation correspondence evidence combines retained-child, write or transaction, and release-authorization observations
- **THEN** it drives qualified instances of the existing delegated owner relations with trusted fixture inputs and retains only evidence-local sequencing, without creating a second semantic owner or changing product behavior
