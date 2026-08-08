## Why

DiskWeave has deterministic simulators and semantic transaction traces, but no
bounded portable artifact that can reproduce an operator workflow across the
simulator and ordinary file-backed stores. A versioned trace fixture now makes
cross-layer regressions and uncertain outcomes replayable without exposing raw
payloads or waiting for Linux and bridge implementations.

## What Changes

- Add a bounded, versioned normalized trace model at the portable semantic
  boundary.
- Record privacy-safe lifecycle, I/O, durability, recovery, checksum,
  degraded-read, rebuild, and repair decisions without embedding payload bytes.
- Export, render, and replay traces through the existing `dwv demo` CLI.
- Replay the same trace against the deterministic simulator and the disposable
  file-backed path and compare normalized terminal state.
- Reject malformed, oversized, unsupported-version, and non-canonical traces;
  support one explicit schema migration.
- Emit a minimized failing trace for replay mismatches.

## Capabilities

### New Capabilities

- `normalized-trace-replay`: bounded semantic trace export, rendering,
  migration, validation, replay, and deterministic cross-backend comparison.

### Modified Capabilities

None.
