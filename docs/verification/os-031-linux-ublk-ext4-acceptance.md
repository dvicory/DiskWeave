# OS-031 Linux ublk/ext4 acceptance record

## Claim boundary

This record proves a disposable one-data/one-parity file-backed fixture can be exported through real Linux ublk, formatted as ext4, mounted, mutated, flushed, unmounted, restarted, and read again on Linux. It does not claim production durability, power-loss safety, multi-device atomic publication, daemon recovery, FUA, discard, write-zeroes, online topology mutation, or hardware safety. SQLite remains an evaluation-only recovery adapter; this run does not select production journal, synchronization, checkpoint, or connection settings.

## Evidence run

Run on 2026-08-09 with `tools/linux-disk-acceptance/run.sh`:

- Source archive SHA-256: `522259d739f8caf723581e0b0791b0ef7de3df667fc514c8345ab8d90c0cb547`.
- Guest: Ubuntu arm64, kernel `7.0.0-28-generic`, real `/dev/ublkb0` endpoint.
- Machine-readable evidence: `verification/os-031-linux-ublk-ext4.json`.
- Retained live traces: `verification/os-031-linux-ublk-trace-first.json` (450,485 bytes, 458 records, SHA-256 `040a6515fa324d73af00a4771c41fd06c22d46a29d7c7f70ce278918d0d6eb57`) and `verification/os-031-linux-ublk-trace-second.json` (79,533 bytes, 81 records, SHA-256 `73eaf4ca520bf9840499a74354eafe777bed7233702edca311fb495928d5a8b7`).
- Both traces use `dwv.ublk.trace.v2`, record queue depth 8, maximum transfer 131,072 bytes, maximum 4,096 records, no exhaustion, and clean deterministic replay through `dwv demo disk trace-replay`.
- Workload: mkfs.ext4, mount, create, fsync, overwrite, rename, directory sync, read, delete, unmount, clean shutdown, restart, read-only remount.
- Durable file content SHA-256 before and after restart: `d4ad659dcd887413e31f0b6d272b2b353d29734c3cba9f1cb9b74ab45865f4d7`.
- Data and parity payload SHA-256 after shutdown: `33aa9543635fd4a41a7f14e10fcab56e5b857809ec2e32c5337b2fd5b8140762`; byte equality passed.
- The ordinary data backing file mounted directly as read-only ext4 after service shutdown and exposed the same content.
- Both ublk runs ended in lifecycle state `stopped` only after drain, checkpoint, endpoint-removal, and trace-replay checks passed.

Negative cases failed closed for undersized geometry, unsupported topology, second-owner acquisition, cleanup against a differently owned endpoint, stale readiness, missing recovery authority, unsupported discard, unknown endpoint cleanup, and owner process death. Owner-death evidence required explicit owned-endpoint cleanup before successful reacquisition. Payload hashes were unchanged across pre-publication refusal cases. Partial multi-store fence coverage was rejected by the portable transaction regression executed in the guest.

## Portable and proof evidence

- `cargo test --workspace`: 274 passed across 34 suites; 1 explicitly ignored hardware-dependent test.
- `cargo test -p dwv-frontend-ublk` in the Linux guest: 11 passed, including bounded trace replay/divergence, pre-admission reservation, stale/duplicate completion, lifecycle refusal, and probe/shutdown classification.
- `cargo test -p dwv-transaction-ref partial_multi_store_fence_is_rejected` in the Linux guest: passed.
- `cargo test --test cli_demo`: covers portable trace replay, malformed trace refusal, oversized trace refusal, lifecycle confirmation, and source-preserving replay.
- TLC 2.19 on `verification/tla/RecoveryProtocol.tla`: 234 states generated, 125 distinct states, complete depth 9, all configured invariants passed. The model composes durable dirty intent, data/parity mutation, per-store fences, recovery checkpointing, clean publication, and crash/restart handoff.
- Kani 0.67.0 `dirty_region_mapping_covers_every_intersection_once`: 0 of 584 checks failed, 7 unreachable.
- Kani 0.67.0 `fence_coverage_requires_every_store_region_and_incarnation`: 0 of 893 checks failed, 6 unreachable.

The deterministic recovery model exercises crash cuts before durable intent and after intent, protected mutation, fence, and checkpoint. The before-intent mutant is retained as a regression requiring `ReconciliationRequired`; clean publication is permitted only after durable intent, complete protected mutation, complete store-fence coverage, and recovery checkpoint evidence. Trace replay is validation-only: it rechecks adapter translation and completion mapping without opening a fixture, backend, or device, mutating payload bytes, or claiming application consumption or physical durability.
