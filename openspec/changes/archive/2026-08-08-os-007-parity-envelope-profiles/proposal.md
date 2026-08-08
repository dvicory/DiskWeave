## Why

DiskWeave has a provisional parity-device metadata-envelope direction but no
implemented comparison or independent decoder. Before recovery-state loss,
torn metadata, or future clean certificates can rely on an envelope, the
portable project needs measured evidence for its byte boundary, crash behavior,
and recovery interpretation.

## What Changes

- Add a disposable, format-experimental comparison of bare parity, redundant
  envelope, and envelope-plus-bitmap profiles.
- Define bounded encode/decode and exact protected-capacity accounting.
- Add an independent decoder/inspector that does not depend on the writer's
  in-memory types.
- Model torn, missing, disagreeing, stale, and unknown-feature envelope copies
  conservatively.
- Exercise interrupted migration and session-state reconciliation through the
  simulator and file-backed fixtures.
- Record a provisional profile choice or an explicit fallback ADR; do not claim
  a stable on-disk format or enable the Gate-H clean fast path.

## Capabilities

### New Capabilities

- `parity-envelope-profiles`: Experimental parity-device envelope profiles,
  independent inspection, capacity rules, and conservative crash/recovery
  interpretation.

### Modified Capabilities

<!-- No existing requirement changes; this adds the previously skipped
     format-evidence capability. -->

## Impact

- Adds a portable format/inspection seam, likely as a new workspace crate or
  narrowly scoped format module.
- Touches recovery-envelope integration, simulator schedules, disposable
  parity fixtures, and verification documentation.
- Adds no required bytes to ordinary data members and does not alter the
  portable parity equation.
- Persistent envelope bytes remain experimental and disposable; no compatibility
  migration is promised.
- No Linux, live macOS bridge, frontend, or production durability dependency.
