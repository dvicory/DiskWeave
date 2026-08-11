## 1. Preserve portable evidence

- [x] 1.1 Add deterministic fixture tests for undersized initialization and missing recovery authority without payload mutation.
- [x] 1.2 Run the complete ublk frontend test suite and the partial multi-store fence regression outside the timed Linux guest workflow.

## 2. Reuse the Linux runner

- [x] 2.1 Replace per-run Lima creation and deletion with one named instance, an ownership-safe host lock, a guest `flock` covering extraction/build/live execution, explicit module loading, and a prepared marker written only after successful promotion.
- [x] 2.2 Resolve repository and Lima configuration paths from the script location, resolve caller-supplied output before any directory change, use `mktemp` for host and guest workspaces, and retain only the Cargo target below the guest XDG cache directory.
- [x] 2.3 Stage copied evidence, enforce the conservative 30-second complete-invocation gate only for a previously prepared runner, promote only successful output, and report whether each invocation was cold or warm.

## 3. Bound the live guest workflow

- [x] 3.1 Create a fresh temporary guest fixture and mountpoint, initialize the main fixture at 16 MiB, and execute every guest CLI operation with the release binary.
- [x] 3.2 Remove deterministic undersized, unsupported-topology, missing-recovery, and partial-fence prerequisites from the guest while retaining every real ublk, ext4, mount, discard, endpoint-lifecycle, process-death, trace, inspection, and parity check.
- [x] 3.3 Record the live elapsed time, retain useful phase timings, and fail conservatively when the whole-second timer reaches 30 seconds.
- [x] 3.4 Update the machine-readable live evidence fields so they describe only behavior executed by the changed guest workflow.

## 4. Refresh maintained evidence

- [x] 4.1 Run the cold preparation path, then run the prepared path and require both the live phase and complete warm invocation to finish below 30 seconds.
- [x] 4.2 Replace the retained Linux acceptance JSON and both traces only with the final prepared-runner output, then replay both retained traces through the current root CLI.
- [x] 4.3 Update the Linux verification record with the exact source digest, reusable VM identity, 16 MiB fixture, release execution, elapsed times, evidence hashes and sizes, trace counts and hashes, portable/live split, and unchanged claim boundary.
- [x] 4.4 Update reviewed requirement evidence or manifest wording only where the changed evidence composition requires it.

## 5. Validate the change

- [x] 5.1 Run shell syntax checks and the focused portable Rust regressions.
- [x] 5.2 Validate the OpenSpec change strictly and run the documentation knowledge check and build gates required by the changed spec and evidence links.
