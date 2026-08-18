## MODIFIED Requirements

### Requirement: Transactions emit normalized semantic actions
<!-- dwv:req req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions -->

The reference machine SHALL expose the semantic action vocabulary for range acquisition, durable dirty/integrity invalidation intent, reads, parity computation, writes, flush/fence, checkpoint/clear, and range release. Actions SHALL contain semantic ranges, generations, identities, and evidence rather than backend child-operation or runtime types. Range release SHALL remain an explicit semantic action after a terminal or aborted outcome; ownership SHALL NOT be inferred to be free merely because a terminal result was produced. Exact action ordering, transition guards, transaction states, outcomes, and release sequencing are defined only by the delegated canonical `RecoveryProtocol` module.

#### Scenario: A write transaction starts

- **WHEN** a caller starts a protected write with affected regions and integrity extents
- **THEN** the normalized trace represents the request with the semantic action vocabulary and supplied semantic evidence without exposing backend runtime types

#### Scenario: Backend fanout is used

- **WHEN** one semantic action is implemented by several child operations
- **THEN** the reference trace records one semantic action and leaves child completion accounting to the operation-slot layer

#### Scenario: A terminal result is produced

- **WHEN** a transaction reaches a terminal or pre-mutation-aborted result
- **THEN** the normalized trace still exposes the delegated range-release action before the semantic range can be reused

### Requirement: Reference traces are deterministic and implementation-independent
<!-- dwv:req req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->

The reference machine SHALL emit versioned normalized action traces with stable semantic result representation and implementation-independent pre/post observations. The exact parameterized state, action, transition, outcome, ordering, release, invalid-transition, repeated-result, and invariant relation SHALL be delegated to the canonical `RecoveryProtocol` module in `models/quint/RecoveryProtocol.qnt`; that module is the sole authority for those exact protocol semantics within its declared surface. The delegated relation SHALL keep a terminal or aborted obligation owned until its explicit release action, SHALL expose no transition for an action or result that is not enabled by the current state, and SHALL keep unresolved intent or home effects non-clean until an explicit reconciliation outcome is admitted. Abandonment after a volatile home effect SHALL preserve incomplete mutation coverage; a durable home-effect reconciliation SHALL be admitted only when the represented mutation coverage supports that outcome. The separate `RecoveryProtocolAnalysis` module in `verification/quint/RecoveryProtocolAnalysis.qnt` binds the finite VE-002 evidence instance to one represented obligation, `Regions = {"data", "parity"}`, `Stores = {"data", "parity"}`, and `MaxDepth = 8`; those values bound verification evidence only and do not limit the product protocol or establish exhaustive reachability. Exact region mapping, checksum extent semantics, topology identity, typed fence admissibility, store persistence, adapter commit observations, operation-slot lifetime, and production recovery authority remain owned by their existing requirements.

#### Scenario: The same plan is replayed

- **WHEN** the same admitted semantic plan and owner observations are replayed through the reference boundary
- **THEN** the normalized trace and stable semantic result representation are identical; the exact state transition remains defined by the canonical `RecoveryProtocol` module

#### Scenario: A terminal obligation is started again before release

- **WHEN** a begin action is attempted while the prior obligation is terminal or pre-mutation-aborted but its range has not been released
- **THEN** the delegated relation exposes no begin transition and preserves the owned terminal state

#### Scenario: A released range starts another obligation

- **WHEN** the prior terminal or pre-mutation-aborted obligation has completed its explicit release action
- **THEN** a subsequent begin transition may acquire the released range and clears the prior release marker

#### Scenario: A result is repeated or out of order

- **WHEN** an action result does not match the currently pending semantic action or repeats an already consumed result
- **THEN** the relation exposes no state transition and preserves the prior semantic state and ownership

#### Scenario: Volatile home work is abandoned

- **WHEN** frontend delivery is abandoned after a volatile home effect but before all affected regions are represented as mutated
- **THEN** the relation preserves an unknown or indeterminate home outcome, retains the obligation for reconciliation, and does not admit a durable-home checkpoint until mutation coverage is complete

#### Scenario: Uncertainty is reconciled

- **WHEN** an unknown intent or home effect receives an explicit rejected, indeterminate, or durable reconciliation observation
- **THEN** the relation reaches only the corresponding conservative state; clean and terminal outcomes remain unavailable unless their delegated evidence predicates hold

#### Scenario: An invalid transition is attempted

- **WHEN** a supplied action or owner result violates a non-delegated admissibility requirement
- **THEN** the implementation reports the owning semantic failure and does not infer delegated state behavior from private implementation details

#### Scenario: Equivalent implementations are compared

- **WHEN** implementations receive the same admitted semantic plan and owner observations within the delegated protocol surface
- **THEN** comparison uses the versioned normalized trace and stable semantic result representation rather than private enum layout, batching, runtime, or database identity

#### Scenario: A non-delegated authority decision is evaluated

- **WHEN** an implementation must decide exact typed evidence, topology, persistence, or resource-lifetime admissibility
- **THEN** it consults the owning current requirement and does not infer that decision from the Quint model

#### Scenario: A finite evidence bound is interpreted

- **WHEN** VE-002 evidence reports the two-region, two-store, `MaxDepth = 8` analysis instance
- **THEN** the report treats those values as reproducible verification scope rather than a product cardinality limit or an unbounded protocol proof
