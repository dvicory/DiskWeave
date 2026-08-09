# security-boundaries Specification

## Purpose

Define cross-cutting safety boundaries for authority, hostile input, privacy, resource admission, and dependency use. Capability specs own their concrete mechanisms.

## Requirements

### Requirement: Read-only inspection and mutation authority are separate
<!-- dwv:req req.security-boundaries.read-only-inspection-and-mutation-authority-are-separate -->

Inspection, verification, planning, and evidence export SHALL be usable without mutation authority. Destructive repair, topology changes, rebaseline, format migration, and external-write release SHALL require an identity- and generation-bound plan with explicit confirmation and revalidation before irreversible mutation.

#### Scenario: An inspection command runs
- **WHEN** an operator requests status, identity evidence, capability evidence, inspection, or verification
- **THEN** the command does not alter payload bytes, parity, topology, recovery generations, or integrity evidence

#### Scenario: A state-changing plan is stale
- **WHEN** a plan's identity, range, generation, or evidence no longer matches the observed state
- **THEN** execution is rejected before protected mutation

### Requirement: Hostile inputs and resources are bounded before admission
<!-- dwv:req req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission -->

Paths, serialized metadata, traces, control requests, ranges, counts, recursion, allocations, file descriptors, buffers, queue depth, retries, jobs, and background bandwidth SHALL be bounded and validated before untrusted input can cause unbounded work. Invalid or exhausted resources SHALL produce an explicit refusal or blocked state rather than silent truncation, looping, or history deletion.

#### Scenario: A declared size exceeds a limit
- **WHEN** a parser or operation declares a range, count, allocation, or nested structure beyond the active bound
- **THEN** it fails before media mutation or unbounded allocation

#### Scenario: Capacity is exhausted
- **WHEN** admission or recovery resources are exhausted
- **THEN** the operation remains explicitly blocked or dirty and existing evidence is retained

### Requirement: Sensitive payload and deployment facts are minimized
<!-- dwv:req req.security-boundaries.sensitive-payload-and-deployment-facts-are-minimized -->

Payload bytes, cleartext paths, credentials, private runtime handles, and private prompt content SHALL be excluded from logs, traces, evidence, and context by default. Semantic artifacts SHALL use ranges, identities, hashes, generated patterns, and bounded summaries unless a deliberate authorized inspection requires more detail.

#### Scenario: A trace records a payload operation
- **WHEN** a semantic trace or evidence report records a read or write
- **THEN** it records bounded identity/range/pattern or digest information rather than cleartext payload bytes by default

#### Scenario: A dependency crosses a semantic boundary
- **WHEN** implementation code adopts a runtime, database, frontend, codec, or verification library
- **THEN** it remains behind a narrow adapter and cannot redefine portable semantics or persistent authority merely through its types
