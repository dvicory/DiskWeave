## 1. Authority Assessment Model

- [x] 1.1 Add the typed lineage, custody, basis-coverage, and non-authorization fields to `OperatorResult`, and cut every operator result over to `dwv.operator.v2`.
- [x] 1.2 Implement one conservative classifier that reuses current member, topology, and recovery observations; produces checked, bounded role-local coverage; and never infers current, prior, or unprotected basis from missing or unrelated evidence.

## 2. Operator Integration

- [x] 2.1 Integrate authority classification into status, members, scrub, damage, recovery preview, and error results, and remove identity-derived “current parity,” remaining-redundancy, writable-start, and next-action claims from observation output.
- [x] 2.2 Update the human renderer and JSON output so both preserve the same authority dimensions, coverage totals, bounded exceptions, blocker, and explicit non-authorization statement.

## 3. Contract Evidence

- [x] 3.1 Extend `tests/operator_cli.rs` to prove that a complete recognized stopped array reports accepted lineage but no proved custody or current basis, that missing/unreadable/ambiguous evidence stays conservative, that every observation command leaves payload and recovery bytes unchanged, and that human and v2 JSON meanings match.
- [x] 3.2 Add focused classifier tests for mixed role-local basis aggregation, exact indeterminate ranges, checked totals, and bounded exceptional detail without adding recovery persistence or mutation fixtures.
- [x] 3.3 Run `cargo test -p diskweave --bin dwv operator::tests` and `cargo test -p diskweave --test operator_cli`; fix every failure without weakening the C0a claim boundary.

## 4. Repository Gates

- [x] 4.1 Run `cargo xtask docs knowledge readiness`, `cargo xtask docs check`, and `cargo xtask docs build`; resolve every diagnostic attributable to this change before verification and archive.
