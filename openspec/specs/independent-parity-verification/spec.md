# independent-parity-verification Specification

## Purpose

This capability provides a standalone, bounded, read-only equation verifier for explicitly bound single-XOR data and parity payloads without production service state or recovery-state authority.

## Requirements

### Requirement: Standalone equation verification is bounded, read-only, and non-authorizing
<!-- dwv:req req.independent-parity-verification.standalone-equation-verification-is-bounded-read-only-and-non-authorizing -->
<!-- dwv:requires req.xor-reference-model.xor-parity-uses-explicit-protected-geometry -->
<!-- dwv:requires req.parity-verification-repair.exhaustive-verification-is-a-read-only-full-scan -->
<!-- dwv:requires req.parity-verification-repair.reports-are-bounded-and-preserve-authority-boundaries -->
<!-- dwv:requires req.evidence-boundaries.evidence-scope-is-explicit -->
<!-- dwv:requires req.evidence-boundaries.unknown-and-ambiguous-evidence-fail-closed -->
<!-- dwv:requires req.evidence-boundaries.verification-artifacts-are-deterministic-and-bounded -->
<!-- dwv:requires req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission -->

The standalone verifier SHALL accept only an explicitly versioned experimental descriptor for one single-XOR profile, a bounded protected length, a bounded region size and count, and distinct data and parity payload identities. It SHALL reject an unknown descriptor version, unsupported profile or feature, invalid geometry, duplicate or aliased payload identity, an out-of-range request, or a resource bound that would require unbounded reads before reading any payload. The descriptor SHALL provide desired context only; it SHALL contain no recovery generation, topology authority, checksum authority, or writable claim.

For every selected protected region, the verifier SHALL read the exact logical bytes from every configured data payload and the parity payload, apply the declared zero-extension rule only beyond a data payload's protected length, and compare the observed parity with the explicit XOR equation. A complete region SHALL be reported as `matched` or `mismatched`; a short, failed, unavailable, or otherwise uncertain read SHALL be reported as `incomplete` or `unknown` without synthesizing in-range bytes. Reports SHALL name only bounded regions, dispositions, evidence scope, and causal errors; a mismatch SHALL NOT identify a bad shard unless a separate canonical integrity owner supplies that evidence.

The verifier SHALL perform no payload, parity, descriptor, or recovery-state write and SHALL not initialize, migrate, repair, publish, recover, clear dirty state, assert clean/current protection, or authorize a stronger capability. It SHALL run without production service, operator workflow, frontend, SQLite, or independent recovery-state inspection. Human and structured output SHALL carry equivalent command, descriptor version, evidence tier, disposition, non-claims, and deterministic process status. A completed observation, including a mismatch or incomplete region report, SHALL be distinguishable from invalid/unsupported input and an operational inability to produce a bounded report. File-backed or model evidence SHALL remain explicitly weaker than platform durability and SHALL NOT graduate a persistent format.

#### Scenario: Matching payloads are verified independently

- **WHEN** a valid experimental descriptor binds distinct readable data and parity payloads, the selected geometry is within bounds, and every XOR equation matches
- **THEN** the verifier reports each selected region as `matched`, exits with the completed-observation status, performs no write, and makes no current, historical, clean, repair, or durability claim

#### Scenario: A bounded equation mismatch is observed

- **WHEN** all bytes for a selected region are read but the observed parity differs from the explicit XOR result
- **THEN** the verifier reports that bounded region as `mismatched` with the evidence scope and no bad-shard, repair, recovery, or clean-state authorization

#### Scenario: A required read is incomplete

- **WHEN** a data or parity payload returns a short, failed, unavailable, or uncertain read for a selected region
- **THEN** the verifier reports `incomplete` or `unknown`, does not fill missing in-range bytes, and does not reinterpret the result as a mismatch, match, clean state, or repair candidate

#### Scenario: Short data payload uses declared zero extension

- **WHEN** a data payload ends before the configured protected length but the requested region is otherwise valid and the parity payload covers the region
- **THEN** bytes beyond that data payload's declared length contribute logical zeros only, and the result remains bounded to the descriptor's file-backed evidence tier

#### Scenario: Descriptor or resource bounds are invalid

- **WHEN** the descriptor version, profile, geometry, payload identities, requested range, region count, or resource limits are invalid, unsupported, aliased, or over bound
- **THEN** the verifier refuses before payload reads with a deterministic invalid/unsupported result and performs no write

#### Scenario: Human and structured reports agree

- **WHEN** the same completed, refused, or operational result is requested in human and structured modes
- **THEN** both renderings preserve the same region dispositions, evidence tier, non-claims, reason, and deterministic process status

#### Scenario: Production private state is unavailable

- **WHEN** the production service, frontend, recovery database, and independent recovery-state inspection are unavailable but the descriptor and payloads are valid
- **THEN** the verifier still performs the same bounded read-only equation check and does not require or create any private state
