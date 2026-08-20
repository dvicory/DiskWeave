## RENAMED Requirements

- FROM: ### Requirement: Home mutation requires durable dirty and integrity invalidation intent
- TO: ### Requirement: Data/parity write requires write-recovery record

- FROM: ### Requirement: Clean and valid claims require typed fence evidence
- TO: ### Requirement: Clean and valid claims require persistence evidence

## MODIFIED Requirements

### Requirement: Data/parity write requires write-recovery record
<!-- dwv:req req.recovery-state-semantics.data-parity-write-requires-write-recovery-record -->
<!-- dwv:refines req.dirty-integrity-invalidation.write-recovery-record-precedes-data-parity-write -->

The semantic store SHALL atomically persist the affected dirty regions and stale integrity extents at one generation before a caller may rely on that state as the write-recovery record required for a data/parity write. Dirty and stale generations SHALL survive in-memory process loss until a later explicit recovery transaction changes them. A rejected or generation-mismatched transaction leaves the prior snapshot authoritative. A lost, corrupt, or otherwise unclassifiable commit observation establishes no authoritative resulting snapshot for the caller, provides no permission for a data/parity write, and requires reconciliation through the write-recovery-record commit-observation contract.

#### Scenario: A valid integrity extent is touched

- **WHEN** one recovery transaction marks a region dirty and invalidates a valid extent
- **THEN** the resulting snapshot records both changes at one committed recovery generation and no valid digest remains authoritative for that extent

#### Scenario: The write-recovery record is rejected before it becomes durable

- **WHEN** the transaction is rejected or generation-mismatched before authoritative commit
- **THEN** the proposed transaction does not become authoritative, the prior snapshot remains authoritative, and the caller receives no permission for a data/parity write

#### Scenario: The write-recovery record's commit outcome cannot be determined

- **WHEN** commitment may have occurred but its observation is lost, corrupt, or cannot be classified safely
- **THEN** neither the prior nor proposed resulting snapshot may be assumed authoritative for a data/parity write, no data/parity-write permission exists, and reconciliation through the commit-observation contract is required

### Requirement: Clean and valid claims require persistence evidence
<!-- dwv:req req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->

The recovery store SHALL own the admissibility of persistence evidence used for durable recovery `CLEAN`, valid-integrity, and clean-session claims. Evidence SHALL identify the store incarnation, ordering domain, accepted and synchronized-through watermarks, topology epoch, affected range or region, capability evidence, and relevant generations. A claim SHALL be rejected when required evidence is missing, volatile, future, stale, partial, cross-store, or mismatched.

#### Scenario: A dirty region is cleared after persistence evidence

- **WHEN** a dirty region has admissible covering persistence evidence and one recovery transaction commits recovery state `CLEAN`
- **THEN** the region may become `CLEAN` and the transition generation is durably recorded

#### Scenario: I/O completion does not prove persistence

- **WHEN** a caller attempts a clean or valid claim using volatile or unknown persistence evidence
- **THEN** the operation is rejected and the affected state remains dirty, stale, or indeterminate

### Requirement: Semantic export and health are independent of storage engine layout
<!-- dwv:req req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->

The recovery boundary SHALL expose a bounded semantic snapshot/manifest, writable-open path, and observational inspection path. Observational inspection SHALL classify absent state, present supported state, corrupt or unreadable state, unsupported interpretation, migration required before writable use, and uncertain state that requires reconciliation without creating a database, acquiring writable ownership, running initialization or migration, repairing state, or changing the inspected artifact. Missing, corrupt, stale, unavailable, unsupported, migration-required, reconciliation-required, or failed-to-load recovery state SHALL remain an explicit conservative result and SHALL block new data/parity writes until the owning recovery semantics establish current authority. A caller SHALL NOT substitute generation zero, a clean snapshot, success, or a successful trace for failed recovery access. Export SHALL not expose SQLite pages, row IDs, or implementation pointers.

#### Scenario: Recovery state is missing

- **WHEN** observational inspection finds no durable recovery artifact
- **THEN** it reports absent state, creates no artifact, and callers receive a conservative recovery decision rather than an implicit clean state

#### Scenario: Recovery state cannot be loaded

- **WHEN** observational inspection finds corrupt, unreadable, stale, locked, unavailable, unsupported, migration-required, or reconciliation-required state
- **THEN** it preserves the exact conservative classification without initialization or migration and callers do not continue with generation zero

#### Scenario: Supported recovery state is inspected

- **WHEN** observational inspection validates a supported current semantic manifest
- **THEN** it returns the bounded semantic manifest while leaving payload, parity, recovery generation, semantic schema, migration state, and integrity state unchanged

### Requirement: Evaluation fixtures cover candidate durability and reset boundaries
<!-- dwv:req req.recovery-state-semantics.evaluation-fixtures-cover-candidate-durability-and-reset-boundaries -->
<!-- dwv:requires req.volatile-media-simulator.media-state-separates-durable-and-process-visible-effects -->

Evaluation SHALL provide deterministic fixtures for candidate journal modes, synchronization modes, checkpoint policies, process reset, VM reset, power loss, commit rejection, lost commit acknowledgement, missing state, main-state corruption, and journal-state corruption. Each fixture SHALL state the conservative semantic result and evidence still required; fixture presence SHALL NOT select a production SQLite mode.

#### Scenario: The write-recovery record is rejected or its outcome is unknown

- **WHEN** the simulator reports that the recovery commit did not become durably known
- **THEN** the expected result forbids a data/parity write and requires reconciliation

#### Scenario: Recovery state is missing or corrupt after reset

- **WHEN** the recovery adapter cannot validate its state or journal
- **THEN** the expected result blocks writable assembly and permits only an explicit recovery plan

### Requirement: Recovery adapters report conservative commit observations
<!-- dwv:req req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->

The replaceable adapter seam SHALL distinguish durable, rejected, lost, and corrupt commit observations. The semantic store SHALL accept a transaction as a protocol fact only when the adapter reports durable commitment. A rejected commit SHALL leave the exact prior semantic state authoritative. Before a concrete writable adapter attempts to publish proposed state, it SHALL durably preserve enough exact prior/proposed semantic evidence to resolve an interrupted or uncertain commit after process-local state is released. After a lost or corrupt acknowledgement for an operation that may have committed, process-local belief SHALL authorize no dependent data/parity write; the caller SHALL release that belief and reopen the durable artifact through current semantic validation.

#### Scenario: Recovery CLEAN cannot be confirmed

- **WHEN** a backend may have committed recovery state `CLEAN` but cannot prove the resulting semantic state
- **THEN** the result requires reconciliation, no clean or writable claim is inferred, and dependent data/parity writes stop

#### Scenario: Reopen finds the exact prior state

- **WHEN** reconciliation reopens and validates durable state semantically identical to the complete prior state
- **THEN** the prior state remains authoritative and the unacknowledged proposal is not assumed committed

#### Scenario: Reopen finds the exact proposed state

- **WHEN** reconciliation reopens and validates durable state semantically identical to the complete proposed state
- **THEN** the proposed state may become authoritative under the same validation required for an ordinarily opened current state

#### Scenario: Reopen finds neither exact state

- **WHEN** reopened state differs semantically from both prior and proposed state, cannot be read, or cannot be interpreted currently
- **THEN** reconciliation remains required and neither candidate authorizes dependent data/parity writes

#### Scenario: Inspection finds an unresolved write-recovery record outcome

- **WHEN** read-only inspection encounters a durable prior/proposed write-recovery-record commit outcome that writable reopen has not reconciled
- **THEN** it reports that reconciliation is required before manifest classification and does not initialize, migrate, clean up, or otherwise mutate semantic state

### Requirement: Offline rebuild progress is durable semantic authority
<!-- dwv:req req.recovery-state-semantics.offline-rebuild-progress-is-durable-semantic-authority -->

Recovery state SHALL represent an offline rebuild with a stable rebuild identifier, source array/topology/recovery generation, missing stable slot and coding position, replacement assignment instance and store identity, protected geometry, first-unprocessed-byte cursor, and lifecycle state. Cursor advancement and verified completion SHALL be generation-checked recovery mutations, bounded in semantic export, and independent of SQL rows, file paths, runtime handles, or executor objects.

#### Scenario: A rebuild cursor advances

- **WHEN** the replacement bytes through offset N have been read back, equation-verified, and covered by persistence evidence under the captured source topology
- **THEN** one atomic recovery transaction advances the first-unprocessed-byte cursor to N and records the matching persistence evidence

#### Scenario: A stale rebuild transaction commits

- **WHEN** another recovery transaction or topology change invalidates the generation captured by a rebuild checkpoint
- **THEN** the checkpoint mutation fails atomically and cannot claim unverified replacement bytes durable

#### Scenario: A rebuild manifest is exported

- **WHEN** a current bounded semantic manifest is exported during an interrupted rebuild
- **THEN** it contains enough typed rebuild state to validate a later resume without exposing a path, SQLite layout, or process-local resource
