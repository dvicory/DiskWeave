## 1. OpenSpec and handoff contract

- [x] 1.1 Ground the change in handoff Sections 7.6, 12.2, 12.7, 17.6, 26.4, the OS-016 roadmap row, and OS-015/OS-014 prerequisites.
- [x] 1.2 Specify exact known-erasure eligibility, degraded telemetry, dirty/unknown refusal, separate-target ordering, durable resume, final verification, and replacement identity semantics.
- [x] 1.3 Keep degraded writes, online rebuild, P/Q, two-erasure, Gate H, live APFS, Linux frontend, and physical durability claims out of scope.

## 2. Typed rebuild recovery authority

- [x] 2.1 Add a dedicated `dwv-recovery` rebuild module with bounded IDs, source/replacement binding, lifecycle, cursor, and receipt-driven mutations.
- [x] 2.2 Advance the semantic schema/migration/export limits for typed rebuild state while retaining v2 migration support and generic non-rebuild maintenance checkpoints.
- [x] 2.3 Test atomic cursor advancement, stale-generation refusal, identity/topology mismatch refusal, bounded export, and interrupted-state round trip.

## 3. Known-erasure degraded reads

- [x] 3.1 Add a dedicated portable degraded-read module and opaque authorization bound to validated single-XOR topology, recovery generation, range evidence, missing stable slot, and source identities.
- [x] 3.2 Reconstruct exact requested bytes with zero-tail semantics, explicit degraded telemetry, and zero writes.
- [x] 3.3 Test clean middle/tail ranges plus stale generation, dirty/unknown range, ambiguous/beyond-tolerance, excluded survivor, aliased source, and short-read refusal.

## 4. Offline resumable rebuild

- [x] 4.1 Add a distinct rebuild-target durability trait and deterministic bounded chunk planner.
- [x] 4.2 Implement reconstruct → separate-target write → readback/equation verification → durable flush/fence → opaque chunk receipt ordering.
- [x] 4.3 Commit chunk receipts through generation-checked recovery transactions and resume only from exactly matching durable state.
- [x] 4.4 Add complete final verification and prepared replacement topology construction that preserves stable slot/coding position and does not publish active topology.
- [x] 4.5 Test crash cuts before/after every irreversible boundary, idempotent replay, source/replacement alias refusal, mismatched resume refusal, and final-verification failure.

## 5. macOS/portable acceptance

- [x] 5.1 Add file-backed adapters using existing exact file-store operations and durable flush evidence without exposing file or OS types in portable APIs.
- [x] 5.2 Add a macOS regular/sparse-file fixture for degraded reads, interrupted resume, full replacement byte comparison, clean shutdown, direct post-service read, and environment-gated independent APFS-image attachment.
- [x] 5.3 Prove source data/parity remain byte-identical and no Linux-only integration is required.

## 6. Acceptance and archive

- [x] 6.1 Run workspace tests, format, strict targeted Clippy, dependency inspection, and OpenSpec validation.
- [x] 6.2 Record requirement/scenario evidence and the portable/macOS claim boundary.
- [x] 6.3 Verify the implementation against all artifacts, commit conventionally, and archive only with no critical issues.
