# OS-015 metadata-loss recovery verification

## Claim boundary

This evidence covers the portable Section 12.5 recovery matrix, explicit operator-confirmed rebaseline policy, run-bound OS-014 repair translation, audited fresh semantic state for completed exhaustive all-data/single-parity recovery, an evaluation-only SQLite target, and macOS ordinary-file preservation. It does not claim a certified parity-envelope fast path, P/Q execution, degraded reads, parity/new-lineage rebuild execution, live APFS bridge behavior, Linux frontend conformance, production SQLite durability, or hardware power-loss safety.

The certified-envelope paths remain gated semantic plans. Gate H has not enabled them as a production optimization.

## Handoff mapping

- Sections 8.10 and 26.3: uncertified all-data recovery requires exhaustive verification; matching regions incur no payload writes and the manifest records checksum baseline creation as pending work, not established historical evidence.
- Section 12.5: `MetadataLossCase::ALL` contains 18 stable cases, each with an action, disposition, evidence requirement, confirmation requirement, baseline disposition, and non-in-place payload-write policy.
- Sections 26.5 and 26.6: recovery topology is derived from validated core array/slot/role/coding-position/assignment evidence; ambiguous identities, lost Q positions, hashless mismatches, incomplete scans, and conflicting evidence do not authorize fresh state. Data-authoritative rebaseline requires the explicit operator-confirmed API.
- Gate E: OS-014 reports translate to exhaustive match, identified-repair-pending, verified identified repair, ambiguous mismatch, or incomplete scan before OS-015 authorization. Repair receipts are opaque and bound to the exact scan, source identities, separate target identity, read-back digest, range, and target role; an identical later scan cannot reuse them.
- Section 17.6: a temporary macOS regular-file fixture deletes recovery state, recreates it at a new target, and proves data/parity payload bytes remain unchanged.

## Implementation evidence

- `dwv-recovery::metadata_loss` owns the bounded matrix, exact per-row assertions, authorization rules, deterministic dry-run rendering, lineage audit, and fresh manifest construction. Only the exhaustive all-data/single-parity row creates fresh state in OS-015; Gate H certificates and later-spec rebuild/reconstruction rows fail closed.
- Recovery topology can only be constructed from a validated `dwv-core::TopologySnapshot` plus a unique one-to-one runtime store mapping. Fresh-state lineage is derived from that topology rather than supplied independently.
- The semantic recovery schema is version 2. Migration from v1 adds the fixed-size metadata-loss audit independently of the unchanged physical SQLite `user_version=1` schema.
- `dwv-service::classify_metadata_loss_verification` is the dependency-safe boundary from OS-014 reports. Identified mismatches remain pending until exact run-bound repair receipts cover them.
- `dwv-recovery-sqlite::SqlitePrototype::recreate_from_metadata_loss` atomically reserves only an absent target, removes the reservation on failure, preserves any existing database or forensic source, and persists every fixed audit field in the semantic summary.
- The SQLite adapter adds only the existing workspace `dwv-store` path dependency; no third-party crate was added.

## Acceptance commands

- `cargo test --workspace`: passed, 162 unit tests plus all doctests.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy -p dwv-recovery --all-targets --no-deps -- -D warnings`: passed.
- `cargo clippy -p dwv-recovery-sqlite --all-targets --no-deps -- -D warnings`: passed.
- `cargo clippy -p dwv-verify --all-targets --no-deps -- -D warnings`: passed.
- `cargo clippy -p dwv-service --all-targets --no-deps -- -D warnings`: passed.
- `cargo run -q -p dwv-recovery --example metadata-loss-dry-run`: passed and printed the 18 cases in deterministic order.
- `cargo tree -p dwv-recovery-sqlite -e normal`: passed; the adapter uses only `dwv-core`, `dwv-recovery`, and `dwv-store`, with the existing BLAKE3 graph beneath recovery.
- `cargo metadata --format-version 1 --no-deps`: passed.
- `openspec validate --all --json`: passed 15/15 items before and after archive. The only notices are informational long-requirement warnings in the XOR, parity-verification, and metadata-loss specs.

## Deferred work

- OS-016: one-erasure degraded reads, replacement output, and resumable offline rebuild.
- OS-017: scrub scheduling and verified repair orchestration.
- OS-022: live APFS metadata-loss/rebuild acceptance through the selected macOS bridge.
- Linux ublk and physical durability gates remain intentionally skipped on this macOS path.
