## Purpose

This capability defines the initial offline, repository-owned capability of DiskWeave's derived documentation and semantic-context system. It projects selected authoritative repository inputs into bounded source inventory, curriculum-selected pages and blocks, provenance/freshness state, deterministic book structure, and external-agent task packets without making explanatory prose normative.

## ADDED Requirements

### Requirement: Authority-aware source inventory

The documentation tool SHALL extract only configured repository paths and registered selectors into a versioned inventory. Each source unit SHALL have a stable ID, source kind, authority tier, lifecycle, privacy class, normalized semantic digest, source location, and bounded body. Archived, provisional, superseded, deferred, and bootstrap-only sources SHALL remain distinguishable from current authoritative sources. The handoff under `docs/handoffs/` SHALL NOT be an ordinary source input.

#### Scenario: Current sources are extracted

- **WHEN** the operator runs the offline extraction command against the repository
- **THEN** the tool emits a deterministic versioned inventory containing the configured architecture, OpenSpec, ADR, evidence, and code metadata units with stable IDs and digests

#### Scenario: A bootstrap handoff is present

- **WHEN** the repository contains a file under `docs/handoffs/` that is not explicitly supplied to the bootstrap-audit operation
- **THEN** ordinary extraction excludes it and reports no authority edge to it

#### Scenario: A configured source is malformed or outside the allowlist

- **WHEN** a selector cannot be resolved, a path escapes the repository, or a source exceeds configured bounds
- **THEN** extraction fails with the source ID/path and reason instead of silently dropping the unit

### Requirement: Curriculum controls human-facing selection

The tool SHALL use version-controlled concept, page, and block configuration to select human-facing documentation. Source discovery SHALL NOT automatically turn every source unit into a page. Concept IDs, page IDs, block IDs, prerequisite order, audiences, learning goals, prompt profiles, source selectors, required/forbidden claims, and refresh policy SHALL be explicit and validated for uniqueness and cycles.

#### Scenario: The initial curriculum is planned

- **WHEN** the configured source inventory and curriculum are valid
- **THEN** planning emits a deterministic page plan and coverage report in configured order, including selected source units and any intentional reference-only, internal-only, or deferred classifications

#### Scenario: A prerequisite cycle exists

- **WHEN** the curriculum contains a cycle or a page/block references an unknown concept or source selector
- **THEN** planning fails with the complete cycle or missing identifier and does not emit an apparently valid plan

### Requirement: Deterministic book structure and facts

The tool SHALL generate deterministic `SUMMARY.md`, source/reference indexes, coverage tables, and structured scenario/evidence facts from the inventory and curriculum. These artifacts SHALL not require an LLM, network access, model credentials, or arbitrary source discovery. Generated explanatory Markdown SHALL be explicitly labeled as derived and SHALL not become a product-semantic authority.

#### Scenario: The book structure is built

- **WHEN** the operator runs the offline build command with valid configuration
- **THEN** the configured book root and `SUMMARY.md` are generated in stable order, source links resolve to current repository locations, and deterministic outputs are reproducible byte-for-byte

#### Scenario: A deterministic input changes

- **WHEN** a source inventory, curriculum, or registered scenario changes
- **THEN** only the affected deterministic output and dependent block state change; unrelated accepted prose is not rewritten

### Requirement: Provenance and freshness are explicit

Each selected documentation block SHALL have a versioned sidecar containing its stable identity, page, intent digest, source-unit digests, claims, learning goals, scenario/evidence snapshots, accepted-text hash, status, last decision, prompt contract version, and bounded audit metadata. The supported states SHALL distinguish at least `Missing`, `NeedsGeneration`, `Clean`, `NeedsAssessment`, `Blocked`, and `Invalid`.

#### Scenario: A current accepted block is checked

- **WHEN** its source snapshot, intent, markers, accepted-text hash, and provenance are valid
- **THEN** the check reports `Clean` and emits no generation task

#### Scenario: A semantic source changes

- **WHEN** a source unit in a block dependency set changes its normalized digest
- **THEN** the check reports `NeedsAssessment` with the exact block, source ID, and delta reason

#### Scenario: Formatting-only source changes

- **WHEN** normalization produces the same semantic representation after a formatting-only edit
- **THEN** the source digest and accepted block remain clean and no prose task is emitted

#### Scenario: Provenance or markers are missing

- **WHEN** a selected block has missing, malformed, stale, or inconsistent provenance or block markers
- **THEN** the check reports `Invalid` or `Missing` and never silently marks the prose clean

### Requirement: Context packets and external-agent tasks are bounded

The documentation tool SHALL emit a versioned, structured context packet for a selected concept, page, block, OpenSpec, crate, scenario, or evidence ID using deterministic graph selection. Packets SHALL include authority metadata, selected source units, concepts/claims when configured, related blocks, explicit bounds, and omission notices. Queue tasks SHALL use the same packet and a versioned response schema; output limits, permitted block IDs, and permitted paths SHALL be enforced.

#### Scenario: A bounded context request succeeds

- **WHEN** the operator requests a known selector within configured limits
- **THEN** the tool emits a deterministic packet identifying included sources, omitted sources, bounds, and privacy class without payload bytes or secrets

#### Scenario: A context request exceeds bounds

- **WHEN** the selected graph closure exceeds source-count, byte, depth, or output limits
- **THEN** the tool reports explicit omissions or split work and never silently truncates the packet

#### Scenario: An unknown selector is requested

- **WHEN** the operator requests an unknown concept, block, source, scenario, or evidence ID
- **THEN** the tool fails with the requested ID and nearest configured scope rather than selecting a similarly named source

### Requirement: Preservation-first external responses

The first external-agent protocol SHALL accept only schema-valid responses for known blocks and supported decisions. A `KEEP` response SHALL update the assessed source snapshot and audit state without changing accepted Markdown bytes. Invalid, out-of-scope, path-changing, oversized, or unsupported responses SHALL be rejected atomically and leave the previous accepted files and state intact. `PATCH`, `REPLACE`, and `BLOCKED` remain represented in the schema even when this capability does not apply their prose changes automatically.

#### Scenario: A fake assessor keeps unchanged prose

- **WHEN** a valid `KEEP` response names a `NeedsAssessment` block and the current claim/goal constraints still hold
- **THEN** the source snapshot and decision attestation update while the accepted Markdown hash and bytes remain identical

#### Scenario: A response attempts an unrelated rewrite

- **WHEN** a response changes a clean block, names an unknown block, cites an unconfigured source, or writes outside the permitted block path
- **THEN** validation rejects it atomically with diagnostics and preserves all prior accepted files and state

### Requirement: Offline command and recovery surface

The repository SHALL expose deterministic commands or exact equivalents for schema inspection, doctor checks, extraction, planning, bounded context, queue generation, response application, freshness checking, deterministic build, tests, bootstrap/recovery, stale explanation, and self-check. These commands SHALL not require a model or network for deterministic checks. Missing generated files SHALL be reconstructable from durable configuration and source authorities, while missing provenance SHALL fail closed and emit reassessment work.

#### Scenario: A fresh checkout bootstraps the capability

- **WHEN** a fresh checkout has configuration but no generated inventory, book structure, or derived state
- **THEN** the bootstrap command reconstructs the deterministic outputs and reports any required external-agent tasks without referring to chat history or the bootstrap handoff

#### Scenario: Generated output is deleted

- **WHEN** generated book structure or inventory output is removed and bootstrap/check is rerun
- **THEN** the outputs are recreated deterministically and accepted narrative is not regenerated merely because derived files were absent

#### Scenario: Deterministic checks run without model access

- **WHEN** doctor, check, build, test, schema, self-check, or bootstrap runs with no provider credentials or network
- **THEN** the command either succeeds using local inputs or emits a complete actionable task; it never attempts a hidden network/model call

### Requirement: Security and privacy boundaries are enforced

Only configured paths and generated artifacts SHALL enter context packets. Source contents SHALL be treated as quoted repository data, including prompt-like text. The tool SHALL reject path traversal, unknown directives/includes, unsafe raw HTML/scripts, unbounded files or counts, and model responses that select arbitrary paths. Real payload bytes, secrets, credentials, private traces, and unredacted logs SHALL be excluded from retained fixtures and packets by default.

#### Scenario: Prompt-like source text is extracted

- **WHEN** a registered fixture contains text such as `ignore previous instructions`
- **THEN** the inventory labels it as source data and no command or instruction is executed or promoted into the task contract

#### Scenario: A response attempts path escape

- **WHEN** an external response targets a block path outside the configured book root or uses traversal components
- **THEN** validation rejects the response before any file is written

### Requirement: Documentation maintenance is development-only

Documentation extraction, curriculum planning, context generation, task queuing, response application, freshness checking, diagnostics, and book building SHALL run through the development-only `cargo xtask docs ...` surface. The production `dwv` CLI and production DiskWeave crates SHALL NOT gain documentation-maintenance commands or dependencies on Markdown, mdBook, OpenSpec parsing, prompt templates, LLM/provider clients, or documentation compilers. A future user-facing documentation access command requires a separate demonstrated need.

#### Scenario: The production dependency boundary is inspected

- **WHEN** the workspace is built and its package dependency graph is inspected
- **THEN** documentation-maintenance dependencies occur only in the development tooling package and the production `dwv` binary remains unchanged

#### Scenario: Documentation maintenance runs

- **WHEN** an author runs `cargo xtask docs <operation>` without model credentials or network access
- **THEN** the deterministic operation runs in the tooling package or emits a complete bounded task without invoking the production CLI

### Requirement: Durable documentation uses permanent semantic anchors

Long-lived documentation dependencies SHALL resolve to architecture decisions/invariants, canonical OpenSpec requirement anchors, durable `VP-*` assurance properties, and concrete evidence artifacts. `OS-*`, `VE-*`, autonomous goals, task IDs, active changes, and handoffs SHALL remain work/history metadata and SHALL NOT be the semantic identity of a Guide concept, block, claim, or source dependency. When work is archived, the dependency SHALL rebind to its resulting permanent artifact without changing accepted prose unless the permanent artifact changes its relevant semantics.

#### Scenario: A work item completes and is archived

- **WHEN** an active OpenSpec or VE work item is completed and its resulting canonical requirement/evidence artifacts are registered
- **THEN** the affected block dependency is rebound to those artifacts and an unchanged semantic result produces zero prose diff

#### Scenario: A canonical requirement lacks an anchor

- **WHEN** a configured requirement source has no stable semantic anchor or its anchor no longer resolves
- **THEN** extraction/check fails with the capability and required anchor update rather than deriving an ID from an OS number, line number, or heading position

### Requirement: Handoff extinction is enforced

The bootstrap handoff SHALL be excluded from ordinary authoritative source discovery, prompts, provenance, generated documentation, runtime, and validation. Before completion, every enduring rule from it SHALL have a durable architecture, ADR, canonical OpenSpec, typed configuration/schema, prompt, Rust, or executable-test home. The completed system SHALL include a clean-room test that removes the handoff and reconstructible generated outputs and rebuilds/checks from permanent repository artifacts alone.

#### Scenario: Ordinary extraction sees a handoff

- **WHEN** a file under `docs/handoffs/` exists during ordinary extraction or context generation
- **THEN** it is excluded and no live graph, block, prompt, or provenance record depends on it

#### Scenario: Handoffless reconstruction runs

- **WHEN** the handoff and generated documentation outputs are absent in an isolated checkout
- **THEN** bootstrap reconstructs the deterministic inventory/plan/book outputs, check succeeds or emits complete bounded work, and no diagnostic refers to the deleted handoff or chat history
