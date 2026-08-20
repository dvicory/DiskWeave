# transaction-engine-evidence-comparison Specification

## Purpose
This capability defines the evidence contract for comparing DiskWeave's
explicit transaction machine with an isolated procedural implementation before
selecting a production transaction engine.
## Requirements
### Requirement: Both engines consume and emit the same semantic contract
<!-- dwv:req req.transaction-engine-evidence-comparison.both-engines-consume-and-emit-the-same-semantic-contract -->

The comparison SHALL provide both engines with the same transaction identity, topology/recovery generations, ranges, dirty regions, integrity extents, reads, parity plan, writes, stores, watermarks, and limits. Backend child operations, runtime handles, and `procmachines` internals SHALL NOT cross the semantic comparison boundary.

#### Scenario: Representative write is driven through both engines

- **WHEN** the same representative dirty-region parity-write plan and result sequence is submitted to both engines
- **THEN** both engines emit the same permitted semantic action/result kinds and final safety classification

#### Scenario: Candidate dependency is isolated

- **WHEN** the procedural adapter is compiled and inspected
- **THEN** the dependency is confined to the candidate crate and no public DiskWeave semantic type names or stores procedural implementation internals

### Requirement: Normalized traces preserve safety semantics
<!-- dwv:req req.transaction-engine-evidence-comparison.normalized-traces-preserve-safety-semantics -->

The comparison SHALL normalize action and result traces to deterministic sequence numbers, semantic action/result kinds, stage transitions, and final result classifications. Allowed batching or child-operation ordering differences SHALL be representable without changing semantic outcomes.

#### Scenario: Equivalent batching is normalized

- **WHEN** one engine batches child I/O differently while producing the same semantic action results
- **THEN** normalized traces compare equivalent

#### Scenario: Dirty clearing proof differs

- **WHEN** either engine attempts a recovery `CLEAN` transition without equivalent persistence evidence, generation, and integrity evidence
- **THEN** the comparison reports a semantic mismatch and rejects the candidate

### Requirement: Deterministic fault schedules cover irreversible boundaries
<!-- dwv:req req.transaction-engine-evidence-comparison.deterministic-fault-schedules-cover-irreversible-boundaries -->

The comparison SHALL drive deterministic schedules covering successful completion, EIO/failure, delayed and out-of-order completion, short I/O, uncertain completion, frontend abandonment before and after the irreversible boundary, daemon crash at each suspension point, power loss after each modeled persistence transition, duplicate delivery, and stale operation-slot tokens.

#### Scenario: Failure occurs before the write-recovery record is durable

- **WHEN** range acquisition or the first write-recovery-record action fails before durable `DIRTY` state
- **THEN** both engines classify the transaction as safely aborted or report a mismatch; neither may claim a data/parity write occurred

#### Scenario: Failure after the irreversible boundary

- **WHEN** a write, uncertain completion, crash, or power loss occurs after a durable write-recovery record or data/parity write
- **THEN** both engines retain dirty state or the requirement for reconciliation and do not silently complete or clear dirty state

#### Scenario: Abandonment does not cancel ownership

- **WHEN** frontend abandonment occurs before or after the irreversible boundary
- **THEN** both engines preserve the same completion/reconciliation classification and operation resources remain owned until backend completion is known

#### Scenario: Duplicate or stale delivery arrives

- **WHEN** a terminal result is delivered twice or a result carries a stale operation generation
- **THEN** duplicate terminal delivery is ignored where permitted and stale generation delivery is rejected without mutating reused state

### Requirement: Candidate selection is evidence-backed and reversible
<!-- dwv:req req.transaction-engine-evidence-comparison.candidate-selection-is-evidence-backed-and-reversible -->

The comparison SHALL record correctness equivalence, trace results, fault
coverage, resource/cost measurements, dependency/audit findings, known
limitations, fallback behavior, and an exit path. The procedural candidate SHALL
be selected only when mandatory safety equivalence passes; otherwise the
explicit machine remains the oracle and production fallback.

#### Scenario: Candidate passes mandatory gates

- **WHEN** all required schedules have equivalent permitted outcomes and
  acceptable bounded cost/dependency evidence
- **THEN** the ADR may select the procedural candidate behind the semantic
  adapter while retaining the explicit machine as a replaceable oracle

#### Scenario: Candidate fails or evidence is inconclusive

- **WHEN** any mandatory safety gate fails or evidence cannot distinguish the
  implementations
- **THEN** the explicit machine remains selected, no production migration is
  performed, and the ADR records the candidate's removal or exit procedure

