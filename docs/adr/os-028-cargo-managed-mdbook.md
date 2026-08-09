# ADR OS-028: Cargo-managed mdBook rendering

**Status:** Superseded by ADR OS-030
**Date:** 2026-08-08
**Scope:** Development-only documentation tooling
**Supersession:** ADR OS-030 replaces Cargo-managed mdBook with repository-pinned MyST/Sphinx, Sphinx-Needs, Sphinx-CodeLinks, rustdoc, and sphinx-autobuild. The renderer remains development-only and non-authoritative.


## Context

The documentation pipeline previously treated an mdBook executable on `PATH` as optional. That made rendered-book acceptance depend on an unrecorded machine toolchain even though the repository already owns the `xtask` documentation boundary.

## Decision

1. Pin the mdBook library at `0.4.40` as a normal dependency of the publish-disabled `xtask` package. A dev-only binary needs the dependency at command runtime; a Cargo dev-dependency is available only to tests and examples.
2. `cargo xtask docs build` SHALL generate the source projection and render `docs/book` through the Cargo-managed mdBook API, reporting the pinned renderer version and output directory.
3. mdBook remains a renderer only. It does not select sources, own curriculum or provenance, validate product claims, or become a dependency of any storage crate or the production `dwv` binary.
4. Accepted Markdown remains the source artifact. Rendering is verification/output; semantic normalization is used for digests and validation, never written into accepted prose.

## Consequences

- A clean checkout can build the HTML book without `mise`, a globally installed executable, a provider, or network access after Cargo dependencies are available.
- Cargo.lock records the renderer dependency and its transitive versions.
- Replacing the renderer remains a development-tool change at the `xtask` boundary; storage semantics and the documentation source graph are unaffected.

This decision supersedes the optional-executable environment gate in ADR OS-026 consequence 31.

## Evidence

- `cargo run -p xtask -- docs doctor` reports the pinned mdBook version.
- `cargo run -p xtask -- docs build` reports `renderer: mdbook` and writes `docs/book/book`.
- `cargo test -p xtask` covers accepted Markdown preservation and normalization safety.
