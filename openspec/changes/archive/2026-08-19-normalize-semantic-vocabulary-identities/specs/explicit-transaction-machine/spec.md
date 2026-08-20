## MODIFIED Requirements

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
