## MODIFIED Requirements

### Requirement: Session transitions have ordered recovery semantics
<!-- dwv:req req.parity-envelope-profiles.session-transitions-have-ordered-recovery-semantics -->

The envelope protocol SHALL represent enough session state to distinguish prepared/active, dirty/unknown, and clean/closed transitions. A clean state SHALL be publishable only after the required data/parity writes, persistence evidence, and recovery state `CLEAN` have completed according to the portable protocol. The envelope SHALL NOT independently authorize a clean state before its evidence gate is proven.

#### Scenario: Dirty state precedes protected mutation

- **WHEN** a writable session is about to write data or parity
- **THEN** the durable recovery protocol records the dirty or indeterminate session state before the data/parity write

#### Scenario: Clean state follows durable completion

- **WHEN** required data/parity writes are complete, required persistence evidence has been accepted, and recovery state `CLEAN` has been committed
- **THEN** a matching envelope copy may record the resulting clean session state

#### Scenario: Crash occurs during transition

- **WHEN** the process or simulated media stops between any session transition, data/parity write, persistence-evidence observation, or recovery `CLEAN` commit
- **THEN** reopening does not infer clean state from an incomplete transition and reports the conservative recoverable state
