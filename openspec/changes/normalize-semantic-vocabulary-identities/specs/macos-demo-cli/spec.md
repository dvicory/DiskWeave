## MODIFIED Requirements

### Requirement: Demo fixture ownership is explicit
<!-- dwv:req req.macos-demo-cli.demo-fixture-ownership-is-explicit -->

The demo SHALL operate on a bounded disposable fixture root containing an identifiable demo manifest and all referenced files. Initialization SHALL establish ownership and expected identities for the root, member payloads, parity payload, recovery state, replacement targets, and evidence. Commands SHALL resolve fixture references within that owned root and SHALL refuse missing, ambiguous, aliased, or ownership-inconsistent entries before changing data, parity, or recovery state.

#### Scenario: A new disposable fixture is initialized

- **WHEN** the operator initializes a fixture at an unused root with supported geometry and size
- **THEN** the CLI creates the manifest and required ordinary files, reports their logical roles and identities, and leaves a reproducible fixture for later commands

#### Scenario: An existing fixture has an unexpected path or identity

- **WHEN** a referenced file is missing, replaced, aliased, outside the owned root, or has a mismatched identity or protected length
- **THEN** the command refuses the operation before changing data, parity, or recovery state and reports the required reconciliation

### Requirement: State-changing workflows require an identity-bound plan
<!-- dwv:req req.macos-demo-cli.state-changing-workflows-require-an-identity-bound-plan -->

Every state-changing demo workflow SHALL first produce a plan containing the expected fixture/member identities, topology and recovery generations, protected ranges, evidence used, persistent-state impact, and rollback limits. Execution SHALL require a confirmation value that matches the displayed plan and SHALL revalidate the plan before any irreversible data/parity or recovery-state change. A plan or confirmation mismatch SHALL preserve the fixture and recovery state.

#### Scenario: A rebuild plan is confirmed

- **WHEN** the operator supplies a plan produced for the current fixture and the matching confirmation value
- **THEN** the CLI revalidates the identities and generations, executes only the planned separate-target rebuild, and reports the receipts and final outcome

#### Scenario: A plan is stale or altered

- **WHEN** the fixture, topology, recovery generation, ranges, replacement identity, or confirmation value differs from the displayed plan
- **THEN** execution refuses before changing data, parity, or recovery state and reports the stale or mismatched plan
