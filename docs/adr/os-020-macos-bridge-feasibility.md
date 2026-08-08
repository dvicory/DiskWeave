# ADR: OS-020 macOS bridge feasibility

- Status: Provisional, environment-blocked
- Date: 2026-08-07
- Scope: macOS `portable-demo` reference only

## Decision

Keep the regular-file fixture and normalized trace as the current macOS baseline. Do not select FSKit, macFUSE, or DiskImages as the live bridge until a matching Swift toolchain and a privileged/manual attachment session produce the required operation and synchronization evidence. The portable core remains unchanged and no bridge API is allowed to define parity or recovery semantics.

## Evidence

| Candidate/boundary | Host evidence | Result |
|---|---|---|
| Regular-file baseline | `tools/macos-bridge-probe` creates separate fixed-size backing/export files, checks bounded exact-range I/O, sparse zero reads, copy byte equality, copy identity, hard-link alias detection, disposable out-of-range extension/truncate behavior, close/reopen persistence, and validates a deterministic normalized trace | Available as a portable-demo baseline; host regular files permit extension/resize, so this does not certify fixed-size bridge denial or emulate a virtual disk |
| FSKit | Command Line Tools SDK exposes FSKit headers, modulemap, Swift interfaces, and `FSKit.tbd` | Not attached: no extension, signing, entitlement, or matching Swift build evidence |
| DiskImages | `/usr/bin/hdiutil` exists and `hdiutil help` runs; a disposable `hdiutil create -size 16m -fs APFS -type SPARSE` attempt failed with `Device not configured`; `diskutil` framework access is restricted in this session | Attachment through a candidate proxy is not attempted; the failure is an environment/device boundary, not bridge evidence |
| macFUSE | `/Library/Filesystems/macfuse.fs` and `/usr/local/bin/mount_macfuse` are absent | No comparison run; candidate is unavailable on this host |
| Swift toolchain | selected developer directory is `/Library/Developer/CommandLineTools`; SDK is 26.2; no Xcode app is present; SwiftPM reports a compiler/SDK patch mismatch and an unwritable user module cache | Environment blocker, not semantic bridge evidence |

## Claim boundary

The baseline can test ordinary host-file geometry, bounded exact-range behavior, sparse-hole behavior, copy/alias identity separation, process-local file operations, close/reopen persistence, and normalized evidence formatting. It cannot establish FSKit/macFUSE coherence, DiskImages page-cache mapping, detach/disconnect behavior, FUA, controller-cache behavior, or physical power-loss durability. Simulator schedules remain authoritative for modeled crash/power-loss semantics.

## Exit evidence for OS-020

With a matching toolchain and appropriate session, rerun the manual sequence in `tools/macos-bridge-probe/README.md`: install/sign the candidate, attach only the exported proxy through DiskImages, capture read/write/flush/sync/close/detach/disconnect traces, kill/restart, remove a backing member, and verify backing/export separation. Select a candidate only if all required fixed-geometry and synchronization criteria pass; otherwise preserve this blocker and narrow OS-021.

## References

- [Apple FSKit documentation](https://developer.apple.com/documentation/FSKit)
- [Apple DiskImageKit documentation](https://developer.apple.com/documentation/DiskImageKit)
- [Apple DiskImage documentation](https://developer.apple.com/documentation/diskimagekit/diskimage)
