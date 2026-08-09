# Goal v5 — Normalize DiskWeave Knowledge and Build Durable Human/Agent Projections

## Objective

Converge DiskWeave onto a durable information architecture in which there is **one current semantic truth** and every architecture reference, human guide, assurance view, scenario explanation, contributor view, and AI-agent context is a reproducible projection of that truth.

Do not optimize for fixing the current mdBook, reducing JSON, improving prose, preserving already-implemented documentation machinery, or producing one attractive demonstration page.

The current documentation implementation has exposed architectural problems that must be corrected together:

* the dense hand-authored architecture remains a competing source of semantic authority;
* canonical OpenSpecs are not yet sufficiently first-class;
* historical/bootstrap semantics leak into current documentation generation;
* `docs/` contains substantial duplicated machine state and semantic registries;
* provenance is too coarse;
* the Human Guide is technically grounded but pedagogically poor;
* scenario and assurance projections often describe what they should contain rather than rendering the actual useful artifacts;
* AI context selection is not yet clearly superior to giving an agent the architecture plus relevant specs;
* preservation mechanisms such as `PATCH` are weaker in implementation than in design;
* current usefulness checks demonstrate structural presence, not reader or agent usefulness.

Repair the **whole information architecture**, then prove it through both a genuinely useful human learning path and genuinely useful task-specific agent context.

---

# 1. End-state architecture

DiskWeave SHALL converge toward:

```text
                         CURRENT SEMANTIC TRUTH

                            openspec/specs/
                                  │
                 ┌────────────────┴────────────────┐
                 │                                 │
        architecture constitution           capability specs
       cross-cutting invariants          locally owned semantics
                 │                                 │
                 └────────────────┬────────────────┘
                                  │
                       code / schemas / scenarios
                         verification / evidence
                                  │
                                  ▼
                         semantic context layer
                                  │
          ┌───────────────────────┼────────────────────────┐
          │                       │                        │
          ▼                       ▼                        ▼
   Human projections       Agent projections       Reference/evidence
   pedagogical/selective    task-specific/exact     comprehensive/queryable
```

Other repository artifacts have distinct roles:

```text
ADRs
    = why a current decision exists

openspec/changes/
    = how current semantics are being changed

openspec/changes/archive/
    = historical implementation/change record

VP-style properties
    = durable properties that should hold

actual tests/models/harnesses/scenarios/evidence
    = why a property is believed

OS/VE/task/goal identifiers
    = work organization, not permanent semantic identity

handoffs and old architecture revisions
    = historical/bootstrap material
```

Generated projections may intentionally repeat semantics for a particular audience.

They SHALL NOT become independently maintained semantic authority.

---

# 2. Architecture v0.6–v0.8 lifecycle

Keep architecture v0.6, v0.7, and v0.8 checked into the repository.

Do not decompose, delete, or rewrite those historical documents merely to satisfy this goal.

Their lifecycle SHALL become explicit.

## 2.1 v0.6 and v0.7

Treat v0.6 and v0.7 as historical architecture snapshots.

They remain useful for archaeology, rationale, regression investigation, and explicitly requested historical context.

They SHALL NOT participate by default in:

* current semantic source discovery;
* generated-document freshness;
* AI implementation context;
* current architecture projections;
* normative conflict resolution.

## 2.2 v0.8

v0.8 remains a temporary semantic authority/input until a normalization audit proves all current normative content has a durable home.

Do not demote v0.8 merely because canonical OpenSpecs exist.

Perform a semantic normalization audit over v0.8.

Every normative or correctness-sensitive semantic unit MUST receive one disposition:

```text
canonical capability requirement
cross-cutting architecture-constitution requirement
ADR rationale
verification property / evidence relationship
non-normative explanatory material
historical / superseded
MISSING durable owner
```

`MISSING` is a real architecture gap and requires corrective OpenSpec/ADR work.

The audit must detect semantic loss, not merely match headings or keywords.

When all current v0.8 semantics have durable homes and the required evidence passes, v0.8 may become historical like v0.6/v0.7.

At that point:

> v0.8 should effectively become the last hand-authored encompassing architecture revision.

Future architecture evolution should happen through canonical OpenSpecs, architecture-constitution changes, ADRs, and evidence—not by creating v0.9 merely to reassemble them.

## 2.3 Historical architecture remains available but opt-in

The semantic/context tooling MUST exclude historical architecture and handoffs by default.

Historical sources may be explicitly requested for questions such as:

* Why did this decision change?
* What did v0.7 say?
* Which architectural transition introduced this behavior?

They may not silently influence current implementation semantics.

## 2.4 Normalization audit state is transitional

Do not create another permanent `architecture-coverage.json`-style semantic database merely to track this migration.

The normalization process may use generated indexes/maps under tooling state while the migration is active.

Once permanent stable identities and canonical owners exist, the migration mapping should be reconstructible or disposable.

---

# 3. Make canonical OpenSpecs first-class

Detailed current DiskWeave semantics SHALL primarily live in canonical:

```text
openspec/specs/*/spec.md
```

not in a separately maintained documentation semantic database.

The tooling should discover canonical specs automatically.

Do not require humans or agents to manually duplicate every canonical spec into a second `sources`, `requirements`, or equivalent registry.

## 3.1 Exactly one canonical owner

Every current semantic contract should have exactly one canonical semantic owner.

Example:

```text
"uncertain recovery evidence cannot justify CLEAN"

canonical owner:
    one canonical requirement

referenced by:
    Human Guide
    Architecture Reference
    Assurance Atlas
    crash scenarios
    agent context
```

The projections may explain the contract differently, but none may redefine it.

## 3.2 Stable requirement identity

Fine-grained generation/freshness/provenance should operate on canonical requirement-level identities rather than entire files where practical.

Inspect current OpenSpec capabilities before inventing a mechanism.

If stable requirement IDs are insufficient, add the least-invasive intrinsic identity scheme.

Permanent semantic identities MUST NOT derive from:

* OS numbers;
* VE numbers;
* autonomous goal numbers;
* archived change paths;
* Markdown line numbers;
* architecture-version line positions.

Renaming/reordering text should not unnecessarily destroy requirement identity.

## 3.3 Small architecture constitution

Retain a canonical architecture-level specification for genuinely cross-cutting contracts only.

It may own matters such as:

* what DiskWeave fundamentally is and is not;
* global safety philosophy;
* fail-closed handling of uncertainty;
* major layering/dependency rules;
* global identity distinctions;
* evidence-strength philosophy;
* compatibility/format governance;
* global product non-goals;
* authority/change model.

It SHOULD NOT duplicate capability-local details such as:

* exact transaction state machines;
* exact recovery transition rules;
* checksum-state mechanics;
* degraded-read eligibility;
* scrub/repair algorithms;
* metadata-loss procedures.

Those belong to their canonical capability specs.

Remove historical bootstrap/work-program requirements from current architecture authority as their semantics are normalized.

---

# 4. Simplify the documentation repository

The current documentation system SHALL be simplified around long-term repository comprehension.

Do not make “replace JSON with TOML” the objective.

The objective is:

> A human or AI opening `docs/` should see documentation, not a second requirements-management database.

Audit all current state under areas such as:

```text
docs/model/
docs/state/
docs/generated/
```

and classify every artifact as:

1. human-maintained durable configuration;
2. irreducible durable machine acceptance state;
3. reproducible/generated execution state.

Converge toward something conceptually like:

```text
docs/
    book.toml
    src/
        guide/
        scenarios/
        assurance/
        contributors/
        reference/
        documentation-system/

    docs.toml              # or similarly small ergonomic curriculum/config
    prompts/               # trusted prompt contracts if retained

.dwv-docs.lock             # optional compact machine-owned accepted state

target/dwv-docs/
    source-graph/
    inventories/
    context-packs/
    plans/
    tasks/
    responses/
    generated-indexes/
    diagnostics/
```

Exact paths and formats are implementation decisions.

Required properties:

* reconstructible state does not clutter permanent `docs/`;
* authoritative information is not copied into multiple registries;
* machine-generated diagnostics/tasks/responses normally live outside source-controlled documentation;
* durable machine state is compact and clearly machine-owned;
* committed human-maintained configuration is small enough to understand directly;
* adding a parser dependency is preferable to permanently degrading repository maintainability.

Prefer deleting redundant state to implementing synchronization between redundant state.

---

# 5. Distinct projections for distinct consumers

Do not attempt to make a single mdBook page hierarchy simultaneously optimize for newcomer learning, exhaustive reference, assurance, contributor lookup, and autonomous implementation.

The semantic graph may back multiple projections.

## 5.1 Human Guide

Purpose:

> Teach DiskWeave to a strong software engineer who is not a filesystem/storage developer.

Optimize for comprehension rather than semantic coverage.

Prefer causal progression:

```text
What problem does DiskWeave solve?
→ tiny concrete pool
→ ordinary read
→ ordinary write
→ interrupt the write
→ why durable intent/dirty state exists
→ parity
→ why parity is not integrity
→ degraded read
→ rebuild
→ corruption
→ scrub / repair / recovery / rebaseline
→ identity/topology/generalization
→ why the architecture is structured this way
→ why we believe the behavior is safe
```

Concrete problems should motivate abstractions.

Do not begin with a taxonomy of all DiskWeave concepts merely because those concepts are important.

The Human Guide is allowed to omit details covered by the Architecture Reference.

## 5.2 Architecture Reference

Purpose:

> Provide a complete, current, searchable encompassing view of DiskWeave architecture.

It may be dense.

It should be generated from the canonical semantic graph and architecture constitution.

It is a projection, not normative authority.

This projection replaces the long-term need for hand-authored v0.9/v1.0 monolithic architecture documents.

## 5.3 Scenario Book

Purpose:

> Teach temporal correctness through concrete executable histories.

Scenario entries SHOULD be based on real simulator, normalized-trace, model, or equivalent executable facts.

Render actual material such as:

```text
initial persistent state
request
transition
durability point
fault injection
post-crash observations
legal recovery result
forbidden inference
```

The LLM explains these facts.

It does not invent them.

Do not produce meta-pages whose primary content is an explanation of how scenario pages are supposed to work.

## 5.4 Assurance Atlas

Purpose:

> Explain what DiskWeave claims, under what assumptions, and what evidence supports each claim.

Render actual structured relationships:

```text
claim
fault model / assumptions
architectural mechanism
invariant/property
concrete evidence artifacts
evidence strength
explicit non-claims
known gaps
```

Do not organize primarily around VE work-program IDs.

Prefer actual durable properties and actual evidence artifacts.

## 5.5 Contributor Map

Purpose:

> Help humans and agents locate implementation ownership.

Derive mechanically where possible:

* crates;
* dependency direction;
* semantic ownership;
* relevant Rust source entry points;
* prohibited dependency/ownership relationships;
* canonical requirements implemented;
* related scenarios/evidence.

LLM prose may add small navigational explanations.

It should not manually maintain facts available from Cargo/source metadata.

## 5.6 Agent Context

Purpose:

> Give an implementation/review agent the exact context required for a task.

The normal context packet should contain:

```text
task/change intent
relevant canonical requirements
relevant cross-cutting invariants
implementation ownership/source entry points
relevant scenarios
failure/forbidden outcomes
relevant evidence
```

Human Guide prose should generally NOT be part of implementation-agent context.

Architecture Reference prose should generally NOT be part of implementation-agent context when canonical requirements are available.

Do not make agents read duplicated explanatory projections to recover normative semantics.

---

# 6. Human curriculum must encode pedagogy

Keep a small explicit human curriculum, but do not let it become another semantic requirements database.

For each major human chapter/concept, support the minimum information necessary to define the teaching intent, such as:

* intended reader starting knowledge;
* question being answered;
* learning outcome;
* prerequisites;
* concepts introduced;
* concepts deliberately deferred;
* misconception(s) to correct;
* canonical source requirements;
* executable examples/scenarios;
* required teaching devices when useful:

  * timeline;
  * state snapshot;
  * diagram;
  * comparison;
  * failure injection;
  * worked example;
* target projection/surface.

Do not encode information in the curriculum that can be derived from canonical sources.

Do not treat “the concept is mentioned somewhere” as successful teaching.

---

# 7. Gold-standard human proving gate

Before scaling regenerated prose across the whole Guide, prove the new projection contract with a difficult concrete chapter.

Use a temporal correctness topic such as:

# A write, interrupted

Start from a tiny concrete DiskWeave setup.

Follow one mutation through meaningful cut points.

At each point make the reader able to answer:

```text
What has happened?
What has reached durable storage?
What may the caller believe?
What would recovery observe?
What is still uncertain?
What must DiskWeave refuse to infer?
```

Introduce concepts such as dirty intent, generation, durability evidence, checkpoint, or recovery state only once the reader has encountered the problem they solve.

Where executable scenario machinery exists, state/timeline facts MUST come from it.

The resulting chapter MUST expose meaningful but unobtrusive semantic provenance.

This is an architecture proving gate, not the endpoint.

If this chapter does not genuinely improve understanding for the target reader, stop expansion and fix the projection/curriculum/generation model.

Do not simply ask a model for friendlier wording.

Once this contract passes, use it to build the rest of the initial Human Guide.

---

# 8. Semantic provenance

Replace coarse:

```text
generated block
    → bag of source documents
```

with meaningful provenance:

```text
claim / stable fragment
    → canonical requirement(s)
    → relevant scenario(s)
    → evidence where applicable
```

Hashes remain important for machine freshness but are not sufficient semantic provenance.

The system must be able to explain:

* why a claim appears;
* which exact canonical contracts support it;
* what scenario illustrates it;
* which evidence supports a safety/correctness statement;
* which source change caused reassessment.

Rendered documentation should make provenance inspectable without overwhelming the narrative.

For example:

```text
Sources / Why this is true
    Never-false-clean requirement
    Intent-before-mutation requirement
    Crash-at-cut-point scenario
```

may expand into exact identifiers.

OS/VE/goal/handoff identifiers should not be permanent provenance when a canonical requirement or evidence artifact exists.

---

# 9. Preservation-first maintenance must be real

Audit and repair the implementation semantics of:

```text
NEW
KEEP
PATCH
REPLACE
CONFLICT / NEEDS_SOURCE
```

Required meanings:

## NEW

No accepted prose exists.

Generation occurs through the normal bounded context → structured response → grounding/review → acceptance path.

Do not make manual prose + `adopt` the normal bootstrap workflow.

## KEEP

Existing accepted prose remains byte-identical.

A source may be rebound after explicit semantic assessment if its change does not affect the explanation.

## PATCH

Only stable affected fragments are changed.

Unrelated accepted fragments remain byte-identical.

Do not call whole-block replacement a PATCH.

Use preimage hashes/stable fragment IDs or an equivalently strong mechanism.

## REPLACE

Exceptional broad rewrite.

Requires explicit semantic or pedagogical justification.

Acceptable reasons include:

* concept model fundamentally changed;
* previous projection proven pedagogically defective;
* audience/scope materially changed.

A model or prompt upgrade is not sufficient justification.

## Provider/model/prompt changes

Changing:

* model;
* provider;
* temperature;
* default wording;
* prompt style;

must NOT automatically stale accepted prose.

A genuine prompt-contract defect may trigger an explicitly scoped review campaign.

There SHALL be no routine “regenerate the whole book with the new model” operation.

---

# 10. Agent-context proving gate

Select one real DiskWeave implementation task/OpenSpec after the semantic-context layer is repaired.

Generate a context packet.

Compare it with the practical previous baseline:

```text
v0.8
+
relevant OpenSpecs
+
manual repository/source discovery
```

The generated task context must demonstrate:

* no known correctness-sensitive semantic omission;
* substantially less irrelevant content;
* exact provenance/reason for selected requirements;
* relevant implementation ownership;
* relevant failure/scenario information;
* relevant evidence/forbidden outcomes;
* deterministic/reproducible selection.

A packet created by concatenating whole specs until a token/byte bound is reached does not pass.

Prefer graph/stable-identity selection.

Do not build embeddings, vector databases, or general RAG infrastructure unless concrete evidence shows the structured graph is inadequate.

---

# 11. Usefulness and correctness are separate dimensions

The tooling must stop treating structural presence as equivalent to usefulness.

## Deterministic checks should continue to validate things such as:

* source identities;
* coverage;
* stale projections;
* provenance existence;
* required executable scenario artifacts;
* graph consistency;
* safe rendering;
* no unauthorized broad rewrite;
* no-op stability;
* privacy/source bounds.

## Qualitative evaluation should assess:

### Human questions

Examples:

* What happens when I write through DiskWeave?
* Where can a power loss matter?
* Why doesn't ordinary completion necessarily imply durability?
* Why can a region remain dirty?
* Why isn't parity enough to identify corruption?
* What is the difference between degraded read, rebuild, repair, scrub, and rebaseline?
* What does metadata loss destroy and what does it leave intact?

### Agent tasks

Use real repository implementation/review tasks.

Qualitative review may use an external model or human reviewer.

Do not misrepresent such review as deterministic proof.

Keep a small stable evaluation corpus so projection quality can regress visibly.

---

# 12. Render the useful data we already have

If the system already has:

* executable scenario artifacts;
* structured assurance records;
* Cargo-derived crate/dependency data;
* requirement/source links;

then the projections MUST render those useful artifacts directly.

Do not write prose that merely says:

> The Scenario Book contains executable scenarios.

Show the scenario.

Do not write:

> The Assurance Atlas maps claims to evidence.

Show the claim/evidence map.

Do not write:

> The Contributor Map explains crate ownership.

Render the actual current ownership/dependency structure.

Meta-documentation about a projection belongs in the documentation-system reference, not as a substitute for the projection.

---

# 13. Self-documentation and handoff extinction

The documentation/context system must remain self-documenting.

Its durable behavior should be discoverable from:

* canonical docs-system OpenSpecs;
* ADRs;
* small human-maintained config;
* prompt contracts;
* schemas;
* Rust implementation;
* command help;
* tests/evidence;
* generated documentation-system reference.

This goal, previous handoffs, and chat context SHALL NOT become semantic dependencies.

Keep existing bootstrap/handoff material checked in as history if useful.

Exclude it from current semantic extraction by default.

Completion requires proving that a future agent can determine:

* current source authority;
* how projections are selected;
* why a block exists;
* why it is stale;
* what provenance supports it;
* how to regenerate/update it;
* how to add a new canonical concept;
* how to recover after generated state is deleted;

without this goal document being the only source.

The goal document itself is eventually disposable as operational guidance once its enduring contracts have been transferred to canonical artifacts.

---

# 14. OpenSpec and ADR execution

The agent SHALL inspect current repository state and create/amend/supersede the smallest coherent dependency-ordered set of OpenSpec changes and ADRs necessary to make this goal real.

Do not mechanically create one OpenSpec per section in this goal.

Do not retain an existing documentation-system decision merely because implementation exists for it.

Supersede or simplify prior docs-system OpenSpecs/ADRs when evidence shows that they encode the wrong architecture.

Likely work areas include:

* current architecture normalization;
* architecture constitution cleanup;
* stable canonical requirement identity;
* source-graph simplification;
* permanent-vs-generated repository state;
* human curriculum/projection semantics;
* claim-level provenance;
* true fragment PATCH;
* executable Scenario Book rendering;
* Assurance Atlas rendering;
* deterministic contributor projection;
* task-specific agent context;
* usefulness evaluation;
* migration/removal of redundant docs databases;
* self-documentation/recovery.

Actual decomposition must follow repository dependency evidence.

Do not stop after creating the OpenSpecs.

Execute them in dependency order and continue until this goal’s completion criteria are satisfied.

---

# 15. Preserve good existing properties

Do not throw away good safety properties merely because the current output is poor.

Preserve or reimplement:

* `cargo xtask docs` as development tooling;
* separation from production `dwv`;
* model/network-free CI checks;
* explicit source authority;
* privacy/source-root/context bounds;
* semantic normalization that ignores irrelevant formatting churn;
* no-op update behavior;
* handoff/historical-source exclusion;
* deterministic scenario extraction;
* Cargo-based implementation metadata;
* failure-safe/atomic generation;
* provider-neutral generation protocol where it remains useful;
* source and provenance inspection commands.

Simplification may change their storage representation or implementation.

Their guarantees matter more than preserving old internal APIs.

---

# 16. Explicit anti-local-optimization constraints

This goal is NOT satisfied by any one of:

* rewriting the current prose;
* adding diagrams;
* adding citations;
* replacing JSON with TOML;
* moving JSON under `target/`;
* fixing `PATCH`;
* creating one excellent chapter;
* improving the curriculum prompt;
* generating an Architecture Reference;
* cleaning `architecture-contract`;
* demoting v0.8;
* producing one agent context packet;
* adding a usefulness evaluator.

Those may all be required pieces.

The success criterion is the coherent end state.

Do not make a local fix that creates a new duplicate source of truth.

Do not add synchronization machinery where one representation can be removed.

Do not create a generic abstraction merely because several future uses are imaginable.

Do not preserve accidental complexity merely because it already has tests.

Do not optimize primarily for short-term implementation diff size.

Optimize for:

> the repository architecture we would still want after years of predominantly autonomous AI development.

---

# 17. Required evidence before completion

Leave inspectable repository evidence for all of the following.

## A. Architecture normalization

Demonstrate that every current normative v0.8 semantic unit has a durable disposition.

Show unresolved gaps explicitly.

Before demoting v0.8, prove no current semantic contract is lost.

## B. Repository simplification

Show the before/after authority/state model.

Demonstrate that reconstructible internal state no longer requires permanent duplication under `docs/`.

## C. Gold-standard human chapter

Show:

```text
canonical requirements
→ executable scenario
→ generation context
→ generated claims
→ claim provenance
→ accepted prose
→ rendered chapter
```

and qualitative usefulness evaluation.

## D. Provenance

Demonstrate multiple rendered claims resolving to exact canonical requirements and scenario/evidence artifacts.

## E. No-op stability

Change only formatting/model/provider/prompt-style metadata.

Expected:

```text
no unnecessary generation
no prose diff
```

## F. True narrow PATCH

Make one narrow semantic change.

Expected:

```text
exact dependent claims/fragments reassessed
minimal affected fragment patch
unrelated prose byte-identical
```

## G. New semantic coverage

Add a representative correctness-sensitive canonical requirement with no existing human/context binding.

The system must detect it and require an explicit disposition.

## H. Actual Scenario Book

Render at least one real executable scenario rather than a description of scenario documentation.

## I. Actual Assurance Atlas

Render at least one real:

```text
claim → mechanism → evidence → non-claim
```

chain.

## J. Agent-context comparison

For one real implementation task, compare generated context against the old monolithic baseline and document omissions/relevance.

## K. Reconstruction

Delete regenerable state and generated projections in a temporary copy.

Reconstruct successfully from permanent repository artifacts.

## L. Historical-source isolation

Demonstrate that v0.6/v0.7 and, after normalization, v0.8 do not enter current generation/context unless history is explicitly requested.

---

# 18. Completion criteria

Goal v5 is complete only when:

1. canonical OpenSpecs plus a small architecture constitution can serve as current architecture authority;
2. v0.8 has been fully normalized and may safely be treated as historical;
3. v0.6–v0.8 remain available for archaeology but are excluded from normal current-semantic context;
4. there is no second independently maintained semantic requirements database under `docs/`;
5. stable canonical requirement identities support fine-grained provenance/context/freshness;
6. the Human Guide teaches rather than compresses the architecture;
7. at least one gold-standard temporal chapter demonstrably improves target-reader understanding;
8. the initial Guide applies that pedagogical model beyond the proving chapter;
9. Scenario Book pages render actual executable facts;
10. Assurance Atlas pages render actual claims/evidence/non-claims;
11. Contributor Map facts are substantially deterministic;
12. claim/fragment-level semantic provenance is inspectable;
13. AI implementation context is sourced primarily from canonical semantics/evidence rather than generated human prose;
14. one real agent-context packet clearly improves relevance without known correctness loss;
15. `KEEP` and true targeted `PATCH` semantics work;
16. model/provider/prompt upgrades do not thrash accepted prose;
17. new critical semantics cannot silently escape coverage;
18. `docs/` is comprehensible to humans and agents without understanding a large internal generated database;
19. `cargo xtask docs check` remains deterministic and model-independent;
20. the system can reconstruct itself and its derived projections without relying on goal-v5, bootstrap handoffs, or chat history.

---

# 19. Final operating invariant

When making tradeoffs throughout this goal, preserve this invariant:

> **DiskWeave has one current semantic truth. Human documentation, architecture references, assurance material, contributor maps, and AI-agent context are audience-specific projections of that truth.**

Corollaries:

> **Duplicate explanations are acceptable. Duplicate authority is not.**

> **Historical architecture is valuable. Historical architecture is not current authority.**

> **A projection is worth maintaining only if it is genuinely better for its intended consumer than reading the canonical sources directly.**

> **Fresh documentation that nobody can understand is a failed projection.**

> **A sophisticated context compiler that cannot outperform direct canonical context is accidental complexity.**

Execute toward the durable system implied by these invariants rather than toward preservation of the current documentation implementation.
