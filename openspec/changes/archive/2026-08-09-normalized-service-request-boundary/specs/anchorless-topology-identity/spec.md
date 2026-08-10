## MODIFIED Requirements

### Requirement: Topology identities are explicit and immutable within an epoch
<!-- dwv:req req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch -->

The system SHALL represent array identity, logical slot identity, member role, coding position, assignment-instance identity, assignment generation, protected geometry, and topology epoch as separate semantic values. A published topology snapshot SHALL be immutable for the lifetime of requests that captured its epoch. Every opened member used by a service SHALL be bound explicitly to one exact snapshot assignment by stable slot identity, assignment-instance identity, assignment generation, and store identity; collection or discovery position SHALL carry no semantic meaning.

#### Scenario: A request captures a topology

- **WHEN** a request is admitted against a published topology
- **THEN** it retains the exact slot, role, coding position, protected length, assignment generation, and topology epoch it observed

#### Scenario: A topology is changed

- **WHEN** a new assignment is prepared or published
- **THEN** a new topology epoch is created and existing requests continue to resolve only against their captured snapshot

#### Scenario: Member collection order changes

- **WHEN** the same opened members are supplied in a different collection order with unchanged stable binding identities
- **THEN** every request resolves to the same snapshot assignment and physical store


### Requirement: Topology validation rejects ambiguous or inconsistent assignments
<!-- dwv:req req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->

The system SHALL reject snapshots with duplicate logical slots, duplicate active coding positions, mismatched array/epoch identities, invalid protected lengths, or role/coding assignments that cannot be interpreted by the selected parity profile. Service assembly SHALL reject missing, extra, duplicate, aliased, stale-generation, or assignment-mismatched member bindings. Validation SHALL not use physical enumeration or collection order as a substitute for a logical slot or coding position.

#### Scenario: Two candidates claim one slot

- **WHEN** a topology candidate contains two active assignments for the same logical slot
- **THEN** validation fails without publishing the candidate

#### Scenario: Coding positions are reordered by discovery order

- **WHEN** devices are discovered in a different physical order but carry the same recorded assignments
- **THEN** the same logical topology is produced and coding positions remain unchanged

#### Scenario: An opened binding disagrees with its assignment

- **WHEN** a member names a valid slot but carries a different assignment instance or generation
- **THEN** assembly fails before admission or member mutation
