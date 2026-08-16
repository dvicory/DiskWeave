---
name: diskweave-architecture-normalization
description: Normalize a settled DiskWeave architecture revision or bounded architecture delta into durable product semantics without losing source meaning or duplicating semantic reconciliation. Use for architecture adoption, architecture-source retirement, or bounded architecture changes before behavior-changing implementation.
---

# DiskWeave architecture normalization

Use this skill to move consequential meaning out of a temporary architecture source and into its proper durable homes.

Architecture normalization answers:

> What consequential meaning does this architecture source contain, how does it affect useful operator behavior, what requires semantic reconciliation, and what must be true before the source is no longer needed?

It does **not** decide current canonical semantic ownership. Use `diskweave-semantic-reconciliation` for that.

The normalization artifact is temporary campaign state, not semantic authority.

## Authority

Keep these roles distinct.

- Current canonical OpenSpecs define current required product behavior.
- A reviewed active OpenSpec change defines target behavior within its scope until verified and synced.
- Architecture defines constitutional constraints and target intent under its declared authority boundary. It does not silently override current semantics.
- Implementation shows realized behavior, not semantic authority.
- Tests, models, and evidence may expose gaps or limit claims; they do not manufacture missing semantics.
- Milestones, Beads, normalization records, reports, and historical artifacts are planning, work, evidence, or rationale rather than current product authority.

Never implement changed product behavior directly from architecture prose.

Use existing repository workflows rather than reproducing them here:

- `diskweave-knowledge` for deterministic current context, relationships, impact, affected dependents, history, and readiness;
- `diskweave-semantic-reconciliation` for current canonical meaning, ownership, composition, contradiction, gaps, and dependent semantic review;
- normal OpenSpec workflows for accepted target behavior;
- milestone planning and Beads for execution planning and work.

## Normalize product meaning, not document structure

Organize architecture work around useful operator or system outcomes, not architecture headings, implementation mechanisms, or one change per invariant.

Prefer the smallest capability that can be used truthfully and safely.

Ask:

> If this capability became available now, are the semantics required for its first legal uses, refusals, uncertainty, recovery, and claims sufficiently determined?

If not, the capability boundary is incomplete.

Do not merge independently useful capabilities merely to make a larger slice. Do not split semantics merely because the source uses several headings or mechanisms.

Split source material when its parts can legitimately receive different current/target conclusions, decisions, lifecycles, durable homes, or completion times.

A deferred future feature must not swallow an independently applicable baseline refusal, authority limit, preservation rule, compatibility rule, lifecycle obligation, or claim boundary.

## Discover inherited obligations

A new transition, authority path, recovery path, publication path, or lifecycle does not reset obligations that were already active.

For a behavior-changing target path, determine which existing states can enter it and what may still be live there, such as blockers, indeterminate state, preservation duties, recovery obligations, or retained claims.

Ask:

> Could a currently conforming implementation enter this new path and thereby bypass an obligation that would otherwise still apply?

If yes or unresolved, include that obligation in semantic reconciliation and target definition.

Do not decide locally whether the obligation is current-owned, superseded, or incomplete. That is a semantic-reconciliation question.

## Account for the complete architecture source

The architecture relationship graph and marked `arch.*` invariants are useful review inputs, but they are not proof that every consequential source semantic has been found.

For a whole architecture revision or source retirement:

1. account for every marked invariant;
2. identify consequential unmarked normative material;
3. perform an independent beginning-to-end source walk after the working map has been built.

A consequential semantic is one whose removal could change required success, refusal, authority, uncertainty, recovery, preservation, compatibility, lifecycle/progress, persistent interpretation, or permitted claims.

The independent source walk exists because a ledger cannot prove its own completeness.

For a bounded architecture delta, inspect the complete delta, affected invariants, affected operator outcomes and entry paths, and inherited obligations. Do not reopen unrelated architecture.

## Use semantic reconciliation as the decision boundary

Normalization may identify:

- the architecture predicate or question;
- its source anchor;
- affected operator outcomes;
- relevant invariants;
- plausible current semantic review surfaces;
- inherited obligations;
- important success, refusal, uncertainty, recovery, lifecycle, or claim dimensions;
- whether the architecture appears current-like, target-like, deferred, or obsolete as a question.

These are review inputs, not current-semantic verdicts.

Before making a consequential terminal conclusion that depends on what current canonical semantics do or do not require, use bounded `diskweave-semantic-reconciliation`.

This includes conclusions equivalent to:

- current semantics already determine the architecture predicate;
- the architecture contains a genuine target semantic delta;
- current canonical behavior is contradictory or missing;
- no current semantic debt exists for the reviewed scope;
- an existing semantic owner must evolve or a distinct owner may be needed;
- a source semantic can safely be discarded because current semantics already cover it;
- a capability touching existing policy is semantically ready for target canonicalization.

Treat architecture as non-authoritative context in that review. Reconciliation establishes current semantics first, then compares the architecture proposition with them.

Several independently owned decisions may compose to entail a broader architecture proposition without requiring an umbrella semantic owner.

Record only enough of the reconciliation result for a fresh agent to reproduce or locate the conclusion. Do not copy its reasoning into normalization campaign state.

## Distinguish target intent from current debt

Failure to find a current owner locally does not establish current debt.

Current debt requires both:

1. independent accepted current authority already requires the semantic; and
2. semantic reconciliation establishes that current canonical semantics are missing, contradictory, or insufficient.

Architecture target intent alone cannot create current debt. Conversely, architecture non-authority must not hide independently established current debt.

When current product scope remains unresolved, keep that uncertainty explicit rather than forcing the semantic into current debt or target-only status.

## Keep campaign state proportional

Record only what another capable agent needs to resume safely.

For each still-relevant source semantic or cluster, this will usually be some subset of:

- source anchor or invariant identity;
- concise meaning;
- affected operator outcome;
- reconciliation conclusion or open semantic question;
- target/deferred/removal disposition;
- durable home, once one exists;
- genuine unresolved decision.

Do not copy deterministic relationship graphs into campaign prose. Do not invent permanent IDs for temporary architecture clusters. Do not require fixed tables, row counts, state taxonomies, or report sections.

A fact already durably settled and reproducibly discoverable does not need permanent campaign bookkeeping.

## Advance incrementally

The whole architecture does not need to be normalized before useful implementation can proceed.

For a selected capability:

1. understand the operator outcome and truthful claim;
2. identify relevant architecture meaning, invariants, entry paths, and inherited obligations;
3. scope and run bounded semantic reconciliation where current meaning matters;
4. resolve genuine constitutional or product ambiguity rather than inventing it locally;
5. establish the minimum coherent target semantics through the repository's normal semantic workflow;
6. recheck the operator outcome and inherited obligations against the accepted target;
7. update only affected campaign state.

Do not preselect requirement split/merge, new semantic owner, existing-owner evolution, executable-spec ownership, or exact change boundaries before semantic reconciliation establishes the semantic shape.

Under the current knowledge architecture, executable specifications and models are verification/design artifacts rather than canonical product authority. If reconciliation concludes that a bounded executable transition owner would materially reduce duplicated normative semantics, record that as a candidate knowledge-system semantic change; do not assume the authority transition has already occurred.

Implementation may expose missing semantics or invalid architecture assumptions. Feed those findings back to the artifact that owns the issue; do not treat implementation as authority.

## Source retirement

The architecture source may leave ordinary semantic use only when:

> No consequential normative semantic depends on the source as its only durable home.

Establish three things.

### Complete coverage

Every consequential marked and unmarked source semantic has been accounted for.

### Durable disposition

Each consequential semantic has an appropriate durable outcome: current semantics already determine it, accepted target semantics own it, it is durably retained as future/deferred intent, it was deliberately removed, it is genuinely non-normative, or it remains explicitly unresolved and therefore blocks retirement as appropriate.

Any disposition that depends on current canonical meaning has an applicable semantic-reconciliation basis.

### Source independence

A capable fresh agent can determine required current behavior, authority, refusal, uncertainty, recovery meaning, and claim limits without treating the retiring architecture source or conversation history as authority.

For retirement of a monolithic architecture source, prefer an actual bounded reconstruction or source-exclusion check when the repository supports one. The proof obligation matters; this skill does not prescribe its implementation.

## Quality test

A good normalization pass lets a fresh agent answer:

> What useful operator behavior is being advanced, what architecture meaning constrains it, what current semantic conclusions were established by reconciliation, what target or deferred meaning still needs a durable home, what inherited obligations remain, and what architecture meaning is still preventing source retirement?

If answering that requires re-performing semantic reconciliation inside normalization, guessing from implementation, or rereading retired architecture for current required behavior, normalization is incomplete.
