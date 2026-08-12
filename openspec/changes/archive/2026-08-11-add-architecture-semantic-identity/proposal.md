## Why

DiskWeave can identify and propagate changes among canonical `req.*` requirements, but it cannot identify selected enduring architecture invariants or deterministically preview which requirement owners a future architecture revision would affect. A narrow extension is needed before any v0.9 candidate is normalized so candidate and historical architecture remain non-authoritative and isolated from current implementation context.

## What Changes

- Extract explicitly marked `arch.*` invariant units from source-local architecture roadmap revisions without requiring markers in the accepted v0.8 roadmap.
- Validate roadmap document identity, the single whole-system series and scope, revision, active/candidate/superseded lifecycle, active marker, and document supersession metadata.
- Compute format- and location-stable local semantic fingerprints for marked invariant units.
- Add one source-local `architecture invariant -> constrains -> current req.*` relationship and derive `constrained_by` views without making architecture a detailed product owner.
- Include active invariant fingerprints and constraint-edge sets in current requirement effective review fingerprints so existing dependent propagation and individual review resolution remain authoritative.
- Add bounded explicit candidate-impact and historical-architecture queries while excluding candidate and historical semantics from current readiness and ordinary context.
- Add source-local invariant supersession lineage for replacement, split, and merge cases without treating lineage as semantic equivalence or a product dependency.
- Include only complete active invariant units relevant to a current requirement packet; never concatenate an architecture document.
- Narrow the existing roadmap, context, and requirement-fingerprint exclusions only enough to admit marked active invariant constraints; unmarked, candidate, and historical architecture remains excluded.
- Retain only the selected active roadmap in clean-room reconstruction because marked active invariants become permanent current inputs; candidate and historical roadmaps remain absent.
- Add isolated synthetic temp-tree tests, including current-semantic reconstruction with a marked active invariant. Do not modify v0.8 or introduce v0.9 product semantics.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `documentation-knowledge-architecture`: Add stable architecture-invariant identity, lifecycle isolation, semantic fingerprints, constraint impact, bounded retrieval, and lineage behavior to the existing knowledge spine.

## Impact

- `xtask/src/knowledge.rs`: architecture discovery, validation, fingerprints, active propagation, bounded packets, candidate preview, history, lineage, and fixture evidence.
- `xtask/src/lib.rs`: fixed-purpose candidate/history command dispatch and schemas plus clean-room current-input selection.
- `.agents/skills/diskweave-knowledge/SKILL.md`: concise command selection and authority guidance for ordinary, candidate, and historical retrieval.
- `openspec/specs/documentation-knowledge-architecture/spec.md` after archive: canonical behavior for the extension.
- Generated knowledge JSON schema versions and tests change; product crates and behavior do not.
- The accepted v0.8 roadmap remains byte-identical and valid with no invariant markers.
