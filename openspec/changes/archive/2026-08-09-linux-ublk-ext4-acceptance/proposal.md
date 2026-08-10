## Why

DiskWeave's portable file-backed service is executable, but no real Linux block device carries filesystem traffic through it. Goal-v7 needs one bounded ARM64 Linux ublk/ext4 acceptance path without promoting the experimental file backend into a production durability claim.

## What Changes

- Add a Linux-only DiskWeave-owned ublk adapter that translates real kernel reads, writes, and flushes into the existing normalized request and portable service contracts.
- Add deterministic adapter conformance checks for ranges, flags, bounded tags/buffers/slots, stale completion, error mapping, lifecycle, aliasing, and unsupported topologies.
- Add a bounded one-data-slot/one-parity-slot disposable fixture and local lifecycle commands for probe, initialize, serve, inspect, and owned stale-device cleanup.
- Package the selected ublk library as an internal Linux-only dependency; users do not build or invoke an upstream serving utility.
- Add a reproducible ARM64 Ubuntu VM workflow that formats/mounts ext4, exercises file operations and synchronization, restarts the service, verifies retained content, and reads the ordinary member independently after shutdown.
- Record exact environment facts, normalized traces, parity/recovery/integrity disposition, failure evidence, and non-claims.
- Refuse wider topologies, unsupported operations/flags, missing authority, aliases, stale fixtures, exhausted resources, and cleanup conflicts before partial publication or protected mutation.
- Correct the portable seams exposed by live Linux traffic: complete checked dirty-region coverage, real monotonic store watermarks, exact multi-store/multi-region fence composition, explicit recovery-access failures, crash-releasing store/recovery ownership, and complete normalized-request field preservation.
- Add layered correction evidence: deterministic regressions, a TLA+ mutation/crash model, a bounded Kani region-mapping harness, an independent fence-coverage model, and a member-process crash/reacquisition integration case.

## Capabilities

### New Capabilities

- `linux-ublk-frontend`: Linux ublk request translation, bounded ownership, single-endpoint support profile, local lifecycle, environment probing, and ext4 acceptance evidence.

### Modified Capabilities

- `dirty-integrity-invalidation`: define one checked complete range-to-region mapping and exact region/fence clearing rules.
- `store-operation-contracts`: require real per-store monotonic write and flush watermarks.
- `recovery-state-semantics`: preserve recovery access failures and require crash-releasing writable ownership.
- `file-backed-stores`: replace marker-owned leases with operating-system advisory descriptor locks.
- `healthy-portable-io`: use complete region sets and exact watermark fence evidence through write/checkpoint.

## Impact

- Adds one Linux-specific workspace library crate behind the existing portable frontend/service/store seams and exposes it as the `disk` frontend of the portable `dwv demo <file|disk>` CLI.
- Adds a Linux-only packaged dependency on `libublk`; non-Linux builds retain the same CLI surface without compiling or exposing ublk, io_uring, kernel, runtime, or raw-descriptor types in portable crates.
- Adds disposable fixture files, an external VM acceptance runner, focused tests, and Linux-scoped verification documentation.
- Modifies existing portable correctness seams before relying on them for Linux acceptance; readers, writers, recovery logic, simulators, fixtures, and tests move together with no compatibility shim.
- Adds model/proof/process-crash evidence whose absence leaves the corresponding claim unmet.
- Does not add multi-device publication, daemon recovery, raw-device durability, FUA certification, deployment management, online topology changes, or a stable persistent format.
