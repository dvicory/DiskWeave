# healthy-portable-io Specification

## Purpose
This capability provides the first end-to-end portable single-parity path: normalized requests are admitted through stable topology and operation slots, executed against ordinary file-backed members, and completed only with the evidence required by the canonical dirty-region, recovery, transaction, XOR, and store contracts.
## Requirements
### Requirement: Assembly and request admission are bounded and identity-safe
<!-- dwv:req req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.checksum-plane.current-baseline-completion-is-persisted-and-exact -->

The service SHALL assemble exactly one compatible member binding for every required assignment in a validated captured topology and admit each canonical request through a generational operation slot. Local collection, discovery, or vector order SHALL NOT define slot, role, coding position, assignment, or target identity. Before child I/O, the service SHALL resolve the request's stable target slot through its captured topology and reject missing, extra, duplicate, aliased, role/position-mismatched, assignment-mismatched, or stale bindings and requests. Each opened store SHALL be selected by the topology assignment's stable store identity, and the service SHALL reject any binding whose store identity, slot, role, coding position, assignment instance, or assignment generation differs. When current recovery state carries a mandatory new-checksum-baseline obligation, read/write assembly and request admission SHALL remain blocked until the checksum owner reconstructs complete current persisted coverage. Request validation, admission ownership, child execution, terminal reconciliation, and returned operation evidence SHALL preserve the same canonical request fields without a parallel service request model.

#### Scenario: Healthy topology is assembled

- **WHEN** all required data/parity roles have unambiguous identity and compatible capabilities and no current admission prerequisite is outstanding
- **THEN** the service enters serving state and accepts normalized requests with a captured topology epoch

#### Scenario: Mandatory post-recovery baseline is incomplete

- **WHEN** current recovery state requires a new checksum baseline and current persisted coverage is absent, partial, invalid, or unsupported
- **THEN** read/write assembly and request admission fail before member mutation

#### Scenario: Mandatory post-recovery baseline is complete

- **WHEN** current recovery state requires a new checksum baseline and the checksum owner reconstructs complete correctly bound current persisted coverage
- **THEN** that baseline prerequisite is satisfied while every other current assembly and admission prerequisite still applies

#### Scenario: Unrelated healthy array has no mandatory baseline

- **WHEN** an otherwise eligible array carries no current new-baseline obligation
- **THEN** the new baseline barrier does not narrow its existing assembly or request admission behavior

#### Scenario: Ambiguous or stale assembly is attempted

- **WHEN** identity evidence is ambiguous, a required role is unavailable, or a captured generation is stale
- **THEN** assembly or request admission fails closed without mutating a member

#### Scenario: Collection order differs from topology order

- **WHEN** member collection order and topology assignment order differ while every binding retains the same stable slot, role, coding position, assignment instance, generation, and store identity
- **THEN** the same request slot resolves to the same assignment and member

#### Scenario: A positional binding disagrees with topology

- **WHEN** collection position would select a different member than the topology assignment's stable store identity
- **THEN** assembly or request admission fails before resource reservation, child I/O, or member mutation

#### Scenario: A canonical request completes through the service

- **WHEN** a validated request is admitted, executed, and reconciled
- **THEN** operation evidence identifies the same frontend, request, target slot, topology epoch, operation, range, buffer token, ordering, and durability fields

### Requirement: Publication identity is derived from admitted semantics
<!-- dwv:req req.healthy-portable-io.publication-identity-is-derived-from-admitted-semantics -->
<!-- dwv:requires req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations -->

A service that passes portable admission SHALL expose a deterministic publication identity derived from its exact admitted recovery topology and the complete current identity-observation sets of every bound member in stable semantic assignment order. Each member identity assessment used by publication SHALL be the assessment of that member's complete current observation set, not a synthetic assessment of one selected observation. The identity SHALL change when any array identity, topology epoch, geometry, profile, slot, role, coding position, assignment instance or generation, store identity, assignment evidence summary, member observation source, member observation fingerprint, or member identity assessment changes. It SHALL NOT depend on fixture paths, collection order, SQL layout, runtime handles, or frontend-local rediscovery.

#### Scenario: The same admitted object is derived twice

- **WHEN** topology and all bound member identity observations are semantically identical
- **THEN** the service derives the same publication identity regardless of collection order

#### Scenario: A member observation changes

- **WHEN** one bound member retains its policy label and store identifier but its observed identity fingerprint changes
- **THEN** the publication identity changes and a frontend cannot report the old admitted object as the new one

#### Scenario: One observation in a multi-observation member conflicts

- **WHEN** a bound member has multiple current observations and any observation makes the complete assessment ambiguous, conflicting, or changed
- **THEN** publication identity includes that conservative complete assessment and cannot claim the member merely matches

### Requirement: Healthy reads preserve exact-range evidence
<!-- dwv:req req.healthy-portable-io.healthy-reads-preserve-exact-range-evidence -->

A read SHALL validate the normalized byte range, split it at required boundaries, read the selected data member through the store contract, and return exact completed-range/disposition/persistence evidence. It SHALL not fabricate bytes for short or uncertain reads.

#### Scenario: Aligned healthy read

- **WHEN** a request reads an available data range and the store returns complete bytes
- **THEN** the service returns the exact requested bytes and structured successful completion

#### Scenario: Short or uncertain read

- **WHEN** a child read completes short or with unknown media effect
- **THEN** the service returns structured partial/uncertain evidence and does not report a full successful read

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

### Requirement: Portable members remain ordinary and control state is disposable
<!-- dwv:req req.healthy-portable-io.portable-members-remain-ordinary-and-control-state-is-disposable -->

Data and parity member files SHALL contain no required DiskWeave metadata. Loss of disposable control state SHALL not make intact payloads unreadable or authorize unsafe writes; recovery state and topology evidence remain the authority for protected mutation.

#### Scenario: Control database is removed

- **WHEN** the management/control database is deleted while data and recovery stores remain
- **THEN** management state can be rebuilt without changing payload bytes or silently authorizing a write

#### Scenario: Backing and export paths alias

- **WHEN** a proposed backing path is also an active exported endpoint
- **THEN** the service rejects the configuration before opening competing access
