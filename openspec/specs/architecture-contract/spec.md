# architecture-contract Specification

## Purpose

DiskWeave is a portable block-parity engine beneath conventional filesystems. This constitution owns only the product boundary and invariants that must remain true across storage backends, frontends, persistence engines, and recovery implementations. Capability specifications define the detailed protocols within those boundaries.

## Requirements

### Requirement: DiskWeave protects conventional member images at block level
<!-- dwv:req req.architecture-contract.diskweave-protects-conventional-member-images-at-block-level -->

DiskWeave SHALL provide parity protection for normalized block ranges beneath conventional filesystems. It SHALL NOT own a filesystem namespace, extent allocator, inode/object graph, snapshot or reflink model, per-file redundancy policy, or hidden namespace required to interpret a data member. A regular file placed through a namespace layer SHALL remain wholly on one conventional member.

#### Scenario: A data member is inspected independently
- **WHEN** a conventional data-member image is opened outside DiskWeave
- **THEN** its filesystem or block-image bytes remain usable without a DiskWeave namespace or proprietary data-member interpreter

#### Scenario: A proposed feature adds filesystem ownership
- **WHEN** a feature would split file extents, define inode or allocation state, or make a filesystem namespace authoritative for parity recovery
- **THEN** it is outside this product boundary and requires a separate architecture and recovery contract

### Requirement: Data payloads and protection metadata remain separate
<!-- dwv:req req.architecture-contract.data-payloads-and-protection-metadata-remain-separate -->

Required DiskWeave metadata SHALL NOT be embedded in ordinary data-member payloads. Parity payloads, recovery state, integrity records, and frontend exposure SHALL remain explicit separate layers. Parity consistency SHALL NOT be treated as proof of exact content or checksum validity.

#### Scenario: Parity metadata changes
- **WHEN** parity, recovery, integrity, or frontend metadata is migrated or lost
- **THEN** ordinary data-member bytes retain their independent interpretation, while the affected protection or authority state becomes explicitly unavailable or requires re-establishment

#### Scenario: Parity disagrees with present members
- **WHEN** a parity equation disagrees with data bytes
- **THEN** the system does not identify a corrupt target or authorize repair without independent current evidence

### Requirement: Portable semantics are independent of implementation mechanisms
<!-- dwv:req req.architecture-contract.portable-semantics-are-independent-of-implementation-mechanisms -->

Portable block, topology, parity, integrity, recovery, transaction, and evidence semantics SHALL NOT depend on a particular filesystem, kernel, database, asynchronous runtime, codec library, or frontend type. Implementations SHALL cross those boundaries through explicit adapters that preserve observable ranges, identities, ordering, ownership, durability intent, and terminal outcomes.

#### Scenario: An implementation mechanism is replaced
- **WHEN** a storage backend, persistence engine, runtime, codec, or frontend is replaced
- **THEN** the portable semantic contract and its allowed outcomes remain comparable without importing the mechanism's types into the semantic core

#### Scenario: An adapter cannot preserve intent
- **WHEN** an adapter lacks evidence to implement a requested range, ordering, durability, or lifecycle intent
- **THEN** it rejects or reports the weaker result explicitly instead of silently changing the semantic request

### Requirement: Identity and topology authority are explicit and conservative
<!-- dwv:req req.architecture-contract.identity-and-topology-authority-are-explicit-and-conservative -->

Logical slots, member roles, coding positions, assignment instances, protected geometry, and topology epochs SHALL be distinct semantic identities. A request SHALL retain the topology snapshot under which it was admitted. Writable assembly and topology publication SHALL fail closed when identity, geometry, role, generation, or assignment evidence is ambiguous, stale, conflicting, or incomplete.

#### Scenario: Two physical observations could satisfy one slot
- **WHEN** identity evidence cannot distinguish candidates or a role/coding assignment is inconsistent
- **THEN** writable assembly is refused or restricted to an explicitly read-only decision; the system does not choose by path or enumeration order

#### Scenario: A topology changes during an operation
- **WHEN** the current topology epoch or assignment generation differs from the captured request snapshot
- **THEN** the operation is rejected or reconciled under an explicit transition and does not reinterpret the request against the new topology

### Requirement: Durable authority and uncertainty are not inferred
<!-- dwv:req req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred -->

Acknowledgement, completion, persistence, durability, clean state, integrity validity, and recovery authorization SHALL remain distinct facts. Protected mutation SHALL require the applicable durable intent and current authority evidence. Failed, short, cancelled, abandoned, crashed, lost, stale, or uncertain effects SHALL remain visible and SHALL NOT be converted into clean state, valid integrity, writable authorization, or proof that an irreversible effect did not occur.

#### Scenario: A protected write is acknowledged before a fence
- **WHEN** a write completes from the caller's perspective but covering durable fence evidence is unavailable
- **THEN** the request may report completion only at the established scope and recovery state remains dirty, uncertain, or otherwise conservative

#### Scenario: A caller abandons an operation
- **WHEN** completion interest is dropped after submission
- **THEN** delivery may be suppressed, but media effects, resource ownership, and recovery reconciliation remain governed by the operation and are not rolled back by abandonment

### Requirement: Recovery and repair never promote algebraic possibility to authority
<!-- dwv:req req.architecture-contract.recovery-and-repair-never-promote-algebraic-possibility-to-authority -->

Read reconstruction, rebuild, repair, rebaseline, and format interpretation SHALL use explicit current identity, geometry, generation, integrity, and durability evidence. Mathematical computability or a successful process return alone SHALL NOT authorize serving data as healthy, publishing a replacement, clearing dirty state, accepting a checksum, or interpreting an unknown format as writable.

#### Scenario: A missing member is mathematically reconstructible
- **WHEN** surviving bytes permit a candidate reconstruction but required authority or integrity evidence is missing, stale, or conflicting
- **THEN** the system refuses or marks the result degraded/uncertain and performs no unauthorized protected mutation

#### Scenario: An unknown format is encountered
- **WHEN** a tool cannot establish the payload offsets, coding profile, topology, or recovery semantics of a format family
- **THEN** it refuses writable interpretation while allowing only bounded safe inspection or direct ordinary-payload access
