DiskWeave is correctness-first storage software. Prefer simple, explicit designs over clever abstractions.

Current files under `openspec/specs/*/spec.md` own current required product behavior. The retained document marked `dwv:active-architecture-roadmap` guides future architecture and sequencing without silently overriding current specs; implementation and historical artifacts do not silently override them either. Surface contradictions, missing invariants, and consequential ambiguity. OpenSpec naming and authoring rules live in `openspec/config.yaml`.

Preserve the portable semantic core. Keep platform-specific mechanisms behind explicit seams.

During early development, do not preserve backward compatibility at the expense of a better design. Persistent formats and schemas may change freely, but changes must be applied coherently across readers, writers, recovery logic, simulators, fixtures, and tests. Prefer clean breaks over compatibility scaffolding until the project explicitly declares a stability boundary.

Prefer explicit, typed effects at correctness boundaries and deterministic/testable interpreters where useful. Do not introduce a general effect-system framework unless it clearly improves the design without obscuring DiskWeave-specific I/O, durability, cancellation, or ownership semantics.

Design correctness-critical state machines and I/O/recovery seams so failures, interleavings, and crash points can be exercised deterministically. Prefer property/fuzz testing of operation and fault sequences through the simulator, with discovered failures becoming regression tests.

Do not assume cancellation means rollback, I/O completion means durability, or dropping an async operation means the underlying I/O did not occur.

For reversible implementation choices, use good judgment and proceed. Escalate choices that materially affect architecture or correctness semantics.

## Knowledge workflow

Before changing correctness-sensitive behavior or documentation, follow `.agents/skills/diskweave-knowledge/SKILL.md` and run `cargo xtask docs knowledge readiness`.

## Comments

Prefer self-explanatory code over comments. Add comments only when they preserve
important information a reader cannot reasonably infer from the local code.

In particular, comment non-local invariants, lifecycle/order dependencies,
ownership or concurrency constraints, external system semantics, and deliberately
non-obvious choices that a future maintainer might otherwise "simplify" incorrectly.

Where possible, encode constraints in types, structure, assertions, or tests instead.
Comments should explain the hidden constraint and its consequence, not narrate the code.

Treat data-integrity, crash-consistency, and unsafe-code invariants as especially
worth documenting when they are not enforced locally.
