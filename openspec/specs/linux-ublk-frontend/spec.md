## Purpose

Define the first bounded Linux ublk frontend that carries a real ext4 workload through DiskWeave's normalized request and portable service path while keeping platform evidence and unsupported claims explicit.
## Requirements
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
<!-- dwv:refines req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics -->
<!-- dwv:refines req.normalized-block-semantics.ordering-and-durability-intent-cannot-be-silently-weakened -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->

The Linux adapter SHALL validate every kernel-provided operation, flag, range, count, and identifier before semantic admission, then map supported exact-range reads, writes, and flushes directly into the canonical normalized request and portable service contracts. It SHALL preserve every normalized request field, pass write payloads as borrowed frontend-owned buffers without allocating a second request-boundary payload, and map terminal outcomes deterministically. Unsupported discard, write-zeroes, zoned operations, FUA, preflush, or unknown flags SHALL fail explicitly unless the complete path advertises and establishes the requested semantics.

#### Scenario: An ext4 read, write, or flush arrives

- **WHEN** the request is aligned, within fixed geometry, uses only advertised flags, and required authority is current
- **THEN** it completes through the normalized portable service path with exact range and persistence evidence

#### Scenario: A range overflows virtual geometry

- **WHEN** sector conversion, byte-count conversion, or checked end arithmetic overflows or exceeds the published capacity
- **THEN** the adapter rejects the request before semantic admission or protected mutation

#### Scenario: Unsupported intent arrives

- **WHEN** a request carries an operation or intent unsupported by the complete path
- **THEN** the adapter returns an explicit unsupported result and never translates it into a weaker operation or success

#### Scenario: A semantic result has no kernel completion mapping

- **WHEN** the semantic terminal result cannot be represented by the selected ublk completion contract
- **THEN** the adapter records the semantic result and explicit mapping refusal and does not report successful completion

#### Scenario: A retained frontend trace is replayed

- **WHEN** a bounded versioned frontend trace is imported
- **THEN** replay revalidates every normalized request and terminal mapping in sequence without executing backing I/O and reports the first divergence

#### Scenario: A write payload crosses the request boundary

- **WHEN** ublk supplies a validated write buffer
- **THEN** translation and service dispatch retain the same borrowed payload storage until synchronous semantic completion rather than cloning it into a service request wrapper

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

### Requirement: The initial Linux publication profile is complete and narrow
<!-- dwv:req req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow -->
<!-- dwv:requires req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.healthy-portable-io.publication-identity-is-derived-from-admitted-semantics -->

The initial Linux profile SHALL publish exactly one writable endpoint only when the validated topology contains exactly one data slot, one parity slot, and all required current recovery and portable admission authority. The endpoint SHALL target that explicit stable data slot while the portable topology remains variable-width. Publication metadata SHALL carry the exact publication identity supplied by the admitted service, including its array identity, and live discovery SHALL match by array identity before comparing the complete publication identity rather than reconstructing admission from fixture or path coincidence. Publication SHALL return a distinct successful published result only after the owned endpoint exists and is accepting work; admission success alone SHALL remain pre-publication. A wider or otherwise unsupported valid topology SHALL be reported as unsupported before publication or mutation; the adapter SHALL NOT publish a subset, classify the topology itself as invalid, or encode the profile limit in portable APIs or persistent state.

#### Scenario: The acceptance topology is assembled

- **WHEN** one data assignment, one parity assignment, recovery authority, portable admission, geometry, identity, epoch, capabilities, and admitted publication identity all validate and endpoint publication succeeds
- **THEN** the frontend reports the complete data-device group published as one fixed-size writable endpoint bound to that publication identity

#### Scenario: Admission succeeds before publication

- **WHEN** portable admission succeeds but endpoint publication has not completed
- **THEN** the frontend remains pre-publication and does not report an online/read-write endpoint

#### Scenario: A different admitted object shares the fixture directory

- **WHEN** live endpoint metadata carries the same array identity but a publication identity different from the currently assessed admitted service even if fixture paths or labels coincide
- **THEN** discovery reports reconciliation-required and does not report the current array online

#### Scenario: A different admitted object for the same array is live

- **WHEN** live endpoint metadata carries the same array identity but a publication identity different from the currently assessed admitted service
- **THEN** discovery reports reconciliation-required and does not report the current array online

#### Scenario: An unrelated array is live

- **WHEN** live endpoint metadata carries a different array identity from the currently assessed admitted service
- **THEN** discovery ignores that endpoint while continuing the bounded search for the requested array

#### Scenario: A wider topology is supplied

- **WHEN** a valid topology contains more than one data slot or otherwise exceeds the initial Linux profile
- **THEN** publication is refused as unsupported with no endpoint, partial claims, recovery mutation, or payload mutation

#### Scenario: Publication fails after admission

- **WHEN** current state changes, endpoint creation fails, or publication outcome requires reconciliation after portable admission
- **THEN** the frontend reports the exact failed or reconciliation-required outcome and does not report successful publication

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
<!-- dwv:requires req.evidence-boundaries.evidence-scope-is-explicit -->
<!-- dwv:requires req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow -->

A Linux acceptance profile already declared supported by current canonical Linux requirements SHALL run a reproducible bounded workflow that creates the disposable fixture, publishes a real DiskWeave-owned ublk endpoint, formats and mounts ext4, performs bounded create/overwrite/rename/fsync/read/delete operations, verifies exact content, unmounts and cleanly stops, restarts against the same fixture, remounts and verifies retained content, inspects parity/recovery/integrity disposition, and mounts the ordinary data member read-only after final shutdown. Evidence SHALL record the exact environment, configured bounds, source digest, and correlated kernel submission, normalized request, semantic result, and completion without payload bytes, raw pointers, or private host paths. A successful run in any environment SHALL NOT create or broaden a canonically supported profile.

#### Scenario: A canonically supported profile passes

- **WHEN** every required operation and lifecycle transition succeeds for a profile already supported by current canonical Linux requirements
- **THEN** evidence may claim functional file-backed Linux ublk/ext4 behavior only for that canonical profile in the recorded environment and does not establish another supported profile

#### Scenario: Live execution is unavailable

- **WHEN** the required architecture, VM, kernel capability, permission, or tool is unavailable
- **THEN** deterministic adapter tests may pass but Linux frontend and ext4 acceptance remain explicitly unmet rather than skipped as success

#### Scenario: The canonical profile passes in another environment

- **WHEN** the same canonically supported profile completes successfully in an additional environment
- **THEN** evidence records that environment but the successful run does not create or broaden canonical Linux support

#### Scenario: A stronger claim is requested

- **WHEN** evidence is used to infer production concurrency, daemon recovery, raw-device durability, FUA, broader filesystems, deployment correctness, online topology mutation, or hardware safety
- **THEN** the report identifies those claims as unsupported
