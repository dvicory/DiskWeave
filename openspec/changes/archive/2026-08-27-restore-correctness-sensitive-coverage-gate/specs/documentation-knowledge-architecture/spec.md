## MODIFIED Requirements

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

## ADDED Requirements

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
