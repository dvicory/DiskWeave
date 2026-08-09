# degraded-read-offline-rebuild Specification

## Purpose
This capability serves one known missing single-XOR data member read-only when recovery evidence proves a range reconstructable, and materializes a verified replacement through a durable resumable offline rebuild.
## Requirements
### Requirement: Degraded-read eligibility is explicit and fail-closed
<!-- dwv:req req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed -->

The portable engine SHALL authorize a reconstructed read only when exactly one data slot is a known erasure, the validated topology identifies that stable slot and its coding position, the captured topology and recovery generations remain current, the requested range is parity-clean or replay-proven, every required survivor is readable and not excluded by current integrity evidence, and writes are quiesced. Eligibility SHALL be decided per requested range and SHALL NOT be inferred from algebraic solvability alone.

#### Scenario: One clean known erasure is read

- **WHEN** one single-XOR data slot is known missing and every eligibility condition holds for the requested range
- **THEN** the range is authorized for read-only reconstruction under the captured topology and recovery generation

#### Scenario: Evidence belongs to another generation

- **WHEN** the topology epoch or recovery generation changes after eligibility is captured
- **THEN** reconstruction is refused without reading or mutating a replacement target

#### Scenario: The failure is ambiguous or beyond tolerance

- **WHEN** the missing role is not uniquely identified, parity is also unavailable, or more than one required member has failed
- **THEN** degraded service is unavailable and no survivor is selected or mutated by discovery order

### Requirement: Known-erasure reads reconstruct exact requested bytes
<!-- dwv:req req.degraded-read-offline-rebuild.known-erasure-reads-reconstruct-exact-requested-bytes -->

An authorized single-XOR degraded read SHALL read the parity range and every surviving data operand, apply the documented zero-tail rules, reconstruct only the requested missing-slot bytes, and return exact completion evidence marked degraded. It SHALL perform zero payload writes and SHALL NOT label the result healthy or clean.

#### Scenario: A middle range is reconstructed

- **WHEN** an authorized read targets a nonzero offset within the missing member's protected length
- **THEN** the returned bytes equal the same range from the original member and telemetry identifies the stable slot, range, topology epoch, and degraded result

#### Scenario: A short survivor read occurs

- **WHEN** any required survivor cannot provide its exact logical bytes under the configured geometry
- **THEN** the read fails with incomplete evidence and returns no silently zero-filled in-range data

#### Scenario: A healthy member is shorter than parity coverage

- **WHEN** a surviving data member ends before the requested parity range according to validated topology geometry
- **THEN** its documented tail contributes zeros while a physical short read inside its declared protected length still fails

### Requirement: Dirty or excluded ranges refuse reconstruction
<!-- dwv:req req.degraded-read-offline-rebuild.dirty-or-excluded-ranges-refuse-reconstruction -->

Dirty, indeterminate, uncovered, or stale-checkpoint ranges SHALL NOT be automatically reconstructed. A survivor with current integrity evidence identifying it as invalid SHALL be excluded; if the remaining verified operands are insufficient, the read SHALL fail. A refusal SHALL not clear dirty state, advance integrity coverage, or write any payload.

#### Scenario: The requested range overlaps dirty state

- **WHEN** any portion of the degraded-read range is dirty or indeterminate and no replay proof covers it
- **THEN** the entire requested reconstruction is refused and recovery state remains unchanged

#### Scenario: A surviving operand is independently invalid

- **WHEN** current integrity evidence excludes one of the sources required in addition to the known erasure
- **THEN** the request is beyond the declared single-parity tolerance and fails without best-effort output

### Requirement: Offline rebuild writes only a separate replacement target
<!-- dwv:req req.degraded-read-offline-rebuild.offline-rebuild-writes-only-a-separate-replacement-target -->

The first rebuild implementation SHALL require read-only/quiesced source topology and a distinct empty replacement identity. It SHALL process deterministic bounded ranges in increasing order, reconstruct each range with the same eligibility rules as degraded reads, write only the replacement, verify exact readback and the parity equation, durably flush the replacement range, and only then advance the durable rebuild cursor. It SHALL never overwrite the missing member path, parity, or surviving data.

#### Scenario: One rebuild chunk completes

- **WHEN** reconstruction, replacement write, readback, equation verification, and replacement durability all succeed for the cursor range
- **THEN** recovery state advances the cursor to the first unprocessed byte and records the replacement evidence

#### Scenario: Replacement durability is unknown

- **WHEN** the replacement write succeeds but its durability fence fails or is unknown
- **THEN** the cursor does not advance and restart reprocesses that range conservatively

#### Scenario: Replacement aliases a source

- **WHEN** the replacement identity matches any surviving data or parity source
- **THEN** rebuild is refused before the first payload write

### Requirement: Rebuild resumes and completes only after full verification
<!-- dwv:req req.degraded-read-offline-rebuild.rebuild-resumes-and-completes-only-after-full-verification -->

An interrupted rebuild SHALL resume from its durable cursor only when the rebuild identity, source topology/generation, missing stable slot/coding position, replacement assignment instance, geometry, and replacement identity all match. Completion SHALL require the cursor to cover the full protected length and a final complete replacement digest/equation verification pass. A partial or mismatched rebuild SHALL remain resumable or blocked and SHALL NOT be represented as verified.

#### Scenario: Process loss follows a durable cursor

- **WHEN** rebuild restarts with matching source and replacement evidence after one or more chunks were durably checkpointed
- **THEN** it resumes at the recorded cursor and leaves earlier replacement ranges byte-identical

#### Scenario: Resume evidence does not match

- **WHEN** the replacement identity, assignment instance, topology, geometry, or source generation differs from the checkpoint
- **THEN** resume is refused and the old checkpoint is preserved for reconciliation

#### Scenario: Final verification fails

- **WHEN** any replacement range differs from the expected reconstructed bytes or parity equation during the final pass
- **THEN** the rebuild remains unverified and topology promotion is unavailable

### Requirement: Replacement promotion preserves logical identity
<!-- dwv:req req.degraded-read-offline-rebuild.replacement-promotion-preserves-logical-identity -->

A verified data replacement SHALL retain the old stable slot and coding position, use a new assignment instance and generation for the replacement store, and produce a prepared topology newer than the active topology. Verification SHALL NOT itself publish the topology; durable topology commit and publication remain explicit separate protocol stages.

#### Scenario: Verified replacement is prepared

- **WHEN** offline rebuild and final verification succeed for the full protected length
- **THEN** the prepared replacement topology keeps the logical slot/coding position, changes assignment/store identity, and leaves the old active topology unchanged

#### Scenario: Rebuild is incomplete

- **WHEN** the cursor has not reached the protected length or final verification has no valid receipt
- **THEN** no prepared replacement topology can be produced

### Requirement: Portable and macOS file-backed acceptance remains independent
<!-- dwv:req req.degraded-read-offline-rebuild.portable-and-macos-file-backed-acceptance-remains-independent -->

The degraded read and rebuild semantics SHALL run without Linux frontend types, async-runtime types, SQLite handles, or filesystem metadata inside data members. A macOS regular/sparse-file fixture SHALL prove a rebuilt raw image byte-equals the reference member and remains directly readable after the service stops. An environment-gated macOS fixture SHALL additionally prove that a rebuilt detached APFS disk image attaches independently after every DiskWeave store closes. This change SHALL make no live APFS bridge, Linux request, physical-device durability, or P/Q conformance claim.

#### Scenario: A regular-file member is rebuilt on macOS

- **WHEN** a clean file-backed single-XOR array loses one data file and rebuilds it to a separate regular or sparse file
- **THEN** degraded reads match the original, interruption resumes, the final replacement byte-equals the reference, direct reads work after service shutdown, and the environment-gated APFS image attaches independently read-only

#### Scenario: Linux-only integration is absent

- **WHEN** no ublk or Linux kernel frontend is available
- **THEN** all portable degraded-read, rebuild, interruption, and direct-file acceptance tests remain runnable without a Linux conformance claim

