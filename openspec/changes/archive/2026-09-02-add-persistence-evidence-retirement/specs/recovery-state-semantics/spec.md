## ADDED Requirements

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
- A legacy-unreconciled root is created only by the explicit semantic migration for an older manifest and binds the old schema, exact occurrence, and any ambiguous copied-value candidates. It is discharged only by a current owner-qualified rebind or by the migration owner's complete predecessor-bound legacy inventory proof; it is not an historical/audit retention product.

Each root SHALL carry its exact occurrence identity, owner binding, relevant topology and generation, and the durable predecessor or successor fact that creates, discharges, or rebinds it. Serialized phase values, DTO equality, process reachability, age, latest-N selection, and scalar watermarks SHALL NOT create or discharge a root. A settled ordinary write SHALL NOT remain a permanent root merely because it once produced a fence. Releasing one root SHALL NOT retire evidence needed by another root. A fence may be retired only when every root that can reach its occurrence has been discharged or atomically rebound.

For this requirement's bounded registry-liveness portion, the state, actions,
and invariants in
`models/quint/PersistenceEvidenceRetirement.qnt` SHALL be the sole exact
semantic authority for associating supplied owner-qualified roots with
immutable fence certificates, retaining a predecessor while a supplied root
reaches it, preserving unrelated roots during a retirement proposal, and
removing an occurrence only after every root reaching it is discharged or
atomically rebound. Owner qualification, root creation, and root discharge
facts remain owned by the requirements named above; the model receives them
as inputs.

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

For this requirement's bounded occurrence-and-certificate portion, the state,
actions, and invariants in
`models/quint/PersistenceEvidenceRetirement.qnt` SHALL be the sole exact
semantic authority for monotonic non-reused occurrence identity, immutable
certificate versus mutable owner-fact separation, exact root/fence binding,
componentwise per-store and per-claim compatibility, and predecessor-root
rebind or discharge validation where each rebind names its own successor
occurrence. Serialized ordering, malformed-input duplicate handling, and
legacy migration remain prose-owned.

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
 
For this requirement's bounded transaction-and-reconciliation portion, the
state, actions, and invariants in
`models/quint/PersistenceEvidenceRetirement.qnt` SHALL be the sole exact
semantic authority for generation/topology-checked preparation, durable
successor installation, known rejection, stale rejection, exact-intent
unknown observation, and prior/proposed/neither reconciliation. The model's
retirement transition consumes the occurrence/root plan delegated by
`req.recovery-state-semantics.fence-occurrences-have-stable-exact-identities-and-coverage`
and preserves the exact predecessor until the corresponding outcome is
authoritative.

The model receives owner-qualified certificate facts, owner facts, admissions,
rebind/discharge proofs, and reopen observations as inputs. Their production,
qualification, owner liveness, serialized canonical ordering and
malformed-input duplicate detection, semantic schema migration, export and
capacity bounds, physical durability, adapter mechanics, and implementation
conformance remain governed by the named requirements; they are not delegated
to the model.
`verification/quint/PersistenceEvidenceRetirementAnalysis.qnt`,
`PersistenceEvidenceRetirementWideAnalysis.qnt`, and
`PersistenceEvidenceRetirementMutants.qnt` are evidence-only finite
configurations. They do not define product semantics or establish exhaustive
coverage of the wide analysis, all parameterized inputs, or Rust behavior.

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

Adding fence-occurrence identities and root bindings SHALL create a new semantic recovery schema version after the current version and an explicit migration step. A current adapter SHALL classify the older manifest as migration-required before writable use rather than interpreting missing fields through defaults. Migration SHALL assign stable identities to every legacy fence occurrence while preserving duplicates and exact certificate values, and SHALL create a bounded `legacy-unreconciled` root for every occurrence or ambiguous copied-value reference. In this change, migrated manifests with retained legacy roots SHALL remain stale/read-only and no migration-owner inventory proof, current-owner rebind, reset, or rebuild path is provided; a future change MAY define one only with an explicit data-preservation contract.

#### Scenario: A legacy manifest is opened

- **WHEN** a manifest uses the prior semantic schema without occurrence identities or root bindings
- **THEN** read-only inspection reports migration-required before writable use without writable mutation, and no legacy fence is retired

#### Scenario: Legacy duplicates are migrated

- **WHEN** an older manifest contains equal or permuted fence values at distinct persisted positions
- **THEN** migration assigns distinct stable occurrence identities, preserves every exact value, and does not silently coalesce occurrences

#### Scenario: A legacy copied reference is ambiguous

- **WHEN** a legacy clean, integrity, session, or capture binding matches more than one fence occurrence
- **THEN** migration retains every exact candidate under a legacy-unreconciled root, keeps the migrated manifest stale/read-only, and does not silently coalesce, delete, rebind, reset, or rebuild the candidate

#### Scenario: Migration acknowledgement is unknown

- **WHEN** the adapter may have published the migrated successor but cannot classify the acknowledgement
- **THEN** neither legacy predecessor nor migrated successor is assumed from process-local state and reopen reconciliation is required
