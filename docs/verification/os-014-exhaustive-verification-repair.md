# OS-014 verification record

This record covers exhaustive single-parity verification and separate-target
repair planning on the portable/macOS regular-file path. It does not claim
metadata-loss recovery, missing-member degraded service, resumable rebuild,
scrub migration, FSKit/DiskImages attachment, Linux behavior, or physical
durability.

## Evidence run on 2026-08-07

- `cargo test --workspace` — passed; the workspace includes the new
  `dwv-verify` model tests and the macOS regular-file adapter test.
- `cargo fmt --all -- --check` — passed.
- `cargo clippy -p dwv-verify --all-targets --no-deps -- -D warnings` — passed.
- `cargo clippy -p dwv-service --all-targets --no-deps -- -D warnings` — passed.
- `cargo tree --workspace -e normal` — passed; OS-014 adds no third-party
  runtime dependency.
- `cargo metadata --format-version 1` — passed; the verification crate
  depends only on portable workspace semantics.
- `openspec validate --all --json` — passed; all repository specs and the
  active OS-014 change are valid.

The model matrix covers exhaustive matching scans with zero writes, sampled
non-clean diagnostics, short/incomplete reads, parity-identified repair,
single-data-identified repair, hashless/stale/conflicting/multiple-suspect
refusal, all-valid equation conflict, failed repair source preservation, and
source/target identity alias rejection. The macOS adapter uses temporary
ordinary files, verifies a separate repair target, checks hard-link alias
rejection, and confirms the corrupt source remains unchanged.

## Authority boundary

The report distinguishes parity consistency from recovery `CLEAN`; even an
exhaustive matching scan does not create a recovery checkpoint. Repair writes
are separate-target and read-back verified, but this change does not persist a
new recovery generation or checksum record. OS-015 owns metadata-loss actions,
OS-016 owns missing-member/degraded reads and resumable rebuild, and OS-017
owns full checksum scrub and verified repair integration.

`cargo-deny` and `cargo-about` are not installed in the host environment. Cargo
metadata remains the dependency/license inspection boundary, as documented for
the earlier portable slices.
