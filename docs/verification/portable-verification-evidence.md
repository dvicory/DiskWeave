# Portable verification evidence log

Date: 2026-08-08

## Baseline

The pre-remediation portable workflow is recorded in
[`portable-demo-baseline.md`](portable-demo-baseline.md), including the
disposable fixture, commands, outputs, and claim boundary.

## VE-002 focused evidence

Source: `verification/tla/RecoveryProtocol.tla`
Configuration: `verification/tla/RecoveryProtocol.cfg`

Commands:

```text
mise exec -- java -cp /tmp/dwv-tla2tools-1.7.4.jar pcal.trans verification/tla/RecoveryProtocol.tla
mise exec -- java -cp /tmp/dwv-tla2tools-1.7.4.jar tlc2.TLC -config verification/tla/RecoveryProtocol.cfg verification/tla/RecoveryProtocol.tla
tla verification/tla/RecoveryProtocol.tla --config verification/tla/RecoveryProtocol.cfg --json
```

Observed:

- PlusCal translation completed with no parse errors.
- TLC: no error; 82 states generated, 53 distinct states, complete depth 9;
  all six invariants passed.
- `tla-rs`/`tla-checker` 0.6.11: `status: ok`, 53 states explored, 81
  transitions, maximum depth 9.
- A temporary home-write guard mutation was rejected by both checkers with
  `MutationRequiresIntent` at depth 2.

## VE-001 focused evidence

Source tests:

- `crates/dwv-core/src/topology.rs`
- `crates/dwv-service/src/range.rs`
- `crates/dwv-codec/src/lib.rs`

Commands:

```text
cargo test -p dwv-core --lib bounded_geometry_accepts_only_valid_capacity_and_alignment
cargo test -p dwv-service --lib bounded_split_ranges_preserve_aligned_coverage
cargo test -p dwv-codec --lib bounded_geometry_and_parity_exhaustive
```

Observed: all three focused tests passed. The finite harness covers block
sizes 1, 2, 4, and 512; capacities through four blocks; range splitting
through eight blocks; every bounded data vector over `{0, 1, 255}` for the
selected geometries; every valid parity-update range/replacement vector; and
every single-erasure reconstruction range, including zero-extended tails.
The evidence is bounded to those domains and does not claim arbitrary-width
formal proof, P/Q semantics, I/O, concurrency, or durability.

The bounded suite is complementary evidence, not a substitute for formal
verification. The Kani portfolio is:

```text
cargo kani -p dwv-core --harness byte_range_constructor_matches_checked_add
cargo kani -p dwv-core --harness geometry_512_acceptance_is_exact_and_reachable
cargo kani -p dwv-core --harness geometry_4096_acceptance_is_exact_and_reachable
cargo kani -p dwv-service --harness split_range_math_preserves_aligned_coverage
cargo kani -p dwv-codec --harness full_parity_matches_explicit_xor
cargo kani -p dwv-codec --harness incremental_update_matches_full_recomputation
cargo kani -p dwv-codec --harness fixed_single_erasure_reconstructs_exactly
```

Observed: all seven harnesses completed with no failed checks. Geometry
reachability covers for valid and invalid inputs were satisfied. The direct
`Vec`-backed service harness was canceled after symbolic execution became
pathological; the replacement fixed-array arithmetic harness completed in
2.66 seconds and proves VP-002's bounded split/coverage arithmetic. Public
list/allocation and error-formatting behavior remains covered by the bounded
Rust tests and later property/fuzz layers. Kani diagnostics for
`caller_location` and foreign functions were emitted as successful checks,
not proof failures.

Kani claim map: the core and service harnesses support VP-002; the codec
parity, incremental-update, and single-erasure harnesses support VP-001.
These runs do not establish arbitrary-width `u64` proof, P/Q behavior,
concurrency, recovery ordering, filesystem/device I/O, or physical durability.

## VE-005 initial structured corpus evidence

Source: `verification/corpus/trace-seeds.json` and the seeded
`dwv-trace` parser-mutation test.

Command:

```text
cargo test -p dwv-trace --lib seeded_trace_corpus_round_trips_and_rejects_or_normalizes_mutations
```

Observed: 24 retained seeds generate valid traces across 512-byte through
16 KiB fixtures with bounded writes and optional read, degraded-read, rebuild,
checksum, and terminal-outcome events. Each generated trace round-trips through
JSON. Deterministic byte mutations are accepted only when the parser produces
a trace that also canonical-round-trips; malformed or invalid mutations are
refused. The corpus contains seeds, not raw payloads.

This is initial VP-010 hostile trace-input and VP-011 reproducibility evidence.
It does not claim broad simulator operation/fault/crash/topology schedules,
parity generation, concurrency, filesystem behavior, or hardware durability.

## VE-005 seeded simulator schedule corpus evidence

Source: `verification/corpus/simulator-schedule-seeds.txt` and the seeded
`dwv-sim` schedule/replay test.

Command:

```text
cargo test -p dwv-sim --lib seeded_operation_and_fault_schedules_replay_stably
```

Observed: 16 retained seeds generate bounded simulator schedules with
operation submissions, delivery, write faults, recovery/envelope commits,
latent corruption, daemon/controller interruption, power loss, and store
disappearance/reappearance. Every schedule round-trips through the simulator
reproducer format, replays deterministically, drains pending work, and ends
with the store available. The corpus retains seeds and producer logic, not
payload data.

This supports initial VP-005, VP-008, VP-010, and VP-011 evidence. It does not
claim exhaustive schedule coverage, topology generation, parity equivalence,
concurrency, filesystem behavior, or hardware durability.

## Integrated portable checkpoint

Disposable root: `/tmp/dwv-v3-followup`

Commands, in order:

```text
cargo run -q --bin dwv -- demo init --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo run --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo status --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo inspect --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo capabilities --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo verify --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo trace-export --root /tmp/dwv-v3-followup --trace verification-trace.json
cargo run -q --bin dwv -- demo trace-replay --root /tmp/dwv-v3-followup --trace verification-trace.json
```

Observed:

- `demo.init`, `demo.run`, `demo.status`, `demo.inspect`, `demo.capabilities`,
  `demo.verify`, `demo.trace-export`, and `demo.trace-replay` all succeeded.
- Healthy write/read/flush/reopen matched.
- Degraded read matched the reference; four rebuild chunks completed; final
  verification passed; replacement bytes equaled the reference; source parity
  remained unchanged.
- Exhaustive verification reported clean with zero writes.
- Trace export/render/replay remained equivalent with 11 events.
- The CLI continued to report live bridge, Linux, physical durability, P/Q,
  and degraded writes as unsupported.

## Post-VE-001 integrated checkpoint

Disposable root: `/tmp/dwv-v3-ve001`

Commands, in order:

```text
cargo run -q --bin dwv -- demo init --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo run --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo status --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo inspect --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo capabilities --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo verify --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo trace-export --root /tmp/dwv-v3-ve001 --trace verification-trace.json
cargo run -q --bin dwv -- demo trace-replay --root /tmp/dwv-v3-ve001 --trace verification-trace.json
```

Observed: all eight commands succeeded. Healthy read/write/flush/reopen
matched; degraded read and four-chunk rebuild matched the reference; final
verification passed; replacement bytes matched; source parity remained
unchanged; exhaustive verification reported clean with zero writes; and
trace replay was equivalent across 11 events. Unsupported bridge, Linux,
physical durability, P/Q, and degraded-write claims remained explicit.

## Repository checks


```text
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo metadata --format-version 1 --no-deps
openspec validate --all --strict --json
```

Observed: 227 tests passed across 29 suites with one ignored; formatting,
Clippy, metadata, and strict OpenSpec validation passed. OpenSpec validation
reported 21/21 items valid; informational long-requirement notices are not
failures.
