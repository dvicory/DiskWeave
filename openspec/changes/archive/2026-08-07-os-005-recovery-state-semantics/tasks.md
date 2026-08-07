## 1. Recovery contract

- [x] 1.1 Add the portable `dwv-recovery` package with no SQLite, filesystem, runtime, or frontend dependency.
- [x] 1.2 Define recovery generations, topology epochs, dirty regions, integrity states, store-fence evidence, sessions, and semantic export.
- [x] 1.3 Define generation-checked all-or-nothing transactions and conservative health/error states.

## 2. Reference implementation

- [x] 2.1 Implement an in-memory recovery-state store with transactional candidate snapshots and immutable topology epochs.
- [x] 2.2 Implement dirty/stale intent, fence recording, clean checkpoint, valid digest, session, topology, and maintenance mutations.
- [x] 2.3 Define the SQLite evaluation seam and deterministic candidate crash-matrix case descriptions without selecting a production mode.

## 3. Evidence

- [x] 3.1 Add deterministic tests for stale generations, atomic rejection, dirty-before-clean ordering, fence coverage, invalid topology epochs, semantic export, and missing/corrupt health.
- [x] 3.2 Run `openspec validate os-005-recovery-state-semantics --json`, workspace tests, and the OpenSpec apply instructions; resolve failures.
- [x] 3.3 Mark OS-005 complete only after the reference protocol is independently tested and record OS-006, OS-008, and OS-010 as successors.

## 4. Handoff acceptance expansion

- [x] 4.1 Add a versioned, bounded semantic schema/migration/export representation with no SQLite layout types in the portable API.
- [x] 4.2 Add adapter-neutral commit observations, simulator-facing reset/corruption/failure cases, and conservative recovery dispositions.
- [x] 4.3 Expand candidate SQLite journal/synchronization/checkpoint evaluation fixtures across process reset, VM reset, power loss, commit uncertainty, missing state, and corruption.
- [x] 4.4 Harden recovery-state validation so invalid fence coverage, stale integrity evidence, and bounded-export violations cannot produce clean/valid claims.
- [x] 4.5 Add deterministic focused tests, run OpenSpec validation and the recovery package tests, and record unresolved SQLite/hardware gaps without archiving.

## 5. Evaluation-only SQLite prototype

- [x] 5.1 Add the separate `dwv-recovery-sqlite` adapter crate and checked-in migration without adding SQLite types or dependencies to portable `dwv-recovery`.
- [x] 5.2 Exercise candidate journal/synchronization/checkpoint configurations, semantic-header persistence/export, integrity, missing-state, and corruption fixtures through the host `sqlite3` executable.
- [x] 5.3 Verify the prototype remains a candidate-evaluation seam rather than a production SQLite or physical-durability selection.
- [x] 5.4 Verify deleting prototype recovery state leaves a separate direct-data fixture readable, and run focused adapter tests.
