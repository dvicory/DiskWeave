# ADR: Deterministic documentation pipeline and external-agent boundary

**Status:** Superseded by [Open traceable knowledge tooling](open-traceable-knowledge.md)
**Date:** 2026-08-08
**Scope:** Documentation and semantic-context tooling only
**Supersession:** The open-traceable-knowledge ADR retains the development-only `xtask` boundary and bounded offline checks, but removes the custom graph, generated `SUMMARY.md`, mdBook shell, task/response protocol, model policy, and bespoke projection state.


## Context

The repository has one root `dwv` binary and no existing documentation tooling package. The initial useful capability must work offline and must not create a framework-shaped crate or bind correctness to an LLM vendor, vector service, or renderer.

## Decisions

1. Keep documentation maintenance in a separate development-only `xtask` package exposed as `cargo xtask docs ...`. The production `dwv` CLI remains free of documentation maintenance commands and dependencies. A future `dwv docs` is allowed only for a demonstrated user-facing documentation access need.
2. Use TOML for the durable reader-intent curriculum and verification-owned manifest, and JSON for prompt/task/response contracts and generated outputs because the workspace already has `serde`, `serde_json`, and a TOML parser in the development tool.
3. Use a real CommonMark event parser in `xtask` for Markdown normalization. Selectors are stable semantic IDs plus structural selectors, never indefinite line-number coupling.
4. Use one explicit graph for documentation generation and implementation-agent context. Initial retrieval is selector and graph closure with hard bounds; embeddings require measured failure and a new ADR.
5. Keep accepted Markdown and machine-readable provenance under version control. Generate `SUMMARY.md`, source links, indexes, traces, tables, and coverage deterministically.
6. Treat mdBook as a replaceable renderer/preprocessor shell owned by the development tool. It cannot own source authority, curriculum, freshness, or product semantics.
7. Use schema-constrained external-agent task/response files before adding a provider SDK. Rust controls IDs, bounds, paths, and atomic application.
8. Bind long-lived documentation to architecture/canonical requirement anchors, durable `VP-*` assurance properties, and concrete evidence artifacts. `OS-*`, `VE-*`, goals, task IDs, active changes, and handoffs remain work/history identifiers only. Archiving work rebinds dependencies to its resulting permanent artifacts and should not change accepted prose.
9. Keep the exact `fresh_isolated_context` execution policy as a compiled, model-free default in `xtask`; prompt metadata under `docs/prompts/` is a durable contract, not a semantic registry.
10. Keep the stable boundary as bounded DocTask + execution policy -> external fresh model context -> structured DocResponse -> Rust validation/apply. The long-running coding agent orchestrates task JSON and application but never authors model responses; writers/reviewers receive only bounded task context; Rust never invokes a provider or model.
11. Require `cargo xtask docs apply <response.json>` to validate task/context digests and execution-policy provenance. `cargo xtask docs jobs` and `cargo xtask docs next` are the operational entry points for emitted fresh work. `cargo xtask docs queue --rebaseline` is an explicit one-time authorization; after its marker is consumed, normal queue/check returns to preservation-first anti-thrash behavior.
The policy and schemas are inspectable without chat history. No provider dependency, model SDK, credentials, or spawn-model path is introduced; missing sources remain bounded work rather than a guessed response.

## Consequences

- The initial implementation can be useful with no model credentials or network.
- A provider may be added later without changing task/response or freshness semantics.
- The development-only `xtask` owns tooling commands; storage crates and the production root CLI remain unaware of them.
- A pinned Cargo-managed mdBook renderer makes rendered-book acceptance reproducible; the renderer remains a replaceable development-tool boundary.

## Reversal evidence

Move the module to a dedicated crate when another binary, library, or preprocessor consumes the same types. Adopt a different config/parser/renderer only with dependency, license, boundedness, and migration evidence. Adopt richer retrieval only after a recorded selector/graph failure case.
