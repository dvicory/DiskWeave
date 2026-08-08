# macos-demo-cli Specification

## Purpose
This capability defines the smallest experimental operator boundary needed to run and inspect a disposable macOS file-backed DiskWeave demo. It makes the first workflow scriptable without freezing the future production command tree, daemon protocol, or persistent data format.
## Requirements
### Requirement: The demo CLI has a versioned observable contract

The demo SHALL expose a `dwv` entrypoint with a documented command surface for fixture initialization, status, inspection, verification, rebuild planning, rebuild execution, and the bounded end-to-end demo workflow. Every command SHALL identify its command name and contract version in machine-readable output. Human-readable output SHALL describe the same semantic result rather than exposing a separate truth source.

#### Scenario: A supported command succeeds

- **WHEN** an operator invokes a supported command with valid fixture context and inputs
- **THEN** the command exits successfully and reports a versioned result containing the command identity, outcome, and bounded diagnostics

#### Scenario: An unsupported or blocked operation is requested

- **WHEN** a command cannot run because the capability, platform, evidence, or required daemon boundary is unavailable
- **THEN** it exits with a stable non-success class and reports an explicit unsupported, blocked, stale, ambiguous, or uncertain outcome without claiming success

#### Scenario: Output is requested in machine-readable mode

- **WHEN** the operator requests structured output
- **THEN** the result uses the documented versioned envelope, contains no payload bytes or private runtime handles, and is sufficient for a script to distinguish success from each refusal class

### Requirement: Demo fixture ownership is explicit

The demo SHALL operate on a bounded disposable fixture root containing an identifiable demo manifest and all referenced files. Initialization SHALL establish ownership and expected identities for the root, member payloads, parity payload, recovery state, replacement targets, and evidence. Commands SHALL resolve fixture references within that owned root and SHALL refuse missing, ambiguous, aliased, or ownership-inconsistent entries before payload mutation.

#### Scenario: A new disposable fixture is initialized

- **WHEN** the operator initializes a fixture at an unused root with supported geometry and size
- **THEN** the CLI creates the manifest and required ordinary files, reports their logical roles and identities, and leaves a reproducible fixture for later commands

#### Scenario: An existing fixture has an unexpected path or identity

- **WHEN** a referenced file is missing, replaced, aliased, outside the owned root, or has a mismatched identity or protected length
- **THEN** the command refuses the operation before mutating payload or recovery state and reports the required reconciliation

### Requirement: The offline demo exercises healthy and recoverable behavior

The first demo SHALL be runnable without a live macOS bridge or Linux frontend. Its documented workflow SHALL create or open a file-backed single-XOR array, exercise healthy reads, writes, and flushes, close and reopen the fixture, simulate one known missing data member, reconstruct an authorized read-only range, rebuild the missing member to a separate replacement target, resume from an interrupted checkpoint, complete final verification, and report source/parity preservation and replacement byte equality.

#### Scenario: The complete disposable workflow runs

- **WHEN** the operator runs the documented demo workflow against a valid fixture
- **THEN** healthy results, degraded-read results, rebuild progress, final verification, reopen evidence, source/parity preservation, and replacement equality are all reported with no live bridge or Linux claim

#### Scenario: The workflow is interrupted after a checkpoint

- **WHEN** the rebuild process stops after a durable checkpoint and is run again with matching fixture identities and generations
- **THEN** it resumes from the first unprocessed range, preserves earlier replacement bytes, and reports the resumed and final verification evidence

#### Scenario: The replacement target is not distinct

- **WHEN** a rebuild target aliases a source or parity file or does not have the expected replacement identity
- **THEN** rebuild planning or execution refuses before the first replacement payload write

### Requirement: Inspection and verification are read-only

Status, state explanation, member/identity evidence, capability evidence, inspection, and exhaustive verification commands SHALL be usable without a daemon for the file-backed demo. These commands SHALL not alter payload bytes, parity bytes, topology, recovery generations, or integrity evidence. Sampled checks MAY report diagnostic confidence but SHALL NOT certify clean state.

#### Scenario: A healthy fixture is inspected

- **WHEN** the operator requests status, identity evidence, inspection, or exhaustive verification for a valid fixture
- **THEN** the CLI reports lifecycle, topology/recovery identity, geometry, parity/integrity disposition, and evidence boundaries without changing the fixture

#### Scenario: Verification cannot establish a clean result

- **WHEN** a check is sampled, incomplete, ambiguous, or encounters a mismatch without unique independent evidence
- **THEN** the CLI reports the bounded diagnostic or refusal and does not promote clean state or perform automatic repair

### Requirement: State-changing workflows require an identity-bound plan

Every state-changing demo workflow SHALL first produce a plan containing the expected fixture/member identities, topology and recovery generations, protected ranges, evidence used, persistent-state impact, and rollback limits. Execution SHALL require a confirmation value that matches the displayed plan and SHALL revalidate the plan before any irreversible payload mutation. A plan or confirmation mismatch SHALL preserve the fixture and recovery state.

#### Scenario: A rebuild plan is confirmed

- **WHEN** the operator supplies a plan produced for the current fixture and the matching confirmation value
- **THEN** the CLI revalidates the identities and generations, executes only the planned separate-target rebuild, and reports the receipts and final outcome

#### Scenario: A plan is stale or altered

- **WHEN** the fixture, topology, recovery generation, ranges, replacement identity, or confirmation value differs from the displayed plan
- **THEN** execution refuses before payload mutation and reports the stale or mismatched plan

### Requirement: The CLI remains a replaceable boundary over portable semantics

The demo CLI SHALL expose portable service, verification, recovery, and store behavior without making command handlers the authority for parity equations, recovery eligibility, topology transitions, durability ordering, or identity assessment. The offline demo SHALL not require Linux frontend types, an async runtime, a live bridge, or filesystem metadata embedded in data payloads. Any future daemon interaction SHALL remain behind a versioned replaceable local boundary.

#### Scenario: Portable behavior is exercised without Linux

- **WHEN** the demo runs on macOS without a Linux frontend
- **THEN** healthy file-backed operation, verification, degraded reconstruction, rebuild, interruption/resume, and evidence reporting remain runnable without a Linux conformance claim

#### Scenario: A future daemon is unavailable

- **WHEN** an operation is marked offline-capable
- **THEN** the CLI runs it directly through the portable/file-backed boundary; daemon-required behavior reports a distinct unavailable boundary rather than silently substituting a different semantic path

