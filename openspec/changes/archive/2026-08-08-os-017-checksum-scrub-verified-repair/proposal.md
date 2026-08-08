## Why

The repository has separate checksum evidence, exhaustive verification, repair planning, recovery-state, and file-store seams, but no integrated scrub path that safely turns their evidence into a verified repair outcome. OS-017 closes that gap after OS-016 by making scrub and repair behavior executable through the existing disposable CLI without guessing from parity disagreement.

## What Changes

- Add a portable scrub classifier for data, parity, ambiguous, conflicting, missing, stale, and unreadable evidence.
- Bind repair plans to the verification identity, topology/recovery generation, target range, and current checksum evidence.
- Route uniquely identified repairs through separate targets and verify target readback, digest, and parity equation before acceptance.
- Preserve source members and refuse repair for ambiguity, conflicting evidence, stale plans, failed writes, interrupted work, or beyond-tolerance faults.
- Exercise induced data/parity faults, absent or stale evidence, multiple suspects, target failures, crash boundaries, and checksum-set migration through deterministic tests and the existing demo CLI.

## Capabilities

### New Capabilities

- `checksum-scrub-verified-repair`: Evidence-gated scrub classification and separately targeted, readback-verified repair.

### Modified Capabilities

None.

## Impact

- Extends the portable recovery/verification integration and disposable file-backed demo path.
- Uses existing checksum, verification, recovery, and file-store contracts; no frontend, SQLite, Linux, FSKit, or hardware semantics enter the portable capability.
- Does not authorize online scrub/rebuild, degraded writes, P/Q repair, a stable checksum format, or automatic repair from parity disagreement alone.
