# Documentation Knowledge Architecture

## Purpose

Provide a durable, model-independent documentation/context subsystem in which canonical OpenSpecs and a small cross-cutting constitution are the only current semantic authority, while human, evidence, contributor, reference, and agent surfaces remain reproducible projections.

## Requirements

### Requirement: Canonical requirements are discovered without a duplicate semantic registry
<!-- dwv:req req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry -->

The documentation tool SHALL discover every current `openspec/specs/*/spec.md` automatically, extract each `Requirement` as a stable semantic unit, and expose an intrinsic requirement identity that is independent of OS/VE/goal numbers, archive paths, line numbers, and generated state. A requirement identity SHALL remain stable when surrounding Markdown is reordered or reformatted, and a changed or missing identity SHALL produce an explicit diagnostic.

#### Scenario: A canonical requirement is added
- **WHEN** a new correctness-sensitive `Requirement` appears in a current canonical spec without a projection or context disposition
- **THEN** extraction succeeds, coverage reports the requirement as newly uncovered, and `docs check` fails closed until an explicit disposition exists.

#### Scenario: Formatting and ordering change
- **WHEN** a canonical spec changes only Markdown formatting or reorders unrelated requirements
- **THEN** requirement identities and dependent accepted prose remain unchanged.

### Requirement: Historical architecture is opt-in only
<!-- dwv:req req.documentation-knowledge-architecture.historical-architecture-is-opt-in-only -->

The normal source graph, freshness checks, projections, and agent context SHALL exclude `docs/handoffs/**`, archived changes, and architecture v0.6/v0.7/v0.8 inputs. A history request SHALL include an explicit opt-in and SHALL label the selected material historical; historical text SHALL never resolve a current requirement conflict.

#### Scenario: A handoff changes
- **WHEN** a v0.6, v0.7, or v0.8 handoff is edited while current canonical sources are unchanged
- **THEN** normal extraction, `docs check`, projections, and context selection remain unchanged.

#### Scenario: An agent requests archaeology
- **WHEN** a query explicitly requests a historical architecture version
- **THEN** the packet contains only the requested bounded historical material, marks it non-authoritative, and does not use it to satisfy current coverage.

### Requirement: Durable documentation configuration is small and state is reconstructible
<!-- dwv:req req.documentation-knowledge-architecture.durable-documentation-configuration-is-small-and-state-is-reconstructible -->

The repository SHALL retain only human-maintained projection/curriculum intent, trusted prompt contracts, schemas, accepted Markdown, canonical sources, and verification evidence under `docs/`. Inventories, plans, coverage reports, generated indexes, task/response queues, diagnostics, scenario fact caches, and other reconstructible execution state SHALL be written beneath `target/dwv-docs/` or an equivalent ignored workspace directory. Deleting that state SHALL not delete accepted prose or canonical semantics.

#### Scenario: A fresh checkout runs the tool
- **WHEN** generated state is absent
- **THEN** extraction, planning, projection generation, and checks reconstruct it from canonical specs, source metadata, small config, fixtures, and tests without a handoff or chat transcript.

#### Scenario: A duplicate registry is edited
- **WHEN** a file attempts to restate canonical requirement text or source ownership facts already available from specs/Cargo metadata
- **THEN** validation rejects it or the file is not part of current semantic extraction.

### Requirement: Projections render useful facts for their intended consumers
<!-- dwv:req req.documentation-knowledge-architecture.projections-render-useful-facts-for-their-intended-consumers -->

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
<!-- dwv:req req.documentation-knowledge-architecture.human-curriculum-is-pedagogical-intent-not-semantic-authority -->

Human projection configuration SHALL describe audience, question, learning outcome, prerequisites, misconceptions, deferred concepts, canonical requirement/scenario/evidence references, and an ordered set of section briefs. Each section brief SHALL provide a proposed Markdown heading, focus, questions it must answer, applicable sources, and explicit non-claims. The briefs guide AI drafting and human review but SHALL NOT copy canonical requirement text, define semantic authority, or become generated page state. Deterministic readiness SHALL reject missing or empty section-brief fields for Human Guide entries. The initial Guide SHALL use those briefs to form a causal journey in which readers can identify what happened, what is durable, what callers may believe, what recovery observes, what remains uncertain, and what DiskWeave refuses to infer.

#### Scenario: A Human Guide chapter is drafted
- **WHEN** an agent writes or revises a Guide chapter
- **THEN** it follows the entry's ordered section briefs, answers each required question from the linked current sources, preserves the stated non-claims, and uses ordinary checked-in Markdown rather than generated prose.

### Requirement: Provenance is claim- and fragment-level
<!-- dwv:req req.documentation-knowledge-architecture.provenance-is-claim-and-fragment-level -->

Each maintained correctness-sensitive prose section SHALL name the exact current requirement identities and, where applicable, scenario or evidence identities that support it. Inspection SHALL report the current typed endpoint paths and lines for a supplied requirement ID. Referential checks SHALL reject unknown identities, but SHALL NOT be represented as proof that prose accurately explains a changed relationship. Diagnostic impact inspection SHALL return bounded current context, exact maintained pages to review, actions, and omitted-reference counts.

#### Scenario: A supported relationship is inspected
- **WHEN** a maintainer supplies a current requirement identity or changed artifact path
- **THEN** the tool reports the current canonical unit and typed Rust, verification, curriculum, and Markdown endpoint locations without asserting that the prose remains correct.


### Requirement: Change-boundary impact is deterministic and bounded
<!-- dwv:req req.documentation-knowledge-architecture.change-boundary-impact-is-deterministic-and-bounded -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->

At a change boundary, deterministic readiness SHALL compare current local and effective semantic/review fingerprints and current typed relationships with an available repository revision baseline through a provider-neutral interface. Local semantic changes, dependency-closure semantic-prerequisite changes, removed or reassigned relationships, and retired IDs SHALL identify the exact requirements requiring individual review. Formatting-only changes and implementation edits with unchanged relationships remain context rather than automatically creating documentation work. The command SHALL report when no baseline is available instead of claiming that relationships are unchanged, and diagnostics SHALL provide exact bounded inspection commands.

#### Scenario: A semantic prerequisite changes

- **WHEN** a target's effective semantic/review fingerprint changes or a source relationship is removed or reassigned
- **THEN** `docs check` reports every exact direct and transitive review-required dependent ID with `semantic-prerequisite-changed` diagnostics while unrelated requirements remain unchanged

#### Scenario: The revision baseline is unavailable

- **WHEN** no supported repository revision baseline can be read
- **THEN** semantic readiness still runs and relationship-delta status is reported as unavailable rather than unchanged

### Requirement: Preservation-first updates are truly narrow
<!-- dwv:req req.documentation-knowledge-architecture.preservation-first-updates-are-truly-narrow -->

Checked-in Markdown SHALL remain the prose baseline. When change-boundary readiness reports a semantic change or removed or reassigned relationship, the maintainer SHALL inspect dependents for the reported old and current requirement IDs, edit only sections whose claims no longer remain accurate, and leave unrelated files byte-identical. Pure relationship additions and implementation edits with unchanged relationships SHALL NOT automatically stale prose. Formatting, provider, model, temperature, and prompt-style metadata changes SHALL NOT stale or rewrite prose. Missing evidence or conflicting authority SHALL block the claim rather than produce guessed prose.

#### Scenario: A formatting-only source change occurs
- **WHEN** a canonical source changes only in representation
- **THEN** its semantic fingerprint and maintained prose remain unchanged.

#### Scenario: One semantic claim changes
- **WHEN** one canonical requirement changes and only one maintained section depends on it
- **THEN** that section is reassessed or edited and unrelated Markdown remains byte-identical.

### Requirement: Agent context is identity-selected and reproducible
<!-- dwv:req req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->

Given an explicit current requirement ID, the context and ownership commands SHALL return its normalized canonical unit, local and effective semantic/review fingerprints, forward relationships, derived backlinks, reviewed outcome and reason, bounded owner-before-dependent reading order, and bounded typed current implementation, verification, curriculum, and Markdown references. They SHALL report omitted counts when bounds truncate results. The commands SHALL NOT concatenate whole specifications, retain a revision-diff or ownership registry, infer prose validity or semantic coherence, or rely on generated human prose when canonical semantics are available.

#### Scenario: A current requirement is selected

- **WHEN** an agent requests context or ownership facts for a current requirement ID
- **THEN** the packet contains the canonical unit, typed relationships, derived reading order, current endpoint locations, explicit bounds, and omissions in stable order

#### Scenario: Context exceeds a bound

- **WHEN** serialized context exceeds the configured byte bound
- **THEN** the command fails closed instead of silently dropping correctness context

### Requirement: Usefulness is sampled separately from correctness
<!-- dwv:req req.documentation-knowledge-architecture.usefulness-and-correctness-are-separate-checks -->

Deterministic checks SHALL validate identity, coverage, stale state, provenance, executable artifacts, graph consistency, safe rendering, no-op/preservation behavior, privacy, and bounds. A major documentation architecture change SHALL separately include observed human and agent exercises with concrete observations recorded in its completion evidence. The repository SHALL NOT maintain a reusable usefulness corpus, scoring schema, result registry, or evaluator command without a documented recurring regression that justifies the maintenance cost. Participant or model self-reports SHALL NOT count as acceptance evidence.

#### Scenario: Structural checks pass without observed exercises
- **WHEN** deterministic documentation and traceability checks pass but the change has not been exercised by its intended human and agent audiences
- **THEN** structural correctness is established while usefulness acceptance remains incomplete, without adding permanent evaluator machinery.

### Requirement: Reconstruction and historical isolation are inspectable
<!-- dwv:req req.documentation-knowledge-architecture.reconstruction-and-historical-isolation-are-inspectable -->

The tool SHALL provide a clean-room check that removes regenerable state, generated indexes, and historical handoffs from a temporary copy, reconstructs current projections from permanent artifacts, and verifies equivalent semantic output. Completion evidence SHALL show that current generation/context does not depend on any prior documentation implementation, handoff, or chat history.

#### Scenario: Regenerable state is deleted
- **WHEN** `target/dwv-docs` and generated projection caches are absent in a temporary copy
- **THEN** the documented extraction, planning, build, check, and reconstruction commands recreate them and produce equivalent current facts and accepted pages.

### Requirement: Canonical semantic relationships are colocated and derived
<!-- dwv:req req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry -->

Current canonical requirements SHALL declare a forward `requires` relationship for every consequential independently owned semantic prerequisite needed to interpret or implement the local requirement, and SHALL declare a forward `refines` relationship when the local requirement is a narrower specialization or adapter realization of another requirement's detailed operational policy. Requirements with no such consequential relationship SHALL declare none. `requires` means the source requirement's own semantics depend on an independently owned semantic fact. `refines` means the target requirement owns the same underlying detailed operational policy and the source defines a narrower specialization or adapter realization of that policy. Composition alone SHALL NOT qualify as refinement.

Semantic reconciliation SHALL determine and author the consequential relationships required by the local semantics. Deterministic tooling SHALL validate the authored representation and SHALL NOT infer a missing semantic relationship from arbitrary English prose.

Relationship markers SHALL be contiguous immediately after the intrinsic identity. A source-target pair SHALL use at most one relation kind, and every target SHALL resolve to a current canonical requirement ID. The tool SHALL derive backlinks, capability aggregation, bounded ownership views, and owner-before-dependent reading order from forward markers without a second registry or manual backlink list. Unknown, non-current, self, duplicate, mixed-kind, misplaced, or cyclic relationships SHALL fail deterministically with stable source and cycle diagnostics.

Universal constitutional, security, and evidence constraints SHALL NOT require repeated edges on every detailed requirement. Lexical or terminology similarity, shared tests or evidence, implementation call graphs, and optional or future work SHALL NOT create semantic edges. Reverse relationships SHALL remain derived.

Each requirement SHALL have a local semantic fingerprint that is a formatting-stable digest of its local normative prose and scenarios, excluding intrinsic and relationship marker comments and non-semantic formatting. After validating that the combined semantic graph is acyclic, the tool SHALL compute each effective semantic/review fingerprint prerequisite-first as a deterministic digest of the local semantic fingerprint, sorted outgoing `(relation kind, target ID)` pairs, and each target's effective semantic/review fingerprint. Reviewed state SHALL store and compare the effective semantic/review fingerprint. Formatting-only changes and changes only to review outcome or reason SHALL preserve semantic fingerprints. A local semantic or outgoing-edge change SHALL make every direct and transitive semantic dependent review-suspect. Every suspect requirement SHALL require an individual semantic review and concrete reason; bulk acceptance SHALL remain forbidden.

#### Scenario: A consequential independently owned prerequisite exists

- **WHEN** semantic reconciliation establishes that a local requirement needs an independently owned semantic fact for interpretation or implementation
- **THEN** the local requirement declares a forward `requires` relationship to that fact before semantic review may approve it

#### Scenario: A narrower specialization or adapter realization exists

- **WHEN** semantic reconciliation establishes that the target owns the same detailed operational policy and the local requirement narrows or realizes that policy
- **THEN** the local requirement declares a forward `refines` relationship and does not also declare `requires` for the same source-target pair

#### Scenario: No consequential semantic relationship exists

- **WHEN** a requirement has no consequential prerequisite, specialization, or adapter realization beyond universal constraints, lexical similarity, shared evidence, implementation calls, or optional future work
- **THEN** it declares no relationship for those non-semantic associations

#### Scenario: Authored relationships are extracted

- **WHEN** required forward markers are present and structurally valid
- **THEN** extraction exposes the forward edges, derived backlinks, bounded owner-before-dependent order, local fingerprints, and effective semantic/review fingerprints without a manual backlink record

#### Scenario: The relationship graph is invalid

- **WHEN** an edge has an unknown or non-current target, is self-referential, duplicates or conflicts with another edge, is misplaced, or participates in a cycle
- **THEN** readiness fails with source path, line, source ID, relation kind, target ID, and the smallest deterministic discovered cycle when applicable

#### Scenario: An owner's local semantics change

- **WHEN** requirements B and C transitively depend on owner A, unrelated requirement D has no path to A, and A's normalized local prose or scenarios change while B, C, and D remain locally unchanged
- **THEN** B and C receive changed effective semantic/review fingerprints and `semantic-prerequisite-changed` diagnostics until individually reviewed while D's effective semantic/review fingerprint remains stable

#### Scenario: Only owner formatting changes

- **WHEN** an owner's semantics and relationships are unchanged but Markdown formatting changes
- **THEN** local and dependent effective semantic/review fingerprints remain unchanged
