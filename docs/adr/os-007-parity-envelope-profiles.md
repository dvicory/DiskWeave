# ADR: OS-007 parity-envelope profile baseline

- **Status:** Provisional / format-experimental
- **Date:** 2026-08-08
- **Scope:** Portable parity-device metadata evidence only

## Decision

Keep Profile B (two bounded redundant session-envelope copies) as the
provisional comparison baseline. Keep Profile A (bare parity plus external
recovery state) as the compatibility and fallback interpretation. Keep Profile C
(Profile B plus a coarse dirty-region bitmap) diagnostic until simulation and
representative media measurements show that its avoided verification work
justifies the extra metadata durability participant and write amplification.

This decision does not select a stable on-disk format, enable the Gate-H clean
fast path, or add metadata to ordinary data members.

## Evidence

The portable format implementation provides:

- exact protected-capacity accounting for all three profiles;
- fixed 4096-byte envelope copies with bounded canonical bodies and checksums;
- independent bounded inspection with required/compatible feature handling;
- conservative matching-copy, missing-copy, torn-copy, stale, clone-ambiguous,
  and conflict assessment;
- simulator schedules for dirty-session ordering, fence/checkpoint gating,
  crashes, torn copies, disagreement, and interrupted migration.

The disposable file-backed demo reports, for a 16,384-byte protected range:

| Profile | Metadata bytes | Payload capacity | Result |
|---|---:|---:|---|
| A | 0 | 24,577 | accepted |
| B | 8,192 | 16,385 | accepted |
| C | 8,193 | 16,384 | accepted |

The same demo independently decodes both Profile B copies as matching clean
session evidence while reporting `clean_recovery_authorized: false`. Clean
certificate use therefore remains outside this change's authority.

## Consequences

- Profile B can be exercised without changing ordinary data-member bytes or
  the XOR parity equation.
- Profile C's bitmap is bounded and does not contain a checksum table, but its
  foreground durability cost is not yet justified for production use.
- Envelope bytes and offsets remain disposable experimental artifacts. A later
  stable-format change must define import, migration, rollback, and independent
  recovery tooling before protecting non-disposable data.
