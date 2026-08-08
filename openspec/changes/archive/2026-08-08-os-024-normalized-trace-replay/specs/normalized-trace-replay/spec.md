## Purpose

This capability provides a bounded, privacy-safe semantic trace that can be
stored, rendered, migrated, and replayed against portable backends without
capturing filesystem paths or protected payload bytes.

## ADDED Requirements

### Requirement: Traces use a bounded canonical semantic format

A trace SHALL be a canonical JSON document with an explicit schema version,
portable fixture geometry, symbolic payload descriptions, and an ordered event
sequence. It SHALL contain no filesystem paths, raw payload bytes, database
blobs, or backend-specific object handles. Export SHALL reject a trace larger
than 1 MiB or containing more than 512 events.

#### Scenario: Exporting a normal workflow
- **WHEN** a supported demo workflow is exported
- **THEN** the document contains its schema version, deterministic fixture
  description, normalized events, and terminal state summary
- **AND** the document contains symbolic offsets, lengths, generations, and
  outcomes rather than raw payload bytes or host paths

#### Scenario: Oversized or non-canonical input
- **WHEN** an imported document exceeds the byte/event bound, has duplicate or
  non-monotonic event sequence numbers, or contains unknown required fields
- **THEN** import rejects it before replay and reports a bounded validation
  error without executing any event

### Requirement: Traces cover portable correctness boundaries

The event vocabulary SHALL represent reads, writes, flushes, recovery intent,
recovery checkpoints, checksum work, degraded reads, rebuild chunks, repair
decisions, refusals, and uncertain outcomes. Events SHALL carry only portable
semantic values: symbolic store/target identifiers, exact ranges, generations,
durability class, decision class, and deterministic payload pattern metadata.

#### Scenario: Recording a fault and repair decision
- **WHEN** a workflow detects a data mismatch, parity mismatch, ambiguous
  evidence, or uncertain completion
- **THEN** the trace records the classification and repair/refusal decision
- **AND** it never records the source path, source bytes, or backend handle

#### Scenario: Recording durability and recovery
- **WHEN** a workflow performs intent, checkpoint, flush, rebuild, or repair
  work
- **THEN** the trace records the normalized range, generation, fence/uncertain
  outcome, and terminal result in event order

### Requirement: Replay is deterministic across portable backends

The replay command SHALL construct the same deterministic fixture from the
trace description, execute the normalized event sequence against the volatile
media simulator and the ordinary file-backed demo path, and compare payload,
parity, integrity, recovery, and terminal outcome summaries. A mismatch SHALL
be a refusal/diagnostic, not a successful replay.

#### Scenario: Matching simulator and file-backed replay
- **WHEN** a valid exported trace is replayed against both supported backends
- **THEN** both runs complete with the same normalized terminal summary
- **AND** the result reports the compared schema version and event count

#### Scenario: Backend divergence
- **WHEN** one backend produces a different normalized payload, parity,
  integrity, recovery, or terminal outcome summary
- **THEN** replay reports the first differing event and a minimized valid trace
  containing the failing prefix
- **AND** replay does not claim equivalence

### Requirement: Versioning and failure minimization are explicit

The importer SHALL support one documented migration from the immediately prior
trace schema, reject unsupported future versions, and preserve canonical output
when a migrated trace is re-exported. A replay mismatch SHALL be reducible by
removing event suffixes while retaining a valid failing trace within the same
bounds.

#### Scenario: Migrating the prior schema
- **WHEN** a valid schema-version-zero trace is imported
- **THEN** it is converted to the current schema before validation and replay
- **AND** re-export emits only the current canonical schema version

#### Scenario: Unsupported version
- **WHEN** a trace declares a future or otherwise unsupported schema version
- **THEN** import refuses it without touching a backend

#### Scenario: Minimizing a failing trace
- **WHEN** replay finds a backend mismatch
- **THEN** the reported minimized trace preserves the fixture description and
  shortest failing event prefix that reproduces the mismatch
