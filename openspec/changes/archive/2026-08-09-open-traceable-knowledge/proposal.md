## Why

The current bespoke documentation CMS preserves canonical authority and deterministic checks, but its registries, projection logic, and whole-source context are harder to maintain than the documentation they produce. DiskWeave needs a sparse, checked requirement-to-code/evidence/documentation spine built on maintained open-source tools so autonomous agents can maintain useful ordinary Markdown without creating another semantic authority.

## What Changes

- Replace the custom mdBook/page-plan/block-state projection system with pinned MyST, Sphinx, Sphinx-Needs, sphinx-codelinks, and a maintained Rust/Sphinx integration.
- Export canonical OpenSpec requirements as non-authoritative Sphinx-Needs objects with stable intrinsic IDs, semantic fingerprints, and reversible Sphinx IDs.
- Add sparse checked links from requirements to Rust semantic owners, tests, executable scenarios, evidence, ADRs, and bounded Markdown claims.
- Add compact reviewed-link state whose requirement fingerprint changes make downstream relationships suspect until explicitly resolved.
- Add deterministic `knowledge` inspection/context commands and a single change-readiness gate for autonomous agents.
- Replace generated/meta documentation with readable checked-in MyST Guide, Scenario, Assurance, Contributor, and Architecture Reference sources backed by the trace graph.
- Demonstrate human and agent usefulness through one-time observed acceptance exercises, without adding a permanent evaluator, corpus, scoring schema, or result registry.
- **BREAKING**: retire mdBook and the custom projection CMS after Sphinx parity, clean-room reconstruction, and migration evidence pass.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `documentation-knowledge-architecture`: Replace the custom projection CMS with the open traceability spine, Sphinx/MyST projections, suspect-link lifecycle, AI-maintained Markdown workflow, and agent-operability/readiness contract.

## Impact

- Documentation tooling and configuration under `docs/`, `xtask`, and the repository-local agent skill change substantially.
- Canonical OpenSpec requirement headings gain intrinsic IDs; selected Rust semantic owners gain non-runtime CodeLinks markers.
- Generated inventories, Sphinx interchange, context packets, and rendered output remain ignored build artifacts.
- Production storage crates gain no Sphinx, Python, model, or runtime traceability dependency.
- Existing documentation state, mdBook configuration, block markers, prompt schemas, and projection code are removed after migration evidence passes.
