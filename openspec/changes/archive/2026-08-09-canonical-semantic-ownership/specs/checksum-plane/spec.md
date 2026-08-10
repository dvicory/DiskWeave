## MODIFIED Requirements

### Requirement: Invalidation precedes protected mutation
<!-- dwv:req req.checksum-plane.invalidation-precedes-protected-mutation -->
<!-- dwv:refines req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation -->

For every protected mutation touching a `VALID` checksum extent, the checksum capability SHALL identify the affected extents and require their `VALID` to `STALE` transition through the owning dirty/integrity invalidation protocol. This requirement owns only the checksum-state transition and its independence from parity `CLEAN` or `DIRTY` state; it does not redefine the owner's durable-intent predicate.

#### Scenario: Valid extent is modified

- **WHEN** a protected write touches a valid data or parity extent
- **THEN** the checksum transition is included in durable invalidation before the corresponding home mutation

#### Scenario: Parity is clean with stale records

- **WHEN** parity is clean but checksum coverage is stale or absent
- **THEN** the system reports the two dimensions separately and never upgrades stale records from parity state alone
