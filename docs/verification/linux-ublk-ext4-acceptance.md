# Linux ublk/ext4 acceptance record

## Claim boundary

This record proves a disposable one-data/one-parity file-backed fixture can be exported through real Linux ublk, formatted as ext4, mounted, mutated, flushed, unmounted, restarted, and read again on Linux. It does not claim production durability, power-loss safety, multi-device atomic publication, daemon recovery, FUA, discard, write-zeroes, online topology mutation, or hardware safety. SQLite remains an evaluation-only recovery adapter; this run does not select production journal, synchronization, checkpoint, or connection settings.

## Evidence run

Run on 2026-08-11 with `tools/linux-disk-acceptance/run.sh` through the prepared reusable Lima runner:

- Exact final execution-archive SHA-256: `afa56d9bccf183c0f2e33b0f6e308654017685ffe3f7ee6646e46c0de82af37c`.
- `run.sh` archived repository root `.` with `tar -C` before guest startup and before copying new evidence back. It excluded `.git`, `.jj`, `.omp`, `target`, and `tools/macos-bridge-probe/.build`; every other present working-tree path was included.
- The runner staged generated evidence and traces in a host `mktemp` directory, enforced the warm bound, and promoted them only after the bound passed. The archive digest identifies the exact guest input rather than the later tree containing refreshed evidence and prose; gzip headers also make separate compressed snapshots differ.
- Runner instance: `dwv-linux-acceptance`. Guest: Ubuntu 26.04 arm64 image, kernel `7.0.0-28-generic`, real `/dev/ublkb0` endpoint.
- Host and guest workspaces, the fixture, mountpoint, and scratch files came from `mktemp`. The runner derived repository and Lima configuration paths from its script location. Invoking the absolute script from `/tmp` with a relative output path completed successfully.
- Cold preparation completed in 83 seconds, including a 57-second first release build; its live platform phase completed in 23 seconds. The final prepared invocation completed in 26 seconds, including a 24-second live platform phase.
- The 16 MiB fixture retained ext4 journaling; `mkfs.ext4` created a 1,024-block journal.
- Machine-readable evidence: `verification/linux-ublk-ext4-acceptance.json` (10,749 bytes, SHA-256 `c6024af2a2d6924b7b0fc32af26842d6ca32cf1ded020af7265f4d184ee85cdf`).
- Before production start, the acceptance deliberately changed fixture-local array identity while leaving the admitted `array.json` unchanged. The production `dwv start --array ... --json` path re-observed two recognized members, passed admitted array and publication identity to the frontend without reopening fixture authority, reported `reason_code: frontend-published`, `lifecycle: online`, `access: read-write`, and `publication.status: published` for `/dev/ublkb0`, then remained attached until signal-driven clean shutdown.
- Retained live traces: `verification/linux-ublk-trace-first.json` (272,306 bytes, 277 records, SHA-256 `77a61aa7079747046d5a4f02ffe73d56d6df5d1f4b18154c39786da0a9078fa7`) and `verification/linux-ublk-trace-second.json` (72,636 bytes, 74 records, SHA-256 `708b6b731e42b2f0a2587fe7d27a1abbdbc6e3e80cd6cf358a7ad5122cf51e7c`).
- Both traces use `dwv.ublk.trace.v2`, record queue depth 8, maximum transfer 131,072 bytes, maximum 4,096 records, no exhaustion, and clean replay through the current root `dwv demo disk trace-replay`.
- Workload: mkfs.ext4, mount, create, fsync, overwrite, rename, directory sync, read, delete, unmount, clean shutdown, restart, and read-only remount.
- Durable file content SHA-256 before and after restart: `d4ad659dcd887413e31f0b6d272b2b353d29734c3cba9f1cb9b74ab45865f4d7`.
- Data and parity payload SHA-256 after shutdown: `c574095223c6293beb3fd031e400216943851ce9030745b91d80179773f3f7e4`; byte equality passed.
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
