# Linux ublk/ext4 acceptance record

## Claim boundary

This record proves a disposable one-data/one-parity file-backed fixture can be exported through real Linux ublk, formatted as ext4, mounted, mutated, flushed, unmounted, restarted, and read again on Linux. It does not claim production durability, power-loss safety, multi-device atomic publication, daemon recovery, FUA, discard, write-zeroes, online topology mutation, or hardware safety. SQLite remains an evaluation-only recovery adapter; this run does not select production journal, synchronization, checkpoint, or connection settings.

## Evidence run

Run on 2026-08-10 with `tools/linux-disk-acceptance/run.sh` after the Milestone 9 operator path and production-start acceptance step were present:

- Exact execution-archive SHA-256: `2d052477a1ac7c95d545bc3e2ebdc55c4df8b8a52ecbad37fee7c05c45505f55`.
- `run.sh:20-29` archived repository root `.` before guest startup and before copying new evidence back. It excluded `.git`, `.jj`, `.omp`, `target`, and `tools/macos-bridge-probe/.build`; every other present working-tree path was included, including source, tests, scripts, specs, documentation, and prior verification files.
- The generated evidence and traces were copied into `verification/` only after the archive was closed and hashed. The digest identifies the exact archive executed in the guest, not a content-stable digest of the later post-run tree; the gzip header makes separate compressed snapshots differ even when their uncompressed tar payloads are identical.
- VM prefix: `dwv-linux-acceptance-`; observed instance: `dwv-linux-acceptance-1786424164`.
- Fixture root: `/var/tmp/dwv-linux-acceptance`; mountpoint: `/mnt/dwv-linux-acceptance`.
- Guest evidence path: `/tmp/dwv-linux-acceptance-evidence.json`.
- Guest: Ubuntu 26.04 arm64 image, kernel `7.0.0-28-generic`, real `/dev/ublkb0` endpoint.
- Machine-readable evidence: `verification/linux-ublk-ext4-acceptance.json` (12,033 bytes, SHA-256 `9594bfe75e0f63703486bfc5cfed86ade71c8a14c5f218a9da2511e1d345fbac`).
- Before production start, the acceptance deliberately changed fixture-local array identity while leaving the admitted `array.json` unchanged. The production `dwv start --array ... --json` path re-observed two recognized members, published the admitted service without reopening fixture authority, reported `reason_code: frontend-published`, `lifecycle: online`, `access: read-write`, and `publication.status: published` for `/dev/ublkb0`, then remained attached until signal-driven clean shutdown.
- Retained live traces: `verification/linux-ublk-trace-first.json` (451,478 bytes, 459 records, SHA-256 `855eed4ef7ba96030208caa4c8c4e50715e6c0cd80313c010854586910842b10`) and `verification/linux-ublk-trace-second.json` (79,531 bytes, 81 records, SHA-256 `5b3b11d30d34951075d60e3754ebdc85878361a8a95618874af5087029884f2c`).
- Both traces use `dwv.ublk.trace.v2`, record queue depth 8, maximum transfer 131,072 bytes, maximum 4,096 records, no exhaustion, and clean deterministic replay through the current root `dwv demo disk trace-replay`.
- Workload: mkfs.ext4, mount, create, fsync, overwrite, rename, directory sync, read, delete, unmount, clean shutdown, restart, and read-only remount.
- Durable file content SHA-256 before and after restart: `d4ad659dcd887413e31f0b6d272b2b353d29734c3cba9f1cb9b74ab45865f4d7`.
- Data and parity payload SHA-256 after shutdown: `ee2128debcb80c2b254e2d6b69f6fe169be8ccd500cb62abf3d267f0f04363ac`; byte equality passed.
- The ordinary data backing file mounted directly as read-only ext4 after service shutdown and exposed the same content.
- Both workload ublk runs ended in lifecycle state `stopped` only after drain, checkpoint, endpoint-removal, and trace-replay checks passed.

Negative cases failed closed for undersized geometry, unsupported topology, second-owner acquisition, cleanup against a differently owned endpoint, stale readiness, missing recovery authority, unsupported discard, unknown endpoint cleanup, and owner process death. Owner-death evidence required explicit owned-endpoint cleanup before successful reacquisition. Payload hashes were unchanged across pre-publication refusal cases. Partial multi-store fence coverage was rejected by the portable transaction regression executed in the guest.

## Portable and proof evidence

- `cargo test --workspace --all-targets`: 313 passed across 23 suites; 1 explicitly ignored hardware-dependent test.
- `cargo test -p dwv-frontend-ublk` in the Linux guest: 12 passed, including bounded trace replay/divergence, pre-admission reservation, stale/duplicate completion, lifecycle refusal, probe/shutdown classification, borrowed write-payload identity, and fixture validation.
- `cargo test -p dwv-transaction-ref partial_multi_store_fence_is_rejected` in the Linux guest: passed.
- `cargo test --test cli_demo`: covers portable trace replay, malformed trace refusal, oversized trace refusal, lifecycle confirmation, and source-preserving replay.
- TLC 2.19 on `verification/tla/RecoveryProtocol.tla`: 234 states generated, 125 distinct states, complete depth 9, all configured invariants passed. The model composes durable dirty intent, data/parity mutation, per-store fences, recovery checkpointing, clean publication, and crash/restart handoff.
- Kani 0.67.0 `dirty_region_mapping_covers_every_intersection_once`: 0 of 584 checks failed, 7 unreachable.
- Kani 0.67.0 `fence_coverage_requires_every_store_region_and_incarnation`: 0 of 893 checks failed, 6 unreachable.

The deterministic recovery model exercises crash cuts before durable intent and after intent, protected mutation, fence, and checkpoint. The before-intent mutant is retained as a regression requiring `ReconciliationRequired`; clean publication is permitted only after durable intent, complete protected mutation, complete store-fence coverage, and recovery checkpoint evidence. Trace replay is validation-only: it rechecks adapter translation and completion mapping without opening a fixture, backend, or device, mutating payload bytes, or claiming application consumption or physical durability.
