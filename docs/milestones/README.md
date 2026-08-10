# Milestone planning

Milestones are work-planning artifacts. They may reference canonical requirements, but they never own product semantics or override current canonical specifications.

## Source roles

- Current required product behavior lives in `openspec/specs/*/spec.md`.
- The document marked `dwv:active-architecture-roadmap` is the active architecture roadmap and design rationale. It guides future direction and sequencing without overriding current canonical requirements.
- Active changes contain proposed or in-flight semantics. Completed changes, implementation, tests, evidence, ADRs, and prior planning artifacts establish repository state, history, and constraints.

## Generate a milestone

1. Inspect current canonical specifications.
2. Inspect current implementation, tests, evidence, and active and completed changes.
3. Inspect the active architecture roadmap.
4. Identify dependency-ready roadmap outcomes and any correctness or authority prerequisites.
5. Reconcile each candidate roadmap node with work already completed.
6. Choose the smallest coherent executable work slice.
7. Use a roadmap-numbered OpenSpec change only when the work actually corresponds to that roadmap identity.
8. Use a descriptive unnumbered OpenSpec change for prerequisite fixes, cleanup, documentation, evidence work, and other non-roadmap work.
9. Never reimplement an already-satisfied roadmap outcome merely to consume its OS identifier.
10. Never let milestone text become a second source of product semantics.
11. When canonical specifications materially disagree with the active roadmap, expose and deliberately resolve the divergence rather than silently choosing implementation, history, or roadmap prose.
12. Keep milestone numbering independent from OpenSpec roadmap numbering.

Each milestone should name its inputs, canonical requirement IDs, dependencies, intended behavior or removal, observable acceptance results, proof or validation methods, explicit non-goals, and deferred work. Complete it only after its focused checks, applicable smoke workflow, workspace regressions, OpenSpec validation, documentation readiness, and residual-risk record pass.
