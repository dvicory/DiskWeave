## 1. Durable authority and configuration

- [x] 1.1 Add the standalone accepted documentation-system architecture and decision records without referring to the bootstrap handoff for meaning
- [x] 1.2 Add versioned source, canonical-requirement-anchor, concept, curriculum, page/block, glossary, prompt, and schema configuration for the initial Guide and self-documentation pages
- [x] 1.3 Add the mdBook-compatible book root, entry point, accepted block-marked pages, and initial provenance fixtures

## 2. Deterministic extraction and planning

- [x] 2.1 Add the bounded Markdown source extractor and Cargo metadata extractor with stable IDs, authority/lifecycle/privacy metadata, selectors, and semantic digests
- [x] 2.2 Add deterministic curriculum/page planning with source coverage, prerequisite-cycle detection, and generated `SUMMARY.md`/indexes
- [x] 2.3 Add versioned inventory, page-plan, coverage, and block-state serialization with atomic file writes and path containment checks

## 3. Context and preservation protocol

- [x] 3.1 Add bounded selector-based ContextPack generation with omission reporting and quoted-source metadata
- [x] 3.2 Add schema-constrained queue task emission for missing or stale blocks
- [x] 3.3 Add atomic external-response validation and `KEEP` application while preserving accepted Markdown bytes; reject unsupported or out-of-scope responses

## 4. Command surface and checks

- [x] 4.1 Add the development-only `xtask` package, `cargo xtask docs ...` alias, and deterministic help/schema/doctor commands without modifying the production `dwv` CLI
- [x] 4.2 Add extract, plan, context, queue, apply, check, build, bootstrap, explain-stale, test, and self-check operations with model-free behavior
- [x] 4.3 Add explicit bootstrap/recovery behavior for missing generated outputs and fail-closed behavior for missing provenance or retired-handoff references

## 5. Regression and evidence

- [x] 5.1 Add unit and integration fixtures for formatting-only stability, semantic stale detection, `KEEP`, bounds, unknown selectors, path traversal, unsafe Markdown, and prompt-like source data
- [x] 5.2 Add xtask CLI and offline acceptance evidence for extraction, planning, context, queue/apply, deterministic build, and self-check
- [x] 5.3 Update the dependency/status work program and document the next curriculum/coverage capability without claiming later provider, mdBook, scenario, or retirement gates
- [x] 5.4 Run focused and workspace verification, strict OpenSpec validation, and leave this change archive-ready only after all scope boundaries remain truthful
