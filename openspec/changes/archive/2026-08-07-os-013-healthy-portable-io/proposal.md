## Why

The portable contracts, parity reference, recovery protocol, checksum plane, and file-backed stores need one end-to-end path before a macOS frontend can be evaluated. OS-013 is the handoff’s first functional single-parity product slice: ordinary files host independently readable data members while normalized requests exercise the same transaction, recovery, parity, operation-slot, and integrity semantics.

## What Changes

- Add an end-to-end healthy portable read/write service over fixed-size file-backed stores.
- Route normalized requests through topology/operation-slot admission and the OS-008/OS-010 transaction ordering.
- Perform single-XOR parity updates and reads while preserving independent checksum coverage.
- Implement flush/FUA/clean-checkpoint behavior only from store fence evidence.
- Add randomized reference-image comparisons, restart/abandonment schedules, bounded-resource tests, and macOS portable-demo evidence.

## Capabilities

### New Capabilities

- `healthy-portable-io`: Portable healthy read/write path using ordinary file-backed member images and conservative recovery semantics.

### Modified Capabilities

None.

## Impact

- Adds a portable orchestration/core service over `dwv-core`, `dwv-store`, `dwv-codec`, `dwv-recovery`, OS-008, OS-010, OS-011, and OS-012.
- Registers the file-store and transaction crates in the workspace without introducing Linux APIs.
- Adds macOS file-backed integration tests and a portable-demo claim; actual FSKit/DiskImages bridge work remains OS-020.
- Keeps data members ordinary images with no DiskWeave metadata and rejects backing/export aliasing.
