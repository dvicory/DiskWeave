## ADDED Requirements

### Requirement: Active roadmap is explicit and non-authoritative
<!-- dwv:req req.documentation-knowledge-architecture.active-roadmap-is-explicit-and-non-authoritative -->
<!-- dwv:requires req.documentation-knowledge-architecture.historical-architecture-is-opt-in-only -->

The documentation workflow SHALL deterministically discover exactly one active architecture roadmap from one explicit repository marker. The selected roadmap SHALL remain excluded from ordinary current semantic discovery, projections, freshness checks, and requirement context. A deliberate milestone-planning request MAY select the marked roadmap, but the resulting material SHALL be identified as non-authoritative design direction and SHALL NOT satisfy current requirement coverage. A detected disagreement between the roadmap and current canonical requirements SHALL be reported for explicit reconciliation and SHALL NOT be resolved by silently preferring either source.

#### Scenario: Exactly one active roadmap is marked
- **WHEN** documentation readiness scans the maintained repository
- **THEN** it resolves one deterministic active-roadmap path and continues without treating that document as current semantic authority.

#### Scenario: The active-roadmap marker is missing or ambiguous
- **WHEN** zero or multiple active-roadmap markers are present
- **THEN** documentation readiness fails with the marker count and every marked path rather than choosing a roadmap implicitly.

#### Scenario: Milestone planning deliberately requests roadmap context
- **WHEN** milestone planning selects the marked active roadmap after inspecting current canonical requirements
- **THEN** the roadmap is available as non-authoritative direction and remains outside ordinary requirement coverage and context.

#### Scenario: Current requirements and roadmap direction disagree
- **WHEN** planning or semantic reconciliation detects a material disagreement
- **THEN** it records the disagreement for an explicit specification or roadmap decision without silently rewriting current semantics.

## MODIFIED Requirements

### Requirement: Canonical requirements are discovered without a duplicate semantic registry
<!-- dwv:req req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry -->

The documentation tool SHALL discover every current `openspec/specs/*/spec.md` automatically, extract each `Requirement` as a stable semantic unit, and expose an intrinsic requirement identity that is independent of roadmap nodes, verification identifiers, milestone numbers, historical planning paths, archive paths, line numbers, and generated state. A requirement identity SHALL remain stable when surrounding Markdown is reordered or reformatted, and a changed or missing identity SHALL produce an explicit diagnostic.

#### Scenario: A canonical requirement is added
- **WHEN** a new correctness-sensitive `Requirement` appears in a current canonical spec without a projection or context disposition
- **THEN** extraction succeeds, coverage reports the requirement as newly uncovered, and `docs check` fails closed until an explicit disposition exists.

#### Scenario: Formatting and ordering change
- **WHEN** a canonical spec changes only Markdown formatting or reorders unrelated requirements
- **THEN** requirement identities and dependent accepted prose remain unchanged.

#### Scenario: A milestone contains requirement-like prose
- **WHEN** a retained milestone contains headings, normative language, or current requirement identifiers
- **THEN** canonical extraction ignores the milestone and derives current requirements only from `openspec/specs/*/spec.md`.

### Requirement: Historical architecture is opt-in only
<!-- dwv:req req.documentation-knowledge-architecture.historical-architecture-is-opt-in-only -->

The normal source graph, freshness checks, projections, and agent context SHALL exclude `docs/handoffs/**`, `docs/milestones/**`, archived changes, and prior architecture revisions. The marked active roadmap SHALL remain excluded from those ordinary current-semantic surfaces and SHALL be selected only through the deliberate non-authoritative planning contract. A history request SHALL include an explicit opt-in and SHALL label the selected material historical; historical or milestone text SHALL never resolve a current requirement conflict.

#### Scenario: A historical or planning record changes
- **WHEN** a prior architecture, handoff, archived change, or milestone is edited while current canonical sources are unchanged
- **THEN** normal extraction, `docs check`, projections, relationship review, and current requirement context remain unchanged.

#### Scenario: An agent requests archaeology
- **WHEN** a query explicitly requests a historical architecture, archived change, or milestone
- **THEN** the packet contains only the requested bounded historical material, marks it non-authoritative, and does not use it to satisfy current coverage.

### Requirement: Agent context is identity-selected and reproducible
<!-- dwv:req req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->

Given an explicit current requirement ID, the context and ownership commands SHALL return its normalized canonical unit, local and effective semantic/review fingerprints, forward relationships, derived backlinks, reviewed outcome and reason, bounded owner-before-dependent reading order, and bounded typed current implementation, verification, curriculum, and Markdown references. They SHALL report omitted counts when bounds truncate results. Ordinary current requirement packets SHALL exclude roadmap and milestone prose. The commands SHALL NOT concatenate whole specifications, retain a revision-diff or ownership registry, infer prose validity or semantic coherence, or rely on generated human prose, historical planning, or milestone text when canonical semantics are available.

#### Scenario: A current requirement is selected
- **WHEN** an agent requests context or ownership facts for a current requirement ID
- **THEN** the packet contains the canonical unit, typed relationships, derived reading order, current endpoint locations, explicit bounds, and omissions in stable order without roadmap or milestone prose.

#### Scenario: Context exceeds a bound
- **WHEN** serialized context exceeds the configured byte bound
- **THEN** the command fails closed instead of silently dropping correctness context.

#### Scenario: A milestone references the selected requirement
- **WHEN** a milestone contains the selected requirement ID or matching normative prose
- **THEN** current context, ownership, affected-path analysis, and review freshness remain unchanged.

### Requirement: Reconstruction and historical isolation are inspectable
<!-- dwv:req req.documentation-knowledge-architecture.reconstruction-and-historical-isolation-are-inspectable -->

The tool SHALL provide a clean-room check that removes regenerable state, generated indexes, historical handoffs, milestones, archived changes, and roadmap inputs from a temporary copy, reconstructs current projections from permanent current artifacts, and verifies equivalent semantic output. Completion evidence SHALL show that current generation and requirement context do not depend on any prior documentation implementation, handoff, milestone, roadmap, archived change, or chat history.

#### Scenario: Regenerable and historical state is deleted
- **WHEN** `target/dwv-docs`, generated projection caches, handoffs, milestones, archived changes, and roadmap inputs are absent in a temporary copy
- **THEN** the documented extraction, planning, build, check, and reconstruction commands recreate current generated state and produce equivalent current facts and accepted pages.

### Requirement: Canonical semantic relationships are colocated and derived
<!-- dwv:req req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry -->

Current canonical requirements SHALL declare a forward `requires` relationship for every consequential independently owned semantic prerequisite needed to interpret or implement the local requirement, and SHALL declare a forward `refines` relationship when the local requirement is a narrower specialization or adapter realization of another requirement's detailed operational policy. Requirements with no such consequential relationship SHALL declare none. `requires` means the source requirement's own semantics depend on an independently owned semantic fact. `refines` means the target requirement owns the same underlying detailed operational policy and the source defines a narrower specialization or adapter realization of that policy. Composition alone SHALL NOT create `refines`; a composer SHALL use `requires` for consequential prerequisites and SHALL NOT restate their predicates. Forward relationship markers in current canonical specifications SHALL be the only relationship authority; roadmap, milestone, handoff, archive, implementation, test, and generated prose SHALL NOT contribute relationship edges.

Semantic reconciliation SHALL determine and author the consequential relationships required by the local semantics. Deterministic tooling SHALL validate the authored representation and SHALL NOT infer a missing semantic relationship from arbitrary English prose.

Relationship markers SHALL be contiguous immediately after the intrinsic identity. A source-target pair SHALL use at most one relation kind, and every target SHALL resolve to a current canonical requirement ID. The tool SHALL derive backlinks, capability aggregation, bounded ownership views, and owner-before-dependent reading order from forward markers without a second registry or manual backlink list. Unknown, non-current, self, duplicate, mixed-kind, misplaced, or cyclic relationships SHALL fail deterministically with stable source and cycle diagnostics.

Universal constitutional, security, and evidence constraints SHALL NOT require repeated edges on every detailed requirement. Lexical or terminology similarity, shared tests or evidence, implementation call graphs, and optional or future work SHALL NOT create semantic edges. Reverse relationships SHALL remain derived.

Each requirement SHALL have a local semantic fingerprint that is a formatting-stable digest of its local normative prose and scenarios, excluding intrinsic and relationship marker comments and non-semantic formatting. After validating that the combined semantic graph is acyclic, the tool SHALL compute each effective semantic/review fingerprint prerequisite-first as a deterministic digest of the local semantic fingerprint, sorted outgoing `(relation kind, target ID)` pairs, and each target's effective semantic/review fingerprint. Reviewed state SHALL store and compare the effective semantic/review fingerprint. Formatting-only changes and changes only to review outcome or reason SHALL preserve semantic fingerprints. A local semantic or outgoing-edge change SHALL stale that requirement and every transitive dependent effective fingerprint until each is explicitly reviewed; unrelated requirements SHALL remain unchanged.

#### Scenario: A consequential independently owned prerequisite exists
- **WHEN** semantic reconciliation establishes that a local requirement needs an independently owned semantic fact for interpretation or implementation
- **THEN** the local requirement declares a forward `requires` relationship to that fact before semantic review may approve it.

#### Scenario: A narrower specialization or adapter realization exists
- **WHEN** semantic reconciliation establishes that the target owns the same detailed operational policy and the local requirement narrows or realizes that policy
- **THEN** the local requirement declares a forward `refines` relationship and does not also declare `requires` for the same source-target pair.

#### Scenario: No consequential semantic relationship exists
- **WHEN** a requirement has no consequential prerequisite, specialization, or adapter realization beyond universal constraints, lexical similarity, shared evidence, implementation calls, or optional future work
- **THEN** it declares no relationship for those non-semantic associations.

#### Scenario: Authored relationships are extracted
- **WHEN** required forward markers are present and structurally valid in current canonical specifications
- **THEN** canonical extraction produces stable typed forward edges, derived backlinks, capability aggregation, and owner-before-dependent reading order without consulting roadmap, milestone, handoff, archive, implementation, test, or generated prose.

#### Scenario: Relationship markers are invalid
- **WHEN** a relationship target is unknown or non-current, a source targets itself, a pair is duplicated or assigned both kinds, markers are misplaced, or the graph contains a cycle
- **THEN** extraction or readiness fails with stable source-target or cycle diagnostics.

#### Scenario: An owner changes
- **WHEN** a requirement's local semantics or outgoing relationships change
- **THEN** its effective semantic/review fingerprint and every transitive dependent effective fingerprint change, while unrelated components remain stable, and readiness fails until every stale requirement is individually reviewed.

#### Scenario: Review metadata or formatting changes
- **WHEN** only Markdown formatting, review outcome, or review reason changes
- **THEN** local and dependent effective semantic/review fingerprints remain unchanged.
