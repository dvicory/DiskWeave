# Linux ublk/ext4 acceptance record

## Claim boundary

This record proves a disposable one-data/one-parity file-backed fixture can be exported through real Linux ublk, formatted as ext4, mounted, mutated, flushed, unmounted, restarted, and read again on Linux. It does not claim production durability, power-loss safety, multi-device atomic publication, daemon recovery, FUA, discard, write-zeroes, online topology mutation, or hardware safety. SQLite remains an evaluation-only recovery adapter; this run does not select production journal, synchronization, checkpoint, or connection settings.

## Evidence run

Run on 2026-08-14 with `tools/linux-disk-acceptance/run.sh` through the prepared reusable Lima runner:

- Exact final execution-archive SHA-256: `b8f4ed0311bbe783b00882759ab2891f9d1262142f509696a712a087159300f2`.
- `run.sh` archived repository root `.` with `tar -C` before guest startup and before copying new evidence back. It excluded `.git`, `.jj`, `.omp`, `target`, and `tools/macos-bridge-probe/.build`; every other present working-tree path was included.
- The runner staged generated evidence and traces in a host `mktemp` directory, enforced the warm bound, and promoted them only after the bound passed. The archive digest identifies the exact guest input rather than the later tree containing refreshed evidence and prose; gzip headers also make separate compressed snapshots differ.
- Runner instance: `dwv-linux-acceptance`. Guest: Ubuntu 26.04 arm64 image, kernel `7.0.0-28-generic`, real `/dev/ublkb0` endpoint.
- Host and guest workspaces, the fixture, mountpoint, and scratch files came from `mktemp`. The runner derived repository and Lima configuration paths from its script location.
- The prepared invocation completed in 24 seconds, including a 23-second live platform phase.
- The 16 MiB fixture retained ext4 journaling; `mkfs.ext4` created a 1,024-block journal.
- Machine-readable evidence: `verification/linux-ublk-ext4-acceptance.json` (11,747 bytes, SHA-256 `27acc32f8c78e1c6dab96d3ced9d39f6daa4511a85fa4510faad818faac3ba42`).
- Before production start, the acceptance deliberately changed fixture-local array identity while leaving the admitted `array.json` unchanged. The production `dwv start --array ... --json` path re-observed two recognized members, passed admitted array and publication identity to the frontend without reopening fixture authority, reported `dwv.operator.v2`, `reason_code: frontend-published`, `lifecycle: online`, `access: read-write`, and `publication.status: published` for `/dev/ublkb0`, then remained attached until signal-driven clean shutdown.
- The retained production result also reports accepted lineage, continuity-unproved custody, not-yet-interpretable parity basis, and an explicit non-authorization statement; these observations do not authorize C0b or C2 behavior.
- Retained live traces: `verification/linux-ublk-trace-first.json` (273,287 bytes, 278 records, SHA-256 `e6582b4d79bea2aecffa32a2fd9117e33f36aa9a16d1ddf1fe7ade06a3e6bf91`) and `verification/linux-ublk-trace-second.json` (72,606 bytes, 74 records, SHA-256 `0c4de5dd717dc69699a5b6249a96c133263016f999b8e1af23cbf8295f5023fe`).
- Both traces use `dwv.ublk.trace.v2`, record queue depth 8, maximum transfer 131,072 bytes, maximum 4,096 records, no exhaustion, and clean replay through the current root `dwv demo disk trace-replay`.
- Workload: mkfs.ext4, mount, create, fsync, overwrite, rename, directory sync, read, delete, unmount, clean shutdown, restart, and read-only remount.
- Durable file content SHA-256 before and after restart: `d4ad659dcd887413e31f0b6d272b2b353d29734c3cba9f1cb9b74ab45865f4d7`.
- Data and parity payload SHA-256 after shutdown: `357613671821bb61d6e2bbbe03bce787bc281176be2c529a1a4aae2800d3b7ac`; byte equality passed.
- The ordinary data backing file mounted directly as read-only ext4 after service shutdown and exposed the same content.
- Both workload ublk runs ended in lifecycle state `stopped` only after drain, checkpoint, endpoint-removal, and trace-replay checks passed.

Live negative cases failed closed for second-owner acquisition, cleanup against a differently owned endpoint, stale readiness, unsupported discard, unknown endpoint cleanup, and owner process death. Owner-death evidence required explicit owned-endpoint cleanup before successful reacquisition. Payload hashes remained unchanged across the live discard refusal. Deterministic Rust tests separately covered undersized geometry, unsupported topology, missing recovery authority without payload mutation, adapter behavior, and partial multi-store fence refusal.

## Portable and proof evidence

- `cargo test --workspace --all-targets`: 317 passed across 23 suites; 1 explicitly ignored hardware-dependent test.
- `cargo test -p dwv-frontend-ublk`: 13 passed across two suites, including the deterministic undersized-fixture and missing-recovery-authority checks moved out of the guest workflow.
- `cargo test -p dwv-transaction-ref partial_multi_store_fence_is_rejected`: passed outside the timed guest workflow.
- `cargo test --test cli_demo`: covers portable trace replay, malformed trace refusal, oversized trace refusal, lifecycle confirmation, and source-preserving replay.
- TLC 2.19 on `verification/tla/RecoveryProtocol.tla`: 234 states generated, 125 distinct states, complete depth 9, all configured invariants passed. The model composes durable dirty intent, data/parity mutation, per-store fences, recovery checkpointing, clean publication, and crash/restart handoff.
- Kani 0.67.0 `dirty_region_mapping_covers_every_intersection_once`: 0 of 584 checks failed, 7 unreachable.
- Kani 0.67.0 `fence_coverage_requires_every_store_region_and_incarnation`: 0 of 893 checks failed, 6 unreachable.

The deterministic recovery model exercises crash cuts before durable intent and after intent, protected mutation, fence, and checkpoint. The before-intent mutant is retained as a regression requiring `ReconciliationRequired`; clean publication is permitted only after durable intent, complete protected mutation, complete store-fence coverage, and recovery checkpoint evidence. Trace replay is validation-only: it rechecks adapter translation and completion mapping without opening a fixture, backend, or device, mutating payload bytes, or claiming application consumption or physical durability.
