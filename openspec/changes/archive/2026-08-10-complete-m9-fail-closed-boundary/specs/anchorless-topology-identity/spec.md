## MODIFIED Requirements

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
