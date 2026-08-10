# ADR: Open traceable knowledge tooling

**Status:** Accepted

**Date:** 2026-08-09

**Scope:** Development-only knowledge, traceability, and documentation tooling

## Context

DiskWeave's current documentation pipeline correctly keeps canonical OpenSpecs authoritative and runs deterministic checks offline, but it also implements generic Markdown parsing, page planning, rendering, source indexing, backlinks, and projection state in one bespoke Rust subsystem. The durable replacement needs checked requirement-to-implementation/evidence/explanation relationships while leaving ordinary Markdown readable and keeping every Python dependency outside production crates.

## Decision

Use canonical OpenSpecs as the only intended-behavior source and project them through a sparse trace spine into open-source Sphinx tooling. The repository-local `mise.toml` pins every Python entry point directly: Sphinx runs through mise's pipx backend with exact `uvx_args --with` extension pins, `sphinx-autobuild` has its own equivalently pinned mise tool environment, and mise-pinned `uv` provides the isolated runner and managed Python runtime. No global pip installation, project virtualenv, provider, hosted service, or proprietary index is required.

Generated requirements, symbols, links, Sphinx interchange, context packets, rustdoc output, and rendered HTML are ignored build products. Checked-in OpenSpecs, sparse source markers, evidence metadata, reviewed-link state, curriculum intent, and MyST Markdown are durable inputs.

## Reuse matrix

| Capability | Project and selected version | License | Disposition | Owned boundary / evidence |
|---|---|---|---|---|
| Site and cross-reference build | Sphinx 9.1.0 | BSD-2-Clause | ADOPT | Navigation, cross-references, HTML/search, warnings-as-errors, and link checking. Installed and reports `sphinx-build 9.1.0` through local mise. |
| Markdown | MyST Parser 5.1.0 | MIT | ADOPT | MyST/CommonMark parsing and Sphinx directives. Loaded only inside the Sphinx mise tool environment. |
| Engineering objects | Sphinx-Needs 8.3.0 | MIT | ADOPT | Imported non-authoritative needs, typed links, backlinks, tables, flows, filters, and validations. It does not author DiskWeave requirements. |
| Rust marker extraction | sphinx-codelinks 1.4.0 | MIT | ADOPT | Bounded Rust discovery, Tree-sitter scope association, semantic `dwv:req` marker extraction, and source backlinks. A thin mapping adapter validates semantic IDs because CodeLinks cannot translate them to opaque Sphinx IDs. Need creation from Rust is disabled. |
| Live local preview | sphinx-autobuild 2025.8.25 | MIT | ADOPT | Development-only rebuilding/server; pinned as its own mise pipx tool so its executable is exposed without global installation. |
| Python runner | uv 0.11.29 | Apache-2.0 OR MIT | ADOPT | Mise pipx/uvx backend and managed Python runtime. It is not a production dependency. |
| Rust API reference | repository-pinned rustdoc / `cargo doc` | Apache-2.0 OR MIT | ADAPT WITH THIN GLUE | Official Rust tooling owns API extraction and HTML. Sphinx links to ignored rustdoc output; Cargo metadata owns workspace/package structure. |
| Rust/Sphinx generator | sphinxcontrib-rust 1.2.1 | GPL-3.0-or-later | REJECT WITH CONCRETE EVIDENCE | Its documented installation invokes `cargo install` outside the isolated Python tool, and its generated pages must live in the Sphinx source tree. Both violate the no-global-mutation and ignored-reconstructible-output boundaries. Rustdoc already supplies the required reference facts. |
| Trace interchange | LOBSTER | BSD-3-Clause | USE AS OPTIONAL COMPARISON | May receive an export for interoperability; it is not another requirements authoring system and is not required by CI. |
| Requirements authoring/management | StrictDoc and Doorstop | GPL-3.0 / LGPL-3.0 | REJECT WITH CONCRETE EVIDENCE | Their authoring stores would duplicate canonical OpenSpec authority. No thin adapter is needed for behavior already covered by Sphinx-Needs imports. |
| Standard test reports | Sphinx-Test-Reports | MIT | ADAPT WITH THIN GLUE WHEN NEEDED | Use only when standard report artifacts exist. Simulator schedules, normalized traces, and bounded platform evidence retain small artifact-specific adapters. |
| Closed-source editors/indexes/services | Any | varies | REJECT | Personal convenience only. Never part of build, CI, context, readiness, reconstruction, or required maintenance procedure. |

Version and package metadata were checked against the projects' official documentation and PyPI records on 2026-08-09. The selected Sphinx environment resolved successfully through `mise install`; Python 3.12.13 was obtained by uv without adding a repository Python tool or touching global pip state.

## DiskWeave-specific glue retained

Only these responsibilities remain custom:

- canonical OpenSpec ID, lifecycle, and semantic fingerprint validation;
- reversible canonical-to-Sphinx identity mapping;
- import adaptation from OpenSpec and artifact-owned scenario/evidence metadata;
- relationship policy and compact reviewed/suspect state;
- bounded identity/graph-selected agent context;
- claim provenance validation, provider-neutral change-boundary impact discovery, and the single readiness coordinator;
- privacy, path-root, historical-source, revision-baseline, and output bounds specific to DiskWeave.

Each adapter writes open or simple ignored interchange and can be removed when an adopted tool gains the exact behavior. Custom code may scan the bounded `dwv:req` comment line to validate and map its semantic ID, but it does not duplicate CodeLinks' Rust scope parsing. It must not render generic graph views when Sphinx-Needs can or define requirements outside OpenSpec.

## Security and reproducibility

- All source roots are explicit, repository-relative, symlink-safe, and bounded.
- Remote includes and unsafe raw HTML are disabled; warnings and broken internal links fail builds.
- Source text and model output are data and are never executed.
- Deterministic checks invoke no model, provider, network service, or proprietary tool.
- Mise owns Python tool activation; storage crates have no Python/Sphinx dependency.
- Reconstructible state and output stay under ignored `target/` paths.

## Consequences

The checked-in documentation becomes ordinary MyST Markdown, while requirements, Rust owners, evidence, scenarios, and claims remain independently inspectable through sparse links. The previous mdBook and custom projection CMS are removed only after the Sphinx vertical slice, content migration, no-op behavior, and clean-room reconstruction pass. There is no permanent dual-renderer architecture.

Usefulness acceptance for the open-traceable-knowledge change is deliberately sampled rather than automated: one reader exercises the Guide and one fresh agent performs a real implementation task from graph-selected context. A human reviewer records concrete observations and residual gaps in completion evidence. The repository retains no usefulness corpus, scoring schema, result registry, evaluator command, or model-issued verdict.
