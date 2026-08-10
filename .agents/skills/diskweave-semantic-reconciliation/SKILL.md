---
name: diskweave-semantic-reconciliation
description: Audit and repair semantic drift across DiskWeave canonical OpenSpecs. Use when requirements or scenarios overlap, conflict, duplicate policy, depend on historical knowledge, disagree with implementation/evidence, or when preparing/reviewing cross-capability semantic work. Reconstruct one canonical owner per semantic rule, distinguish legitimate refinement from duplicate ownership, identify shadow architecture, and repair drift through bounded OpenSpec changes.
---

# DiskWeave semantic reconciliation

Use this skill to review and repair **semantic drift**.

This is the reasoning layer above DiskWeave's deterministic knowledge checks.

Deterministic tooling can establish facts such as:

* which requirements exist;
* which requirement fingerprints changed;
* which requirements require or refine another requirement;
* which implementation/evidence artifacts link to a requirement;
* which references are invalid;
* which dependents require re-review;
* which historical or removed identifiers leak into current artifacts.

It cannot determine whether two English requirements actually own the same semantic rule.

That judgment is the purpose of this skill.

## Core invariant

DiskWeave should converge toward:

> **One detailed operational semantic policy has one canonical owning requirement.**

Cross-cutting constitutional, security, and evidence requirements may constrain multiple detailed owners without becoming duplicate operational owners. They must remain high-level and must not independently define a competing operational predicate, authority decision, or state transition.

Other requirements may:

* require that fact;
* refine it with a narrower local constraint;
* compose it with other owned facts;
* map an external interface to it;
* provide evidence for it.

Refiners, composers, and adapters must not independently redefine the same detailed operational policy. Cross-cutting constraints may restrict that policy only at their own constitutional, security, or evidence boundary.

Do not mistake repetition for harmlessness merely because current wording happens to agree.

Do not mistake any textual overlap for duplication merely because two requirements mention the same event or state.

The objective is **unambiguous semantic ownership**, not minimum file count or minimum prose.

# Source roles

Use these source roles. They are not a precedence stack in which non-canonical material may override a current canonical requirement.

1. Current `openspec/specs/*/spec.md` files own current required product behavior.
2. Active OpenSpec changes contain proposed or in-flight semantics. They may explicitly propose changes to current requirements but do not become current authority until incorporated into the canonical specification set.
3. The active architecture roadmap guides future architecture, architectural coherence, dependency ordering, validation strategy, and roadmap work. It does not silently override current canonical requirements.
4. Current implementation, tests, fixtures, manifests, and executable evidence show actual behavior, assumptions, and falsifying evidence. They may reveal nonconformance or missing architecture but do not silently rewrite current requirements.
5. ADRs, archived changes, prior architecture revisions, verification history, milestones, handoffs, and other planning artifacts are historical evidence or rationale.

Implementation and history do not silently resolve a contradiction between current canonical requirements.

When the active roadmap disagrees with current canonical requirements, preserve the disagreement and determine whether it represents deliberate refinement, accidental architectural loss, an unresolved architectural change, or obsolete roadmap intent.

If two current canonical requirements materially conflict and repository evidence does not establish a semantics-preserving ownership repair, isolate the conflict rather than selecting whichever agrees with implementation, history, or roadmap prose.

# Modes

Choose one mode explicitly at the beginning of the review.

## Bounded reconciliation

Use for:

* a proposed requirement change;
* a changed requirement fingerprint;
* a suspected ownership conflict;
* a cross-capability implementation change;
* an implementation/evidence discrepancy;
* a specific requirement cluster.

Start with the named requirement and its transitive semantic owner/dependent closure.

Do not inspect unrelated capabilities unless the bounded review discovers a dependency into them.

## Full semantic audit

Use when:

* explicitly asked to reconcile or audit the canonical specification system;
* preparing a major architecture revision;
* substantial cross-cutting semantic work has completed;
* repeated drift has appeared across unrelated capabilities;
* ownership structure has materially changed;
* deterministic checks reveal widespread stale dependents.

Review all current canonical requirements as one system.

Do not convert a bounded task into a full audit merely because more files exist.

# Before reasoning

Run the repository's deterministic knowledge gates first.

At minimum:

```bash
cargo xtask docs knowledge readiness
```

For each requirement in the initial scope:

```bash
cargo xtask docs knowledge inspect <requirement-id>
cargo xtask docs knowledge context <requirement-id>
```

Use repository relationship/ownership views when available.

At a revision boundary, inspect deterministic impact information before deciding what requires semantic review.

If a required command is unavailable or fails, record that fact. Do not pretend its result.

Do not mark reviewed fingerprints resolved before completing the semantic review.

# Step 1 — Establish the canonical semantic closure

Start only from current canonical specs.

When typed `requires`/`refines` relationships are unavailable, infer the needed closure from current canonical requirement text and ownership evidence; use those relationships when they exist.

For the candidate requirement:

1. identify its stable requirement ID;
2. identify explicit `requires` relationships;
3. identify explicit `refines` relationships;
4. identify inbound dependents;
5. identify requirements governing the same state, resource, authority decision, or lifecycle boundary;
6. identify terminology whose definition is imported from another capability.

Build only enough closure to understand the semantic decision.

Do not begin with architecture history or implementation.

The first question is:

> What does the current canonical system claim?

# Step 2 — Normalize each consequential requirement into a semantic claim card

For reasoning only, normalize every relevant requirement into this shape:

```text
Requirement ID:
Capability:

Subject/resource:
Preconditions:
Authority or evidence consulted:
Decision or allowed action:
State transition:
Success postcondition:
Failure/uncertainty behavior:
Resource/lifecycle consequence:
Externally observable consequence:
Explicit non-claims:
Local refinement, if any:
```

Do not persist claim cards as a registry.

They are temporary reasoning artifacts.

Ignore stylistic wording differences.

Compare the semantic fields.

Two requirements that use very different prose may own the same semantic fact.

Two requirements that use nearly identical words may govern different states.

# Step 3 — Perform the two-conforming-implementations test

For every ambiguous pair or cluster ask:

> Could two independent implementations satisfy these requirements as written and still behave materially differently?

If no, the overlap may be harmless or a valid refinement.

If yes, determine why:

* duplicate ownership with divergent wording;
* undefined term;
* conflicting precondition;
* conflicting authority source;
* different failure/uncertainty behavior;
* different state transition;
* missing ordering;
* missing lifecycle consequence;
* missing canonical owner.

Do not call wording drift a correctness problem unless it permits materially different behavior.

# Step 4 — Classify the relationship

Classify every consequential overlap as exactly one of:

## Canonical owner + valid refinement

One requirement owns the complete policy.

Another requirement adds only a narrower constraint relevant to its capability.

Example shape:

```text
owner:
  mutation requires durable invalidation

refinement:
  this transaction machine may emit HOME_WRITE
  only after the owner's invalidation predicate succeeds
```

This is healthy.

## Canonical owner + composition

A higher-level capability combines multiple independently owned policies.

It must describe orchestration without redefining those policies.

Example:

```text
healthy service:
  requires request semantics
  requires dirty intent
  requires recovery commit
  requires XOR update
  requires store completion
```

The composer does not define what each prerequisite means.

## Canonical owner + adapter conformance

A platform or storage adapter translates an external mechanism into an existing canonical semantic contract.

The adapter owns:

* translation;
* rejection of unmappable input;
* resource binding;
* adapter-local lifecycle;
* conformance evidence.

It does not redefine the portable policy.

## Constitutional constraint + detailed owner

A cross-cutting architecture/security/evidence requirement can constrain detailed capabilities without becoming duplicate ownership.

Do not delete a useful constitution merely because lower-level requirements instantiate it.

## Drift-prone duplicate ownership

Two or more requirements independently specify materially the same:

* invariant;
* authority decision;
* state transition;
* ordering predicate;
* lifecycle rule;
* durability condition;
* repair authorization;
* failure rule.

Even if wording currently agrees, this is drift that should be repaired.

## Confirmed contradiction

Two canonical requirements permit or require incompatible outcomes for the same semantic situation.

Do not hide this behind cross-references.

## Missing canonical behavior

Correct implementation depends on a consequential rule absent from all current canonical requirements.

Inspect non-canonical sources only after confirming the absence.

## Historical-authority leakage

A current requirement depends on:

* an OS/change number;
* milestone or old planning identifier;
* architecture section number;
* named historical gate or phase;
* ADR;
* archived proposal;
* implementation symbol;
* conversation context

to determine current behavior.

## Implementation nonconformance

Canonical semantics are sufficiently clear but code does something else.

Repair implementation, tests, or evidence. Do not change the spec to make the bug conform.

## Shadow architecture

Implementation, tests, evidence, or rationale contain a consequential persistent rule that has no canonical owner.

Only classify behavior as shadow architecture when it affects:

* correctness;
* integrity;
* durability;
* recovery;
* authority;
* lifecycle/ownership;
* portability;
* compatibility;
* security;
* persistent interpretation;
* externally observable behavior;
* dependency direction.

Do not promote ordinary implementation choices.

## No semantic problem

Record this when an apparent overlap is intentionally valid.

This prevents future agents from repeatedly reopening the same question.

# Step 5 — Choose the canonical owner

When duplicate ownership exists, choose one owner.

Use these tests.

## Direct-subject test

Prefer the capability whose durable subject is the policy itself.

Example:

* dirty-region transition → dirty/integrity capability;
* recovery generation/fence authority → recovery-state capability;
* exact store persistence evidence → store-operation capability;
* request meaning → normalized request capability;
* topology identity → topology/identity capability.

## Lowest-policy test

Prefer the lowest-level reusable semantic policy that can be consumed without knowing its callers.

A service composition should not own a rule reusable by multiple services.

An adapter should not own portable semantics.

## Durable-vs-incidental test

Prefer durable product behavior over:

* experiment procedure;
* evidence procedure;
* implementation sequence;
* temporary architecture gate;
* roadmap work item;
* historical change.

## State-machine boundary test

A state-machine requirement owns **when its own actions/states transition**.

It does not automatically own the domain predicate consumed by that transition.

For example:

```text
transaction machine:
  owns when HOME_WRITE may be emitted

dirty protocol:
  owns what durable invalidation means
```

Do not collapse those into one owner.

## Evidence boundary test

Evidence requirements can constrain what may be claimed.

They cannot create product authorization merely because evidence exists.

## Adapter boundary test

Frontend/backend adapters own translation and local resource behavior.

They cannot create alternate portable identity, durability, recovery, or authority semantics.

# Step 6 — Check failure and uncertainty first

Semantic drift often hides outside the successful path.

For each apparent owner/refinement relationship compare:

* validation failure;
* partial mutation;
* ambiguous evidence;
* lost acknowledgement;
* stale generation;
* topology mismatch;
* crash;
* restart;
* abandonment;
* resource release;
* retry;
* reconciliation;
* evidence unavailable;
* conflicting evidence.

Two requirements that agree on success but disagree on uncertainty are not coherent refinements.

Do not complete an ownership review by checking only successful scenarios.

# Step 7 — Inspect scenarios separately from requirement prose

After identifying the likely owner, inspect every scenario in the cluster.

A scenario belongs with the owner when it tests the generic policy.

A scenario belongs with a refiner when it tests the **additional local constraint**.

A scenario belongs with a composer when it tests the interaction between independently owned contracts.

A scenario belongs with an adapter when it tests translation/conformance.

Delete or move a scenario when its only purpose is to restate another owner's generic behavior.

Do not retain duplicate generic scenarios merely because they provide extra prose coverage.

A single executable test may provide evidence for multiple requirements. That does not mean multiple requirements should own the tested semantic rule.

# Step 8 — Only now inspect implementation and evidence

Once the current canonical model is understood, inspect:

* Rust types and behavior;
* tests;
* fixtures;
* traces;
* verification manifests;
* retained evidence;
* scripts;
* CLI behavior;
* deployment assumptions.

Ask:

1. Does implementation conform to the chosen canonical owner?
2. Does implementation contain an alternate hidden authority?
3. Are tests encoding a rule absent from current specs?
4. Does evidence claim more than the canonical rule permits?
5. Does implementation rely on vector order, type shape, path, enum value, process behavior, or another accidental mechanism as semantic authority?
6. Does a supposed implementation detail affect persistent interpretation or externally observable outcomes?

Do not change canonical semantics simply because implementation already behaves differently.

# Step 9 — Use the active roadmap for coherence; use historical sources for archaeology

After understanding the current canonical model and inspecting implementation/evidence, inspect the active architecture roadmap.

Use the active roadmap to ask:

* Does the current canonical system still form the intended architecture?
* Has an architectural invariant or dependency direction been accidentally lost?
* Is current divergence a deliberate refinement, an unresolved architectural change, or obsolete roadmap intent?
* Does the proposed ownership repair preserve the intended future dependency structure?
* Does the repair accidentally make a temporary implementation shape permanent architecture?

The active roadmap guides this coherence analysis and future direction. It does not silently override current canonical requirements.

Then inspect historical sources as needed:

* ADRs;
* archived changes;
* prior architecture revisions;
* milestone and handoff material;
* historical verification.

Use historical sources to ask:

* Why was the present boundary chosen?
* Was an important contract present previously and later lost?
* Is duplicate ownership residue from implementation sequencing or an earlier experiment?
* Did experimental or temporary behavior accidentally become permanent?
* What evidence or rationale supports a proposed semantics-preserving repair?

Historical material may explain intent and support a proposed canonical repair.

Neither historical material nor the active roadmap silently resolves a contradiction between current canonical requirements.

# Step 10 — Decide the minimum repair

For every finding choose one disposition.

## Preserve

No semantic ownership problem exists.

Make no normative edit.

## Reference owner

The local requirement needs the owner's policy but adds no semantic constraint of its own in that area.

Remove duplicated policy wording and use the canonical relationship.

## Refine owner

Retain only the local additional constraint.

The owner remains complete without the refiner.

The refiner must be understandable as:

> canonical owner **plus this additional local condition**

## Relocate ownership with semantics preserved

Move the detailed policy to the better canonical owner.

Rewrite consumers as references/refinements.

Preserve stable requirement IDs when their materially owned semantics remain the same.

## Split semantic responsibilities

A requirement currently owns two independent policies.

Separate them only when they have different natural owners or independent lifecycle/evidence.

Do not split for prose length.

## Merge requirements

Merge only when two requirements have no meaningful independent semantic responsibility after ownership analysis.

Do not merge merely because they interact frequently.

## Delete obsolete requirement

Delete a requirement that describes:

* completed experiment procedure;
* historical implementation sequence;
* obsolete compatibility;
* completed correction program;
* behavior no longer part of the product.

Preserve historical rationale outside current canonical semantics when useful.

## Add missing canonical behavior

Add a requirement only for consequential shadow architecture that should be durable product behavior.

Do not canonicalize convenience implementation choices.

## Fix implementation

When current semantics are correct and implementation violates them, change implementation rather than weakening the requirement.

# Step 11 — Classify every normative edit

Use exactly one classification:

* **clarification/no semantic change**
* **ownership relocation with semantics preserved**
* **semantic narrowing**
* **semantic broadening**
* **semantic correction/change**
* **requirement deletion because behavior is obsolete**

For every changed requirement ID answer:

1. Does the stable ID still describe materially the same obligation?
2. If yes, preserve it.
3. If no, retire it and create a new ID.
4. Which current references/evidence move?
5. What interpretation or compatibility consequence follows?

Do not preserve an ID merely to avoid migration.

Do not create a new ID merely because wording moved.

# Step 12 — Repair through OpenSpec

If the task authorizes repair, make normative repairs through a bounded OpenSpec change.

Follow the repository's OpenSpec identifier policy.

Do not use a roadmap `OS-###` identity for semantic drift repair unless the repair is itself the corresponding roadmap work.

The change proposal/design must contain a semantic-ownership reconciliation table for affected clusters:

```text
Current requirements:
Semantic rule:
Selected owner:
Other requirements:
  - requires
  - refines
  - composition only
  - delete
Contradiction resolved:
Semantic edit classification:
Stable-ID consequence:
Scenario migration:
Implementation/evidence migration:
Verification:
```

This table is change-local review material.

Do not create a permanent duplicate semantic registry.

# Step 13 — Re-review dependent requirements

When the semantic fingerprint of an owner changes:

1. enumerate every `requires` and `refines` dependent;
2. inspect each dependent against the new owner semantics;
3. explicitly record one of:

   * still valid;
   * wording update required;
   * local scenario update required;
   * implementation/evidence update required;
   * dependency no longer valid;
4. resolve reviewed-state fingerprints individually with concrete reasons.

Never bulk-accept all dependents because the build passes.

A changed owner is exactly when semantic drift is most likely to propagate.

# Step 14 — Perform the ownership challenge

Before declaring the repair complete, challenge every affected semantic rule with these questions.

## Single-owner test

Can you point to exactly one current requirement that completely defines the policy?

## Deletion test

If every refiner/composer/adapter requirement disappeared, would the underlying policy still be completely defined by its owner?

If not, important semantics remain scattered.

## Duplicate-predicate test

Does another requirement independently state the same precondition + authority decision + transition + postcondition?

If yes, consolidation is incomplete.

## Two-implementation test

Could two conforming implementations choose materially different behavior because requirements disagree or leave a gap?

If yes, reconciliation is incomplete.

## Failure-path test

Does the owner define conservative behavior for relevant failure and uncertainty states?

## Scenario-locality test

Does every non-owner scenario test a real local refinement, composition, or adapter mapping rather than generic owner behavior?

## Historical-independence test

Could a new implementation agent understand the current rule after deleting every archived change, ADR, milestone, handoff, and conversation?

## Implementation-authority test

Is any current behavior still authorized only because a Rust type, enum, vector order, file path, or existing test happens to encode it?

If yes, inspect for shadow architecture or nonconformance.

# False positives to reject

Do not “repair” these merely because wording overlaps.

## Constitution and refinement

A constitutional invariant plus detailed capability implementation is intentional when the constitution remains high-level.

## Same event, different states

Two capabilities may react to the same crash, flush, repair, or topology event while owning different state transitions.

## Shared evidence

One executable test or trace can support several independent requirements.

Shared evidence is not duplicate semantics.

## Adapter scenarios

An adapter may repeat an external stimulus such as flush or discard in order to verify exact translation.

It should not repeat the portable policy being translated.

## Composition ordering

A composer may state that A must complete before B without redefining either A or B.

## Explanatory prose

Non-normative explanation may restate a rule for readers.

Do not confuse explanatory repetition with a second canonical owner.

# Required review output

For a bounded review, report only consequential findings.

For a full audit, produce a temporary ledger:

| Finding | Requirements | Classification | Canonical owner | Drift | Semantic effect | Exact repair | Evidence |
| ------- | ------------ | -------------- | --------------- | ----- | --------------- | ------------ | -------- |

Use stable finding IDs when the review will feed implementation work.

Also record confirmed non-problems when doing so prevents repeated future re-litigation.

Do not persist this ledger as a parallel requirements database.

# Verification after repair

Run the deterministic gates appropriate to the changed repository.

At minimum:

```bash
cargo xtask docs knowledge readiness
cargo xtask docs check
```

Also run:

```bash
cargo xtask docs build
cargo xtask docs clean-room
```

when current semantic links, maintained documentation, or generated relationship views changed.

Run OpenSpec validation and the focused product tests/proofs/evidence for every changed semantic contract.

When relationship tooling exists, verify:

* no unknown relation target;
* no self-edge;
* no duplicate edge;
* no hard dependency cycle;
* no relation to a retired/non-current requirement;
* every changed owner has all dependents explicitly reviewed.

A green deterministic gate does **not** prove semantic coherence.

Finish by repeating the ownership challenge in Step 14 against the final source state.

# Completion standard

Semantic reconciliation is complete only when:

* every affected semantic fact has one identifiable canonical owner;
* refiners state only their local additional constraints;
* composers orchestrate rather than redefine;
* adapters map rather than invent portable semantics;
* generic duplicate scenarios have been removed or moved;
* confirmed contradictions have been resolved explicitly;
* missing consequential behavior has a canonical owner or an explicit blocking decision;
* implementation/evidence does not silently substitute for canonical authority;
* historical artifacts are not required to understand current behavior;
* changed owner fingerprints have caused explicit dependent review;
* stable requirement IDs accurately reflect semantic identity;
* deterministic gates pass;
* remaining semantic uncertainty is stated explicitly.

# Forbidden shortcuts

* Do not merge capability files merely because their requirements interact.
* Do not preserve duplicate policy wording “for clarity.”
* Do not add a cross-reference while leaving both requirements as independent owners.
* Do not choose implementation as truth when canonical requirements disagree.
* Do not use architecture history as a silent override.
* Do not create policy IDs, ownership registries, manual backlinks, semantic databases, or generalized graph infrastructure.
* Do not bulk-resolve reviewed requirement fingerprints.
* Do not use passing tests as proof that duplicate canonical semantics are coherent.
* Do not canonicalize ordinary implementation choices.
* Do not treat every wording difference as semantic drift.
* Do not perform unrelated cleanup while repairing an ownership cluster.

---

The desired end state is not fewer specifications.

The desired end state is:

> **many well-bounded capabilities, one owner for each semantic decision, explicit composition between them, and no hidden second source of authority.**
