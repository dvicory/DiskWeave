## MODIFIED Requirements

### Requirement: Portable semantics are independent of implementation mechanisms
<!-- dwv:req req.architecture-contract.portable-semantics-are-independent-of-implementation-mechanisms -->

Portable block, topology, parity, integrity, recovery, transaction, and evidence semantics SHALL NOT depend on a particular filesystem, kernel, database, asynchronous runtime, codec library, or frontend type. Implementations SHALL cross those boundaries through explicit adapters that preserve observable ranges, identities, ordering, ownership, durability intent, and terminal outcomes.

#### Scenario: An implementation mechanism is replaced

- **WHEN** a storage backend, persistence engine, runtime, codec, or frontend is replaced
- **THEN** the portable semantic contract and its allowed outcomes remain comparable without importing the mechanism's types into the semantic core

#### Scenario: An adapter cannot preserve intent

- **WHEN** an adapter lacks evidence to implement a requested range, ordering, durability, or lifecycle intent
- **THEN** it rejects or reports the weaker result explicitly instead of silently changing the semantic request
