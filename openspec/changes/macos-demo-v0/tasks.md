## 1. CLI contract boundary

- [x] 1.1 Add the root `dwv` binary target with a bounded parser for the experimental demo command set.
- [x] 1.2 Add versioned result envelopes, human rendering, stable diagnostics, and exit-class mapping.
- [x] 1.3 Add plan serialization and confirmation-token derivation from canonical plan contents.

## 2. Disposable fixture lifecycle

- [x] 2.1 Add the versioned fixture manifest with owned-root path resolution and identity validation.
- [x] 2.2 Implement deterministic fixture initialization with ordinary data/parity files and semantic SQLite recovery state.
- [x] 2.3 Implement status, inspection, capability, and exhaustive-verification command paths without payload mutation.

## 3. End-to-end macOS demo

- [x] 3.1 Exercise healthy read/write/flush behavior and close/reopen persistence through the existing service boundary.
- [x] 3.2 Implement known-erasure degraded-read and separate-target resumable rebuild orchestration through existing adapters.
- [x] 3.3 Implement rebuild planning, confirmation matching, stale-plan refusal, final verification, and source/parity preservation reporting.
- [x] 3.4 Keep daemon, bridge, Linux, physical-durability, stable-format, P/Q, and degraded-write claims out of the demo.

## 4. Acceptance and evidence

- [x] 4.1 Add process-boundary CLI tests for usage, JSON output, refusal classes, fixture lifecycle, reopen, rebuild, and alias safety.
- [x] 4.2 Add macOS demo documentation with commands, fixture ownership, evidence output, and unsupported behavior.
- [x] 4.3 Run targeted smoke tests, workspace validation, strict Clippy, dependency inspection, and OpenSpec validation.
- [x] 4.4 Record requirement/scenario evidence and archive only after verification finds no critical issue.
