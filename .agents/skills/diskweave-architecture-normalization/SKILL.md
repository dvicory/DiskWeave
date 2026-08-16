---
name: diskweave-architecture-normalization
description: Move consequential meaning from a settled DiskWeave architecture source or bounded architecture delta into durable product semantics without losing source meaning or duplicating semantic reconciliation.
---

# DiskWeave architecture normalization

Use this skill when architecture is temporary input that must be reconciled with the product's durable semantic owners.

Architecture normalization answers:

> What consequential meaning is in this source, what useful behavior does it affect, what requires semantic reconciliation, and what must be true before the source is no longer needed?

Follow the repository authority rules. Architecture guides target direction; it does not silently define current product behavior. Use `diskweave-semantic-reconciliation` when a conclusion depends on what current semantics own or require.

Use the repository's normal knowledge tooling to retrieve current relationships, affected dependents, history, and readiness rather than reproducing those procedures here.

## Normalize behavior, not document structure

Organize work around useful operator or system behavior, not architecture headings, implementation mechanisms, or one change per invariant.

Prefer the smallest capability that can be used truthfully and safely.

Ask:

> If this path existed now, what could an operator or caller do next, what could DiskWeave claim, and what would still have to refuse or remain uncertain?

If those answers depend on unresolved semantics, the capability is not closed yet.

Do not merge independently useful capabilities merely to make a larger slice. Split source material when its parts can legitimately have different current/target conclusions, dispositions, lifecycles, or completion times.

A deferred future feature must not swallow an independently applicable refusal, authority limit, preservation rule, compatibility rule, lifecycle obligation, or claim boundary.

## Carry forward existing obligations

A new transition, recovery path, publication path, authority path, or lifecycle does not erase obligations that were already live before entering it.

Ask:

> Could a conforming implementation enter this new path and thereby bypass an obligation that would otherwise still apply?

If yes or unclear, include that obligation in the bounded semantic review and target definition.

Do not decide locally whether an inherited obligation is already complete, needs a new owner, or is superseded. Use semantic reconciliation for that conclusion.

## Account for the complete source

Marked `arch.*` invariants and generated relationship views are useful inputs, but they are not proof that every consequential source semantic has been found.

For a whole-source normalization or retirement:

1. account for every marked invariant;
2. find consequential unmarked normative material;
3. perform an independent beginning-to-end source walk after the working map exists.

Treat source material as consequential when dropping it could change required success, refusal, authority, uncertainty, recovery, preservation, compatibility, lifecycle, persistent interpretation, or permitted claims.

For a bounded architecture delta, inspect the complete delta, affected behavior, relevant invariants, entry paths, and inherited obligations. Do not reopen unrelated architecture.

## Use semantic reconciliation for current-meaning conclusions

Normalization may identify a source proposition, affected behavior, likely current owners, inherited obligations, and the important success/failure/authority surfaces that need review.

Before concluding that current semantics already cover a proposition, leave a gap, conflict, or require an ownership change, run bounded semantic reconciliation.

Several independently owned current decisions may completely determine a broader architecture proposition. Do not create an umbrella owner merely because the architecture states that proposition directly.

Failure to find a current owner locally does not make architecture target intent into current semantic debt. Current debt must be established from current authority, not from the architecture source being normalized.

Record only enough of the reconciliation result for another agent to locate or reproduce it. Do not copy the reconciliation procedure into normalization state.

## Advance incrementally

The whole architecture does not need to be normalized before useful work can proceed.

For a selected capability:

1. understand the useful outcome and truthful claim;
2. identify the relevant architecture meaning, entry paths, and inherited obligations;
3. reconcile current semantics where they matter;
4. resolve genuine product or constitutional ambiguity through the normal semantic workflow;
5. establish the minimum coherent target semantics through the normal semantic-change workflow;
6. recheck the useful outcome and inherited obligations against that target;
7. update only the temporary normalization state needed to resume safely.

Target semantics remain target until that workflow completes its authority transition. Normalization does not make them current by editing canonical specs directly.

Do not preselect requirement split/merge, new owner, existing-owner evolution, or exact change boundaries before the semantic shape is understood.

Implementation may expose missing semantics or invalid architecture assumptions. Feed those findings back to the artifact that owns the issue; do not treat implementation as authority.

## Keep temporary state small

Record only what another capable agent needs to resume safely, such as:

- source anchor and concise meaning;
- affected behavior;
- reconciliation conclusion or open semantic question;
- target, deferred, removal, or other disposition;
- a genuine unresolved decision.

Do not copy deterministic relationship graphs into prose. Do not require fixed tables, state taxonomies, row counts, or report sections.

## Retire the source only when its meaning survives elsewhere

The architecture source may leave ordinary semantic use only when:

> No consequential normative semantic depends on the source as its only durable home.

Establish three things:

### Coverage

Every consequential marked and unmarked source semantic has been accounted for.

### Durable disposition

Each consequential semantic has an appropriate outcome: current semantics already determine it; target semantics own it; it is deliberately deferred without relying on the retiring source as the only record of retained intent; it is deliberately removed; it is genuinely non-normative; or it remains explicitly unresolved and blocks retirement as appropriate.

Any disposition that depends on current meaning must have a semantic-reconciliation basis.

### Source independence

A fresh repository-aware agent can determine required current behavior, authority, refusal, uncertainty, recovery meaning, and claim limits without treating the retiring architecture source or conversation history as authority.

For retirement of a monolithic source, use a bounded reconstruction or source-exclusion check when the repository provides a practical way to do so. The proof matters; this skill does not prescribe the mechanism.
