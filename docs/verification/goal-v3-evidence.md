# Goal-v3 verification evidence log

Date: 2026-08-08

## Baseline

The pre-remediation portable workflow is recorded in
[`goal-v3-baseline.md`](goal-v3-baseline.md), including the disposable fixture,
commands, outputs, and claim boundary.

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

## Repository checks

```text
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo metadata --format-version 1 --no-deps
openspec validate --all --strict --json
```

Observed: 224 tests passed across 29 suites with one ignored; formatting,
Clippy, metadata, and strict OpenSpec validation passed. OpenSpec validation
reported 21/21 items valid; informational long-requirement notices are not
failures.
