## Purpose

This capability defines experimental parity-device metadata envelopes and
independent inspection so DiskWeave can evaluate crash-safe recovery metadata
without embedding metadata in ordinary data payloads or freezing a stable format.

## ADDED Requirements

### Requirement: Envelope profiles remain experimental and capacity-safe

The system SHALL compare the bare parity, redundant-envelope, and
redundant-envelope-plus-bitmap profiles as explicitly experimental profiles.
Every profile SHALL report the usable protected parity capacity and SHALL reject
any configuration where protected data capacity exceeds usable parity payload
capacity.

#### Scenario: Profile comparison reports capacity

- **WHEN** an operator evaluates all supported experimental profiles for a
  parity payload and protected geometry
- **THEN** each result identifies its profile, metadata overhead, usable parity
  capacity, and maximum protected data capacity

#### Scenario: Metadata overhead would reduce protection

- **WHEN** a profile's metadata reservation leaves less usable parity capacity
  than the configured protected data range
- **THEN** the profile is rejected before any payload or envelope mutation

#### Scenario: Data payload remains conventional

- **WHEN** an experimental envelope is created or inspected
- **THEN** no required envelope bytes, headers, trailers, sidecars, or hidden
  reservations are added to an ordinary data member

### Requirement: Envelope copies are independently inspectable

The system SHALL provide an independent bounded decoder that can inspect an
 envelope without relying on the writer's in-memory representation. The decoder
 SHALL validate version, profile, bounds, generation, topology identity, exact
 payload coverage, checksum/integrity fields, and required feature flags before
 treating a copy as usable evidence.

#### Scenario: Valid copy is decoded

- **WHEN** an envelope copy has supported version, bounded lengths, valid
  integrity, and internally consistent generation and topology fields
- **THEN** the decoder returns its semantic metadata and marks the copy usable

#### Scenario: Unknown required feature is encountered

- **WHEN** a copy declares a required feature that the decoder does not support
- **THEN** the decoder preserves the copy as inspectable but marks it unusable
  and does not select it for recovery

#### Scenario: Malformed or oversized input is inspected

- **WHEN** an envelope has invalid lengths, truncated fields, overflowed bounds,
  or exceeds configured decode limits
- **THEN** inspection fails with a bounded format error and allocates no
  unbounded buffer

### Requirement: Envelope disagreement resolves conservatively

The system SHALL evaluate multiple envelope copies by generation, topology,
profile, integrity, and session state. Missing, torn, stale, cloned,
conflicting, or disagreeing copies SHALL resolve to `DIRTY` or `UNKNOWN` rather
than optimistic clean evidence.

#### Scenario: Matching current copies are available

- **WHEN** all required copies agree on the current topology, profile,
  generation, and session state and each copy passes integrity validation
- **THEN** the inspector reports the shared semantic state and its evidence
  strength

#### Scenario: Copies disagree

- **WHEN** valid-looking copies disagree on generation, topology, profile, or
  session state
- **THEN** the inspector reports a conflict and does not select a clean state
  or authorize a recovery shortcut

#### Scenario: A required copy is missing or torn

- **WHEN** a required envelope copy is absent or fails integrity validation
- **THEN** the result is `UNKNOWN` or `DIRTY` and the missing evidence is
  reported explicitly

### Requirement: Session transitions have ordered recovery semantics

The envelope protocol SHALL represent enough session state to distinguish
prepared/active, dirty/unknown, and clean/closed transitions. A clean state
SHALL be publishable only after the required protected writes, durability
fences, and recovery-state checkpoint have completed according to the portable
protocol. The envelope SHALL NOT independently authorize a clean state before
its evidence gate is proven.

#### Scenario: Dirty state precedes protected mutation

- **WHEN** a writable session is about to mutate protected data or parity
- **THEN** the durable recovery protocol records the dirty or indeterminate
  session state before home-media mutation

#### Scenario: Clean state follows durable completion

- **WHEN** protected writes, required store fences, and the recovery checkpoint
  all complete durably
- **THEN** a matching envelope copy may record the resulting clean session state

#### Scenario: Crash occurs during transition

- **WHEN** the process or simulated media stops between any session transition,
  protected mutation, fence, or checkpoint
- **THEN** reopening does not infer clean state from an incomplete transition
  and reports the conservative recoverable state

### Requirement: Envelope migration is bounded and interruptible

The system SHALL treat profile or envelope-generation migration as a separate
experimental operation. Migration SHALL preserve the previously interpretable
profile until the new profile has complete valid evidence and an atomic
selection decision.

#### Scenario: Migration completes

- **WHEN** every required migrated envelope copy is valid and agrees on the new
  profile and generation
- **THEN** the new profile can be selected and the prior profile remains
  inspectable as migration history

#### Scenario: Migration is interrupted

- **WHEN** migration stops before all required new copies are valid
- **THEN** the prior profile remains the active interpretable profile and the
  incomplete profile cannot authorize recovery

#### Scenario: Migration input is unsupported

- **WHEN** a requested profile or feature cannot be decoded or validated
- **THEN** migration refuses without relabeling existing evidence or mutating
  protected data payloads
