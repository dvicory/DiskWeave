## ADDED Requirements

### Requirement: Publication identity is derived from admitted semantics
<!-- dwv:req req.healthy-portable-io.publication-identity-is-derived-from-admitted-semantics -->
<!-- dwv:requires req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations -->

A service that passes portable admission SHALL expose a deterministic publication identity derived from its exact admitted recovery topology and the complete current identity-observation sets of every bound member in stable semantic assignment order. The identity SHALL change when any array identity, topology epoch, geometry, profile, slot, role, coding position, assignment instance or generation, store identity, assignment evidence summary, member observation source, member observation fingerprint, or member identity assessment changes. It SHALL NOT depend on fixture paths, collection order, SQL layout, runtime handles, or frontend-local rediscovery.

#### Scenario: The same admitted object is derived twice

- **WHEN** topology and all bound member identity observations are semantically identical
- **THEN** the service derives the same publication identity regardless of collection order

#### Scenario: A member observation changes

- **WHEN** one bound member retains its policy label and store identifier but its observed identity fingerprint changes
- **THEN** the publication identity changes and a frontend cannot report the old admitted object as the new one
