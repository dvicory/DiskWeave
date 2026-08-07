# volatile-media-simulator Specification

## Purpose
The volatile-media simulator is a deterministic oracle for separating acknowledged volatile effects, durable bytes, pending backend work, and completion delivery. It makes every selected fault schedule reproducible without depending on an operating system, storage device, database, or async runtime.

The simulator is the handoff's bounded Phase 0 fault model. Recovery-state and parity-envelope objects are semantic fixtures, not SQLite pages or a selected persistent format.
## Requirements
### Requirement: Media state separates durable and process-visible effects

The simulator SHALL maintain durable media, acknowledged volatile writes, pending operations, completion delivery, a configured fault model, and store availability as distinct state. A daemon crash SHALL discard process-owned pending operations and undelivered completions while preserving device volatile state. A power loss SHALL resolve pending volatile writes according to the configured deterministic policy and SHALL discard unresolved volatile state that the policy does not persist.

#### Scenario: An ordinary write is acknowledged before a flush

- **WHEN** a write is delivered successfully without durable intent
- **THEN** its bytes become visible in the simulator's acknowledged volatile state but are not durable until a modeled flush or persistence rule commits them

#### Scenario: The daemon crashes with volatile writes present

- **WHEN** a daemon-crash action occurs after an acknowledged ordinary write
- **THEN** pending process state and undelivered completions disappear while the configured volatile media remains available for a later power-loss decision

#### Scenario: Power loss discards volatile state

- **WHEN** power loss is applied under a discard-volatile policy
- **THEN** durable bytes remain unchanged by unflushed writes and volatile state is empty after recovery

### Requirement: Operations use exact normalized ranges and structured evidence

Simulator submissions SHALL use exact byte ranges, generation-bearing child operation identities, and the OS-002 completion dispositions and persistence evidence. Short completion SHALL expose only the completed subset. Failed and uncertain outcomes SHALL preserve the evidence needed to decide whether retry is legal; the simulator SHALL not silently turn them into success.

#### Scenario: A short write is delivered

- **WHEN** the fault model completes only a prefix of a write
- **THEN** the completion reports that exact prefix with `Short` disposition and the media effect is limited to the same modeled prefix

#### Scenario: A completion is uncertain

- **WHEN** the configured fault model leaves the media effect unknown
- **THEN** the completion reports `Uncertain` and volatile or durable evidence is not promoted to a stronger claim merely because the call returned

### Requirement: Schedules are deterministic, serializable, and minimizable

The simulator SHALL execute a finite ordered schedule of submissions, completion deliveries, crashes, disappearance/reappearance, and power-loss actions. The same serialized schedule and configuration SHALL produce the same durable snapshot and delivery trace. A minimizer SHALL remove schedule steps greedily while preserving a caller-supplied failure predicate, and minimized schedules SHALL be serializable regression fixtures.

#### Scenario: Completion order is permuted

- **WHEN** two pending completions are delivered in either order
- **THEN** the resulting trace records the chosen order explicitly and the durable media follows the configured effects rather than an implicit queue order

#### Scenario: A failing schedule is minimized

- **WHEN** a predicate identifies an invariant failure for a schedule
- **THEN** the minimizer returns a no-longer-reducible schedule that still satisfies the predicate and round-trips through the reproducer format

### Requirement: The simulator exposes conservative fault boundaries

The simulator SHALL model read, write, flush, FUA-like write, short, backend failure, uncertain completion, duplicate delivery, store disappearance/reappearance, daemon crash, and power loss. It SHALL expose durable snapshots and delivery traces for invariant checkers. It SHALL not claim physical power-loss certification or infer a durable write from an ordinary completion.

#### Scenario: A store disappears before submission

- **WHEN** a submission occurs while the store is unavailable
- **THEN** the simulator rejects the submission and does not mutate durable or volatile media

#### Scenario: A duplicate completion is delivered

- **WHEN** a previously delivered completion is replayed
- **THEN** the trace records a duplicate delivery without mutating media a second time

### Requirement: Core safety invariants are executable

The simulator SHALL provide checks for no durable mutation from discarded volatile writes, exact range bounds, no successful incomplete completion, and stable serialized replay. A later transaction OpenSpec MAY add dirty-region, integrity, and parity-envelope invariants; OS-004 SHALL keep those concerns outside this media primitive.

#### Scenario: A power-loss schedule is replayed

- **WHEN** the same schedule is run twice from the same initial image and configuration
- **THEN** durable bytes, volatile bytes, pending-state count, and delivery trace are identical

### Requirement: Destructive range operations and torn media effects are explicit

The simulator SHALL model write-zeroes and discard as distinct normalized store operations. The deterministic simulator profile SHALL represent discard as a logical zeroing effect and SHALL not claim physical deallocation. Short, torn, failed, and uncertain effects SHALL remain distinguishable in delivery evidence; a torn effect SHALL never be reported as a complete successful operation.

#### Scenario: Write-zeroes is delivered

- **WHEN** a write-zeroes request is delivered successfully
- **THEN** exactly its requested range becomes zero according to its durability intent and the trace identifies it as `WriteZeroes`

#### Scenario: Discard is delivered

- **WHEN** a discard request is delivered successfully
- **THEN** the deterministic logical read view returns zeroes for exactly that range and the simulator makes no physical deallocation claim

#### Scenario: A torn write is delivered

- **WHEN** the fault model applies a torn prefix
- **THEN** only that modeled prefix may change, completion evidence is uncertain, and the trace records the torn fault

### Requirement: Independent recovery-state and parity-envelope faults are modeled

The simulator SHALL maintain durable recovery-state bytes and at least two independently addressable parity-envelope copy fixtures. Recovery-state commit failure SHALL not advance the committed generation optimistically. Uncertain or torn commits SHALL be visible in the snapshot. A torn envelope commit SHALL invalidate the affected copy and SHALL not silently make the copy authoritative.

#### Scenario: Recovery-state commit fails

- **WHEN** a recovery-state commit is delivered with a backend failure
- **THEN** the prior committed generation and bytes remain authoritative and the trace records the failure

#### Scenario: A parity-envelope copy tears

- **WHEN** an envelope commit tears while updating one copy
- **THEN** only that copy is marked invalid/uncertain and the other copy remains unchanged

### Requirement: Controller reset and latent corruption are separate transitions

The simulator SHALL model controller reset separately from daemon crash and power loss. Controller reset SHALL discard pending controller work while preserving durable media and acknowledged volatile state. A latent-corruption action SHALL mutate exactly the requested durable range using a deterministic mask and SHALL be visible in the trace.

#### Scenario: Controller reset interrupts pending work

- **WHEN** controller reset occurs with pending operations
- **THEN** pending controller work is dropped without a completion and durable/volatile media remains otherwise unchanged

#### Scenario: Latent corruption is injected

- **WHEN** a deterministic corruption mask is applied
- **THEN** only the selected durable bytes change and a subsequent read can observe the corruption

### Requirement: Bounded schedule coverage is deterministic

The simulator SHALL expose a finite one-range schedule enumerator covering the new operation and fault transitions. Enumeration SHALL be deterministic and SHALL enforce an explicit maximum schedule count. Every emitted schedule SHALL be serializable and replayable through the existing reproducer format.

#### Scenario: Bounded schedules are replayed

- **WHEN** all schedules under a small configured bound are replayed from the same image and configuration
- **THEN** each schedule has identical traces across two runs and no accepted schedule violates the core invariants

