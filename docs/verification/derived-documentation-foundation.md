# Derived documentation foundation

**Date:** 2026-08-08
**Scope:** Offline repository-owned documentation tooling only

## Observed acceptance

- `cargo check -p xtask` passed.
- `cargo test -p xtask` passed: 3 tests.
- `cargo xtask docs help` passed and listed the development-only command surface.
- `cargo xtask docs doctor` passed: 25 bounded source units, no network use; `mdbook` is unavailable in this environment.
- `cargo xtask docs plan` passed: 2 pages, 3 blocks, 25 coverage entries.
- `cargo xtask docs bootstrap --accept-initial` passed and attested 3 accepted blocks.
- `cargo xtask docs check` passed with all 3 blocks `Clean` and `inventory_current: true`.
- `cargo xtask docs context guide` passed with 5 selected sources, no omissions, and bounded output.
- `cargo xtask docs queue --changed` passed with 0 clean blocks queued.
- `cargo xtask docs self-check` passed with the handoff excluded and state `clean`.
- `cargo metadata --format-version 1 --no-deps` showed Markdown/parser dependencies only in `xtask`; the production `diskweave` package remained unchanged.

The accepted pages are source-grounded projections. This record does not claim complete curriculum coverage, semantic prose assessment, provider access, executable mdBook rendering, platform behavior, or bootstrap-handoff retirement.
