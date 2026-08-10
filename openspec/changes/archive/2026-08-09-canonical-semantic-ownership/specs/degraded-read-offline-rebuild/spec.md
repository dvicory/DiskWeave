## MODIFIED Requirements

### Requirement: Degraded-read eligibility is explicit and fail-closed
<!-- dwv:req req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:requires req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->
<!-- dwv:requires req.xor-reference-model.every-single-known-erasure-reconstructs-exact-bytes -->

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

### Requirement: Offline rebuild writes only a separate replacement target
<!-- dwv:req req.degraded-read-offline-rebuild.offline-rebuild-writes-only-a-separate-replacement-target -->
<!-- dwv:requires req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed -->
<!-- dwv:requires req.recovery-state-semantics.offline-rebuild-progress-is-durable-semantic-authority -->

The first rebuild implementation SHALL require read-only or quiesced source topology and a distinct empty replacement identity. It SHALL process deterministic bounded ranges in increasing order, reconstruct each range with the degraded-read eligibility rules, write only the replacement, verify exact readback and the parity equation, durably flush the replacement range, and only then advance the durable rebuild cursor. It SHALL never overwrite the missing member path, parity, or surviving data.

#### Scenario: One rebuild chunk completes

- **WHEN** reconstruction, replacement write, readback, equation verification, and replacement durability all succeed for the cursor range
- **THEN** recovery state advances the cursor to the first unprocessed byte and records the replacement evidence

#### Scenario: Replacement durability is unknown

- **WHEN** the replacement write succeeds but its durability fence fails or is unknown
- **THEN** the cursor does not advance and restart reprocesses that range conservatively

#### Scenario: Replacement aliases a source

- **WHEN** the replacement identity matches any surviving data or parity source
- **THEN** rebuild is refused before the first payload write
