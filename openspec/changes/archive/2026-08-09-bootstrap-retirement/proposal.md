## Why

The bootstrap handoff describes a substantially larger documentation system than the current two-page foundation exposes. The repository needs the handoff's promised Guide spine, contributor/scenario/assurance views, deterministic recovery checks, and an auditable path to handoffless operation before claiming completion.

## What Changes

- **BREAKING** Expand the curriculum from the foundation pages to a useful Guide, contributor, scenario, assurance, and operator/maintainer self-documentation set.
- Add durable source registrations, concept classifications, provenance/coverage indexes, and generated page content grounded in current repository evidence.
- Add deterministic recovery, stale-state, missing-output, and handoff-dependency checks to the development-only documentation tool.
- Add a bounded Markdown-parser disposition audit for the bootstrap handoff and a compact historical retirement record.
- Run a clean-room reconstruction with generated outputs and the bootstrap handoff absent, then remove the handoff from the active tree.

## Capabilities

### New Capabilities

- `bootstrap-retirement`: prove that the documentation subsystem operates from durable repository artifacts without the bootstrap handoff.

### Modified Capabilities

- None. This change extends tooling and derived documentation; DiskWeave product semantics, storage formats, and the production `dwv` CLI remain unchanged.
