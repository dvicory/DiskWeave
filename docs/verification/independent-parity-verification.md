# Independent parity verification

## Scope

This record covers the standalone `dwv-independent-parity` command and its
experimental `dwv.independent-parity.v1` descriptor. It is file-backed/model
evidence only. The command is not production service, operator workflow,
frontend, SQLite, recovery-state inspection, repair, publication, migration,
or durable-format authority.

## Evidence run

On 2026-08-14, the focused executable evidence passed:

```text
cargo test -p dwv-verify --test independent_parity
2 passed; 0 failed
```

The test uses disposable regular files and checks BLAKE3 digests for both
payloads, parity, and the descriptor before and after every observation. It
covers:

- matching equations and a bounded mismatch;
- valid selected-range observation, outside-range corruption isolation, and
  human/structured projection equivalence for completed and refused results;
- deterministic repeated human and structured rendering;
- a short parity payload reported as `incomplete`;
- a short data payload using logical zero extension beyond its declared length;
- an unavailable payload reported as invalid input before any region result;
- duplicate JSON members, duplicate payload identities, and symlink-aliased
  payload identities;
- invalid selected-range geometry and aggregate buffer bounds;
- unsupported descriptor version and an oversized descriptor.

The test asserts that no fixture or descriptor digest changes, that reports
remain bounded, and that completed observations remain distinct from invalid
input. A mismatch is reported only as a bounded equation disposition; it does
not identify a bad payload or authorize repair.

The normal dependency boundary was checked with
`cargo tree -p dwv-verify --edges normal`. The verifier depends on the portable
codec/core/recovery/store crates and serialization support; it does not depend
on `dwv-service`, frontend adapters, SQLite recovery adapters, or the
independent recovery-inspection command.
This evidence does not establish hardware or platform durability, current or
historical protection, identity, custody, clean state, checksum authority,
repair, recovery, publication, migration, stable-format compatibility, or
production concurrency. Failed-read coverage here is bounded unavailable-file
admission plus incomplete regular-file coverage; the implementation's
`unknown` read-disposition and exit-1 output-failure branches are not induced
by this deterministic fixture suite. It does not claim arbitrary filesystem or
output-channel fault injection.
