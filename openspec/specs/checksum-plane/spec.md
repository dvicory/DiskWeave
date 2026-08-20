# checksum-plane Specification

## Purpose
This capability provides independent generational checksum evidence for data and parity extents, with asynchronous revalidation that cannot install a digest for different bytes or unsupported durability evidence.
## Requirements
### Requirement: Checksum coverage names targets and extents
<!-- dwv:req req.checksum-plane.checksum-coverage-names-targets-and-extents -->

The checksum plane SHALL identify each data, P, and optional Q target by stable semantic identity and map it to explicit bounded checksum extents. Each persisted record SHALL carry the exact target and range, topology epoch, profile ID, checksum-set generation, target content generation, digest, persistence evidence, verification generation, and state needed to validate that record without reconstructing omitted bindings from a current descriptor. A record missing any required binding or carrying an unsupported profile SHALL remain non-valid. Current baseline completeness SHALL require exactly one valid record for every expected current extent and reject missing, duplicate, extra, stale-topology, wrong-target, wrong-range, wrong-profile, wrong-set-generation, wrong-content-generation, unsupported, and mixed-set records.

#### Scenario: Data and parity have separate records

- **WHEN** a checksum baseline is built for data and P targets
- **THEN** records identify the target and extent independently and do not treat parity cleanliness as checksum coverage

#### Scenario: Extent boundaries are crossed

- **WHEN** a write spans multiple checksum extents
- **THEN** each affected extent is independently invalidated and revalidated under its own generation

#### Scenario: A descriptor changes after records were persisted

- **WHEN** the current baseline descriptor names a different profile, set generation, topology epoch, target mapping, range, or content generation than an existing record carried at creation
- **THEN** the existing record remains non-valid and cannot be retrospectively relabeled as current evidence

#### Scenario: Persisted records are mixed or duplicated

- **WHEN** records from different checksum sets coexist or more than one record claims the same expected extent binding
- **THEN** baseline completeness is invalid rather than complete

### Requirement: Validity is generation-bound
<!-- dwv:req req.checksum-plane.validity-is-generation-bound -->

A checksum record SHALL become `VALID` only through a durable recovery commit that names the digest profile/set, target content generation, and persistence evidence covering the exact target bytes. Missing, stale, unknown, unsupported, or mismatched evidence SHALL remain non-valid.

#### Scenario: Persistence evidence matches the generation

- **WHEN** a worker hashes the captured extent and the target generation and covering persistence evidence still match at commit
- **THEN** the recovery store may install the digest as `VALID`

#### Scenario: Persistence evidence is missing or stale

- **WHEN** persistence evidence is unavailable, does not cover the read bytes, or the target generation changed before commit
- **THEN** the result is rejected and the record remains stale or absent

### Requirement: Invalidation precedes data/parity write
<!-- dwv:req req.checksum-plane.invalidation-precedes-data-parity-write -->
<!-- dwv:refines req.dirty-integrity-invalidation.write-recovery-record-precedes-data-parity-write -->

For every data/parity write touching a `VALID` checksum extent, the checksum capability SHALL identify the affected extents and require their `VALID` to `STALE` transition through the owning dirty/integrity write-recovery-record protocol. This requirement owns only the checksum-state transition and its independence from parity `CLEAN` or `DIRTY` state; it does not redefine the owner's write-recovery-record or persistence-evidence predicate.

#### Scenario: Valid extent is modified

- **WHEN** a data/parity write touches a valid data or parity extent
- **THEN** the checksum transition is included in durable invalidation before the corresponding data/parity write

#### Scenario: Parity is clean with stale records

- **WHEN** parity is clean but checksum coverage is stale or absent
- **THEN** the system reports the two dimensions separately and never upgrades stale records from parity state alone

### Requirement: Revalidation rejects stale worker results
<!-- dwv:req req.checksum-plane.revalidation-rejects-stale-worker-results -->

Checksum jobs SHALL capture target identity, extent, profile/set generation, content generation, the exact bytes read, and applicable persistence evidence. A result that races with invalidation, topology/capability change, or another active set SHALL be rejected without changing current validity.

#### Scenario: Writer invalidates while hashing

- **WHEN** a writer changes the target generation after a hash job reads bytes but before the result commits
- **THEN** the worker result is discarded and the record remains stale for the new generation

#### Scenario: Two matching workers finish

- **WHEN** two jobs produce the same valid evidence for the current generation
- **THEN** one idempotent durable record may be committed and the other is a no-op, with identical semantic result

### Requirement: Full-overwrite hashing is equivalent to fenced readback
<!-- dwv:req req.checksum-plane.full-overwrite-hashing-is-equivalent-to-fenced-readback -->

A full-overwrite operation MAY compute a checksum from trusted final buffers without rereading the extent only when it covers the complete extent, the final bytes are covered by persistence evidence for the target, and the generation rules are satisfied. Otherwise revalidation SHALL read the target extent. `Fenced readback` remains the narrower store-mechanism comparison named by this requirement; it is not the claim-level evidence term.

#### Scenario: Complete trusted overwrite

- **WHEN** a full extent overwrite has final trusted bytes and covering persistence evidence
- **THEN** its committed digest equals a fenced readback digest for the same generation

#### Scenario: Partial or unproven overwrite

- **WHEN** the write covers only part of an extent or lacks covering persistence evidence
- **THEN** the optimization is not used and the record remains stale until a valid revalidation

### Requirement: Profile migration is explicit and interruptible
<!-- dwv:req req.checksum-plane.profile-migration-is-explicit-and-interruptible -->

Checksum profiles and active checksum sets SHALL have stable IDs. Migration SHALL build a parallel set and switch the active set atomically only after required records are valid; an interrupted or unsupported set SHALL NOT relabel or invalidate the old set’s evidence implicitly.

#### Scenario: Migration is interrupted

- **WHEN** the process stops while a new profile is being built
- **THEN** the prior active set remains interpretable and the incomplete set cannot authorize integrity decisions

#### Scenario: Unknown profile is encountered

- **WHEN** an inspector sees a record with an unsupported profile ID
- **THEN** it preserves the record as inspectable metadata but treats its integrity evidence as unavailable

### Requirement: Current baseline completion is persisted and exact
<!-- dwv:req req.checksum-plane.current-baseline-completion-is-persisted-and-exact -->
<!-- dwv:requires req.checksum-plane.checksum-coverage-names-targets-and-extents -->
<!-- dwv:requires req.checksum-plane.validity-is-generation-bound -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->

A checksum baseline SHALL identify its digest profile, checksum-set generation, topology epoch, complete required target and extent set, each target content generation, and each record's validity and persistence evidence in durable recovery state. A current baseline is complete only when every required data and parity extent appears exactly once with a supported active profile and set, matches the current topology and target identity, and carries current `VALID` evidence for the exact bytes. Missing, duplicate, stale, unknown, unsupported, wrong-topology, wrong-target, wrong-profile, wrong-set, invalid, or out-of-range records SHALL remain absent, partial, or invalid rather than complete. Reopen SHALL derive completion from the validated persisted semantic state; a persisted completion bit, process-local job success, parity cleanliness, or matching current digest alone SHALL NOT establish completeness.

#### Scenario: Baseline work is interrupted

- **WHEN** only a strict subset of required current extents has durably valid records before interruption
- **THEN** reopen reports the same baseline as partial and does not promote it to complete

#### Scenario: Complete current coverage reopens

- **WHEN** every required current target extent has one valid correctly bound persisted record and the semantic state reopens successfully
- **THEN** the checksum owner reconstructs a complete current baseline from those records

#### Scenario: Persisted evidence is no longer current

- **WHEN** a record names another topology, target, profile, set, content generation, unsupported digest, invalid persistence evidence, or duplicate extent
- **THEN** that record does not contribute to completion and the baseline remains partial or invalid

#### Scenario: A post-recovery baseline is calculated

- **WHEN** current bytes are read and committed as valid checksum evidence after metadata-loss recovery
- **THEN** the baseline may establish current coverage but SHALL remain labeled as newly calculated evidence rather than historical correctness proof

