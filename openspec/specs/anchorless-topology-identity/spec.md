# anchorless-topology-identity Specification

## Purpose
This capability gives DiskWeave stable logical topology and deterministic identity evidence without requiring DiskWeave metadata inside ordinary data-member payloads.
## Requirements
### Requirement: Topology identities are explicit and immutable within an epoch
<!-- dwv:req req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch -->

The system SHALL represent array identity, logical slot identity, member role, coding position, assignment-instance identity, assignment generation, protected geometry, and topology epoch as separate semantic values. A published topology snapshot SHALL be immutable for the lifetime of requests that captured its epoch.

#### Scenario: A request captures a topology

- **WHEN** a request is admitted against a published topology
- **THEN** it retains the exact slot, role, coding position, protected length, assignment generation, and topology epoch it observed

#### Scenario: A topology is changed

- **WHEN** a new assignment is prepared or published
- **THEN** a new topology epoch is created and existing requests continue to resolve only against their captured snapshot

### Requirement: Topology validation rejects ambiguous or inconsistent assignments
<!-- dwv:req req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->

The system SHALL reject snapshots with duplicate logical slots, duplicate active coding positions, mismatched array/epoch identities, invalid protected lengths, or role/coding assignments that cannot be interpreted by the selected parity profile. Validation SHALL not use physical enumeration order as a substitute for a coding position.

#### Scenario: Two candidates claim one slot

- **WHEN** a topology candidate contains two active assignments for the same logical slot
- **THEN** validation fails without publishing the candidate

#### Scenario: Coding positions are reordered by discovery order

- **WHEN** devices are discovered in a different physical order but carry the same recorded assignments
- **THEN** the same logical topology is produced and coding positions remain unchanged

### Requirement: Identity evidence is assessed from multiple observations
<!-- dwv:req req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations -->

The identity resolver SHALL accept observations with source, normalized fingerprint, provenance, stability classification, and confidence. It SHALL return deterministic decisions for confident match, changed-but-explainable, ambiguous clone, insufficient evidence, conflicting assignment, and new unassigned device. Path names and probe order SHALL never be sufficient stable identity.

#### Scenario: A cloned filesystem UUID is observed

- **WHEN** two candidates share a filesystem or partition identifier but differ in independent file/device identity evidence
- **THEN** the resolver reports clone ambiguity or conflict and does not authorize writable assignment

#### Scenario: A USB bridge omits serial and WWN

- **WHEN** stable hardware identifiers are unavailable but file identity, capacity, geometry, and an explicit operator attestation are consistent
- **THEN** the resolver reports the required deterministic confidence decision and records which evidence is missing or attested

### Requirement: Writable assembly fails closed on unresolved identity
<!-- dwv:req req.anchorless-topology-identity.writable-assembly-fails-closed-on-unresolved-identity -->

Writable assembly SHALL require exactly one confident or explicitly attested candidate for every required slot, matching role/coding position and compatible geometry. Ambiguous, conflicting, missing, stale, or unexplained replacement evidence SHALL produce a blocked/read-only decision rather than an automatic choice.

#### Scenario: A replacement has changed capacity

- **WHEN** a candidate matches a slot identity but its capacity or protected geometry differs from the committed assignment
- **THEN** normal assembly is refused and a staged replacement/resize decision is required

#### Scenario: All required candidates match

- **WHEN** each required assignment has one confident candidate and no unaccounted clone conflict
- **THEN** the topology may be prepared for a staged transition but is not published until its durable commit step

### Requirement: Topology transition plans bind source, target, and recovery
<!-- dwv:req req.anchorless-topology-identity.topology-transition-plans-bind-source-target-and-recovery -->

Every prepared topology transition SHALL have a bounded plan bound to the current array identity, source topology epoch, expected assignment-instance identities and generations, exact source and target protected geometry and coding profiles, required quiescence or explicit reconciliation mode, recovery and rollback boundaries, and verification evidence required before promotion. The system SHALL revalidate those bindings before protected mutation and before promotion. A stale binding or unsupported execution mode SHALL be refused without changing transition state, protected bytes, or the active topology.

#### Scenario: An assignment changes after planning

- **WHEN** an expected assignment instance or generation no longer matches when the transition is about to mutate protected bytes
- **THEN** the transition is refused without mutation and the active topology remains authoritative

#### Scenario: A requested transition mode is unsupported

- **WHEN** the system cannot execute or verify the plan's required quiescence or reconciliation mode
- **THEN** it refuses the transition without changing transition state, protected bytes, or the active topology

### Requirement: Topology transitions are staged and recoverable
<!-- dwv:req req.anchorless-topology-identity.topology-transitions-are-staged-and-recoverable -->

Adding, removing, replacing, resizing, or role-changing a member, or changing a coding profile, parity-role count, coding position, or protected geometry, SHALL follow prepared, verified, committed, and published stages. The old active topology SHALL remain authoritative until the new generation is durably committed and verified. Target state SHALL be established and independently verified without overwriting or releasing the only assignments or parity bytes required by the old topology. The system SHALL NOT reinterpret existing parity bytes under a different profile, coding position, or protected geometry. Failure at any stage SHALL leave a deterministic recovery decision and SHALL not release the old assignment prematurely.

#### Scenario: A prepared replacement fails verification

- **WHEN** rebuild or complete verification fails under a prepared topology
- **THEN** the old topology remains active and the replacement is not published or treated as writable

#### Scenario: A data slot is added

- **WHEN** a plan introduces an identified data assignment and establishes target parity under its declared quiescence or reconciliation mode
- **THEN** the new topology is not committed until the target protected range and parity have been independently verified

#### Scenario: A data slot is removed

- **WHEN** a plan removes a data slot
- **THEN** it requires quiescence, evidence from the external retention owner that all retained user data has been removed, and independently verified parity under the target topology before commit

#### Scenario: Protected capacity changes

- **WHEN** a plan shrinks or increases protected capacity
- **THEN** shrink requires proof that no protected bytes exist beyond the target length, while increase requires parity-capacity validation and a verified target baseline before commit

#### Scenario: A coding-profile migration is interrupted before commit

- **WHEN** target parity has been partially or completely generated under a proposed coding profile but verification or durable commit has not completed
- **THEN** the source topology and its required parity remain authoritative and the target profile is not published or used to interpret existing parity bytes

#### Scenario: Topology commit succeeds but publication is interrupted

- **WHEN** durable recovery state records the new generation but frontend publication does not complete
- **THEN** restart reconciles to the committed generation before publishing a new request-visible epoch

### Requirement: Identity and topology evidence is exportable for forensic recovery
<!-- dwv:req req.anchorless-topology-identity.identity-and-topology-evidence-is-exportable-for-forensic-recovery -->

The system SHALL expose bounded, versioned evidence reports that list candidates, observations, assessment, topology generation, and required operator action without exposing private runtime handles or requiring data-member metadata. A read-only report SHALL never itself authorize a destructive assignment.

#### Scenario: Metadata is missing

- **WHEN** recovery state is absent but candidates and evidence can be inspected
- **THEN** the tool emits a new-lineage or unresolved-topology report and preserves all candidates without silently selecting one

