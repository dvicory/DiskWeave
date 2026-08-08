# ADR: VE-001 bounded arithmetic and pure-component verification

- **Status:** Accepted for the portable evidence lane
- **Date:** 2026-08-08
- **Scope:** `dwv-core` geometry, `dwv-service` range decomposition, and
  `dwv-codec` XOR geometry/parity update/reconstruction

## Decision

Use dependency-free exhaustive finite-domain Rust tests as the bounded
verification mechanism for the current pure seams. Kani is not installed in
the host environment, and adding a verifier dependency or changing production
architecture is not justified for these small deterministic components.

The harness exhaustively enumerates:

- protected and parity lengths from zero through four logical blocks, with
  block sizes 1, 2, 4, and 512;
- aligned range starts, lengths, and transfer limits through eight blocks;
- codec geometries `[0, 1, 2]`, `[1, 2, 3]`, and `[2, 0, 3]` bytes;
- every data vector over the byte alphabet `{0, 1, 255}` within those bounded
  geometries;
- every valid update range and replacement vector over the same alphabet;
- every single missing-slot reconstruction range, including logical-zero
  tails.

The tests compare implementation results with an explicit parity calculation,
check exact range coverage and alignment, and verify reconstruction against the
original bounded data. Existing near-`u64::MAX` overflow tests remain part of
the arithmetic boundary evidence.

## Evidence and limits

Focused tests pass in `dwv-core`, `dwv-service`, and `dwv-codec`. This is a
complete check only for the enumerated finite domains. It is not a symbolic
proof of arbitrary `u64` inputs, allocation safety for unbounded lengths, P/Q
semantics, filesystem/device I/O, concurrency, or physical durability. Future
changes to these pure seams must preserve or extend the bounded harness before
claiming VE-001 conformance.
