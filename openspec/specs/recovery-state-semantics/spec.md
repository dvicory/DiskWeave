# recovery-state-semantics Specification

## Purpose
Recovery state is a durable protocol authority, not a mirror of home-media bytes. It records the evidence and generations needed to decide whether future writes, clean transitions, and integrity claims are allowed. SQLite may implement this interface later, but SQL tables and row IDs SHALL not define the portable semantics.
## Requirements
### Requirement: Recovery transactions are generation-checked and atomic

Each transaction SHALL capture an expected recovery generation and topology epoch. Commit SHALL apply all valid mutations together or none at all. A generation mismatch, invalid topology epoch, or failed mutation SHALL leave the durable snapshot unchanged.

#### Scenario: A transaction commits at the expected generation

- **WHEN** the expected recovery generation and topology epoch match the durable snapshot and all mutations are valid
- **THEN** the store commits the complete mutation batch and returns the next recovery generation

#### Scenario: A stale transaction commits

- **WHEN** another transaction has advanced the recovery generation before commit
- **THEN** the stale transaction is rejected with no durable side effect

### Requirement: Home mutation requires durable dirty and integrity invalidation intent

The semantic store SHALL support marking affected regions dirty and affected valid integrity extents stale before a caller records home-media mutation. Dirty and stale generations SHALL survive later in-memory process loss until a subsequent explicit recovery transaction changes them.

#### Scenario: A valid integrity extent is touched

- **WHEN** a transaction marks a region dirty and invalidates a valid extent
- **THEN** the resulting snapshot records both changes at one committed recovery generation and no valid digest remains authoritative for that extent

#### Scenario: Recovery intent commit fails

- **WHEN** a transaction containing dirty or stale mutations is rejected
- **THEN** the snapshot remains unchanged and a caller cannot treat the rejected intent as permission for home mutation

### Requirement: Clean and valid claims require typed fence evidence

The store SHALL record typed store-fence evidence with store identity, topology epoch, watermark, and capability evidence. A region SHALL not become clean, a writable session SHALL not close cleanly, and an integrity record SHALL not become valid unless the required fence/checkpoint evidence is present and matches the captured topology.

#### Scenario: A region is cleared after a durable fence

- **WHEN** a dirty region has covering fence evidence and the transaction records a clean checkpoint
- **THEN** the region may become clean and the checkpoint generation is durably recorded

#### Scenario: A volatile completion is supplied as a fence

- **WHEN** a caller attempts to clear a region using volatile or unknown persistence evidence
- **THEN** the mutation is rejected and the region remains dirty or indeterminate

### Requirement: Topology snapshots are immutable within a transaction

Transactions SHALL capture one topology epoch. A commit under a different epoch SHALL fail. Preparing a new topology SHALL create an explicit pending snapshot; publication requires a separate commit mutation and SHALL not rewrite the epoch of an existing transaction.

#### Scenario: Topology changes during an operation

- **WHEN** a transaction captured epoch E and the store now requires epoch E+1
- **THEN** the transaction is rejected or invalidated and no mutation is applied under the old snapshot

### Requirement: Semantic export and health are independent of storage engine layout

The reference store SHALL expose a bounded semantic snapshot/manifest and health classification. Missing, corrupt, or stale recovery state SHALL be observable and SHALL block new home mutations until an explicit recovery plan establishes a new generation. Export SHALL not expose SQLite pages, row IDs, or implementation pointers.

#### Scenario: Recovery state is missing

- **WHEN** an implementation reports no durable recovery snapshot
- **THEN** health is not healthy and callers receive a conservative recovery decision rather than an implicit clean state

### Requirement: SQLite remains an evidence-driven adapter decision

The project SHALL keep SQLite, journal mode, synchronization, checkpoint policy, connection topology, schema, and migration details behind the semantic interface. OS-005 SHALL record evaluation cases and reject selecting a mode from folklore or a successful process-local commit alone.

#### Scenario: Candidate SQLite modes are compared

- **WHEN** candidate journal/synchronization configurations are run through simulator crash and reset cases
- **THEN** the selected mode is recorded with its evidence, and unsupported or untested durability claims remain unavailable

### Requirement: Semantic schema, migrations, and exports are versioned independently of SQLite

The portable recovery boundary SHALL expose a versioned semantic schema descriptor, an explicit migration plan, and a bounded export manifest. These representations SHALL contain recovery concepts, generations, topology, evidence, and state, but SHALL NOT expose SQL table names, row IDs, pages, journal files, connection handles, or crate-specific database types.

#### Scenario: A semantic manifest is exported

- **WHEN** the requested current generation is healthy and within export limits
- **THEN** the export includes its semantic schema version and recovery snapshot and contains no storage-engine layout

#### Scenario: An unsupported migration is requested

- **WHEN** an adapter requests a migration from an unknown or incompatible semantic version
- **THEN** the plan is rejected without changing the recovery snapshot

### Requirement: Evaluation fixtures cover candidate durability and reset boundaries

OS-005 SHALL provide deterministic fixtures covering candidate journal modes, synchronization modes, checkpoint policies, process reset, VM reset, power loss, commit rejection, lost commit acknowledgement, missing state, main-state corruption, and journal-state corruption. Each fixture SHALL state the conservative semantic disposition and evidence still required; fixture presence SHALL NOT select a production SQLite mode.

#### Scenario: A dirty-intent commit is rejected or uncertain

- **WHEN** the simulator reports that the recovery commit did not become durably known
- **THEN** the expected disposition forbids the protected home-media mutation and requires reconciliation

#### Scenario: Recovery state is missing or corrupt after reset

- **WHEN** the recovery adapter cannot validate its state or journal
- **THEN** the expected disposition blocks writable assembly and permits only an explicit recovery/rebuild plan

### Requirement: Recovery adapters report conservative commit observations

The replaceable adapter seam SHALL distinguish durable, rejected, lost, and corrupt commit observations. The semantic store SHALL accept a transaction as a protocol fact only when the adapter reports durable commitment; uncertain or failed observations SHALL not be converted into a clean checkpoint, valid digest, or writable authorization.

#### Scenario: A checkpoint acknowledgement is lost

- **WHEN** a backend may have committed a checkpoint but cannot prove the resulting generation
- **THEN** the semantic disposition is reconciliation-required and no clean claim is inferred

### Requirement: The SQLite prototype remains evaluation-only and storage-independent at the semantic boundary

The project SHALL provide an evaluation-only SQLite prototype in a separate adapter package. The prototype SHALL apply the checked-in candidate migration, exercise candidate journal/synchronization/checkpoint settings, persist a bounded semantic recovery header, export it, run an integrity check, and make missing or corrupt state observable. It SHALL NOT expose SQLite handles, SQL row identity, or SQLite layout types through `dwv-recovery`, and it SHALL NOT claim production durability.

#### Scenario: A candidate migration and semantic header are evaluated

- **WHEN** the prototype initializes a temporary database, applies a candidate configuration, and writes a current recovery manifest
- **THEN** integrity succeeds and export returns the semantic schema, generation, topology epoch, and bounded semantic payload without exposing SQL layout

#### Scenario: The prototype state is missing or corrupt

- **WHEN** the recovery row is deleted or a persisted generation is corrupted
- **THEN** export reports missing/invalid state and never turns the condition into a clean recovery decision

#### Scenario: Recovery state is deleted independently of direct data

- **WHEN** the prototype database is removed while a separate direct-data fixture remains
- **THEN** the direct-data bytes remain readable and the recovery adapter reports missing state rather than claiming those bytes are clean or unreconciled

### Requirement: Offline rebuild progress is durable semantic authority

Recovery state SHALL represent an offline rebuild with a stable rebuild identifier, source array/topology/recovery generation, missing stable slot and coding position, replacement assignment instance and store identity, protected geometry, first-unprocessed-byte cursor, and lifecycle state. Cursor advancement and verified completion SHALL be generation-checked recovery mutations, bounded in semantic export, and independent of SQL rows, file paths, runtime handles, or executor objects.

#### Scenario: A rebuild cursor advances

- **WHEN** the replacement bytes through offset N have been read back, equation-verified, and durably fenced under the captured source topology
- **THEN** one atomic recovery transaction advances the first-unprocessed-byte cursor to N and records the matching fence evidence

#### Scenario: A stale rebuild transaction commits

- **WHEN** another recovery transaction or topology change invalidates the generation captured by a rebuild checkpoint
- **THEN** the checkpoint mutation fails atomically and cannot claim unverified replacement bytes durable

#### Scenario: A rebuild manifest is exported

- **WHEN** a current bounded semantic manifest is exported during an interrupted rebuild
- **THEN** it contains enough typed rebuild state to validate a later resume without exposing a path, SQLite layout, or process-local resource

