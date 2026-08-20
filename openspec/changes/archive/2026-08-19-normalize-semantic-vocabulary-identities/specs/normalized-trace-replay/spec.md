## MODIFIED Requirements

### Requirement: Traces cover portable correctness boundaries
<!-- dwv:req req.normalized-trace-replay.traces-cover-portable-correctness-boundaries -->

The event vocabulary SHALL represent reads, data/parity writes, flushes, write-recovery records, recovery-`CLEAN` transitions, checksum work, degraded reads, rebuild chunks, repair decisions, refusals, and uncertain outcomes. Events SHALL carry only portable semantic values: symbolic store/target identifiers, exact ranges, generations, durability class, decision class, and deterministic payload pattern metadata.

#### Scenario: Recording a fault and repair decision

- **WHEN** a workflow detects a data mismatch, parity mismatch, ambiguous evidence, or uncertain completion
- **THEN** the trace records the classification and repair/refusal decision
- **AND** it never records the source path, source bytes, or backend handle

#### Scenario: Recording durability and recovery

- **WHEN** a workflow commits or confirms a write-recovery record, commits recovery state `CLEAN`, flushes, rebuilds, or repairs
- **THEN** the trace records the normalized range, generation, persistence or uncertain outcome, and final result in event order

