## MODIFIED Requirements

### Requirement: The demo CLI has a versioned observable contract
<!-- dwv:req req.macos-demo-cli.the-demo-cli-has-a-versioned-observable-contract -->
<!-- dwv:requires req.architecture-contract.operator-result-projections-preserve-consequential-meaning -->

The demo SHALL expose a `dwv` entrypoint with a documented command surface for fixture initialization, status, inspection, verification, rebuild planning, rebuild execution, and the bounded end-to-end demo workflow. Every command SHALL identify its command name and contract version in machine-readable output. Human-readable output SHALL preserve command identity, outcome, and bounded diagnostics required by the applicable command contract.

#### Scenario: A supported command succeeds

- **WHEN** an operator invokes a supported command with valid fixture context and inputs
- **THEN** the command exits successfully and reports a versioned result containing the command identity, outcome, and bounded diagnostics

#### Scenario: An unsupported or blocked operation is requested

- **WHEN** a command cannot run because the capability, platform, evidence, or required daemon boundary is unavailable
- **THEN** it exits with a stable non-success class and reports an explicit unsupported, blocked, stale, ambiguous, or uncertain outcome without claiming success

#### Scenario: Output is requested in machine-readable mode

- **WHEN** the operator requests structured output
- **THEN** the result uses the documented versioned envelope, contains no payload bytes or private runtime handles, and is sufficient for a script to distinguish success from each refusal class
