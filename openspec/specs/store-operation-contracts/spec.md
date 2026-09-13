# store-operation-contracts Specification

## Purpose
The store and operation contract separates portable range semantics from concrete files or devices while making partial completion, persistence evidence, uncertain effects, identity, capability limits, and resource lifetime explicit.
## Requirements
### Requirement: Stores report exact range outcomes and persistence evidence
<!-- dwv:req req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence -->

Each store operation SHALL identify the operation, requested range, exact completed ranges, a disposition of success, short, failed, uncertain, or duplicate, a stable error class when known, and persistence evidence. Reads, writes, flushes, write-zeroes, and discard use exact-range semantics. A timeout or lost completion with unknown media effect SHALL be uncertain rather than hidden by an adapter retry.

#### Scenario: An exact write completes

- **WHEN** a store completes the requested write range with known persistence behavior
- **THEN** completion reports the exact range and strongest evidence actually established for the store and watermark

#### Scenario: A short write occurs

- **WHEN** only part of the requested range completes
- **THEN** completion reports the completed subset and short disposition so the caller can apply its own policy

#### Scenario: Completion persistence is unknown

- **WHEN** timeout or disappearance leaves the media effect unknown
- **THEN** the store reports uncertain persistence and the caller does not infer durable success or safe retry

### Requirement: Store write watermarks are real monotonic evidence
<!-- dwv:req req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->

Each store incarnation and ordering domain SHALL assign a monotonic watermark to every accepted write. Write completion SHALL report the exact assigned watermark. Flush evidence SHALL name the same store incarnation and SHALL report a synchronized-through watermark no greater than the highest write that the store actually synchronized. Sentinel, guessed, future, stale, partial, or cross-store watermarks SHALL NOT authorize a fence, recovery state `CLEAN` transition, or resource release.

#### Scenario: A store synchronizes accepted writes

- **WHEN** writes receive watermarks W1 through Wn and a successful flush synchronizes through Wk
- **THEN** the resulting evidence names that store and Wk, where Wk is an accepted watermark and no later write is implied durable

#### Scenario: Fence evidence cites an invalid watermark

- **WHEN** evidence cites a future watermark, a watermark from another store/incarnation, or incomplete synchronization
- **THEN** fence composition fails closed and affected recovery state remains dirty or indeterminate

### Requirement: Capability evidence determines the allowed safety profile
<!-- dwv:req req.store-operation-contracts.capability-evidence-determines-the-allowed-safety-profile -->

The system SHALL represent logical and physical geometry, alignment, transfer limits, flush and FUA support, ordering, torn-write model, volatile-cache model, write-zeroes and discard support, sparse behavior, cancellation behavior, and identity sources as evidence. It SHALL distinguish simulation-certified, portable-demo, production-read-only, and production-write-safe profiles and refuse a profile whose required evidence is absent or unknown.

#### Scenario: A store has simulation-certified behavior

- **WHEN** a deterministic store satisfies the formal simulator model
- **THEN** it may use the simulation-certified profile without claiming physical power-loss certification

#### Scenario: Flush capability is unknown

- **WHEN** a store cannot provide certified flush evidence for a write-safe profile
- **THEN** production-write-safe assembly is refused or limited to a named safer profile

#### Scenario: Geometry cannot satisfy alignment

- **WHEN** a range cannot be represented under logical, physical, minimum, or required alignment limits
- **THEN** the store rejects or explicitly adapts the operation and reports the constraint rather than issuing unsafe I/O

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

### Requirement: Resource admission and identity remain bounded and explicit
<!-- dwv:req req.store-operation-contracts.resource-admission-and-identity-remain-bounded-and-explicit -->

The system SHALL bound live operation slots, buffers, backend submissions, retries, range locks, and background work. Store disappearance or identity/geometry change SHALL invalidate operations under the captured topology without rewriting that snapshot. Callers SHALL decide retry legality from explicit idempotence and duplicate semantics.

#### Scenario: Admission reaches its bound

- **WHEN** no operation slot or buffer is available within the configured bound
- **THEN** admission applies backpressure or returns a bounded resource error instead of allocating unbounded work

#### Scenario: A store changes identity

- **WHEN** a store reappears with changed or conflicting identity observations
- **THEN** operations under the old topology are invalidated and writable reuse is refused until a new topology decision

#### Scenario: A non-idempotent operation is uncertain

- **WHEN** a write may have reached media but completion is uncertain
- **THEN** the caller does not blindly retry it and preserves the uncertain evidence for reconciliation

### Requirement: Store failures are conservative and testable
<!-- dwv:req req.store-operation-contracts.store-failures-are-conservative-and-testable -->

Store contracts SHALL define deterministic behavior for EIO, timeout, delayed/out-of-order completion, duplicate/stale delivery, disappearance/reappearance, unsupported capability, topology-generation mismatch, and bounded exhaustion. Unknown, ambiguous, or indeterminate evidence SHALL remain visible and SHALL not authorize automatic repair, clean state, or unsafe writable assembly.

#### Scenario: A backend returns EIO

- **WHEN** a child operation fails with a known backend error
- **THEN** the completion reports failed with stable error evidence and the slot remains unreleased until reconciliation

#### Scenario: A completion arrives out of order

- **WHEN** child completions arrive in an order different from submission
- **THEN** generation and child identity checks preserve exact accounting and do not infer ordering or durability that was not proven

#### Scenario: Identity evidence is ambiguous

- **WHEN** observations could describe a clone, replacement, or conflicting geometry
- **THEN** writable assembly and automatic reconstruction are refused pending an explicit topology decision

### Requirement: Portable evidence does not certify concrete stores
<!-- dwv:req req.store-operation-contracts.portable-evidence-does-not-certify-concrete-stores -->

The store artifacts SHALL distinguish standard-library/fake-adapter and simulator evidence from file-backed, SQLite, Linux, macOS, device, power-loss, and hardware evidence. A validated portable contract SHALL not be presented as production-write-safe certification.

#### Scenario: Only a fake adapter is available

- **WHEN** the current environment can run deterministic store contract tests but not a real device
- **THEN** portable semantics may be accepted while concrete-store and physical durability gates remain visible and unmet

#### Scenario: A later adapter selects a crate or runtime

- **WHEN** a later implementation chooses io_uring, ublk, SQLite, or another library
- **THEN** it remains behind this boundary and must provide separate conformance and evidence without changing the store semantics

