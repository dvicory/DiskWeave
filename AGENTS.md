DiskWeave is correctness-first storage software. Prefer simple, explicit designs over clever abstractions.

Write technical prose using ISO 24495-1 and Zinsser's principles of simplicity and economy. Preserve DiskWeave terminology and correctness-relevant semantics.
Use descriptive names for current concepts. When a stable identifier is needed, use a descriptive semantic ID instead of an opaque code. Keep generated Bead IDs and version identifiers as-is. Use legacy IDs only when referring to historical material.

Current `openspec/specs/*/spec.md` requirements own current product behavior. A current requirement may explicitly delegate a bounded part of its exact semantics to an executable model; only that delegated surface is model authority. Active OpenSpec changes contain proposed target behavior. When work is being executed through a change, its delta specs define the target contract for that work and its tasks record implementation progress. Current requirements and their delegated semantics remain canonical until the completed, verified change is synced. The document marked `dwv:active-architecture-roadmap` guides future architecture and sequencing; it does not override current specs. Implementation, tests, non-delegated models, evidence, milestones, Beads, and historical artifacts are not semantic authority. Surface contradictions, missing invariants, and consequential ambiguity rather than resolving them from non-authoritative sources.

Preserve the portable semantic core. Keep platform-specific mechanisms behind explicit seams.

Persistent formats and schemas may change to satisfy current canonical semantics. Do not preserve an obsolete format merely for compatibility or silently reinterpret persisted state. Define needed migration, rejection, or reset behavior and update affected readers, writers, recovery paths, fixtures, and tests.

Proceed autonomously on reversible choices. Require human judgment only when a decision would alter an established architecture or correctness invariant, or materially change a significant operator-facing contract such as recovery authority, data-preservation behavior, or an existing compatibility guarantee. Represent such decisions as Beads human gates. If it is unclear whether an invariant or contract already exists, determine the current semantics first; uncertainty alone is not a human gate.

Keep correctness-boundary effects, ownership, durability, and state transitions explicit and testable. Prefer typed effects and deterministic interpreters where useful, but do not introduce a general effect-system framework unless it clearly improves the design without obscuring DiskWeave-specific I/O, durability, cancellation, or ownership semantics. Design critical state machines and I/O/recovery seams so failures, interleavings, and crash points can be exercised deterministically. Do not assume cancellation means rollback, I/O completion means durability, or dropping an async operation means the underlying I/O did not occur.

## Shared work and local agent state

DiskWeave uses Beads (`bd`) for shared work, status, dependencies, and human gates.

If Beads workflow context is not already available in the current session, or after context loss or compaction, run `bd prime` and follow its current guidance. Do not rerun it unnecessarily when that context is already loaded.

Beads track work; they do not define product behavior. Required product semantics belong in OpenSpec.

Use isolated jj workspaces for parallel or delegated repository changes. Do not create independent jj or Git repositories inside DiskWeave workspaces.

If the environment provides guidance for managing local agent workspace state, load and follow it when non-authoritative development state should survive the current session, support a handoff, or be shared across agents. Project obligations must not depend on agent-local state.

## Code and comments

Prefer types and structure that make constraints explicit. Use rustdoc for public APIs when types alone do not express behavior needed for correct use. Comment non-local invariants, lifecycle or ordering dependencies, ownership or concurrency constraints, external-system semantics, and non-obvious choices a future maintainer might incorrectly simplify.
