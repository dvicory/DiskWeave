---
name: diskweave-milestone-planning
description: Produce a DiskWeave milestone as an autonomous execution contract for a capable repository-aware implementation agent. Preserve outcomes, accepted decisions, dependencies, correctness boundaries, acceptance evidence, and delegated choices without turning the milestone into semantic authority or task-level implementation instructions.
---

# DiskWeave milestone planning

Write a milestone as an **autonomous execution contract** for a capable repository-aware implementation agent.

A milestone should give the agent enough product context, accepted decisions, sequencing, constraints, acceptance evidence, and review boundaries to work autonomously without repeatedly asking what was intended.

Optimize for useful autonomous runway, not document length.

Milestones are planning artifacts, not semantic authority.

## Own at milestone level

A milestone should establish, when consequential:

- the useful user-visible or system-level outcome;
- why the work belongs together;
- important starting assumptions and dependencies;
- accepted product or architecture decisions that should not be casually reopened;
- scope and meaningful non-goals;
- dependency-ordered slices or work packages;
- the useful new fact each slice should make DiskWeave able to claim;
- acceptance evidence for that fact;
- important refusal, uncertainty, recovery, preservation, and failure behavior;
- consequential correctness or architecture review boundaries;
- end-to-end interactions that prove the slices form a coherent result;
- final completion conditions;
- non-obvious shortcuts that could satisfy local checks while violating the intended result;
- important engineering choices intentionally left to the implementation agent.

Prefer slices that establish meaningful capability or evidence over infrastructure-only phases.

## Respect semantic authority

Current canonical OpenSpecs define current required product behavior. A reviewed active OpenSpec change may define the target within its scope. Architecture supplies architectural direction within its declared authority boundary.

The milestone may preserve accepted decisions and identify semantic questions or target outcomes, but it does not silently create canonical behavior.

Implementation, tests, models, and evidence do not manufacture missing semantics.

When milestone work encounters consequential overlap, ambiguity, contradiction, target-versus-current uncertainty, or a question about semantic ownership, the implementation agent should use the repository's semantic-reconciliation process rather than treating milestone prose as the answer.

Under the current knowledge architecture, do not preassign executable specifications as canonical product owners. A milestone may call for formal modeling or verification evidence when it provides distinct correctness value. If the work reveals a strong case for bounded executable semantic ownership, treat the authority change as a separate canonical semantic decision.

## Delegate reversible structure

Do not normally pre-author:

- the number or names of OpenSpec changes;
- speculative `req.*` identities;
- exact canonical ownership edits;
- whether a particular change requires `design.md`;
- the contents or outline of `design.md`;
- exact Bead decomposition or dependency graph;
- prose task lists duplicating Beads;
- exact Rust types, modules, crates, or files;
- reversible library or implementation choices;
- repository procedures already owned by `AGENTS.md`, OpenSpec, knowledge, normalization, reconciliation, or other repo-local skills.

State the outcome and constraints. Let the repository-aware agent inspect the current tree and choose the minimum coherent semantic and implementation structure.

Prescribe an exact artifact or change boundary only when that boundary is itself part of an accepted architecture, migration, compatibility, or correctness contract.

## Work-package test

A work package should answer:

> What useful new fact can DiskWeave safely claim after this work, what semantic authority supports that claim, what evidence establishes it, and what still fails closed when the required proof is absent?

Use dependency ordering when one claim genuinely relies on another.

Do not create phases merely to separate ordinary engineering disciplines.

A work package may include investigation when a consequential semantic or evidence question must be resolved before implementation can safely proceed.

Do not turn open engineering choices into fake product decisions.

## Detail test

Include detail when removing it would likely cause a capable agent to:

- implement the wrong product behavior;
- cross an authority or correctness boundary;
- miss a real dependency or inherited obligation;
- broaden a claim beyond its evidence;
- take an attractive but architecturally harmful shortcut;
- lose important refusal, uncertainty, recovery, preservation, or lifecycle behavior;
- repeatedly require a product decision that is already settled;
- or be unable to tell whether the work package is complete.

Omit detail when it:

- explains ordinary engineering;
- repeats repository procedures;
- inventories facts the agent can reliably rediscover;
- duplicates canonical semantics;
- predicts exact OpenSpec, model, Bead, or implementation structure unnecessarily;
- or merely makes each section independently complete.

## Economy and single ownership

Give each milestone decision, constraint, and acceptance condition one primary home.

Do not repeat the same rule across goals, work packages, end-to-end stories, completion criteria, and forbidden shortcuts.

Detailed acceptance belongs with the work package that proves it.

Final milestone completion should contain only cross-package and integration conditions.

End-to-end stories should exercise interactions between work packages rather than restating individual acceptance.

Forbidden shortcuts should contain only plausible failures not already made clear by nearby outcomes or acceptance.

Do not inventory requirement IDs, implementation symbols, or repository state a capable agent can reliably retrieve unless the exact identity is necessary to preserve a decision or dependency.

## Review gates

Call for explicit review when a mistake could invalidate a major correctness, durability, recovery, authority, compatibility, or architecture claim.

A review gate should state what must be established before proceeding, not prescribe a ceremonial report format.

Do not require human approval for ordinary reversible engineering.

When a review can be resolved mechanically or by an existing semantic/verification workflow, use that workflow rather than inventing a milestone-specific process.

## Completion

A milestone is complete when:

- its intended useful system outcome exists;
- its cross-package dependencies and interactions are satisfied;
- required semantic changes are owned through normal repository authority;
- evidence supports the claims actually made;
- important failure/refusal/uncertainty behavior remains conservative;
- unresolved work outside the milestone is explicit and does not invalidate the completed claim;
- planning prose is not required to understand current product behavior.

A milestone should leave a capable implementation agent knowing what outcome matters and how to recognize success, while leaving reversible semantic artifact structure, Bead decomposition, formal-model structure, and implementation engineering to the workflows that already own them.
