## Purpose

The normalized block contract gives every DiskWeave frontend one portable vocabulary for byte ranges, operations, ordering, durability intent, lifecycle events, capabilities, and terminal results without importing platform or runtime types.

## ADDED Requirements

### Requirement: Requests have validated frontend-neutral semantics

The system SHALL represent each block request with stable request and frontend identities, target slot, captured topology epoch, operation, byte range, optional generational buffer handle, submission sequence, ordering intent, and durability intent. Byte ranges SHALL use checked end arithmetic. The operation vocabulary SHALL support read, write, flush, write-zeroes, and discard while explicitly rejecting unsupported zoned operations.

#### Scenario: A valid aligned write is normalized

- **WHEN** a frontend submits a non-overflowing write with its target, epoch, buffer, sequence, and durability intent
- **THEN** normalization preserves every semantic field without an operating-system tag, pointer, runtime future, or database type

#### Scenario: A byte range overflows

- **WHEN** offset plus length cannot be represented safely
- **THEN** normalization returns a stable range error before backend admission

#### Scenario: A request has an invalid buffer relationship

- **WHEN** a read or write lacks the required generational buffer, or a flush supplies a data buffer
- **THEN** normalization rejects the request without submitting backend I/O

### Requirement: Ordering and durability intent cannot be silently weakened

The system SHALL preserve submission sequence, preflush intent, fence domain, ordinary-write intent, FUA intent, and explicit flush intent through normalization and adapter translation. An adapter MAY reject unsupported intent or emulate it only when equivalent capability evidence exists; it SHALL NOT silently discard or strengthen the intent and SHALL report only the persistence evidence actually established.

#### Scenario: FUA is supported

- **WHEN** a request carries FUA intent and the selected path has certified equivalent evidence
- **THEN** the adapter preserves the intent and reports completion with the corresponding evidence class

#### Scenario: FUA is not supported

- **WHEN** a request carries FUA intent and the selected path cannot prove equivalent behavior
- **THEN** the adapter returns deterministic unsupported-capability or selects an explicitly equivalent path without claiming stronger durability

#### Scenario: Preflush precedes a write

- **WHEN** a write carries preflush intent and a submission sequence
- **THEN** the normalized action order retains preflush before the write and preserves the same fence domain

### Requirement: Frontend lifecycle events have explicit abandonment semantics

The system SHALL represent frontend abandonment, quiescence, loss, recovery, and completion-interest state. Abandonment SHALL suppress completion delivery interest only; it SHALL NOT cancel or roll back an irreversible transaction. Loss SHALL preserve duplicate-delivery uncertainty, and quiescence SHALL identify the sequence drained without being treated as media durability.

#### Scenario: A request is abandoned after submission

- **WHEN** a frontend abandons an admitted request
- **THEN** the core suppresses delivery when appropriate but retains all backend lifetime and durability obligations

#### Scenario: A frontend is lost with duplicate delivery possible

- **WHEN** the adapter reports loss and possible duplicate delivery
- **THEN** the core records uncertainty and does not treat the event as cancellation or safe resource reclamation

#### Scenario: A frontend quiesces

- **WHEN** the adapter reports quiescence through sequence N
- **THEN** the core records N as ordering evidence without conflating it with persistence evidence

### Requirement: Adapters expose bounded deterministic conformance behavior

An adapter SHALL translate operations and flags without silent semantic changes, advertise only limits the complete path can satisfy, apply backpressure before unbounded allocation, retain frontend-owned resources until semantic reclamation is safe, and map terminal core results deterministically. Backing and exported endpoints SHALL never alias while active.

#### Scenario: Admission exceeds a declared bound

- **WHEN** request admission would exceed the configured queue, memory, buffer, tag, or operation bound
- **THEN** the adapter backpressures or returns deterministic resource exhaustion before creating unbounded work

#### Scenario: Discard is disabled

- **WHEN** a frontend submits discard under the initial discard-disabled profile
- **THEN** the adapter rejects it explicitly and does not translate it into a write, zero-fill, or silent success

#### Scenario: A frontend completion arrives

- **WHEN** the semantic operation reaches a terminal result
- **THEN** the adapter retains tags/resources until the operation lifetime contract permits reclamation and then delivers the mapped outcome unless completion interest was abandoned

### Requirement: Portable evidence does not imply platform certification

The normalized contract SHALL identify portable tests separately from macOS, Linux, device, power-loss, and hardware tests. Passing request/event tests SHALL not be reported as proof of physical flush, FUA, frontend, or production behavior.

#### Scenario: The current host lacks a platform adapter

- **WHEN** OS-001 is validated without ublk, FSKit, or a real store
- **THEN** portable contract evidence may pass while platform integration remains a visible successor gate

#### Scenario: A later adapter is introduced

- **WHEN** a Linux or macOS adapter translates flags and completions
- **THEN** it must pass this semantic conformance contract and retain separate platform evidence rather than changing the portable API
