---
name: diskweave-semantic-reconciliation
description: Determine and repair DiskWeave semantic ownership when requirements overlap, compose, conflict, leave consequential behavior underdetermined, or may benefit from delegated executable semantics. Use bounded review by default and make the smallest durable semantic repair.
---

# DiskWeave semantic reconciliation

Use this skill when repository facts are available but judgment is required to determine what the product contract actually says and which requirement owns each decision.

Semantic reconciliation answers:

> What behavior is being decided, what do current canonical semantics require, who owns each independently meaningful decision, and what is the smallest repair if the answer is ambiguous, duplicated, contradictory, or incomplete?

Follow the repository authority rules. For current behavior, start from current canonical OpenSpecs. When reviewing work inside an active change, keep the current contract distinct from that change's target contract.

Use the repository's knowledge tooling for deterministic context, relationships, and affected dependents. Do not treat retrieval output as a semantic verdict.

## One decision, one owner

Each independently meaningful product decision should have one canonical OpenSpec owner. That owner may define the decision directly or explicitly delegate a bounded part of its exact semantics to a canonical executable model.

Other requirements may refine that decision, compose it with independently owned decisions, map a platform mechanism into it, or constrain it at an architecture, security, or evidence boundary. They should not independently redefine the same policy.

A broader proposition may be completely determined by several independently owned decisions. Do not add an umbrella owner unless the broader proposition adds its own policy.

## Models and executable specifications

Executable specifications and models can make bounded state, ordering, crash/recovery behavior, interleavings, and invariants precise.

A delegated model defines the exact bounded semantics named by its OpenSpec owner. The requirement retains ownership, meaning, assumptions, scope, and product-facing boundaries. Other models are not semantic authority.

Prefer delegation when a model can replace duplicated prose with a clearer exact transition or invariant definition. Do not move product interpretation, compatibility policy, operator meaning, or claim boundaries into a model merely because they can be encoded there.

Keep delegated semantics distinct from finite verification instances, exploration bounds, and other evidence-only configuration.

Creating, changing, or removing delegated semantics is a semantic change. Keep current and proposed model semantics distinct through the normal OpenSpec authority transition.

If a non-authoritative model and current semantics disagree, reconcile the product contract first, then bring the model or implementation into conformance as appropriate.

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
- After an ownership change, re-test each affected requirement for independent semantic responsibility. A requirement whose remaining proposition is completely determined by other owners or delegated semantics should not survive only as a restatement or stable-ID shell.

Do not choose split, merge, relocation, or a new identity before understanding the behavior being owned.

Any normative semantic repair — including correcting or removing current canonical behavior—belongs in an OpenSpec change. Start or update that change with the appropriate OpenSpec skill. Do not edit canonical specs as the repair itself; canonical specs change through the OpenSpec sync workflow. Keep explicit whether the work repairs current semantics or defines new target behavior. The OpenSpec workflow owns its proposal, design, task, sync, and archive mechanics.

When editing OpenSpec, write the product meaning directly. If exact bounded semantics are delegated to a model, make that delegation explicit instead of repeating the model in prose. Keep conditions that affect authority, durability, ordering, refusal, uncertainty, recovery, preservation, or claims explicit. Do not leave reconciliation commentary in canonical requirement prose.

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
- current meaning does not depend on implementation, non-delegated models, history, campaign state, or conversation context as hidden authority;
- verification and evidence claims remain within what they establish.

A successful reconciliation may conclude that no change is needed.
