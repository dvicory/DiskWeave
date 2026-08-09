## ADDED Requirements

### Requirement: Topology transition plans bind source, target, and recovery
<!-- dwv:req req.anchorless-topology-identity.topology-transition-plans-bind-source-target-and-recovery -->

Every prepared topology transition SHALL have a bounded plan bound to the current array identity, source topology epoch, expected assignment-instance identities and generations, exact source and target protected geometry and coding profiles, required quiescence or explicit reconciliation mode, recovery and rollback boundaries, and verification evidence required before promotion. The system SHALL revalidate those bindings before protected mutation and before promotion. A stale binding or unsupported execution mode SHALL be refused without changing transition state, protected bytes, or the active topology.

#### Scenario: An assignment changes after planning

- **WHEN** an expected assignment instance or generation no longer matches when the transition is about to mutate protected bytes
- **THEN** the transition is refused without mutation and the active topology remains authoritative

#### Scenario: A requested transition mode is unsupported

- **WHEN** the system cannot execute or verify the plan's required quiescence or reconciliation mode
- **THEN** it refuses the transition without changing transition state, protected bytes, or the active topology

## MODIFIED Requirements

### Requirement: Topology transitions are staged and recoverable
<!-- dwv:req req.anchorless-topology-identity.topology-transitions-are-staged-and-recoverable -->

Adding, removing, replacing, resizing, or role-changing a member, or changing a coding profile, parity-role count, coding position, or protected geometry, SHALL follow prepared, verified, committed, and published stages. The old active topology SHALL remain authoritative until the new generation is durably committed and verified. Target state SHALL be established and independently verified without overwriting or releasing the only assignments or parity bytes required by the old topology. The system SHALL NOT reinterpret existing parity bytes under a different profile, coding position, or protected geometry. Failure at any stage SHALL leave a deterministic recovery decision and SHALL not release the old assignment prematurely.

#### Scenario: A prepared replacement fails verification

- **WHEN** rebuild or complete verification fails under a prepared topology
- **THEN** the old topology remains active and the replacement is not published or treated as writable

#### Scenario: A data slot is added

- **WHEN** a plan introduces an identified data assignment and establishes target parity under its declared quiescence or reconciliation mode
- **THEN** the new topology is not committed until the target protected range and parity have been independently verified

#### Scenario: A data slot is removed

- **WHEN** a plan removes a data slot
- **THEN** it requires quiescence, evidence from the external retention owner that all retained user data has been removed, and independently verified parity under the target topology before commit

#### Scenario: Protected capacity changes

- **WHEN** a plan shrinks or increases protected capacity
- **THEN** shrink requires proof that no protected bytes exist beyond the target length, while increase requires parity-capacity validation and a verified target baseline before commit

#### Scenario: A coding-profile migration is interrupted before commit

- **WHEN** target parity has been partially or completely generated under a proposed coding profile but verification or durable commit has not completed
- **THEN** the source topology and its required parity remain authoritative and the target profile is not published or used to interpret existing parity bytes

#### Scenario: Topology commit succeeds but publication is interrupted

- **WHEN** durable recovery state records the new generation but frontend publication does not complete
- **THEN** restart reconciles to the committed generation before publishing a new request-visible epoch
