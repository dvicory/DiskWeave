## MODIFIED Requirements

### Requirement: Read-only inspection and mutation authority are separate
<!-- dwv:req req.security-boundaries.read-only-inspection-and-mutation-authority-are-separate -->

Inspection, verification, planning, and evidence export SHALL be usable without mutation authority. Destructive repair, topology changes, rebaseline, format migration, and external-write release SHALL require an identity- and generation-bound plan with explicit confirmation and revalidation before the protected state change.

#### Scenario: An inspection command runs
- **WHEN** an operator requests status, identity evidence, capability evidence, inspection, or verification
- **THEN** the command does not alter payload bytes, parity, topology, recovery generations, or integrity evidence

#### Scenario: A state-changing plan is stale
- **WHEN** a plan's identity, range, generation, or evidence no longer matches the observed state
- **THEN** execution is rejected before the protected state change
