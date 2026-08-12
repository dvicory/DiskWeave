## Context

See `proposal.md` for motivation. The knowledge implementation currently extracts only current OpenSpec requirements into `KnowledgeModel`, hashes normalized requirement units, derives inverse `requires`/`refines` views, and stores reviewed local/effective requirement fingerprints. Roadmap handling separately scans for exactly one active marker and validates planning identifiers. The v0.8/v0.7/v0.6 roadmap files already use a leading scalar YAML metadata block, but invariant-level identity does not exist and v0.8 must remain unchanged.

The change crosses extraction, lifecycle validation, effective fingerprints, fixed-purpose context packets, CLI schemas, and clean-room behavior. A design is required because a local parser choice can otherwise create candidate leakage or duplicate semantic authority.

## Goals / Non-Goals

**Goals:**

- Extend the existing knowledge model with the minimum architecture-specific records needed for marked invariants.
- Preserve one current requirement graph and reviewed-requirement workflow.
- Make active constraint changes reuse current effective-fingerprint dependent propagation.
- Keep candidate comparison and historical retrieval explicit, deterministic, bounded, and read-only.
- Keep the accepted v0.8 valid and byte-identical with an empty invariant set.

**Non-Goals:**

- No generalized semantic-node graph, registry, or database.
- No v0.9 content, product semantics, promotion mutation command, or automatic conflict resolution.
- No architecture identities for unmarked prose or historical v0.8 content.
- No architecture review-outcome store; affected current `req.*` owners remain the individually reviewed units.
- No Sphinx page for candidate or historical semantics.

## Decisions

### Discover roadmap documents only under `docs/architecture`

Walk `docs/architecture` recursively, reject symlinks, and parse only Markdown files whose leading metadata declares `kind: architecture-roadmap`. This includes the current top-level roadmap and archived revisions while excluding test source, temp fixtures outside the selected root, ADRs, verification records, and ordinary architecture notes. Tests use `App::new(temp_root)` and generate synthetic documents only under that isolated root.

Reuse the current one-MiB authority-file bound. Parse required scalar metadata directly instead of adding a YAML dependency: the existing documents use simple `key: value` fields and no architecture lifecycle field needs YAML collections. Reject duplicate required keys and invalid scalar values. Treat `scope: whole-system` and one shared `series` as the supported DiskWeave model; preserve unrelated metadata such as `title` without giving it semantic meaning.

### Use existing lifecycle names and active marker

The exact metadata contract is:

```yaml
---
id: arch.test.v2
series: arch.test
kind: architecture-roadmap
revision: v2
status: active | candidate | superseded
scope: whole-system
supersedes: arch.test.v1 # omit only when no predecessor applies
---
```

An active document contains exactly one line:

```text
<!-- dwv:active-architecture-roadmap -->
```

Candidate and superseded documents contain none. The sole active marker identifies the active document and its series. Static validation requires every discovered whole-system roadmap to use that series and requires exactly one active document and marker, so a promotion is one coherent source edit: mark the successor active and add the marker while marking the predecessor superseded and removing its marker. A different whole-system series is rejected rather than silently treated as another independently active architecture. The tool does not mutate architecture sources.

When present, document supersession names the direct predecessor. Present targets are checked for same-series coherence and cycles; missing targets are allowed because retained revisions may outlive their predecessors. The accepted v0.6 document therefore remains valid without a `supersedes` field.

### Extract only explicitly marked invariant sections

The exact unit syntax is:

```markdown
### Invariant: Present bytes remain evidence-bounded
<!-- dwv:arch-invariant arch.test.foo -->
<!-- dwv:constrains req.cap.owner -->
<!-- dwv:arch-supersedes arch.test.old -->

Normative invariant body.
```

`Invariant:` headings without a marker remain ordinary prose. A marker must be the first non-blank line after the heading; relationship and lineage markers must be contiguous before body prose. The unit ends at the next heading of equal or higher level. Multiple `constrains` and `arch-supersedes` markers are allowed. Identity validation reuses the lowercase component rules with an `arch.` prefix and additionally rejects collision with any document ID.

Normalize the complete marked section with the existing Markdown normalizer. HTML comments are already excluded, so identity, relationship, and lineage markers stay outside the local digest without a second prose representation. The heading and normative body remain inside the digest. Source path, line range, front matter, and lifecycle are outside it.

### Keep architecture records specialized

Add `ArchitectureDocument`, `ArchitectureInvariant`, `ArchitectureConstraint`, and `ArchitectureCatalog` records inside `xtask/src/knowledge.rs`. Do not generalize `RequirementObject` or `SemanticRelationship` into polymorphic graph nodes.

The catalog retains all valid lifecycle revisions for explicit candidate/history lookup and exposes one active invariant slice for current knowledge construction. `RequirementObject` gains only derived `constrained_by: Vec<String>`.

### Feed active architecture only into requirement effective fingerprints

For each current requirement, derive a sorted set of active `(invariant_id, invariant_local_fingerprint)` constraints. Encode that set beside the requirement's existing local fingerprint and imported `requires`/`refines` effective fingerprints. This gives the desired behavior without a second review engine:

- active invariant body change changes direct target effective fingerprints;
- active edge add/remove changes the old/new target effective fingerprints;
- existing recursive requirement hashing changes transitive dependents;
- reviewed local fingerprints remain unchanged, so readiness reports `semantic-prerequisite-changed`;
- unrelated requirements remain unchanged;
- candidate and historical catalog entries are never inputs.

Invariant lineage is not included in this digest.

### Expose one stored direction and derived inverse

Store only invariant `constrains` targets from the architecture source. Populate each requirement's `constrained_by` from active edges and return it in inspect/ownership/context relationship views. Current exports and context include the active invariant unit and stored edges; they do not add an inverse source marker.

### Add two fixed-purpose explicit operations

Add:

```text
cargo xtask docs knowledge architecture-candidate <document-id>
cargo xtask docs knowledge architecture-history <document-id>
```

Candidate output compares the selected candidate with the active document by invariant identity. For each identity in the union—including an invariant with zero `constrains` targets—it returns `unchanged`, `changed`, `added`, or `removed`, fingerprints, added/removed constraints, explicit lineage, direct affected current requirements, and the transitive current dependent closure. `added` means only that the identity is absent from the active invariant set; it makes no claim that the meaning is historically new. The output is labeled `non-authoritative-candidate` and never writes review state.

History output returns only the selected superseded document's marked units, stored constraints, and stored/derived lineage, labeled `historical`. It does not compare or satisfy current semantics.

Both operations serialize complete results and use `max_context_bytes`; invariant extraction uses the existing source-unit byte bound. No field-selection or partial-unit option is added.

The existing `req.documentation-knowledge-architecture.agent-knowledge-workflows-use-one-complete-retrieval-path` owner already delegates concrete command choreography to trusted skill instructions. Update `.agents/skills/diskweave-knowledge/SKILL.md` under that stable owner: ordinary work keeps using current requirement/context commands, candidate and history operations require explicit non-authoritative intent, candidate impact identifies review scope rather than mandatory edits, and architecture prose does not imply requirement changes. Do not encode a full architecture-adoption sequence before the first real reconciliation supplies evidence.

### Include only relevant active units in current packets

For a current `context` or `audit-context` requirement component, select active invariants whose stored constraints target any included requirement. Return each invariant once in a separate `architecture_invariants` array and return its stored edges in `architecture_constraints`. `inspect` and `ownership` expose derived `constrained_by` identities but do not repeat unrelated invariant bodies. Existing packet-size enforcement covers the new arrays.

### Keep lineage separate from dependency

Each `dwv:arch-supersedes` marker is an edge between distinct invariant identities found in different revisions of the same series. One successor with several predecessors is a merge; several successors may name one predecessor as a split. Validate unknown, self, duplicate, cross-series, and cyclic edges. Candidate/history outputs derive `superseded_by`; requirement context, effective fingerprints, reading order, and review propagation ignore lineage.

### Version changed JSON contracts

Bump object, inspect, context, audit-context, ownership, and readiness schema versions where fields or fingerprint meaning change. Add separate candidate/history schema identifiers. Existing requirement IDs, reviewed-state file schema, and command semantics remain intact.

## Semantic ownership reconciliation

| Current requirements | Semantic rule | Selected owner | New requirement role | Contradiction resolved | Semantic edit classification | Stable-ID consequence | Scenario and evidence migration |
|---|---|---|---|---|---|---|---|
| `historical-architecture-is-opt-in-only`; `active-roadmap-is-explicit-and-non-authoritative` | Roadmap lifecycle, selection, authority, and ordinary-surface exclusion | The two existing requirements retain lifecycle-boundary ownership | The lifecycle and invariant requirements require these boundaries and establish the sole marked-active-unit exception | Absolute roadmap exclusion is narrowed; unmarked, candidate, and historical prose remains excluded | semantic correction/change | Preserve both IDs: their durable subjects remain historical isolation and active selection/authority | Existing marker and isolation scenarios remain; add real v0.8 and marked-unit isolation fixtures |
| `agent-context-is-graph-selected-and-reproducible` | Complete bounded current requirement packet | Existing agent-context requirement | `architecture-context-and-lineage-are-bounded-and-authority-labeled` uses `requires`, not `refines`, because it adds architecture-specific units and explicit operations rather than narrowing the same detailed packet policy | Absolute roadmap-prose exclusion is narrowed only for directly linked complete active invariant units | semantic correction/change | Preserve the ID: the general context contract remains its subject | Existing context bounds remain; extend focused packet and inverse-view fixtures |
| `canonical-semantic-relationships-are-colocated-and-derived` | Requirement `requires`/`refines` authority and effective review fingerprints | Existing relationship requirement | `architecture-constraints-propagate-review-without-owning-product-policy` requires the existing graph/review mechanism and contributes a separate architecture review input | Roadmaps remain unable to create requirement dependency edges; marked active constraints become an explicit additional fingerprint input | semantic broadening | Preserve the ID: requirement relations and effective review fingerprints remain its subject | Existing graph-cycle tests remain; add constraint validation and propagation fixtures |

No changed owner delegates detailed product policy to architecture. Canonical `req.*` requirements remain the only detailed current product owners.

The clean-room owner is also modified because active invariant units become permanent current inputs. Clean-room reconstruction copies only the selected active roadmap into the temporary repository, omitting candidate and archived revisions. Extraction ignores its unmarked prose. This preserves equivalent current output without restoring historical architecture as an input. A focused temp-tree regression uses the production clean-room input-selection helper to compare active invariant identity and fingerprint, stored and derived constraint facts, requirement effective fingerprints, current context, and readiness before and after reconstruction while proving candidate and historical sources are absent.

## Risks / Trade-offs

- The scalar front-matter parser deliberately supports only the syntax the architecture lifecycle needs. This avoids a dependency but rejects YAML features such as arrays or quoted multiline values for these fields. The exact syntax is documented and test-covered.
- Adding active architecture to effective fingerprints changes every constrained requirement at activation, as intended. With v0.8 unmarked, the real repository has no initial fingerprint churn.
- A malformed candidate can fail structural documentation validation even though candidate semantics never create current review suspects. This is intentional: lifecycle isolation does not permit invalid source to masquerade as a valid preview.
- Historical constraint targets may no longer be current. History returns them as labeled historical facts and excludes them from current dangling-reference gates.
- Static promotion validation cannot make two file edits physically atomic. It guarantees that every observable repository state accepted by the knowledge tool contains exactly one active revision; normal review/CI rejects intermediate states.
