## 1. Artifact and module contract

- [x] 1.1 Cite handoff D-008/D-009/D-010/D-021, Gate D/E, and Phase 1 OS-010 prerequisites.
- [x] 1.2 Define all twenty design sections, semantic boundaries, forbidden outcomes, and successor changes.
- [x] 1.3 Define the capability requirements and deterministic acceptance cases.

## 2. Recovery transition model

- [x] 2.1 Split `dwv-recovery` into generation, invalidation, intent, checkpoint, and transition modules without changing existing public semantics unnecessarily.
- [x] 2.2 Implement typed dirty/checksum generation state and atomic intent decisions for first-write and already-dirty paths.
- [x] 2.3 Implement fence coverage and generation-checked checkpoint decisions, including session-dirty state.
- [x] 2.4 Add structured failure, uncertainty, crash, and reconciliation-required outcomes.

## 3. Machine and simulator integration

- [x] 3.1 Connect OS-008 normalized actions to `PersistIntent`, home mutation, fence, and checkpoint evidence.
- [x] 3.2 Add deterministic cut-point and overlapping-write schedules to `dwv-sim`.
- [x] 3.3 Add model/property tests for ordering, generation monotonicity, and conservative outcomes.

## 4. Verification and gate evidence

- [x] 4.1 Add SQLite/in-memory semantic adapter tests and recovery export fixtures.
- [x] 4.2 Run focused/workspace tests, format, dependency inspection, and OpenSpec validation.
- [x] 4.3 Confirm no Linux/runtime/database types leak into portable public semantics and document macOS evidence.
- [x] 4.4 Mark implementation tasks complete only after the OS-010 acceptance matrix passes; leave OS-011/OS-013 for their own changes.
