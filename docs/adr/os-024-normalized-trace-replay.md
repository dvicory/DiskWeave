# ADR: OS-024 normalized trace and replay

- **Status:** Accepted for the portable disposable fixture; not a stable
  production trace-format commitment
- **Date:** 2026-08-08
- **Scope:** Semantic trace export, validation, and replay across portable
  simulator and regular-file backends

## Decision

Adopt a pure `dwv-trace` model with bounded canonical JSON, symbolic payload
patterns, strict validation, and one schema-0-to-schema-1 migration. Keep
backend execution in the existing simulator and `FileStore` seams. Replay
compares payload/parity digests and normalized integrity, recovery, and
terminal state; it never mutates the caller's fixture.

The trace vocabulary explicitly records I/O, durability, recovery,
checksum, degraded-read, rebuild, repair, refusal, and uncertain semantic
boundaries. It does not encode raw payloads, host paths, SQLite rows, or
backend handles.

## Consequences

- Portable semantic regressions can be exported, reviewed, minimized, and
  replayed without Linux or a live macOS bridge.
- A copied regular-file fixture incurs setup I/O but protects the caller's
  source media.
- The vocabulary is intentionally bounded. New operation classes require an
  explicit schema change before claiming replay coverage.
- This ADR does not claim Linux, bridge, physical durability, online rebuild,
  degraded writes, or P/Q equivalence.

## Evidence

See `docs/verification/os-024-normalized-trace-replay.md` and the strict
workspace/OpenSpec checks recorded there.
