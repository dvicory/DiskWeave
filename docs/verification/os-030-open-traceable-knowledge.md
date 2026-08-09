# OS-030 completion evidence

## Claim boundary

This record covers the development-only documentation and knowledge workflow: canonical requirement identity, bounded context, sparse Rust/evidence/Markdown relationships, change-boundary impact, checked-in Human Guide prose, pinned offline Sphinx rendering, and clean-room reconstruction. It does not establish DiskWeave runtime correctness, a production frontend, platform installation, or physical durability.

## Human Guide exercise

A reader exercised the rendered Guide during OS-030. The first version failed acceptance: it was described as barely comprehensible, simultaneously too complex and insufficiently detailed. After the Guide was rewritten around concrete member files, byte examples, operational commands, and evidence boundaries, the reader found it much more broadly useful but reported that the parity chapter still jumped from “the equation is not enough” to four operation names without carrying the reader through the decisions.

The accepted revision adds validated ordered section briefs to `docs/curriculum.toml`. Each brief names a proposed heading, focus, must-answer questions, exact requirement/scenario sources, and non-claims. Chapter 3 now carries the same bytes through a known-erasure read, interrupted separate-target rebuild, ambiguous parity mismatch, independently authorized repair, and loss of authority requiring rebaseline before presenting the comparison table.

**Human conclusion:** pass, with an explicit reservation: the reader did not love the TOML mechanism and questioned whether TOML is the best long-term medium for guiding prose toward pedagogical goals. This is a usability gap, not evidence that the accepted pages are incorrect. Revisit the representation only when another real Guide revision demonstrates that structured section briefs impede authorship or prose quality; do not add a template engine or generated-prose subsystem speculatively.

## Fresh-agent implementation exercise

A fresh agent began from `.agents/skills/diskweave-knowledge/SKILL.md`, without handoffs or historical instructions. It used readiness, inspect, and bounded context for `req.documentation-knowledge-architecture.change-boundary-impact-is-deterministic-and-bounded`, then inspected `RevisionControl::detect`, `change_impact`, its call sites, the temporary fixture helper, and adjacent tests.

The agent made one bounded implementation-test change: `change_impact_reports_unavailable_baseline_outside_revision_control` in `xtask/src/knowledge.rs`. The test proves that an environment without supported revision control reports `baseline_available = false`, does not create a documentation review task, and emits an explicit baseline-unavailable diagnostic. It changed no canonical requirement, maintained page, production behavior, dependency, or generated output. The agent explicitly reported that no documentation edit was needed for this test-only coverage change.

**Agent conclusion:** pass. The skill and graph-selected context led to the exact owner and test seam. Source hunting was limited to the implementation, fixture, and call sites needed by the task. The agent reported no omitted relationship from its context packet. Parent verification ran the focused behavior and the completion-only `docs check`; no before/after-edit documentation workflow was used.

## Deterministic verification

Observed on 2026-08-09:

- `cargo test --workspace`: 251 passed across 32 suites; 1 ignored.
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo xtask docs doctor`: valid configuration; 140 canonical requirements; no network.
- `cargo xtask docs check`: ready; zero unknown, uncovered, suspect, orphaned, historical, or mapping-collision diagnostics; no change review required.
- `cargo xtask docs build`: warning-clean Sphinx output for 140 requirements.
- `cargo xtask docs clean-room`: equivalent reconstruction without handoffs, archived changes, generated output, or global Python packages; object digest `885d09ad5d5c26a074efc187ac8711742c8d799b3a7dfe1c6a4b5baa1ea1c71a`.
- `openspec validate --all --json`: 28 of 28 current specs and changes valid; informational long-requirement notices remain non-blocking.
- Rendered parity chapter inspection: expected causal headings and comparison table present; no broken local links observed.

## Residual gaps and next work

- TOML section briefs are accepted but not preferred enthusiastically by the human reader. No prose generator, model evaluator, reusable usefulness corpus, scoring schema, or result registry is justified by this single reservation.
- Change-boundary relationship comparison depends on an available supported local revision provider. Absence is explicit and does not masquerade as an unchanged graph; deterministic semantic readiness still runs.
- Usefulness evidence is one-time sampled acceptance, not a general claim about every reader, task, or model.
- Product work remains outside OS-030: checksum scrub and verified repair integration (OS-017), accepted macOS bridge/attach behavior (OS-021/OS-022), Linux frontend work, and physical durability certification.
