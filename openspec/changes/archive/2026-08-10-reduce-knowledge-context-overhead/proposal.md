## Why

The current agent knowledge workflow repeatedly returns the same canonical unit and expands one selected requirement to an entire connected semantic component. In the measured seven-requirement recovery-planning case, `inspect` plus `ownership` plus `context` emitted 118,667 JSON characters (29,195 `o200k_base` tokens), obscuring useful facts without adding semantic authority. Compression is valid only when it removes duplicate or unrelated context; every requirement included as necessary semantic context must retain its canonical semantics.

## What Changes

- Give `inspect`, `ownership`, `context`, and `affected` distinct, complete purposes so normal workflows do not retrieve the same requirement through overlapping commands.
- Replace whole-connected-component context expansion with a deterministic owner-before-dependent packet containing complete canonical units for transitive semantic prerequisites, selected requirements, and direct dependents; retain full downstream impact in `affected` and reserve complete whole-component semantics for an explicit single-seed audit command.
- Allow one `context` request to select multiple requirement IDs and de-duplicate requirements, relationships, review facts, references, bounds, omissions, and reading order while reporting reference omissions by selected requirement and category.
- Replace redundant packet source metadata with one exact current source locator containing path and requirement line range.
- On an oversized multi-ID context request, suggest one complete singleton request per selected ID only when every singleton packet fits; otherwise report that the required scope cannot be split without omission.
- Keep the canonical workflow requirement durable: trusted workflows use a sufficient fixed-purpose retrieval path and avoid overlapping retrieval by default, while skills own the current readiness/context/affected recipe.
- Keep context packets semantically complete without progressive field, depth, direction, or closure-selection options; retain the source-unit bound for `inspect` and the complete-packet byte bound for context-bearing results.
- **BREAKING**: revise the structured knowledge packet schemas and command result shapes; early development does not retain aliases for the overlapping packet contracts.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `documentation-knowledge-architecture`: refine agent-context semantic sufficiency, command responsibilities, batching, deterministic closure, source and packet bounds, actionable omissions, safe split guidance, and durable workflow invariants without changing canonical requirement identity or semantic relationship authority.

## Impact

- Affects `cargo xtask docs knowledge` command schemas, graph selection, serialization, tests, and help/schema output.
- Affects the DiskWeave knowledge and semantic-reconciliation skills that prescribe command usage.
- Preserves current OpenSpecs, reviewed-state storage, relationship markers, fingerprints, readiness behavior, historical isolation, and change-boundary `affected` semantics.
- Adds no runtime dependency, persistent semantic registry, progressive retrieval protocol, or product-code behavior.
