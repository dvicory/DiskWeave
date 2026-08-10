## 1. Durable curriculum and source coverage

- [x] 1.1 Expand source configuration and classifications to current architecture, canonical specs, ADRs, evidence, simulator/replay records, verification boundaries, and workspace metadata while excluding bootstrap inputs
- [x] 1.2 Expand concepts, glossary, prompt profiles, and curriculum into the Guide spine, contributor/operator, scenario/evidence, assurance, and documentation-system maintainer views
- [x] 1.3 Add source-grounded derived pages with explicit non-claims and stable provenance block markers
- [x] 1.4 Generate updated inventory, plan, coverage, summary, indexes, and block state from the durable model

## 2. Fresh-isolated-context execution and provenance

- [x] 2.1 Extend the DocTask/DocResponse contract for human-facing `NEW`, pedagogical `PATCH`, `CURRICULUM_PLANNING`, and `INDEPENDENT_REVIEW` tasks with the exact `fresh_isolated_context` execution policy
- [x] 2.2 Emit each human-facing task with complete writer and reviewer instructions, bounded selected-closure context, task/source provenance, and task/context digests; do not depend on `docs/handoffs/**` or hidden context
- [x] 2.3 Keep external fresh model/agent invocation outside Rust; have Rust emit task JSON and validate/apply only structured response JSON, with no model/provider/OMP SDK or spawn-model logic
- [x] 2.4 Make unavailable required inputs return structured `NEED_SOURCES` with missing source IDs, keep the work non-clean and queueable, and prevent partial prose application
- [x] 2.5 Make `docs apply` validate and record execution-policy provenance for accepted responses, including matching task/context digests and disposition
- [x] 2.6 Add the explicit one-time `--rebaseline` queue authorization and durable evidence for its authorization, task inventory, task/context digests, execution-policy provenance, and every disposition; leave rebaseline evidence pending until actually produced and validated
- [x] 2.7 Preserve normal queue/check anti-thrash after migration: clean blocks stay clean, provider/model changes do not queue prose, and source- or intent-driven assessment chooses `KEEP`, bounded `PATCH`, `REPLACE`, or `BLOCKED`

## 3. Deterministic recovery and audit

- [x] 3.1 Add fail-closed coverage/provenance/recovery checks and fixtures for missing generated outputs, missing state, stale sources, and handoff-path dependencies
- [x] 3.2 Add bounded Markdown-parser bootstrap disposition audit with durable mappings and fresh-context review output
- [x] 3.3 Add clean-room reconstruction evidence without generated outputs or bootstrap handoff
- [x] 3.4 Keep provider-backed prose generation, mdBook executable rendering, and Linux/macOS/platform or physical-durability claims explicitly gated until their evidence exists
- [x] 3.5 Add compact non-operational retirement record and remove temporary audit artifacts from live inputs

## 4. Retirement and verification

- [x] 4.1 Run handoffless gate and confirm no operational references to the handoff remain
- [x] 4.2 Remove the bootstrap handoff from the active tree only after the gate, rebaseline evidence, and all retirement evidence pass
- [x] 4.3 Run focused tests, workspace tests, format/lint, strict OpenSpec validation, and documentation acceptance
- [x] 4.4 Update work program and archive only after all claims remain truthful
