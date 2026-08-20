## RENAMED Requirements

- FROM: ### Requirement: Human curriculum is pedagogical intent, not semantic authority
- TO: ### Requirement: Guide curriculum is pedagogical intent, not semantic authority

## MODIFIED Requirements

### Requirement: Guide curriculum is pedagogical intent, not semantic authority
<!-- dwv:req req.documentation-knowledge-architecture.guide-curriculum-is-pedagogical-intent-not-semantic-authority -->

Guide projection configuration SHALL describe audience, question, learning outcome, prerequisites, misconceptions, deferred concepts, canonical requirement/scenario/evidence references, and an ordered set of section briefs. Each section brief SHALL provide a proposed Markdown heading, focus, questions it must answer, applicable sources, and explicit non-claims. The briefs guide drafting and review but SHALL NOT copy canonical requirement text, define semantic authority, or become generated page state. Deterministic readiness SHALL reject missing or empty section-brief fields for DiskWeave Guide entries. The initial DiskWeave Guide SHALL use those briefs to form a causal journey in which readers can identify what happened, what is durable, what callers may believe, what recovery observes, what remains uncertain, and what DiskWeave refuses to infer.

#### Scenario: A DiskWeave Guide chapter is drafted

- **WHEN** an agent writes or revises a DiskWeave Guide chapter
- **THEN** it follows the entry's ordered section briefs, answers each required question from the linked current sources, preserves the stated non-claims, and uses maintained checked-in Markdown rather than generated prose.

### Requirement: Projections render useful facts for their intended consumers
<!-- dwv:req req.documentation-knowledge-architecture.projections-render-useful-facts-for-their-intended-consumers -->

The tool SHALL produce separate projections whose observable content is better suited to the declared audience than a raw canonical-spec dump: a causal DiskWeave Guide, a complete architecture reference, executable Scenario Book entries, claim/evidence Assurance Atlas entries, deterministic Cargo/source Contributor Map entries, and task-specific agent context. Scenario and assurance projections SHALL render actual structured facts and relationships, not descriptions of what a projection would contain.

#### Scenario: A scenario is projected

- **WHEN** a registered executable simulator, normalized-trace, model, or equivalent fixture passes
- **THEN** the Scenario Book renders its initial state, actions/transitions, durability/fault points, observations, final result classification, and explicit forbidden inferences.

#### Scenario: An assurance claim is projected

- **WHEN** a claim has assumptions, mechanisms, properties, evidence, strength, non-claims, or gaps
- **THEN** the Assurance Atlas renders those relationships and links exact canonical requirements and evidence artifacts.

#### Scenario: Contributor ownership is projected

- **WHEN** Cargo metadata and Rust entry points are available
- **THEN** the Contributor Map renders current packages, dependency direction, ownership, canonical requirement bindings, scenarios/evidence, and prohibited edges without duplicating package facts in hand-maintained JSON.


## ADDED Requirements

### Requirement: Concepts and Terminology is maintained incrementally
<!-- dwv:req req.documentation-knowledge-architecture.concepts-and-terminology-is-maintained-incrementally -->
<!-- dwv:requires req.documentation-knowledge-architecture.provenance-is-claim-and-fragment-level -->
<!-- dwv:refines req.documentation-knowledge-architecture.preservation-first-updates-are-truly-narrow -->

Concepts and Terminology SHALL be maintained as checked-in explanatory Markdown with provenance to the current semantic owners of each correctness-sensitive entry. A changed owner or relationship SHALL trigger targeted review of affected entries through the existing change-impact and provenance mechanisms. Unaffected entries SHALL remain unchanged rather than being regenerated or rewritten. Concepts and Terminology SHALL explain current semantics and important non-implications without becoming a duplicate semantic registry or semantic authority.

#### Scenario: One concept owner changes

- **WHEN** change-impact identifies a semantic owner change for one Concepts and Terminology entry
- **THEN** that entry is reviewed against its current sources while unrelated entries remain unchanged

#### Scenario: A concept has conflicting or missing authority

- **WHEN** the current semantic sources do not support one unambiguous explanatory claim
- **THEN** the entry records the gap or omits the unsupported claim instead of generating an inferred definition
