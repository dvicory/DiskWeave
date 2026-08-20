## MODIFIED Requirements

### Requirement: Repairs use a separate target and verified readback
<!-- dwv:req req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback -->
<!-- dwv:requires req.dirty-integrity-invalidation.write-recovery-record-precedes-data-parity-write -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->

An authorized repair SHALL write only to a separate replacement target through the owning integrity-invalidation and write-recovery-record protocols. Acceptance SHALL require complete target readback, current checksum verification for the repaired generation, a recomputed parity equation, and any required persistence evidence for the repair claim. The original mismatch report SHALL remain available.

#### Scenario: Separate-target repair succeeds

- **WHEN** a uniquely authorized repair completes, the replacement target is readable, its digest is current and valid, and the recomputed parity equation matches
- **THEN** the system reports a verified repair outcome naming the target, range, digest, and source evidence while leaving source members unchanged

#### Scenario: Target write or verification fails

- **WHEN** a target write, readback, digest check, or parity check fails or is interrupted
- **THEN** the outcome is failed or uncertain, no clean or valid state is asserted, and source members remain unchanged
