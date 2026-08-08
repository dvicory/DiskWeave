# OS-020: macOS bridge feasibility

## 1. Architecture decisions and target gate

Implement handoff D-004, D-017, D-018, P-009, V-007, Section 17, and the Phase 2 OS-020 row. The target is Gate F evidence preparation; no production macOS or hardware-durability claim is made.

## 2. Concrete outcome

There is a reproducible macOS probe and comparative ADR showing whether the smallest fixed-size proxy path can reach the normalized request boundary and preserve the declared portable-demo contract.

## 3. Prerequisites

Use OS-012 file stores and OS-013 normalized service seams. The current host evidence includes macOS 15.7.7, Command Line Tools SDK 26.2, FSKit headers/modulemap, `hdiutil`, `xcrun`, and `sqlite3`; attachment tests may require entitlements, GUI/automation, or a less restricted session.

## 4. Exact scope and non-scope

Probe/fixture/trace/ADR only. Do not add parity logic to FSKit/macFUSE, implement a filesystem namespace, claim physical power-loss behavior, or begin Linux frontend work.

## 5. Semantic APIs and contracts

Keep the bridge adapter in a macOS-specific tool or crate that translates to normalized request/completion/evidence values. Candidate types are boundary-only. Use Apple’s FSKit, DiskImageKit/DiskImages, and any macFUSE API solely in the harness; see the official FSKit and DiskImageKit documentation for platform interfaces.

## 6. State ownership and lifecycle

The probe owns a temporary fixture and trace; the candidate bridge owns proxy lifecycle; OS-013 owns normalized request semantics; file stores own backing bytes. Setup, attach, open, I/O, sync, disconnect, detach, and cleanup states are explicit and reproducible.

## 7. Persistent-state impact

Use temporary ordinary backing files and disposable state/control databases. Export a bounded JSON/JSONL or normalized trace/ADR bundle; never place DiskWeave metadata in a member payload or rely on raw SQLite pages as bridge evidence.

## 8. Irreversible and durability boundaries

Bridge tests must distinguish host-file synchronization from physical durability. A candidate completion can carry volatile/unknown evidence and must not authorize OS-010 checkpoint/OS-011 validity without the normal store fence contract.

## 9. State and sequence diagrams

```text
fixture -> candidate install/launch -> proxy open -> DiskImages attach
       -> APFS/block workload -> sync/close/detach
       -> backing failure/kill/restart -> trace + evidence + cleanup
```

Each boundary records both candidate operations and normalized service events.

## 10. Concurrency and resource rules

Keep fixtures bounded in size and operation count. Serialize attach/detach and cleanup, but run bounded concurrent reads/writes and sync-heavy workloads to reveal ordering. Do not use unbounded logging or retain payload buffers in traces.

## 11. Failure matrix

Test missing framework/tool, entitlement/signing failure, attach failure, short/failed proxy I/O, backing disappearance, close/detach race, process kill/restart, stale page/cache observation, path/file-ID change, alias, truncate, and cleanup failure. Classify environment blockers separately from semantic failures.

## 12. Deterministic simulator cases

Reuse normalized traces and `dwv-sim` schedules for process crash, restart, flush omission, backing failure, and detach. The live bridge cannot prove power loss, so simulator schedules remain the authoritative crash model.

## 13. Property, model, and fuzz tests

Validate fixed geometry, range/offset arithmetic, no truncate, path/identity separation, trace normalization, bounded lengths, and candidate-result mapping. Fuzz synthetic proxy operations and disconnect orderings without requiring privileged attachment.

## 14. Integration tests

Run a regular-file proxy baseline first, then candidate-specific attachment when available. Exercise `hdiutil`/DiskImages, APFS-visible geometry, sync-heavy operations, detach/restart, and independent backing/export identity. Record skipped tests with exact environment reason.

## 15. Observability, security, and operator behavior

Record OS/build/SDK/tool versions, candidate, entitlements, paths only as redacted fixture IDs, file identities, operation summaries, errors, and evidence disposition. Require explicit disposable paths and refuse active backing/export aliasing. No payload bytes or secrets enter trace artifacts.

## 16. Performance and resource bounds

Measure operation latency, sync latency, throughput only as feasibility evidence, queue/backpressure behavior, and memory for a small fixed fixture. Do not optimize or extrapolate to production; declare the portable-demo ceiling.

## 17. Executable acceptance criteria

Run the fixed proxy, block trace, synchronization/cache, disconnect, identity/alias, entitlements/automation, and cleanup matrix. Produce a comparative ADR with selected bridge or blocker, normalized traces, environment details, and explicit unsupported semantics. Validate OpenSpec and preserve a manual rerun path where the sandbox blocks attachment.

## 18. Forbidden outcomes

No candidate-specific API in core semantics, no backing/export alias, no silent truncate/resize, no fabricated durable evidence, no automatic bridge selection from a compile check, and no physical-durability claim from a successful macOS sync call.

## 19. Migration and compatibility consequences

The selected bridge is replaceable and must target normalized requests. A failed candidate remains documented evidence, not a format dependency. Proxy files are fixed-size transport endpoints, not DiskWeave member formats; future OS-021 can build on the chosen trace contract.

## 20. Next OpenSpecs unlocked

OS-021 macOS frontend is unlocked only after a viable candidate or explicitly accepted narrowed path. OS-022/023 consume the frontend and sync evidence. OS-024 consumes normalized traces. Linux OS-030+ remains out of scope.
