## Context

See `proposal.md` for motivation. The current host runner creates and deletes an Ubuntu Lima VM on every invocation. Guest provisioning performs `apt-get update` and installs the complete build and filesystem toolchain. The guest then builds a debug binary, runs portable Rust tests, and executes the live ublk/ext4 workflow.

The Linux acceptance requirement needs real ublk control, io_uring queue traffic, ext4 format and mount operations, clean endpoint lifecycle, process-death cleanup, and independent loopback mounting. It does not require a fresh operating-system image for every run. Exact source and environment evidence, a fresh disposable fixture, and fail-closed live behavior remain mandatory.

Measured on the selected ARM64 Lima environment, the 16 MiB workflow still created a 1,024-block ext4 journal. A cached debug binary took 74 seconds for the live phase. The release binary reduced the same live phase to 19–21 seconds; complete prepared-runner invocations took 20–22 seconds.

## Goals / Non-Goals

**Goals:**

- Keep every Linux-only operation and lifecycle transition required by the modified delta spec.
- Make the 30-second live and warm-runner bounds executable failures with observed timing evidence.
- Preserve source isolation while reusing only VM provisioning, Cargo downloads, and build outputs.
- Keep deterministic refusal and fence behavior in the ordinary portable Rust gate.

**Non-Goals:**

- Broaden the supported Linux profile, filesystem set, topology, durability claim, or hardware claim.
- Add a daemon, custom VM image, container layer, compiler cache service, or cross-compilation toolchain.
- Promise that initial VM provisioning or the first release build finishes within 30 seconds.
- Treat cached fixture, recovery, endpoint, or trace state as acceptance input.

## Decisions

### Reuse one named Lima runner

`run.sh` uses one named Lima instance instead of a timestamped instance. It starts the instance when stopped and provisions it only when absent. The host lock is derived from the host system temporary directory and is installed only after successful acquisition. Each invocation asks the guest to create an isolated temporary source workspace. A guest `flock` covers source extraction, the cached release build, and the live workflow so a surviving SSH command cannot overlap a later invocation.

Alternative: retain a fresh VM for every invocation. Rejected because VM boot provisioning accounts for minutes and does not exercise additional DiskWeave behavior.

Alternative: maintain a custom golden image. Rejected because a persistent standard Lima instance provides the needed cache without another image lifecycle.

### Keep source fresh and cache only build outputs

Every invocation still creates and hashes the source archive, creates a fresh guest workspace with `mktemp`, and extracts the exact current archive there. The extracted `target` path is a symlink to a persistent path below the guest user's XDG cache directory. Cargo therefore revalidates current source and dependencies while retaining compiled artifacts and registry downloads. The remote workflow removes its temporary source workspace on exit.

The runner stages copied evidence in its host temporary directory, enforces the applicable time bound, promotes the staged files, and only then writes its prepared marker below the guest user's XDG state directory. A missing marker classifies the invocation as cold even when the VM object already exists.

Alternative: update the extracted source tree in place. Rejected because files removed from the current source snapshot could remain and affect the build.

### Execute an optimized acceptance binary

The runner builds `dwv` with Cargo's release profile, and every guest command uses `target/release/dwv`. The live path is dominated by repeated ext4 flushes through DiskWeave; debug-mode execution exceeded the bound even after reducing fixture size. Release mode changes execution cost, not request semantics or evidence scope.

### Use the minimum supported 16 MiB fixture

The main fixture uses the initializer's existing 16 MiB lower bound. The selected environment still creates an ext4 journal and emits real read, write, and flush traffic. Acceptance validates required operations, bounded trace integrity, exact content, parity equality, and recovery health rather than requiring a stable trace count or payload hash across runs.

Alternative: retain 64 MiB. Rejected because the canonical profile does not require that capacity and the additional bytes increase hashing and flush work without adding a distinct behavior.

### Separate portable and live evidence

The timed guest workflow keeps actual prerequisite probing, production publication, ownership conflict against a live service, wrong-owner cleanup refusal, kernel discard refusal, ext4 mutation and flushes, clean restart, stale readiness, killed-owner endpoint cleanup and reacquisition, direct loopback mount, parity comparison, inspection, and live trace replay.

Undersized fixture, wider unsupported topology, missing or corrupt recovery authority, adapter translation, tag generations, trace bounds, and partial fence coverage remain ordinary Rust evidence. Existing fixture tests gain the exact undersized and missing-recovery cases removed from the guest script.

### Apply conservative whole-second gates

The guest records cumulative elapsed seconds from probe through final direct inspection. The host records elapsed seconds from source snapshot creation through staged evidence copy. Because the available portable shell timer has whole-second resolution, an observed value of 30 seconds is rejected; successful evidence therefore has at least the resolution margin needed to establish the 30-second ceiling.

The host enforces its bound only after the prepared marker exists. The live bound applies to cold and warm invocations.

## Risks / Trade-offs

- [A persistent VM can drift] → Record the exact kernel, architecture, tool availability, source digest, and configured bounds on every run; recreate the disposable VM when its Lima configuration or required environment changes.
- [An interrupted host command can leave remote work running] → Hold one guest-side `flock` across extraction, build, and live execution; use per-invocation guest and fixture directories from `mktemp`; let the surviving remote command finish and clean its workspace; and refuse a later invocation while the guest lock is held.
- [Cargo cache reuse could conceal removed source] → Replace the extracted source tree every run and reuse only Cargo's target directory.
- [A 16 MiB filesystem changes incidental trace counts] → Validate operations, trace schema and bounds, replay, content, parity, and recovery state; do not make exact request counts part of the profile.
- [Release compilation makes the first run slower] → Classify it as cold preparation and enforce the complete-invocation bound only after successful preparation.
- [Wall-time variance can approach the limit] → Use a conservative whole-second comparison and retain phase timestamps to localize regressions.

## Migration Plan

1. Add the portable fixture refusal checks before removing their guest-script duplicates.
2. Convert the host runner to the named reusable VM, ownership-safe host locking, isolated host and guest temporary directories, guest-side serialization, clean source extraction, persistent Cargo target, release build, staged evidence promotion, and warm timing gate.
3. Reduce the live fixture to 16 MiB, remove portable prerequisites from the guest, retain Linux-only checks, and add live timing evidence.
4. Run once to provision and populate release caches, then run again through the prepared path and require both live and complete warm durations below 30 seconds.
5. Replace retained Linux evidence and traces only with the final successful prepared run, replay both traces, and update maintained verification prose and mappings.
6. Run focused portable tests plus the documentation knowledge gates.

Rollback removes the reusable VM and reverts the runner, guest workflow, retained evidence, and verification prose together. No product format or user state requires migration.
