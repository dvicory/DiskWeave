# Linux ublk/ext4 acceptance record

## Claim boundary

This record proves a disposable one-data/one-parity file-backed fixture can be exported through real Linux ublk, formatted as ext4, mounted, mutated, flushed, unmounted, restarted, and read again on Linux. It does not claim production durability, power-loss safety, multi-device atomic publication, daemon recovery, FUA, discard, write-zeroes, online topology mutation, or hardware safety. SQLite remains an evaluation-only recovery adapter; this run does not select production journal, synchronization, checkpoint, or connection settings.

## Evidence run

Run on 2026-08-10 with `tools/linux-disk-acceptance/run.sh` after all four Milestone 8 changes were archived:

- Exact execution-archive SHA-256: `2d8c947cfdbdc5012964b6464eb23433366cdcf91a1c35c77568578c5e5d328c`.
- `run.sh:20-29` archived repository root `.` before guest startup and before copying new evidence back. It excluded `.git`, `.jj`, `.omp`, `target`, and `tools/macos-bridge-probe/.build`; every other present working-tree path was included, including source, tests, scripts, specs, archives, documentation, and prior verification files.
- The generated evidence and traces were copied into `verification/` only after the archive was closed and hashed. The digest identifies the exact archive executed in the guest, not a content-stable digest of the later post-run tree; the gzip header makes separate compressed snapshots differ even when their uncompressed tar payloads are identical.
- VM prefix: `dwv-linux-acceptance-` (`run.sh:6`); observed instance: `dwv-linux-acceptance-1786384884`.
- Fixture root: `/var/tmp/dwv-linux-acceptance`; mountpoint: `/mnt/dwv-linux-acceptance` (`guest.sh:6-7`).
- Guest evidence path: `/tmp/dwv-linux-acceptance-evidence.json` (`guest.sh:275`, copied by `run.sh:36`).
- Guest: Ubuntu 26.04 arm64 image, kernel `7.0.0-28-generic`, real `/dev/ublkb0` endpoint.
- Machine-readable evidence: `verification/linux-ublk-ext4-acceptance.json` (9,975 bytes, SHA-256 `4443201278b62bf6070590f8bf2008dcf14891c6230a6adb5b771a79cf2cda6e`).
- Retained live traces: `verification/linux-ublk-trace-first.json` (448,488 bytes, 456 records, SHA-256 `f01f8c2fc1b7e1a5d7fb2e792d59fb69406e2f0722361f2b2016990df7c72192`) and `verification/linux-ublk-trace-second.json` (81,494 bytes, 83 records, SHA-256 `b9524022e53b7cc12e2025d90383f40abc05b4a8c9169922a1ac8efc248decf8`).
- Both traces use `dwv.ublk.trace.v2`, record queue depth 8, maximum transfer 131,072 bytes, maximum 4,096 records, no exhaustion, and clean deterministic replay through the current root `dwv demo disk trace-replay`.
- Workload: mkfs.ext4, mount, create, fsync, overwrite, rename, directory sync, read, delete, unmount, clean shutdown, restart, read-only remount.
- Durable file content SHA-256 before and after restart: `d4ad659dcd887413e31f0b6d272b2b353d29734c3cba9f1cb9b74ab45865f4d7`.
- Data and parity payload SHA-256 after shutdown: `61c7461a00d6395a25930af9e6d98a30f2105d76c2c0769f1f0858c63a794d10`; byte equality passed.
- The ordinary data backing file mounted directly as read-only ext4 after service shutdown and exposed the same content.
- Both ublk runs ended in lifecycle state `stopped` only after drain, checkpoint, endpoint-removal, and trace-replay checks passed.

Negative cases failed closed for undersized geometry, unsupported topology, second-owner acquisition, cleanup against a differently owned endpoint, stale readiness, missing recovery authority, unsupported discard, unknown endpoint cleanup, and owner process death. Owner-death evidence required explicit owned-endpoint cleanup before successful reacquisition. Payload hashes were unchanged across pre-publication refusal cases. Partial multi-store fence coverage was rejected by the portable transaction regression executed in the guest.

## Portable and proof evidence

- `cargo test --workspace --all-targets`: 303 passed across 22 suites; 1 explicitly ignored hardware-dependent test.
- `cargo test -p dwv-frontend-ublk` in the Linux guest: 12 passed, including bounded trace replay/divergence, pre-admission reservation, stale/duplicate completion, lifecycle refusal, probe/shutdown classification, borrowed write-payload identity, and fixture validation.
- `cargo test -p dwv-transaction-ref partial_multi_store_fence_is_rejected` in the Linux guest: passed.
- `cargo test --test cli_demo`: covers portable trace replay, malformed trace refusal, oversized trace refusal, lifecycle confirmation, and source-preserving replay.
- TLC 2.19 on `verification/tla/RecoveryProtocol.tla`: 234 states generated, 125 distinct states, complete depth 9, all configured invariants passed. The model composes durable dirty intent, data/parity mutation, per-store fences, recovery checkpointing, clean publication, and crash/restart handoff.
- Kani 0.67.0 `dirty_region_mapping_covers_every_intersection_once`: 0 of 584 checks failed, 7 unreachable.
- Kani 0.67.0 `fence_coverage_requires_every_store_region_and_incarnation`: 0 of 893 checks failed, 6 unreachable.

The deterministic recovery model exercises crash cuts before durable intent and after intent, protected mutation, fence, and checkpoint. The before-intent mutant is retained as a regression requiring `ReconciliationRequired`; clean publication is permitted only after durable intent, complete protected mutation, complete store-fence coverage, and recovery checkpoint evidence. Trace replay is validation-only: it rechecks adapter translation and completion mapping without opening a fixture, backend, or device, mutating payload bytes, or claiming application consumption or physical durability.
