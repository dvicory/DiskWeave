# recovery-state-semantics Specification

## Purpose
Recovery state is a durable protocol authority, not a mirror of home-media bytes. It records the evidence and generations needed to decide whether future writes, clean transitions, and integrity claims are allowed. SQLite may implement this interface later, but SQL tables and row IDs SHALL not define the portable semantics.
## Requirements
### Requirement: Recovery transactions are generation-checked and atomic
<!-- dwv:req req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic -->

Each transaction SHALL capture an expected recovery generation and topology epoch. Commit SHALL apply all valid mutations together or none at all. A generation mismatch, invalid topology epoch, or failed mutation SHALL leave the durable snapshot unchanged.

#### Scenario: A transaction commits at the expected generation

- **WHEN** the expected recovery generation and topology epoch match the durable snapshot and all mutations are valid
- **THEN** the store commits the complete mutation batch and returns the next recovery generation

#### Scenario: A stale transaction commits

- **WHEN** another transaction has advanced the recovery generation before commit
- **THEN** the stale transaction is rejected with no durable side effect

### Requirement: Home mutation requires durable dirty and integrity invalidation intent
<!-- dwv:req req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent -->
<!-- dwv:refines req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation -->

The semantic store SHALL atomically persist the affected dirty regions and stale integrity extents at one generation before a caller may rely on that transaction as the owner's durable invalidation intent. Dirty and stale generations SHALL survive in-memory process loss until a later explicit recovery transaction changes them. A rejected or generation-mismatched transaction leaves the prior snapshot authoritative. A lost, corrupt, or indeterminate commit observation establishes no authoritative resulting snapshot for the caller, provides no permission for protected home mutation, and requires reconciliation through the recovery commit-observation contract.

#### Scenario: A valid integrity extent is touched

- **WHEN** one recovery transaction marks a region dirty and invalidates a valid extent
- **THEN** the resulting snapshot records both changes at one committed recovery generation and no valid digest remains authoritative for that extent

#### Scenario: Recovery intent commit is rejected before authoritative commit

- **WHEN** the transaction is rejected or generation-mismatched before authoritative commit
- **THEN** the proposed transaction does not become authoritative, the prior snapshot remains authoritative, and the caller receives no permission for home mutation

#### Scenario: Recovery intent commit observation is lost, corrupt, or indeterminate

- **WHEN** commitment may have occurred but its acknowledgement is lost, corrupt, or indeterminate
- **THEN** neither the prior nor proposed resulting snapshot may be assumed authoritative for protected mutation, no home-mutation permission exists, and reconciliation through the recovery commit-observation contract is required

### Requirement: Clean and valid claims require typed fence evidence
<!-- dwv:req req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->

The recovery store SHALL own the admissibility of typed evidence used for durable clean, valid-integrity, and clean-session claims. Evidence SHALL identify the store incarnation, ordering domain, accepted and synchronized-through watermarks, topology epoch, affected range or region, capability evidence, and relevant generations. A claim SHALL be rejected when required evidence is missing, volatile, future, stale, partial, cross-store, or mismatched.

#### Scenario: A region is cleared after valid typed evidence

- **WHEN** a dirty region has admissible covering evidence and one recovery transaction records a clean checkpoint
- **THEN** the region may become clean and the checkpoint generation is durably recorded

#### Scenario: Volatile completion is supplied as durable authority

- **WHEN** a caller attempts a clean or valid claim using volatile or unknown persistence evidence
- **THEN** the mutation is rejected and the affected state remains dirty, stale, or indeterminate

### Requirement: Topology snapshots are immutable within a transaction
<!-- dwv:req req.recovery-state-semantics.topology-snapshots-are-immutable-within-a-transaction -->

Transactions SHALL capture one topology epoch. A commit under a different epoch SHALL fail. Preparing a new topology SHALL create an explicit pending snapshot; publication requires a separate commit mutation and SHALL not rewrite the epoch of an existing transaction.

#### Scenario: Topology changes during an operation

- **WHEN** a transaction captured epoch E and the store now requires epoch E+1
- **THEN** the transaction is rejected or invalidated and no mutation is applied under the old snapshot

### Requirement: Semantic export and health are independent of storage engine layout
<!-- dwv:req req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->

The recovery boundary SHALL expose a bounded semantic snapshot/manifest, writable-open path, and observational inspection path. Observational inspection SHALL classify absent state, present supported state, corrupt or unreadable state, unsupported interpretation, migration required before writable use, and uncertain or reconciliation-required state without creating a database, acquiring writable ownership, running initialization or migration, repairing state, or changing the inspected artifact. Missing, corrupt, stale, unavailable, unsupported, migration-required, reconciliation-required, or failed-to-load recovery state SHALL remain an explicit conservative result and SHALL block new home mutations until the owning recovery semantics establish current authority. A caller SHALL NOT substitute generation zero, a clean snapshot, success, or a successful trace for failed recovery access. Export SHALL not expose SQLite pages, row IDs, or implementation pointers.

#### Scenario: Recovery state is missing

- **WHEN** observational inspection finds no durable recovery artifact
- **THEN** it reports absent state, creates no artifact, and callers receive a conservative recovery decision rather than an implicit clean state

#### Scenario: Recovery state cannot be loaded

- **WHEN** observational inspection finds corrupt, unreadable, stale, locked, unavailable, unsupported, migration-required, or reconciliation-required state
- **THEN** it preserves the exact conservative classification without initialization or migration and callers do not continue with generation zero

#### Scenario: Supported recovery state is inspected

- **WHEN** observational inspection validates a supported current semantic manifest
- **THEN** it returns the bounded semantic manifest while leaving payload, parity, recovery generation, semantic schema, migration state, and integrity state unchanged
### Requirement: Writable recovery ownership is crash-releasing
<!-- dwv:req req.recovery-state-semantics.writable-recovery-ownership-is-crash-releasing -->

A writable recovery adapter SHALL hold its single-writer claim through an operating-system descriptor lock whose ownership is released by process death without destructor or cleanup execution. Marker content MAY aid diagnostics but marker existence SHALL NOT own the claim. Acquisition and reacquisition SHALL validate the current recovery identity, topology, schema, and health; every create/open failure SHALL release partial ownership and leave no residual claim.

#### Scenario: Recovery owner process dies

- **WHEN** one process holds the writable recovery claim and is killed without cleanup
- **THEN** the operating system releases the claim and a new process may reacquire only after current recovery authority is revalidated

#### Scenario: Recovery creation fails after claiming

- **WHEN** `create_new` or later initialization fails after acquiring ownership
- **THEN** the descriptor claim is released and no marker existence prevents a valid later acquisition


### Requirement: SQLite remains an evidence-driven adapter decision
<!-- dwv:req req.recovery-state-semantics.sqlite-remains-an-evidence-driven-adapter-decision -->

The project SHALL keep SQLite, journal mode, synchronization, checkpoint policy, connection topology, schema, and migration details behind the semantic interface. Production selection SHALL require evaluation across the declared durability and reset cases and SHALL NOT follow from folklore or a successful process-local commit alone.

#### Scenario: Candidate SQLite modes are compared

- **WHEN** candidate journal and synchronization configurations run through the declared crash and reset cases
- **THEN** the evidence records each result while unsupported or untested durability claims remain unavailable

### Requirement: Semantic schema, migrations, and exports are versioned independently of SQLite
<!-- dwv:req req.recovery-state-semantics.semantic-schema-migrations-and-exports-are-versioned-independently-of-sqlite -->

The portable recovery boundary SHALL expose a versioned semantic schema descriptor, an explicit migration plan, and a bounded export manifest. These representations SHALL contain recovery concepts, generations, topology, evidence, and state, but SHALL NOT expose SQL table names, row IDs, pages, journal files, connection handles, or crate-specific database types.

#### Scenario: A semantic manifest is exported

- **WHEN** the requested current generation is healthy and within export limits
- **THEN** the export includes its semantic schema version and recovery snapshot and contains no storage-engine layout

#### Scenario: An unsupported migration is requested

- **WHEN** an adapter requests a migration from an unknown or incompatible semantic version
- **THEN** the plan is rejected without changing the recovery snapshot

### Requirement: Evaluation fixtures cover candidate durability and reset boundaries
<!-- dwv:req req.recovery-state-semantics.evaluation-fixtures-cover-candidate-durability-and-reset-boundaries -->
<!-- dwv:requires req.volatile-media-simulator.media-state-separates-durable-and-process-visible-effects -->

Evaluation SHALL provide deterministic fixtures for candidate journal modes, synchronization modes, checkpoint policies, process reset, VM reset, power loss, commit rejection, lost commit acknowledgement, missing state, main-state corruption, and journal-state corruption. Each fixture SHALL state the conservative semantic disposition and evidence still required; fixture presence SHALL NOT select a production SQLite mode.

#### Scenario: A dirty-intent commit is rejected or uncertain

- **WHEN** the simulator reports that the recovery commit did not become durably known
- **THEN** the expected disposition forbids protected home mutation and requires reconciliation

#### Scenario: Recovery state is missing or corrupt after reset

- **WHEN** the recovery adapter cannot validate its state or journal
- **THEN** the expected disposition blocks writable assembly and permits only an explicit recovery plan

### Requirement: Recovery adapters report conservative commit observations
<!-- dwv:req req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->

The replaceable adapter seam SHALL distinguish durable, rejected, lost, and corrupt commit observations. The semantic store SHALL accept a transaction as a protocol fact only when the adapter reports durable commitment. A rejected commit SHALL leave the exact prior semantic state authoritative. Before a concrete writable adapter attempts to publish proposed state, it SHALL durably preserve enough exact prior/proposed semantic evidence to resolve an interrupted or uncertain commit after process-local state is released. After a lost or corrupt acknowledgement for an operation that may have committed, process-local belief SHALL authorize no dependent mutation; the caller SHALL release that belief and reopen the durable artifact through current semantic validation.

#### Scenario: A checkpoint acknowledgement is lost

- **WHEN** a backend may have committed a checkpoint but cannot prove the resulting semantic state
- **THEN** the semantic disposition is reconciliation-required, no clean or writable claim is inferred, and dependent mutation stops

#### Scenario: Reopen finds the exact prior state

- **WHEN** reconciliation reopens and validates durable state semantically identical to the complete prior state
- **THEN** the prior state remains authoritative and the unacknowledged proposal is not assumed committed

#### Scenario: Reopen finds the exact proposed state

- **WHEN** reconciliation reopens and validates durable state semantically identical to the complete proposed state
- **THEN** the proposed state may become authoritative under the same validation required for an ordinarily opened current state

#### Scenario: Reopen finds neither exact state

- **WHEN** reopened state differs semantically from both prior and proposed state, cannot be read, or cannot be interpreted currently
- **THEN** reconciliation remains required and neither candidate authorizes dependent mutation

#### Scenario: Inspection encounters unresolved commit intent

- **WHEN** read-only inspection encounters a durable prior/proposed intent that writable reopen has not reconciled
- **THEN** it reports reconciliation-required before manifest classification and does not initialize, migrate, clean up, or otherwise mutate semantic state

### Requirement: The SQLite prototype remains evaluation-only and storage-independent at the semantic boundary
<!-- dwv:req req.recovery-state-semantics.the-sqlite-prototype-remains-evaluation-only-and-storage-independent-at-the-semantic-boundary -->

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
<!-- dwv:req req.recovery-state-semantics.offline-rebuild-progress-is-durable-semantic-authority -->

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
