# DiskWeave documentation

Canonical OpenSpecs remain authoritative. The Sphinx site, knowledge objects, Rust/API links, guide, scenario pages, assurance views, and contributor maps are checked projections of current repository artifacts.

## Commands

Run maintenance through the development-only package:

```text
cargo xtask docs doctor
cargo xtask docs knowledge readiness
cargo xtask docs knowledge inspect <semantic-id>
cargo xtask docs knowledge context <semantic-id>
cargo xtask docs knowledge affected --path <repo-relative-path>
cargo xtask docs knowledge doctor --path <repo-relative-path>
cargo xtask docs check
cargo xtask docs build
cargo xtask docs serve
```

`docs check` is the single fail-closed readiness gate. At the change boundary it uses an available repository revision baseline to report canonical semantic changes, removed or reassigned requirement relationships, and implementation-only context without making every Rust edit a documentation task. `--base <revision>` selects a wider multi-commit baseline when needed. `knowledge affected` and `knowledge doctor` are diagnostics, not before/after-edit workflow. A successful renderer alone is insufficient. `docs build` regenerates ignored interchange, Cargo/rustdoc projections, Sphinx-Needs objects, and warning-clean linked HTML through the repository-pinned mise environment. `docs serve` is development-only live preview.


## Durable entry points

- Architecture: `docs/architecture/derived-documentation-system.md`
- Accepted decisions: `docs/adr/os-030-open-traceable-knowledge.md`
- Canonical requirements: `openspec/specs/*/spec.md`
- Human and engineering pages: `docs/sphinx/`
- Reader intent: `docs/curriculum.toml`
- Reviewed semantic state: `docs/reviewed-requirements.toml`
- Evidence and executable fixtures: `verification/manifest.toml`, `verification/corpus/`
- Agent workflow: `.agents/skills/diskweave-knowledge/SKILL.md`
- Toolchain pins: `mise.toml`
- Reconstructible output: `target/dwv-docs/` (ignored)

Normal discovery excludes handoffs, archived changes, rendered output, and historical goal-v5 machinery. Historical material is consulted only for an explicit archaeology task.
