## MODIFIED Requirements

### Requirement: Linux prerequisite probing is executable and non-destructive
<!-- dwv:req req.linux-ublk-frontend.linux-prerequisite-probing-is-executable-and-non-destructive -->

The Linux frontend SHALL provide a bounded probe that reports architecture, kernel release, ublk control availability or loadable module, required io_uring/UAPI support, effective device-management permission, and ext4/mount tooling availability. The probe SHALL distinguish present, unsupported, and blocked facts without creating a device, mounting storage, changing host files, or treating headers, module files, or successful compilation as proof of working I/O. Deterministic evidence SHALL exercise both unavailable-control and denied-control classifications without requiring either condition on the acceptance host.

#### Scenario: The ublk control device is absent

- **WHEN** the module cannot be loaded or the control device remains unavailable
- **THEN** the probe reports an exact unsupported reason and performs no fixture or block-device mutation

#### Scenario: Control access is denied

- **WHEN** the control device exists but the process lacks required permission
- **THEN** the probe reports a blocked permission result rather than claiming ublk support

### Requirement: Kernel requests preserve normalized semantics
<!-- dwv:req req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics -->

The adapter SHALL validate every kernel-provided operation, flag, range, count, and identifier before allocation or semantic admission, then translate supported exact-range reads, exact-range writes, and flushes into the current normalized request and portable service contracts. It SHALL preserve stable request and frontend identities, explicit stable target slot, captured topology epoch, operation, checked byte range, optional generational buffer token, submission sequence, ordering intent, and durability intent through the semantic boundary, and SHALL map semantic terminal outcomes deterministically. Unsupported discard, write-zeroes, zoned operations, FUA, preflush, or unknown flags SHALL fail explicitly unless the complete path advertises and establishes equivalent semantics. Every retained frontend trace record SHALL separately encode the kernel submission, normalized request fields, semantic terminal result, and kernel completion so deterministic replay can reject any unrepresentable or divergent mapping.

#### Scenario: An ext4 read, write, or flush arrives

- **WHEN** the request is aligned, within fixed geometry, uses only advertised flags, and required authority is current
- **THEN** it completes through the normalized portable service path with exact range and persistence evidence

#### Scenario: A range overflows virtual geometry

- **WHEN** sector conversion, byte-count conversion, or checked end arithmetic overflows or exceeds the published capacity
- **THEN** the adapter rejects the request before buffer allocation or protected mutation

#### Scenario: Unsupported intent arrives

- **WHEN** a request carries discard, write-zeroes, zoned, FUA, preflush, or an unknown operation or flag not supported by the complete path
- **THEN** the adapter returns an explicit unsupported result and never translates it into success, ordinary write, zero-fill, or flush

#### Scenario: A semantic result has no kernel completion mapping

- **WHEN** the semantic terminal result cannot be represented by the selected ublk completion contract
- **THEN** the adapter records the semantic result and explicit mapping refusal and does not report successful completion

#### Scenario: A retained frontend trace is replayed

- **WHEN** a bounded versioned frontend trace is imported
- **THEN** replay revalidates every normalized request and terminal mapping in sequence without executing backing I/O and reports the first divergence

### Requirement: Kernel tags and operation resources remain bounded and generation-safe
<!-- dwv:req req.linux-ublk-frontend.kernel-tags-and-operation-resources-remain-bounded-and-generation-safe -->

The frontend SHALL publish finite queue, depth, transfer, buffer, operation-slot, child-operation, trace, and shutdown bounds. It SHALL reserve existing semantic admission before accepting irreversible work and retain each kernel tag and buffer until the corresponding generational operation reaches a safe terminal or reconciliation point. Exhaustion SHALL backpressure or fail deterministically before protected mutation; stale or duplicate completions SHALL NOT complete a reused tag or reclaim live resources. Deterministic adapter evidence SHALL cover admission exhaustion, explicit abandonment, stale and duplicate completion, and trace-bound exhaustion.

#### Scenario: Admission is exhausted

- **WHEN** no configured tag, buffer, or semantic operation slot is available
- **THEN** the request is backpressured or fails with bounded resource exhaustion before protected mutation

#### Scenario: A stale completion names a reused tag

- **WHEN** a completion carries an earlier generation than the tag's current admitted operation
- **THEN** it is rejected without completing or reclaiming the current operation

#### Scenario: An admitted request is abandoned

- **WHEN** frontend ownership ends before a safe terminal completion is observed
- **THEN** the tag remains unavailable until explicit reconciliation releases it and the trace records an abandoned terminal result

#### Scenario: The trace bound is exhausted

- **WHEN** the configured maximum trace record count has been retained
- **THEN** another request is refused before semantic admission rather than executed without evidence

### Requirement: Assembly and shutdown preserve ownership and recovery authority
<!-- dwv:req req.linux-ublk-frontend.assembly-and-shutdown-preserve-ownership-and-recovery-authority -->

Before publication, the Linux workflow SHALL validate fixture ownership, stable opened-file identities, fixed geometry, topology epoch, recovery-state access, required roles, capability compatibility, and backing/export non-aliasing, and SHALL acquire every writable store and recovery claim with crash-releasing operating-system descriptor locks. Marker existence SHALL not own a claim. Any failure SHALL release partial claims and leave no writable endpoint. Reacquisition after process death SHALL revalidate current store and recovery authority. Shutdown SHALL close admission, quiesce through a captured sequence, drain or conservatively reconcile admitted operations, require the portable global flush/checkpoint with exact region and store-watermark fence evidence, remove only the owned ublk endpoint, and release claims. Unmount success SHALL NOT mask a later drain, checkpoint, or cleanup failure.

#### Scenario: A backing file is replaced or resized

- **WHEN** current opened identity or geometry differs from the fixture's recorded assignment
- **THEN** assembly fails before publication or recovery/payload mutation

#### Scenario: Backing and export identities alias

- **WHEN** a backing payload and proposed exported endpoint resolve to the same underlying object or permit a direct active backing bypass
- **THEN** assembly fails before publication

#### Scenario: Clean shutdown completes

- **WHEN** consumers have unmounted, admission closes, all admitted operations drain, the portable flush/checkpoint succeeds, and the owned ublk endpoint is removed
- **THEN** the service closes all backing/recovery handles and reports a bounded successful shutdown

#### Scenario: Shutdown cannot complete after unmount

- **WHEN** consumers have unmounted but drain, checkpoint, or owned-endpoint removal fails
- **THEN** shutdown reports failure or reconciliation-required state and does not report a clean checkpoint

#### Scenario: Stale cleanup is interrupted

- **WHEN** startup or cleanup observes an owned stale ublk endpoint
- **THEN** competing publication is refused and only an explicit bounded cleanup operation for the owned disposable fixture may remove it

#### Scenario: The owner process dies

- **WHEN** the frontend process is killed without running shutdown or destructors
- **THEN** store and recovery descriptor claims are released, but dirty or indeterminate recovery state and any stale endpoint remain explicit inputs to conservative reacquisition

#### Scenario: Recovery authority cannot be loaded

- **WHEN** recovery snapshot access fails or returns missing, corrupt, stale, or mismatched authority
- **THEN** assembly fails without substituting generation zero, publishing an endpoint, or mutating protected payloads

### Requirement: Live ext4 acceptance evidence is bounded and scope-accurate
<!-- dwv:req req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate -->

A reproducible ARM64 Linux workflow SHALL create the fixture, publish a real DiskWeave-owned `/dev/ublkbN`, format and mount ext4, perform bounded create/overwrite/rename/fsync/read/delete operations, verify exact content hashes, unmount and cleanly stop, restart against the same fixture, remount and verify retained content, inspect parity/recovery/integrity disposition, and mount the ordinary data member read-only after final shutdown. Evidence SHALL retain a bounded versioned trace that separately correlates kernel submission, normalized request, semantic result, and kernel completion without payload bytes, raw pointers, or private host paths, and SHALL replay that trace successfully before claiming acceptance.

#### Scenario: The complete VM workflow passes

- **WHEN** every required operation and lifecycle transition succeeds on the selected guest/kernel profile and its retained trace replays without divergence
- **THEN** evidence may claim functional file-backed Linux ublk/ext4 behavior only for that exact environment and records all configured bounds

#### Scenario: Live execution is unavailable

- **WHEN** the required ARM64 VM, kernel capability, permission, or tool is unavailable
- **THEN** deterministic adapter tests may pass but Linux frontend and ext4 acceptance remain explicitly unmet rather than skipped as success

#### Scenario: A stronger claim is requested

- **WHEN** evidence is used to infer production concurrency, daemon recovery, raw-device durability, FUA, broader filesystems, deployment correctness, online topology mutation, or hardware safety
- **THEN** the report identifies those claims as unsupported

### Requirement: Correction evidence covers the composed correctness boundaries
<!-- dwv:req req.linux-ublk-frontend.correction-evidence-covers-the-composed-correctness-boundaries -->

OS-031 completion SHALL retain deterministic regression evidence that the canonical dirty-region mapping covers all intersected regions, store watermarks are monotonic and fence composition rejects future, stale, partial, omitted-region, and cross-store evidence, recovery access failures cannot become generation zero, and store/recovery ownership is released by process death without marker cleanup. It SHALL preserve a TLA+ model checking mutation crash points around intent, home writes, fences, and checkpoints; a bounded Kani harness for checked region mapping; an independent fence-coverage model; and a member-process crash integration case. Evidence SHALL record each bounded proof's exact symbolic domain and assumptions. Counterexamples SHALL be retained as deterministic regressions.

#### Scenario: One proof layer is unavailable

- **WHEN** any required model, bounded proof, process-crash integration, or executable regression cannot run in the recorded environment
- **THEN** its covered claim remains explicitly unmet and passing neighboring checks do not substitute for it

#### Scenario: Evidence records the acceptance boundary

- **WHEN** all correction checks and the live ublk/ext4 workflow pass
- **THEN** the evidence maps each check to its canonical requirement and architecture-v0.8 property, records exact proof bounds, and still denies production SQLite selection, physical power-loss durability, FUA, broader concurrency, multi-device publication, and online topology mutation
