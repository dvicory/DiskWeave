## Purpose

Define the first bounded Linux ublk frontend that carries a real ext4 workload through DiskWeave's normalized request and portable service path while keeping platform evidence and unsupported claims explicit.

## ADDED Requirements

### Requirement: Linux prerequisite probing is executable and non-destructive
<!-- dwv:req req.linux-ublk-frontend.linux-prerequisite-probing-is-executable-and-non-destructive -->

The Linux frontend SHALL provide a bounded probe that reports architecture, kernel release, ublk control availability or loadable module, required io_uring/UAPI support, effective device-management permission, and ext4/mount tooling availability. The probe SHALL distinguish present, unsupported, and blocked facts without creating a device, mounting storage, changing host files, or treating headers, module files, or successful compilation as proof of working I/O.

#### Scenario: The ublk control device is absent

- **WHEN** the module cannot be loaded or the control device remains unavailable
- **THEN** the probe reports an exact unsupported reason and performs no fixture or block-device mutation

#### Scenario: Control access is denied

- **WHEN** the control device exists but the process lacks required permission
- **THEN** the probe reports a blocked permission result rather than claiming ublk support

### Requirement: Kernel requests preserve normalized semantics
<!-- dwv:req req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics -->

The adapter SHALL validate every kernel-provided operation, flag, range, count, and identifier before allocation or semantic admission, then translate supported exact-range reads, exact-range writes, and flushes into the current normalized request and portable service contracts. It SHALL preserve stable request and frontend identities, explicit stable target slot, captured topology epoch, operation, checked byte range, optional generational buffer token, submission sequence, ordering intent, and durability intent through the semantic boundary, and SHALL map semantic terminal outcomes deterministically. Unsupported discard, write-zeroes, zoned operations, FUA, preflush, or unknown flags SHALL fail explicitly unless the complete path advertises and establishes equivalent semantics.

#### Scenario: An ext4 read, write, or flush arrives

- **WHEN** the request is aligned, within fixed geometry, uses only advertised flags, and required authority is current
- **THEN** it completes through the normalized portable service path with exact range and persistence evidence

#### Scenario: A range overflows virtual geometry

- **WHEN** sector conversion, byte-count conversion, or checked end arithmetic overflows or exceeds the published capacity
- **THEN** the adapter rejects the request before buffer allocation or protected mutation

#### Scenario: Unsupported intent arrives

- **WHEN** a request carries discard, write-zeroes, zoned, FUA, preflush, or an unknown operation or flag not supported by the complete path
- **THEN** the adapter returns an explicit unsupported result and never translates it into success, ordinary write, zero-fill, or flush

### Requirement: Kernel tags and operation resources remain bounded and generation-safe
<!-- dwv:req req.linux-ublk-frontend.kernel-tags-and-operation-resources-remain-bounded-and-generation-safe -->

The frontend SHALL publish finite queue, depth, transfer, buffer, operation-slot, child-operation, trace, and shutdown bounds. It SHALL reserve existing semantic admission before accepting irreversible work and retain each kernel tag and buffer until the corresponding generational operation reaches a safe terminal or reconciliation point. Exhaustion SHALL backpressure or fail deterministically before protected mutation; stale or duplicate completions SHALL NOT complete a reused tag or reclaim live resources.

#### Scenario: Admission is exhausted

- **WHEN** no configured tag, buffer, or semantic operation slot is available
- **THEN** the request is backpressured or fails with bounded resource exhaustion before protected mutation

#### Scenario: A stale completion names a reused tag

- **WHEN** a completion carries an earlier generation than the tag's current admitted operation
- **THEN** it is rejected without completing or reclaiming the current operation

### Requirement: The initial Linux publication profile is complete and narrow
<!-- dwv:req req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow -->

The initial Linux profile SHALL publish exactly one writable endpoint only when the validated topology contains exactly one data slot, one parity slot, and all required recovery authority. The endpoint SHALL target that explicit stable data slot while the portable topology remains variable-width. A wider or otherwise unsupported valid topology SHALL be reported as unsupported before publication or mutation; the adapter SHALL NOT publish a subset, classify the topology itself as invalid, or encode the profile limit in portable APIs or persistent state.

#### Scenario: The acceptance topology is assembled

- **WHEN** one data assignment, one parity assignment, recovery authority, geometry, identity, epoch, and capabilities all validate
- **THEN** the frontend may publish the complete data-device group as one fixed-size endpoint

#### Scenario: A wider topology is supplied

- **WHEN** a valid topology contains more than one data slot or otherwise exceeds the initial Linux profile
- **THEN** publication is refused as unsupported with no endpoint, partial claims, recovery mutation, or payload mutation

### Requirement: Assembly and shutdown preserve ownership and recovery authority
<!-- dwv:req req.linux-ublk-frontend.assembly-and-shutdown-preserve-ownership-and-recovery-authority -->

Before publication, the Linux workflow SHALL validate fixture ownership, stable opened-file identities, fixed geometry, topology epoch, recovery-state access, required roles, capability compatibility, and backing/export non-aliasing, and SHALL acquire every writable store and recovery claim with crash-releasing operating-system descriptor locks. Marker existence SHALL not own a claim. Any failure SHALL release partial claims and leave no writable endpoint. Reacquisition after process death SHALL revalidate current store and recovery authority. Shutdown SHALL close admission, quiesce through a captured sequence, drain or conservatively reconcile admitted operations, require the portable global flush/checkpoint with exact region and store-watermark fence evidence, remove the owned endpoint, close every store, and only then report success. Timeout, checkpoint failure, interrupted cleanup, or a visible stale endpoint SHALL remain failed or reconciliation-required and SHALL NOT report clean or release authority before submitted operations are terminal.

#### Scenario: A backing file is replaced or resized

- **WHEN** current opened identity or geometry differs from the fixture's recorded assignment
- **THEN** assembly fails before publication or recovery/payload mutation

#### Scenario: Backing and export identities alias

- **WHEN** a backing payload and proposed exported endpoint resolve to the same underlying object or permit a direct active backing bypass
- **THEN** assembly fails before publication

#### Scenario: Clean shutdown completes

- **WHEN** consumers have unmounted, admission closes, all admitted operations drain, the portable flush/checkpoint succeeds, and the owned ublk endpoint is removed
- **THEN** the service closes all backing/recovery handles and reports a bounded successful shutdown

#### Scenario: Stale cleanup is interrupted

- **WHEN** startup or cleanup observes an owned stale ublk endpoint
- **THEN** competing publication is refused and only an explicit bounded cleanup operation for the owned disposable fixture may remove it

#### Scenario: The owner process dies

- **WHEN** the frontend process is killed without running shutdown or destructors
- **THEN** store and recovery descriptor claims are released, but dirty or indeterminate recovery state and any stale endpoint remain explicit inputs to conservative reacquisition

#### Scenario: Recovery authority cannot be loaded

- **WHEN** recovery snapshot access fails or returns missing, corrupt, stale, or mismatched authority
- **THEN** assembly fails without substituting generation zero, publishing an endpoint, or mutating protected payloads


### Requirement: The disposable Linux fixture remains independently inspectable
<!-- dwv:req req.linux-ublk-frontend.the-disposable-linux-fixture-remains-independently-inspectable -->

The Linux workflow SHALL create or open only a bounded identifiable disposable fixture containing ordinary data and parity payload files plus separate recovery state. Initialization and inspection SHALL validate that every referenced path remains within the owned root and matches recorded type, identity, and protected length. After successful shutdown, read-only inspection SHALL report exact data/parity hashes and recovery/integrity disposition without mounting or mutating the fixture, and the ordinary data member SHALL remain independently mountable read-only as its conventional filesystem image.

#### Scenario: A fixture path escapes its root

- **WHEN** a manifest reference is absolute, traverses outside the root, is missing, or has changed identity or length
- **THEN** initialization, assembly, or inspection refuses before protected mutation

#### Scenario: The frontend is stopped

- **WHEN** all DiskWeave handles and the owned endpoint are closed after clean shutdown
- **THEN** the data member can be opened and mounted read-only without the frontend while parity and recovery remain separate

### Requirement: Live ext4 acceptance evidence is bounded and scope-accurate
<!-- dwv:req req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate -->

A reproducible ARM64 Linux workflow SHALL create the fixture, publish a real DiskWeave-owned `/dev/ublkbN`, format and mount ext4, perform bounded create/overwrite/rename/fsync/read/delete operations, verify exact content hashes, unmount and cleanly stop, restart against the same fixture, remount and verify retained content, inspect parity/recovery/integrity disposition, and mount the ordinary data member read-only after final shutdown. Evidence SHALL correlate bounded kernel submission, normalized request, semantic result, and kernel completion without payload bytes, raw pointers, or private host paths.

#### Scenario: The complete VM workflow passes

- **WHEN** every required operation and lifecycle transition succeeds on the selected guest/kernel profile
- **THEN** evidence may claim functional file-backed Linux ublk/ext4 behavior only for that exact environment and records all configured bounds

#### Scenario: Live execution is unavailable

- **WHEN** the required ARM64 VM, kernel capability, permission, or tool is unavailable
- **THEN** deterministic adapter tests may pass but Linux frontend and ext4 acceptance remain explicitly unmet rather than skipped as success

#### Scenario: A stronger claim is requested

- **WHEN** evidence is used to infer production concurrency, daemon recovery, raw-device durability, FUA, broader filesystems, deployment correctness, online topology mutation, or hardware safety
- **THEN** the report identifies those claims as unsupported

### Requirement: Correction evidence covers the composed correctness boundaries
<!-- dwv:req req.linux-ublk-frontend.correction-evidence-covers-the-composed-correctness-boundaries -->

The Linux frontend's correction evidence SHALL retain deterministic regressions proving that the canonical dirty-region mapping covers all intersected regions, store watermarks are monotonic and fence composition rejects future, stale, partial, omitted-region, and cross-store evidence, recovery access failures cannot become generation zero, and store/recovery ownership is released by process death without marker cleanup. It SHALL include a TLA+ model checking mutation crash points around intent, home writes, fences, and checkpoints; a bounded Kani harness for checked region mapping; an independent fence-coverage model; and a member-process crash integration case. Counterexamples SHALL be retained as deterministic regressions.

#### Scenario: One proof layer is unavailable

- **WHEN** any required model, bounded proof, process-crash integration, or executable regression cannot run in the recorded environment
- **THEN** its covered claim remains explicitly unmet and passing neighboring checks do not substitute for it

#### Scenario: Evidence records the acceptance boundary

- **WHEN** all correction checks and the live ublk/ext4 workflow pass
- **THEN** the evidence maps each check to its canonical requirement and v0.8 property and still denies production SQLite selection, physical power-loss durability, FUA, broader concurrency, multi-device publication, and online topology mutation

