# ADR: VE-001 bounded arithmetic and pure-component verification

- **Status:** Accepted for the portable evidence lane
- **Date:** 2026-08-08
- **Scope:** `dwv-core` geometry, `dwv-service` range decomposition, and
  `dwv-codec` XOR geometry/parity update/reconstruction

## Decision

Use Kani 0.67.0 through the repository's pinned `mise.toml` tool for the
small, high-consequence pure seams. The formal portfolio is deliberately
bounded and split by property:

| Harness | Property | VP support |
|---|---|---|
| `dwv-core::byte_range_constructor_matches_checked_add` | checked range end arithmetic | VP-002 |
| `dwv-core::geometry_512_acceptance_is_exact_and_reachable` | 512-byte geometry acceptance and rejection | VP-002 |
| `dwv-core::geometry_4096_acceptance_is_exact_and_reachable` | 4096-byte geometry acceptance and rejection | VP-002 |
| `dwv-service::split_range_math_preserves_aligned_coverage` | fixed-array split arithmetic, bounds, alignment, and coverage | VP-002 |
| `dwv-codec::full_parity_matches_explicit_xor` | bounded reference parity equivalence | VP-001 |
| `dwv-codec::incremental_update_matches_full_recomputation` | bounded incremental-update equivalence | VP-001 |
| `dwv-codec::fixed_single_erasure_reconstructs_exactly` | bounded single-erasure reconstruction | VP-001 |

The service harness intentionally proves a fixed-array arithmetic model rather
than the public `Vec`-allocating wrapper. The wrapper's list/allocation and
error-formatting behavior remains covered by the exhaustive Rust tests and
property/fuzz layers. This avoids making a verifier-friendly representation
the production representation.

The bounded Rust tests remain required complementary evidence. They enumerate:

- protected and parity lengths from zero through four logical blocks, with
  block sizes 1, 2, 4, and 512;
- aligned range starts, lengths, and transfer limits through eight blocks;
- codec geometries `[0, 1, 2]`, `[1, 2, 3]`, and `[2, 0, 3]` bytes;
- every data vector over the byte alphabet `{0, 1, 255}` within those bounded
  geometries;
- every valid update range and replacement vector over the same alphabet;
- every single missing-slot reconstruction range, including logical-zero
  tails.

## Evidence and limits

Commands are reproducible with `mise exec -- cargo kani ...`. All seven
listed harnesses passed with no failed checks; Kani's `caller_location` and
foreign-function diagnostics were reported as successful checks, not proof
failures. The focused finite-domain tests and the integrated `dwv demo`
workflow also pass.

This evidence is complete only for the declared finite domains and the
fixed-array range arithmetic model. It is not a symbolic proof of arbitrary
`u64` inputs, unbounded allocation/list behavior, P/Q semantics, filesystem or
device I/O, concurrency, recovery ordering, or physical durability.
Future changes to these pure seams must preserve or extend both the Kani
portfolio and the bounded regression tests before claiming stronger VE-001
conformance.
