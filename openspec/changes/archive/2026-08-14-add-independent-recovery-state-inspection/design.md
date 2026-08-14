## Context

The portable recovery core already owns `RecoveryInspection`, `RecoveryManifest`, schema versions, and export limits. The SQLite adapter already exposes `SqliteRecoveryStore::inspect`, which performs read-only classification without taking its writer lock or running initialization, migration, or commit-intent cleanup. No executable exposes that seam independently of the production operator path.

The current SQLite adapter remains evaluation-only. This change must produce useful inspection evidence without implying a production durability decision or stable persistent format.

## Goals / Non-Goals

**Goals:**

- Expose the existing observational inspection seam as a small independently runnable command.
- Preserve one semantic result across human and JSON rendering.
- Keep production service, operator, and frontend crates out of the executable dependency closure.
- Make read-only behavior, bounded output, classification completeness, and experimental claim scope directly testable.

**Non-Goals:**

- Introduce another recovery model, parser, migration engine, or format registry.
- Execute migration, reconciliation, repair, recovery, parity verification, or payload reads.
- Generalize the command framework or select a stable-format compatibility policy.

## Decisions

### Add one binary target beside the existing read-only adapter

Add `dwv-recovery-inspect` as a binary target in `dwv-recovery-sqlite`. It accepts `dwv-recovery-inspect [--json] <artifact>` and calls only the public `SqliteRecoveryStore::inspect` seam. Keeping the binary beside the adapter avoids a new crate and dependency while still allowing `cargo build -p dwv-recovery-sqlite --bin dwv-recovery-inspect` to prove independence from the production `diskweave`, `dwv-service`, and frontend crates.

A binary in the production `dwv` CLI was rejected because it would not prove the independent-tool boundary. A new workspace crate was rejected because one source file and the existing package dependency closure are sufficient.

### Render one explicit semantic result

Map `RecoveryInspection` once into a result containing:

- schema `dwv.recovery-inspection.v1`;
- `observed` outcome;
- the six-value inspection classification;
- known format layer and version facts;
- format-claim status `experimental`;
- the portable manifest only for `supported`;
- an explicit non-authorization statement; and
- a deterministic next action.

Both renderers consume this result. JSON uses the existing `serde_json` dependency. Human output uses direct formatting; no CLI or rendering dependency is added. The binary must not serialize storage-engine tables, paths found inside private state, runtime handles, or adapter errors as if they were portable semantics.

### Separate observation completion from artifact health

Every value returned by `SqliteRecoveryStore::inspect` is a completed observation and exits zero, including absent, corrupt-or-unreadable, unsupported, migration-required, and reconciliation-required. Missing/extra arguments and result-rendering failure are nonzero operational outcomes. This matches the repository's current observational contract: a conservative state classification is useful evidence, not command failure.

### Prove bounds and non-mutation at the executable boundary

Reuse recovery-core manifest/export bounds and the adapter's checked reader. Focused command tests snapshot the artifact bytes and directory entries before and after each disposition, including missing state and a commit-intent sidecar. Oversized or malformed fixtures must return a bounded classification or refusal without creating a database, lock, journal, or migrated state.

A focused dependency check inspects the binary package graph and rejects production service, operator, and frontend dependencies. This is a boundary check, not a new generalized architecture linter.

## Risks / Trade-offs

- **An executable may look production-ready.** The required `experimental` format status and stable-format non-claim keep the evidence boundary explicit.
- **The SQLite adapter combines corruption and unreadability.** The command preserves the canonical `corrupt-or-unreadable` result rather than guessing a narrower cause.
- **A supported manifest can be large.** Existing recovery bounds remain authoritative; the command adds no unbounded detail or alternate export path.
- **The binary location couples it to the current adapter package.** That is deliberate for the current experimental format. Portable result semantics remain in `dwv-recovery`, so a later adapter can replace the mechanism without changing the contract.
