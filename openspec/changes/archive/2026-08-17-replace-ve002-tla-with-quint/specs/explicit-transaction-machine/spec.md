## MODIFIED Requirements

### Requirement: Durable intent precedes every protected home mutation
<!-- dwv:req req.explicit-transaction-machine.durable-intent-precedes-every-protected-home-mutation -->
<!-- dwv:refines req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation -->
<!-- dwv:refines req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->

Within the bounded reference-machine surface delegated to the canonical Quint model, the model SHALL define whether a protected home-mutation transition is enabled and SHALL expose no such transition before durable dirty and integrity invalidation intent is accepted. The dirty/integrity and recovery-state requirements SHALL remain the owners of exact affected-region and checksum-extents, generation, topology, typed-evidence, and commit-observation predicates. Outside the bounded model, this requirement SHALL preserve those owner decisions and SHALL transition to blocked or reconciliation-required without emitting protected mutation when an owner rejects, loses, corrupts, or cannot classify intent.

#### Scenario: Durable invalidation result is accepted

- **WHEN** the Quint analysis reaches a state with accepted durable intent and the remaining bounded preconditions hold
- **THEN** the model enables the bounded home-mutation transition and the reference machine may emit its local read, compute, and write actions only under the captured owner evidence

#### Scenario: Durable invalidation result is unavailable

- **WHEN** the intent is pending, rejected, uncertain, stale, or otherwise not an accepted durable fact
- **THEN** the model has no protected home-mutation successor and the reference machine records a conservative blocked or reconciliation-required outcome

### Requirement: Clean and checkpoint claims require fence evidence
<!-- dwv:req req.explicit-transaction-machine.clean-and-checkpoint-claims-require-fence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence -->
<!-- dwv:refines req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence -->
<!-- dwv:refines req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->

Within the bounded reference-machine surface delegated to the canonical Quint model, the model SHALL define the ordering and admissibility boundary for durable home effects, store fences, checkpoint, and terminal ownership. The store, recovery-state, and dirty/integrity requirements SHALL remain the owners of child terminality, exact watermarks, typed authority, region selection, generations, and clean-claim admissibility. The reference machine SHALL emit checkpoint, clear, and range-release actions only when those owners accept current covering evidence.

#### Scenario: Recovery authority accepts the completed write set

- **WHEN** all bounded home effects are durable, every bounded store fence is present, and the recovery disposition is dirty
- **THEN** the model enables checkpoint, records the bounded evidence coverage, and enters terminal ownership only through that transition

#### Scenario: Recovery authority rejects the evidence

- **WHEN** a required bounded fence is absent or an owning store, recovery, or dirty protocol rejects the evidence
- **THEN** the model does not enable checkpoint or terminal release and the reference machine remains conservative or reconciliation-required

### Requirement: Failure, abandonment, and crash states are conservative
<!-- dwv:req req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative -->
<!-- dwv:requires req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->
<!-- dwv:refines req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->

Within the bounded reference-machine surface delegated to the canonical Quint model, the model SHALL distinguish pre-intent abandonment, post-intent ownership, uncertain home effects, process loss, recovery handoff, and terminal ownership without silently converting uncertainty to clean state or rollback. Frontend delivery interest, backend lifetime, resource reclamation, and durable dirty/restart consequences SHALL remain owned by their respective requirements. The reference machine SHALL preserve action order and transaction state until those owners record terminal reconciliation.

#### Scenario: Delivery interest is abandoned after intent

- **WHEN** the bounded obligation is abandoned while intent is still pending and no home effect occurred
- **THEN** the model may return to the clean unowned state without fabricating a protected mutation or terminal home evidence

#### Scenario: Process loss follows a home write

- **WHEN** process state is lost after a volatile or durable home effect and before checkpoint
- **THEN** the model records an unknown home effect, an indeterminate recovery disposition, and a recovery handoff, and it does not enable clean terminalization until explicit reconciliation

### Requirement: Reference traces are deterministic and implementation-independent
<!-- dwv:req req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->

The reference machine SHALL emit versioned normalized action traces with stable error classes and semantic pre/post states. The exact bounded transition relation for one admitted write obligation, two affected regions, two stores, explicit intent uncertainty, home-effect uncertainty, crash/loss, abandonment, reconciliation, fences, checkpoint, and terminal ownership SHALL be delegated to the canonical Quint model at `verification/quint/RecoveryProtocol.qnt`. The Quint model SHALL be authoritative only for that named bounded state, action, and invariant surface. Exact region mapping, checksum extent semantics, topology identity, typed fence admissibility, store persistence, adapter commit observations, operation-slot lifetime, and production recovery authority SHALL remain owned by their existing requirements. Equivalent implementations SHALL be compared by allowed trace normalization rather than private enum layout, batching, runtime, or database identity.

#### Scenario: The same plan is replayed

- **WHEN** the Quint analysis receives the same bounded action choices and result observations twice
- **THEN** it produces the same semantic trace and terminal state

#### Scenario: An invalid transition is attempted

- **WHEN** a caller supplies a protected mutation before durable intent, checkpoint before fence coverage, or clean terminalization after unresolved uncertainty
- **THEN** the model disables or rejects that transition and leaves the prior semantic state unchanged

#### Scenario: A non-delegated authority decision is evaluated

- **WHEN** an implementation must decide exact typed evidence, topology, persistence, or resource-lifetime admissibility
- **THEN** it consults the owning current requirement and does not infer that decision from the bounded Quint model
