## Why

The Linux ublk/ext4 acceptance workflow takes about 6.5 minutes because each run provisions a VM, rebuilds from a cold cache, and executes the workload with an unoptimized binary. This delay makes the only real-kernel acceptance evidence too expensive to run frequently.

## What Changes

- Reuse one prepared Lima instance and persistent Cargo target directory while creating a fresh disposable DiskWeave fixture for every run.
- Build and execute the Linux acceptance binary in release mode.
- Reduce the disposable acceptance fixture from 64 MiB to 16 MiB while retaining ext4 journaling and every required format, mount, mutation, flush, restart, read-only remount, shutdown, owner-death, cleanup, trace, parity, and independent-member check.
- Move deterministic undersized-fixture, unsupported-topology, missing-recovery-authority, and partial-fence checks to the ordinary Rust gate; keep actual Linux ublk, ext4, mount, discard, endpoint, and process-death behavior in the live workflow.
- Require the live workflow and a prepared warm-runner invocation to finish within 30 seconds. Cold VM provisioning and the first release build prepare the runner and remain outside the warm-runner bound.
- Retain exact source, environment, trace, content, parity, and claim-boundary evidence while recording live elapsed time.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `linux-ublk-frontend`: Bound the established live ext4 acceptance workflow to 30 seconds on a prepared runner and make the live-versus-deterministic evidence split explicit without reducing the required Linux behavior.

## Impact

- `tools/linux-disk-acceptance/run.sh` reuses and serializes a named Lima instance, preserves build caches, builds release mode, and enforces the warm-runner time bound.
- `tools/linux-disk-acceptance/guest.sh` runs only the platform acceptance layer, uses a 16 MiB fixture, records elapsed time, and enforces the live time bound.
- `crates/dwv-frontend-ublk/src/fixture.rs` retains deterministic coverage moved out of the guest workflow.
- Linux acceptance JSON, traces, and maintained verification prose are regenerated for the changed workflow and source archive.
- No product API, persistent product format, dependency, or supported Linux profile changes.
