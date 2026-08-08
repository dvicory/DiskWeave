# macOS file-backed demo

The `dwv` binary exposes a disposable, file-backed demonstration of the portable DiskWeave boundary. It does not attach a virtual disk and does not claim FSKit, DiskImages, Linux frontend, physical power-loss, P/Q, or degraded-write behavior.

## Commands

Use a missing or empty directory owned by the current run:

```sh
ROOT="$(mktemp -d /tmp/dwv-demo.XXXXXX)"
cargo run --bin dwv -- demo init --root "$ROOT"
cargo run --bin dwv -- demo status --root "$ROOT"
cargo run --bin dwv -- demo inspect --root "$ROOT"
cargo run --bin dwv -- demo capabilities --root "$ROOT"
cargo run --bin dwv -- demo verify --root "$ROOT"
```

`init` creates ordinary `data0.raw`, `data1.raw`, and `parity.raw` payloads, a byte-equal `reference.raw`, a zeroed `replacement.raw`, a versioned `fixture.json` with expected file identities, and semantic SQLite recovery state. It refuses a non-empty or unrecognized root.

Run the complete smoke workflow:

```sh
cargo run --bin dwv -- demo run --root "$ROOT"
```

This exercises healthy service read/write/flush, closes and reopens a member, authorizes a single known erasure, performs an offline separate-target rebuild, verifies the replacement, and reports that source parity was preserved.

For an explicit rebuild plan and confirmation boundary:

```sh
PLAN="rebuild-plan.json"
PLAN_JSON="$(cargo run --quiet --bin dwv -- demo plan --root "$ROOT")"
TOKEN="$(printf '%s' "$PLAN_JSON" | jq -r .confirmation)"
cargo run --bin dwv -- demo rebuild --root "$ROOT" --plan "$PLAN" --confirm "$TOKEN"
```

The plan binds the rebuild ID, missing slot, topology epoch, recovery generation, replacement identity, protected length, and chunk size. A wrong token, stale generation, identity change, topology change, path escape, or replacement alias is refused before payload mutation.

To exercise interruption/resume deterministically, stop after durable chunk checkpoints and then rerun the same confirmed plan:

```sh
cargo run --bin dwv -- demo rebuild --root "$ROOT" --plan "$PLAN" --confirm "$TOKEN" --stop-after 2
cargo run --bin dwv -- demo rebuild --root "$ROOT" --plan "$PLAN" --confirm "$TOKEN"
```

The first command leaves final verification pending; the second resumes from the persisted cursor and performs final replacement verification.

Successful commands print JSON to stdout with `schema`/`contract: "dwv.cli.v0"`, `command: "demo.<name>"`, `ok: true`, `outcome`, `diagnostics`, and command-specific evidence. Errors print the same envelope with `ok: false`, a stable error class, and exit classes: `2` usage, `3` blocked/unavailable prerequisite, `4` refused unsafe or stale operation, and `5` operation failure.

The fixture root is disposable. There is intentionally no command that deletes it; remove it manually only after confirming the path is the generated fixture root.

## Verification evidence

Observed on 2026-08-08:

- `cargo test --workspace`: 187 passed, 1 ignored.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo metadata --format-version 1 --no-deps`: passed.
- `openspec validate macos-demo-v0 --strict --json`: 1/1 valid.
- A fresh disposable fixture completed `demo.run` with `command: "demo.run"`, `workflow: "complete"`, final verification `passed`, and `source_parity_preserved: true`.
- The process-boundary test covers usage, missing/corrupt recovery metadata refusal, wrong confirmation, durable checkpoint/resume, identity replacement, final rebuild verification, and an outside-root alias refusal.
