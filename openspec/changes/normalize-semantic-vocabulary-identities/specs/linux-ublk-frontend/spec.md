## MODIFIED Requirements

### Requirement: Assembly and shutdown preserve ownership and recovery authority
<!-- dwv:req req.linux-ublk-frontend.assembly-and-shutdown-preserve-ownership-and-recovery-authority -->

Before publication, the Linux workflow SHALL validate fixture ownership, stable opened-file identities, fixed geometry, topology epoch, recovery-state access, required roles, capability compatibility, and backing/export non-aliasing, and SHALL acquire every writable store and recovery claim with crash-releasing operating-system descriptor locks. Marker existence SHALL not own a claim. Any failure SHALL release partial claims and leave no writable endpoint. Reacquisition after process death SHALL revalidate current store and recovery authority. Shutdown SHALL close admission, quiesce through a captured sequence, drain or conservatively reconcile admitted operations, require the portable global flush and recovery state `CLEAN` commit with exact region and store-watermark persistence evidence, remove only the owned ublk endpoint, and release claims. Unmount success SHALL NOT mask a later drain, recovery `CLEAN` commit, or cleanup failure.

#### Scenario: A backing file is replaced or resized

- **WHEN** current opened identity or geometry differs from the fixture's recorded assignment
- **THEN** assembly fails before publication, recovery mutation, or a data/parity write

#### Scenario: Backing and export identities alias

- **WHEN** a backing payload and proposed exported endpoint resolve to the same underlying object or permit a direct active backing bypass
- **THEN** assembly fails before publication

#### Scenario: Clean shutdown completes

- **WHEN** consumers have unmounted, admission closes, all admitted operations drain, the portable flush and recovery state `CLEAN` commit succeed, and the owned ublk endpoint is removed
- **THEN** the service closes all backing/recovery handles and reports a bounded successful shutdown

#### Scenario: Shutdown cannot complete after unmount

- **WHEN** consumers have unmounted but drain, recovery `CLEAN` commit, or owned-endpoint removal fails
- **THEN** shutdown reports failure or reconciliation-required state and does not report recovery state `CLEAN`

#### Scenario: Stale cleanup is interrupted

- **WHEN** startup or cleanup observes an owned stale ublk endpoint
- **THEN** competing publication is refused and only an explicit bounded cleanup operation for the owned disposable fixture may remove it

#### Scenario: The owner process dies

- **WHEN** the frontend process is killed without running shutdown or destructors
- **THEN** store and recovery descriptor claims are released, but dirty or indeterminate recovery state and any stale endpoint remain explicit inputs to conservative reacquisition

#### Scenario: Recovery authority cannot be loaded

- **WHEN** recovery snapshot access fails or returns missing, corrupt, stale, or mismatched authority
- **THEN** assembly fails without substituting generation zero, publishing an endpoint, or mutating protected payloads

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
- **THEN** the adapter rejects the request before semantic admission or a data/parity write

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

The frontend SHALL publish finite queue, depth, transfer, buffer, operation-slot, child-operation, trace, and shutdown bounds. It SHALL reserve existing semantic admission before accepting irreversible work and retain each kernel tag and buffer until the corresponding generational operation reaches a safe terminal or reconciliation point. Exhaustion SHALL backpressure or fail deterministically before a data/parity write; stale or duplicate completions SHALL NOT complete a reused tag or reclaim live resources. Deterministic adapter evidence SHALL cover admission exhaustion, explicit abandonment, stale and duplicate completion, and trace-bound exhaustion.

#### Scenario: Admission is exhausted

- **WHEN** no configured tag, buffer, or semantic operation slot is available
- **THEN** the request is backpressured or fails with bounded resource exhaustion before a data/parity write

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

The initial Linux profile SHALL publish exactly one writable endpoint only when the validated topology contains exactly one data slot, one parity slot, and all required current recovery and portable admission authority. The endpoint SHALL target that explicit stable data slot while the portable topology remains variable-width. Publication metadata SHALL carry the exact publication identity supplied by the admitted service, including its array identity, and live discovery SHALL match by array identity before comparing the complete publication identity rather than reconstructing admission from fixture or path coincidence. Publication SHALL return a distinct successful published result only after the owned endpoint exists and is accepting work; admission success alone SHALL remain pre-publication. A wider or otherwise unsupported valid topology SHALL be reported as unsupported before publication or data/parity writes; the adapter SHALL NOT publish a subset, classify the topology itself as invalid, or encode the profile limit in portable APIs or persistent state.

#### Scenario: The acceptance topology is assembled

- **WHEN** one data assignment, one parity assignment, recovery authority, portable admission, geometry, identity, epoch, capabilities, and admitted publication identity all validate and endpoint publication succeeds
- **THEN** the frontend reports the complete data-device group published as one fixed-size writable endpoint bound to that publication identity

#### Scenario: Admission succeeds before publication

- **WHEN** portable admission succeeds but endpoint publication has not completed
- **THEN** the frontend remains pre-publication and does not report an online/read-write endpoint

#### Scenario: A different admitted object shares the fixture directory

- **WHEN** live endpoint metadata carries the same array identity but a publication identity different from the currently assessed admitted service even if fixture paths or labels coincide
- **THEN** discovery reports that reconciliation is required and does not report the current array online

#### Scenario: A different admitted object for the same array is live

- **WHEN** live endpoint metadata carries the same array identity but a publication identity different from the currently assessed admitted service
- **THEN** discovery reports that reconciliation is required and does not report the current array online

#### Scenario: An unrelated array is live

- **WHEN** live endpoint metadata carries a different array identity from the currently assessed admitted service
- **THEN** discovery ignores that endpoint while continuing the bounded search for the requested array

#### Scenario: A wider topology is supplied

- **WHEN** a valid topology contains more than one data slot or otherwise exceeds the initial Linux profile
- **THEN** publication is refused as unsupported with no endpoint, partial claims, recovery mutation, or data/parity write

#### Scenario: Publication fails after admission

- **WHEN** current state changes, endpoint creation fails, or publication outcome requires reconciliation after portable admission
- **THEN** the frontend reports the exact failed result or that reconciliation is required and does not report successful publication

### Requirement: The disposable Linux fixture remains independently inspectable
<!-- dwv:req req.linux-ublk-frontend.the-disposable-linux-fixture-remains-independently-inspectable -->

The Linux workflow SHALL create or open only a bounded identifiable disposable fixture containing ordinary data and parity payload files plus separate recovery state. Initialization and inspection SHALL validate that every referenced path remains within the owned root and matches recorded type, identity, and protected length. After successful shutdown, read-only inspection SHALL report exact data/parity hashes and recovery/integrity result without mounting or changing the fixture, and the ordinary data member SHALL remain independently mountable read-only as its conventional filesystem image.

#### Scenario: A fixture path escapes its root

- **WHEN** a manifest reference is absolute, traverses outside the root, is missing, or has changed identity or length
- **THEN** initialization, assembly, or inspection refuses before changing protected state

#### Scenario: The frontend is stopped

- **WHEN** all DiskWeave handles and the owned endpoint are closed after clean shutdown
- **THEN** the data member can be opened and mounted read-only without the frontend while parity and recovery remain separate
