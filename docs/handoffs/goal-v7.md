# Goal v7 — Mount and Use an ext4 Member Through DiskWeave on Linux

## 0. Outcome

Deliver the first real Linux block-device path through DiskWeave:

```text
ext4 workload
  -> /dev/ublkbN
  -> DiskWeave-owned ublk frontend
  -> existing normalized request and portable service path
  -> ordinary file-backed member + parity/recovery state
```

A fresh supported Linux VM must install/build DiskWeave, start its own service, expose one fixed-size virtual member, format and mount it as ext4, perform healthy file I/O, cleanly stop and reopen it, and preserve exact data.

The acceptance is the goal. ublk libraries, bindings, crates, queue structure, process boundaries, and control messages are implementation choices justified only by what this acceptance requires.

---

## 1. Starting evidence and authority

Before planning implementation, inspect in this order:

1. current canonical specs and active OpenSpec changes;
2. current crates, dependency direction, tests, CLI behavior, and executable evidence;
3. architecture v0.8 sections covering the Linux stack, normalized frontend boundary, operation ownership, crash consistency, shutdown, verification, and Linux implementation ordering;
4. prior goals only for migration rationale.

Goal-v6 is expected to normalize v0.8 into canonical requirements, but goal-v7 SHALL NOT assume that demotion succeeded merely because v0.8 is labeled historical or a coverage report says it is complete. Reconcile every normative v0.8 statement relevant to this goal before implementation:

- **OWNED:** name the exact current canonical requirement that preserves the contract;
- **SUPERSEDED:** name the exact later canonical requirement that deliberately changes it and the accepted decision evidence explaining why;
- **GAP:** add or modify the canonical capability spec in the goal-v7 OpenSpec change, then validate it before changing implementation.

“Historical,” “relocated,” a section-level coverage row, an ADR alone, or similar prose is not a valid disposition for normative behavior. ADRs may preserve rationale and implementation choices; they cannot own SHALL requirements, forbidden outcomes, state transitions, failure semantics, or acceptance boundaries.

If ownership is missing, ambiguous, contradictory, or unproven, v0.8 remains the latest contract for that area until the canonical spec is corrected. Never silently choose between v0.8, current specs, existing code, and executable evidence.

Existing portable code already provides normalized requests, frontend lifecycle events, bounded operation slots, file-backed stores, transaction/recovery/integrity behavior, and normalized traces. The ordinary-file `dwv demo` is the portable regression baseline.

A disposable ARM64 Lima/VZ probe has already established that Ubuntu 26.04 with Linux 7.0 can load `ublk_drv`, create `/dev/ublk-control`, run io_uring, expose a file-backed ublk device, mount ext4, write/fsync data, remove/recreate the device, and recover the same bytes. That probe used upstream `ublksrv` only as a known-good environmental oracle.

DiskWeave users SHALL NOT need to clone or compile upstream `ublksrv`. The supported installation/build must produce the DiskWeave-owned service and frontend needed for acceptance. A selected `libublksrv`, Rust binding, or direct UAPI implementation may be an internal packaged dependency behind a narrow adapter.

Inspect the post-goal-v6 repository before allocating a change name, crate, dependency, binary, or protocol. Reuse the existing semantic seams; do not create Linux-specific copies of portable behavior.

### Goal-v7 correction baseline

Before live acceptance can count, the linux-ublk-ext4-acceptance change SHALL close and verify the composed correctness seams exposed by the frontend:

- `req.dirty-integrity-invalidation.dirty-region-coverage-is-complete-and-checked`: one checked recovery-owned mapping covers every region intersected by a write and rejects unrepresentable identities;
- `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence` plus `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence`: write completions carry real store watermarks and checkpoint/clear accepts only exact matching multi-store, multi-region fence coverage;
- `req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout`: recovery access failure remains a failure and can never become generation zero;
- `req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit` and `req.recovery-state-semantics.writable-recovery-ownership-is-crash-releasing`: writable claims use crash-releasing descriptor locks, release partial acquisition, and revalidate after owner death;
- `req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics`: the adapter preserves request/frontend identities, stable target slot, topology epoch, operation, range, buffer token, submission sequence, ordering intent, and durability intent.

Completion evidence SHALL include the normal deterministic regressions plus a TLA+ mutation/crash model, a bounded Kani dirty-region mapping harness, an independent fence-coverage model, and a member-process crash/reacquisition integration case. Each artifact must name its property boundary. No layer substitutes for another.

This correction does not select a production SQLite configuration or prove physical durability. Those remain explicit later gates.


---

## 2. User-visible acceptance story

On a clean disposable Linux VM, an autonomous agent can run a documented bounded workflow that:

1. checks kernel and ublk prerequisites without changing host storage;
2. creates a disposable DiskWeave fixture containing ordinary backing files and recovery state;
3. starts a DiskWeave-owned Linux service/frontend;
4. exposes one stable fixed-size virtual block device for one logical data slot;
5. formats the virtual device as ext4 and mounts it;
6. creates, overwrites, renames, syncs, reads, and deletes files;
7. verifies exact content and expected parity/recovery/integrity disposition;
8. unmounts, drains, stops, and removes the virtual device cleanly;
9. restarts DiskWeave against the same fixture, remounts ext4, and verifies retained content;
10. stops DiskWeave and opens the ordinary backing member read-only without the virtual frontend.

The workflow must use DiskWeave’s frontend and portable service path. Attaching the active backing file directly is forbidden.

The acceptance fixture contains one data slot and one parity slot, so its one
published data endpoint is the complete data-device group. This is a tested
Linux support profile, not a topology invariant: portable topology remains
bounded and variable-width, and the adapter targets an explicit stable slot
without owning array width or coding layout. Goal-v7 SHALL refuse a wider
topology as unsupported before publication or mutation; it SHALL NOT publish a
subset, classify the topology itself as invalid, or encode a one-data-slot
limit in portable APIs or persistent state.

---

## 3. Scope

### 3.1 Supported guest and prerequisite detection

Choose one reproducible ARM64 Linux VM profile as the initial development support target. Detect and report:

- kernel release and architecture;
- ublk control availability or loadable module name;
- required io_uring/UAPI features;
- permissions/capabilities needed to create and manage devices;
- ext4 and mount tooling availability;
- exact unsupported or blocked reason when the environment cannot proceed.

The probe must not assume that the module is named identically on every distribution. It must never treat a present header, module file, or compile success as proof of working ublk I/O.

### 3.2 DiskWeave-owned frontend

Implement the minimum Linux adapter that translates actual ublk operations into existing normalized requests and maps terminal results back to the kernel. Cover the operations needed by the ext4 acceptance workload:

- exact-range reads;
- exact-range writes;
- flush;
- operation ordering represented by the current normalized contract;
- explicit refusal of unsupported discard, write-zeroes, zoned, or FUA behavior unless the complete path truthfully supports it.

Keep queue IDs, ublk tags, UAPI structs, SQEs/CQEs, raw descriptors, candidate-library handles, and unsafe code out of the portable core. Validate every kernel-provided range, count, flag, and identifier before allocation or semantic admission.

### 3.3 Bounded ownership

Before accepting a kernel request, reserve bounded existing admission resources. The adapter must retain the ublk tag and any associated buffer until the semantic operation reaches a safe terminal/reconciliation point. A stale completion must not target a reused operation slot or kernel tag.

Use the smallest queue/depth configuration that proves behavior. Record hard limits for:

- ublk queues and queue depth;
- operation slots;
- buffers and maximum transfer;
- outstanding child operations;
- trace volume;
- shutdown/drain time.

Backpressure or deterministic failure must occur before irreversible mutation when admission is unavailable.

### 3.4 Minimal service lifecycle

Provide only the local lifecycle needed by the acceptance workflow:

```text
Stopped -> Assembling -> Published -> AdmissionClosed -> Draining -> Stopped
```

Assembly must validate fixture identity, geometry, topology epoch, backing/export non-aliasing, recovery-state access, and required role availability before publishing the device. Failure releases partial claims and leaves no writable virtual device.

Do not design a network API, GUI, permanent daemon framework, stable public wire protocol, system installer, or full deployment manager. A small private/local command boundary is sufficient.

### 3.5 Evidence surface

Extend the existing `dwv demo` command family or its smallest current equivalent with a Linux-VM workflow. Human and JSON output must agree and distinguish:

- portable semantic success;
- ublk frontend success;
- host-file synchronization evidence;
- unsupported kernel/flag/capability;
- blocked, failed, uncertain, or reconciliation-required lifecycle.

Emit bounded privacy-safe normalized traces that correlate kernel submission, normalized request, semantic result, and kernel completion without payload bytes or raw pointers.

---

## 4. Deliberate non-goals

Goal-v7 does not require:

- multiple simultaneously published data devices;
- coherent whole-array group publication;
- daemon recovery/reissue after an unclean server death;
- a production raw-device/io_uring backend;
- multi-queue throughput optimization, registered buffers, zero copy, affinity, or batching;
- XFS, dm-crypt, mergerfs, databases, fio/fsx certification, or sustained benchmarks;
- systemd, udev rules, boot/shutdown integration, mount namespaces, or automount policy;
- physical disks, FUA certification, controller-cache claims, resets, cable pulls, or power loss;
- degraded operation, online rebuild, P/Q, degraded writes, journal/PPL, or stable format declarations;
- adding, removing, replacing, or resizing data/parity assignments, changing the coding profile, or performing an online topology transition;
- macOS frontend work.

A file-backed member inside the isolated Linux VM is sufficient. This goal proves the frontend boundary, not production storage durability.

---

## 5. Dependency-ordered execution

### A. Re-establish the safe environment

- Recreate the no-host-mount Linux VM profile.
- Confirm upstream known-good ublk operation if needed to distinguish environment from DiskWeave defects.
- Record exact guest image/kernel/module/io_uring facts and unsupported claim boundaries.

### B. Prove one request before building lifecycle

- Expose one DiskWeave-owned fixed-size device.
- Translate one aligned read and write through the existing normalized/service path.
- Compare exact bytes and trace semantics.
- Stop if candidate/UAPI mechanics force Linux types into the portable core.

### C. Complete the ext4 operation set

- Add only the operation/flag mappings ext4 actually exercises.
- Make unsupported requests fail explicitly.
- Verify bounded tags, buffers, slot generations, completions, and errors.

### D. Add clean lifecycle and reopen

- Assemble, publish, mount, exercise, unmount, drain, remove, stop, reopen, and remount.
- Preserve ordinary-member direct-read behavior after every DiskWeave handle closes.

### E. Integrate evidence and knowledge workflow

- Run goal-v6 context/readiness on this first real Linux product change.
- Add sparse requirement/code/evidence relationships only at meaningful ownership boundaries.
- Update only affected documentation claims.
- Archive completed change artifacts and name the next acceptance gap.

---

## 6. Required failure checks

At minimum verify:

- kernel lacks ublk support;
- control device exists but permissions are insufficient;
- requested range overflows or exceeds virtual geometry;
- unsupported operation or flag arrives;
- backing and exported identities alias;
- backing file is missing, resized, replaced, or already claimed;
- operation-slot/buffer admission is exhausted;
- short or failed backing I/O occurs;
- ublk completion mapping cannot represent the semantic result;
- ext4 unmount succeeds but frontend drain/checkpoint fails;
- restart sees dirty, uncertain, or mismatched recovery evidence;
- cleanup is interrupted and stale virtual devices remain visible.

Failures must leave bounded diagnostics and conservative recovery state. They must not silently zero-fill, truncate, report clean, or leave a writable device backed by released authority.

---

## 7. Verification

### Deterministic adapter checks

Exercise synthetic ublk requests for ranges, flags, limits, ordering, failures, abandonment, stale tags, duplicate completion, and bounded trace encoding without requiring a mounted filesystem.

### Live VM acceptance

Record one complete run on the selected VM:

- module/control detection;
- DiskWeave device creation;
- fixed geometry;
- ext4 format/mount;
- bounded create/overwrite/rename/fsync/read/delete workload;
- exact content hash;
- clean unmount/drain/device removal;
- service restart and ext4 remount;
- identical retained content;
- portable parity/recovery/integrity inspection;
- direct read-only member access after shutdown.

### Regression

Run focused neighboring tests and the complete portable ordinary-file demo. Validate that no ublk/io_uring/platform dependency enters production portable crates that do not own the adapter.

### Claim boundary

Success proves a functional file-backed Linux ublk frontend on the exact VM/kernel configuration tested. It does not prove production concurrency, daemon recovery, raw-device durability, broader filesystem compatibility, deployment correctness, or hardware safety.

---

## 8. Completion criteria

Goal-v7 is complete only when:

1. A fresh documented ARM64 Linux VM can run the workflow without host mounts or host block-device access.
2. DiskWeave—not an operator-built upstream utility—owns the serving process exposed to the user.
3. A real `/dev/ublkbN` device carries reads, writes, and flushes through the existing normalized and portable service path.
4. The device has fixed validated geometry and unsupported operations fail explicitly.
5. Tags, buffers, and operation slots remain bounded and generation-safe through terminal completion.
6. A disposable ext4 filesystem completes the required healthy workload.
7. Clean unmount, drain, checkpoint, device removal, stop, reopen, and remount preserve exact content.
8. Assembly and restart fail closed on identity, geometry, capability, or recovery conflicts.
9. No active backing/export alias or direct backing bypass is possible.
10. The ordinary backing member remains independently readable after shutdown.
11. Normalized evidence is bounded, versioned, privacy-safe, and replayable at the current portable boundary.
12. Existing portable demos and focused regression checks pass.
13. The goal-v6 knowledge/readiness workflow is exercised by the change.
14. Every relevant normative v0.8 statement is mapped to an exact current requirement, explicitly superseded by one, or added/modified in a validated canonical spec before its implementation.
15. Exact environment support and non-claims are recorded.
16. The one-data-slot Linux acceptance profile introduces no portable or persistent array-width limit, and wider topologies fail as unsupported without partial publication or mutation.
17. Residual concurrency, daemon-loss, multi-member, topology-transition, workload, deployment, and hardware gaps remain explicit.
18. Checked dirty-region derivation covers every touched region end to end; no service-local hard-coded mapping remains.
19. Store-assigned monotonic watermarks and independent fence composition reject future, stale, partial, omitted-region, and cross-store evidence.
20. Recovery snapshot failures remain explicit and never fall back to generation zero.
21. Store and recovery claims are released by process death without marker cleanup, with partial-acquisition and reacquisition checks.
22. The TLA+, Kani, independent model, process-crash integration, and deterministic regression evidence required by the correction baseline pass and are mapped to canonical requirements and v0.8 properties.

---

## 9. Forbidden shortcuts

Goal-v7 is not satisfied by:

- invoking upstream `ublk`/`ublksrv` as the DiskWeave product frontend;
- exposing a kernel loop device instead of a DiskWeave ublk server;
- routing only a synthetic request while ext4 bypasses DiskWeave;
- directly mounting the active backing file;
- acknowledging flush/FUA without the corresponding semantic evidence;
- treating process exit as successful drain;
- using unbounded tasks, buffers, queues, or logs;
- copying parity/recovery semantics into the Linux adapter;
- marking environment-gated live tests complete because they were skipped;
- using guessed or sentinel store watermarks as fence evidence;
- deriving one dirty region from a write start when the range may cross regions;
- replacing recovery access failure with generation zero, clean state, or successful trace output;
- treating marker-file existence or destructor cleanup as writable ownership;
- treating a passing live ext4 path as a substitute for the required model, bounded proof, or process-crash evidence;
- expanding into multi-member coherence or deployment before the single-device acceptance is solid.

Final invariant:

> A real ext4 filesystem uses a DiskWeave-owned Linux block device, while every stronger concurrency, recovery, deployment, and durability claim remains visibly unproven.
