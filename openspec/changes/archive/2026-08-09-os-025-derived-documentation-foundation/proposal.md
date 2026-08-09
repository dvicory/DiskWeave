## Why

DiskWeave has authoritative architecture, OpenSpecs, executable evidence, and a portable demo, but no repository-owned way to turn those inputs into bounded, provenance-bearing explanatory documentation or agent context. This change establishes the smallest useful offline foundation now: one real Guide concept, one self-documentation concept, deterministic source extraction and page selection, and an external-agent task path that preserves accepted prose instead of regenerating it indiscriminately.

## What Changes

- Add a standalone documentation-system architecture and decision records without changing DiskWeave storage semantics.
- Add a separate development-only `xtask` package and `cargo xtask docs ...` command surface for schema, doctor, extraction, planning, bounded context, queue, check, build, bootstrap, and self-check operations; do not modify the production `dwv` CLI.
- Extract registered Markdown and Cargo metadata into stable source units with authority, lifecycle, privacy, location, and semantic digests using stable canonical requirement anchors rather than work IDs or line numbers.
- Bind long-lived Guide/self-documentation blocks to architecture decisions, canonical requirement anchors, durable assurance properties, and concrete evidence artifacts; keep `OS-*`, `VE-*`, goals, tasks, active changes, and handoffs as work/history metadata only.
- Generate deterministic source inventory, coverage, page-plan, `SUMMARY.md`, source links, and bounded context packets without an LLM or network access.
- Track accepted Markdown blocks and provenance sidecars; detect stale, invalid, missing, or uncovered state without rewriting clean prose.
- Emit schema-constrained external-agent tasks and validate/apply only bounded `KEEP` responses in this change; provider-backed generation remains a later concern.
- Add a compact derived Guide page and documentation-system page sourced from durable repository artifacts, plus offline regression fixtures for formatting-only changes, semantic changes, bounds, path safety, and prompt-like source text.
- Record the exact current scope: this change does not claim complete curriculum coverage, semantic assessment/writing, mdBook binary availability, Linux/macOS storage behavior, or a retired bootstrap handoff.

## Capabilities

### New Capabilities

- `derived-documentation-foundation`: Deterministic source inventory, curriculum-selected pages and blocks, provenance/freshness state, bounded context/task packets, and an offline self-check for the initial derived documentation capability.

### Modified Capabilities

<!-- No existing DiskWeave product capability requirements change. The new CLI is a tooling boundary over existing authoritative sources. -->
