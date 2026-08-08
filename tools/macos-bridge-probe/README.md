# macOS bridge feasibility probe

This package is the unprivileged baseline for OS-020. It creates separate fixed-size sparse files under `backends/` and `exports/`, records host tool/SDK availability, checks apparent geometry and host file identity separation, disposable regular-file extension/resize behavior, close/reopen persistence, and emits bounded JSON evidence plus a normalized regular-file trace. It does not attach a filesystem extension or claim DiskImages synchronization.

Run it from this directory when a matching Swift toolchain is available:

```sh
swift build
swift run dwv-macos-bridge-probe --keep --size 16777216
```

The `--keep` fixture is disposable. Use a path outside the repository for manual attachment tests, and remove it only after all candidate operations and traces have been collected.

## Restricted-session evidence

The current host reports:

- `/Library/Developer/CommandLineTools` as the selected developer directory;
- macOS SDK version `26.2`, with FSKit headers/modulemap available;
- `/usr/bin/hdiutil`, `/usr/bin/xcrun`, `/usr/bin/swiftc`, and `/usr/bin/sqlite3` present;
- no `/Applications/Xcode*.app` toolchain in the session.
- a disposable `hdiutil create -size 16m -fs APFS -type SPARSE` attempt failed with
  `Device not configured`; this does not establish any DiskImages bridge behavior.

SwiftPM currently cannot build this package in the restricted session because its user cache/module cache is outside the writable workspace and the installed compiler/SDK patch versions disagree. This is an environment limitation, not evidence that FSKit or DiskImages is semantically viable.

## Privileged/manual rerun

1. Select a matching Xcode/Command Line Tools toolchain and confirm `xcrun --show-sdk-path`.
2. Run `swift build` and the probe with `--keep` into a disposable temporary root.
3. Install/sign the candidate FSKit extension or confirm the macFUSE alternative, recording OS version, entitlements, signing, and minimum version.
4. Attach only the exported proxy endpoint through the candidate and DiskImages; never attach or export the active backing file directly.
5. Capture read/write/flush/sync/close/detach/disconnect behavior and map each event to normalized requests and store evidence.
6. Repeat after process kill/restart and backing-file disappearance; preserve failures and cleanup evidence.
7. Update the OS-020 comparative ADR with pass, fail, skipped, and environment-blocked results. Do not infer physical FUA, controller-cache, or power-loss guarantees from a successful host sync call.
