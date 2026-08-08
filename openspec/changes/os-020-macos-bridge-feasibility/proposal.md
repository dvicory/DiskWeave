## Why

The handoff makes macOS a functional portable reference, but the core must reach APFS through a small, seekable proxy without moving parity semantics into a filesystem extension. OS-020 answers whether FSKit, macFUSE, and DiskImages can preserve fixed geometry, range I/O, synchronization, disconnect, and backing/export separation well enough for a `portable-demo` reference.

## What Changes

- Add a macOS-only feasibility probe for fixed-size seekable proxy files and candidate DiskImages attachment.
- Characterize FSKit-first, macFUSE alternative, and DiskImages behavior with normalized operation traces.
- Record synchronization, cache, close, disconnect, identity, size, truncate, hole, and failure evidence.
- Verify backing files and exported proxy endpoints are distinct and cannot alias.
- Produce a comparative ADR selecting a bridge path or documenting the blocker; do not change portable core semantics or claim physical durability.

## Capabilities

### New Capabilities

- `macos-bridge-feasibility`: Evidence and decision contract for the smallest macOS proxy/bridge path.

### Modified Capabilities

None.

## Impact

- Adds macOS probe/fixture tooling and an ADR/evidence bundle; portable Rust APIs remain OS-neutral.
- Uses the existing OS-012/OS-013 file-store and normalized-request seams.
- May add a narrowly scoped Swift/Objective-C or shell harness only at the macOS adapter boundary; no FSKit types enter core semantics.
- Records `portable-demo` limitations for process kill, restart, detach, and host-file synchronization; it does not certify FUA, controller caches, or physical power loss.
