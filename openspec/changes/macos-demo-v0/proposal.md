## Why

DiskWeave currently has portable parity, recovery, verification, and file-store libraries but no operator-visible path that runs the intended macOS demo end to end. The architecture names `dwv` and `dwvd` while leaving their first observable contract open; this change defines the smallest experimental CLI boundary needed to exercise existing file-backed semantics without prematurely freezing the production command tree or live macOS bridge.

## What Changes

- Define an experimental `macOS-demo-v0` CLI contract for a documented, disposable file-backed single-XOR demo.
- Add a minimal `dwv` command surface for fixture setup, structured status, inspection/evidence, exhaustive verification, rebuild planning/execution, and evidence reporting.
- Define versioned machine-readable result envelopes, human-readable rendering, exit classes, and unsupported/blocked/uncertain outcomes.
- Require state-changing workflows to display an identity- and generation-bound plan before accepting a matching confirmation value.
- Compose existing `dwv-service`, `dwv-verify`, `dwv-recovery`, `dwv-store-file`, and `dwv-recovery-sqlite` seams rather than duplicating parity or recovery policy in argument handlers.
- Add a deterministic disposable fixture and end-to-end smoke path covering healthy operation, reopen, known-erasure degraded reads, resumable separate-target rebuild, final verification, and source/parity preservation.
- Keep daemon interaction behind a replaceable local boundary; the first offline-capable demo must not require a live FSKit, DiskImages, macFUSE, or Linux frontend.
- Record the exact portable/macOS claim boundary and leave live bridge, physical durability, stable format, P/Q, degraded writes, and Linux integration out of scope.

## Capabilities

### New Capabilities

- `macos-demo-cli`: Experimental operator CLI contract and runnable disposable macOS file-backed demo workflow.

### Modified Capabilities

<!-- No existing capability requirements are changed; this change composes the existing portable APIs at a new operator boundary. -->
