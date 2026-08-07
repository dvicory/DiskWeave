# architecture-contract Specification

## Purpose
OS-000 turns the v0.6 handoff into the repository-local OpenSpec source of truth for the portable contracts and dependency-ordered Phase 0 work. It governs artifact structure and evidence claims; it does not implement runtime behavior.
## Requirements
### Requirement: Architecture artifacts preserve the handoff contract

The OS-000 artifacts SHALL cite the applicable handoff sections and decision IDs, classify accepted, provisional, validation, tunable, user-decision, deferred, rejected, and format-experimental choices where applicable, and make the handoff’s portable boundaries and fail-closed invariants normative. Every affected design SHALL contain the handoff’s twenty numbered sections in order.

#### Scenario: A later OpenSpec relies on an accepted decision

- **WHEN** a later change relies on a handoff invariant such as ordinary data images, replaceable frontend/runtime/database seams, bounded resources, or independent parity/integrity state
- **THEN** its artifacts cite the relevant decision or format ID and preserve the invariant without choosing an implementation library as semantic truth

#### Scenario: A choice is provisional or validation-gated

- **WHEN** a change mentions SQLite, a frontend, a runtime, a codec implementation, a queue topology, or a durability behavior that the handoff classifies as provisional or validation-gated
- **THEN** the artifact names the required evidence or ADR and does not present the choice as a stable format or platform guarantee

#### Scenario: The twenty-section contract is reviewed

- **WHEN** an agent opens an OS-000, OS-001, or OS-002 design
- **THEN** sections 1 through 20 appear in the prescribed order, with a concrete scope or a reasoned not-applicable statement in every section

### Requirement: OpenSpec dependencies and readiness are explicit

OS-000 SHALL identify OS-001, OS-002, and OS-003 as its direct successors, and SHALL identify the dependency conditions for OS-004 and later Phase 0 work. Artifact status, task progress, and validation SHALL be recoverable from the OpenSpec CLI; no custom checker or parallel status registry is normative.

#### Scenario: OS-000 is validated

- **WHEN** the four OS-000 artifacts pass `openspec validate`
- **THEN** an agent can use `openspec status` and `openspec instructions apply` to identify the current change and the next dependency-ready OpenSpecs

#### Scenario: A correctness-critical predecessor is incomplete

- **WHEN** a later change requires OS-000 or another predecessor whose artifacts/evidence are incomplete
- **THEN** the later task remains blocked or explicitly pending and the artifact does not claim readiness

#### Scenario: A task checkbox is stale

- **WHEN** a task is checked but its required artifact or evidence is absent
- **THEN** the task is reconciled before completion and CLI validation is not treated as proof of runtime or hardware behavior

### Requirement: Portable boundaries and safety invariants are normative

The architecture contract SHALL keep portable semantic behavior independent of ublk, FSKit, io_uring, SQLite, async runtimes, `procmachines`, and namespace implementation types. It SHALL preserve ordinary independently readable data payloads, fail-closed ambiguous identity/recovery behavior, explicit durability evidence, bounded resources, independent parity and integrity state, and no stable format promise before its recovery and independent-decoder gates.

#### Scenario: A later implementation selects a library

- **WHEN** a later implementation selects a frontend, database, runtime, or codec library
- **THEN** the selection stays behind the semantic boundary and does not redefine portable behavior, recovery truth, or durable format semantics

#### Scenario: Evidence is uncertain

- **WHEN** identity, topology, completion, durability, integrity, or reconstruction evidence is unknown or ambiguous
- **THEN** the relevant contract retains `UNKNOWN`/`DIRTY`/stale state or refuses the operation and does not infer safe success or repair

#### Scenario: A persistent representation is unproven

- **WHEN** a schema, parity envelope, trace, or manifest lacks crash, capacity, migration, or independent-reader evidence
- **THEN** it remains experimental or deferred and is not described as a stable compatibility promise

### Requirement: Completion is evidence-backed and scope-accurate

OS-000 and its successor artifacts SHALL define observable outcomes, failure behavior, acceptance criteria, forbidden outcomes, compatibility consequences, and next unlocked work. Portable tests may establish portable semantics only; unavailable Linux, macOS, device, power-loss, or hardware evidence SHALL remain explicitly unmet or gated.

#### Scenario: The active artifact set is validated

- **WHEN** an agent runs `openspec validate` for an affected change
- **THEN** malformed delta structure and missing required artifacts are reported before the change is considered artifact-complete

#### Scenario: A portable test passes

- **WHEN** a standard-library contract test passes
- **THEN** the artifact may claim the corresponding portable semantic behavior but not a platform, physical durability, or production certification

#### Scenario: Platform evidence is unavailable

- **WHEN** a criterion requires Linux, macOS, a real database crash, power loss, or hardware and that environment is unavailable
- **THEN** the criterion remains visible as gated/unmet and no task or design claims it passed

### Requirement: Agent workflow uses only OpenSpec artifacts and CLI

OS-000 SHALL define the handoff read → dependency selection → artifact refinement → implementation/evidence → validation → archive workflow. It SHALL forbid repository-specific architecture-checker code, duplicate decision registries, and hidden chat-only readiness state.

#### Scenario: An agent resumes without chat history

- **WHEN** an agent reads the committed artifacts and runs the OpenSpec status/instructions commands
- **THEN** it can identify scope, prerequisites, remaining work, evidence gaps, and next dependency-ready changes

#### Scenario: A proposed shortcut changes the architecture boundary

- **WHEN** a shortcut would add a checker, choose a crate as durable truth, or claim unavailable evidence
- **THEN** the artifact records the shortcut as forbidden or deferred and preserves the semantic seam

