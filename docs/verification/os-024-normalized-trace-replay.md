# OS-024 normalized trace and replay verification

## Claim boundary

This evidence covers the bounded portable trace model and one deterministic
symbolic workflow replayed through the volatile-media simulator and a copied
ordinary regular file using `FileStore`. It covers semantic I/O, flush,
recovery intent/checkpoint, checksum, degraded-read, rebuild, repair, refusal,
and successful terminal events without serializing raw bytes, paths, SQLite
rows, or backend handles.

It does not claim Linux ublk behavior, a live macOS bridge, physical-device
power-loss durability, online rebuild, degraded writes, P/Q semantics, or
backend equivalence for operations not represented by the trace vocabulary.

## Implementation evidence

- `dwv-trace` owns the versioned canonical JSON model, strict unknown-field and
  range validation, 512-event/1 MiB bounds, schema-0 migration, symbolic
  payload generation, model summaries, and failing-prefix minimization.
- `dwv demo trace-export` writes the 11-event deterministic workflow.
- `dwv demo trace-render` validates and renders event summaries without replay.
- `dwv demo trace-replay` executes write/read/flush operations through
  `dwv-sim` and a copied regular file, then compares payload/parity digests and
  integrity/recovery/terminal summaries. The caller fixture remains unchanged.

## Executable evidence

Targeted checks passed on 2026-08-08:

- `cargo test -p dwv-trace` — model tests passed.
- `cargo test -p diskweave --test cli_demo` — lifecycle and trace boundary
  tests passed.
- Clean disposable fixture: `trace-export`, `trace-render`, and `trace-replay`
  returned `equivalent: true` with 11 events and matching simulator/file
  payload and parity digests.
- Schema-0 trace rendering migrated to schema 1.
- Malformed and oversized trace files were refused before replay.
- A one-byte source divergence produced exit code 4 and a minimized
  `trace-minimized.json`; restoring the source returned the fixture to its
  original bytes.

## Deferred boundaries

The trace is a portable semantic fixture, not a replacement for the existing
transaction trace or a universal recorder. Future operation vocabularies must
be added explicitly before claiming replay coverage for them.

- Repository verification passed: `cargo fmt --all -- --check`, `cargo test
  --workspace` (224 passed, 1 ignored), workspace `cargo clippy` with
  `-D warnings`, `cargo metadata --format-version 1 --no-deps`, and
  `cargo tree --workspace -e normal`.
- `openspec validate --all --strict --json` — 22/22 items passed; informational
  long-requirement notices remain on pre-existing specifications.
