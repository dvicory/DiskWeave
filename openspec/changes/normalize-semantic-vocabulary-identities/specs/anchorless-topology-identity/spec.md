## MODIFIED Requirements

### Requirement: Topology transition plans bind source, target, and recovery
<!-- dwv:req req.anchorless-topology-identity.topology-transition-plans-bind-source-target-and-recovery -->

Every prepared topology transition SHALL have a bounded plan bound to the current array identity, source topology epoch, expected assignment-instance identities and generations, exact source and target protected geometry and coding profiles, required quiescence or explicit reconciliation mode, recovery and rollback boundaries, and verification evidence required before promotion. The system SHALL revalidate those bindings before changing protected data/parity state and before promotion. A stale binding or unsupported execution mode SHALL be refused without changing transition state, protected bytes, or the active topology.

#### Scenario: An assignment changes after planning

- **WHEN** an expected assignment instance or generation no longer matches when the transition is about to change protected data/parity state
- **THEN** the transition is refused without changing protected data/parity state and the active topology remains authoritative

#### Scenario: A requested transition mode is unsupported

- **WHEN** the system cannot execute or verify the plan's required quiescence or reconciliation mode
- **THEN** it refuses the transition without changing transition state, protected bytes, or the active topology
