## ADDED Requirements

### Requirement: Canonical semantic relationships are colocated and derived
<!-- dwv:req req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry -->

Current canonical requirements MAY declare forward `requires` and `refines` relationships immediately after their intrinsic identity. `requires` means the local requirement depends on another independently owned semantic fact. `refines` means the target owns the underlying detailed operational policy and the local requirement defines that policy's narrower local specialization, composition mapping, or adapter conformance. A source-target pair SHALL use at most one relation kind. Targets SHALL resolve to current canonical requirement IDs. The tool SHALL derive backlinks, capability aggregation, bounded ownership views, and owner-before-dependent reading order from those forward markers without a second registry or manual backlink list. Unknown, non-current, self, duplicate, mixed-kind duplicate, or cyclic relationships SHALL fail deterministically with exact source and cycle diagnostics.

Each requirement SHALL have a local semantic fingerprint derived from normalized local prose and scenarios while excluding intrinsic/relationship comments and formatting-only differences. After validating that the combined semantic graph is acyclic, the tool SHALL compute each requirement's effective semantic fingerprint prerequisite-first from its local semantic fingerprint, sorted outgoing relation kind/target pairs, and each target's effective semantic fingerprint. Reviewed state SHALL store and compare the effective semantic fingerprint. Formatting-only changes and changes only to review outcome/reason SHALL preserve effective fingerprints. A local semantic or edge change SHALL make every direct and transitive semantic dependent review-suspect. Every suspect requirement SHALL require an individual reviewed outcome and concrete reason; bulk acceptance SHALL remain unavailable.

#### Scenario: A valid owner and refiner are extracted

- **WHEN** one current requirement refines another current requirement and both markers are valid
- **THEN** extraction exposes the forward edge, derived backlink, bounded owner-before-dependent order, local fingerprints, and effective semantic fingerprints without a manual backlink record

#### Scenario: The relationship graph is invalid

- **WHEN** an edge has an unknown or non-current target, is self-referential, duplicates or conflicts with another edge, or participates in a cycle
- **THEN** readiness fails with source path, line, source ID, relation kind, target ID, and the smallest deterministic discovered cycle when applicable

#### Scenario: An owner's local semantics change

- **WHEN** requirements B and C transitively depend on owner A, unrelated requirement D has no path to A, and A's normalized local prose or scenarios change while B, C, and D remain locally unchanged
- **THEN** B and C receive changed effective semantic fingerprints and `semantic-prerequisite-changed` diagnostics until individually reviewed while D's effective semantic fingerprint remains stable

#### Scenario: Only owner formatting changes

- **WHEN** an owner's semantics and relationships are unchanged but Markdown formatting changes
- **THEN** local and dependent effective semantic fingerprints remain unchanged

## MODIFIED Requirements

### Requirement: Change-boundary impact is deterministic and bounded
<!-- dwv:req req.documentation-knowledge-architecture.change-boundary-impact-is-deterministic-and-bounded -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->

At a change boundary, deterministic readiness SHALL compare current local and effective semantic fingerprints and current typed relationships with an available repository revision baseline through a provider-neutral interface. Local semantic changes, dependency-closure semantic-prerequisite changes, removed or reassigned relationships, and retired IDs SHALL identify the exact requirements requiring individual review. Formatting-only changes and implementation edits with unchanged relationships remain context rather than automatically creating documentation work. The command SHALL report when no baseline is available instead of claiming that relationships are unchanged, and diagnostics SHALL provide exact bounded inspection commands.

#### Scenario: A semantic prerequisite changes

- **WHEN** a target's effective semantic fingerprint changes or a source relationship is removed or reassigned
- **THEN** `docs check` reports every exact direct and transitive review-required dependent ID with `semantic-prerequisite-changed` diagnostics while unrelated requirements remain unchanged

#### Scenario: The revision baseline is unavailable

- **WHEN** no supported repository revision baseline can be read
- **THEN** semantic readiness still runs and relationship-delta status is reported as unavailable rather than unchanged

### Requirement: Agent context is identity-selected and reproducible
<!-- dwv:req req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->

Given an explicit current requirement ID, the context and ownership commands SHALL return its normalized canonical unit, local and effective semantic fingerprints, forward relationships, derived backlinks, reviewed outcome and reason, bounded owner-before-dependent reading order, and bounded typed current implementation, verification, curriculum, and Markdown references. They SHALL report omitted counts when bounds truncate results. The commands SHALL NOT concatenate whole specifications, retain a revision-diff or ownership registry, infer prose validity or semantic coherence, or rely on generated human prose when canonical semantics are available.

#### Scenario: A current requirement is selected

- **WHEN** an agent requests context or ownership facts for a current requirement ID
- **THEN** the packet contains the canonical unit, typed relationships, derived reading order, current endpoint locations, explicit bounds, and omissions in stable order

#### Scenario: Context exceeds a bound

- **WHEN** serialized context exceeds the configured byte bound
- **THEN** the command fails closed instead of silently dropping correctness context
