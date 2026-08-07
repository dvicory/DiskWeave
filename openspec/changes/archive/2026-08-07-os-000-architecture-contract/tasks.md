## 1. Artifact contract

- [x] 1.1 Cite handoff Sections 4.5, 5, 22, and 25 and the applicable D/P/V/T/U/F identifiers.
- [x] 1.2 Make `design.md` contain the twenty numbered sections in the prescribed order.
- [x] 1.3 Define validator-compatible delta requirements for source of truth, dependencies, portable boundaries, evidence, and agent workflow.
- [x] 1.4 Record OS-001, OS-002, and OS-003 as direct successors and preserve the Phase 0 dependency graph.

## 2. Evidence accounting

- [x] 2.1 State that OS-000 has no runtime, database, platform, hardware, or persistent-format implementation.
- [x] 2.2 State that OpenSpec validation proves artifact structure only and does not prove runtime, durability, or hardware behavior.
- [x] 2.3 Record custom architecture-checker code, duplicate registries, and hidden chat-only readiness as forbidden outcomes.

## 3. OpenSpec verification

- [x] 3.1 Run `openspec validate os-000-architecture-contract --json` after the final artifact edits and resolve all reported errors.
- [x] 3.2 Run `openspec status --change os-000-architecture-contract --json` and confirm the artifact state and task status are reproducible.
- [x] 3.3 Run `openspec instructions apply --change os-000-architecture-contract --json` and confirm the task context names only the permitted artifact scope.
- [x] 3.4 Keep the handoff, Rust source, and archive unchanged while performing this artifact-only update.
