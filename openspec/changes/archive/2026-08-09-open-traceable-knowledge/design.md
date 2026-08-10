## Context

The current `xtask` owns requirement extraction, source inventories, page planning, block state, context packs, response application, mdBook rendering, and projection JSON. Canonical OpenSpecs and the small architecture constitution are already authoritative, generated state is ignored, and historical handoffs are excluded by default. The remaining problem is that generic indexing/rendering and DiskWeave semantics are coupled in one large Rust module, while source ownership, evidence, and explanatory claims lack a sparse checked relationship spine.

The production workspace must remain free of Python/Sphinx/model dependencies. Documentation checks must run deterministically without network or credentials after pinned dependencies are available. Source text, generated model prose, paths, and includes are untrusted.

## Goals / Non-Goals

**Goals:**

- Keep canonical requirement prose solely in current OpenSpecs and cross-cutting invariants in the architecture constitution.
- Use mature open tooling for Markdown parsing/rendering, needs objects, Rust marker extraction, API reference, links, search, and relationship views.
- Keep only DiskWeave-specific identity, fingerprint, policy, evidence/scenario adaptation, reviewed-link lifecycle, bounded context selection, and readiness coordination in repository glue.
- Prove the complete vertical slice before broad annotation or content migration.
- End with ordinary checked-in MyST Markdown and compact, comprehensible durable configuration/state.

**Non-Goals:**

- Whole-program Rust call-graph analysis.
- Annotation coverage over every function or test.
- A second requirements authoring system, semantic registry, vector database, provider integration, or hosted documentation service.
- Runtime traceability behavior or dependencies in storage crates.
- Preserving generated prose or custom machinery that has no irreducible role.

## Decisions

### 1. Open-source reuse dispositions

| Capability | Tool | Disposition | Boundary and evidence to collect |
|---|---|---|---|
| Markdown and site | MyST Parser + Sphinx | ADOPT | Own parsing, navigation, cross-references, HTML, search, warnings-as-errors, and link checking. |
| Engineering objects and graph views | Sphinx-Needs | ADOPT | Import generated current objects; own typed links, backlinks, filters, tables, flows, and validation. OpenSpec remains canonical. |
| Rust marker extraction and scope association | sphinx-codelinks | ADOPT | Use Rust discovery and `@need-ids:` parsing. Configure need definitions from source off; validate actual nested scopes, attributes, tests, moves, deletion, and unknown IDs in the vertical slice. |
| Rust API/reference pages | `cargo doc` / rustdoc | ADAPT WITH THIN GLUE | Official maintained Rust tooling owns API extraction and HTML. Sphinx links to ignored rustdoc output and Cargo metadata supplies structural views. `sphinxcontrib-rust` is rejected because installation invokes `cargo install` outside its isolated Python environment and generated pages must be written into the Sphinx source tree, violating the no-global-mutation and reconstructible-build-output boundaries. |
| Workspace structure | Cargo metadata | ADOPT | Own package, target, and dependency facts. |
| Trace interchange | LOBSTER | USE AS OPTIONAL COMPARISON | Its trace model is useful for export/comparison, but OpenSpec import, reviewed fingerprints, and current/historical authority remain DiskWeave-specific. Do not make LOBSTER another authoring authority. |
| Requirements authoring | StrictDoc and Doorstop | REJECT | Both would duplicate OpenSpec authority. Reusable ideas do not justify a second requirements store. |
| Test report projection | Sphinx-Test-Reports | ADAPT WITH THIN GLUE when real standard reports exist | Use for standard report formats; retain thin adapters for deterministic simulator schedules, normalized traces, and repository evidence records. |
| Proprietary indexes/services | Any closed-source tool | REJECT | May be personal convenience only; cannot participate in CI, reconstruction, readiness, or required agent workflow. |

Pin Sphinx directly in the repository-local `mise.toml` through mise's `pipx` backend, with exact `uvx_args --with` pins for MyST, Sphinx-Needs, sphinx-codelinks, and build helpers in the same isolated tool environment. Pin `uv` in mise as the pipx backend runner. Every documentation command runs through `mise exec`, so no global package installation, ambient Python package, project virtualenv, or separate Python tool pin is required. Rust API output uses the repository-pinned Rust toolchain's rustdoc. Record licenses and exact selected versions in the accepted ADR and verification evidence.

### 2. Canonical IDs and fingerprints

Canonical requirement headings carry a colocated comment `<!-- dwv:req <capability>.<semantic-slug> -->`. The intrinsic ID is primary. Extraction rejects missing IDs, duplicate IDs, malformed lowercase semantic IDs, and IDs whose capability prefix disagrees with their canonical location.

The semantic fingerprint is BLAKE3 over a normalized requirement unit containing the heading, requirement body, and scenarios. The existing CommonMark event normalizer is retained only for this DiskWeave-specific semantic change detector. It removes representation-only whitespace and formatting variation while retaining textual tokens, code, links, modality, negation, statuses, and scenario structure. Fixtures prove formatting stability and semantic sensitivity.

Sphinx IDs are `R_` plus a bounded uppercase BLAKE3 prefix. Export records both the canonical and Sphinx IDs, validates the checked mapping, and rejects collisions.

Requirement migration metadata is a small canonical OpenSpec-adjacent file only when an ID is split, merged, removed, or superseded; no migration file exists speculatively.

### 3. Generated spine and compact reviewed state

`cargo xtask docs knowledge export` derives current requirements into ignored `target/dwv-docs/knowledge/objects.json`. The exported schema is deliberately narrow and exists for Sphinx interchange and bounded inspection rather than becoming a general semantic database.

Durable review state lives in `docs/reviewed-requirements.toml` and contains current requirement fingerprints plus explicit outcomes and reasons where needed. Source markers and artifact-owned metadata define relationships; reviewed state does not copy claims.

If the current requirement fingerprint differs from the reviewed fingerprint, the requirement and its dependent relationships are suspect. Resolution runs through a command that validates the current identity and atomically updates only the named requirement. Direct digest churn cannot satisfy readiness because current requirements and references are re-extracted.

### 4. Sparse markers and classification

CodeLinks markers reference generated Sphinx requirement IDs, never define needs. Markers are placed only at semantic module/type/impl/operation/test boundaries selected by the vertical slice. CodeLinks owns discovery, parsing, surrounding-scope association, source location, and backlinks. Thin Rust glue imports CodeLinks output and classifies relations by configured source roots and endpoint kind (`implements` for production owners, `verifies` for tests/harnesses).

A compact sidecar is permitted only for external evidence, generated fixtures, or relationships without one honest source scope. It contains no prose. Unknown targets, ambiguous scopes, path escapes, historical targets, and unsupported relation kinds fail.

### 5. Sphinx interchange and project layout

Ordinary Markdown and configuration under `docs/sphinx/` replace `docs/book/` and mdBook. `cargo xtask docs build` writes current knowledge objects and generated Sphinx sources under ignored `target/dwv-docs/`, builds rustdoc, and runs the pinned Sphinx toolchain. Human-authored orientation remains Markdown; structural tables and source links are generated projections.

The repository-local Sphinx generator imports OpenSpec-derived objects, artifact-owned evidence/scenario facts, Cargo metadata, and CodeLinks output. It may not define canonical requirements, own Rust scope parsing, or author prose.

`docs/curriculum.toml` is an executable drafting contract rather than a loose prompt profile. Each Human Guide entry carries ordered section briefs with a proposed heading, focus, must-answer questions, exact source identities, and non-claims. The skill tells an agent to use those briefs as a causal drafting checklist. Readiness validates their structure and current references. The briefs do not contain canonical prose, generate Markdown, or recreate page-plan/block-state machinery.

### 6. AI-maintained Markdown and context

Checked-in MyST Markdown is the prose baseline. `docs check` performs impact discovery once at a change boundary through a small revision-provider seam. The Rust tool normalizes changed paths and prior file contents from a supported local repository provider; the skill contains no provider-specific commands. Canonical fingerprint changes and removed or reassigned requirement relationships require review. Pure relationship additions and implementation edits whose relationships are unchanged remain bounded context, avoiding automatic documentation work after every Rust edit. `--base` selects a wider provider-native revision for multi-commit work.

No revision-diff or relationship-hash registry is retained. `affected` remains a bounded current-context/page diagnostic, while `knowledge doctor --path` recovers a relationship removed from the current file using the selected revision baseline. Baseline absence is explicit. Whether prose still explains arbitrary implementation behavior remains an agent review decision informed by task intent, not a model or deterministic-tool verdict.

Usefulness is sampled at acceptance of the open-traceable-knowledge change, not automated. One reader exercises the Guide and one fresh agent performs a real implementation task from graph-selected context. A human reviewer records concrete observations and residual gaps in the change completion evidence. No durable corpus, scoring schema, result registry, evaluator command, or model-issued verdict is added. Revisit recurring evaluation only after a concrete regression shows that its value exceeds its maintenance cost.

### 7. Agent command and skill contract

Retain `cargo xtask` as the development-only entry point and add concise `knowledge export|inspect|context|trace|why|affected|readiness|resolve` commands. Reduce `docs` commands to `doctor|check|build|serve|clean-room`; `docs check` is the single change-readiness gate with exact failed gates and next actions. Human output is concise; `--json` enables stable machine output.

A repository-local skill explains procedure and commands, not architecture. The root agent instruction only points semantic/documentation changes to the skill and readiness gate.

### 8. Migration sequence

1. Prove five representative requirements end to end with pinned tools, markers, evidence/scenario links, one claim, suspect behavior, and offline build.
2. Add deterministic commands, lock workflow, skill, and readiness gate.
3. Generate and agent-maintain the interrupted-write chapter through the real context workflow; exercise it with a reader before expanding.
4. Migrate Guide, Scenario, Assurance, Contributor, and Reference pages.
5. Remove mdBook configuration, custom page plans, block-state/task-response CMS, obsolete prompt schemas, and renderer code.
6. In a temporary copy, delete generated state/output and reconstruct without historical inputs, network, credentials, or model access.

## Risks / Trade-offs

- CodeLinks Rust scope extraction may mishandle macros, attributes, or nested items. The vertical slice gates adoption; a fallback must retain an evidenced minimal parser seam rather than fork CodeLinks.
- Python packages increase supply-chain surface. Exact lock, license record, bounded source roots, warnings-as-errors, no remote includes, and offline CI mitigate it.
- `sphinxcontrib-rust` may not cover every Rust item. Missing reference details may link to rustdoc; custom API extraction is prohibited unless a concrete required gap remains.
- Semantic normalization cannot prove language equivalence. It is intentionally only a suspect-state trigger; agent review remains mandatory.
- Sparse links can leave real gaps. Requirement-centric policy and readiness expose missing relations rather than maximizing marker count.
- Removing the custom CMS deletes tested machinery. Migration waits for Sphinx parity, reconstruction, no-op stability, and content usefulness evidence, then makes a clean cutover without dual renderers.
- Usefulness scoring can become circular and grow into another documentation subsystem. One-time observed acceptance exercises provide the needed evidence without permanent evaluator machinery; self-attested model verdicts are inadmissible.
