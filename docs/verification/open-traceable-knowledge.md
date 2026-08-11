# Open traceable knowledge completion evidence

## Claim boundary

This record covers the development-only documentation and knowledge workflow: canonical requirement identity, bounded context, sparse Rust/evidence/Markdown relationships, change-boundary impact, checked-in Human Guide prose, pinned offline Sphinx rendering, and clean-room reconstruction. It does not establish DiskWeave runtime correctness, a production frontend, platform installation, or physical durability.

## Human Guide exercise

A reader exercised the rendered Guide during the open-traceable-knowledge change. The first version failed acceptance: it was described as barely comprehensible, simultaneously too complex and insufficiently detailed. After the Guide was rewritten around concrete member files, byte examples, operational commands, and evidence boundaries, the reader found it much more broadly useful but reported that the parity chapter still jumped from “the equation is not enough” to four operation names without carrying the reader through the decisions.

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

### Knowledge packet cutover (2026-08-10)

The `reduce-knowledge-context-overhead` change now uses fixed `inspect.v1`, `ownership.v2`, `context.v3`, `affected.v3`, and `audit-context.v1` result contracts. Ordinary and audit packets carry a complete canonical unit for every requirement in their reading order, and audit accepts exactly one seed ID. Batched reference omissions are attributable by selected requirement and category. An oversized multi-ID context suggests one complete singleton request per selected ID only when every singleton packet fits; shared canonical units may repeat across suggestions. An oversized singleton or audit component fails without false split advice.

Semantic sufficiency was exercised separately from compression. A fresh agent was restricted to one readiness call and one context call for `req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible`; canonical-spec reads, repository search, and additional packet retrieval were forbidden. Initial readiness correctly reported the two changed requirements as stale. From one complete packet, the agent identified the governing identity, relationship, provenance, ordinary-scope, audit-scope, omission, bound, and workflow semantics and judged the packet sufficient for coherent implementation without source hunting. That exercise preceded the final schema renumbering and singleton-fallback correction, so it establishes complete-packet sufficiency rather than the final split algorithm. The final real `context.v3` command separately returned five reading-order IDs and five matching complete canonical units with non-empty bodies and exact source locators; focused regression covers the revised fallback.

Compression remains a measurement, not semantic evidence. The seven-requirement recovery-planning sample previously emitted 118,667 pretty-printed JSON characters across repeated inspect, ownership, and context calls. The corrected final `context.v3` command emitted 57,516 stdout characters and a 52,555-byte compact result: a 51.5% character reduction while carrying 22 reading-order IDs and 22 matching complete canonical units with zero reference omissions for every selected ID. The all-current 154-requirement invocation exited 1 with `context_bound_exceeded`, returned 154 proven singleton suggestions, and emitted no partial result.

Observed verification:

- `openspec validate reduce-knowledge-context-overhead --strict` and `openspec validate --specs`: passed; 25 current specs valid.
- `cargo test -p xtask`: 41 passed across 3 suites.
- `cargo test --workspace`: 307 passed across 34 suites; 1 ignored.
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- Real CLI exercises observed `context.v3` with five of five complete units, `ownership.v2` with requirement-attributed omissions, `affected.v3`, and single-seed `audit-context.v1` with six of six complete component units; a multi-ID audit was rejected with `usage`, and the all-current context bound failure returned one singleton suggestion per selected ID.
- `cargo xtask docs knowledge readiness`: ready for 154 requirements with zero unknown, uncovered, suspect, orphaned, historical, semantic-prerequisite, or mapping-collision diagnostics after individual review of the two changed requirements.
- `cargo xtask docs check`: ready; the two relationship additions remained context-only and required no further review.
- `cargo xtask docs build`: warning-clean Sphinx output for 154 requirements.
- `cargo xtask docs clean-room`: equivalent reconstruction without handoffs, archived changes, milestones, generated output, or global Python packages; object digest `1088aa35997a013e9d19699715572a4cb0f6ae3c11727963ba0e6a4a01a6bd2d`.

## Residual gaps and next work

- TOML section briefs are accepted but not preferred enthusiastically by the human reader. No prose generator, model evaluator, reusable usefulness corpus, scoring schema, or result registry is justified by this single reservation.
- Change-boundary relationship comparison depends on an available supported local revision provider. Absence is explicit and does not masquerade as an unchanged graph; deterministic semantic readiness still runs.
- Usefulness evidence is one-time sampled acceptance, not a general claim about every reader, task, or model.
- Product work remains outside the open-traceable-knowledge change: checksum scrub and verified repair integration (OS-017), accepted macOS bridge/attach behavior (OS-021/OS-022), Linux frontend work, and physical durability certification.
