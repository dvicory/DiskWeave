DiskWeave is correctness-first storage software. Prefer simple, explicit designs over clever abstractions.

Write technical prose using ISO 24495-1 and Zinsser's principles of simplicity and economy. Preserve DiskWeave terminology and correctness-relevant semantics.

Current `openspec/specs/*/spec.md` files own required product behavior. The document marked `dwv:active-architecture-roadmap` guides future architecture and sequencing; it does not override current specs. Implementation and historical artifacts are not semantic authority. Surface contradictions, missing invariants, and consequential ambiguity.

Preserve the portable semantic core. Keep platform-specific mechanisms behind explicit seams.

Persistent formats and schemas may change to satisfy current canonical semantics. Do not preserve an obsolete format merely for compatibility or silently reinterpret persisted state. Define needed migration, rejection, or reset behavior and update affected readers, writers, recovery paths, fixtures, and tests.

Proceed on reversible implementation choices. Human approval is required for destructive or irreversible changes, operator-visible recovery behavior changes, or changes to an existing compatibility guarantee. Surface choices that materially affect architecture or correctness semantics.

Keep correctness-boundary effects, ownership, durability, and state transitions explicit and testable. Prefer typed effects and deterministic interpreters where useful, but do not introduce a general effect-system framework unless it clearly improves the design without obscuring DiskWeave-specific I/O, durability, cancellation, or ownership semantics. Design critical state machines and I/O/recovery seams so failures, interleavings, and crash points can be exercised deterministically. Do not assume cancellation means rollback, I/O completion means durability, or dropping an async operation means the underlying I/O did not occur.

## Local agent work

`work/` is a Git-ignored symlink to persistent local agent state for evidence, experiments, handoffs, and useful intermediate work that does not belong in DiskWeave. Nothing there is semantic authority.

Use `jj workspace` inside `work/workspaces/` for isolated or parallel repository changes. Launch implementation agents from the workspace they own. Do not create independent jj or Git repositories inside DiskWeave workspaces.

DiskWeave uses Beads (`bd`) for shared executable work. Beads belongs to the DiskWeave repository, not `work/`. OpenSpec owns required product semantics; Beads records execution state and dependencies only. Use `bd prime` for workflow guidance and `bd where` to verify the shared tracker before changing work from a jj workspace.

## Code and comments

Prefer types and structure that make constraints explicit. Use rustdoc for public APIs when types alone do not express behavior needed for correct use. Comment non-local invariants, lifecycle or ordering dependencies, ownership or concurrency constraints, external-system semantics, and non-obvious choices a future maintainer might incorrectly simplify.
