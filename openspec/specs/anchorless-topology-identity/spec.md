# anchorless-topology-identity Specification

## Purpose
This capability gives DiskWeave stable logical topology and deterministic identity evidence without requiring DiskWeave metadata inside ordinary data-member payloads.
## Requirements
### Requirement: Topology identities are explicit and immutable within an epoch
<!-- dwv:req req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch -->

The system SHALL represent array identity, logical slot identity, member role, coding position, assignment-instance identity, assignment generation, protected geometry, and topology epoch as separate semantic values. A published topology snapshot SHALL be immutable for the lifetime of requests that captured its epoch. Every opened member used by a service SHALL be bound explicitly to one exact snapshot assignment by stable slot identity, assignment-instance identity, assignment generation, and store identity; collection or discovery position SHALL carry no semantic meaning. Topology-derived identities SHALL traverse assignments in stable semantic order rather than caller collection order.

#### Scenario: A request captures a topology

- **WHEN** a request is admitted against a published topology
- **THEN** it retains the exact slot, role, coding position, protected length, assignment generation, and topology epoch it observed

#### Scenario: A topology is changed

- **WHEN** a new assignment is prepared or published
- **THEN** a new topology epoch is created and existing requests continue to resolve only against their captured snapshot

#### Scenario: Member collection order changes

- **WHEN** the same opened members are supplied in a different collection order with unchanged stable binding identities
- **THEN** every request resolves to the same snapshot assignment and physical store and every topology-derived identity remains unchanged

### Requirement: Topology validation rejects ambiguous or inconsistent assignments
<!-- dwv:req req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->

The requirement retains ownership of topology and member-binding validity, its supplied-input assumptions, its product-facing rejection meaning, and its refusal boundaries. The exact finite pure validity relation over an already supplied, owner-accepted topology snapshot and member-binding set SHALL be delegated to the canonical `TopologyMemberBindingValidation` module in `models/quint/TopologyMemberBindingValidation.qnt`; within the declared surface, that module SHALL be the sole exact authority for the valid/invalid result.

The delegated domain SHALL contain only supplied semantic values: an owner-accepted selected profile and protected geometry; one snapshot array identity and topology epoch; snapshot assignments with logical slot, role, coding position, assignment-instance identity, and assignment generation; member bindings with the corresponding captured fields; opened-store identity, topology epoch, protected-length, and block-size facts; and one owner-qualified identity comparison for every pair of bound stores.

Identity observations MAY accompany production correspondence, but the relation SHALL consume only their owner-qualified pairwise comparison results and SHALL NOT decide observation provenance, confidence, assessment precedence, candidate choice, or operator attestation. It SHALL NOT own discovery; candidate enumeration; profile or protected-geometry construction, deserialization validation, or trust; identity evidence production or assessment; service-profile selection or authorization; implementation resource ceilings; writable-assembly or request-admission policy; capability support beyond equality to supplied geometry; topology preparation, transition, verification, commit, or publication; payload validity; parity computation; recovery; persistence; physical durability; or physical I/O. Snapshot construction and service assembly SHALL compose the delegated result with those non-delegated owners and SHALL preserve current acceptance and rejection outcomes within the mapped input domain.

The current obligations to reject mismatched array identities and invalid protected lengths remain non-delegated because the current pure boundary supplies no second array claim and unchecked deserialization can bypass the checked profile/geometry constructors. This change SHALL NOT invent the missing comparison source, treat transition-owned active/candidate array comparison as snapshot validation, silently trust unchecked deserialization, or change current admission outcomes. The requirement that a topology assignment authorize the selected stable store identity also remains non-delegated; declared/opened-store equality and uniqueness do not supply that authorization.

A rejected snapshot SHALL not be published. A rejected member-binding set SHALL fail before request admission or member mutation. Neither the relation nor a consumer SHALL use physical enumeration, vector position, discovery order, or collection order as a substitute for a logical slot, coding position, or stable identity.

The parameterized canonical module SHALL define semantics independently of any verification bound. Analysis, deterministic scenario, mutation, and Connect modules under `verification/quint/` SHALL be evidence only and SHALL NOT broaden the delegated domain or establish exhaustive behavior outside their stated finite scopes.

#### Scenario: Two candidates claim one slot

- **WHEN** a topology candidate contains two active assignments for the same logical slot
- **THEN** the delegated relation rejects the candidate and no consumer publishes it

#### Scenario: Coding positions are reordered by discovery order

- **WHEN** devices, snapshot assignments, or bindings are supplied in a different order with unchanged semantic values and owner-qualified identity comparisons
- **THEN** the delegated valid/invalid result is unchanged, the same logical topology is produced, and coding positions remain unchanged

#### Scenario: An opened binding disagrees with its assignment

- **WHEN** a member names a valid slot but carries a different role, coding position, assignment instance, assignment generation, or topology epoch
- **THEN** the delegated relation rejects the binding set and assembly fails before admission or member mutation

#### Scenario: An identity comparison is not distinct

- **WHEN** the identity-assessment owner supplies an aliased or ambiguous comparison for any two member bindings
- **THEN** the delegated relation rejects the binding set without choosing a candidate or changing the supplied assessment

#### Scenario: A finite analysis succeeds

- **WHEN** a bounded analysis instance checks the delegated relation without finding an invariant violation
- **THEN** the evidence reports only that exact finite scope and does not claim arbitrary-width topology coverage, identity-assessment correctness, Rust correctness, transition safety, or physical-I/O correctness

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

Every prepared topology transition SHALL have a bounded plan bound to the current array identity, source topology epoch, expected assignment-instance identities and generations, exact source and target protected geometry and coding profiles, required quiescence or explicit reconciliation mode, recovery and rollback boundaries, and verification evidence required before promotion. The system SHALL revalidate those bindings before changing protected data/parity state and before promotion. A stale binding or unsupported execution mode SHALL be refused without changing transition state, protected bytes, or the active topology.

#### Scenario: An assignment changes after planning

- **WHEN** an expected assignment instance or generation no longer matches when the transition is about to change protected data/parity state
- **THEN** the transition is refused without changing protected data/parity state and the active topology remains authoritative

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

