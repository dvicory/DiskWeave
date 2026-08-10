## Context

See `proposal.md` for motivation. Current canonical semantics already come only from `openspec/specs/*/spec.md`; milestone exclusion and the active-roadmap marker validator already exist. The repository still contains seven predecessor planning records, one superseded incomplete archive, one completed acceptance archive with predecessor naming, and broad textual/path references that must be migrated atomically.

The migration changes historical organization, not product behavior. Clean cutover is required: compatibility names, redirects, aliases, duplicate copies, and persistent mapping tables would keep the retired planning system alive and defeat the deterministic residual check.

## Goals / Non-Goals

**Goals:**

- Preserve one canonical owner for each documentation-system obligation.
- Move seven predecessor records one-to-one to `m1.md` through `m7.md` and keep their technical chronology intact.
- Make every retained project-planning reference use milestone terminology.
- Prove milestones cannot affect current canonical discovery, relationships, context, freshness, or clean-room output.
- Fail deterministically on any future reintroduction of the retired planning identifier family.
- Keep Linux acceptance evidence honest by regenerating it only after runtime scratch names change.

**Non-Goals:**

- Rewriting historical technical decisions to resemble the current authority model.
- Building a migration registry, generalized policy engine, or new documentation subsystem.
- Changing product runtime semantics or expanding Linux acceptance claims.

## Decisions

### 1. Semantic ownership reconciliation

The five existing requirements remain the owners of their direct subjects; their stable IDs are preserved.

| Requirement | Direct subject after cutover | Classification |
|---|---|---|
| `canonical-requirements-are-discovered-without-a-duplicate-semantic-registry` | current requirement discovery and path-independent identity | clarification; milestone identifiers become another excluded identity source |
| `historical-architecture-is-opt-in-only` | exclusion and explicit bounded archaeology for historical/planning sources | semantic broadening to name milestones and the active-roadmap selection seam |
| `agent-context-is-graph-selected-and-reproducible` | bounded current requirement packets | clarification that milestone/roadmap prose cannot enter ordinary packets |
| `reconstruction-and-historical-isolation-are-inspectable` | clean-room independence from regenerable and historical inputs | semantic broadening to remove milestones, archives, and roadmap inputs in the proof copy |
| `canonical-semantic-relationships-are-colocated-and-derived` | canonical forward-edge authority and derived graph views | clarification that milestones and other non-canonical sources cannot contribute edges |

A new `active-roadmap-is-explicit-and-non-authoritative` requirement is warranted. The direct-subject test identifies unique-marker discovery, deliberate planning selection, and explicit disagreement handling as one durable policy not wholly owned by the five requirements above. Deleting it would leave exact-one selection and roadmap/change identity checks as implementation without a current contract. Two implementations could satisfy historical-source exclusion while selecting zero, one, or several roadmaps. The new requirement therefore owns that distinct predicate and `requires` the historical-exclusion owner rather than duplicating it.

### 2. One-to-one moves, no compatibility layer

Move the seven retained records in chronological order to `docs/milestones/m1.md` through `m7.md`. Add the standard historical-planning banner to each. Rewrite only project-planning titles, sequence labels, status language, paths, and internal references; preserve technical claims and chronology.

Delete the complete superseded incomplete archive. Rename the completed acceptance archive to milestone-seven terminology and rewrite its retained planning references without changing checked task state, accepted specification delta, or evidence meaning.

Use direct filesystem moves and update every inbound reference in the same change. Do not leave source files, tombstones, redirects, symlinks, aliases, duplicate directories, or old-to-new maps.

### 3. Extend the existing readiness validator

Add one bounded retired-planning scan to the existing repository-native documentation readiness path rather than a new command or registry.

- Enumerate working-copy files through the existing revision-control seam so tracked files and current additions are checked consistently.
- Check path strings before opening files.
- Read bounded regular files; skip non-UTF-8 payloads rather than treating binary evidence as prose.
- Match only the retired numbered project-planning family, the retired acceptance archive identity, and retired Linux scratch identifiers. A bare ordinary English word is not an error.
- Report every offending path and, for text, its one-based line.
- Keep scanner negative fixtures in the scanner test module; construct fixture tokens so the repository scan does not exempt an entire source file.

This is deliberately a small validator extension, not a generic forbidden-word framework.

### 4. Milestones remain outside semantic inputs

Reuse the existing source-root allowlists and milestone-exclusion tests. Add focused regressions only where current tests do not cover one of these observable boundaries:

- requirement discovery and relationship extraction;
- current reference/context and ownership packets;
- affected-path and review-freshness analysis;
- clean-room reconstruction.

The active roadmap remains discoverable by its existing explicit marker validator. Ordinary context excludes it; planning obtains it deliberately by resolving the unique marker and labels it non-authoritative. Detected semantic disagreements are recorded in explicit reconciliation/change artifacts, never resolved by the validator.

### 5. Linux evidence is regenerated, not edited

Change VM, fixture-root, mountpoint, and guest-evidence names to the functional Linux acceptance names specified by the milestone. Update scripts and their focused tests together. Do not edit embedded scratch names in retained machine-generated acceptance JSON or traces; the final live run replaces those artifacts from the post-cutover source.

### 6. Self-sanitize the current milestone

After the moves and archive changes, replace m8's temporary migration-input block with a milestone-only completion record. Update work-package status from observed commands, state the active-roadmap requirement decision above, and preserve m8 only as non-authoritative planning/completion history.

## Risks / Trade-offs

- **Broad rename can alter technical history.** Limit edits to project-planning identifiers and links; review moved records separately from product code.
- **Residual scanner can over-match ordinary prose.** Match versioned planning tokens and exact retired path/name forms, not the bare word; retain positive and negative tests.
- **Deleting the incomplete archive removes unchecked planning notes.** This is intentional: accepted current specs and completed archives retain authority/rationale; preserving unchecked tasks would create misleading history.
- **Linux evidence becomes temporarily stale after naming changes.** Milestone completion stays blocked until a fresh live run regenerates evidence and traces.
- **New roadmap requirement overlaps historical exclusion if worded broadly.** Keep exact-one discovery, deliberate selection, and disagreement handling local; link to rather than restate the historical-exclusion policy.
