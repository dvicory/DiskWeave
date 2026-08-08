## Why

The handoff requires loss of `array.sqlite3` to preserve ordinary data payloads while stopping unsafe writes, but the current portable implementation only exposes a conservative blocked state. OS-015 needs an executable recovery matrix so metadata loss becomes a reviewable plan with explicit evidence gates, a fresh-state path, and no silent clean or topology claims.

## What Changes

- Encode every Section 12.5 metadata-loss survivor/evidence case as a bounded semantic plan and recovery action.
- Gate the certified-clean fast path on the handoff’s session-certificate condition; route uncertified all-data cases through exhaustive verification and OS-014 evidence classification.
- Refuse writable assembly for ambiguous topology, cloned identities, lost Q coding positions, conflicting replicas, and hashless ambiguous mismatches.
- Add a fresh recovery-state builder for completed exhaustive all-data/single-parity recovery that records the loss event and creates a new checksum-baseline obligation without rewriting matching payload regions; keep the certificate optimization gated on a future Gate H receipt.
- Add an evaluation-only SQLite adapter operation and a macOS regular-file fixture for deleting/recreating recovery state while direct payload files remain readable.
- Add a bounded CLI-style dry run that prints the complete matrix without payload bytes or OS/runtime types.
- Keep P/Q execution, parity/new-lineage rebuild, and one- and two-erasure degraded reads/rebuild in their later OpenSpecs; those OS-015 matrix rows remain non-authorizing plans until verified completion receipts exist. Keep Linux frontend work out of scope.

## Capabilities

### New Capabilities

- `metadata-loss-recovery`: Conservative Section 12.5 planning, evidence-gated fresh recovery state, bounded dry-run reporting, and portable/macOS evaluation behavior.

### Modified Capabilities

- None.

## Impact

- Extends `dwv-recovery` with the metadata-loss matrix, authorization/evidence types, bounded audit record, and fresh semantic manifest builder.
- Extends `dwv-recovery-sqlite` with an evaluation-only fresh-state operation; it does not select a production SQLite binding or durability profile.
- Adds a `dwv-recovery` dry-run example and macOS temporary regular-file tests.
- No data-member format, Linux frontend, SQLite schema authority, or third-party dependency is introduced.
