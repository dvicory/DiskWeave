## MODIFIED Requirements

### Requirement: Session transitions have ordered recovery semantics
<!-- dwv:req req.parity-envelope-profiles.session-transitions-have-ordered-recovery-semantics -->
<!-- dwv:refines req.recovery-state-semantics.durable-writable-session-lifecycle-binds-authority-and-close-evidence -->

The envelope protocol SHALL represent enough session state to distinguish prepared/active, dirty/unknown, prepared/closing, and clean/closed transitions while preserving the session identity, captured topology identity, and relevant generation bindings. A non-clean prepared or closing state MAY be durably represented before recovery `CLEAN`, but it SHALL authorize neither clean closure nor claim release. A clean/closed envelope state SHALL be publishable only after the required data/parity writes and persistence evidence have completed and recovery state `CLEAN` has been accepted, or in one atomic transition with that recovery `CLEAN` evidence and the durable close fact. The envelope SHALL own only representation, copy agreement, and conservative decoding; it SHALL NOT independently authorize a clean state, replace the recovery-session owner, or weaken its evidence gate.

#### Scenario: Dirty recovery state precedes data/parity writes

- **WHEN** a writable session is about to write data or parity
- **THEN** the durable recovery protocol records the dirty or indeterminate session state before the data/parity write

#### Scenario: Clean state follows durable completion

- **WHEN** required data/parity writes are complete, required persistence evidence has been accepted, and recovery state `CLEAN` has been committed
- **THEN** a matching envelope copy may record the resulting clean session state

#### Scenario: Crash occurs during transition

- **WHEN** the process or simulated media stops between any session transition, data/parity write, persistence-evidence observation, or recovery `CLEAN` commit
- **THEN** reopening does not infer clean state from an incomplete transition and reports the conservative recoverable state

#### Scenario: A non-clean closing state precedes recovery CLEAN

- **WHEN** the session has begun close processing but required persistence evidence or recovery `CLEAN` is not yet accepted
- **THEN** the envelope may record prepared or closing state, but it does not report clean/closed state or authorize claim release

#### Scenario: Clean/closed state follows durable completion

- **WHEN** required data/parity writes are complete, required persistence evidence has been accepted, recovery state `CLEAN` has been accepted for the exact closed mutation set, and the durable close fact is ordered after or atomically with that evidence
- **THEN** a matching envelope copy may record the resulting clean/closed session state

#### Scenario: A definite envelope transition is rejected

- **WHEN** an envelope transition is rejected by a known version, generation, topology, capacity, or evidence precondition
- **THEN** no new envelope state is authoritative and the exact prior interpretable state remains authoritative

#### Scenario: Envelope transition outcome is uncertain

- **WHEN** the process or simulated media stops, or the transition observation is lost or corrupt, between any session transition, data/parity write, persistence-evidence observation, or recovery `CLEAN` commit
- **THEN** reopening does not infer clean/closed state from the incomplete transition and reports the conservative recoverable state requiring reconciliation where applicable
