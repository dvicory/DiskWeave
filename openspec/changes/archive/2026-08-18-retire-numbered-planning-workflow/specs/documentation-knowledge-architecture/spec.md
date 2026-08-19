## MODIFIED Requirements

### Requirement: Canonical requirements are discovered without a duplicate semantic registry
<!-- dwv:req req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry -->

The documentation tool SHALL discover every current `openspec/specs/*/spec.md` automatically, extract each `Requirement` as a stable semantic unit, and expose an intrinsic requirement identity that is independent of roadmap nodes, verification identifiers, numbered planning identifiers, historical source paths, archive paths, line numbers, and generated state. A requirement identity SHALL remain stable when surrounding Markdown is reordered or reformatted, and a changed or missing identity SHALL produce an explicit diagnostic.

#### Scenario: A canonical requirement is added
- **WHEN** a new correctness-sensitive `Requirement` appears in a current canonical spec without a projection or context disposition
- **THEN** extraction succeeds, coverage reports the requirement as newly uncovered, and `docs check` fails closed until an explicit disposition exists.

#### Scenario: Formatting and ordering change
- **WHEN** a canonical spec changes only Markdown formatting or reorders unrelated requirements
- **THEN** requirement identities and dependent accepted prose remain unchanged.

#### Scenario: A milestone contains requirement-like prose
- **WHEN** an explicitly selected historical source outside the current canonical source set contains headings, normative language, or current requirement identifiers
- **THEN** canonical extraction ignores that record and derives current requirements only from `openspec/specs/*/spec.md`.

### Requirement: Historical architecture is opt-in only
<!-- dwv:req req.documentation-knowledge-architecture.historical-architecture-is-opt-in-only -->

The normal source graph, freshness checks, projections, and agent context SHALL exclude `docs/architecture/archive/**`, archived changes, and prior architecture revisions. Unmarked prose from the marked active roadmap SHALL remain excluded from those ordinary current-semantic surfaces. Complete explicitly marked active invariant units SHALL be the only roadmap exception and MAY participate in current fingerprints and bounded context solely through the architecture-invariant contracts; they SHALL NOT satisfy current requirement coverage. A history request SHALL include an explicit opt-in and SHALL label selected historical architecture or archived-change material historical; that material SHALL never resolve a current requirement conflict. The prohibited `docs/milestones/**` path SHALL NOT be treated as a historical source; if it is recreated, knowledge tooling SHALL fail closed with a bounded retired-path diagnostic before the path can contribute to extraction, references, affected-path analysis, readiness, direct knowledge output, or clean-room reconstruction.

#### Scenario: A historical or planning record changes
- **WHEN** a prior architecture or archived change is edited while current canonical sources are unchanged
- **THEN** normal extraction, `docs check`, projections, relationship review, and current requirement context remain unchanged.

#### Scenario: An agent requests archaeology
- **WHEN** a query explicitly requests a historical architecture or archived change
- **THEN** the packet contains only the requested bounded historical material, marks it non-authoritative, and does not use it to satisfy current coverage.

#### Scenario: Retired planning path is recreated
- **WHEN** `docs/milestones/**` is recreated in the current tree
- **THEN** knowledge tooling SHALL fail closed with a bounded retired-path diagnostic before the path can contribute to extraction, reference scanning, affected-path analysis, readiness, direct knowledge output, or clean-room reconstruction.

### Requirement: Active roadmap is explicit and non-authoritative
<!-- dwv:req req.documentation-knowledge-architecture.active-roadmap-is-explicit-and-non-authoritative -->
<!-- dwv:requires req.documentation-knowledge-architecture.historical-architecture-is-opt-in-only -->

The documentation workflow SHALL deterministically discover exactly one active architecture roadmap from one explicit repository marker and matching lifecycle metadata. The selected roadmap's unmarked prose SHALL remain excluded from ordinary current semantic discovery, projections, freshness checks, and requirement context. Complete explicitly marked active invariant units MAY supply architecture-level constraints to canonical requirements and appear in bounded current context only through the architecture-invariant contracts; they SHALL NOT become detailed product-policy owners, silently override a conflicting current OpenSpec, or satisfy current requirement coverage. A deliberate roadmap-context request MAY select the whole marked roadmap, but the resulting material SHALL be identified as non-authoritative design direction. A detected disagreement between the roadmap and current canonical requirements SHALL be reported for explicit reconciliation and SHALL NOT be resolved by silently preferring either source.

#### Scenario: Exactly one active roadmap is marked
- **WHEN** documentation readiness scans the maintained repository
- **THEN** it resolves one deterministic active-roadmap path and continues without treating unmarked roadmap prose or marked architecture constraints as detailed current product authority.

#### Scenario: The active-roadmap marker is missing or ambiguous
- **WHEN** zero or multiple active-roadmap markers are present
- **THEN** documentation readiness fails with the marker count and every marked path rather than choosing a roadmap implicitly.

#### Scenario: Milestone planning deliberately requests roadmap context
- **WHEN** a caller requests the marked active roadmap after inspecting current canonical requirements
- **THEN** the whole roadmap is available as non-authoritative direction while its unmarked prose remains outside ordinary requirement coverage and context.

#### Scenario: Current requirements and roadmap direction disagree
- **WHEN** planning or semantic reconciliation detects a material disagreement
- **THEN** it records the disagreement for an explicit specification or roadmap decision without silently rewriting current semantics.

### Requirement: Agent context is identity-selected and reproducible
<!-- dwv:req req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->
<!-- dwv:requires req.documentation-knowledge-architecture.provenance-is-claim-and-fragment-level -->

Knowledge commands SHALL have distinct complete contracts. `inspect`, `ownership`, and `audit-context` SHALL each select exactly one explicit current requirement ID; `context` SHALL select one or more explicit current requirement IDs. `inspect` SHALL return one selected complete normalized canonical unit, its intrinsic identity, exact current source locator, local and effective semantic fingerprints, direct forward requirement relationships, derived requirement backlinks, and any derived active architecture-constraint backlinks. `ownership` SHALL return selected identity and fingerprint facts, those same direct relationships and derived backlinks, reviewed outcome and reason, and bounded typed current implementation, verification, curriculum, and Markdown references without repeating the normalized canonical body or a transitive reading order.

For selected set `S`, ordinary `context` SHALL compute transitive semantic prerequisites `P` through outgoing `requires` and `refines` edges and direct dependents `D` through incoming `required_by` and `refined_by` edges. It SHALL return one de-duplicated packet containing the complete normalized canonical unit and source locator for every requirement in `P ∪ S ∪ D`, selected fingerprint and review facts, direct requirement relationships in the induced scope, directly applicable complete active architecture-invariant units and constraint edges, bounded typed references, bounds and actionable omissions, and a stable owner-before-dependent reading order. Requirements, relationships, invariant units, constraint edges, review facts, references, bounds, omissions, and reading-order entries shared by multiple selected IDs SHALL appear once.

The ordinary context scope SHALL remain directed and SHALL NOT traverse unrelated sibling branches through an undirected component. Complete downstream change impact SHALL remain owned by the affected-path contract. An explicit `audit-context` operation selected by one current requirement ID SHALL traverse that requirement's complete undirected connected semantic component and SHALL include the complete normalized canonical unit of every returned component member. Neither ordinary nor audit context SHALL require caller-selected fields, graph depth, relationship direction, or closure policy.

Canonical extraction SHALL enforce the source-unit byte bound before projection. `inspect` therefore returns its one complete unit or fails source extraction without also applying the combined context-packet byte bound. Context-bearing packet operations SHALL fail closed when their complete serialized result exceeds the packet byte bound. Bounded reference-category truncation SHALL report omitted counts by selected requirement ID and category. When an oversized multi-ID ordinary context packet fails the bound, the failure SHALL suggest one singleton context request per selected requirement ID only if every complete singleton packet has been proven to fit; those requests MAY repeat canonical units shared by their directed scopes. If any selected singleton packet or the one required audit component is oversized, the result SHALL state that the scope cannot be split without omission.

Source locators SHALL contain the exact current canonical path and requirement line range while line movement remains independent of intrinsic requirement identity and semantic fingerprints. Ordinary current requirement packets SHALL exclude unmarked roadmap prose and every candidate or superseded architecture unit. They MAY include complete marked active architecture invariant units only when the architecture-invariant contracts select them for a requirement already in the packet. The commands SHALL NOT concatenate whole specifications or architecture documents, retain a revision-diff or ownership registry, infer prose validity or semantic coherence, or rely on generated human prose or historical source prose when canonical semantics are available.

#### Scenario: One exact canonical unit is inspected

- **WHEN** an agent requests inspect facts for one current requirement ID whose canonical unit satisfies the source-unit bound
- **THEN** the result contains that complete local canonical unit, exact current source locator, fingerprints, direct requirement relationships, and derived active architecture-constraint backlinks without ownership references, transitive reading order, or a second combined-packet bound

#### Scenario: A canonical unit exceeds the source bound

- **WHEN** canonical extraction finds that one normalized requirement unit exceeds the source-unit byte bound
- **THEN** extraction fails before inspect or context can return a partial canonical unit

#### Scenario: Direct ownership facts are requested

- **WHEN** an agent requests ownership facts for one current requirement ID
- **THEN** the result contains direct semantic edges, derived active architecture-constraint backlinks, reviewed facts, and typed endpoint locations without repeating the canonical body or expanding a connected semantic component

#### Scenario: A prerequisite is necessary ordinary context

- **WHEN** a selected requirement transitively requires or refines another requirement
- **THEN** ordinary context contains the prerequisite's complete canonical unit without requiring the caller to select it separately

#### Scenario: A current requirement is selected

- **WHEN** an agent requests context for one current requirement ID
- **THEN** the packet contains complete canonical units for that requirement's directed semantic scope, directly applicable complete active architecture-invariant units and constraints, exact locators, relationships, selected review and endpoint facts, bounds, and actionable omissions in stable order

#### Scenario: Several requirements are selected for context

- **WHEN** an agent requests context for multiple current requirement IDs
- **THEN** one stable packet contains every complete canonical unit and directly applicable complete active architecture-invariant unit in the combined directed scope and every shared relationship, constraint edge, selected review fact, reference, bound, omission, and reading-order entry exactly once

#### Scenario: Ordinary context selects a bounded semantic neighborhood

- **WHEN** selected requirements belong to a larger connected semantic component
- **THEN** ordinary context includes complete canonical units for their transitive semantic prerequisites, the selected requirements, and direct dependents plus directly applicable active architecture invariants, without traversing unrelated sibling branches

#### Scenario: A full semantic component is deliberately audited

- **WHEN** an agent explicitly requests audit context for a selected current requirement
- **THEN** every requirement named in the complete connected component and reading order and every directly applicable active architecture invariant has its complete semantic unit in the audit packet

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
- **THEN** the failure returns one deterministic singleton request per selected ID even when their directed scopes share canonical units or active architecture invariants

#### Scenario: One audit component exceeds the bound

- **WHEN** a complete audit component exceeds the configured packet byte bound
- **THEN** the command fails closed and states that the component cannot be split without omission rather than suggesting selected-ID requests that reproduce it

#### Scenario: Canonical source movement is non-semantic

- **WHEN** a canonical requirement or marked active invariant moves to different source lines without semantic change
- **THEN** only its source locator changes while intrinsic identity and local and effective semantic fingerprints remain unchanged

#### Scenario: A milestone references the selected requirement

- **WHEN** an explicitly selected historical source outside the current input set contains a selected requirement ID or matching normative prose
- **THEN** current inspect, context, ownership, audit-context, affected-path analysis, and review freshness remain unchanged.

### Requirement: Reconstruction and historical isolation are inspectable
<!-- dwv:req req.documentation-knowledge-architecture.reconstruction-and-historical-isolation-are-inspectable -->

The tool SHALL provide a clean-room check that removes regenerable state, generated indexes, candidate and archived architecture roadmaps, and archived changes from a temporary copy; retains only the selected active architecture roadmap as the source of any explicitly marked current invariant units; reconstructs current projections from permanent current artifacts; and verifies equivalent semantic output. Completion evidence SHALL show that current generation and requirement context do not depend on any prior documentation implementation, candidate or archived source, unmarked roadmap prose as semantic input, or chat history.

#### Scenario: Regenerable and historical state is deleted
- **WHEN** `target/dwv-docs`, generated projection caches, candidate and archived architecture roadmaps, and archived changes are absent in a temporary copy that retains only the selected active roadmap
- **THEN** the documented extraction, planning, build, check, and reconstruction commands recreate current generated state and produce equivalent current facts and accepted pages from canonical requirements and any marked active architecture invariant units.

### Requirement: Architecture invariants have stable source-local identity and fingerprints
<!-- dwv:req req.documentation-knowledge-architecture.architecture-invariants-have-stable-source-local-identity-and-fingerprints -->

An architecture roadmap MAY identify only selected enduring constitutional or cross-cutting units under a heading whose title begins `Invariant:`. The first non-blank line after that heading SHALL be exactly one source-local `<!-- dwv:arch-invariant arch.<identity> -->` marker. The marked unit SHALL end before the next heading of equal or higher level. Architecture invariant identities SHALL be lowercase `arch.*` semantic identities, SHALL be distinct from every architecture document identity and `req.*` identity, and SHALL NOT derive identity from a document revision, section number, path, numbered planning identifier, OpenSpec change, or generated state. Unmarked architecture prose SHALL remain ordinary roadmap material and SHALL NOT acquire implicit identity.

The knowledge system SHALL compute a local semantic fingerprint from the complete normalized marked unit while excluding the identity marker, `constrains` markers, invariant-lineage markers, document metadata, lifecycle, path, and source locator. Markdown wrapping, whitespace, section movement, path movement, and lifecycle promotion SHALL preserve the fingerprint. A normative semantic change SHALL change it. The same enduring contract SHALL retain its invariant identity across revisions even when its source location or surrounding document structure changes. Malformed or duplicate invariant identities within one revision, or duplicate active invariant identities, SHALL fail with explicit bounded diagnostics. The accepted v0.8 roadmap SHALL remain valid without invariant markers.

#### Scenario: Marked active invariant is extracted
- **WHEN** an architecture revision contains an `Invariant:` unit followed by `<!-- dwv:arch-invariant arch.test.foo -->`
- **THEN** the knowledge system returns `arch.test.foo`, its complete normalized unit, semantic fingerprint, document identity, lifecycle, and exact source locator without treating adjacent unmarked prose as another invariant

#### Scenario: Formatting and location change
- **WHEN** the same marked invariant is rewrapped, moved under another section, moved to another valid architecture path, or promoted from candidate to active without changing its normalized semantics
- **THEN** its stable identity and local semantic fingerprint remain unchanged while document lifecycle and source locator may change

#### Scenario: Normative meaning changes
- **WHEN** normative content inside the marked invariant unit changes
- **THEN** the invariant local semantic fingerprint changes

#### Scenario: Existing active roadmap has no invariant markers
- **WHEN** the accepted active v0.8 roadmap is scanned without any `dwv:arch-invariant` marker
- **THEN** lifecycle validation succeeds and the active invariant set is empty without manufacturing historical identities

#### Scenario: Invariant identity is invalid or duplicated
- **WHEN** a marker is malformed, equals a document identity, uses a non-`arch.*` identity, or duplicates an invariant identity in one revision or the active view
- **THEN** validation fails with the document path and conflicting identity

### Requirement: Architecture context and lineage are bounded and authority-labeled
<!-- dwv:req req.documentation-knowledge-architecture.architecture-context-and-lineage-are-bounded-and-authority-labeled -->
<!-- dwv:requires req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible -->
<!-- dwv:requires req.documentation-knowledge-architecture.architecture-revision-lifecycle-is-explicit-and-isolated -->
<!-- dwv:requires req.documentation-knowledge-architecture.architecture-constraints-propagate-review-without-owning-product-policy -->

Ordinary `context` and `audit-context` packets SHALL include each complete active invariant unit that directly constrains any requirement in the packet exactly once, together with its identity, fingerprint, active document identity, source locator, and stored constraint edges. `inspect` and `ownership` SHALL expose the derived `constrained_by` identities for their selected current requirement. Current packets SHALL NOT concatenate a whole architecture document or include candidate, superseded, rationale-only, alternative, or unmarked architecture prose. Architecture-bearing current, candidate, and history packets SHALL enforce the existing source-unit and serialized context bounds and fail closed rather than truncate a required invariant unit.

A marked invariant MAY declare zero or more source-local `<!-- dwv:arch-supersedes arch.<identity> -->` markers after its identity and relationship markers. The marker SHALL record replacement lineage only: it SHALL NOT assert semantic equivalence, become a requirement prerequisite, contribute to requirement effective fingerprints, or authorize product behavior. Multiple markers on one invariant SHALL represent a merge; the same predecessor named by several successor invariants SHALL represent a split. Lineage targets SHALL name a distinct invariant identity present in another revision of the same architecture series; duplicate, self, unknown, cross-series, or cyclic lineage SHALL fail. Candidate and history results SHALL expose stored `supersedes` and derived `superseded_by` views.

#### Scenario: Current context selects one active invariant
- **WHEN** an active invariant constrains a selected requirement and another active invariant constrains no requirement in the packet
- **THEN** the packet includes only the complete relevant invariant unit once and remains within the packet bound

#### Scenario: Candidate packet exceeds its bound
- **WHEN** the complete marked candidate invariant set and impact result exceed the context-packet byte bound
- **THEN** candidate retrieval fails closed without returning partial invariant semantics

#### Scenario: Historical packet exceeds its bound
- **WHEN** the explicitly selected historical invariant set exceeds the context-packet byte bound
- **THEN** history retrieval fails closed without returning partial historical semantics

#### Scenario: Replacement lineage is queried
- **WHEN** `arch.test.new` declares `<!-- dwv:arch-supersedes arch.test.old -->`
- **THEN** candidate or history output returns the stored predecessor and derived successor while requirement relationships and fingerprints remain unchanged by lineage alone

#### Scenario: Split and merge lineage is queried
- **WHEN** two successor invariants each supersede one predecessor or one successor invariant supersedes two predecessors
- **THEN** the lineage query preserves every explicit edge without inferring equivalence or semantic dependency

#### Scenario: Synthetic fixtures exist outside production architecture roots
- **WHEN** test-only architecture text exists in source code or a fixture directory outside the isolated test repository's `docs/architecture` tree
- **THEN** production architecture discovery cannot select or ingest it

### Requirement: Canonical semantic relationships are colocated and derived
<!-- dwv:req req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry -->

Current canonical requirements SHALL declare a forward `requires` relationship for every consequential independently owned semantic prerequisite needed to interpret or implement the local requirement, and SHALL declare a forward `refines` relationship when the local requirement is a narrower specialization or adapter realization of another requirement's detailed operational policy. Requirements with no such consequential relationship SHALL declare none. `requires` means the source requirement's own semantics depend on an independently owned semantic fact. `refines` means the target requirement owns the same underlying detailed operational policy and the source defines a narrower specialization or adapter realization of that policy. Composition alone SHALL NOT create `refines`; a composer SHALL use `requires` for consequential prerequisites and SHALL NOT restate their predicates. Forward relationship markers in current canonical specifications SHALL be the only authority for requirement prerequisite and refinement edges; roadmap, handoff, archive, implementation, test, and generated prose SHALL NOT contribute `requires` or `refines` edges. A separately governed active architecture `constrains` edge SHALL remain an architecture-to-requirement review input and SHALL NOT become a requirement prerequisite or refinement edge.

Semantic reconciliation SHALL determine and author the consequential requirement relationships required by the local semantics. Deterministic tooling SHALL validate the authored representation and SHALL NOT infer a missing requirement relationship from arbitrary English prose.

`dwv:requires` and `dwv:refines` markers SHALL be contiguous immediately after the intrinsic identity. A source-target pair SHALL use at most one relation kind, and every target SHALL resolve to a current canonical requirement ID. The tool SHALL derive requirement backlinks, capability aggregation, bounded ownership views, and owner-before-dependent reading order from forward markers without a second registry or manual backlink list. Unknown, non-current, self, duplicate, mixed-kind, misplaced, or cyclic requirement relationships SHALL fail deterministically with stable source and cycle diagnostics.

Universal constitutional, security, and evidence constraints SHALL NOT require repeated `requires` or `refines` edges on every detailed requirement. Lexical or terminology similarity, shared tests or evidence, implementation call graphs, and optional or future work SHALL NOT create semantic edges. Reverse requirement relationships and active architecture-constraint backlinks SHALL remain derived from their respective canonical forward markers.

Each requirement SHALL have a local semantic fingerprint that is a formatting-stable digest of its local normative prose and scenarios, excluding intrinsic and relationship marker comments and non-semantic formatting. After validating that the requirement semantic graph is acyclic, the tool SHALL compute each effective semantic/review fingerprint prerequisite-first as a deterministic digest of the local semantic fingerprint, sorted outgoing requirement `(relation kind, target ID)` pairs and each target's effective semantic/review fingerprint, and sorted active architecture constraint `(invariant ID, invariant local fingerprint)` inputs. Reviewed state SHALL store and compare the effective semantic/review fingerprint. Formatting-only changes and changes only to review outcome or reason SHALL preserve semantic fingerprints. A local semantic, outgoing requirement-edge, active invariant, or active constraint-edge change SHALL stale each directly affected requirement and every transitive requirement dependent effective fingerprint until each is explicitly reviewed; unrelated requirements SHALL remain unchanged.

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
- **WHEN** required forward `requires` or `refines` markers are present and structurally valid in current canonical specifications
- **THEN** canonical requirement extraction produces stable typed forward edges, derived backlinks, capability aggregation, and owner-before-dependent reading order without consulting roadmap, handoff, archive, implementation, test, or generated prose.

#### Scenario: Relationship markers are invalid
- **WHEN** a requirement relationship target is unknown or non-current, a source targets itself, a pair is duplicated or assigned both kinds, markers are misplaced, or the requirement graph contains a cycle
- **THEN** extraction or readiness fails with stable source-target or cycle diagnostics.

#### Scenario: An owner changes
- **WHEN** a requirement's local semantics or outgoing requirement relationships change, or one of its active architecture constraint inputs changes
- **THEN** its effective semantic/review fingerprint and every transitive requirement dependent effective fingerprint change, while unrelated requirements remain unchanged, and readiness fails until every stale requirement is individually reviewed.

#### Scenario: Review metadata or formatting changes
- **WHEN** only Markdown formatting, review outcome, or review reason changes
- **THEN** local and dependent effective semantic/review fingerprints remain unchanged.
