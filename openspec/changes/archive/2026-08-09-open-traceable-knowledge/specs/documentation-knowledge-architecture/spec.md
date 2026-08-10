## ADDED Requirements

### Requirement: Canonical requirement identity is explicit and checked

Every current canonical requirement that participates in implementation, verification, scenario, context, or documentation relationships SHALL carry one colocated stable semantic ID. The system SHALL reject missing, malformed, duplicate, unknown, historical-only, or implicitly changed identities and SHALL support explicit split, merge, removal, and supersession migrations.

#### Scenario: Requirement text is reformatted
- **WHEN** a requirement changes only in representation while its colocated ID remains unchanged
- **THEN** its identity and dependent reviewed relationships remain stable.

#### Scenario: Requirement identity changes accidentally
- **WHEN** a modified requirement loses or changes its stable ID without an explicit migration
- **THEN** deterministic validation fails with the affected capability and requirement.

### Requirement: Semantic fingerprints detect meaningful requirement changes

The system SHALL calculate a deterministic normalized semantic fingerprint for each current requirement and architecture invariant. Formatting-only changes SHALL preserve the fingerprint, while changes to normative modality, states, ordering, durability, failure outcomes, identity, topology, geometry, evidence, compatibility, assumptions, or explicit non-claims SHALL change it.

#### Scenario: Normative failure behavior changes
- **WHEN** a refusal requirement is changed to permit the previously forbidden outcome
- **THEN** its semantic fingerprint changes and linked reviewed relationships become suspect.

### Requirement: Sparse trace links resolve to real implementation and evidence endpoints

Correctness-critical requirements SHALL have policy-appropriate links to meaningful Rust semantic owners and to applicable tests, models, executable scenarios, or evidence. Source markers SHALL be machine parsed, attached to real Rust scopes, resolve only to existing current canonical requirements, and add no runtime behavior. Canonical requirements SHALL NOT be defined in source markers.

#### Scenario: A traced symbol is removed
- **WHEN** a source marker remains but its associated Rust scope no longer exists
- **THEN** deterministic trace validation fails rather than retaining a stale implementation relationship.

#### Scenario: An unknown requirement is linked
- **WHEN** a marker names an ID not exported from current canonical OpenSpecs
- **THEN** extraction and readiness fail with the marker location and unknown ID.

### Requirement: Evidence and scenarios remain artifact-owned

Evidence and executable scenario relationships SHALL be obtained from the evidence artifact, its canonical registry, or a compact relationship sidecar when no source marker is honest. Relationship records SHALL contain identities, resolvable endpoints, scope or non-claims where required, and the reviewed requirement fingerprint; they SHALL NOT copy canonical requirement prose or maintain assurance narratives.

#### Scenario: Executable scenario facts are projected
- **WHEN** a registered scenario executes successfully
- **THEN** its current actions, cut points, states, outcome, and forbidden inferences are linked to the requirements it exercises.

### Requirement: Reviewed relationships become suspect after semantic change

Every durable requirement relationship SHALL record the requirement fingerprint against which it was reviewed. A changed semantic fingerprint SHALL make the relationship `SUSPECT` until an agent records one of `STILL_VALID`, `UPDATED`, `REPLACED_BY`, `NO_LONGER_APPLICABLE`, `CONFLICT`, or `NEEDS_EVIDENCE` with a concise reason and current validation. Editing digests alone, bulk acceptance, deleting coverage, or weakening policy SHALL NOT clear suspect state.

#### Scenario: Formatting changes only
- **WHEN** source formatting changes without changing the semantic fingerprint
- **THEN** reviewed links remain current and checked-in Markdown remains byte-identical.

#### Scenario: Suspect relationship is unresolved
- **WHEN** a semantic change affects a reviewed implementation or documentation relationship
- **THEN** readiness reports the exact suspect link and required resolution command and does not report the change complete.

### Requirement: Open projection tooling is reproducible and non-authoritative

The documentation site SHALL build from ordinary MyST Markdown using pinned Sphinx, Sphinx-Needs, sphinx-codelinks, and an adequate maintained Rust documentation integration unless a checked vertical-slice record demonstrates a concrete incompatibility. Generated needs, extracted symbols, trace graphs, indexes, context packets, and rendered output SHALL be ignored build artifacts and SHALL NOT become semantic authority. No proprietary or hosted tool SHALL be required.

#### Scenario: Offline deterministic build
- **WHEN** pinned dependencies are already available and model/provider credentials and network access are absent
- **THEN** extraction, trace validation, Sphinx build, link validation, and readiness checks complete deterministically.

### Requirement: Human documentation is ordinary traceable Markdown

AI agents SHALL maintain readable checked-in Markdown with exact current requirement and applicable scenario/evidence IDs at section granularity. At the change boundary, deterministic readiness SHALL compare current canonical fingerprints and requirement relationships with an available repository revision baseline. It SHALL require review for semantic changes and removed or reassigned relationships, return unchanged implementation relationships as context only, and report an unavailable baseline explicitly. Diagnostics SHALL provide bounded current dependents and pages to review or recover a lost old ID without a persistent relationship registry. The agent SHALL update only claims that no longer remain accurate. Model, provider, or style metadata changes alone SHALL NOT rewrite prose.

#### Scenario: One relationship changes narrowly
- **WHEN** one requirement relationship is removed or reassigned within the selected change
- **THEN** the completion check identifies its old and current IDs, a diagnostic identifies dependent pages, and the agent preserves unrelated Markdown byte-for-byte.

### Requirement: Change-boundary impact is deterministic and bounded

At a change boundary, deterministic readiness SHALL compare current canonical fingerprints and requirement relationships with an available repository revision baseline through a provider-neutral interface. It SHALL require review for semantic changes and removed or reassigned relationships, return pure additions and unchanged implementation relationships as context only, and report an unavailable baseline explicitly. Diagnostics SHALL recover a lost old ID without a persistent relationship registry. Provider-specific commands SHALL remain outside the agent skill.

#### Scenario: A relationship is removed
- **WHEN** a requirement relationship present at the selected baseline is absent from the current tree
- **THEN** the completion check reports its old ID and an exact bounded diagnostic action.

### Requirement: Human projections render real linked facts

The Guide SHALL teach through concrete causal experiences before abstractions. Each Human Guide curriculum entry SHALL contain ordered section briefs with a proposed Markdown heading, focus, must-answer questions, exact current sources, and non-claims. The skill SHALL direct an agent to draft and review those briefs in order, while deterministic readiness rejects missing or empty fields. The briefs guide ordinary checked-in Markdown and SHALL NOT become canonical prose, generated page state, or a replacement for current requirement/evidence inspection. Scenario, assurance, contributor, and architecture views SHALL render actual linked facts rather than meta-descriptions or copied registries.

#### Scenario: A Guide chapter is revised
- **WHEN** an agent updates a Human Guide chapter
- **THEN** it follows the section briefs as a causal journey, answers each required question from linked sources, and preserves explicit non-claims.

### Requirement: Agent context is identity and graph selected

Given an explicit current requirement ID, implementation context SHALL contain the normalized canonical unit and fingerprint plus bounded, typed, stably ordered current Rust, verification, curriculum, and Markdown endpoint locations. It SHALL report omissions caused by bounds and fail closed when the serialized packet exceeds its byte limit. Task intent remains in the caller's prompt; the agent reads returned endpoint sections as needed. The context command SHALL not retain revision-diff state, infer prose validity, or concatenate whole specifications.

#### Scenario: A requirement is selected
- **WHEN** an agent requests context for a current requirement
- **THEN** the packet contains the exact canonical unit, typed current endpoint locations, explicit bounds, and omitted-reference counts.

### Requirement: Agents can inspect and complete knowledge impact without historical instructions

A discoverable repository-local skill SHALL teach the maintenance procedure. Deterministic commands SHALL expose extraction, check, trace, why, affected, context, documentation doctor/build, and one readiness operation that reports pass/fail by gate plus exact next actions. A fresh agent SHALL be able to resolve a bounded semantic change without consulting handoffs or chat history.

#### Scenario: Change is not ready
- **WHEN** a change has unresolved evidence and documentation relationships
- **THEN** the readiness command reports those gates as failed and gives resolvable identities and commands for the next actions.

### Requirement: Usefulness is sampled without permanent evaluator machinery

Acceptance of the open-traceable-knowledge change SHALL include one observed human reading exercise and one real fresh-agent implementation exercise using the maintained documentation and graph-selected context. A human reviewer SHALL record the concrete observations, omissions, and acceptance conclusion in the change's completion evidence. The repository SHALL NOT retain a reusable usefulness corpus, scoring schema, result registry, or evaluator command unless a later documented regression demonstrates that recurring machinery is worth its maintenance cost. Participant or model self-reports SHALL NOT count as acceptance evidence.

#### Scenario: Structural checks pass
- **WHEN** deterministic documentation and traceability checks pass without observed human and agent exercises
- **THEN** structural correctness is established but the open-traceable-knowledge usefulness acceptance remains incomplete.

#### Scenario: Acceptance exercises complete
- **WHEN** a reader uses the Guide and a fresh agent completes a real task from graph-selected context under human observation
- **THEN** the reviewer records specific successes, failures, source hunting, and residual gaps in the open-traceable-knowledge completion evidence without creating a reusable evaluation subsystem.

### Requirement: Generated state is reconstructible and historical inputs are isolated

Current extraction, traceability, documentation, context, and readiness SHALL reconstruct from canonical OpenSpecs, Rust source, ordinary Markdown, curriculum intent, evidence metadata, reviewed-link state, pinned tooling, and tests. Historical architecture, handoffs, generated state, model access, provider access, and closed-source services SHALL not be required or enter normal current-semantic selection.

#### Scenario: Clean-room reconstruction
- **WHEN** generated knowledge, Sphinx interchange, caches, and rendered output are absent from a clean temporary copy
- **THEN** the documented offline commands reproduce equivalent current objects, trace checks, and documentation output from durable non-historical inputs.

## MODIFIED Requirements

### Requirement: Durable documentation configuration is small and state is reconstructible

The repository SHALL retain only ordinary human documentation, small pedagogical curriculum intent, compact reviewed-link state, canonical sources, evidence metadata, open tooling configuration/locks, and agent procedure under `docs/` and its obvious adjacent tooling locations. Inventories, plans, coverage reports, generated indexes, task/response queues, diagnostics, scenario caches, trace interchange, extracted API data, and rendered output SHALL remain ignored and reconstructible. Deleting generated state SHALL not delete accepted prose, reviewed relationships, or canonical semantics.

#### Scenario: A fresh checkout runs the tool
- **WHEN** generated state is absent
- **THEN** knowledge extraction, trace validation, documentation build, and readiness reconstruct required intermediates without a handoff or chat transcript.

### Requirement: Reconstruction and historical isolation are inspectable

The system SHALL provide a clean-room check that removes regenerable state and generated output from a temporary copy, reconstructs current knowledge and documentation from permanent artifacts, and verifies equivalent semantic output. Completion evidence SHALL show that current generation and context do not depend on historical architecture, handoffs, archived work records, or chat history.

#### Scenario: Regenerable state is deleted
- **WHEN** generated knowledge and documentation output are absent in a temporary copy
- **THEN** the documented extraction, check, build, and reconstruction commands recreate equivalent current facts and accepted pages.
