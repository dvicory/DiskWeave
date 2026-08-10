## 1. Adapter boundary and deterministic contracts

- [x] 1.1 Add `dwv-frontend-ublk` as a workspace library crate with a Linux-only `libublk` dependency, expose explicit portable `file` and Linux ublk-backed `disk` frontends through the root demo CLI, and add no reverse dependency from portable crates.
- [x] 1.2 Implement checked ublk operation/flag/range translation into validated normalized requests with deterministic unsupported and terminal error mappings.
- [x] 1.3 Implement bounded generational tag ownership, trace capture, publication-profile validation, and lifecycle transitions with deterministic unit checks.

## 2. Disposable fixture and portable service integration

- [x] 2.1 Implement the versioned one-data/one-parity fixture initializer with stable semantic IDs, fixed geometry, zero parity baseline, recovery state, and atomic manifest publication.
- [x] 2.2 Make fixture open and inspection fail closed for root escape, opened identity/length mismatch, aliasing, unsupported topology, and every recovery snapshot error, with no generation-zero fallback or pre-publication mutation.
- [x] 2.3 Route reads, writes, and global flush/checkpoint through `HealthyPortableService` for the explicit stable data slot and captured topology epoch using canonical region coverage and real store watermark evidence.
- [x] 2.4 Add focused fixture/service regressions for path escape, replacement/resize, aliasing, stale epoch/identity, wider-topology refusal, recovery access failure, partial claim release, and independent post-shutdown data readability.

## 3. Correctness-boundary closure

- [x] 3.1 Add one recovery-owned checked range-to-dirty-region mapping that covers every intersected region exactly once and rejects empty, arithmetic-overflow, representation-overflow, collision, and unsupported-geometry cases.
- [x] 3.2 Replace the service-local start-offset `RegionId` derivation and propagate exact region sets through intent, transaction ranges, fence evidence, checkpoint clearing, simulator interpretation, and recovery decisions.
- [x] 3.3 Add per-store-incarnation monotonic write watermark assignment and exact write/flush completion evidence; remove every sentinel, guessed, future, or cross-store watermark.
- [x] 3.4 Compose and enforce independent multi-store, multi-region fence coverage so clean/checkpoint clears only matching proven regions and rejects stale, partial, future, omitted-region, cross-store, or generation-mismatched evidence.
- [x] 3.5 Preserve all recovery snapshot/load failures through service, fixture, trace, and CLI results; remove every fallback to `RecoveryGeneration::ZERO`, clean state, or success.
- [x] 3.6 Replace store and recovery marker ownership with crash-releasing operating-system advisory descriptor locks, keep markers diagnostic only, release all partial claims on every failure, and revalidate identity/topology/recovery authority on reacquisition.
- [x] 3.7 Verify adapter construction and translation preserve request/frontend identities, target slot, topology epoch, operation, checked range, buffer token, submission sequence, ordering intent, and durability intent end to end.

## 4. Correctness evidence

- [x] 4.1 Add or extend a TLA+ model for intent, multi-region home/parity mutation, store fences, checkpoint/clear, crash, and restart; model-check the declared safety invariants and retain counterexamples as regressions.
- [x] 4.2 Add a bounded Kani harness for dirty-region mapping coverage, uniqueness, checked arithmetic, representability, and member-ID collision freedom over the declared production bounds.
- [x] 4.3 Add an independent fence-coverage model that rejects future, stale, partial, omitted-region, cross-store, store-incarnation, topology-generation, and recovery-generation mismatches.
- [x] 4.4 Add a subprocess integration case that kills a store/recovery owner without cleanup, proves operating-system claim release, and proves conservative reacquisition and authority revalidation.
- [x] 4.5 Add deterministic regressions for every corrected seam, including multi-region writes, partial fences, watermark monotonicity, recovery corruption/lock/unavailability, partial claim acquisition, and crash points before and after each durable boundary.

## 5. Linux frontend and lifecycle

- [x] 5.1 Implement non-destructive Linux prerequisite probing with exact present, unsupported, and blocked dispositions.
- [x] 5.2 Implement the bounded single-queue `libublk` server for read, write, and flush with fixed geometry and fail-closed handling of every unsupported operation or flag.
- [x] 5.3 Implement portable root `dwv demo disk probe`, `init`, `serve`, `inspect`, and owned `cleanup` commands with explicit non-Linux refusal and bounded readiness, trace, and shutdown records.
- [x] 5.4 Implement transactional publication, signal-driven admission close, queue drain, portable exact-watermark flush/checkpoint, endpoint removal, handle release, and reconciliation-required failure outcomes.
- [x] 5.5 Restrict stale cleanup to an explicitly named endpoint whose exported target metadata matches the fixture digest; refuse unknown or conflicting devices.

## 6. Live Linux acceptance

- [x] 6.1 Add a reproducible no-host-mount ARM64 Ubuntu VM workflow with bounded guest resources and explicit prerequisite checks.
- [x] 6.2 Exercise real ext4 format/mount, create/overwrite/rename/fsync/read/delete, clean stop, restart/remount verification, and exact content hashes through the DiskWeave ublk endpoint.
- [x] 6.3 After final shutdown, inspect data/parity/recovery state and mount the ordinary data member read-only without DiskWeave to verify retained conventional-filesystem readability.
- [x] 6.4 Exercise negative live cases for unsupported topology/flags, ownership conflict, stale endpoint, interrupted cleanup, resource bounds, recovery authority loss, owner process death, and partial fence coverage without partial publication or protected mutation.

## 7. Evidence and completion

- [x] 7.1 Run focused portable/Linux checks, the TLA+ model, Kani harness, independent fence model, process-crash integration, workspace regressions, formatting, linting, and strict OpenSpec validation.
- [x] 7.2 Record exact guest/kernel/library/tool facts, configured bounds, normalized trace digest/count, hashes, region/fence/watermark evidence, parity/recovery/integrity dispositions, ownership/reacquisition evidence, cleanup state, failures, and explicit non-claims.
- [x] 7.3 Map every correction check to its canonical requirement and architecture-v0.8 property; explicitly deny production SQLite, physical durability/FUA, broader concurrency, multi-device publication, online topology mutation, and hardware claims.
- [x] 7.4 Run documentation readiness and clean-room reconstruction checks, review every affected requirement relationship, and resolve all required actions.
- [x] 7.5 Verify the implementation against the proposal, design, capability requirements, Milestone 7 handoff, and architecture v0.8 before archiving.
- [x] 7.6 Record coherent conventional revisions and archive only after no critical issue or unmet acceptance requirement remains.
