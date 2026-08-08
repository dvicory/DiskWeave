## 1. Artifact and evidence contract

- [x] 1.1 Cite handoff Section 17, D-004/D-017/D-018, P-009, V-007, Gate F, and Phase 2 OS-020.
- [x] 1.2 Define all twenty design sections, candidate boundaries, portable-demo claim, and environment-blocker policy.
- [x] 1.3 Define delta requirements and the fixed proxy/synchronization/alias acceptance matrix.

## 2. Baseline probe

- [x] 2.1 Add a macOS fixture/probe with explicit temporary backing/export directories and fixed geometry.
- [x] 2.2 Add normalized operation trace capture for read/write/flush/sync/close/detach/failure.
- [ ] 2.3 Verify exact range, no truncate/resize, hole/copy/file-ID, and backing/export separation behavior.

## 3. Candidate evidence

- [ ] 3.1 Characterize the regular-file/DiskImages baseline available on the host.
- [ ] 3.2 Probe FSKit-first attachment and document required extension, entitlement, signing, and automation setup.
- [ ] 3.3 Compare macFUSE only if available without changing core or requiring Linux-specific semantics.
- [ ] 3.4 Run sync/cache/disconnect/kill/restart tests and classify environment versus semantic failures.

## 4. Decision and verification

- [ ] 4.1 Produce a comparative ADR with selected bridge or documented blocker and portable-demo limitations.
- [ ] 4.2 Run OpenSpec validation, format/dependency checks, and deterministic synthetic-trace tests on macOS.
- [x] 4.3 Record reproducible manual steps for privileged/GUI attachment tests that cannot run in the restricted session.
- [ ] 4.4 Mark complete only when every required feasibility question has evidence or an explicit blocker; leave OS-021 implementation separate.
