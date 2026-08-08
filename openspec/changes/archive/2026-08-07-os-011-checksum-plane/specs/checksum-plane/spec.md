## Purpose

This capability provides independent generational checksum evidence for data and parity extents, with asynchronous revalidation that cannot install a digest for different bytes or unsupported durability evidence.

## ADDED Requirements

### Requirement: Checksum coverage names targets and extents

The checksum plane SHALL identify each data, P, and optional Q target by stable semantic identity and map it to explicit bounded checksum extents. Each record SHALL carry a profile ID, checksum-set generation, target content generation, extent range, digest, and state.

#### Scenario: Data and parity have separate records

- **WHEN** a checksum baseline is built for data and P targets
- **THEN** records identify the target and extent independently and do not treat parity cleanliness as checksum coverage

#### Scenario: Extent boundaries are crossed

- **WHEN** a write spans multiple checksum extents
- **THEN** each affected extent is independently invalidated and revalidated under its own generation

### Requirement: Validity is generation-bound

A checksum record SHALL become `VALID` only through a durable recovery commit that names the digest profile/set, target content generation, and fence evidence covering the exact target bytes. Missing, stale, unknown, or unsupported evidence SHALL remain non-valid.

#### Scenario: Fenced generation matches

- **WHEN** a worker hashes the captured extent and the target generation and durable fence still match at commit
- **THEN** the recovery store may install the digest as `VALID`

#### Scenario: Volatile or changed evidence

- **WHEN** the read was volatile/unfenced or the target generation changed before commit
- **THEN** the result is rejected and the record remains stale/absent

### Requirement: Invalidation precedes protected mutation

Every write that touches a `VALID` checksum extent SHALL use the dirty/integrity invalidation protocol to durably mark that extent `STALE` before home mutation. Checksum validity SHALL remain independent from parity `CLEAN`/`DIRTY` state.

#### Scenario: Valid extent is modified

- **WHEN** a protected write touches a valid data or parity extent
- **THEN** OS-010 intent durably invalidates the record before the corresponding home mutation

#### Scenario: Parity is clean with stale records

- **WHEN** parity is clean but checksum coverage is stale or absent
- **THEN** the system reports the two dimensions separately and never upgrades stale records from parity state alone

### Requirement: Revalidation rejects stale worker results

Checksum jobs SHALL capture target identity, extent, profile/set generation, content generation, and read/fence evidence. A result that races with invalidation, topology/capability change, or another active set SHALL be rejected without changing current validity.

#### Scenario: Writer invalidates while hashing

- **WHEN** a writer changes the target generation after a hash job reads bytes but before the result commits
- **THEN** the worker result is discarded and the record remains stale for the new generation

#### Scenario: Two matching workers finish

- **WHEN** two jobs produce the same valid evidence for the current generation
- **THEN** one idempotent durable record may be committed and the other is a no-op, with identical semantic result

### Requirement: Full-overwrite hashing is equivalent to fenced readback

A full-overwrite operation MAY compute a checksum from trusted final buffers without rereading the extent only when it covers the complete extent, the final bytes are the bytes durably fenced to the target, and the generation rules are satisfied. Otherwise revalidation SHALL read the target extent.

#### Scenario: Complete trusted overwrite

- **WHEN** a full extent overwrite has final trusted bytes and covering fence evidence
- **THEN** its committed digest equals a fenced readback digest for the same generation

#### Scenario: Partial or volatile overwrite

- **WHEN** the write covers only part of an extent or lacks durable fence evidence
- **THEN** the optimization is not used and the record remains stale until a valid revalidation

### Requirement: Profile migration is explicit and interruptible

Checksum profiles and active checksum sets SHALL have stable IDs. Migration SHALL build a parallel set and switch the active set atomically only after required records are valid; an interrupted or unsupported set SHALL NOT relabel or invalidate the old set’s evidence implicitly.

#### Scenario: Migration is interrupted

- **WHEN** the process stops while a new profile is being built
- **THEN** the prior active set remains interpretable and the incomplete set cannot authorize integrity decisions

#### Scenario: Unknown profile is encountered

- **WHEN** an inspector sees a record with an unsupported profile ID
- **THEN** it preserves the record as inspectable metadata but treats its integrity evidence as unavailable
