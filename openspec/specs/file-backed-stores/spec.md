# file-backed-stores Specification

## Purpose
This capability supplies fixed-geometry file-backed stores and disposable control state so the portable engine can run real macOS file-backed arrays without making Linux transports, SQLite layout, or host paths part of core semantics.
## Requirements
### Requirement: File-backed stores expose fixed exact-range semantics

The file store SHALL open a regular or sparse file at a fixed protected length, reject resize/truncate through the store API, and implement exact read, write, write-zeroes, and flush operations using the normalized store contract. Ranges outside geometry, overflow, invalid alignment, and missing buffers SHALL fail before media mutation.

#### Scenario: A sparse hole is read

- **WHEN** a valid read targets an unallocated region of a sparse file
- **THEN** the store returns zero bytes for the exact range and preserves apparent fixed length

#### Scenario: A request exceeds fixed geometry

- **WHEN** a write or read ends beyond the protected length
- **THEN** the store returns a range error without extending or mutating the file

### Requirement: File completion evidence is conservative

The store SHALL distinguish exact success, short, backend failure, uncertain, and duplicate completion. A successful write SHALL report only the evidence established by the requested sync/FUA contract. Host-file synchronization SHALL be named portable-demo evidence until the macOS bridge characterization proves its mapping.

#### Scenario: A write is followed by explicit sync

- **WHEN** the file write and requested synchronization both succeed
- **THEN** completion reports the exact range and the strongest portable evidence allowed by the configured profile

#### Scenario: Synchronization fails

- **WHEN** host synchronization returns an error or the process loses the operation result
- **THEN** the store reports failed or uncertain evidence and does not promote the write to durable

### Requirement: Capabilities and identity observations are probed, not invented

The store SHALL expose fixed geometry, alignment, transfer limits, write/flush support, sparse behavior, cancellation limitations, file identity observations, and evidence strength. Unknown physical cache, FUA, discard, and power-loss behavior SHALL remain unknown or unsupported.

#### Scenario: A file is opened on macOS APFS

- **WHEN** metadata and file identity are probed
- **THEN** the capability report includes logical length and file identity while physical durability claims remain gated

#### Scenario: File identity changes

- **WHEN** a path is replaced with a different file identity or geometry
- **THEN** comparison reports changed/ambiguous identity and active writable reuse is refused

### Requirement: Single-writer ownership and endpoint aliasing are explicit

The store layer SHALL acquire a bounded ephemeral lease before writable use, release it only after close, and reject a backing path that aliases an exported/proxy endpoint while active. Lease loss or conflicting ownership SHALL block writable assembly without modifying payload bytes.

#### Scenario: A second writer opens the same backing store

- **WHEN** the lease already exists
- **THEN** the second open fails deterministically and the existing payload remains untouched

#### Scenario: Backing and exported paths alias

- **WHEN** assembly resolves the same file identity for a backing payload and exported endpoint
- **THEN** assembly refuses the alias before serving the endpoint

### Requirement: Disposable control SQLite state is separate from recovery authority

The project SHALL provide a versioned control-state projection for inventory/history/job presentation that can be deleted and rebuilt from semantic topology and store observations. Control-state loss SHALL not make direct data unreadable and SHALL not authorize or invalidate recovery-state transitions.

#### Scenario: Control state is deleted

- **WHEN** `control.sqlite3` is removed while direct payloads and recovery authority remain
- **THEN** the system continues safety-critical semantics and rebuilds the projection on demand

#### Scenario: Control state is corrupt

- **WHEN** the projection fails integrity validation
- **THEN** it is quarantined/rebuilt without changing direct data or authoritative recovery state

### Requirement: File-backed evidence remains platform-scoped

The implementation SHALL identify which behavior is portable file semantics, macOS bridge evidence, Linux-only evidence, or hardware durability evidence. Passing file-store tests SHALL not certify APFS/DiskImages synchronization, physical flush/FUA, or Linux frontend conformance.

#### Scenario: The Linux frontend is unavailable

- **WHEN** the current host runs the file-backed store on macOS
- **THEN** portable and macOS evidence may be recorded while Linux-only criteria remain visibly unmet

