---
name: diskweave-semantic-reconciliation
description: Determine and repair DiskWeave semantic ownership when current requirements overlap, compose, conflict, leave consequential behavior underdetermined, or interact with changed architecture, implementation, evidence, or candidate executable specifications. Use bounded review by default and seek the minimum nonduplicative durable ownership shape.
---

# DiskWeave semantic reconciliation

Use this skill when the repository can locate relevant semantic material but human reasoning is required to determine what it actually means and who owns it.

Semantic reconciliation answers:

> What are the independently meaningful semantic decisions in this bounded question, what does the current canonical system require, who owns each decision, and what minimum repair is needed?

Deterministic knowledge tooling discovers facts and review scope. It does not issue semantic-consistency verdicts.

## Core invariant

DiskWeave should converge toward unambiguous, nonduplicative semantic authority.

Under the current knowledge architecture, each independently meaningful product decision has one canonical OpenSpec owner. A broader proposition may be completely entailed by composition of several independently owned decisions without needing an umbrella requirement.

Non-owning semantics may:

- require an independently owned decision;
- refine it with an additional local constraint;
- compose several owned decisions;
- map an external mechanism into it;
- constrain it at a constitutional, security, or evidence boundary;
- provide verification or evidence for it.

They must not independently redefine the same decision.

The objective is unambiguous semantic authority, not minimum file count or minimum prose.

## Executable specifications

Executable specifications may be especially useful when correctness depends on reachable state, ordering, crash/recovery points, interleavings, or global invariants.

A useful boundary is:

> Domain semantics define what facts mean. An executable protocol can define what transitions are legal once those facts are supplied.

For example, a canonical OpenSpec owner may define what constitutes sufficient fence authority while an executable model checks that checkpoint cannot be reached without that predicate.

Under the **current** DiskWeave knowledge contract, an executable model is verification/design evidence rather than canonical product authority. Do not silently treat it as normative merely because it is more precise.

If reconciliation finds that a bounded executable transition relation would be the minimum nonduplicative owner and would allow materially duplicated prose semantics to be removed, record that as a candidate semantic/knowledge-system change. Adopting that ownership shape requires a deliberate canonical change to the current authority model first.

Do not introduce executable modeling merely because behavior can be modeled.

## Authority

Begin with current canonical OpenSpecs and the repository's explicitly governed constitutional constraints.

A reviewed active OpenSpec change may deliberately propose changed target semantics within its scope but is not silently current authority until the normal authority transition completes.

Architecture guides architectural coherence and target direction within its authority boundary. It does not silently override current product semantics.

Implementation, tests, models, fixtures, and evidence may expose nonconformance, missing semantics, hidden assumptions, or overly strong claims. They do not silently resolve contradictions or manufacture product authority.

ADRs, archives, milestones, Beads, prior architecture, verification history, and conversation context are rationale, work state, or archaeology rather than current semantic authority.

When current canonical sources materially conflict, preserve the conflict until it is deliberately repaired.

## Scope

Use bounded reconciliation by default.

Start from the named semantic question and retrieve the current canonical context needed to understand its owners, prerequisites, refinements, and affected dependents using `diskweave-knowledge`.

Broaden only when the semantic dependency itself requires it.

A whole-system audit is the same reasoning applied across all relevant connected semantic clusters. Do not turn a bounded question into a general audit merely because the repository is large.

## Understand the complete semantic question

Reason about complete consequential behavior rather than comparing similar sentences.

As relevant, understand:

- subject or resource;
- preconditions and admission;
- authority or evidence consulted;
- allowed action or decision;
- ordering or state transition;
- success meaning;
- refusal, failure, and uncertainty behavior;
- crash/restart/recovery behavior;
- preservation and lifecycle consequences;
- persistent or externally observable interpretation;
- permitted claims and explicit non-claims.

Do not require a fixed worksheet when ordinary reasoning is sufficient.

Success-path agreement is not enough when consequential failure or uncertainty behavior differs.

## Determine the ownership shape

For each independently meaningful decision, determine the smallest natural durable owner under current repository authority.

Healthy shapes commonly include:

- one owner plus a narrower refinement;
- composition of independently owned decisions;
- adapter or platform conformance to portable semantics;
- cross-cutting constitutional, security, or evidence constraints;
- a canonical domain predicate consumed by a separate executable verification model.

Semantic defects commonly include:

- duplicate ownership of one actual policy;
- contradictory canonical behavior;
- consequential underdetermination or a missing owner;
- historical or implementation authority leakage;
- implementation nonconformance;
- consequential shadow semantics outside canonical authority.

These are reasoning categories, not a required output taxonomy.

### Composition versus distributed ownership

Do not create an umbrella owner merely because a useful proposition is derived from several requirements.

If A, B, and C are independently owned decisions and their valid composition completely entails P, P needs no additional owner.

But if P adds its own ordering, authority choice, refusal rule, transition, lifecycle rule, or other independent semantic decision, that added policy needs an owner.

### State-machine boundary

A protocol model may precisely represent legal transitions without owning every domain predicate consumed by those transitions.

Do not move a predicate into an executable model merely because the model branches on it.

Do not maintain the same exact transition semantics normatively in several prose owners and then add a model as another copy. Under current authority, the model remains evidence; if making it normative would remove meaningful duplication, surface the required authority change explicitly.

## Challenge material ambiguity

Use these questions when ownership or completeness is consequential or unclear.

### Material-divergence test

> Could two implementations satisfy the current canonical semantics yet behave materially differently on this question?

If yes, determine whether the difference is legitimate delegated choice or consequential underdetermination/conflict.

### Deletion test

> If the non-owning descriptions disappeared, would each independently meaningful semantic decision still be completely defined by its owner?

If not, important semantics remain scattered.

For a candidate executable owner, ask the analogous future-state question: would adopting it allow materially duplicated normative transition prose to disappear? If not, the model is probably verification evidence rather than a better semantic owner.

### Failure/uncertainty challenge

> Do the apparent owners agree on consequential refusal, uncertainty, recovery, abandonment, preservation, and lifecycle behavior, not only successful execution?

Only examine dimensions that matter to the bounded question.

### Fresh-agent test

> Can a capable fresh agent identify the rule and its authority without implementation, historical artifacts, campaign state, or conversation context?

If not, the semantic system still depends on a hidden second authority.

## Inspect implementation and evidence after current semantics

Once current canonical meaning is understood, inspect implementation, tests, models, and evidence as needed.

Use them to ask:

- Does implementation conform?
- Is there a consequential rule implemented but not canonically owned?
- Do tests encode a shadow policy?
- Does a model expose a missing transition decision or duplicated prose policy?
- Does evidence claim more than the semantic contract permits?
- Is an accidental mechanism being treated as authority?

Do not weaken or rewrite clear current semantics merely to match implementation.

Use architecture after current semantics are established to evaluate intended coherence and target direction. Use history only when rationale or provenance materially helps distinguish a semantics-preserving repair from an unintended change.

## Choose the minimum repair

Make the smallest durable repair that removes ambiguity or duplicate authority while preserving intended semantics.

Possible outcomes include:

- no semantic problem; make no normative edit;
- leave valid composition alone;
- remove duplicated prose and reference the actual owner;
- reduce a requirement to its genuine local refinement;
- relocate ownership while preserving semantic identity when appropriate;
- split genuinely independent decisions;
- merge artifacts that no longer have independent semantic responsibility;
- add genuinely missing consequential behavior;
- deliberately change or retire obsolete behavior;
- fix implementation rather than changing correct semantics;
- recommend a bounded executable transition owner as a future authority-model change when it would materially reduce semantic duplication.

Do not choose split, merge, relocation, new identity, or executable ownership before understanding the semantic shape.

Use the repository's normal OpenSpec workflow for normative edits. This skill does not prescribe proposal structure, `design.md`, task decomposition, or artifact mechanics.

## Preserve semantic identity deliberately

When semantics move or wording changes, distinguish semantic identity from textual location.

Preserve a stable `req.*` identity when it still denotes materially the same durable obligation.

Do not preserve an identity merely to avoid migration when the obligation has materially changed. Do not create a new identity merely because prose moved between artifacts or because a model now provides additional verification evidence.

If DiskWeave later deliberately adopts executable semantic ownership, its identity and relationship model must be defined by that canonical change rather than invented locally by reconciliation.

## Review affected dependents

When an owned semantic decision changes, re-review affected refiners, composers, adapters, claims, implementation links, and evidence through the repository's normal affected/dependent workflow.

Do not bulk-assume dependents remain valid because tests pass.

A semantics-preserving relocation may require less downstream change than a semantic correction, but that conclusion should be explicit.

## Completion

A bounded reconciliation is complete when:

1. each independently meaningful decision in scope has an identifiable canonical owner under current authority, or the remaining ambiguity is explicitly unresolved;
2. valid refinement, composition, adapter, constitutional, and verification-model boundaries do not independently redefine owned decisions;
3. two conforming implementations cannot materially disagree because of an unintended gap or contradiction in the reviewed semantics;
4. consequential failure and uncertainty behavior is either determined or explicitly unresolved;
5. normative repairs use the minimum durable ownership shape and remove meaningful duplication;
6. affected dependents have been reviewed when owned semantics changed;
7. current meaning does not depend on implementation, models, historical artifacts, campaign state, or conversation context as hidden authority;
8. verification and evidence claims remain within what they actually establish.

A successful result may be "no change needed."

Do not create semantic work merely to demonstrate that reconciliation was performed.

## Quality test

A good reconciliation lets a fresh agent answer:

> What are the actual semantic decisions here, who owns each one, how do other capabilities refine, compose, map, constrain, or verify them, what happens under consequential uncertainty, and what—if anything—must change?

If the answer requires reproducing a large reconciliation report or remembering the process that discovered the result, durable semantic ownership is still too dependent on the review itself.
