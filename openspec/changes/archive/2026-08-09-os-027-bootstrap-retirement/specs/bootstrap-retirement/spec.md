# bootstrap-retirement Specification

## Purpose

This capability retires the temporary documentation bootstrap handoff only after its enduring contracts are represented by durable repository artifacts and the documentation system passes an offline, handoffless reconstruction.

## ADDED Requirements

### Requirement: Useful documentation is selected by durable curriculum

The documentation book SHALL contain a newcomer Guide spine, contributor/operator material, at least one executable-scenario or evidence projection, assurance-boundary material, and a documentation-system maintainer view. Each page/block SHALL be selected by version-controlled concepts, source selectors, claims, learning goals, and prompt profiles.

#### Scenario: A fresh agent opens the book

- **WHEN** the agent reads `docs/book/src/SUMMARY.md` and the linked pages
- **THEN** it can learn the portable boundary, core mental model, write/recovery distinction, parity/integrity distinction, identity/degraded/rebuild boundaries, evidence limits, contributor workflow, and documentation maintenance workflow without opening the bootstrap handoff

### Requirement: Durable source coverage and provenance are checked

The tool SHALL extract configured current sources with authority, lifecycle, privacy, stable IDs, normalized semantic digests, and explicit classifications. Every selected block SHALL have current source snapshots and accepted-text provenance. A critical unclassified source, missing selector, stale source snapshot, invalid marker, or missing state SHALL fail closed with an actionable diagnostic.

#### Scenario: A source changes

- **WHEN** a source unit used by a block changes semantically
- **THEN** `cargo xtask docs check` reports the exact block and source reason as `NeedsAssessment` and does not silently rewrite accepted prose

### Requirement: Human-facing work uses bounded fresh isolated context

For human-facing `NEW`, pedagogical `PATCH`, `CURRICULUM_PLANNING`, and `INDEPENDENT_REVIEW` work, the queue SHALL emit a `DocTask` whose execution policy is exactly `fresh_isolated_context`. Each such task SHALL carry its task kind, complete writer instructions, complete reviewer instructions, a bounded selected-closure context, a context digest, and task/source provenance sufficient for an independent fresh invocation. The task packet SHALL not rely on the bootstrap handoff, hidden chat state, or unbounded repository access.

External fresh model or agent invocation is outside Rust. Rust SHALL only emit bounded task JSON and validate/apply structured `DocResponse` JSON; it SHALL not contain model/provider/OMP SDK integration or spawn-model logic.

#### Scenario: A human-facing task lacks required context

- **WHEN** the selected source closure or another required input is unavailable
- **THEN** the structured response is `NEED_SOURCES` with the missing source identifiers, the block remains non-clean and queueable, and no prose is generated or applied

### Requirement: Execution-policy provenance is validated at apply

`docs apply` SHALL validate that every accepted human-facing `DocResponse` identifies the `fresh_isolated_context` execution policy and matches the emitted task and context provenance. Apply SHALL record durable execution-policy provenance, including the task/context digests and response disposition, before accepting prose. Provider or model identity MAY be recorded as audit metadata but SHALL not become a freshness key.

#### Scenario: A response bypasses the fresh boundary

- **WHEN** a human-facing response omits, changes, or cannot match its task execution-policy provenance
- **THEN** apply rejects it atomically and leaves accepted prose and provenance state unchanged

### Requirement: One-time human-facing prose rebaseline is explicit and evidenced

An explicit one-time `--rebaseline` authorization MAY queue all human-facing prose, including clean blocks, for the migration. Ordinary queue/check operation SHALL remain preservation-first and SHALL not repeat that migration. The rebaseline SHALL produce durable evidence containing the authorization, emitted task inventory, task/context digests, execution-policy provenance, and every structured disposition, including `NEED_SOURCES`; until that evidence and its offline validation exist, the rebaseline and the retirement gate SHALL remain incomplete.

#### Scenario: Ordinary maintenance follows the migration

- **WHEN** queue or check runs without the one-time rebaseline authorization after migration
- **THEN** clean blocks are preserved, no provider/model change alone queues prose, and only source- or intent-driven `KEEP`, bounded `PATCH`, `REPLACE`, or `BLOCKED` assessment is eligible

### Requirement: Recovery is deterministic and provider-free

`doctor`, `extract`, `plan`, `build`, `test`, `check`, `bootstrap`, `self-check`, and the handoff audit SHALL not require model credentials or network access. Removing generated book/state outputs SHALL allow `bootstrap` to reconstruct deterministic outputs or emit complete bounded work. Missing provenance SHALL fail closed rather than mark prose clean.

#### Scenario: Generated output is absent

- **WHEN** generated book pages, summary, inventory, plan, and block state are absent in an isolated workspace
- **THEN** bootstrap reconstructs the deterministic pages and indexes, and explicit initial attestation is required before `check` reports clean

### Requirement: Platform and provider claims remain evidence-gated

Offline Rust and repository evidence SHALL not claim provider-backed prose generation, model execution, mdBook executable rendering, Linux/macOS frontend behavior, physical durability, or other unavailable platform behavior. Such claims SHALL remain explicitly gated or unclaimed until their required evidence exists.

#### Scenario: A portable check is mistaken for platform evidence

- **WHEN** an artifact has only offline Rust or simulated evidence for a provider, mdBook, platform, or physical behavior
- **THEN** the artifact records that boundary as gated/non-claimed rather than treating the portable check as completion evidence

### Requirement: Bootstrap handoff dependencies are rejected

Ordinary source extraction, context generation, curriculum selection, prompts, generated pages, tests, and self-check SHALL reject or exclude `docs/handoffs/**`. A bounded audit MAY read the selected handoff as non-authoritative. After retirement, no operational reference to the handoff path may remain.

#### Scenario: A handoff path enters ordinary configuration

- **WHEN** a configured source or generated artifact references `docs/handoffs/`
- **THEN** validation fails with a handoff-dependency diagnostic

### Requirement: Handoff retirement has evidence

Retirement SHALL produce a machine-readable disposition inventory parsed from the handoff, a fresh-context review result, a clean-room drill record, and a compact non-operational retirement record containing the retired path digest, durable replacement IDs, evidence IDs, disposition counts, and the fact that the handoff is not operational. The record SHALL not claim completion until the one-time human-facing prose rebaseline evidence requirement and all other retirement evidence exist.

#### Scenario: Retirement is attempted early

- **WHEN** a disposition is unmapped, a live reference remains, generated state cannot be reconstructed, the rebaseline evidence is absent, or the clean-room gate fails
- **THEN** retirement refuses atomically and leaves the handoff and accepted pages unchanged
