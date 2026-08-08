# anchorless-topology-identity Specification

## Purpose
This capability gives DiskWeave stable logical topology and deterministic identity evidence without requiring DiskWeave metadata inside ordinary data-member payloads.
## Requirements
### Requirement: Topology identities are explicit and immutable within an epoch

The system SHALL represent array identity, logical slot identity, member role, coding position, assignment-instance identity, assignment generation, protected geometry, and topology epoch as separate semantic values. A published topology snapshot SHALL be immutable for the lifetime of requests that captured its epoch.

#### Scenario: A request captures a topology

- **WHEN** a request is admitted against a published topology
- **THEN** it retains the exact slot, role, coding position, protected length, assignment generation, and topology epoch it observed

#### Scenario: A topology is changed

- **WHEN** a new assignment is prepared or published
- **THEN** a new topology epoch is created and existing requests continue to resolve only against their captured snapshot

### Requirement: Topology validation rejects ambiguous or inconsistent assignments

The system SHALL reject snapshots with duplicate logical slots, duplicate active coding positions, mismatched array/epoch identities, invalid protected lengths, or role/coding assignments that cannot be interpreted by the selected parity profile. Validation SHALL not use physical enumeration order as a substitute for a coding position.

#### Scenario: Two candidates claim one slot

- **WHEN** a topology candidate contains two active assignments for the same logical slot
- **THEN** validation fails without publishing the candidate

#### Scenario: Coding positions are reordered by discovery order

- **WHEN** devices are discovered in a different physical order but carry the same recorded assignments
- **THEN** the same logical topology is produced and coding positions remain unchanged

### Requirement: Identity evidence is assessed from multiple observations

The identity resolver SHALL accept observations with source, normalized fingerprint, provenance, stability classification, and confidence. It SHALL return deterministic decisions for confident match, changed-but-explainable, ambiguous clone, insufficient evidence, conflicting assignment, and new unassigned device. Path names and probe order SHALL never be sufficient stable identity.

#### Scenario: A cloned filesystem UUID is observed

- **WHEN** two candidates share a filesystem or partition identifier but differ in independent file/device identity evidence
- **THEN** the resolver reports clone ambiguity or conflict and does not authorize writable assignment

#### Scenario: A USB bridge omits serial and WWN

- **WHEN** stable hardware identifiers are unavailable but file identity, capacity, geometry, and an explicit operator attestation are consistent
- **THEN** the resolver reports the required deterministic confidence decision and records which evidence is missing or attested

### Requirement: Writable assembly fails closed on unresolved identity

Writable assembly SHALL require exactly one confident or explicitly attested candidate for every required slot, matching role/coding position and compatible geometry. Ambiguous, conflicting, missing, stale, or unexplained replacement evidence SHALL produce a blocked/read-only decision rather than an automatic choice.

#### Scenario: A replacement has changed capacity

- **WHEN** a candidate matches a slot identity but its capacity or protected geometry differs from the committed assignment
- **THEN** normal assembly is refused and a staged replacement/resize decision is required

#### Scenario: All required candidates match

- **WHEN** each required assignment has one confident candidate and no unaccounted clone conflict
- **THEN** the topology may be prepared for a staged transition but is not published until its durable commit step

### Requirement: Topology transitions are staged and recoverable

Adding, removing, replacing, resizing, or role-changing a member SHALL follow prepared, verified, committed, and published stages. The old active topology SHALL remain authoritative until the new generation is durably committed and verified. Failure at any stage SHALL leave a deterministic recovery decision and SHALL not release the old assignment prematurely.

#### Scenario: A prepared replacement fails verification

- **WHEN** rebuild or complete verification fails under a prepared topology
- **THEN** the old topology remains active and the replacement is not published or treated as writable

#### Scenario: Topology commit succeeds but publication is interrupted

- **WHEN** durable recovery state records the new generation but frontend publication does not complete
- **THEN** restart reconciles to the committed generation before publishing a new request-visible epoch

### Requirement: Identity and topology evidence is exportable for forensic recovery

The system SHALL expose bounded, versioned evidence reports that list candidates, observations, assessment, topology generation, and required operator action without exposing private runtime handles or requiring data-member metadata. A read-only report SHALL never itself authorize a destructive assignment.

#### Scenario: Metadata is missing

- **WHEN** recovery state is absent but candidates and evidence can be inspected
- **THEN** the tool emits a new-lineage or unresolved-topology report and preserves all candidates without silently selecting one

