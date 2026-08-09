## Purpose

Provide a durable, model-independent documentation/context subsystem in which canonical OpenSpecs and a small cross-cutting constitution are the only current semantic authority, while human, evidence, contributor, reference, and agent surfaces remain reproducible projections.

## ADDED Requirements

### Requirement: Canonical requirements are discovered without a duplicate semantic registry

The documentation tool SHALL discover every current `openspec/specs/*/spec.md` automatically, extract each `Requirement` as a stable semantic unit, and expose an intrinsic requirement identity that is independent of OS/VE/goal numbers, archive paths, line numbers, and generated state. A requirement identity SHALL remain stable when surrounding Markdown is reordered or reformatted, and a changed or missing identity SHALL produce an explicit diagnostic.

#### Scenario: A canonical requirement is added
- **WHEN** a new correctness-sensitive `Requirement` appears in a current canonical spec without a projection or context disposition
- **THEN** extraction succeeds, coverage reports the requirement as newly uncovered, and `docs check` fails closed until an explicit disposition exists.

#### Scenario: Formatting and ordering change
- **WHEN** a canonical spec changes only Markdown formatting or reorders unrelated requirements
- **THEN** requirement identities and dependent accepted prose remain unchanged.

### Requirement: Historical architecture is opt-in only

The normal source graph, freshness checks, projections, and agent context SHALL exclude `docs/handoffs/**`, archived changes, and architecture v0.6/v0.7/v0.8 inputs. A history request SHALL include an explicit opt-in and SHALL label the selected material historical; historical text SHALL never resolve a current requirement conflict.

#### Scenario: A handoff changes
- **WHEN** a v0.6, v0.7, or v0.8 handoff is edited while current canonical sources are unchanged
- **THEN** normal extraction, `docs check`, projections, and context selection remain unchanged.

#### Scenario: An agent requests archaeology
- **WHEN** a query explicitly requests a historical architecture version
- **THEN** the packet contains only the requested bounded historical material, marks it non-authoritative, and does not use it to satisfy current coverage.

### Requirement: Durable documentation configuration is small and state is reconstructible

The repository SHALL retain only human-maintained projection/curriculum intent, trusted prompt contracts, schemas, accepted Markdown, canonical sources, and verification evidence under `docs/`. Inventories, plans, coverage reports, generated indexes, task/response queues, diagnostics, scenario fact caches, and other reconstructible execution state SHALL be written beneath `target/dwv-docs/` or an equivalent ignored workspace directory. Deleting that state SHALL not delete accepted prose or canonical semantics.

#### Scenario: A fresh checkout runs the tool
- **WHEN** generated state is absent
- **THEN** extraction, planning, projection generation, and checks reconstruct it from canonical specs, source metadata, small config, fixtures, and tests without a handoff or chat transcript.

#### Scenario: A duplicate registry is edited
- **WHEN** a file attempts to restate canonical requirement text or source ownership facts already available from specs/Cargo metadata
- **THEN** validation rejects it or the file is not part of current semantic extraction.

### Requirement: Projections render useful facts for their intended consumers

The tool SHALL produce separate projections whose observable content is better suited to the declared audience than a raw canonical-spec dump: a causal Human Guide, a complete architecture reference, executable Scenario Book entries, claim/evidence Assurance Atlas entries, deterministic Cargo/source Contributor Map entries, and task-specific agent context. Scenario and assurance projections SHALL render actual structured facts and relationships, not descriptions of what a projection would contain.

#### Scenario: A scenario is projected
- **WHEN** a registered executable simulator, normalized-trace, model, or equivalent fixture passes
- **THEN** the Scenario Book renders its initial state, actions/transitions, durability/fault points, observations, terminal disposition, and explicit forbidden inferences.

#### Scenario: An assurance claim is projected
- **WHEN** a claim has assumptions, mechanisms, properties, evidence, strength, non-claims, or gaps
- **THEN** the Assurance Atlas renders those relationships and links exact canonical requirements and evidence artifacts.

#### Scenario: Contributor ownership is projected
- **WHEN** Cargo metadata and Rust entry points are available
- **THEN** the Contributor Map renders current packages, dependency direction, ownership, canonical requirement bindings, scenarios/evidence, and prohibited edges without duplicating package facts in hand-maintained JSON.

### Requirement: Human curriculum is pedagogical intent, not semantic authority

Human projection configuration SHALL describe audience, question, learning outcome, prerequisites, misconception, deferred concepts, required teaching devices, and canonical requirement/scenario/evidence references. It SHALL not copy canonical requirement text or derived package facts. The initial Guide SHALL include a causal interrupted-write chapter in which readers can identify what happened, what is durable, what callers may believe, what recovery observes, what remains uncertain, and what DiskWeave refuses to infer.

#### Scenario: The interrupted-write chapter is rendered
- **WHEN** the executable cut-point scenario and referenced canonical requirements are available
- **THEN** the chapter presents the concrete timeline before introducing dirty intent, generation, fence, checkpoint, or recovery-state abstractions and exposes compact inspectable provenance for each semantic claim.

### Requirement: Provenance is claim- and fragment-level

Every generated or accepted explanatory claim that asserts a current semantic fact SHALL resolve to one or more exact canonical requirement identities and, where applicable, scenario and evidence artifact identities. Rendered pages SHALL expose this provenance without replacing the narrative with metadata. A source digest alone SHALL not satisfy provenance.

#### Scenario: A claim lacks support
- **WHEN** a projection or response introduces a correctness-sensitive claim without canonical requirement support
- **THEN** application is rejected or the block is marked blocked/needs-source and the unsupported claim is not accepted.

#### Scenario: A supported claim is inspected
- **WHEN** a maintainer asks why a claim appears
- **THEN** the tool can report the exact requirement, scenario, evidence, and source change that selected it.

### Requirement: Preservation-first updates are truly narrow

`KEEP` SHALL preserve accepted prose byte-for-byte. `PATCH` SHALL require stable fragment identity and preimage evidence, change only affected fragments, and preserve all unrelated accepted bytes. `REPLACE` SHALL require an explicit semantic or pedagogical reason. Provider, model, temperature, and prompt-style metadata changes SHALL not stale or rewrite accepted prose by themselves. Missing evidence or conflicting authority SHALL produce `NEEDS_SOURCE`/`BLOCKED` rather than guessed prose.

#### Scenario: A formatting-only source change occurs
- **WHEN** a canonical source changes only in representation
- **THEN** no task is queued, no accepted prose changes, and the check remains clean.

#### Scenario: One semantic claim changes
- **WHEN** one canonical requirement changes and only one accepted fragment depends on it
- **THEN** exactly that fragment is reassessed or patched, its preimage is checked, and unrelated fragments remain byte-identical.

### Requirement: Agent context is graph-selected and reproducible

A task-specific context packet SHALL contain task intent, selected canonical requirements, cross-cutting invariants, implementation ownership/entry points, relevant scenarios, failure/forbidden outcomes, evidence, selection reasons, omissions, and explicit bounds. Selection SHALL use stable graph relationships and deterministic ordering; it SHALL not concatenate whole specs to fill a byte limit or rely on Human Guide prose when canonical semantics are available.

#### Scenario: A real implementation task is selected
- **WHEN** a task names a capability, symbol, scenario, or failure boundary
- **THEN** context includes the exact relevant requirement closure and ownership/evidence while omitting unrelated canonical documents, and repeated runs produce the same packet digest.

#### Scenario: Context is bounded
- **WHEN** the selected closure exceeds configured limits
- **THEN** the tool reports explicit omitted identities/reasons and fails closed if a correctness-critical requirement would be omitted.

### Requirement: Usefulness and correctness are separate checks

Deterministic checks SHALL validate identity, coverage, stale state, provenance, executable artifacts, graph consistency, safe rendering, no-op/preservation behavior, privacy, and bounds. A small stable evaluation corpus SHALL record qualitative human and agent questions and expected evidence; qualitative results SHALL remain explicitly non-deterministic review rather than being misrepresented as proof.

#### Scenario: Structural checks pass but a projection is unhelpful
- **WHEN** a qualitative evaluator cannot answer a corpus question from the intended projection
- **THEN** the evaluation records a usefulness failure without weakening deterministic correctness checks or silently marking the projection complete.

### Requirement: Reconstruction and historical isolation are inspectable

The tool SHALL provide a clean-room check that removes regenerable state, generated indexes, and historical handoffs from a temporary copy, reconstructs current projections from permanent artifacts, and verifies equivalent semantic output. Completion evidence SHALL show that current generation/context does not depend on goal-v5, prior handoffs, or chat history.

#### Scenario: Regenerable state is deleted
- **WHEN** `target/dwv-docs` and generated projection caches are absent in a temporary copy
- **THEN** the documented extraction/plan/build/check path recreates them and produces equivalent current facts and accepted pages.
