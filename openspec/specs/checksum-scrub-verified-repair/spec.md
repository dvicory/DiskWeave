# checksum-scrub-verified-repair Specification

## Purpose
This capability turns independent checksum and parity evidence into a conservative scrub and repair workflow. It must identify only uniquely evidenced faults, preserve source media when authority is insufficient, and accept a repair only after separate-target readback verifies both integrity and the parity equation.
## Requirements
### Requirement: Scrub classifies every selected extent conservatively

A scrub SHALL read every selected data and parity extent and report a bounded disposition for verified-good, known-bad, missing, stale or unknown, unreadable, ambiguous, and conflicting evidence. Scrub SHALL be read-only and SHALL NOT infer a bad target from parity disagreement alone.

#### Scenario: Matching extents are scrubbed

- **WHEN** all selected payloads are readable, their current checksum evidence matches, and each parity equation holds
- **THEN** the report marks the extents verified-good and performs no payload writes

#### Scenario: Evidence is absent or stale

- **WHEN** a selected extent lacks current valid checksum evidence or its evidence names another content generation
- **THEN** the report marks the extent stale or unknown and creates no automatic repair authorization

#### Scenario: Read or evidence disagreement occurs

- **WHEN** a selected extent cannot be read, multiple targets are invalid, or checksum and parity evidence conflict
- **THEN** the report preserves the conflict or unreadable disposition and creates no automatic repair authorization

### Requirement: Automatic repair requires one unique verified solution

A repair plan SHALL be created only when current independent evidence uniquely identifies one damaged data or parity target and the surviving inputs are sufficient to reconstruct the selected range. Parity disagreement without independent target evidence SHALL remain ambiguous.

#### Scenario: Parity is uniquely identified

- **WHEN** all required data evidence is current and valid, parity evidence is invalid, and the parity equation mismatches
- **THEN** the report creates one parity repair candidate for the affected range

#### Scenario: One data target is uniquely identified

- **WHEN** exactly one data target is invalid, every other required target is current and valid, and the equation mismatches
- **THEN** the report creates one data repair candidate for that target and range

#### Scenario: Fault authority exceeds tolerance

- **WHEN** more than one target is invalid or the available evidence cannot identify one target uniquely
- **THEN** the report refuses automatic repair and leaves all source members untouched

### Requirement: Repair plans are identity- and generation-bound

A repair plan SHALL name the source identities, replacement target identity, protected range, topology or recovery generation, checksum profile/set generation, and evidence used to authorize it. The system SHALL reject a plan when any named source, target, range, or generation no longer matches the observed state.

#### Scenario: A source changes after planning

- **WHEN** a source is replaced, its content generation changes, or its recovery/topology generation advances before mutation
- **THEN** the plan is rejected as stale and no source or target bytes are written

#### Scenario: The target aliases a source

- **WHEN** the proposed repair target is the same identity as any source member
- **THEN** the plan is rejected before mutation

### Requirement: Repairs use a separate target and verified readback

An authorized repair SHALL write only to a separate replacement target through the ordinary integrity invalidation and durability protocol. Acceptance SHALL require complete target readback, current checksum verification for the repaired generation, and a recomputed parity equation. The original mismatch report SHALL remain available.

#### Scenario: Separate-target repair succeeds

- **WHEN** a uniquely authorized repair completes, the replacement target is readable, its digest is current and valid, and the recomputed parity equation matches
- **THEN** the system reports a verified repair outcome naming the target, range, digest, and source evidence while leaving source members unchanged

#### Scenario: Target write or verification fails

- **WHEN** a target write, readback, digest check, or parity check fails or is interrupted
- **THEN** the outcome is failed or uncertain, no clean or valid state is asserted, and source members remain unchanged

### Requirement: Checksum-set migration preserves an interpretable baseline

A checksum-set migration SHALL build and validate a parallel set before switching the active set. An interrupted, unsupported, or incomplete set SHALL NOT relabel existing evidence, create a coverage gap for the active set, or authorize repair.

#### Scenario: Migration is interrupted

- **WHEN** execution stops before the new set is complete and durably selected
- **THEN** the prior active set remains interpretable and the incomplete set is unavailable for repair authority

