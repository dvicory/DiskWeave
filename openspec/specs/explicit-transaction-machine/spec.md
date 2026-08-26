# explicit-transaction-machine Specification

## Purpose
This capability provides an auditable reference state machine for write-recovery records, data/parity writes, persistence evidence, recovery CLEAN, interrupted processing, reconciliation, and explicit release.
## Requirements
### Requirement: Transactions emit normalized semantic actions
<!-- dwv:req req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions -->

The reference machine SHALL expose the semantic action vocabulary for range acquisition, write-recovery-record durability confirmation, reads, parity computation, data/parity writes, persistence-evidence observation, recovery-`CLEAN` commit, and range release. Actions SHALL contain semantic ranges, generations, identities, and evidence rather than backend child-operation or runtime types. The normalized action vocabulary and any ordering of finer actions not represented by the delegated relation remain owned by this requirement and applicable current requirements. The delegated canonical `RecoveryProtocol` module SHALL define only the exact parameterized state, abstract actions, transition guards, outcomes, and release relation it declares.

#### Scenario: A write transaction starts

- **WHEN** a caller starts a protected write with affected regions and integrity extents
- **THEN** the normalized trace represents the request with the semantic action vocabulary and supplied semantic evidence without exposing backend runtime types

#### Scenario: Backend fanout is used

- **WHEN** one semantic action is implemented by several child operations
- **THEN** the reference trace records one semantic action and leaves child completion accounting to the operation-slot layer

#### Scenario: A completed or aborted write still requires release

- **WHEN** a transaction reaches a `CompletedAwaitingRelease` or `AbortedAwaitingRelease` result
- **THEN** the normalized trace still exposes the delegated `releaseRange` action before the semantic range can be reused

### Requirement: Reference traces are deterministic and implementation-independent
<!-- dwv:req req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->

The reference machine SHALL emit versioned normalized action traces with stable semantic result representation and implementation-independent pre/post observations. For an admitted current write, the exact parameterized state, abstract action, transition, outcome, ordering, release, invalid-transition, repeated-result, and invariant relation SHALL be delegated to the canonical `RecoveryProtocol` module in `models/quint/RecoveryProtocol.qnt`; that module is the sole authority for those exact abstract protocol semantics within its declared surface. The normalized trace vocabulary and ordering for finer actions absent from that relation remain owned by the applicable OpenSpec requirements. The relation SHALL NOT determine whether a concrete result belongs to a prior write after release and reuse; result correlation or generation remains owned by the applicable current evidence requirement, and Quint SHALL receive only an admitted current-write observation. Exact region mapping, checksum extent semantics, topology identity, persistence-evidence admissibility, store persistence, adapter commit observations, operation-slot lifetime, and production recovery authority remain owned by their existing requirements.

#### Scenario: The same plan is replayed

- **WHEN** the same admitted semantic plan and owner observations are replayed through the reference boundary
- **THEN** the normalized trace and stable semantic result representation are identical; the exact state transition remains defined by the canonical `RecoveryProtocol` module

#### Scenario: A completed or aborted write cannot restart before release

- **WHEN** a `startWrite` action is attempted while the prior write is `CompletedAwaitingRelease` or `AbortedAwaitingRelease` but its range has not been released
- **THEN** the delegated relation exposes no `startWrite` transition and preserves the owned completed or aborted state

#### Scenario: A released range starts another write

- **WHEN** the prior completed or aborted write has completed its explicit `releaseRange` action
- **THEN** a subsequent `startWrite` transition may acquire the released range and clears the prior release marker

#### Scenario: A result is repeated or out of order

- **WHEN** an already-correlated result does not match the currently pending semantic action or repeats an already consumed result for the current write
- **THEN** the relation exposes no state transition and preserves the prior semantic state and ownership

#### Scenario: A result crosses an ownership release boundary

- **WHEN** a concrete result arrives after one write was released and a new write acquired the same range
- **THEN** the owning correlation or generation requirement decides whether the result belongs to the current write; the delegated relation receives only an admitted current-write observation

#### Scenario: Request is abandoned before data/parity durability is established

- **WHEN** frontend delivery is abandoned after a data/parity write in `DataParityWritesAwaitingDurability` but before all affected regions are represented as known applied
- **THEN** the relation preserves an unknown or indeterminate data/parity-write outcome, retains the owned write for reconciliation, and does not admit a recovery-`CLEAN` transition until write coverage is complete

#### Scenario: Uncertainty is reconciled

- **WHEN** `WriteRecoveryRecordUnknown` is reconciled with `CommitRejected` or `CommitDurable`, or `DataParityWritesUnknown` is reconciled with `DataParityWriteEffectIndeterminate` or `DataParityWriteEffectDurable`
- **THEN** the relation applies only that domain's observation vocabulary and conservative state; clean and completed outcomes remain unavailable unless their delegated evidence predicates hold

#### Scenario: An invalid transition is attempted

- **WHEN** a supplied action or owner result violates a non-delegated admissibility requirement
- **THEN** the implementation reports the owning semantic failure and does not infer delegated state behavior from private implementation details

#### Scenario: Equivalent implementations are compared

- **WHEN** implementations receive the same admitted semantic plan and owner observations within the delegated protocol surface
- **THEN** comparison uses the versioned normalized trace and stable semantic result representation rather than private enum layout, batching, runtime, or database identity

#### Scenario: A non-delegated authority decision is evaluated

- **WHEN** an implementation must decide exact persistence-evidence, topology, persistence, or resource-lifetime admissibility
- **THEN** it consults the owning current requirement and does not infer that decision from the Quint model

#### Scenario: A finite evidence bound is interpreted

- **WHEN** `evidence.write-recovery-lifecycle-model` reports the two-region, two-store finite analysis instance with bounded checker depth 12
- **THEN** the report treats those values as reproducible verification scope rather than a product cardinality limit or an unbounded protocol proof

### Requirement: Coded parity-range authority covers shared parity conflicts
<!-- dwv:req req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts -->
<!-- dwv:requires req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics -->
<!-- dwv:requires req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:requires req.xor-reference-model.xor-parity-uses-explicit-protected-geometry -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.store-operation-contracts.resource-admission-and-identity-remain-bounded-and-explicit -->
<!-- dwv:requires req.store-operation-contracts.store-failures-are-conservative-and-testable -->
<!-- dwv:requires req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts -->

The transaction boundary SHALL own the product conflict meaning for a bounded coded-range authority claim derived from the exact validated semantic mutation unit, captured topology epoch, assignment instances and generations, protected geometry, and selected coding profile. A semantic mutation unit MAY be a validated request-decomposition unit rather than an undecomposed frontend request, but it SHALL carry a complete mapped coded claim covering every dependent basis observation and mutation before admission. Under one captured topology and profile, operations SHALL conflict when their mapped parity/codeword claims intersect, even when they target different member identities; per-member identity, store range, dirty region, or checksum extent SHALL NOT establish or remove a coded conflict.

Partial physical acquisition is pre-admission and authorizes no dependent read or mutation. The healthy service SHALL perform no dependent basis I/O or protected mutation until complete coded authority and bounded operation/resource admission accept the semantic unit; acquisition order and mechanism remain implementation choices. An otherwise valid complete claim that overlaps held coded authority SHALL remain pending through the existing wait, queue, backpressure, or admission behavior until the contention clears. Only actual bounded authority/resource exhaustion MAY return bounded refusal. Both outcomes occur before dependent I/O; incomplete mappings or stale topology/profile/generation are refused before dependent I/O.

Mutations and relevant mutation-basis reads SHALL use the same coded-range conflict meaning. A relevant basis observation SHALL be correlated to its consuming semantic mutation unit and complete coded claim, remain coherent through a consuming mutation, or be discarded/reconciled before release. Independent overlapping observations coordinate under the same claims; no release-then-use gap or unmodeled claim upgrade is accepted. Basis-read lifecycle and coherence evidence remain supplied by existing transaction, store, recovery, and healthy-service owners.

The coded owner SHALL consume, not manufacture, captured topology/profile and mapping validity, bounded operation/resource admission, recovery and store observations, authoritative reopen/reconciliation, and exact external `ReleaseAllowed(operation-generation)` from the canonical healthy-portable-io lifecycle-release owner. The coded owner SHALL NOT inspect or reconstruct the contributing lifecycle predicates, decide terminality, duplicate release policy, redefine `RecoveryProtocol.releaseRange`, decide persistence admissibility, or create product claims. CLEAN-capture uncertainty neither supplies nor negates `ReleaseAllowed`; independently releasable operation state may release while dirty-integrity retains bounded capture evidence.

Request, coded parity/codeword, exact store, dirty-region, and checksum-extents geometries SHALL remain separately observable. The coded claim SHALL come from the captured topology/profile mapping and SHALL NOT be substituted with another geometry. The exact bounded coded relation is delegated to `models/quint/CodedRangeClean.qnt`, which receives externally supplied complete validated claims and owner-approved observations as inputs and is the sole authority for their overlap/disjoint classification, admission-before-dependent-effect, and coded removal conditional on exact `ReleaseAllowed`; external authority, topology mapping, geometry construction, lifecycle terminality, basis coherence, persistence, resources, sessions, and product claims remain outside that delegation.

#### Scenario: Complete coded claims are classified

- **WHEN** complete valid claims under one captured topology and profile intersect or remain disjoint
- **THEN** the coded owner classifies intersection as a coded conflict; disjoint claims are not a coded conflict and may coexist subject to other owners, with any bounded refusal returned before dependent I/O and other owners retaining their obligations

#### Scenario: Admission scope is incomplete or partially acquired

- **WHEN** a complete mapped coded claim is missing, stale, or only partial physical authority has been acquired
- **THEN** admission is refused before dependent basis I/O or mutation, and no later claim upgrade is inferred

#### Scenario: Basis and release facts remain externally owned

- **WHEN** a relevant basis observation needs authority through a consuming mutation, or the canonical lifecycle-release owner supplies exact `ReleaseAllowed` while a separate `CLEAN` capture remains unresolved
- **THEN** the observation is held coherently or discarded/reconciled before release, and the coded claim may be removed only under that authorization while dirty-integrity retains bounded capture evidence and CLEAN uncertainty remains independent

