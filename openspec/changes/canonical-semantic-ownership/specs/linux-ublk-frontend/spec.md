## MODIFIED Requirements

### Requirement: Kernel requests preserve normalized semantics
<!-- dwv:req req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics -->
<!-- dwv:refines req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics -->
<!-- dwv:refines req.normalized-block-semantics.ordering-and-durability-intent-cannot-be-silently-weakened -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->

The Linux adapter SHALL validate every kernel-provided operation, flag, range, count, and identifier before allocation or semantic admission, then map supported exact-range reads, writes, and flushes into the canonical normalized request and portable service contracts. It SHALL preserve every normalized request field and map terminal outcomes deterministically. Unsupported discard, write-zeroes, zoned operations, FUA, preflush, or unknown flags SHALL fail explicitly unless the complete path advertises and establishes the requested semantics.

#### Scenario: An ext4 read, write, or flush arrives

- **WHEN** the request is aligned, within fixed geometry, uses only advertised flags, and required authority is current
- **THEN** it completes through the normalized portable service path with exact range and persistence evidence

#### Scenario: A range overflows virtual geometry

- **WHEN** sector conversion, byte-count conversion, or checked end arithmetic overflows or exceeds the published capacity
- **THEN** the adapter rejects the request before buffer allocation or protected mutation

#### Scenario: Unsupported intent arrives

- **WHEN** a request carries an operation or intent unsupported by the complete path
- **THEN** the adapter returns an explicit unsupported result and never translates it into a weaker operation or success

#### Scenario: A semantic result has no kernel completion mapping

- **WHEN** the semantic terminal result cannot be represented by the selected ublk completion contract
- **THEN** the adapter records the semantic result and explicit mapping refusal and does not report successful completion

#### Scenario: A retained frontend trace is replayed

- **WHEN** a bounded versioned frontend trace is imported
- **THEN** replay revalidates every normalized request and terminal mapping in sequence without executing backing I/O and reports the first divergence

### Requirement: Live ext4 acceptance evidence is bounded and scope-accurate
<!-- dwv:req req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate -->
<!-- dwv:requires req.evidence-boundaries.evidence-scope-is-explicit -->

A declared supported Linux acceptance profile SHALL run a reproducible bounded workflow that creates the disposable fixture, publishes a real DiskWeave-owned ublk endpoint, formats and mounts ext4, performs bounded create/overwrite/rename/fsync/read/delete operations, verifies exact content, unmounts and cleanly stops, restarts against the same fixture, remounts and verifies retained content, inspects parity/recovery/integrity disposition, and mounts the ordinary data member read-only after final shutdown. Evidence SHALL record the exact environment, configured bounds, source digest, and correlated kernel submission, normalized request, semantic result, and completion without payload bytes, raw pointers, or private host paths.

#### Scenario: The declared supported profile passes

- **WHEN** every required operation and lifecycle transition succeeds in the declared acceptance profile
- **THEN** evidence may claim functional file-backed Linux ublk/ext4 behavior only for that recorded profile and environment

#### Scenario: Live execution is unavailable

- **WHEN** the required architecture, VM, kernel capability, permission, or tool is unavailable
- **THEN** deterministic adapter tests may pass but Linux frontend and ext4 acceptance remain explicitly unmet rather than skipped as success

#### Scenario: A stronger claim is requested

- **WHEN** evidence is used to infer production concurrency, daemon recovery, raw-device durability, FUA, broader filesystems, deployment correctness, online topology mutation, or hardware safety
- **THEN** the report identifies those claims as unsupported

## REMOVED Requirements

### Requirement: Correction evidence covers the composed correctness boundaries
<!-- dwv:req req.linux-ublk-frontend.correction-evidence-covers-the-composed-correctness-boundaries -->

**Reason:** This requirement describes a completed correction and evidence program rather than durable product behavior. Its executable artifacts remain mapped directly to the durable requirements they verify.

**Migration:** Remove every current reference to this ID. Preserve the existing TLA+, Kani, deterministic regression, fence-model, process-death, trace, and Linux evidence by mapping each check directly to the applicable current owners: `req.dirty-integrity-invalidation.dirty-region-coverage-is-complete-and-checked`, `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation`, `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence`, `req.dirty-integrity-invalidation.failures-and-restart-are-conservative`, `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`, `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence`, `req.store-operation-contracts.store-failures-are-conservative-and-testable`, `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence`, `req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout`, `req.recovery-state-semantics.writable-recovery-ownership-is-crash-releasing`, `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics`, `req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit`, `req.evidence-boundaries.evidence-scope-is-explicit`, `req.evidence-boundaries.unknown-and-ambiguous-evidence-fail-closed`, `req.evidence-boundaries.verification-artifacts-are-deterministic-and-bounded`, `req.linux-ublk-frontend.kernel-tags-and-operation-resources-remain-bounded-and-generation-safe`, `req.linux-ublk-frontend.assembly-and-shutdown-preserve-ownership-and-recovery-authority`, and `req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate`.
