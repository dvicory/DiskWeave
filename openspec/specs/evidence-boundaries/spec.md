# evidence-boundaries Specification

## Purpose

Define the product-wide boundary between an observed behavior and the claim it supports. Capability specs own their mechanisms; this spec owns only evidence scope, uncertainty, and claim discipline.

## Requirements

### Requirement: Evidence scope is explicit
<!-- dwv:req req.evidence-boundaries.evidence-scope-is-explicit -->

Every correctness or capability claim SHALL identify the executable input, mechanism or invariant exercised, observed outcome, evidence tier, and the closest unsupported claim boundary. Portable, simulator, file-backed, database, frontend, device, power-loss, and hardware evidence SHALL remain distinguishable.

#### Scenario: Portable evidence is reported
- **WHEN** a portable contract test or deterministic simulator passes
- **THEN** the report may claim the exercised portable behavior and SHALL NOT claim filesystem, frontend, physical durability, or hardware behavior that the test did not exercise

#### Scenario: Platform evidence is unavailable
- **WHEN** a required platform or hardware environment cannot run
- **THEN** the criterion remains gated or unclaimed with the missing evidence visible

### Requirement: Unknown and ambiguous evidence fail closed
<!-- dwv:req req.evidence-boundaries.unknown-and-ambiguous-evidence-fail-closed -->

Unknown, stale, conflicting, incomplete, or ambiguous evidence SHALL remain visible and SHALL NOT authorize clean state, writable assembly, destructive repair, durable completion, or a stronger capability profile.

#### Scenario: Evidence conflicts
- **WHEN** independent observations disagree about identity, content, durability, or recovery state
- **THEN** the result is an explicit ambiguous or reconciliation-required disposition rather than an inferred success

### Requirement: Verification artifacts are deterministic and bounded
<!-- dwv:req req.evidence-boundaries.verification-artifacts-are-deterministic-and-bounded -->

Executable verification artifacts SHALL have bounded inputs and outputs, stable identities/digests, reproducible replay or inspection behavior, and explicit non-claims. A verification tool SHALL not become a runtime semantic dependency merely because it produces evidence.

#### Scenario: The same fixture is replayed
- **WHEN** the same bounded fixture, trace, fault schedule, or model input is replayed
- **THEN** the semantic outcome and evidence digest are reproducible across supported portable interpreters
