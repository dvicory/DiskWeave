# independent-recovery-inspection Specification

## Purpose
This capability lets an operator inspect one recovery-state artifact without the production service, mutation, or any stronger recovery or format claim.
## Requirements
### Requirement: Independent recovery-state inspection is bounded and non-authorizing
<!-- dwv:req req.independent-recovery-inspection.independent-recovery-state-inspection-is-bounded-and-non-authorizing -->
<!-- dwv:requires req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->
<!-- dwv:requires req.recovery-state-semantics.semantic-schema-migrations-and-exports-are-versioned-independently-of-sqlite -->
<!-- dwv:requires req.architecture-contract.portable-semantics-are-independent-of-implementation-mechanisms -->
<!-- dwv:requires req.architecture-contract.recovery-and-repair-never-promote-algebraic-possibility-to-authority -->
<!-- dwv:requires req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission -->
<!-- dwv:requires req.architecture-contract.operator-result-projections-preserve-consequential-meaning -->

DiskWeave SHALL provide an independently invocable command that inspects exactly one supplied recovery-state artifact through the portable observational inspection boundary. The command SHALL require no production service, operator workflow, frontend, writable recovery adapter, production-runtime state, or private production-daemon representation. It SHALL classify the artifact as `absent`, `supported`, `corrupt-or-unreadable`, `unsupported`, `migration-required`, or `reconciliation-required` without creating the artifact, acquiring writable ownership, initializing, migrating, reconciling, repairing, or changing recovery or payload state.

The command SHALL emit either a human result or a `dwv.recovery-inspection.v1` structured result. Each representation SHALL include the classification, known storage or semantic format layer and version facts, `experimental` format-claim status, the bounded portable manifest when supported, explicit non-authorization statement, and next action. A produced inspection classification is a successful observation regardless of artifact health; usage or operational failure before classification remains distinct and non-successful. Inputs, parsing, manifest records, detail, and output SHALL remain within explicit bounds, and excess or malformed content SHALL fail before unbounded allocation or mutation.

Every produced result SHALL state that inspection authorizes no writable format interpretation, publication, payload or recovery mutation, repair, migration, parity or integrity claim, lineage or custody claim, historical recovery claim, or stable-format claim. The command SHALL NOT expose storage-engine layout or private runtime handles.

#### Scenario: Supported recovery state is inspected independently
- **WHEN** the supplied artifact contains a supported current semantic manifest within all bounds
- **THEN** the command reports `supported`, emits the bounded portable manifest and schema facts, exits as a successful observation, and leaves the artifact and adjacent recovery state unchanged

#### Scenario: Recovery artifact is absent
- **WHEN** the supplied artifact does not exist
- **THEN** the command reports `absent`, exits as a successful observation, and creates no artifact, lock, journal, initialization state, or generation-zero claim

#### Scenario: Conservative artifact disposition is observed
- **WHEN** inspection establishes corrupt-or-unreadable, unsupported, migration-required, or reconciliation-required state
- **THEN** the command preserves that exact classification and any safely known layer/version facts, exits as a successful observation, and neither attempts nor authorizes a stronger action

#### Scenario: Input exceeds an inspection bound
- **WHEN** the artifact or its declared structures exceed an active parser, record, allocation, or output bound
- **THEN** the command returns a bounded explicit refusal or conservative inspection classification before unbounded work and leaves the artifact unchanged

#### Scenario: Human and structured results are requested
- **WHEN** the same artifact is inspected in human and structured modes
- **THEN** human and structured representations preserve the classification, format facts, `experimental` format-claim status, bounded portable manifest, non-authorization statement, and next action required by this capability, while process status is mapped deterministically from whether a classification was produced

#### Scenario: Production components are unavailable
- **WHEN** the independent inspection command is built and run without production service, operator, or frontend components
- **THEN** it can still inspect the supplied documented recovery artifact through the portable read-only adapter and produce the required result

