# healthy-portable-io Specification

## Purpose
This capability provides the first end-to-end portable single-parity path: normalized requests are admitted through stable topology and operation slots, executed against ordinary file-backed members, and completed only with the recovery and integrity evidence required by the handoff.
## Requirements
### Requirement: Assembly and request admission are bounded and identity-safe

The service SHALL assemble only an unambiguous stable topology with compatible capabilities, reject backing/export aliases, and admit requests through generational operation slots. It SHALL reject stale topology or slot generations before child I/O.

#### Scenario: Healthy topology is assembled

- **WHEN** all required data/parity roles have unambiguous identity and compatible capabilities
- **THEN** the service enters serving state and accepts normalized requests with a captured topology epoch

#### Scenario: Ambiguous or stale assembly is attempted

- **WHEN** identity evidence is ambiguous, a required role is unavailable, or a captured generation is stale
- **THEN** assembly/request admission fails closed without mutating a member

### Requirement: Healthy reads preserve exact-range evidence

A read SHALL validate the normalized byte range, split it at required boundaries, read the selected data member through the store contract, and return exact completed-range/disposition/persistence evidence. It SHALL not fabricate bytes for short or uncertain reads.

#### Scenario: Aligned healthy read

- **WHEN** a request reads an available data range and the store returns complete bytes
- **THEN** the service returns the exact requested bytes and structured successful completion

#### Scenario: Short or uncertain read

- **WHEN** a child read completes short or with unknown media effect
- **THEN** the service returns structured partial/uncertain evidence and does not report a full successful read

### Requirement: Writes follow the reference transaction and update single XOR parity

A protected write SHALL use OS-008/OS-010 ordering: acquire resources, durably record dirty/invalidation intent, read or use trusted full-overwrite data, compute/update single XOR parity, and write affected data/parity ranges. No protected home mutation may precede intent.

#### Scenario: Partial write requires RMW

- **WHEN** a write covers part of a parity extent
- **THEN** the service reads the required old data/parity bytes, computes the reference-equivalent XOR update, and writes only after durable intent

#### Scenario: Full overwrite is aligned

- **WHEN** a write fully covers the required data/parity extent
- **THEN** the service may avoid old-data reads only under the codec contract while producing parity bytes equal to full recomputation

### Requirement: Durable completion and clean checkpoint require fences

Flush, FUA, and checkpoint behavior SHALL preserve the normalized durability intent. A write SHALL not clear dirty state or establish valid checksum evidence unless every required store has covering fence evidence and recovery generations still match. Unsupported durability requirements SHALL be rejected or explicitly reported weaker.

#### Scenario: All required stores are fenced

- **WHEN** data and parity writes are terminal and matching durable fences cover them
- **THEN** recovery may checkpoint the proven region and the service may return the corresponding durable completion

#### Scenario: A fence is missing or volatile

- **WHEN** any required store lacks covering durable evidence
- **THEN** the service leaves the region dirty/uncertain and does not report clean or valid integrity

### Requirement: Abandonment, restart, and failure preserve operation safety

Frontend abandonment SHALL suppress delivery interest only; the logical transaction and operation-slot resources SHALL drain or persist conservative recovery evidence. Short, failed, uncertain, crash, and restart paths SHALL not reuse slots/buffers or blindly reissue non-idempotent writes.

#### Scenario: Request is abandoned after intent

- **WHEN** the frontend abandons a write after durable intent but before terminal completion
- **THEN** the transaction continues reconciliation, retains dirty evidence as needed, and releases resources only after safe terminal state

#### Scenario: Service restarts with dirty state

- **WHEN** the process restarts after an incomplete write/fence
- **THEN** it reopens in recovery/blocked handling and never infers a clean state from the missing completion

### Requirement: Portable members remain ordinary and control state is disposable

Data and parity member files SHALL contain no required DiskWeave metadata. Loss of disposable control state SHALL not make intact payloads unreadable or authorize unsafe writes; recovery state and topology evidence remain the authority for protected mutation.

#### Scenario: Control database is removed

- **WHEN** the management/control database is deleted while data and recovery stores remain
- **THEN** management state can be rebuilt without changing payload bytes or silently authorizing a write

#### Scenario: Backing and export paths alias

- **WHEN** a proposed backing path is also an active exported endpoint
- **THEN** the service rejects the configuration before opening competing access

