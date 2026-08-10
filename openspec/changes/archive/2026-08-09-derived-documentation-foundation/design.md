## Context

The repository has one production `dwv` binary, no `xtask` package, no mdBook configuration, and no documentation graph or generated-state tooling. Existing Rust crates and OpenSpecs already provide authoritative product facts, while `docs/handoffs/` contains planning material that must not become current semantic input. The initial capability must be useful without a model provider, network, or a new daemon, and it must leave the portable storage architecture and production dependency graph untouched.

## Goals / Non-Goals

**Goals:**

- Add a separate development-only `xtask` package for documentation maintenance; expose it as `cargo xtask docs ...` through a repository Cargo alias. Do not add documentation dependencies or maintenance commands to the production `dwv` binary.
- Parse configured Markdown sources structurally with `pulldown-cmark`, use Cargo metadata for the workspace graph, and emit deterministic JSON artifacts.
- Make source authority, lifecycle, privacy, stable IDs, selectors, bounds, curriculum selection, block markers, and provenance explicit.
- Support deterministic extraction, planning, context, queue, check, build, bootstrap, schema, doctor, test, and self-check operations.
- Support an external-agent response path whose first applied decision is preservation-first `KEEP`; represent but do not auto-apply prose-changing decisions.
- Keep generated/accepted Markdown and provenance reviewable, recoverable, and independent of provider identity.

**Non-Goals:**

- No model SDK, HTTP client, network access, embeddings, vector database, or mandatory provider.
- No product-semantics changes, storage formats, daemon, frontend, namespace, or recovery behavior.
- No claim that the initial capability is the complete curriculum, scenario atlas, assurance atlas, or handoff-retirement gate.
- No pretending that a missing external mdBook binary is available; `docs build` validates and emits mdBook-compatible source while a later renderer change owns executable mdBook integration.

## Decisions

### 1. Keep documentation maintenance in a separate development tool

Add an `xtask` workspace package with its own dependencies and dispatch `cargo xtask docs ...` through a repository alias. The production `dwv` binary remains unchanged; a future `dwv docs` is allowed only for a demonstrated user-facing documentation access need and is not created here. The tooling package is `publish = false` and does not enter any production crate dependency list.

### 2. Use JSON for checked-in machine contracts

The repository already uses `serde` and `serde_json`, while TOML parsing is not present. Versioned JSON is explicit, diffable, and avoids a dependency solely for configuration. Files are small and human-maintainable. A future format migration can add a schema version and deterministic converter without changing semantic IDs.

### 3. Use a real CommonMark event parser for source normalization

Add `pulldown-cmark` to the development-only `xtask` package as the bounded Markdown parser. Selectors bind to explicit configured IDs and heading paths, not line numbers. Normalization retains headings, paragraph text, list text, code content, links, and controlled block-marker HTML while excluding formatting-only representation. Parser options, source-size limits, heading depth, and output bytes are bounded.

### 4. Separate source authority from curriculum selection

`docs/model/sources.json` declares allowlisted source paths, stable IDs, authority/lifecycle/privacy metadata, and selectors. `concepts.json`/`curriculum.json` select what is taught. Extraction can know more than the book; it never creates pages implicitly. Work identifiers such as `OS-*`, `VE-*`, goal IDs, and archived changes are not permanent semantic source IDs.

### 5. Treat accepted pages as controlled inputs with sidecars

The initial Guide and documentation-system pages are checked-in Markdown with `dwv-doc:block` markers. Sidecars under `docs/state/blocks/` carry source snapshots, intent/accepted hashes, claims, goals, decision, and audit data. `cargo xtask docs bootstrap --accept-initial` is the explicit initial attestation; ordinary checking never infers cleanliness from absent state.

### 6. Use a deterministic graph closure for context

A context query resolves selectors through source IDs, concepts, pages, blocks, and configured related sources. It emits a bounded packet with omissions rather than silently truncating. The packet is the common input for future generation and implementation-agent context requests.

### 7. Apply external responses atomically and minimally

Responses are JSON files with a schema version, block ID, decision, source IDs, claims/goals, and rationale. `KEEP` updates only sidecar snapshots and audit state after validating current digests. Other decisions remain queued for a later preservation capability. Any invalid or out-of-scope response is rejected before writing.

### 8. Make deterministic checks model-free

`doctor`, `schema`, `extract`, `plan`, `context`, `check`, `build`, `test`, `bootstrap`, and `self-check` do not invoke providers or the network. The repository alias and `xtask` help expose the commands. Provider-backed maintenance is a successor concern.

### 9. Keep bootstrap inputs out of ordinary extraction

The source allowlist rejects `docs/handoffs/**` and the self-check rejects operational references to the bootstrap handoff path. A later retirement command may read a specifically supplied handoff only to generate a temporary disposition inventory; that path is not part of this change's live graph. Completion includes deleting the handoff and rebuilding/checking from permanent artifacts alone.

### 10. Make failure states explicit

Missing source, invalid selector, stale digest, missing provenance, marker mismatch, unknown ID, bound overflow, unsafe Markdown, and path traversal are errors with stable classes. No check downgrades an error to a warning or silently drops an artifact.

## Data flow

```text
allowlisted sources + Cargo metadata
        │
        ▼
source inventory (stable IDs, authority, lifecycle, privacy, digests)
        │
        ├── curriculum + concepts ──► page plan + coverage
        │                              │
        │                              ├──► SUMMARY.md + indexes
        │                              └──► block freshness state
        │
        └── selector query ───────────► bounded ContextPack / task

accepted Markdown + block sidecars ──► docs check / queue / KEEP apply
```

## Configuration and state

- `docs/model/requirements.json`: stable canonical requirement anchors keyed by capability and semantic slug, never by work ID or line number.
- `docs/model/concepts.json`: selected concepts, audiences, goals, misconceptions, and source selectors.
- `docs/model/curriculum.json`: ordered pages and blocks, prerequisites, profiles, claims, and paths.
- `docs/model/glossary.json`: canonical terminology and conflation rules.
- `docs/prompts/`: versioned grounding and profile contracts; no secrets.
- `docs/book/book.toml` and `docs/book/src/`: mdBook-compatible source and accepted derived blocks.
- `docs/state/source-inventory.json`, `coverage.json`, `page-plan.json`, `blocks/*.json`: generated deterministic state.
- `docs/generated/`: deterministic indexes and context-independent facts.

Generated state is tooling state, not DiskWeave array state. It is versioned and may be reconstructed from current sources/configuration; provenance loss fails closed.

## Security and bounds

Only repository-relative allowlisted paths enter extraction. Canonicalization rejects traversal and symlink escapes. Source files, units, context packets, task responses, and generated Markdown have independent byte/count/depth limits. Raw scripts, remote embeds, unsafe HTML, unknown directives, and path-traversing links are rejected. Source contents are quoted data; prompt-like text is never interpreted as an instruction. No payload bytes, credentials, private traces, or unredacted logs are configured as sources.

## Successor seams

- **Curriculum and coverage:** classifications, curriculum patches, stable-plan mutation safeguards, and unclassified safety-critical coverage.
- **Preservation-first maintenance:** assessment, `PATCH`/`REPLACE`/`BLOCKED`, localized writing/review, provider independence, anti-thrash, and idempotence.
- **Scenarios and assurance:** registered executable traces, deterministic facts, claim/evidence matrices, contributor maps, and deterministic includes.
- **Self-recovery and retirement:** CI/OpenSpec gates, adoption, state migration, handoff disposition, clean-room drill, retirement record, and deletion of the bootstrap input.

The initial capability deliberately leaves these later capabilities visible and dependency-ordered rather than hiding them behind placeholder APIs.

## Risks / Trade-offs

- A root-binary module is not used; a separate development-only `xtask` package keeps Markdown/OpenSpec/prompt/provider dependencies out of the production graph.
- JSON is more verbose than TOML but avoids a configuration parser dependency and keeps schemas explicit.
- `pulldown-cmark` adds one maintained parser dependency to `xtask`; it is justified by the requirement to normalize Markdown structurally rather than scrape headings with regex.
- The initial pages are accepted source-grounded prose, not model-generated output. This proves preservation/provenance mechanics without pretending a provider exists.
- mdBook-compatible source is useful now, but a missing `mdbook` executable keeps renderer/build acceptance explicitly pending for later renderer work.
