## ADDED Requirements

### Requirement: Agent knowledge workflows use sufficient fixed-purpose retrieval
<!-- dwv:req req.documentation-knowledge-architecture.agent-knowledge-workflows-use-one-complete-retrieval-path -->
<!-- dwv:requires req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible -->

Trusted agent workflow instructions SHALL choose a fixed-purpose knowledge retrieval whose returned canonical semantics are sufficient for the task and SHALL avoid redundant overlapping retrieval by default. A successful sufficient retrieval SHALL NOT be routinely followed by another operation that returns the same semantic unit for the same need. When task intent changes, an operation fails closed, or changed semantics make downstream impact the new task, a workflow MAY choose the corresponding distinct fixed-purpose operation. Concrete readiness timing, batching recipes, and command choreography SHALL remain in trusted skill instructions rather than becoming canonical policy.

#### Scenario: Normal correctness work needs prerequisite semantics

- **WHEN** an agent begins work from a selected requirement with transitive semantic prerequisites
- **THEN** the workflow obtains one sufficient fixed-purpose context result containing the governing canonical semantics without redundant follow-up retrieval

#### Scenario: A semantic owner changes

- **WHEN** implementation or planning changes a requirement's local semantics or outgoing semantic relationships
- **THEN** the workflow uses the fixed-purpose downstream impact operation and individually reviews the reported dependents rather than treating initial task context as change-impact evidence

#### Scenario: A narrow lookup is sufficient

- **WHEN** the task needs only one exact local unit or one requirement's direct ownership and endpoint facts
- **THEN** the workflow uses the corresponding narrow fixed-purpose retrieval without also requesting broader overlapping context

#### Scenario: Retrieval intent changes

- **WHEN** a fixed-purpose operation fails closed or the task changes to a different retrieval intent
- **THEN** the workflow may use the operation for that new intent without treating the additional retrieval as routine overlap

## MODIFIED Requirements

### Requirement: Agent context is identity-selected and reproducible
<!-- dwv:req req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->
<!-- dwv:requires req.documentation-knowledge-architecture.provenance-is-claim-and-fragment-level -->

Knowledge commands SHALL have distinct complete contracts. `inspect`, `ownership`, and `audit-context` SHALL each select exactly one explicit current requirement ID; `context` SHALL select one or more explicit current requirement IDs. `inspect` SHALL return one selected complete normalized canonical unit, its intrinsic identity, exact current source locator, local and effective semantic fingerprints, and direct forward relationships and derived backlinks. `ownership` SHALL return selected identity and fingerprint facts, direct forward relationships and derived backlinks, reviewed outcome and reason, and bounded typed current implementation, verification, curriculum, and Markdown references without repeating the normalized canonical body or a transitive reading order.

For selected set `S`, ordinary `context` SHALL compute transitive semantic prerequisites `P` through outgoing `requires` and `refines` edges and direct dependents `D` through incoming `required_by` and `refined_by` edges. It SHALL return one de-duplicated packet containing the complete normalized canonical unit and source locator for every requirement in `P ∪ S ∪ D`, selected fingerprint and review facts, direct relationships in the induced scope, bounded typed references, bounds and actionable omissions, and a stable owner-before-dependent reading order. Requirements, relationships, review facts, references, bounds, omissions, and reading-order entries shared by multiple selected IDs SHALL appear once.

The ordinary context scope SHALL remain directed and SHALL NOT traverse unrelated sibling branches through an undirected component. Complete downstream change impact SHALL remain owned by the affected-path contract. An explicit `audit-context` operation selected by one current requirement ID SHALL traverse that requirement's complete undirected connected semantic component and SHALL include the complete normalized canonical unit of every returned component member. Neither ordinary nor audit context SHALL require caller-selected fields, graph depth, relationship direction, or closure policy.

Canonical extraction SHALL enforce the source-unit byte bound before projection. `inspect` therefore returns its one complete unit or fails source extraction without also applying the combined context-packet byte bound. Context-bearing packet operations SHALL fail closed when their complete serialized result exceeds the packet byte bound. Bounded reference-category truncation SHALL report omitted counts by selected requirement ID and category. When an oversized multi-ID ordinary context packet fails the bound, the failure SHALL suggest one singleton context request per selected requirement ID only if every complete singleton packet has been proven to fit; those requests MAY repeat canonical units shared by their directed scopes. If any selected singleton packet or the one required audit component is oversized, the result SHALL state that the scope cannot be split without omission.

Source locators SHALL contain the exact current canonical path and requirement line range while line movement remains independent of intrinsic requirement identity and semantic fingerprints. Ordinary current requirement packets SHALL exclude roadmap and milestone prose. The commands SHALL NOT concatenate whole specifications, retain a revision-diff or ownership registry, infer prose validity or semantic coherence, or rely on generated human prose, historical planning, or milestone text when canonical semantics are available.

#### Scenario: One exact canonical unit is inspected

- **WHEN** an agent requests inspect facts for one current requirement ID whose canonical unit satisfies the source-unit bound
- **THEN** the result contains that complete local canonical unit, exact current source locator, fingerprints, and direct relationships without ownership references, transitive reading order, or a second combined-packet bound

#### Scenario: A canonical unit exceeds the source bound

- **WHEN** canonical extraction finds that one normalized requirement unit exceeds the source-unit byte bound
- **THEN** extraction fails before inspect or context can return a partial canonical unit

#### Scenario: Direct ownership facts are requested

- **WHEN** an agent requests ownership facts for one current requirement ID
- **THEN** the result contains direct semantic edges, reviewed facts, and typed endpoint locations without repeating the canonical body or expanding a connected semantic component

#### Scenario: A prerequisite is necessary ordinary context

- **WHEN** a selected requirement transitively requires or refines another requirement
- **THEN** ordinary context contains the prerequisite's complete canonical unit without requiring the caller to select it separately

#### Scenario: A current requirement is selected

- **WHEN** an agent requests context for one current requirement ID
- **THEN** the packet contains complete canonical units for that requirement's directed semantic scope, exact locators, relationships, selected review and endpoint facts, bounds, and actionable omissions in stable order

#### Scenario: Several requirements are selected for context

- **WHEN** an agent requests context for multiple current requirement IDs
- **THEN** one stable packet contains every complete canonical unit in the combined directed scope and every shared relationship, selected review fact, reference, bound, omission, and reading-order entry exactly once

#### Scenario: Ordinary context selects a bounded semantic neighborhood

- **WHEN** selected requirements belong to a larger connected semantic component
- **THEN** ordinary context includes complete canonical units for their transitive semantic prerequisites, the selected requirements, and direct dependents in owner-before-dependent order without traversing unrelated sibling branches

#### Scenario: A full semantic component is deliberately audited

- **WHEN** an agent explicitly requests audit context for a selected current requirement
- **THEN** every requirement named in the complete connected component and reading order has its complete canonical unit in the audit packet

#### Scenario: Multiple audit seeds are refused

- **WHEN** an agent supplies more than one current requirement ID to audit-context
- **THEN** the command fails before packet construction and requires separate single-seed audit invocations

#### Scenario: Batched references exceed a category bound

- **WHEN** globally bounded references for multiple selected requirements are truncated
- **THEN** the packet reports the exact omitted count for each selected requirement and reference category

#### Scenario: Context exceeds a bound

- **WHEN** a complete required context-bearing packet exceeds the configured packet byte bound
- **THEN** the command fails closed and returns complete singleton suggested requests only if every selected ID's singleton packet fits, or states that a required scope cannot be split without omission

#### Scenario: Selected singleton contexts fit the combined bound failure

- **WHEN** a multi-ID context packet exceeds the byte bound but every selected ID's complete singleton context packet fits
- **THEN** the failure returns one deterministic singleton request per selected ID even when their directed scopes share canonical units

#### Scenario: One audit component exceeds the bound

- **WHEN** a complete audit component exceeds the configured packet byte bound
- **THEN** the command fails closed and states that the component cannot be split without omission rather than suggesting selected-ID requests that reproduce it

#### Scenario: Canonical source movement is non-semantic

- **WHEN** a canonical requirement moves to different source lines without semantic change
- **THEN** only its source locator changes while intrinsic identity and local and effective semantic fingerprints remain unchanged

#### Scenario: A milestone references the selected requirement

- **WHEN** a milestone contains a selected requirement ID or matching normative prose
- **THEN** current inspect, context, ownership, audit-context, affected-path analysis, and review freshness remain unchanged
