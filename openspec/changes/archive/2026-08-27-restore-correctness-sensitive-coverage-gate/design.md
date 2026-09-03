## Context

The current canonical documentation-knowledge architecture derives current requirement identities, fingerprints, typed references, reviewed state, and bounded export/context packets. The current discovery requirement also carries a scenario that reports an uncovered requirement and makes `docs check` fail; that scenario currently overlaps the restored coverage policy and must be narrowed to discovery/identity. `xtask/src/knowledge.rs` scans Rust under `crates/` and `xtask/`, scans model/evidence/documentation inputs, and makes readiness fail for missing fingerprints, stale fingerprints, orphaned reviewed state, unknown references, and planning-path violations. It does not evaluate target-level meaningful ownership, the full evidence contract, endpoint-set freshness, or explicit target dispositions. `docs/reviewed-requirements.toml` stores requirement fingerprints and review outcomes/reasons, but it does not yet persist the typed endpoint set that those reviews covered.

The archived open-traceable change and `docs/verification/open-traceable-knowledge.md` establish historical intent for sparse meaningful owners and artifact-owned evidence. They are useful evidence for reconciliation only. The current `openspec/specs/*/spec.md` set and its explicitly delegated model surfaces remain the only current semantic authority. See `proposal.md` for motivation and the current/proposed boundary.

## Goals / Non-Goals

**Goals:**

- Leave discovery/identity owned by its existing requirement and make the new coverage requirement the only owner of coverage, target dispositions, endpoint-set freshness, and gate outcomes.
- Evaluate implementation and evidence independently so one target’s valid absence/deferment cannot hide omission or contradiction in the other target.
- Require the complete evidence-boundaries contract, including deterministic bounded verification facts, before an evidence endpoint counts.
- Persist a deterministic digest of the typed endpoint set semantically reviewed for each target in the existing reviewed-requirement entry, including repository-relative locators so moves stale review.
- Make deferred triggers machine-evaluable and fail closed when their current repository condition fires.
- Expose target states, endpoint-set digests, dispositions, omissions, and next actions through bounded `knowledge ownership`, export, readiness, and `docs check` results.
- Establish one atomic currentization/backfill/self-coverage cutover after pre-cutover verification and the Rust-root prerequisite, so the gate is never current while its own requirement or the current set is ownerless.

**Non-Goals:**

- Editing current canonical specs, product code, Rust markers, evidence records, models, Beads, or maintained pages in this planning change.
- Defining the repository root set, `src/`/`tests/` traversal, CodeLinks path mapping, or duplicate-traversal rules; those belong to `dwv-6c0.2`.
- Requiring a marker on every Rust helper, test helper, or evidence-adjacent file; performing whole-program call-graph analysis; or inferring ownership from filenames, proximity, or call sites.
- Making source markers, evidence artifacts, generated export, reviewed state, non-delegated models, or historical records semantic authority for product behavior.
- Creating a second requirement/classification registry, a free-standing coverage registry, a sidecar that copies canonical prose, a permanent usefulness corpus, or generic evidence taxonomy.
- Executing downstream `.3` audit/repair work in this change; that work starts only after `.1` closes.

## Decisions

### 1. Separate discovery ownership from coverage ownership

The delta MODIFIES `req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry` so its added-requirement scenario reports only current extraction and identity. It no longer owns “uncovered,” `docs check`, owner/evidence, or disposition consequences.

The durable coverage owner is the new requirement:

`req.documentation-knowledge-architecture.correctness-sensitive-requirements-have-meaningful-coverage-or-explicit-disposition`

The archived requirement named “Sparse trace links resolve to real implementation and evidence endpoints,” but that archived delta has no stable current `req.*` identity to preserve. The discovery identity is narrower and remains stable. Adding this one new identity is the smallest ownership repair; it is not a semantic identity migration.

The ownership table is:

| Decision | Canonical owner | Coverage gate relationship |
| --- | --- | --- |
| Current requirement identity and extraction | `canonical-requirements-are-discovered-without-a-duplicate-semantic-registry` | prerequisite; the gate consumes its current extracted set |
| Requirement fingerprint propagation and review freshness | `canonical-semantic-relationships-are-colocated-and-derived` | prerequisite; its effective fingerprint contract is included in coverage context and review propagation |
| Product behavior and exact delegated model semantics | each product requirement and only its explicitly delegated model surface | endpoints point to these owners but do not redefine them |
| Evidence input, mechanism/invariant, outcome, tier, boundary, scope, non-claims, deterministic replay, and boundedness | `evidence-boundaries.evidence-scope-is-explicit`, `unknown-and-ambiguous-evidence-fail-closed`, and `verification-artifacts-are-deterministic-and-bounded` | prerequisites; coverage counts only complete, reproducible evidence facts |
| Coverage targets, endpoint-set review, dispositions, and export/ownership/readiness/check outcomes | the new coverage requirement | sole owner; no other requirement defines this gate |

Every current extracted `Requirement` is in coverage scope by default. No severity list, roadmap list, lexical classifier, or marker presence decides inclusion. A documentation-only or unsupported requirement remains visible and must have target coverage or a target-specific disposition.

### 2. Evaluate implementation and evidence as independent targets

The evaluator produces two target records per current requirement:

| Target | Satisfying relationship | Valid disposition |
| --- | --- | --- |
| `implementation` | one or more current Rust production semantic scopes that materially realize the requirement | `not-applicable` or `deferred` implementation record |
| `evidence` | one or more complete artifact-owned test/model/scenario/evidence relationships | `not-applicable` or `deferred` evidence record |

There is no `both` target. If both targets are absent, two records are required. Aggregate precedence is deterministic and conservative: `invalid` if any target is invalid, fired, stale, or malformed; otherwise `missing` if any target is missing; otherwise `dispositioned` if every non-covered target has a valid disposition; otherwise `covered` only when both targets are covered. Thus invalid outranks missing, missing outranks dispositioned, and dispositioned outranks covered. A covered evidence target can coexist with a dispositioned implementation target, and vice versa, without a requirement-level contradiction. A disposition can contradict only coverage for its own target.

A meaningful Rust owner is a current `dwv:req` marker attached to a real semantic module, type, implementation, or operation boundary that materially realizes a requirement predicate, state transition, authority boundary, or externally observable operation. A helper can qualify only when it is itself that boundary; a nearby helper or re-export does not count. Scope association and current existence are machine-checked; an individual requirement review records why the scope is semantically material. No marker is required on every helper or test, and no whole-program call graph is introduced.

A counted evidence relationship must satisfy the complete `evidence-boundaries` contract: executable input, mechanism or invariant exercised, observed outcome, evidence tier, closest unsupported claim boundary, bounded scope, and explicit non-claims. It must also satisfy `verification-artifacts-are-deterministic-and-bounded`: bounded inputs and outputs, stable artifact identity and digest, and reproducible replay or inspection with a comparable semantic outcome and evidence digest. The artifact must explicitly name the current requirement and resolve to a current test, executable model, executable scenario, or evidence artifact. A model is exact semantic authority only within the current requirement’s explicit delegated surface; otherwise it is evidence input. The marker, artifact, and generated projection remain non-authoritative.

### 3. Persist the reviewed endpoint set in existing reviewed state

The existing `docs/reviewed-requirements.toml` remains the only durable review-state surface. Extend each current requirement’s reviewed entry with one deterministic endpoint-set digest per target:

```text
coverage_endpoint_digests[requirement_id].implementation = digest
coverage_endpoint_digests[requirement_id].evidence = digest
```

A target disposition stores the digest of its reviewed empty endpoint set. The digest input is the sorted normalized set of typed endpoint identity, endpoint class, repository-relative locator/path, current scope, and applicable claim boundary. Including the repository-relative locator/path makes a source or evidence move change the digest even when its identity and requirement fingerprints remain unchanged. The digest contains no requirement prose, payload, credential, private path, or generated output. Existing local/effective fingerprints and existing outcome/reason fields remain the semantic review record; this digest closes the separate gap where an endpoint can change without changing requirement text.

A current endpoint-set digest mismatch makes the target stale/invalid even when local/effective requirement fingerprints remain equal. Adding, removing, moving, retargeting, reclassifying, or changing the claim boundary of an endpoint requires individual target review. This is a digest of reviewed facts, not a second source of truth: current endpoints still derive from current source/artifact inputs, and the canonical requirement still defines meaning.

`knowledge ownership` must expose both target states, current/reviewed digests, typed endpoint locations, dispositions, target-scoped omissions, and next actions within existing packet bounds. It must preserve existing omission accounting and fail closed rather than truncate a correctness target.

### 4. Make dispositions target-specific and machine-triggered

A disposition entry is keyed by `(current requirement identity, target)` and contains:

```text
requirement_id        current req.* identity
target                implementation | evidence
kind                  not-applicable | deferred
scope                 bounded subject and whole target boundary
non_claims            explicit claims not established for that target
reason                concise explanation
review_trigger        owner-present | evidence-present | fingerprint-changed
local_fingerprint     current requirement fingerprint
effective_fingerprint current effective requirement fingerprint
endpoint_set_digest   reviewed empty/current target set digest
```

The trigger value, not free-text explanation, is enforced:

- `owner-present` applies to an implementation target and fires when a current meaningful Rust owner appears;
- `evidence-present` applies to an evidence target and fires when a complete applicable evidence relationship appears;
- `fingerprint-changed` applies to either target and fires when the stored local/effective fingerprint no longer matches.

The evaluator recomputes those conditions from current canonical extraction, current endpoint sets, and current fingerprints. A fired deferred or not-applicable record becomes invalid with `deferred-trigger-fired` until it is individually resolved or removed; it never silently adopts the new owner/evidence. `not-applicable` is also reviewable if the named target later becomes applicable. Scope, non-claims, reason, trigger, fingerprints, and endpoint digest must all be current and bounded. A target disposition does not affect an independently valid other target.

### 5. Enforce current-authority lifecycle

The evaluator consumes one repository snapshot but enforces the lifecycle that snapshot must represent:

1. The modified discovery requirement extracts a current identity from the canonical OpenSpec set.
2. The coverage requirement evaluates both target relationships against that current set and the current Rust-root model.
3. Current typed endpoints are checked for scope/artifact existence, complete evidence facts, and deterministic replay/digest facts.
4. Existing reviewed state compares local/effective fingerprints and per-target endpoint-set digests including repository-relative locators/paths.
5. Export, ownership, readiness, and `docs check` publish only the current derived result.

A source or artifact naming an active-change, archived, historical-only, removed, or unknown identity is an invalid current relationship, not pending coverage. A deleted/moved scope, missing artifact, changed endpoint set, stale digest, fired trigger, or fingerprint mismatch fails closed. Canonical split, merge, removal, or supersession never silently transfers endpoint or disposition state. No disposition can suppress malformed or non-current links.

### 6. Verify before cutover, then currentize atomically

`dwv-6c0.1` depends on `dwv-6c0.2`; no coverage implementation or backfill may assume that root `src/` and `tests/` are visible until the shared root model is complete. While the change remains proposed, complete focused implementation/spec verification, artifact-bound checks, and fixture review. Do not run live current-readiness or `docs check` as proof of the new requirement before synchronization; those commands cannot establish a current result for a proposed requirement.

After pre-cutover verification passes, the `.1` cutover has one coherent current snapshot containing all of the following:

1. the reviewed-state schema migration for target dispositions and endpoint-set digests;
2. the modified discovery requirement and new coverage requirement synchronized into current canonical specs;
3. gate activation and ownership output support;
4. self-coverage for the new coverage requirement using a real knowledge implementation scope and complete deterministic evidence, or current per-target dispositions if a target truly does not apply;
5. complete current backfill for every canonical requirement target with real owner/evidence relationships or independently validated dispositions; and
6. live ownership, export, readiness, and `docs check` results from that same coherent snapshot.

No intermediate state is treated as current authority. Downstream Bead work that audits or repairs the now-current link set starts only after `.1` closes and is outside this change’s execution. This prevents the gate from becoming current while its own requirement or existing requirements are ownerless and keeps downstream work from becoming a prerequisite for initial validity.

### 7. Define observable outcomes without transferring authority

`knowledge ownership` reports target-level `covered`, `dispositioned`, `missing`, or `invalid` states, current/reviewed endpoint-set digests, endpoint locations, dispositions, omissions, and next actions. `knowledge export` reports the same target records plus one aggregate record per current requirement, applying invalid-over-missing-over-dispositioned-over-covered precedence, and refuses partial publication for structural/lifecycle errors. `knowledge readiness` fails for missing/invalid targets, stale digests, stale fingerprints, or fired triggers; valid target dispositions remain visible non-coverage. `docs check` includes this gate alongside change-boundary impact and keeps unavailable baselines explicit.

Until apply, verification, and sync, current canonical OpenSpecs and explicitly delegated model surfaces remain authoritative and this delta is only proposed. After sync, the new requirement owns coverage/disposition policy; product requirements own product behavior, the discovery requirement owns extraction/identity, the relationship requirement owns fingerprint propagation, evidence-boundaries owns evidence claims and deterministic verification, and source/evidence/model/projection artifacts report or support those meanings only.

## Risks / Trade-offs

- Treating every current requirement as in scope creates a visible target-level disposition workload for documentation-only or not-yet-implemented requirements. That cost is intentional: it prevents an unreviewed classification list from silently excluding correctness-sensitive obligations.
- Persisting endpoint-set digests adds a small reviewed-state field, but without it a moved or retargeted endpoint can remain “reviewed” when requirement text is unchanged. Including repository-relative locators makes moves observable without creating a second registry.
- Machine triggers prevent permanent deferrals but make the cutover and focused fixtures stricter. A fired trigger invalidates rather than auto-accepts, preserving human review at the authority boundary.
- A marker can be structurally valid yet semantically too broad or too narrow. Scope association plus individual review is safer than whole-program analysis; completion evidence must include accepted semantic owners and rejected helper-only cases.
- The full evidence and deterministic-artifact contract can leave a source-only test, unobserved model, unstable digest, or non-replayable fixture invalid. That is intentional: an artifact must establish the claim boundary and reproducibility it is used to cover.
- The gate cannot define complete Rust root visibility. `dwv-6c0.2` must establish the root model first; invisible roots are unresolved input, not an accepted disposition.
