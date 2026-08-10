## MODIFIED Requirements

### Requirement: Automatic repair requires one unique verified solution
<!-- dwv:req req.checksum-scrub-verified-repair.automatic-repair-requires-one-unique-verified-solution -->
<!-- dwv:requires req.checksum-plane.validity-is-generation-bound -->
<!-- dwv:requires req.parity-verification-repair.mismatch-classification-requires-independent-evidence -->

A repair plan SHALL be created only when current generation-bound integrity evidence and the read-only verifier's mismatch classification uniquely identify one damaged data or parity target and the surviving inputs are sufficient to reconstruct the selected range. Parity disagreement without independent target evidence SHALL remain ambiguous.

#### Scenario: Parity is uniquely identified

- **WHEN** all required data evidence is current and valid, parity evidence is invalid, and the parity equation mismatches
- **THEN** the report creates one parity repair candidate for the affected range

#### Scenario: One data target is uniquely identified

- **WHEN** exactly one data target is invalid, every other required target is current and valid, and the equation mismatches
- **THEN** the report creates one data repair candidate for that target and range

#### Scenario: Fault authority exceeds tolerance

- **WHEN** more than one target is invalid or the available evidence cannot identify one target uniquely
- **THEN** the report refuses automatic repair and leaves all source members untouched

### Requirement: Repairs use a separate target and verified readback
<!-- dwv:req req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback -->
<!-- dwv:refines req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation -->
<!-- dwv:refines req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence -->

An authorized repair SHALL write only to a separate replacement target through the owning integrity-invalidation and durability protocols. Acceptance SHALL require complete target readback, current checksum verification for the repaired generation, and a recomputed parity equation. The original mismatch report SHALL remain available.

#### Scenario: Separate-target repair succeeds

- **WHEN** a uniquely authorized repair completes, the replacement target is readable, its digest is current and valid, and the recomputed parity equation matches
- **THEN** the system reports a verified repair outcome naming the target, range, digest, and source evidence while leaving source members unchanged

#### Scenario: Target write or verification fails

- **WHEN** a target write, readback, digest check, or parity check fails or is interrupted
- **THEN** the outcome is failed or uncertain, no clean or valid state is asserted, and source members remain unchanged
