## Why

DiskWeave has already separated canonical OpenSpecs from historical planning, but retained numbered predecessor records, archived planning names, Linux scratch identifiers, and one superseded incomplete archive still expose the retired planning system throughout the repository. The cutover must make the milestone model complete and mechanically enforced without turning milestones or the active roadmap into a second source of current product semantics.

## What Changes

- **BREAKING** Move the seven retained predecessor planning records one-to-one to `docs/milestones/m1.md` through `m7.md`, rewrite their planning terminology and links, and leave no aliases, redirects, duplicate copies, or old-to-new crosswalk.
- Delete the superseded incomplete `2026-08-09-knowledge-normalization` archive and rename the completed milestone-seven acceptance archive while preserving its accepted technical delta, checked tasks, evidence meaning, and chronology.
- Rewrite every retained project-planning identifier in maintained prose, archives, prompts, tests, scripts, comments, paths, VM names, fixture paths, mountpoints, and generated-input references; preserve ordinary English and `Goals / Non-goals` uses.
- Extend the existing repository-native documentation check to reject retired planning identifiers outside its own bounded negative fixtures and report exact paths and text locations.
- Preserve and prove exclusion of `docs/milestones/**` from canonical requirement discovery, relationships, context, implementation/evidence links, affected-path analysis, freshness review, and clean-room reconstruction.
- Add a current `active-roadmap-is-explicit-and-non-authoritative` requirement. This is a genuine durable obligation rather than ceremonial milestone text: existing requirements exclude historical sources, while the already-implemented marker validator separately enforces exactly one deliberately discoverable active roadmap and numeric roadmap/change consistency.
- Clarify the five existing documentation-authority requirements without changing their stable identities: canonical requirements remain independent of planning identifiers; historical and milestone sources remain opt-in/non-authoritative; graph context and reconstruction remain independent of them; relationships remain authored only in current canonical specs.
- Use functional Linux acceptance scratch names and regenerate final Linux evidence later from the final post-cutover source; do not hand-edit retained machine evidence.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `documentation-knowledge-architecture`: make active-roadmap discovery/non-authority and complete milestone/historical isolation explicit while preserving existing stable requirement ownership.

## Impact

- Affected repository surfaces: `docs/`, archived OpenSpec changes, `.agents/` guidance, `xtask` validation, tests/fixtures, Linux acceptance scripts, and retained verification references.
- Canonical semantic impact: one new documentation-system requirement; existing documentation requirement IDs are preserved with clarification or semantic broadening only where historical/planning isolation becomes explicit.
- Runtime product semantics, persistent formats, parity/recovery algorithms, transaction selection, platform support, and durability claims do not change.
- Numeric roadmap identifiers are not consumed; this remains a descriptive unnumbered change.

## Non-goals

- Making architecture v0.8 or any milestone canonical product semantics.
- Adding aliases, redirects, symlinks, compatibility paths, or a planning-name registry.
- Selecting a production SQLite mode, retiring transaction candidates or macOS bridge feasibility, migrating trace schema compatibility, redesigning codecs, or decomposing large modules.
- Expanding Linux support, concurrency, device publication, deployment, physical durability, FUA, or power-loss claims.
