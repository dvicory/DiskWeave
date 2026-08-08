## 1. Artifact and evidence contract

- [x] 1.1 Cite handoff Section 17, D-004/D-017/D-018, P-009, V-007, Gate F, and Phase 2 OS-020.
- [x] 1.2 Define all twenty design sections, candidate boundaries, portable-demo claim, and environment-blocker policy.
- [x] 1.3 Define delta requirements and the fixed proxy/synchronization/alias acceptance matrix.

## 2. Baseline probe

- [x] 2.1 Add a macOS fixture/probe with explicit temporary backing/export directories and fixed geometry.
- [x] 2.2 Add normalized operation trace capture for read/write/flush/sync/close/detach/failure.
- [x] 2.3 Verify host exact-range, hole/copy/file-ID, hard-link alias, and backing/export separation behavior; fixed-size truncate/resize denial remains an explicit bridge-attachment blocker.

## 3. Candidate evidence

- [x] 3.1 Characterize the regular-file/DiskImages baseline available on the host.
- [x] 3.2 Document the FSKit-first attachment, extension, entitlement, signing, and automation setup; live attachment is explicitly blocked by the restricted toolchain/session.
- [x] 3.3 Compare macFUSE only if available without changing core or requiring Linux-specific semantics.
- [x] 3.4 Run and record host sync/close/reopen evidence; classify cache, disconnect, kill, and restart candidate tests as environment-blocked until a signed bridge is available.

## 4. Decision and verification

- [x] 4.1 Produce a comparative ADR with selected bridge or documented blocker and portable-demo limitations.
- [ ] 4.2 Run OpenSpec validation, format/dependency checks, and deterministic synthetic-trace tests on macOS; source parsing and manifest inspection pass, but execution is blocked by the Swift SDK/toolchain mismatch.
- [x] 4.3 Record reproducible manual steps for privileged/GUI attachment tests that cannot run in the restricted session.
- [ ] 4.4 Mark complete only when every required feasibility question has evidence or an explicit blocker; leave OS-021 implementation separate.
