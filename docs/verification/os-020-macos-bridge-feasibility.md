# OS-020 verification record

OS-020 is complete as a feasibility decision, not as a selected production
bridge. The current macOS evidence is the unprivileged regular-file baseline;
FSKit, macFUSE, and DiskImages remain replaceable candidate boundaries.

## Evidence run on 2026-08-07

- `swiftc -parse tools/macos-bridge-probe/Sources/DwvMacOSBridgeProbe/*.swift`
  — passed.
- `openspec validate --all --json` — passed, 13/13 repository specs and
  changes valid.
- `cargo fmt --all -- --check` — passed.
- `cargo tree --workspace -e normal` — passed.
- `swift package dump-package` / `swift build` — blocked by the restricted
  SwiftPM/module-cache permissions and the installed Swift compiler versus SDK
  patch mismatch documented in the README and ADR.

The probe source now checks bounded exact-range host I/O, sparse-hole reads,
copy byte equality and changed file identity, hard-link alias identity,
backing/export separation, disposable host extension/truncate behavior,
close/reopen persistence, and a deterministic normalized trace contract.

## Explicit blockers

- No matching Xcode/Swift toolchain is available in the session, so SwiftPM
  cannot build or execute the probe.
- FSKit extension installation, signing, entitlements, and DiskImages
  attachment require a privileged/manual session and are not attempted here.
- macFUSE is not installed on the host.
- Live cache-coherence, disconnect, candidate kill/restart, and APFS-visible
  fixed-geometry tests require an attached candidate and therefore remain
  manual OS-021/OS-022 entry evidence.
- The regular-file baseline permits extension/resize; that is host evidence,
  not evidence of a bridge proxy's required denial behavior.

These blockers are classified as environment/candidate evidence gaps rather
than portable-core failures. No FSKit, macFUSE, DiskImages, FUA, controller
cache, or physical power-loss claim is made.
