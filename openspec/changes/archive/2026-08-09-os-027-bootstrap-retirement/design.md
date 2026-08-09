## Context

OS-025 implemented the deterministic source/context/provenance shell but intentionally selected only two small pages. The bootstrap handoff's completion standard requires a useful human book and a handoffless recovery proof. The current repository already has accepted product architecture, canonical OpenSpecs, evidence records, a Rust `xtask`, and a bounded Markdown parser dependency.

## Decisions

1. Extend the existing `xtask` model and command surface rather than adding a documentation framework or production-crate dependency.
2. Keep derived pages as checked-in Markdown with existing block markers. Add only source-grounded prose for capabilities represented by current specs, simulator evidence, demos, and verification records; blocked platform claims remain explicit non-claims.
3. Add source registrations and classifications for current architecture, canonical requirements, ADRs, evidence, simulator/replay records, service/verification boundaries, and workspace metadata. Keep `docs/handoffs/**` outside ordinary extraction.
4. Add recovery commands/tests around temporary roots. The normal CLI continues to operate on the workspace root; clean-room tests copy permanent inputs into an isolated directory and run the same deterministic operations.
5. Implement a bounded `bootstrap-audit` operation using `pulldown-cmark` heading/paragraph/code events. The audit emits section-level semantic units with digests, classifications, durable artifact IDs, and verification commands. It is an audit input, never a current source.
6. Keep the retirement record under `docs/state/bootstrap-retirements/` and exclude it from the source graph. The handoff is removed only after the audit and clean-room evidence exist. The record contains no handoff text.

## Data flow

```text
current sources + Cargo metadata
  -> source inventory + coverage
  -> curriculum/page plan
  -> checked-in derived pages + block state
  -> offline check/test

selected bootstrap handoff (audit only)
  -> parsed disposition inventory
  -> fresh-context review + clean-room drill
  -> retirement record
  -> handoff deletion
```

## Scope boundaries

This change does not add an LLM/provider SDK, mdBook preprocessor, vector retrieval, Linux/macOS bridge behavior, storage semantics, or a new production CLI. It does not claim rendered HTML acceptance when `mdbook` is unavailable. It does not convert every repository file into tutorial prose; reference-only and deferred classifications remain explicit.

## Verification

- Focused xtask tests cover source coverage, stale/missing provenance, recovery, parser safety, and disposition completeness.
- Workspace tests, formatting, Clippy, strict OpenSpec validation, and documentation commands run offline.
- A clean-room script/test removes generated outputs and the bootstrap handoff from an isolated copy, bootstraps, attests existing pages, and runs the full deterministic gate.
