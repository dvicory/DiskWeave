## 1. Artifact and dependency contract

- [x] 1.1 Cite handoff D-009/D-010/D-011/D-021, P-007/V-009, Gate E, and OS-010 dependency.
- [x] 1.2 Define all twenty design sections, independent parity/integrity semantics, and the profile migration seam.
- [x] 1.3 Define the checksum capability requirements and crash/concurrency acceptance cases.

## 2. Checksum semantics

- [ ] 2.1 Split integrity types into profile, extent, record, provider, and worker/migration modules.
- [ ] 2.2 Implement typed target/content/set generations and `ABSENT`/`STALE`/`VALID` transitions.
- [ ] 2.3 Implement digest-provider seam and deterministic portable vectors; research and record dependency choice before adding a crate.
- [ ] 2.4 Enforce OS-010 invalidation, fence evidence, and generation-checked valid-result commits.

## 3. Revalidation and migration

- [ ] 3.1 Implement bounded job scheduling semantics and stale-result rejection.
- [ ] 3.2 Implement full-overwrite optimization with equivalence tests against fenced readback.
- [ ] 3.3 Implement parallel checksum-set migration and interrupted-switch recovery.
- [ ] 3.4 Add data/P/Q target mapping while keeping Q implementation optional.

## 4. Verification and evidence

- [ ] 4.1 Add golden vectors, concurrency/model/fuzz tests, and simulator crash schedules.
- [ ] 4.2 Benchmark provisional extent/profile choices and record V-009 evidence.
- [ ] 4.3 Run workspace tests, format, dependency/license inspection, and OpenSpec validation on macOS.
- [ ] 4.4 Mark complete only when no false `VALID` schedule remains; leave repair/scrub to OS-014/017.
