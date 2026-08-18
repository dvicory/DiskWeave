# explicit-transaction-machine Specification

## Purpose
This capability provides an auditable reference state machine that orders protected home-media actions around durable recovery intent, fences, checkpoints, abandonment, and crash reconciliation.
## Requirements
### Requirement: Transactions emit normalized semantic actions
<!-- dwv:req req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions -->

The reference machine SHALL expose the semantic action vocabulary for range acquisition, durable dirty/integrity invalidation intent, reads, parity computation, writes, flush/fence, checkpoint/clear, and range release. Actions SHALL contain semantic ranges, generations, identities, and evidence rather than backend child-operation or runtime types. Exact action ordering, transition guards, transaction states, outcomes, and release sequencing are defined only by the delegated canonical `RecoveryProtocol` module.

#### Scenario: A write transaction starts

- **WHEN** a caller starts a protected write with affected regions and integrity extents
- **THEN** the normalized trace represents the request with the semantic action vocabulary and supplied semantic evidence without exposing backend runtime types

#### Scenario: Backend fanout is used

- **WHEN** one semantic action is implemented by several child operations
- **THEN** the reference trace records one semantic action and leaves child completion accounting to the operation-slot layer

### Requirement: Reference traces are deterministic and implementation-independent
<!-- dwv:req req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->

The reference machine SHALL emit versioned normalized action traces with stable semantic result representation and implementation-independent pre/post observations. The exact parameterized state, action, transition, outcome, ordering, release, and invariant relation SHALL be delegated to the canonical `RecoveryProtocol` module in `models/quint/RecoveryProtocol.qnt`; that module is the sole authority for those exact protocol semantics within its declared surface. The separate `RecoveryProtocolAnalysis` module in `verification/quint/RecoveryProtocolAnalysis.qnt` binds the finite VE-002 evidence instance to one represented obligation, `Regions = {"data", "parity"}`, `Stores = {"data", "parity"}`, and `MaxDepth = 8`; those values bound verification evidence only and do not limit the product protocol or establish exhaustive reachability. Exact region mapping, checksum extent semantics, topology identity, typed fence admissibility, store persistence, adapter commit observations, operation-slot lifetime, and production recovery authority remain owned by their existing requirements.

#### Scenario: The same plan is replayed

- **WHEN** the same admitted semantic plan and owner observations are replayed through the reference boundary
- **THEN** the normalized trace and stable semantic result representation are identical; the exact state transition remains defined by the canonical `RecoveryProtocol` module

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

