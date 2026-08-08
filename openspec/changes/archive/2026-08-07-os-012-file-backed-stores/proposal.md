## Why

The portable contracts need a real macOS-capable store before the healthy path or bridge can be exercised. DiskWeave needs fixed-geometry sparse/regular files, exact-range I/O, synchronization evidence, ephemeral single-writer ownership, file identity observations, and a disposable SQLite control projection without leaking platform or SQL layout into the semantic core.

## What Changes

- Add a portable Unix file-backed `RandomAccessStore` implementation for fixed-size regular and sparse files.
- Implement exact reads/writes, write-zeroes, flush/sync, conservative discard behavior, short/error evidence, and fixed geometry.
- Add bounded capability probing and file identity observations suitable for macOS file-backed arrays.
- Add an atomic ephemeral lease for single-writer ownership and backing/export alias checks.
- Add a disposable SQLite control-state projection with migration, export, deletion, and rebuild behavior; keep recovery authority behind `RecoveryStateStore`.
- Add macOS-runnable fixtures for sparse holes, file-ID changes, synchronization, locking, direct-data equality, and control-state loss.

## Capabilities

### New Capabilities

- `file-backed-stores`: Fixed-geometry file stores, capabilities, identity, leases, and disposable control state.

### Modified Capabilities

None.

## Impact

- Adds `dwv-store-file` and a separate disposable control SQLite adapter without adding third-party Rust dependencies.
- Uses standard Unix file APIs available on macOS; Linux-only ublk/io_uring work remains out of scope.
- Provides the store seam required by OS-013 and the macOS OS-020 bridge spike.
