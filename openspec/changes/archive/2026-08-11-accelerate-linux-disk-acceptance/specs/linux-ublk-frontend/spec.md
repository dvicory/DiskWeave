## MODIFIED Requirements

### Requirement: Live ext4 acceptance evidence is bounded and scope-accurate
<!-- dwv:req req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate -->
<!-- dwv:requires req.evidence-boundaries.evidence-scope-is-explicit -->
<!-- dwv:requires req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow -->

A Linux acceptance profile already declared supported by current canonical Linux requirements SHALL run a reproducible bounded workflow. The workflow SHALL create a fresh disposable fixture, publish a real DiskWeave-owned ublk endpoint, format and mount ext4, perform bounded create/overwrite/rename/fsync/read/delete operations, verify exact content, unmount and cleanly stop, restart against the same fixture, remount and verify retained content, inspect parity/recovery/integrity disposition, and mount the ordinary data member read-only after final shutdown.

The live platform phase from prerequisite probing through final direct-member inspection SHALL complete within 30 elapsed seconds. After a reusable runner completes cold VM provisioning and its first release build, each warm invocation from source snapshot creation through copied evidence SHALL complete within 30 elapsed seconds. Exceeding either applicable bound SHALL fail acceptance rather than skip or weaken required work. Cold preparation time SHALL remain explicit and SHALL NOT be reported as a warm invocation.

Deterministic adapter checks, undersized-fixture refusal, unsupported-topology refusal, missing-recovery-authority refusal, and partial-fence refusal SHALL remain separate portable evidence and SHALL NOT be rerun as prerequisites inside the timed live platform phase. Live ownership conflict, wrong-owner cleanup refusal, stale-readiness refusal, kernel discard refusal, and owner-process-death behavior SHALL remain inside the timed live platform phase.

Evidence SHALL record the exact environment, configured bounds, source digest, live elapsed time, and correlated kernel submission, normalized request, semantic result, and completion without payload bytes, raw pointers, or private host paths. A successful run in any environment SHALL NOT create or broaden a canonically supported profile.

#### Scenario: A canonically supported profile passes

- **WHEN** every required operation and lifecycle transition succeeds for a profile already supported by current canonical Linux requirements within the applicable elapsed-time bounds
- **THEN** evidence may claim functional file-backed Linux ublk/ext4 behavior only for that canonical profile in the recorded environment and does not establish another supported profile

#### Scenario: A prepared runner completes within the bound

- **WHEN** the exact source snapshot runs after successful cold runner preparation
- **THEN** the live platform phase and the complete warm invocation each finish within 30 elapsed seconds while retaining every required live operation and evidence field

#### Scenario: An elapsed-time bound is exceeded

- **WHEN** either the live platform phase or a prepared warm invocation exceeds 30 elapsed seconds
- **THEN** acceptance fails with the observed elapsed time and does not omit operations, reuse prior evidence, or report the run as successful

#### Scenario: A runner requires cold preparation

- **WHEN** the named reusable runner has not completed VM provisioning and its first release build
- **THEN** the workflow identifies the invocation as cold preparation, applies the 30-second bound to its live platform phase, and does not claim the complete invocation met the warm-runner bound

#### Scenario: Portable checks cover non-platform behavior

- **WHEN** undersized-fixture, unsupported-topology, missing-recovery-authority, adapter, or partial-fence behavior can be exercised deterministically without a live ublk endpoint or mounted ext4 filesystem
- **THEN** ordinary portable evidence covers that behavior separately and the timed live phase remains limited to behavior requiring the recorded Linux environment

#### Scenario: Live execution is unavailable

- **WHEN** the required architecture, VM, kernel capability, permission, or tool is unavailable
- **THEN** deterministic adapter tests may pass but Linux frontend and ext4 acceptance remain explicitly unmet rather than skipped as success

#### Scenario: The canonical profile passes in another environment

- **WHEN** the same canonically supported profile completes successfully in an additional environment
- **THEN** evidence records that environment but the successful run does not create or broaden canonical Linux support

#### Scenario: A stronger claim is requested

- **WHEN** evidence is used to infer production concurrency, daemon recovery, raw-device durability, FUA, broader filesystems, deployment correctness, online topology mutation, or hardware safety
- **THEN** the report identifies those claims as unsupported
