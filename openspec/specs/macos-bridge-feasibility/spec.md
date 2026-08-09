# macos-bridge-feasibility Specification

## Purpose
This capability determines whether a macOS filesystem/virtual-disk bridge can expose fixed-size seekable proxy files to APFS while translating operations into DiskWeave’s normalized API and preserving explicit synchronization and failure evidence.
## Requirements
### Requirement: The probe uses a fixed-size seekable proxy
<!-- dwv:req req.macos-bridge-feasibility.the-probe-uses-a-fixed-size-seekable-proxy -->

The macOS feasibility harness SHALL create or expose a proxy endpoint with stable size, bounded read/write ranges, explicit deny-truncate behavior, and no required DiskWeave metadata in the backing data files. It SHALL record every operation needed to compare candidates.

#### Scenario: Proxy geometry is queried

- **WHEN** the candidate bridge is opened by a virtual-disk consumer
- **THEN** it reports the expected fixed logical size and accepts only in-range seekable operations

#### Scenario: Truncate or hole-punch is attempted

- **WHEN** the consumer requests resize, truncate, or unsupported hole mutation
- **THEN** the proxy rejects it explicitly without changing logical geometry

### Requirement: Candidate bridges are compared at the normalized boundary
<!-- dwv:req req.macos-bridge-feasibility.candidate-bridges-are-compared-at-the-normalized-boundary -->

The probe SHALL evaluate FSKit first, macFUSE as an alternative, and DiskImages as the virtual-disk attachment layer without embedding candidate-specific types into portable core semantics. The result SHALL identify which candidate, if any, can translate proxy operations to normalized requests.

#### Scenario: Candidate emits block I/O

- **WHEN** the attached proxy receives reads, writes, flush/sync, close, and metadata operations
- **THEN** the harness records a normalized trace with ranges, ordering, completion, and persistence evidence

#### Scenario: Candidate cannot satisfy the contract

- **WHEN** a candidate cannot provide fixed geometry, seekable I/O, required synchronization, or usable disconnect behavior
- **THEN** the ADR documents the exact blocker and leaves the portable core/portable-demo path unchanged

### Requirement: Synchronization and cache behavior are evidenced
<!-- dwv:req req.macos-bridge-feasibility.synchronization-and-cache-behavior-are-evidenced -->

The probe SHALL distinguish operation arrival, host-file write completion, synchronization/fsync behavior, close/detach, and backend failure. It SHALL not infer durable hardware semantics from a successful API return or elapsed time.

#### Scenario: Sync-heavy workload runs

- **WHEN** the proxy receives interleaved writes, flush/sync, mmap/page-cache activity where available, and close
- **THEN** the trace shows the mapping to host-file synchronization and declares only the evidence the candidate actually provides

#### Scenario: Backing member disappears

- **WHEN** a backing file is removed or becomes unavailable while the proxy remains open
- **THEN** the bridge reports the disconnect/failure explicitly and does not fabricate successful completions or hide recovery state

### Requirement: Backing and exported endpoints cannot alias
<!-- dwv:req req.macos-bridge-feasibility.backing-and-exported-endpoints-cannot-alias -->

The harness SHALL use separate backing paths and exported proxy paths, record file identity/size/copy behavior, and reject configurations where an active exported endpoint can directly bypass the normalized service to the backing member.

#### Scenario: Distinct paths are assembled

- **WHEN** backing files and proxy endpoints have distinct stable identities
- **THEN** the probe accepts the layout and records the identity evidence for later assembly

#### Scenario: Direct backing attachment is attempted

- **WHEN** an active backing file is proposed as an exported endpoint or direct writable attachment
- **THEN** the configuration is rejected before concurrent access can bypass parity/recovery semantics

### Requirement: Installation and platform constraints are explicit
<!-- dwv:req req.macos-bridge-feasibility.installation-and-platform-constraints-are-explicit -->

The evidence bundle SHALL record macOS version, SDK/framework availability, entitlements/signing/automation requirements, installation steps, minimum supported version, and whether the test was run with required privileges. A restricted development session SHALL be reported as an evidence limitation rather than silently treated as success.

#### Scenario: Framework/toolchain is available

- **WHEN** the host exposes the candidate SDK/module/framework and required command-line tools
- **THEN** the probe records versions and proceeds to functional tests

#### Scenario: Attachment is blocked by environment

- **WHEN** the host sandbox or missing entitlement prevents DiskImages/FSKit attachment
- **THEN** the result distinguishes environment restriction from a semantic bridge failure and preserves a reproducible manual test plan

### Requirement: The outcome is a bounded portable-demo decision
<!-- dwv:req req.macos-bridge-feasibility.the-outcome-is-a-bounded-portable-demo-decision -->

The ADR SHALL select a candidate only for the evidence-backed macOS functional reference, or document a blocker and narrow the live goal. It SHALL state that process kill/restart, detach, host-file synchronization, and simulator schedules are covered while physical power loss, controller cache, production FUA, and Linux frontend semantics are not certified.

#### Scenario: Candidate passes the required matrix

- **WHEN** fixed geometry, block trace, synchronization, failure, identity, and separation tests pass
- **THEN** the candidate is selected behind the normalized frontend seam for OS-021

#### Scenario: No candidate passes

- **WHEN** all candidates fail a required coherence or synchronization criterion
- **THEN** the ADR compares DriverKit/SCSI or a narrowed live reference and does not force an unsafe bridge into the product

