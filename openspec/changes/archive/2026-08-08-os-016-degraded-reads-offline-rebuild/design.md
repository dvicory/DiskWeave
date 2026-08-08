## Context

OS-013 supplies exact file-backed healthy I/O, OS-014 supplies exhaustive verification and separate-target repair primitives, and OS-015 supplies validated recovery topology plus fail-closed metadata-loss policy. The current service has no API that can declare one member a known erasure, and recovery state has only a generic maintenance cursor that cannot safely bind a rebuild to source/replacement identity.

The handoff requires one-known-erasure reads to be evidence-gated (Section 12.2), the first rebuild to be offline/read-only and resumable (Section 12.7), replacement to preserve logical slot/coding position while changing assignment identity (Section 7.6), and macOS acceptance to end with an independently readable raw image (Section 17.6).

## Goals / Non-Goals

**Goals:**

- Provide a portable capability object for one known single-XOR data erasure under a captured topology/recovery generation and explicit per-range recovery evidence.
- Reuse one reconstruction implementation for degraded reads, rebuild chunks, and final verification.
- Make replacement durability precede cursor durability and make resume validate every identity/generation field.
- Keep topology preparation separate from rebuild verification and publication.

**Non-Goals:**

- Degraded writes, online rebuild/write reconciliation, automatic EIO-to-erasure policy, P/Q or two-erasure decoding, Gate H certificate implementation, background scheduling/QoS, live APFS bridge behavior, and Linux frontend integration.
- Production physical durability claims for the macOS regular-file fixture.

## Decisions

### Put decode/rebuild mechanics in dedicated modules

`dwv-verify` will gain dedicated degraded-read and rebuild modules rather than expanding `lib.rs`. The core function consumes existing exact-range `VerificationStore` adapters and the in-tree `XorReference`; it returns opaque receipts/telemetry and never owns topology publication or recovery transactions. `dwv-service` remains the orchestration boundary that combines core topology, recovery state, and file-backed adapters.

This keeps codec math reusable and avoids embedding service policy in `dwv-codec`. Adding a new crate was rejected because the current dependency graph already has the needed portable seams and no independent package boundary is justified.

The dependency review was repeated on 2026-08-07. [`simd-rs63`](https://docs.rs/simd-rs63/latest/simd_rs63/) implements a fixed RS(9,6) layout over equal, alignment-constrained blocks; [`rune-xor`](https://docs.rs/rune-xor/latest/rune_xor/) implements repeating-key cipher operations; and [`gf2`](https://docs.rs/gf2/latest/gf2/) models packed bit-space vectors and matrices. None supplies DiskWeave's exact single-parity range contract, per-slot lengths, or documented zero-tail behavior. The dependency-free scalar `XorReference` therefore remains the auditable correctness oracle behind `ParityCodec`; a future optimized backend can implement that trait after conformance and benchmark evidence without changing recovery semantics.

### Use an opaque, generation-bound known-erasure authorization

The authorization records the array identity, topology epoch, recovery generation, missing stable slot/coding position, requested range disposition, and identities of every source. Construction validates a single-XOR profile, exactly one missing data assignment, unambiguous roles/positions, clean or replay-proven range coverage, quiesced writes, and non-excluded survivors. Degraded decoding rechecks source identities and returns degraded telemetry; callers cannot construct a successful receipt by supplying only a slot number.

Automatic conversion from arbitrary read EIO to a known erasure is deliberately excluded. The lifecycle/controller must establish the erasure and evidence first.

### Replace the generic maintenance cursor with typed rebuild state

`dwv-recovery` will add typed rebuild identifiers, source/replacement binding, lifecycle, and cursor mutations. The semantic schema advances with an explicit migration step and export limit. Existing generic maintenance checkpoints remain for unrelated jobs; rebuild correctness does not depend on an untyped `JobId`/integer pair.

The cursor means “first byte not durably materialized and verified.” A rebuild chunk orders reconstruction, separate-target write, exact readback/equation check, replacement flush/fence, then recovery transaction. If the final transaction is lost, replaying the chunk is safe; advancing before replacement durability is forbidden.

### Keep chunk execution and checkpoint commitment separable

The portable engine returns a `RebuildChunkReceipt` after replacement durability. Recovery state alone advances the cursor from that opaque receipt plus current generation. This makes crash cuts testable and avoids pretending an in-process loop can atomically commit raw-file writes and SQLite/recovery state.

The first implementation processes deterministic increasing ranges with a caller-supplied bounded chunk size. It does not hold range guards across unrelated waits because sources are globally read-only/quiesced for offline rebuild.

### Final verification gates prepared topology

After the cursor reaches the protected length, a complete pass re-reads replacement and survivors, checks exact reconstructed bytes/equations, and creates an opaque completion receipt. Only that receipt can create a candidate core topology preserving the stable slot and coding position while replacing assignment instance/generation/store mapping. The active topology is not changed by this helper; existing prepare/commit recovery mutations remain the publication boundary.

### Exercise the implementation with ordinary macOS files

Tests use `FileStore` wrappers for surviving data, parity, and a distinct replacement. They preserve a reference image outside the active source set, simulate interruption after a durable checkpoint, reopen state/target, resume, compare every byte, drop service/store leases, and read the replacement directly. This demonstrates portable behavior without a live APFS frontend or physical power-loss claim.

An environment-gated macOS integration fixture accepts a detached APFS disk image, rebuilds its complete bytes through the same engine, and leaves a replacement for independent read-only `hdiutil` attachment after the Rust process closes every store. This proves the OS-016 direct-attach boundary without claiming degraded APFS access through a DiskWeave frontend; that remains OS-022.

## Risks / Trade-offs

- [Risk] A clean bit without range/generation binding could authorize stale reconstruction. → Bind authorization to explicit range disposition, topology/recovery generation, and source identities; recheck before decode.
- [Risk] Cursor state can outrun durable replacement bytes. → Require an opaque post-flush chunk receipt before the generation-checked cursor mutation and test every crash cut around write/readback/flush/checkpoint.
- [Risk] Final verification could accidentally publish a replacement. → Return a prepared candidate only; retain explicit recovery prepare/commit and topology publication stages.
- [Risk] Large final verification duplicates rebuild I/O. → Accept the cost for the first offline correctness implementation; scheduling and incremental proof optimization belong to later changes.
- [Risk] `VerificationStore` currently lacks a durability operation. → Add a narrowly scoped rebuild-target extension trait; adapters must provide a real fence or return unknown/failure, never a default success.

## Migration Plan

1. Add the new semantic rebuild record and schema migration while retaining readability of v2 manifests.
2. Add portable known-erasure decode and chunk receipts with in-memory crash-cut tests.
3. Add service/file adapters and macOS ordinary-file acceptance.
4. Keep the feature read-only and offline until later OpenSpecs add scheduling, online reconciliation, and frontend exposure.
