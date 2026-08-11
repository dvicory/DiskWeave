## Context

See `proposal.md` for motivation. The current `context` and `ownership` commands are aliases over one packet builder. That builder makes every semantic edge undirected, traverses the selected requirement's complete connected component, and embeds the selected canonical body in both results. Current trusted skills can then request `inspect`, `ownership`, and `context` for the same ID.

The graph remains authoritative only for explicit `requires` and `refines` markers in current canonical specs. Requirement IDs and fingerprints must remain independent of source line movement. `affected` already owns deterministic downstream change impact, and readiness is already compact.

Measured against seven requirements used during recovery planning, the three-command workflow emitted 118,667 JSON characters and 29,195 `o200k_base` tokens. An initial modeled batched projection emitted 26,780 characters and 6,605 tokens, but that projection omitted canonical bodies for graph-selected prerequisites and dependents and therefore was not semantically sufficient. The corrected real `context.v3` command emitted 57,516 stdout characters and a 52,555-byte compact result with 22 reading-order IDs and 22 complete canonical units, a 51.5% character reduction from the repeated workflow. Compression and semantic sufficiency are independent acceptance results.

## Goals / Non-Goals

**Goals:**

- Make one command sufficient for each knowledge retrieval intent.
- Keep ordinary context complete under a deterministic schema without caller-selected omission controls.
- Preserve the complete canonical unit for every requirement in the ordinary semantic scope and audit component, plus semantic identity, fingerprints, review facts, endpoint references, bounds, omissions, and historical isolation.
- Make multi-requirement work cheaper through de-duplication of repeated or unrelated material rather than semantic truncation.
- Keep downstream change-impact review explicit through `affected`.

**Non-Goals:**

- Changing semantic relationship meaning, fingerprint construction, reviewed-state storage, readiness, or documentation authority.
- Adding a stateful continuation protocol, field projection language, token budget, model-specific serializer, cache authority, or duplicate registry.
- Treating line ranges, reading order, implementation links, or evidence links as semantic authority.
- Optimizing generated human projections or product runtime behavior.

## Decisions

### Use fixed command contracts instead of caller-selected projections

`inspect`, `ownership`, `context`, `affected`, and `audit-context` receive fixed result contracts matching distinct tasks:

- `inspect`: one exact local canonical unit and direct semantic identity facts, protected by the canonical source-unit bound;
- `ownership`: direct graph, review, and endpoint facts without body or transitive order;
- `context`: complete bounded task semantics for one or more selected IDs;
- `affected`: deterministic downstream change-impact closure;
- `audit-context`: explicit single-seed complete connected-component traversal with every component member's canonical unit.

The CLI will not add field, depth, direction, or closure-selection options. An intelligent caller cannot know whether an omitted correctness field matters before seeing it; a complete bounded default avoids defensive `--full` calls and duplicate follow-up packets.

Alternative considered: progressive `--fields` and graph-depth controls. Rejected because the measured seven-ID model rose from 6,605 tokens for one complete packet to 10,615 tokens when a minimal packet required a full follow-up, while also creating omission risk and a larger bespoke CLI.

### Select ordinary context through directed semantic meaning

Semantic edges point from a dependent requirement to an independently owned prerequisite. For selected set `S`, ordinary context computes:

```text
P = transitive outgoing requires/refines closure of S
D = direct incoming required_by/refined_by neighbors of S
K = P union S union D
```

The reading order is a stable topological order of the induced directed graph over `K`, owner before dependent. Every requirement in `P`, `S`, and `D` contributes its complete normalized canonical unit; the one-hop dependent boundary controls expansion without downgrading those dependents to semantically incomplete records. Direct relationship records remain de-duplicated. `affected` remains the required operation after an owner changes and returns its complete downstream closure. `audit-context` retains undirected connected-component traversal for explicit full-audit work and includes the complete canonical unit of every returned component member.

Alternative considered: retain connected-component traversal for safety. Rejected because one relationship through a composer can pull unrelated sibling branches into every packet, making consequential facts harder to find. The fixed directed selection plus mandatory change-boundary `affected` preserves the needed safety boundary without treating ordinary task context as impact evidence.

### Batch selected IDs before serialization

`context` accepts one or more IDs. It resolves all IDs before output, rejects any unknown ID without returning a partial packet, and emits each complete canonical unit in the combined semantic scope, semantic edge, selected review fact, selected typed reference, bound, omission, and reading-order entry once in stable order. The byte bound applies to the complete combined packet. On failure, the command constructs the complete singleton packet for every selected ID and suggests those singleton requests only when all of them fit. Shared canonical units may repeat across the suggested requests; this error path favors simple, verifiably complete fallback over packing optimization. If any singleton packet is oversized, no split is suggested. `audit-context` accepts exactly one ID and reports its connected component as indivisible when oversized.

Alternative considered: concatenate existing single-ID packets. Rejected because it repeats bodies, graph components, references, schema envelopes, and review facts.

### Make bounded reference loss attributable

Reference categories retain one global item cap per packet; this avoids a second allocation policy and preserves stable ordering. Before truncation, the packet counts references by selected requirement ID and category. The result reports the exact omitted count for every selected ID/category after the global cut, so a caller can see when one ID consumed the shared category limit and another lost all references.

Alternative considered: reserve an equal quota per selected ID. Rejected because quotas waste available slots and make ordering depend on selection cardinality; attributable omissions expose the same loss without another fairness policy.

### Project agent-specific packet views from one derived model

The extractor continues to build one internal requirement and relationship model. Command serializers project distinct result schemas from that model; they do not create separate semantic indexes. Shared helpers may derive source locators, direct edges, typed references, directed context closure, affected closure, and audit closure, but command-specific packet assembly remains explicit so the contracts cannot silently converge again.

Each breaking result shape receives a new explicit schema identity. Existing overlapping schemas and aliases are removed in the same cutover; no compatibility shim is retained during early development. Help and schema output describe the fixed command purposes, multi-ID context syntax, and single-ID audit syntax.

### Replace redundant source metadata with one current locator

Agent packets represent a canonical source location as one string:

```text
openspec/specs/<capability>/spec.md:<start>-<end>
```

The extractor computes the range from the current requirement heading through the line before the next same-or-higher heading. Packet projections replace redundant `source_path` plus `heading_path` fields with this locator where the locator is exposed. Intrinsic ID and semantic fingerprints remain derived independently of line numbers; moving a requirement without semantic change changes only the locator.

Alternative considered: add `start_line` and `end_line` beside existing source and heading fields. Rejected because the measured additive form increased output by 1–2% and retained redundant data.

### Give source-unit and packet bounds distinct purposes

Canonical extraction rejects any normalized requirement unit whose source exceeds `max_source_bytes`; `inspect` therefore returns one complete unit or fails before projection. Context-bearing results additionally enforce `max_context_bytes` across the complete serialized packet. Neither bound permits truncating canonical semantics: the source-unit bound protects local extraction, while the packet bound protects combined retrieval.

### Update trusted workflows in the same cutover

The canonical workflow requirement owns only the durable invariant: trusted instructions choose a fixed-purpose retrieval whose canonical semantics are sufficient for the task and avoid redundant overlapping retrieval by default. `.agents/skills/diskweave-knowledge/SKILL.md` owns the current readiness-once and batched-context recipe, while `.agents/skills/diskweave-semantic-reconciliation/SKILL.md` owns its current initial-context and post-change `affected` recipe. Tool choreography may evolve without changing the canonical invariant; neither skill may compensate for an incomplete packet with routine overlapping retrieval.

## Semantic Ownership Reconciliation

| Current requirements | Semantic rule | Selected owner | Other requirements | Contradiction resolved | Semantic edit classification | Stable-ID consequence | Scenario / evidence migration |
|---|---|---|---|---|---|---|---|
| `req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible` | Agent-facing command contracts, semantic sufficiency of each returned scope, batching, bounds, and historical isolation | `req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible` | `canonical-semantic-relationships-are-colocated-and-derived` continues to own edge meaning, backlinks, and topological derivation; `provenance-is-claim-and-fragment-level` owns exact path/line provenance; `change-boundary-impact-is-deterministic-and-bounded` owns complete downstream impact | The first implementation labeled graph-selected prerequisites, dependents, and audit members as necessary context while omitting their canonical semantics | semantic correction/change | Preserve the ID: identity-selected reproducible agent context remains its subject, now with semantic completeness defined for every included scope member | Add prerequisite/audit semantic-sufficiency tests, per-ID omission and split-refusal tests, and fresh-agent acceptance evidence |
| `req.documentation-knowledge-architecture.agent-knowledge-workflows-use-one-complete-retrieval-path` | Trusted workflows choose a sufficient fixed-purpose retrieval and avoid redundant overlap by default | `req.documentation-knowledge-architecture.agent-knowledge-workflows-use-one-complete-retrieval-path` | Requires the agent-context owner; the two trusted skills own current readiness, context, ownership, and affected choreography | Current canonical wording makes today's exact tool recipe permanent instead of owning the durable non-overlap and sufficiency invariant | semantic narrowing | Preserve the ID: fixed-purpose non-overlapping retrieval remains its material subject while procedural choreography moves to skill instructions | Update both skill recipes, narrow canonical scenarios, and review the workflow requirement against the corrected context owner |

The agent-context owner continues to require canonical relationship derivation and provenance; those owners remain unchanged. Its workflow dependent must be individually reviewed because the owner's effective semantics change. No other requirement duplicates packet completeness or workflow retrieval discipline.

## Risks / Trade-offs

- **[Risk] Ordinary context omits indirect dependents that later become affected by an owner change.** → Ordinary scope stays limited to direct dependents, and trusted workflows use `affected` after a semantic or outgoing-edge change.
- **[Risk] Complete canonical units increase packet size.** → Remove only duplicate or unrelated material; fail closed when one necessary scope exceeds the packet bound.
- **[Risk] A source locator becomes stale after a concurrent edit.** → The locator is generated from the same current extraction as the fingerprint and remains lookup data, never identity or authority; a subsequent command regenerates it.
- **[Risk] Split guidance suggests an incomplete or ineffective fallback.** → Serialize every selected ID's complete singleton packet and suggest all singleton requests only when they all fit; do not optimize or merge requests in this failure path.
- **[Risk] Breaking schemas disrupt fixtures or downstream scripts.** → Update CLI schemas, help, skills, fixtures, and verification evidence atomically; retain no ambiguous aliases.
- **[Trade-off] Audit work requires one explicit command per connected component and may fail for a large component.** → A single seed identifies the component, keeps the bound rule unambiguous, and never labels an incomplete component as audit context.

## Migration Plan

1. Keep the existing source-range extraction and distinct command projections.
2. Project complete canonical units for every ordinary semantic-scope member and every audit-component member.
3. Add per-selected-ID reference omission counts, verified singleton split guidance, and a single-seed audit contract, then cut over context and audit schema identities.
4. Narrow the canonical workflow invariant while retaining the current concrete recipes in both trusted skills.
5. Add focused semantic-sufficiency tests and a fresh-agent readiness-plus-context acceptance exercise.
6. Run OpenSpec validation, focused and workspace tests, real packet checks, reviewed-state reconciliation, knowledge readiness, docs check/build, and clean-room reconstruction.

No persistent data migration is required. Rollback restores the prior serializers, command dispatch, tests, and skill instructions together; canonical specs, reviewed state, and generated-state reconstructibility remain unchanged.
