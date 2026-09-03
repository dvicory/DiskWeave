# recovery-state-semantics Specification

## Purpose
Recovery state is durable protocol authority, not a mirror of data/parity bytes. It records the evidence and generations needed to decide whether future writes, recovery CLEAN transitions, and integrity claims are allowed. SQLite may implement this interface later, but SQL tables and row IDs SHALL not define the portable semantics.
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

### Requirement: Topology snapshots are immutable within a transaction
<!-- dwv:req req.recovery-state-semantics.topology-snapshots-are-immutable-within-a-transaction -->

Transactions SHALL capture one topology epoch. A commit under a different epoch SHALL fail. Preparing a new topology SHALL create an explicit pending snapshot; publication requires a separate commit mutation and SHALL not rewrite the epoch of an existing transaction.

#### Scenario: Topology changes during an operation

- **WHEN** a transaction captured epoch E and the store now requires epoch E+1
- **THEN** the transaction is rejected or invalidated and no mutation is applied under the old snapshot

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

- **WHEN** the replacement bytes through offset N have been read back, equation-verified, and covered by persistence evidence under the captured source topology
- **THEN** one atomic recovery transaction advances the first-unprocessed-byte cursor to N and records the matching persistence evidence

#### Scenario: A stale rebuild transaction commits

- **WHEN** another recovery transaction or topology change invalidates the generation captured by a rebuild checkpoint
- **THEN** the checkpoint mutation fails atomically and cannot claim unverified replacement bytes durable

#### Scenario: A rebuild manifest is exported

- **WHEN** a current bounded semantic manifest is exported during an interrupted rebuild
- **THEN** it contains enough typed rebuild state to validate a later resume without exposing a path, SQLite layout, or process-local resource


### Requirement: Exact persistence evidence follows owner-qualified claim liveness
<!-- dwv:req req.recovery-state-semantics.exact-persistence-evidence-follows-owner-qualified-claim-liveness -->
<!-- dwv:requires req.recovery-state-semantics.fence-occurrences-have-stable-exact-identities-and-coverage -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set -->
<!-- dwv:requires req.checksum-plane.current-baseline-completion-is-persisted-and-exact -->
<!-- dwv:requires req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts -->

The recovery store SHALL retain each exact fence occurrence while any owner-qualified current or unresolved claim can reach that occurrence. The following root classes SHALL use the stated authoritative fact and exact binding:

- A current `CLEAN` region root is created by the durable dirty-integrity `CLEAN` transition and binds one fence occurrence, region identity, and clean generation. It remains live while that region is currently `CLEAN`; when the region becomes `Dirty` or `Indeterminate`, the transition SHALL discharge or explicitly rebind the root in the same successor. A copied last-clean certificate in a non-clean record is not a root by itself.
- A valid-integrity root is created by the checksum owner when a valid record is durably installed and binds one fence occurrence, extent identity, profile/set generation, content generation, and digest validity. It is discharged or rebound only when the owner durably changes that record to stale/absent or installs a newer valid record.
- A persisted session-lifecycle root is created conservatively whenever a supported successor contains a writable-session global fence, including `CloseWritableSession`, migration, and reopen/import; it binds occurrence identity, session identity, and close generation. Replacing a closed session with `BeginWritableSession` SHALL carry the prior root forward rather than discharge or rebind it. Because no current canonical session-lifecycle discharge owner exists, this change SHALL neither discharge nor rebind that root through generic retirement; session-close and closed-mutation-set supersession remain separate downstream authority.
- A coded-capture root is created by the dirty-integrity capture owner for each exact clean-closure or release-certificate binding. Open, commit-pending, commit-unknown, and otherwise unresolved capture phases retain their roots; discharge requires that owner's phase-specific cleanup, compaction, or retirement authority.
- An unresolved-recovery-commit root is created only when the recovery adapter durably preserves an exact prior/proposed manifest and commit-intent identity for a may-have-committed recovery transaction. It is discharged only by a known rejection/non-commit, an exact durable successor, or authoritative reopen reconciliation; process-local operation state is not a durable root.
- A legacy-unreconciled root is created only by the explicit semantic migration for an older manifest and binds the old schema, exact occurrence, and any ambiguous copied-value candidates. In the current migration boundary, it remains conservative stale/read-only state; no current owner rebind, migration-owner inventory proof, reset, or rebuild path is provided. It is not an historical/audit retention product.

Each root SHALL carry its exact occurrence identity, owner binding, relevant topology and generation, and the durable predecessor or successor fact that creates, discharges, or rebinds it. Serialized phase values, DTO equality, process reachability, age, latest-N selection, and scalar watermarks SHALL NOT create or discharge a root. A settled ordinary write SHALL NOT remain a permanent root merely because it once produced a fence. Releasing one root SHALL NOT retire evidence needed by another root. A fence may be retired only when every root that can reach its occurrence has been discharged or atomically rebound.

For this requirement's bounded registry-liveness portion, the state, actions, and invariants in `models/quint/PersistenceEvidenceRetirement.qnt` SHALL be the sole exact semantic authority for associating supplied owner-qualified roots with immutable fence certificates, retaining a predecessor while a supplied root reaches it, preserving unrelated roots during a retirement proposal, and removing an occurrence only after every root reaching it is discharged or atomically rebound. Owner qualification, root creation, and root discharge facts remain owned by the requirements named above; the model receives them as inputs.

#### Scenario: A never-rewritten clean region remains protected

- **WHEN** a region is currently `CLEAN` and its exact clean root still names fence occurrence F
- **THEN** F remains retained even after the originating operation and capture settle, until the clean owner rebinds that root to a successor or retires the clean claim

#### Scenario: A dirty transition discharges a stale clean binding

- **WHEN** a region changes from `CLEAN` to `Dirty` or `Indeterminate` and its record contains an older last-clean certificate
- **THEN** the same durable successor discharges or rebinds the current clean root, and the copied certificate cannot independently keep F live or authorize a clean claim

#### Scenario: A new session does not erase prior session evidence

- **WHEN** `BeginWritableSession` replaces a previously closed session whose global fence has a retained session-lifecycle root
- **THEN** the prior root and occurrence remain retained, the new session begins without a generic rebind, and only the future canonical session owner may discharge or supersede that prior root

#### Scenario: A valid integrity record keeps exact evidence

- **WHEN** a valid integrity record still names F for its extent, profile/set, content generation, digest, and persistence evidence
- **THEN** F remains retained even if the originating write and operation have otherwise settled

#### Scenario: One unresolved dependency does not pin unrelated roots

- **WHEN** one disk, range, operation, capture, or recovery commit remains known failed, lost, or unresolved while another settled fence occurrence is not reachable by that root
- **THEN** only the dependent occurrence remains retained and the unrelated settled occurrence remains eligible for exact retirement

#### Scenario: A newer certificate supersedes an older claim

- **WHEN** an owner proves that every root formerly reaching F is atomically rebound to a newer occurrence with exact compatible bindings and coverage
- **THEN** F may be retired without weakening any surviving clean, integrity, session, capture, or recovery claim

### Requirement: Fence occurrences have stable exact identities and coverage
<!-- dwv:req req.recovery-state-semantics.fence-occurrences-have-stable-exact-identities-and-coverage -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.recovery-state-semantics.topology-snapshots-are-immutable-within-a-transaction -->

Every persisted composite fence SHALL have a unique monotonic semantic fence-occurrence identity within its recovery lineage. The identity SHALL be persisted, SHALL never be reused, and SHALL be the identity referenced by roots and retirement transitions; vector position, serialized bytes, and certificate value SHALL not identify an occurrence. Equal certificate values MAY occur more than once, but each occurrence remains distinct until its own roots are discharged. Each root reference SHALL bind the occurrence identity to the exact immutable certificate facts it relies on.

Within a newly written certificate, store-fence references SHALL use one canonical semantic order and SHALL contain no duplicate store-fence identity; captured region and integrity pairs SHALL use canonical order and SHALL contain no duplicate target identity. Malformed duplicates SHALL be rejected rather than silently deduplicated. Legacy ordering and duplicates SHALL be handled only by the explicit migration requirement below.

Supersession SHALL be evaluated separately for every root, not globally. Each rebind MAY target its own successor occurrence, so one predecessor with multiple roots MAY atomically rebind those roots to different retained successor occurrences. A rebind SHALL preserve the same captured topology epoch, fence domain, required store set, store incarnation, capability evidence, and exact target identity, and SHALL prove the newer occurrence's generation and synchronized-through watermark cover the particular region or integrity claim. Region and integrity identities are exact semantic targets; partial byte-range overlap or a scalar watermark SHALL not establish coverage or supersession. The last occurrence satisfying a root SHALL not be removed unless that root is atomically rebound or discharged.

For this requirement's bounded occurrence-and-certificate portion, the state, actions, and invariants in `models/quint/PersistenceEvidenceRetirement.qnt` SHALL be the sole exact semantic authority for monotonic non-reused occurrence identity, immutable certificate versus mutable owner-fact separation, exact root/fence binding, componentwise per-store and per-claim compatibility, and predecessor-root rebind or discharge validation where each rebind names its own successor occurrence. Serialized ordering, malformed-input duplicate handling, and legacy migration remain prose-owned.

#### Scenario: Equal certificate occurrences remain distinct

- **WHEN** two persisted fence occurrences have equal certificate values and different occurrence identities, and only one occurrence is bound by a live root
- **THEN** retirement may remove only the unbound occurrence and SHALL preserve the root-bound occurrence

#### Scenario: A permuted or duplicate certificate is proposed

- **WHEN** a new certificate contains non-canonical ordering or duplicate store, region, or integrity identities
- **THEN** the recovery boundary rejects it without changing the predecessor

#### Scenario: Partial overlap is proposed as supersession

- **WHEN** a newer occurrence covers only part of an older root's exact regions or integrity extents, or changes a store incarnation, capability, domain, or topology
- **THEN** the root remains bound to the older occurrence and the older occurrence is not retired

#### Scenario: Exact root rebind commits

- **WHEN** a current owner supplies an exact predecessor-bound rebind for every root reaching F and the successor occurrence satisfies each root's exact coverage
- **THEN** the successor contains the new bindings and F is no longer reachable by any retained root

#### Scenario: Independent successor occurrences are committed atomically

- **WHEN** one predecessor occurrence has multiple roots and each root has a complete exact rebind to a different retained successor occurrence
- **THEN** one generation- and topology-checked successor commits all root rebinds together, removes the predecessor only after every root is covered, and preserves exact bindings for each independent successor

### Requirement: Fence retirement preserves an exact durable predecessor
<!-- dwv:req req.recovery-state-semantics.fence-retirement-preserves-an-exact-durable-predecessor -->
<!-- dwv:requires req.recovery-state-semantics.exact-persistence-evidence-follows-owner-qualified-claim-liveness -->
<!-- dwv:requires req.recovery-state-semantics.fence-occurrences-have-stable-exact-identities-and-coverage -->
<!-- dwv:requires req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->
<!-- dwv:requires req.recovery-state-semantics.topology-snapshots-are-immutable-within-a-transaction -->
<!-- dwv:requires req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->

Every exact fence retirement SHALL be represented by one generation- and topology-checked recovery transaction with the exact durable predecessor, occurrence identities to retire, all root rebinds or discharges, and the complete proposed successor snapshot. The successor SHALL remove only occurrences proven unreachable and SHALL preserve every exact store-incarnation, capability, fence-domain, watermark, topology, target, generation, and claim binding required by remaining roots. A known rejected or known-not-committed retirement SHALL leave the predecessor authoritative. A durable successor SHALL make the successor authoritative. A lost, corrupt, or unclassifiable acknowledgement that may have committed SHALL establish neither candidate as authoritative until exact reopen reconciliation.

This retirement boundary SHALL reject a successor that exceeds the configured semantic representation or export bounds without removing predecessor evidence. Pre-mutation capacity reservation for a protected write remains the separately tracked serving prerequisite `dwv-x6y.2.2`; this requirement SHALL NOT be interpreted as implementing that reservation protocol or as authorizing protected mutation after a later capacity failure.

For this requirement's bounded transaction-and-reconciliation portion, the state, actions, and invariants in `models/quint/PersistenceEvidenceRetirement.qnt` SHALL be the sole exact semantic authority for generation/topology-checked preparation, durable successor installation, known rejection, stale rejection, exact-intent unknown observation, and prior/proposed/neither reconciliation. The model's retirement transition consumes the occurrence/root plan delegated by `req.recovery-state-semantics.fence-occurrences-have-stable-exact-identities-and-coverage` and preserves the exact predecessor until the corresponding outcome is authoritative.

The model receives owner-qualified certificate facts, owner facts, admissions, rebind/discharge proofs, and reopen observations as inputs. Their production, qualification, owner liveness, serialized canonical ordering and malformed-input duplicate detection, semantic schema migration, export and capacity bounds, physical durability, adapter mechanics, and implementation conformance remain governed by the named requirements; they are not delegated to the model.
`verification/quint/PersistenceEvidenceRetirementAnalysis.qnt`, `PersistenceEvidenceRetirementWideAnalysis.qnt`, and `PersistenceEvidenceRetirementMutants.qnt` are evidence-only finite configurations. They do not define product semantics or establish exhaustive coverage of the wide analysis, all parameterized inputs, or Rust behavior.

#### Scenario: An authorized retirement commits

- **WHEN** the expected recovery generation and topology match, every retirement occurrence is unreachable or atomically rebound, and the successor is representable
- **THEN** the store durably commits the successor and retired occurrences are absent from subsequent semantic export and occurrence lookup

#### Scenario: A retirement is known rejected

- **WHEN** the adapter reports that the retirement transaction was rejected or was definitely not committed
- **THEN** the exact predecessor remains authoritative and every occurrence and root remains available for retry

#### Scenario: A retirement acknowledgement is unknown

- **WHEN** the adapter may have committed the retirement but returns a lost, corrupt, or unclassifiable acknowledgement
- **THEN** no process-local belief authorizes evidence deletion or dependent data/parity, `CLEAN`, integrity, session, or release claims, and reopen reconciliation is required

#### Scenario: A stale retirement is submitted

- **WHEN** another recovery transaction has advanced the generation or changed the topology after retirement preparation
- **THEN** the retirement is rejected atomically and the exact prior snapshot remains authoritative

#### Scenario: A retirement successor exceeds bounds

- **WHEN** the proposed successor cannot be represented within configured semantic or export bounds without discarding required evidence
- **THEN** the candidate is rejected before predecessor evidence is removed and the caller receives conservative capacity refusal

### Requirement: Legacy fence retention migrates explicitly
<!-- dwv:req req.recovery-state-semantics.legacy-fence-retention-migrates-explicitly -->
<!-- dwv:requires req.recovery-state-semantics.semantic-schema-migrations-and-exports-are-versioned-independently-of-sqlite -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->
<!-- dwv:requires req.recovery-state-semantics.fence-occurrences-have-stable-exact-identities-and-coverage -->
<!-- dwv:requires req.recovery-state-semantics.exact-persistence-evidence-follows-owner-qualified-claim-liveness -->
<!-- dwv:requires req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->

Adding fence-occurrence identities and root bindings SHALL create a new semantic recovery schema version after the current version and an explicit migration step. A current adapter SHALL classify the older manifest as migration-required before writable use rather than interpreting missing fields through defaults. Migration SHALL assign stable identities to every legacy fence occurrence while preserving duplicates and exact certificate values, and SHALL create a bounded `legacy-unreconciled` root for every occurrence or ambiguous copied-value reference. In the current migration boundary, a manifest with retained legacy roots SHALL remain stale/read-only; no migration-owner inventory proof, current-owner rebind, reset, or rebuild path is provided. A future change MAY define one only with an explicit data-preservation contract.

#### Scenario: A legacy manifest is opened

- **WHEN** a manifest uses the prior semantic schema without occurrence identities or root bindings
- **THEN** read-only inspection reports migration-required before writable use without writable mutation, and no legacy fence is retired

#### Scenario: Legacy duplicates are migrated

- **WHEN** an older manifest contains equal or permuted fence values at distinct persisted positions
- **THEN** migration assigns distinct stable occurrence identities, preserves every exact value, and does not silently coalesce occurrences

#### Scenario: A migrated manifest remains read-only

- **WHEN** explicit migration has assigned identities and retained one or more `legacy-unreconciled` roots
- **THEN** semantic inspection reports stale/read-only state and writable recovery remains unavailable until a separately scoped owner decision

#### Scenario: A legacy copied reference is ambiguous

- **WHEN** a legacy clean, integrity, session, or capture binding matches more than one fence occurrence
- **THEN** migration retains every exact candidate under a legacy-unreconciled root, keeps the migrated manifest stale/read-only, and does not silently coalesce, delete, rebind, reset, or rebuild the candidate

#### Scenario: Migration acknowledgement is unknown

- **WHEN** the adapter may have published the migrated successor but cannot classify the acknowledgement
- **THEN** neither legacy predecessor nor migrated successor is assumed from process-local state and reopen reconciliation is required
