---
name: diskweave-semantic-reconciliation
description: Determine and repair DiskWeave semantic ownership when current requirements overlap, compose, conflict, or leave consequential behavior underdetermined. Use bounded review by default and make the smallest durable semantic repair.
---

# DiskWeave semantic reconciliation

Use this skill when repository facts are available but judgment is required to determine what the product contract actually says and which requirement owns each decision.

Semantic reconciliation answers:

> What behavior is being decided, what do current canonical semantics require, who owns each independently meaningful decision, and what is the smallest repair if the answer is ambiguous, duplicated, contradictory, or incomplete?

Follow the repository authority rules. For current behavior, start from current canonical OpenSpecs. When reviewing work inside an active change, keep the current contract distinct from that change's target contract.

Use the repository's knowledge tooling for deterministic context, relationships, and affected dependents. Do not treat retrieval output as a semantic verdict.

## One decision, one owner

Under the current repository authority model, each independently meaningful product decision should have one canonical OpenSpec owner.

Other requirements may refine that decision, compose it with independently owned decisions, map a platform mechanism into it, or constrain it at an architecture, security, or evidence boundary. They should not independently redefine the same policy.

A broader proposition may be completely determined by several independently owned decisions. Do not add an umbrella owner unless the broader proposition adds its own policy.

## Models and executable specifications

Executable specifications and models can make bounded state, ordering, crash/recovery behavior, interleavings, and invariants precise.

Under the current repository authority model, OpenSpec owns product semantics. Models remain verification or evidence unless that authority is deliberately changed. Do not delete or weaken canonical OpenSpec ownership merely because a model is more precise.

When an executable representation would materially simplify a bounded semantic-ownership problem, surface that authority question explicitly; do not settle it by implication in either direction.

If a model and canonical semantics disagree, reconcile the product contract first, then bring the model or implementation into conformance as appropriate.

## Bound the question

Start from the semantic question at hand. Broaden only when its dependencies require it.

Understand enough of the complete behavior to compare ownership. As relevant, include:

- subject or resource;
- preconditions and admission;
- authority or evidence consulted;
- allowed action or transition;
- ordering and durability conditions;
- success meaning;
- refusal, failure, and uncertainty;
- crash/restart/recovery behavior;
- preservation and lifecycle consequences;
- persistent or externally visible interpretation;
- permitted claims and explicit non-claims.

Do not require a worksheet when ordinary reasoning is sufficient. Do not declare agreement from the success path alone when consequential failure or uncertainty behavior differs.

## Determine the ownership shape

Common healthy shapes include:

- one owner plus a narrower refinement;
- composition of independently owned decisions;
- adapter or platform conformance to portable semantics;
- architecture, security, or evidence constraints that do not redefine the product rule.

Problems include:

- duplicate ownership of one policy;
- contradictory canonical behavior;
- consequential underdetermination or a missing owner;
- historical or implementation behavior being treated as authority;
- implementation nonconformance;
- consequential shadow semantics outside canonical authority.

These are reasoning distinctions, not a required report taxonomy.

Use these questions when the answer is not obvious:

### Material divergence

> Could two implementations satisfy the current canonical semantics yet behave materially differently on this question?

If yes, determine whether the variation is deliberately delegated or whether the contract is incomplete or contradictory.

### Deletion

> If the non-owning descriptions disappeared, would the decision still be completely defined by its owner?

If not, important semantics remain scattered.

### Failure and uncertainty

> Do the apparent owners agree on consequential refusal, uncertainty, recovery, abandonment, preservation, and lifecycle behavior?

Only examine dimensions that matter to the bounded question.

### Fresh-agent authority

> Can a fresh repository-aware agent identify the rule and its owner without using implementation, history, campaign state, or conversation context as semantic authority?

If not, the contract still depends on a hidden second source of truth.

## Inspect implementation and evidence after establishing the contract

Once the current semantics are understood, inspect implementation, tests, models, and evidence as needed.

Use them to ask whether the implementation conforms, whether tests encode a shadow policy, whether a model exposes an under-specified transition, or whether evidence supports claims stronger than the contract allows.

Do not rewrite clear semantics merely to match existing code.

Use architecture after current semantics are established to evaluate target coherence and direction. Use history only when rationale or provenance is needed to distinguish a semantics-preserving repair from an unintended change.

## Make the smallest durable repair

A good result may be no semantic change.

When repair is needed, prefer the smallest change that leaves one clear owner for each decision. This may mean:

- removing duplicated normative prose and referencing the real owner;
- reducing a requirement to its genuine local refinement;
- relocating ownership while preserving semantic identity when the obligation is unchanged;
- splitting genuinely independent decisions;
- merging artifacts that no longer have independent semantic responsibility;
- adding genuinely missing behavior;
- deliberately changing or retiring obsolete behavior;
- fixing implementation rather than changing correct semantics.

Do not choose split, merge, relocation, or a new identity before understanding the behavior being owned.

Any normative semantic repair — including correcting or removing current canonical behavior—belongs in an OpenSpec change. Start or update that change with the appropriate OpenSpec skill. Do not edit canonical specs as the repair itself; canonical specs change through the OpenSpec sync workflow. Keep explicit whether the work repairs current semantics or defines new target behavior. The OpenSpec workflow owns its proposal, design, task, sync, and archive mechanics.

When editing OpenSpec, write the product rule directly. Keep conditions that affect authority, durability, ordering, refusal, uncertainty, recovery, preservation, or claims explicit. Do not leave reconciliation commentary in canonical requirement prose.

## Preserve identities and review dependents

Preserve a stable `req.*` identity when it still denotes materially the same durable obligation. Do not preserve it merely to avoid migration after the obligation materially changes, and do not allocate a new identity merely because prose moved.

When an owned decision changes, review affected refiners, composers, adapters, claims, implementation links, and evidence through the repository's normal affected/dependent workflow.

Do not assume dependents remain valid merely because tests pass.

## Completion

A bounded reconciliation is complete when:

- each current decision in scope has an identifiable owner or explicit defect, and any target change remains visibly target until its normal authority transition completes;
- valid refinement and composition do not independently redefine owned decisions;
- two conforming implementations cannot materially disagree because of an unintended gap or contradiction;
- consequential failure and uncertainty behavior is determined or explicitly unresolved;
- any repair uses the minimum durable ownership shape and removes meaningful duplication;
- affected dependents have been reviewed when owned semantics changed;
- current meaning does not depend on implementation, models, history, campaign state, or conversation context as hidden authority;
- verification and evidence claims remain within what they establish.

A successful reconciliation may conclude that no change is needed.
