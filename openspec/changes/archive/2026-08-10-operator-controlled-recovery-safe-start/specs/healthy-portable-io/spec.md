## MODIFIED Requirements

### Requirement: Assembly and request admission are bounded and identity-safe
<!-- dwv:req req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.checksum-plane.current-baseline-completion-is-persisted-and-exact -->

The service SHALL assemble exactly one compatible member binding for every required assignment in a validated captured topology and admit each canonical request through a generational operation slot. Local collection, discovery, or vector order SHALL NOT define slot, role, coding position, assignment, or target identity. Before child I/O, the service SHALL resolve the request's stable target slot through its captured topology and reject missing, extra, duplicate, aliased, role/position-mismatched, assignment-mismatched, or stale bindings and requests. When current recovery state carries a mandatory new-checksum-baseline obligation, read/write assembly and request admission SHALL remain blocked until the checksum owner reconstructs complete current persisted coverage. Request validation, admission ownership, child execution, terminal reconciliation, and returned operation evidence SHALL preserve the same canonical request fields without a parallel service request model.

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

- **WHEN** member collection order and topology assignment order differ while every binding retains the same stable slot, role, coding position, assignment instance, and generation
- **THEN** the same request slot resolves to the same assignment and member

#### Scenario: A positional binding disagrees with topology

- **WHEN** collection position would select a different member than the request slot's captured assignment
- **THEN** assembly or request admission fails before resource reservation, child I/O, or member mutation

#### Scenario: A canonical request completes through the service

- **WHEN** a validated request is admitted, executed, and reconciled
- **THEN** operation evidence identifies the same frontend, request, target slot, topology epoch, operation, range, buffer token, ordering, and durability fields
