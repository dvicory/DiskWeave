## MODIFIED Requirements

### Requirement: Portable evidence does not imply platform certification
<!-- dwv:req req.normalized-block-semantics.portable-evidence-does-not-imply-platform-certification -->

The normalized contract SHALL identify portable request and lifecycle tests separately from macOS, Linux, concrete-store, power-loss, and hardware tests. Passing portable request or event tests SHALL not be reported as proof of physical flush, FUA, a live frontend, or production behavior.

#### Scenario: Only portable request evidence is available

- **WHEN** the normalized contract is validated without a live frontend or real store
- **THEN** portable contract evidence may pass while each platform and concrete-store acceptance boundary remains visible and unmet

#### Scenario: A platform adapter is introduced

- **WHEN** a Linux or macOS adapter translates flags and completions
- **THEN** it passes the normalized semantic conformance contract and retains separate platform evidence rather than changing the portable API
