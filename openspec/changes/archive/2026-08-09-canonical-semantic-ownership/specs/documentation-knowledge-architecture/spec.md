## ADDED Requirements

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

## MODIFIED Requirements

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
