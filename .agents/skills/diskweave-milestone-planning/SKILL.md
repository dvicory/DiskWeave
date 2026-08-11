---
name: diskweave-milestone-planning
description: Turn DiskWeave discovery, accepted decisions, and repository evidence into an autonomous milestone execution contract. Use when drafting or refining docs/milestones/m*.md for handoff to a repository-aware implementation agent. Preserve outcomes, consequential decisions, horizontal work packages, acceptance evidence, non-goals, validation boundaries, and delegated choices while leaving OpenSpec change decomposition and reversible implementation structure to the implementation agent.
---

# DiskWeave milestone planning

Produce DiskWeave milestone plans as **autonomous execution contracts** for capable repository-aware implementation agents.

The milestone sits between high-level product/architecture discovery and repository-local OpenSpec planning. It should preserve decisions and acceptance boundaries that the implementation agent must not have to rediscover, while leaving current semantic ownership, OpenSpec change structure, and reversible implementation choices to the repository-aware agent.

Optimize for **autonomous runway with low authority duplication**, not minimum document length.

Use as much detail as the milestone's architectural and correctness risk requires. A long milestone is acceptable when its detail prevents consequential mistakes, repeated user decisions, or ambiguous completion. Length that merely repeats the same semantic rule in several sections is not.

## Inputs

Use the available discovery discussions, accepted user decisions, prior milestone patterns, current repository evidence, canonical OpenSpecs, active architecture roadmap, and relevant ADRs.

Treat discovery material as evidence to distill, not text to preserve wholesale.

When reviewing discovery, distinguish:

* accepted product or architecture decisions;
* current repository dependencies that materially constrain the milestone;
* unresolved questions that must remain visible;
* candidate implementation approaches that should remain delegated;
* ideas deliberately deferred or rejected.

Do not silently promote tentative discovery ideas into milestone requirements.

Repository observations that are likely to become stale should appear only when they establish a dependency or explain an accepted planning decision. Phrase such observations as starting evidence the implementation agent should verify against current reality.

## Authority

Milestones are planning artifacts, not semantic authority.

Current canonical `openspec/specs/*/spec.md` requirements own current required product behavior. The active architecture roadmap guides future architecture and sequencing without silently overriding current specs.

If milestone discovery identifies an apparent semantic gap, conflict, or intended behavior change, state the desired milestone-level invariant and require repository reconciliation through OpenSpec. Do not preallocate canonical ownership merely because planning suggests where it probably belongs.

Historical changes, discovery documents, implementation details, and the milestone itself do not silently override current canonical semantics.

## What the milestone owns

A milestone should establish enough of the following to let an implementation agent work autonomously for an extended period:

* the useful user-visible or system-level outcome;
* why the work forms one coherent milestone rather than unrelated features;
* consequential starting dependencies;
* accepted product and architecture decisions that should not be reopened casually;
* included scope and explicit non-goals;
* important refusal, uncertainty, recovery, failure, and claim-boundary behavior;
* dependency-ordered horizontal work packages;
* the useful new system claim delivered by each work package;
* acceptance evidence sufficient to prove each work package;
* consequential validation or review boundaries;
* end-to-end stories that prove interaction between work packages;
* non-obvious forbidden shortcuts that could appear locally reasonable while violating the milestone;
* engineering choices intentionally delegated to the implementation agent;
* final cross-package completion criteria.

Prefer work packages that cross enough layers to produce meaningful capability. Avoid infrastructure-only phases when the infrastructure can instead be built inside the first package that needs it.

### Cohesion and external gates

Every work package must be necessary to one integrated milestone completion claim. Test this by asking whether the package could be deferred while the milestone's central claim remained honestly complete. Execution dependencies and end-to-end interactions are strong evidence of cohesion, but a shared audit theme, defect class, or discovery source is not sufficient by itself.

An external-review gate does not justify including independent blocked work. If an unresolved product, semantic, or architecture choice can be deferred without invalidating the integrated milestone claim, record it separately rather than making the autonomous milestone depend on that decision.

## Work-package format

Each substantial work package should normally contain:

### Useful new claim

State what DiskWeave may truthfully claim after this package that it could not claim before.

Prefer an operator-visible or externally meaningful system capability.

### Scope

Describe the semantic and product boundaries that must be crossed to establish the claim.

Specify necessary behavior and dependencies without prescribing ordinary implementation structure.

### Acceptance evidence

Give concrete scenarios sufficient to prove the new claim and its important fail-closed alternatives.

Acceptance criteria should distinguish what is proven from what remains unknown, unsupported, blocked, refused, failed, or uncertain.

Prefer observable outcomes and forbidden outcomes over implementation checklists.

### Validation gate

Identify any consequential correctness or architecture boundary crossed by the package.

A validation gate means the implementation agent must reconcile the result with current semantic ownership, run the applicable evidence, and resolve material contradictions before depending on the new claim.

A validation gate is **not automatically a user stop**. The agent should continue autonomously when the current repository, specifications, evidence, and milestone decisions resolve the gate.

Label a gate **External review required** only when the milestone intentionally requires the user or planning reviewer to approve a consequential unresolved choice before proceeding.

## Delegate to OpenSpec and the implementation agent

Do not normally pre-author:

* the number or names of OpenSpec changes;
* the mapping between work packages and OpenSpec changes;
* exact `req.*` IDs;
* speculative canonical owner/refiner arrangements;
* whether a particular change needs `design.md`;
* the contents or outline of `design.md`;
* exact task decomposition;
* exact Rust types, modules, crates, files, or APIs;
* reversible parser, library, serialization, storage-layout, concurrency, or process-structure choices;
* detailed repository procedures already owned by `AGENTS.md`, OpenSpec, or repository-local skills.

The implementation agent should inspect current repository state and use OpenSpec to choose the smallest coherent change or changes needed to make meaningful progress toward the milestone.

A work package may require multiple OpenSpec changes. One OpenSpec change may satisfy parts of multiple work packages. Do not make them correspond merely for organizational neatness.

Prescribe an OpenSpec/change boundary only when the boundary itself is an accepted consequential architecture, migration, compatibility, or semantic decision.

Do not require `design.md`. Allow OpenSpec's current schema and the actual change to determine whether design rationale is warranted.

## Preserve consequential decisions, not implementation guesses

Include detail when removing it would likely cause a capable agent to:

* implement the wrong product behavior;
* cross a correctness or semantic-authority boundary;
* miss a hard dependency;
* broaden a claim beyond its evidence;
* collapse meaningful failure or uncertainty distinctions;
* take an attractive but architecturally harmful shortcut;
* reopen a product or architecture decision the user already made;
* repeatedly require user decisions;
* or be unable to tell whether a work package is complete.

Omit detail when it:

* tells a capable agent how to perform ordinary engineering;
* duplicates repository/OpenSpec workflow instructions;
* restates canonical requirements without adding milestone-level consequence;
* records current implementation trivia that the agent can reliably rediscover;
* prematurely chooses among reasonable implementations;
* predicts requirement IDs or source-code structure;
* or exists only to make each section independently self-contained.

## Economy and single ownership

Within the milestone, give each decision, constraint, and acceptance condition **one primary home**.

Do not restate the same rule in background, semantic discussion, operator contract, work-package acceptance, evidence strategy, forbidden shortcuts, end-to-end stories, and final DoD merely to make every section complete.

Use these ownership rules:

* settled cross-cutting product/architecture decisions belong in the settled-decisions section;
* detailed package behavior and acceptance belong with the work package that proves it;
* end-to-end stories test interactions between packages rather than restating package acceptance;
* evidence guidance states unusual claim/evidence boundaries rather than repeating every required test;
* forbidden shortcuts contain only non-obvious attractive failures not already obvious from acceptance;
* final completion criteria contain only facts that must be simultaneously true across the integrated milestone.

Reference an earlier rule when necessary instead of rewriting it.

## OpenSpec handoff quality

The completed milestone must be useful as direct planning input to a repository-aware implementation agent.

It should tell that agent **what outcome and boundaries must survive OpenSpec decomposition**, while leaving the agent free to discover from current repository state:

* which canonical requirements already own the semantics;
* which requirements actually need modification;
* whether new ownership is necessary;
* how many coherent changes are appropriate;
* whether each change needs design rationale;
* and how implementation tasks should be structured.

Do not include a duplicate `/opsx:*` workflow tutorial in the milestone.

## Evidence and claims

Match evidence to the claim being made.

Keep important distinctions explicit when relevant, including:

* deterministic/model evidence versus adapter evidence;
* process/reopen evidence versus physical durability evidence;
* portable semantic admission versus platform publication;
* observation versus mutation;
* current evidence versus stale historical evidence;
* definite failure versus uncertain consequential outcome.

Do not let a convenient test mechanism silently broaden the milestone's claim.

## Final compression pass

After drafting the milestone, perform a dedicated compression review before returning it.

For every paragraph, ask:

1. Does this preserve a decision, dependency, claim boundary, autonomous acceptance criterion, or delegated freedom?
2. Is this information already stated elsewhere?
3. Could a repository-aware frontier model safely rediscover it?
4. Does it belong in OpenSpec rather than the milestone?

Delete or consolidate text when the first answer is no, when the second is yes without a new purpose, or when the latter questions show that the detail belongs downstream.

Do **not** achieve compression by weakening work-package acceptance, removing important refusal behavior, or replacing precise correctness conditions with vague summaries.

Spend words primarily on consequential decisions and work-package acceptance.

## Final quality test

The milestone should let the implementation agent answer throughout execution:

> What useful new fact can DiskWeave safely claim after this work package, which current semantic owner supports that claim, what evidence proves it, and what still fails closed when the required proof is absent?

The final document should provide enough context for long-running autonomous execution while leaving OpenSpec and implementation structure to the repository-aware agent wherever those choices remain reversible.
