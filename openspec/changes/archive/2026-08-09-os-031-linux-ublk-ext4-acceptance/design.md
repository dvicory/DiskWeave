## Context

The portable service already owns topology validation, operation admission, XOR updates, recovery intent, file-store I/O, flush/checkpoint evidence, and ordinary-member readability. The existing two-data-member offline workflow becomes the portable `dwv demo file` frontend and remains the regression baseline rather than absorbing Linux kernel transport code. No current crate owns Linux ublk semantics.

`libublk` 0.4.6 is a packaged Rust library that owns ublk control and queue UAPI/io_uring mechanics. Its types must remain inside the Linux adapter. The selected acceptance environment is a disposable no-host-mount ARM64 Ubuntu 26.04 VM with Linux 7.0, matching the prior environmental oracle; the change must record the exact observed image and kernel rather than assuming them.

## Goals / Non-Goals

**Goals:**

- Route actual ublk reads, writes, and flushes through `HealthyPortableService` for stable slot zero of a validated one-data/one-parity fixture.
- Keep kernel mechanics, tags, buffers, and dependency types outside every existing portable crate.
- Leave a pure deterministic adapter core testable on macOS and a thin Linux queue driver exercised in the VM.
- Make fixture ownership, fixed geometry, single-writer claims, support-profile refusal, clean drain, stale cleanup, and evidence bounds explicit.
- Reuse the portable service and file stores without duplicating parity, recovery, durability, or operation-slot policy.
- Close the portable correctness seams exposed by live Linux traffic before treating ext4 success as acceptance evidence.

**Non-Goals:**

- Add a permanent daemon, separate frontend executable, network/control protocol, installer, systemd/udev integration, mount-namespace manager, raw-device backend, or multi-device controller.
- Put Lima or VM orchestration inside the portable demo CLI.
- Support FUA, discard, write-zeroes, zoned requests, multiple data endpoints, daemon-loss recovery/reissue, online topology mutation, or production durability.
- Make the Linux fixture format stable or compatible outside this disposable acceptance profile.

## Decisions

### Linux adapter library behind the portable demo CLI

Add `dwv-frontend-ublk` with a platform-neutral request decoder, tag-generation table, bounded trace schema, support-profile validator, lifecycle state machine, and a Linux-only `libublk` driver. The portable root executable selects an explicit demo frontend: `dwv demo file <command>` runs the existing macOS/Linux file workflow, while `dwv demo disk probe|init|serve|inspect|cleanup` uses ublk on Linux and reports unsupported operations explicitly elsewhere.

The disk `serve` command remains foreground and local. The external workflow may background it only inside the disposable VM. This keeps one portable demo CLI without coupling file-frontend code to kernel types or embedding Lima orchestration in the executable.

### Correct the shared seams, not the Linux symptom

The Linux adapter SHALL not synthesize dirty regions, store watermarks, recovery generations, or ownership policy. Before acceptance, the shared portable path changes coherently:

- recovery owns one checked range-to-region mapping and returns every intersected region exactly once;
- stores assign monotonic per-incarnation write watermarks, and flush reports the exact synchronized-through watermark;
- fence composition validates exact stores, regions, watermarks, topology/recovery generations, and store incarnations before clearing only proven regions;
- recovery snapshot/load errors propagate unchanged rather than falling back to generation zero or successful trace output;
- file-store and recovery single-writer claims use operating-system advisory descriptor locks, with markers diagnostic only and all partial claims released on failure;
- normalized request construction preserves every required identity, target, epoch, operation, range, buffer, sequence, ordering, and durability field.

This is a clean cutover across service, transaction, store, recovery, simulator, fixture, CLI, and tests. No Linux-only guard or compatibility path is acceptable.


### Bounded synchronous first queue

Use one ublk queue, depth 8, one buffer per tag, maximum transfer 128 KiB, the service's 32 generational operation slots, and at most 4096 normalized trace records. The queue handler invokes the synchronous portable service directly and completes before accepting another operation on that tag. This is intentionally not a throughput design; it minimizes lifetime ambiguity while still exercising real io_uring/ublk transport and ext4 concurrency across bounded tags.

`libublk` owns the raw queue buffer for the tag lifetime. DiskWeave validates sector-to-byte conversion and flags first, reserves the adapter tag generation, then lets `HealthyPortableService` reserve its existing operation slot before child I/O. Completion releases the adapter generation only after the service returns a terminal result. The pure tag table rejects stale and duplicate completion.

### Advertise only read, write, and flush

Set fixed 512-byte logical and 4096-byte physical geometry and advertise volatile-write-cache/flush behavior without FUA. Accept only plain ublk read, write, and flush operations; reject every other operation or flag with a deterministic kernel error. Writes use ordinary durability; flush maps to the service's global flush/checkpoint. The acceptance workload must show that ext4 uses the advertised subset. If the kernel emits FUA or another unsupported flag, the run fails rather than weakening it.

### Dedicated one-data fixture

Use a small versioned disposable manifest under an owned root rather than mutate the offline demo's two-data-member format. The manifest records stable semantic IDs, topology epoch, protected length, logical block size, relative data/parity/recovery paths, and opened-file identity observations. Initialization creates zeroed ordinary data and parity files, establishes the one-data XOR topology and recovery state, then records identities atomically.

Assembly canonicalizes the root, rejects escaping/absolute references, rechecks regular-file identity and exact length from opened objects, acquires all store and recovery descriptor locks, loads and validates recovery state without fallback, validates the complete one-data/one-parity topology, and only then creates ublk. Partial acquisition releases every claim. Reacquisition after owner death repeats identity, geometry, topology, schema, and recovery-health validation. Wider manifests/topologies are valid portable concepts but fail this adapter profile as unsupported before opening a writable endpoint.

### Foreground lifecycle and owned cleanup

`serve` follows `Stopped -> Assembling -> Published -> AdmissionClosed -> Draining -> Stopped`. It writes a bounded readiness record containing the allocated device ID/path and fixture digest after ublk publication. SIGINT/SIGTERM stops kernel admission. After the queue exits, the server performs the portable global flush/checkpoint only with exact region and store-watermark fence evidence, removes the ublk device, writes bounded trace/shutdown evidence, and drops recovery/store handles. Process death may release descriptor locks but never implies a clean checkpoint.

`cleanup` accepts an explicit device ID and fixture root, reads libublk's exported target JSON, and deletes the device only when its DiskWeave fixture digest matches. Unknown or mismatched devices are never removed. Startup reports an owned stale endpoint as reconciliation-required rather than silently deleting it.

### Evidence and live workflow

A checked-in Linux acceptance script performs probe, fixture initialization, foreground-server launch in a distinct mount namespace, readiness parsing, ext4 format/mount, bounded file operations and hashes, unmount/SIGINT/drain, restart/remount verification, inspection, and final read-only loop mount of the ordinary member. It writes no host mounts and uses only a disposable guest directory.

The verification record includes guest release/kernel/module facts, command versions, configured bounds, device geometry, content hashes, parity equality/hash, recovery disposition, exact region/store-watermark/fence evidence, trace digest/count, ownership crash/reacquisition evidence, cleanup state, and explicit non-claims. Upstream `ublksrv` may be used only as an environmental oracle outside the product path and is not required by the final workflow. Correction evidence additionally includes a TLA+ mutation/crash model, a bounded Kani dirty-region harness, an independent fence-coverage model, a killed-owner process integration case, and deterministic regressions; every layer names the claim it does and does not prove.

### Normative v0.8 reconciliation

| v0.8 contract | Disposition |
|---|---|
| §§4.1–4.2 portable Linux frontend seam and exclusion of kernel types from core | OWNED by `req.architecture-contract.portable-semantics-are-independent-of-implementation-mechanisms` and `req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics`. |
| §4.6 complete writable exposure and all-or-nothing assembly | OWNED by `req.architecture-contract.identity-and-topology-authority-are-explicit-and-conservative`, `req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe`, and the new initial Linux publication-profile requirement. The one-data fixture publishes the complete data-device group, not a subset. |
| §5.4 Linux identity/claim ordering and partial-claim release | OWNED by `req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit`, the healthy assembly requirement, and the new Linux assembly/shutdown requirement. |
| §§9.1–9.2 operation/flag/order/result translation and frontend resource ownership | OWNED by all current `normalized-block-semantics` requirements plus `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`; Linux-specific translation is added here. |
| §§10.8, 11.8 cancellation, quiescence, shutdown, and no premature clean/reclamation | OWNED by `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics`, `req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety`, and the new Linux assembly/shutdown requirement. |
| §§6.1–6.4, 11.1–11.7 dirty-region, write-ordering, fence, and checkpoint safety | OWNED by `req.dirty-integrity-invalidation.dirty-region-coverage-is-complete-and-checked`, `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence`, `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence`, and the corrected healthy portable write/fence requirements. |
| §§5.4, 8.10 recovery authority and crash-releasing single-writer ownership | OWNED by `req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout`, `req.recovery-state-semantics.writable-recovery-ownership-is-crash-releasing`, and the corrected file-backed ownership requirement. |
| §§21.1–21.10 layered executable evidence | OWNED for the correction subset by `req.linux-ublk-frontend.correction-evidence-covers-the-composed-correctness-boundaries`; production SQLite and physical durability evidence remain unmet. |
| §§15.4–15.6 Linux boot/shutdown/mount-namespace guidance | GAP for the bounded acceptance subset, added by probe, lifecycle, and live-evidence requirements. Full boot integration, automount policy, and deployment remain non-goals rather than implied requirements. |
| §16.2 ublk provisional choice and adapter boundary | SUPERSEDED for this bounded profile by selecting packaged `libublk` behind the new adapter; replaceability remains OWNED by the architecture seam. No production/performance selection is claimed. |
| §21.10 Linux VM evidence tier and Gate G's ublk/tag/ext4 subset | OWNED by `req.evidence-boundaries.evidence-scope-is-explicit`, `req.normalized-block-semantics.portable-evidence-does-not-imply-platform-certification`, and the new live ext4 evidence requirement. XFS, encryption, mergerfs, workloads, deployment, and hardware portions remain explicitly unmet. |

## Risks / Trade-offs

- The synchronous handler may underperform; queue depth 8 is sufficient only for functional acceptance. Replace it only after measured need and without changing semantics.
- `libublk` is young and Linux-specific. Target-scoped dependency placement and pure adapter tests limit its blast radius.
- ext4 may emit a flag outside the selected profile. That is a useful conformance failure; do not silently enable or emulate it.
- Signal interruption after kernel stop but before checkpoint can leave dirty/reconciliation-required evidence and a stale endpoint. Explicit owned cleanup handles only the endpoint; it never declares recovery clean.
- The file-backed flush proves host-file synchronization in one VM, not controller-cache or power-loss durability.
- A passing ext4 run can conceal a shared semantic defect. Acceptance therefore remains blocked unless deterministic, model, bounded-proof, independent-model, and process-crash layers all pass for their declared properties.
- Advisory locks release on process death while dirty state and stale endpoints may remain. Reacquisition must revalidate authority; lock acquisition alone never implies clean recovery.
- SQLite remains an evaluation adapter. This change does not select its production journal/synchronization/checkpoint configuration or elevate host-file flush to physical durability evidence.
