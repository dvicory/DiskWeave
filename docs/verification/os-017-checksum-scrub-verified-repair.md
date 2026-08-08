# OS-017 checksum scrub and verified repair verification

## Claim boundary

This record covers the portable single-parity, regular-file demo path. It proves
read-only exhaustive scrub classification, identity/generation-bound plan
validation, separate-target reconstruction, target readback, digest generation,
parity-equation verification, and durable replacement-fence evidence.

It does not claim in-place member replacement, online scrub/rebuild, P/Q repair,
degraded writes, Linux behavior, FSKit/DiskImages attachment, a stable checksum
format, or physical power-loss durability. The demo deliberately does not
publish a protected-member integrity record for a replacement target whose
membership mapping has not been committed.

## Implementation evidence

- `dwv-verify::plan_scrub` captures the exhaustive report, source identities,
  replacement identity, topology/recovery generation, and checksum-set generation.
- `dwv-verify::apply_scrub` rejects sampled reports, stale generations, changed
  identities, aliased targets, and anything other than one unique candidate
  before invoking the existing separate-target repair path.
- `dwv-verify::apply_repair` reconstructs the identified data or parity range,
  writes only the replacement target, reads it back, checks the parity equation,
  and returns the repaired digest.
- `dwv demo scrub` builds independent evidence from disposable reference files,
  performs zero payload writes, emits bounded classifications, and persists a
  confirmation-bound scrub plan only for a unique candidate.
- `dwv demo repair` revalidates the saved plan against current identities and
  recovery generations, applies the separate-target repair, and requires a
  durable replacement fence before reporting success.

## Executable evidence

The following targeted checks passed on 2026-08-08:

- `cargo fmt --all`
- `cargo test -p dwv-verify` — 20 tests passed.
- `cargo test -p diskweave -p dwv-verify` — 21 tests passed.
- Clean `dwv demo init` followed by `dwv demo scrub` — exhaustive match,
  `payload_writes: 0`, no repair candidate.
- One-byte data corruption followed by `dwv demo scrub` —
  `Mismatch(DataIdentified { slot: 1 })`, one repair candidate, zero payload
  writes.
- Confirmed `dwv demo repair` — separate-target outcome, readback digest,
  parity verification, and `StoreFenceRef` for the replacement.
- One-byte parity corruption with clean data evidence —
  `Mismatch(ParityIdentified)`, followed by a verified separate-target parity
  repair.
- Simultaneous data and parity corruption — `Mismatch(Ambiguous)`, no repair
  plan, and no source writes.

## Decision boundary

The implementation is a portable evidence and orchestration slice, not a
production scrub scheduler or final checksum-format decision. A later release
must bind replacement assignment and checksum publication to the protected
member topology before treating a repaired target as an authoritative member.
