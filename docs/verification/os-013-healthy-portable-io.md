# OS-013 verification record

This record covers the portable/macOS file-backed slice described by the OS-013
OpenSpec and the handoff's Gate D. It does not claim FSKit, DiskImages,
physical power-loss durability, Linux frontend behavior, or production hardware
flush semantics.

## Evidence run on 2026-08-07

- `cargo test --workspace` — passed; all workspace unit and doctests passed,
  including 20 `dwv-service` tests.
- `cargo fmt --all -- --check` — passed.
- `cargo clippy -p dwv-service --all-targets --no-deps -- -D warnings` — passed.
- `cargo tree --workspace -e normal` — passed; the only third-party runtime
  dependency is `blake3 = "1"` through `dwv-recovery`.
- `cargo metadata --format-version 1` — passed after fetching the missing
  cached registry metadata; the service is a workspace member with only
  portable Rust crate dependencies.
- `openspec validate --all --json` — passed, 13/13 repository specs and
  changes valid.

The service tests cover exact and short reads, stale and corrupt recovery,
single-XOR partial/full writes, randomized reference-image comparison,
generational checksum invalidation, fence/checkpoint gating, alias and active
lease rejection, abandonment reconciliation, bounded slot/buffer/child
resources, clean reopen, disposable control-state deletion/rebuild, and direct
ordinary-file payload reads.

## Dependency/license inspection boundary

`cargo-deny` and `cargo-about` are not installed in the host environment. Cargo
metadata reports licenses for the resolved third-party graph: `blake3` is
`CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`; its resolved
transitives are BSD-2-Clause, MIT/Apache-2.0, CC0-1.0/MIT-0/Apache-2.0, or
MIT/Apache-2.0. This is a metadata inspection, not a policy audit; a dedicated
license-policy tool remains a release-check item.

## Remaining acceptance boundary

OS-013 remains active until the modeled kill/restart cut-point matrix and a
complete license audit are available. OS-020 remains the owner of FSKit,
DiskImages, synchronization/cache/disconnect, and bridge entitlement evidence.
