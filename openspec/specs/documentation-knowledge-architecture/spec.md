# Documentation Knowledge Architecture

## Purpose

Provide a durable, model-independent documentation/context subsystem in which canonical OpenSpecs and a small cross-cutting constitution are the only current semantic authority, while human, evidence, contributor, reference, and agent surfaces remain reproducible projections.

## Requirements

### Requirement: Canonical requirements are discovered without a duplicate semantic registry
<!-- dwv:req req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry -->

The documentation tool SHALL discover every current `openspec/specs/*/spec.md` automatically, extract each `Requirement` as a stable semantic unit, and expose an intrinsic requirement identity that is independent of roadmap nodes, verification identifiers, numbered planning identifiers, historical source paths, archive paths, line numbers, and generated state. A requirement identity SHALL remain stable when surrounding Markdown is reordered or reformatted, and a changed or missing identity SHALL produce an explicit diagnostic.

#### Scenario: A canonical requirement is added
- **WHEN** a new `Requirement` appears in a current canonical spec
- **THEN** extraction succeeds and exposes its intrinsic current identity to the separate coverage requirement; discovery SHALL not infer an owner, evidence relationship, disposition, or readiness result.

#### Scenario: Formatting and ordering change
- **WHEN** a canonical spec changes only Markdown formatting or reorders unrelated requirements
- **THEN** requirement identities and dependent accepted prose remain unchanged.

#### Scenario: A milestone contains requirement-like prose
- **WHEN** an explicitly selected historical source outside the current canonical source set contains headings, normative language, or current requirement identifiers
- **THEN** canonical extraction ignores that record and derives current requirements only from `openspec/specs/*/spec.md`.

### Requirement: Correctness-sensitive requirements have meaningful coverage or explicit disposition
<!-- dwv:req req.documentation-knowledge-architecture.correctness-sensitive-requirements-have-meaningful-coverage-or-explicit-disposition -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->
<!-- dwv:requires req.evidence-boundaries.evidence-scope-is-explicit -->
<!-- dwv:requires req.evidence-boundaries.unknown-and-ambiguous-evidence-fail-closed -->
<!-- dwv:requires req.evidence-boundaries.verification-artifacts-are-deterministic-and-bounded -->

The coverage gate SHALL classify every `Requirement` extracted from the current canonical `openspec/specs/*/spec.md` set as in scope by default. It SHALL NOT use a second hand-maintained requirement registry, a roadmap list, a lexical severity heuristic, generated state, or a marker on every implementation helper to decide which requirements are correctness-sensitive. A requirement MAY leave the covered state only through the explicit validated target disposition defined below; that disposition is an exception for the current requirement identity and target, not a classification registry.

For each in-scope current requirement, the gate SHALL evaluate implementation and evidence as independent targets. The implementation target SHALL have at least one meaningful implementation owner unless it has a valid implementation disposition. The evidence target SHALL have at least one applicable verification relationship unless it has a valid evidence disposition. An aggregate requirement SHALL be `covered` only when both targets are covered; it SHALL be `dispositioned` only when every target not covered has its own valid disposition and no target is missing or invalid. Coverage or disposition for one target SHALL not excuse omission or invalidity of the other target.

A meaningful implementation owner is a `dwv:req` marker attached to a current Rust production scope that materially realizes a requirement boundary, such as a semantic module, type, implementation, or operation. A marker SHALL count only when it resolves to an existing current Rust scope and the scope itself represents the requirement's behavior, state transition, authority boundary, or externally observable operation. A marker on an unrelated helper, re-export, generated file, fixture-only configuration, or evidence-only record SHALL not count merely because the file is nearby. The gate SHALL not require markers on every helper or every test. Rust source markers, extracted endpoint facts, evidence records, and projections SHALL remain non-authoritative relationships and SHALL add no runtime behavior.

An applicable verification relationship SHALL be artifact-owned and complete under the evidence-boundaries contracts. It SHALL identify the executable input, the mechanism or invariant exercised, the observed outcome, the evidence tier, the closest unsupported claim boundary, bounded scope, and explicit non-claims. For executable evidence, it SHALL also have bounded inputs and outputs, a stable artifact identity and evidence digest, and reproducible replay or inspection behavior whose semantic outcome and digest can be compared. It SHALL explicitly name the current requirement and resolve to a current test, executable model, executable scenario, or evidence artifact. A model MAY be an exact semantic owner only for the bounded surface that its current canonical requirement explicitly delegates; otherwise it is verification input. A model marker or model result SHALL not silently become a product semantic owner. Shared tests, models, scenarios, or evidence SHALL count only when their artifact-owned relationship explicitly names the current requirement. Artifact records SHALL retain these facts without copying canonical requirement prose or creating a requirement registry.

When a target is genuinely absent or intentionally deferred, the existing durable reviewed-requirement entry SHALL allow one exception record keyed by the pair `(current requirement identity, target)`. `target` SHALL be exactly `implementation` or `evidence`; a requirement missing both targets SHALL use two records. Each record SHALL contain:

- `kind`: `not-applicable` when that target does not apply within the declared scope, or `deferred` when that target is intentionally absent for now;
- a non-empty bounded `scope` covering the whole unsatisfied target boundary;
- non-empty explicit `non_claims` stating what the repository does not claim for that target;
- a concise reason;
- a machine-evaluable `review_trigger` object; and
- the current local and effective requirement fingerprints against which that target disposition was reviewed.

The trigger object SHALL use one of the bounded conditions `owner-present`, `evidence-present`, or `fingerprint-changed`. `owner-present` SHALL be valid for an implementation target, `evidence-present` SHALL be valid for an evidence target, and `fingerprint-changed` SHALL be valid for either target. The evaluator SHALL recompute the condition from current canonical extraction, current typed endpoints, and current fingerprints. A deferred or not-applicable record whose trigger fires SHALL become invalid with an explicit `deferred-trigger-fired` diagnostic until it is individually reviewed or removed; explanatory trigger prose SHALL not be the enforcing value. A valid target disposition SHALL make that target `dispositioned`, never `covered`, and SHALL not manufacture an endpoint.

The existing reviewed-requirement entry SHALL persist one deterministic endpoint-set digest for each target that was semantically reviewed, including an empty-set digest for a target disposition. The digest SHALL be computed from the sorted typed current endpoint identities, endpoint classes, repository-relative locators/paths, scopes, and applicable claim boundaries; a source or artifact move SHALL therefore change the digest even when its identity and requirement fingerprints remain unchanged. It SHALL contain no canonical requirement prose, payload, credentials, private path, or generated output. The existing review outcome and reason SHALL remain associated with the entry. A changed current endpoint set, identity, class, repository-relative locator/path, scope, or claim boundary SHALL make that target stale/invalid even when the requirement's local and effective fingerprints are unchanged. Unknown or non-current IDs, unsupported target/kind/trigger values, empty or unbounded fields, private payloads, stale fingerprints, stale endpoint-set digests, or a disposition that contradicts coverage for the same target SHALL fail closed. A disposition for one target SHALL not contradict or suppress valid coverage for the other target.

Coverage relationships and dispositions SHALL obey current-authority ordering. The requirement identity SHALL first exist in the current canonical OpenSpec view; only then may a source, model, test, scenario, evidence record, or disposition resolve it. A marker or artifact naming an active-change, archived, historical-only, removed, or otherwise non-current identity SHALL fail closed. Removing or moving the marked Rust scope, deleting or moving the evidence endpoint, changing an evidence identity, changing a typed endpoint set, or changing the requirement's reviewed fingerprint SHALL produce an explicit stale/deleted/changed diagnostic; the gate SHALL not infer a replacement owner or silently grandfather the old relationship. An identity split, merge, removal, or supersession SHALL require explicit current canonical migration and individual endpoint/disposition review before coverage is current again.

`knowledge ownership` SHALL expose target-level implementation and evidence states, current and reviewed endpoint-set digests, typed endpoint locations, dispositions including scope, non-claims, reason, and trigger, bounded omissions, and any stale/invalid diagnostic without expanding beyond its existing packet bounds. `knowledge export` SHALL derive one bounded aggregate coverage record and the two target records for every current requirement, distinguishing aggregate `covered`, `dispositioned`, `missing`, and `invalid` states. Aggregate precedence SHALL be deterministic: `invalid` if any target is invalid, fired, stale, or malformed; otherwise `missing` if any target is missing; otherwise `dispositioned` if every non-covered target has a valid disposition; otherwise `covered` only when both targets are covered. It SHALL report missing owner/evidence coverage rather than presenting an ownerless requirement as complete. Unknown, non-current, stale, deleted, or malformed relationships SHALL fail export before publishing a partial current projection. Exported relationship facts SHALL be labeled as current derived projections and SHALL not become semantic authority.

`knowledge readiness` SHALL fail closed for every missing or invalid target, stale endpoint-set digest, stale disposition, or fired deferred trigger, while allowing valid target dispositions to satisfy the absence gate without counting them as coverage. Its result SHALL report covered and dispositioned target/aggregate counts separately and identify each exact requirement, target, endpoint, digest, disposition, and next action. `docs check` SHALL include the same target-level coverage gate alongside change-boundary impact and SHALL remain non-ready when an omission, lifecycle violation, endpoint-set change, trigger transition, or unresolved endpoint exists. A ready result SHALL mean that every current requirement target is meaningfully covered or explicitly and currently dispositioned; it SHALL never mean that a dispositioned target has implementation or evidence proof.

#### Scenario: A canonical requirement is discovered

- **WHEN** canonical extraction discovers a current `Requirement`
- **THEN** the discovery requirement exposes its intrinsic identity and the coverage gate creates implementation and evidence target evaluations without consulting a separate coverage list.

#### Scenario: A meaningful Rust owner is present

- **WHEN** a current `dwv:req` marker resolves directly to a Rust semantic scope that materially realizes the selected requirement
- **THEN** the implementation target is classified as covered after its endpoint set is individually reviewed, while unmarked helpers and unrelated nearby scopes remain outside the ownership relationship.

#### Scenario: A helper marker is not semantic ownership

- **WHEN** a marker is attached only to a helper, re-export, generated file, or evidence-only configuration that does not represent the requirement boundary
- **THEN** the implementation target remains missing and the gate does not require markers on the other helpers.

#### Scenario: An evidence relationship satisfies the full contract

- **WHEN** a current test, executable model, executable scenario, or evidence artifact explicitly names the current requirement and records executable input, exercised mechanism or invariant, observed outcome, evidence tier, closest unsupported claim boundary, bounded scope, non-claims, bounded inputs and outputs, stable identity and digest, and reproducible replay or inspection facts
- **THEN** the evidence target is classified as covered without transferring semantic authority from the canonical requirement or its explicitly delegated model.

#### Scenario: An evidence fact is incomplete

- **WHEN** an artifact lacks any required executable input, exercised mechanism or invariant, observed outcome, evidence tier, closest unsupported claim boundary, scope, non-claims, bounded input/output, stable identity/digest, or reproducible replay/inspection fact
- **THEN** the evidence target is invalid and cannot satisfy coverage or be suppressed by a disposition.

#### Scenario: One target is covered and the other is independently deferred

- **WHEN** a requirement has covered evidence and a current-fingerprint `deferred` implementation disposition with bounded scope, non-claims, reason, and an `owner-present` trigger
- **THEN** the evidence target remains covered, the implementation target is dispositioned, and the aggregate is dispositioned rather than missing or covered.

#### Scenario: Invalid takes precedence over missing

- **WHEN** one target is invalid because its endpoint digest or evidence facts are stale and the other target is missing without coverage or disposition
- **THEN** the aggregate state is `invalid`, readiness identifies both target diagnostics, and the missing target is not allowed to mask the stronger invalid state.

#### Scenario: A deferred trigger fires

- **WHEN** a deferred implementation target uses `owner-present` and a current meaningful owner appears, or a deferred evidence target uses `evidence-present` and applicable evidence appears
- **THEN** that target disposition becomes invalid with `deferred-trigger-fired` until individually reviewed; the evaluator does not auto-adopt the new endpoint or silently retain the waiver.

#### Scenario: An endpoint set changes without a requirement change

- **WHEN** a Rust owner scope, evidence identity, repository-relative locator/path, scope, endpoint class, or claim boundary changes while the requirement fingerprints remain unchanged
- **THEN** the current endpoint-set digest differs from the reviewed digest, the affected target becomes stale/invalid, and readiness requires individual review.

#### Scenario: A disposition is incomplete or stale

- **WHEN** a target disposition omits scope, non-claims, reason, trigger, or current fingerprints, targets both at once, contradicts coverage for the same target, or stores stale fingerprints/digests
- **THEN** that target is invalid and readiness and `docs check` fail closed without affecting a valid independently reviewed other target.

#### Scenario: Ownership output reports target state

- **WHEN** an agent requests ownership for a current requirement with covered, dispositioned, missing, invalid, or stale targets
- **THEN** the bounded result reports both target states, current/reviewed endpoint-set digests, typed endpoints or disposition details, omissions, and next actions without silently collapsing disposition into coverage.

#### Scenario: A source names a non-current requirement

- **WHEN** a Rust, model, test, scenario, evidence, or documentation marker names an unknown, proposed, archived, historical-only, or removed requirement identity
- **THEN** extraction/export reports the marker location and non-current identity and no current target coverage or disposition satisfies the error.

#### Scenario: A linked scope or artifact is deleted

- **WHEN** a previously linked Rust scope or evidence endpoint no longer exists or no longer resolves to the declared current endpoint
- **THEN** the affected target is stale or deleted, export does not publish it as current, and readiness and `docs check` fail until the relationship is removed, repaired, or explicitly re-reviewed against a current endpoint.

#### Scenario: A link is created before canonical currentization

- **WHEN** an implementation or evidence relationship is added for an identity that exists only in an active change or archive and is later expected to become current
- **THEN** the relationship fails current extraction and cannot satisfy a target until the canonical requirement is current and the relationship is individually reviewed.

#### Scenario: The coverage owner is currentized atomically

- **WHEN** the modified discovery requirement, new coverage requirement, reviewed-state schema, self-coverage relationships, and every current target coverage/disposition are published in one coherent current snapshot
- **THEN** the gate evaluates the complete current set without an intermediate state that is simultaneously authoritative and ownerless; live readiness and `docs check` may then establish the current cutover result.

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

### Requirement: Architecture revision lifecycle is explicit and isolated
<!-- dwv:req req.documentation-knowledge-architecture.architecture-revision-lifecycle-is-explicit-and-isolated -->
<!-- dwv:requires req.documentation-knowledge-architecture.active-roadmap-is-explicit-and-non-authoritative -->
<!-- dwv:requires req.documentation-knowledge-architecture.historical-architecture-is-opt-in-only -->

Each architecture roadmap in DiskWeave's one whole-system architecture set SHALL carry one leading YAML metadata block with scalar `id`, `series`, `kind`, `revision`, `status`, and `scope` fields. `kind` SHALL be `architecture-roadmap`; `scope` SHALL be `whole-system`; and `status` SHALL be exactly `active`, `candidate`, or `superseded`. Every discovered whole-system architecture roadmap SHALL share one `series`. The sole `<!-- dwv:active-architecture-roadmap -->` marker SHALL identify the active document and thereby the supported series; exactly one document in that series SHALL have `status: active`. A `candidate` or `superseded` document SHALL contain no active marker. An optional scalar document `supersedes` field SHALL name a direct predecessor when applicable. A predecessor retained in the repository SHALL belong to the same series and document supersession among retained revisions SHALL be acyclic; a document MAY name a predecessor that is no longer retained.

Duplicate document identities, a different whole-system series or scope, zero or multiple active selections, active status/marker disagreement, invalid known document supersession, or a document-supersession cycle SHALL fail before architecture material participates in a packet.

Only active invariant units and active `constrains` relationships SHALL participate in current requirement fingerprints, readiness review propagation, current export, or ordinary current context. Candidate architecture SHALL be available only through `cargo xtask docs knowledge architecture-candidate <document-id>` and SHALL be labeled `non-authoritative-candidate`. Superseded architecture SHALL be available only through `cargo xtask docs knowledge architecture-history <document-id>` and SHALL be labeled `historical`. Candidate and historical semantic content SHALL NOT satisfy current requirement coverage, become a current prerequisite, change current fingerprints, create current review diagnostics, or enter ordinary implementation context.

Promotion SHALL be represented by one coherent repository state in which the successor document is `active` and carries the sole active marker while its predecessor is `superseded` and carries none. Validation SHALL reject intermediate or final states that expose both or neither as active. Promotion alone SHALL not change an invariant fingerprint.

#### Scenario: Candidate successor is present
- **WHEN** one valid candidate revision coexists with the valid active revision
- **THEN** current readiness and ordinary context use only the active revision while explicit candidate retrieval returns only marked candidate units with a non-authoritative label

#### Scenario: Superseded revision is requested
- **WHEN** a caller explicitly selects a valid superseded document identity
- **THEN** the bounded result labels every returned invariant historical and no returned unit satisfies current coverage or review state

#### Scenario: Lifecycle selection is ambiguous
- **WHEN** no active document exists, two documents are active, marker and status disagree, or a candidate or superseded document carries the active marker
- **THEN** validation fails before current architecture context is returned

#### Scenario: Successor is promoted coherently
- **WHEN** the successor is active with the sole marker and the predecessor is superseded without the marker
- **THEN** ordinary current queries select only the successor and no packet exposes both revisions as current

### Requirement: Architecture constraints propagate review without owning product policy
<!-- dwv:req req.documentation-knowledge-architecture.architecture-constraints-propagate-review-without-owning-product-policy -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->

A marked invariant MAY declare one or more source-local `<!-- dwv:constrains req.<identity> -->` markers immediately after its identity and before normative body content. `constrains` SHALL mean only that the invariant establishes a constitutional or cross-cutting boundary that the detailed canonical requirement must respect. It SHALL NOT mean that architecture duplicates the detailed requirement, that the requirement exhaustively realizes the invariant, that architecture automatically overrides a conflicting current OpenSpec, or that implementation conforms. The stored edge SHALL run only from invariant to requirement; `constrained_by` SHALL be a derived inverse view and SHALL NOT be persisted independently.

Active and candidate `constrains` targets SHALL name current canonical requirements. A malformed target, self-edge, duplicate edge, or dangling active/candidate target SHALL fail explicitly. Historical edges MAY remain queryable as historical facts when their former requirement target is no longer current, but SHALL not affect current validation or review. Changing an active invariant fingerprint, adding an active edge, or removing an active edge SHALL change each directly affected requirement's effective semantic/review fingerprint. Existing `requires` and `refines` propagation SHALL then change every transitive dependent effective fingerprint while unrelated requirements remain unchanged. Each affected requirement SHALL remain individually resolvable through the existing reviewed-requirement workflow; the tool SHALL report review scope and SHALL NOT infer implementation work or semantic conformance.

Candidate comparison SHALL report unchanged, changed, added, and removed invariant identities; added and removed constraint targets; directly affected current requirements; and their transitive `requires`/`refines` dependent closure. These classifications SHALL compare invariant identity presence and content only: `added` SHALL mean that the identity is absent from the active invariant set and SHALL NOT claim that the underlying architectural meaning is historically new. Candidate comparison SHALL be preview-only and SHALL NOT modify reviewed state, current fingerprints, readiness, or current context. A preview result SHALL allow a reviewer to conclude already conformant/no product change, wording clarification only, evidence gap only, deferred/not presently applicable, or actual semantic change required without encoding that conclusion as automatic work.

#### Scenario: Active invariant constrains a current requirement
- **WHEN** `arch.test.foo` contains `<!-- dwv:constrains req.cap.owner -->`
- **THEN** current requirement views derive `arch.test.foo` in `constrained_by` and store no inverse edge

#### Scenario: Active invariant semantics change
- **WHEN** the active local fingerprint for `arch.test.foo` changes while its constraint target remains `req.cap.owner`
- **THEN** `req.cap.owner` requires semantic-prerequisite review and every transitive `requires` or `refines` dependent requires review while unrelated requirements remain current

#### Scenario: Active constraint edge changes
- **WHEN** an active `constrains` edge is added or removed without changing the invariant local fingerprint
- **THEN** the directly added or removed target and its dependent closure require review

#### Scenario: Candidate semantics and edges change
- **WHEN** a candidate changes `arch.test.foo`, adds a constraint, or removes an active constraint
- **THEN** candidate impact reports the exact semantic and edge differences plus current dependent closure without changing current readiness or reviewed fingerprints

#### Scenario: Constraint target is invalid
- **WHEN** an active or candidate invariant names an unknown current `req.*` target or repeats one target
- **THEN** validation fails with the invariant, target, document, and source location

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

### Requirement: Guide curriculum is pedagogical intent, not semantic authority
<!-- dwv:req req.documentation-knowledge-architecture.guide-curriculum-is-pedagogical-intent-not-semantic-authority -->

Guide projection configuration SHALL describe audience, question, learning outcome, prerequisites, misconceptions, deferred concepts, canonical requirement/scenario/evidence references, and an ordered set of section briefs. Each section brief SHALL provide a proposed Markdown heading, focus, questions it must answer, applicable sources, and explicit non-claims. The briefs guide drafting and review but SHALL NOT copy canonical requirement text, define semantic authority, or become generated page state. Deterministic readiness SHALL reject missing or empty section-brief fields for DiskWeave Guide entries. The initial DiskWeave Guide SHALL use those briefs to form a causal journey in which readers can identify what happened, what is durable, what callers may believe, what recovery observes, what remains uncertain, and what DiskWeave refuses to infer.

#### Scenario: A DiskWeave Guide chapter is drafted

- **WHEN** an agent writes or revises a DiskWeave Guide chapter
- **THEN** it follows the entry's ordered section briefs, answers each required question from the linked current sources, preserves the stated non-claims, and uses maintained checked-in Markdown rather than generated prose.

### Requirement: Provenance is claim- and fragment-level
<!-- dwv:req req.documentation-knowledge-architecture.provenance-is-claim-and-fragment-level -->

Each maintained correctness-sensitive prose section SHALL name the exact current requirement identities and, where applicable, scenario or evidence identities that support it. Inspection SHALL report the current typed endpoint paths and lines for a supplied requirement ID. Referential checks SHALL reject unknown identities, but SHALL NOT be represented as proof that prose accurately explains a changed relationship. Diagnostic impact inspection SHALL return bounded current context, exact maintained pages to review, actions, and omitted-reference counts.

#### Scenario: A supported relationship is inspected
- **WHEN** a maintainer supplies a current requirement identity or changed artifact path
- **THEN** the tool reports the current canonical unit and typed Rust, verification, curriculum, and Markdown endpoint locations without asserting that the prose remains correct.


### Requirement: Change-boundary impact is deterministic and bounded
<!-- dwv:req req.documentation-knowledge-architecture.change-boundary-impact-is-deterministic-and-bounded -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->

At a change boundary, deterministic readiness SHALL compare current local and effective semantic/review fingerprints and current typed relationships with an available repository revision baseline through a provider-neutral interface. Local semantic changes, dependency-closure semantic-prerequisite changes, removed or reassigned relationships, and retired IDs SHALL identify the exact requirements requiring individual review. A changed current delegated canonical source identified by its source-local requirement marker SHALL also be treated as a semantic source change even when its marker set and extracted requirement IDs are unchanged; the marked owner and its transitive requirement-dependent closure SHALL require review. Such a path SHALL be classified as a delegated canonical source change rather than as an implementation-only or evidence-only edit. Formatting-only changes and implementation edits with unchanged relationships remain context rather than automatically creating documentation work. The command SHALL report when no baseline is available instead of claiming that relationships are unchanged, and diagnostics SHALL provide exact bounded inspection commands.

#### Scenario: A semantic prerequisite changes

- **WHEN** a target's effective semantic/review fingerprint changes or a source relationship is removed or reassigned
- **THEN** `docs check` reports every exact direct and transitive review-required dependent ID with `semantic-prerequisite-changed` diagnostics while unrelated requirements remain unchanged

#### Scenario: A delegated canonical source changes

- **WHEN** a current model or other executable semantic source carrying a current requirement marker changes while its marker set remains present
- **THEN** affected-path and change-impact classify the path as `delegated_canonical_source_changed`, require review of every marked current owner and its transitive dependent closure, and do not classify the edit as implementation-only evidence

#### Scenario: A delegated source has no current owner marker

- **WHEN** a model path changes without a current requirement marker or its prior marker is removed
- **THEN** the change remains bounded context or an explicit removed-reference diagnostic and does not invent a semantic owner from filename, implementation calls, or historical artifacts

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

### Requirement: Usefulness is sampled separately from correctness
<!-- dwv:req req.documentation-knowledge-architecture.usefulness-and-correctness-are-separate-checks -->

Deterministic checks SHALL validate identity, coverage, stale state, provenance, executable artifacts, graph consistency, safe rendering, no-op/preservation behavior, privacy, and bounds. A major documentation architecture change SHALL separately include observed human and agent exercises with concrete observations recorded in its completion evidence. The repository SHALL NOT maintain a reusable usefulness corpus, scoring schema, result registry, or evaluator command without a documented recurring regression that justifies the maintenance cost. Participant or model self-reports SHALL NOT count as acceptance evidence.

#### Scenario: Structural checks pass without observed exercises
- **WHEN** deterministic documentation and traceability checks pass but the change has not been exercised by its intended human and agent audiences
- **THEN** structural correctness is established while usefulness acceptance remains incomplete, without adding permanent evaluator machinery.

### Requirement: Reconstruction and historical isolation are inspectable
<!-- dwv:req req.documentation-knowledge-architecture.reconstruction-and-historical-isolation-are-inspectable -->

The tool SHALL provide a clean-room check that removes regenerable state, generated indexes, candidate and archived architecture roadmaps, and archived changes from a temporary copy; retains only the selected active architecture roadmap as the source of any explicitly marked current invariant units; reconstructs current projections from permanent current artifacts; and verifies equivalent semantic output. Completion evidence SHALL show that current generation and requirement context do not depend on any prior documentation implementation, candidate or archived source, unmarked roadmap prose as semantic input, or chat history.

#### Scenario: Regenerable and historical state is deleted
- **WHEN** `target/dwv-docs`, generated projection caches, candidate and archived architecture roadmaps, and archived changes are absent in a temporary copy that retains only the selected active roadmap
- **THEN** the documented extraction, planning, build, check, and reconstruction commands recreate current generated state and produce equivalent current facts and accepted pages from canonical requirements and any marked active architecture invariant units.

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
