# Linux ublk/ext4 acceptance record

## Claim boundary

This record proves a disposable one-data/one-parity file-backed fixture can be exported through real Linux ublk, formatted as ext4, mounted, mutated, flushed, unmounted, restarted, and read again on Linux. It does not claim production durability, power-loss safety, multi-device atomic publication, daemon recovery, FUA, discard, write-zeroes, online topology mutation, or hardware safety. SQLite remains an evaluation-only recovery adapter; this run does not select production journal, synchronization, checkpoint, or connection settings.

## Evidence run

Run on 2026-08-10 with `tools/linux-disk-acceptance/run.sh`:

- Source archive SHA-256: `161b38983951cbbddf31d2ab6d616aaed41222e5a7a8d4969806ce858bb92426`.
- Guest: Ubuntu arm64, kernel `7.0.0-28-generic`, real `/dev/ublkb0` endpoint.
- Machine-readable evidence: `verification/linux-ublk-ext4-acceptance.json`.
- Retained live traces: `verification/linux-ublk-trace-first.json` (445,541 bytes, 453 records, SHA-256 `1ecf091313fdcdd1907298a20a0d5549a492e52b5ee2c1358133323945f05171`) and `verification/linux-ublk-trace-second.json` (79,530 bytes, 81 records, SHA-256 `9793698021ba48cace5f2c3d4f115282918452964e6ef1919bb2642f7986b726`).
- Both traces use `dwv.ublk.trace.v2`, record queue depth 8, maximum transfer 131,072 bytes, maximum 4,096 records, no exhaustion, and clean deterministic replay through `dwv demo disk trace-replay`.
- Workload: mkfs.ext4, mount, create, fsync, overwrite, rename, directory sync, read, delete, unmount, clean shutdown, restart, read-only remount.
- Durable file content SHA-256 before and after restart: `d4ad659dcd887413e31f0b6d272b2b353d29734c3cba9f1cb9b74ab45865f4d7`.
- Data and parity payload SHA-256 after shutdown: `7a088635b8c3680d0cb79f527a9f74ca326ffbe88c9e152c3979f84fd511718c`; byte equality passed.
- The ordinary data backing file mounted directly as read-only ext4 after service shutdown and exposed the same content.
- Both ublk runs ended in lifecycle state `stopped` only after drain, checkpoint, endpoint-removal, and trace-replay checks passed.

Negative cases failed closed for undersized geometry, unsupported topology, second-owner acquisition, cleanup against a differently owned endpoint, stale readiness, missing recovery authority, unsupported discard, unknown endpoint cleanup, and owner process death. Owner-death evidence required explicit owned-endpoint cleanup before successful reacquisition. Payload hashes were unchanged across pre-publication refusal cases. Partial multi-store fence coverage was rejected by the portable transaction regression executed in the guest.

## Portable and proof evidence

- `cargo test --workspace --all-targets`: 298 passed across 22 suites; 1 explicitly ignored hardware-dependent test.
- `cargo test -p dwv-frontend-ublk` in the Linux guest: 12 passed, including bounded trace replay/divergence, pre-admission reservation, stale/duplicate completion, lifecycle refusal, probe/shutdown classification, borrowed write-payload identity, and fixture validation.
- `cargo test -p dwv-transaction-ref partial_multi_store_fence_is_rejected` in the Linux guest: passed.
- `cargo test --test cli_demo`: covers portable trace replay, malformed trace refusal, oversized trace refusal, lifecycle confirmation, and source-preserving replay.
- TLC 2.19 on `verification/tla/RecoveryProtocol.tla`: 234 states generated, 125 distinct states, complete depth 9, all configured invariants passed. The model composes durable dirty intent, data/parity mutation, per-store fences, recovery checkpointing, clean publication, and crash/restart handoff.
- Kani 0.67.0 `dirty_region_mapping_covers_every_intersection_once`: 0 of 584 checks failed, 7 unreachable.
- Kani 0.67.0 `fence_coverage_requires_every_store_region_and_incarnation`: 0 of 893 checks failed, 6 unreachable.

The deterministic recovery model exercises crash cuts before durable intent and after intent, protected mutation, fence, and checkpoint. The before-intent mutant is retained as a regression requiring `ReconciliationRequired`; clean publication is permitted only after durable intent, complete protected mutation, complete store-fence coverage, and recovery checkpoint evidence. Trace replay is validation-only: it rechecks adapter translation and completion mapping without opening a fixture, backend, or device, mutating payload bytes, or claiming application consumption or physical durability.
