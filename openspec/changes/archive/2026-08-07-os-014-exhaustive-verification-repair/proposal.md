## Why

OS-013 can produce a healthy single-parity array, but the handoff requires a
portable way to distinguish a matching equation from an unexplained mismatch
before metadata-loss recovery or degraded service. Handoff Sections 8.10,
12.4, 26.3, and 26.6, together with the Phase 1 OS-014 row, require exhaustive
verification, zero writes for matching regions, and evidence-gated repair
without guessing which shard is wrong.

## What Changes

- Add a bounded, full-range parity verification model for data and single-XOR
  parity members.
- Produce per-region verification reports that distinguish matches, identified
  parity faults, identified data faults, ambiguous mismatches, and evidence
  conflicts.
- Add selective repair planning that permits only uniquely identified repairs
  with current independent checksum evidence and prefers a separate target.
- Verify repaired bytes and the parity equation before a repair result can be
  accepted; preserve the original mismatch report.
- Keep sampling explicitly diagnostic and unable to establish `CLEAN`.
- Add portable in-memory and macOS regular-file tests while leaving metadata
  loss, degraded reads, and resumable rebuild as OS-015/016 work.

## Capabilities

### New Capabilities

- `parity-verification-repair`: Exhaustive single-parity verification and
  conservative, checksum-gated selective repair decisions.

### Modified Capabilities

None.

## Impact

- Adds a portable verification/planning crate and an adapter seam for ordinary
  file-backed stores.
- Extends macOS file-backed integration coverage without adding FSKit,
  DiskImages, Linux, or hardware semantics to the core.
- Consumes OS-011 checksum records and OS-013 topology/store boundaries; it
  does not create a new persistent array format or authorize destructive
  in-place repair by default.
