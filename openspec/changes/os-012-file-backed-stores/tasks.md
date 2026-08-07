## 1. Artifact and module contract

- [x] 1.1 Cite archived OS-002/OS-005 prerequisites, handoff Sections 5.1, 5.2, 6.5, 7.1, 17.3–17.5, and 22.3.
- [x] 1.2 Define the handoff-required 20-section design with portable file, macOS bridge, Linux, and hardware evidence separated.
- [x] 1.3 Define the file-backed store capability delta and preserve recovery authority outside control SQLite.

## 2. File store implementation

- [ ] 2.1 Add `dwv-store-file` and split file I/O, capability probes, leases, and control projection into dedicated modules.
- [ ] 2.2 Implement fixed-geometry regular/sparse-file open, exact reads/writes, bounded write-zeroes, and conservative unsupported discard.
- [ ] 2.3 Implement sync/error/short/uncertain completion evidence and `RandomAccessStore` conformance.
- [ ] 2.4 Implement file identity observations, capability reports, path-independent comparison, and atomic ephemeral single-writer leases.
- [ ] 2.5 Implement backing/export alias rejection without adding required payload metadata or sidecars.

## 3. Disposable control state

- [ ] 3.1 Add a separate control SQLite projection with migration, bounded semantic export, integrity handling, deletion, and rebuild fixtures.
- [ ] 3.2 Verify control-state loss does not delete or reinterpret direct payloads or authoritative recovery state.

## 4. macOS-capable evidence and verification

- [ ] 4.1 Add macOS-runnable sparse-hole, fixed-length, identity-change, lease, sync, and direct-data equality tests.
- [ ] 4.2 Run focused/workspace tests, formatting, dependency inspection, and OpenSpec validation on the macOS host.
- [ ] 4.3 Keep physical FUA/discard, DiskImages/FSKit, Linux ublk/io_uring, and hardware claims explicitly gated to OS-020/023/030+.
- [ ] 4.4 Record OS-012 complete only when portable file-store and disposable-control evidence passes; leave bridge selection to OS-020.
