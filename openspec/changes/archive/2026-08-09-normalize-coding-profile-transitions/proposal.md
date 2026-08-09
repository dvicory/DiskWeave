## Why

The v0.8 normalization audit maps topology transactions at section granularity, but canonical requirements do not retain every plan binding and operation-specific safety condition from §§6.6 and 7.8. In particular, no requirement prevents existing parity bytes from being reinterpreted under a changed profile, parity-role count, coding position, or protected geometry. DiskWeave needs an exact timeless owner for this transition contract before Goal-v7 relies on the normalization record.

## What Changes

- Extend staged topology-transition semantics to cover coding-profile, parity-role-count, coding-position, and protected-geometry changes explicitly.
- Bind every transition plan to exact array, topology, assignment, geometry, coding-profile, execution-mode, recovery, rollback, and verification inputs.
- Preserve the operation-specific safety boundaries for data-slot addition/removal and protected-capacity changes without introducing namespace ownership.
- Keep the old topology authoritative until target parity is built, independently verified, durably committed, and published.
- Refuse unsupported profile migrations before mutation without promising online migration or defining a future P/Q algorithm.
- Correct the v0.8 normalization evidence with an exact fragment-level disposition.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `anchorless-topology-identity`: Make coding-profile transitions explicit staged migrations with fail-closed unsupported behavior and no parity-byte reinterpretation.

## Impact

- The canonical topology-transition requirement and its semantic fingerprint change.
- The architecture-normalization evidence gains an exact disposition for v0.8 coding-profile-transition clauses.
- No runtime implementation, persistent format, coding algorithm, or supported topology changes.
