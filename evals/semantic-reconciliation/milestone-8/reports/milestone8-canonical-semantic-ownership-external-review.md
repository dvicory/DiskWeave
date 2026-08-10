# DiskWeave External Specification, Implementation, and Milestone Review

Review the supplied DiskWeave repository ZIP as a self-contained architecture and implementation snapshot. Also review the supplied code-review artifact that was produced while OS-031 was being implemented.

Produce an execution-ready cleanup program beginning with milestone 8.

This is an analysis and planning task. Inspect the archive directly. Treat its extracted root as the repository root. Cite exact paths, requirement IDs, symbols, tests, evidence artifacts, and review findings found in the supplied material.

Do not modify or return a rewritten ZIP. Your output is:

1. an archive-grounded review of the current system;
2. a reconciliation of the supplied code review against the current snapshot;
3. a single recommended cleanup sequence;
4. complete milestone documents that another implementation agent can place under `docs/milestones/` and execute.

You may use archive-local inspection tools, searches, scripts, or commands if your environment permits them. Clearly distinguish:

* checks you actually performed and observed;
* commands you recommend that a later implementation agent run.

Never claim a command was executed unless you actually have its output.

# Authority model

Use the following authority hierarchy for this review.

## Canonical product semantics

Current files under:

`openspec/specs/*/spec.md`

are the canonical source of current required product behavior.

If canonical specs disagree with one another, do not silently choose whichever agrees with implementation or historical architecture. Report the contradiction and recommend a canonical resolution.

## Current implementation evidence

Rust code, tests, fixtures, manifests, verification artifacts, generated reports, CLI behavior, scripts, and similar repository contents are evidence of what the current system does and what assumptions implementation currently contains.

They do not silently override canonical requirements.

When implementation contains important behavior absent from canonical requirements, treat it as possible shadow architecture and evaluate whether it should become canonical.

## Architecture and rationale

Architecture v0.8 is an important coherence and design-intent lens.

Use it to understand the architecture, detect unexplained divergence, and recover potentially lost contracts, but do not treat it as current normative authority over `openspec/specs/*/spec.md`.

ADRs, handoffs, archived changes, prior architecture versions, and similar material are also non-canonical unless a canonical requirement explicitly incorporates their semantics.

A divergence from v0.8 is not automatically a defect. Determine whether it is:

* a deliberate and sound refinement;
* an accidental loss of architecture;
* an unresolved architectural change;
* obsolete historical intent.

## Active and archived OpenSpec changes

Treat active changes as proposed or in-flight semantics unless the repository itself establishes that their requirements have already become current canonical requirements.

Treat archived changes as historical evidence, not current authority.

## Supplied code review

Treat the supplied code review as a set of hypotheses and observations from an earlier point during OS-031 implementation.

It is not authority.

Every consequential code-review finding must be reconciled against the current repository snapshot before being accepted into future work.

# Current situation

* Canonical behavior lives in `openspec/specs/*/spec.md`.
* Architecture handoffs, ADRs, archived changes, tests, implementation, and evidence may reveal intent or shadow architecture, but they do not silently override current canonical requirements.
* The project is retiring the entire `goal-vN` planning model in favor of milestones under `docs/milestones/` named `m1.md`, `m2.md`, and so on.
* This migration is retroactive.
* Every project-planning use of `goal-vN`, `Goal vN`, goal-document paths, and equivalent goal identifiers must be renamed, moved, rewritten, or deleted, including historical material.
* Do not leave compatibility aliases, redirect files, stale links, archived goal-named paths, or lookup tables preserving the retired naming scheme.
* Preserve historical meaning when a historical planning artifact remains useful. Where it has no continuing value, deletion is preferable to a meaningless rename.
* The next new executable cleanup artifact is **milestone 8**.
* Produce `docs/milestones/m8.md` as the first new milestone.
* You may split later work into `m9.md` and subsequent milestones only where a real dependency, safety boundary, review boundary, or completion gate justifies the split.
* Do not renumber new work back to m1.
* Do not create separate milestones merely to make categories or document sizes symmetrical.
* Milestones organize work and evidence. They must never become a second source of product semantics.

Review all current canonical specs and all relevant material in the ZIP. Do not limit the review to OS-031, architecture v0.8, or the supplied code review.

# Snapshot and OS-031 rule

Determine OS-031's actual status from the supplied repository rather than assuming it is complete or active.

State whether the snapshot indicates OS-031 is:

* active/incomplete;
* implementation-complete but awaiting verification or archival;
* fully completed/archived;
* ambiguous from the supplied material.

If OS-031 is still active:

* do not casually move its canonical contract underneath an implementation agent;
* identify work that safely belongs inside OS-031;
* identify work that must wait until OS-031 completes;
* allow immediate correction only where leaving the contract unchanged creates a genuine correctness hazard, and explain the coordination required.

If OS-031 is already complete:

* do not preserve artificial “wait for OS-031” sequencing;
* reconcile its final behavior normally into m8 and later work.

# Finding discipline

Assign every consequential review finding a stable identifier:

`F-001`, `F-002`, ...

The specification issue ledger is the canonical record for those findings.

Other sections should refer to finding IDs rather than independently restating incompatible versions of the same finding.

For each proposed normative spec edit, explicitly classify the edit as one of:

* **clarification/no semantic change**;
* **ownership relocation with semantics preserved**;
* **semantic narrowing**;
* **semantic broadening**;
* **semantic correction/change**;
* **requirement deletion because behavior is obsolete**.

Preserve an existing stable requirement ID when its semantics remain materially the same.

If semantics materially change, state:

* whether the existing ID remains valid;
* whether a new ID is required;
* what references and evidence must migrate;
* what compatibility or interpretation consequence follows.

Do not inflate severity. Similar wording, intentional refinement, or overlapping evidence is not automatically a defect.

# Questions

## 1. Is the current canonical specification set coherent?

Review every current canonical capability and requirement as one system.

Determine:

* whether capability boundaries are sensible;
* whether each consequential invariant and state transition has one clear canonical owner;
* whether requirements compose into an internally consistent architecture;
* whether two independently implemented, nominally conforming versions could behave incompatibly because of ambiguity;
* whether any requirement is too broad, too narrow, obsolete, implementation-shaped, or misplaced;
* whether any current spec describes a completed experiment or historical work rather than durable product behavior;
* whether important terms change meaning between specs;
* whether requirements agree on failure, crash, uncertainty, reconciliation, and resource release—not merely on the successful path.

Distinguish:

* intentional refinement;
* harmless repetition;
* drift-prone duplication;
* ambiguous ownership;
* confirmed contradiction;
* missing behavior.

For every consequential finding, cite exact files and requirement IDs and give an executable disposition.

## 2. What dependency structure actually exists among the specs?

Reconstruct the real semantic dependency graph, including:

* foundational invariants;
* semantic data-model dependencies;
* protocol and ordering dependencies;
* state-machine dependencies;
* adapter and conformance dependencies;
* composition dependencies;
* recovery and authority dependencies;
* verification and evidence dependencies;
* implementation-order gates.

Identify dependencies that are:

* explicitly encoded;
* expressed only through prose;
* expressed through OS, VE, goal, gate, phase, architecture-version, or handoff references;
* visible only in Rust code or tests;
* visible only in active or archived changes;
* entirely unstated.

Recommend the smallest useful dependency representation.

Specify:

* which relationship types are actually needed;
* whether each relationship belongs at capability or requirement level;
* where forward relationships should be written;
* how backlinks and reading order should be derived;
* which relationships should deliberately remain implicit.

Do not propose a second requirements registry, generalized graph framework, parallel architecture database, or manually maintained backlink system.

## 3. Where do specs step on each other's ownership?

Find every consequential case where multiple specs govern the same:

* invariant;
* state transition;
* failure path;
* authority decision;
* durability claim;
* lifecycle rule;
* ownership or resource-release rule;
* repair or recovery decision;
* topology transition;
* evidence boundary.

For each case:

1. Name the competing requirements.
2. Explain whether their wording is actually compatible.
3. Select one canonical owner.
4. State what every other requirement should do: remain a refinement, reference the owner, narrow itself to local conformance, move, merge, split, or be deleted.
5. Supply replacement wording or an exact editing instruction.
6. Classify the normative edit using the semantic-change categories above.
7. State what executable evidence must continue to cover after the change.

## 4. Which requirements depend on non-canonical knowledge?

Find requirements that cannot be implemented correctly from current canonical sources alone because they depend on:

* OS or VE numbers;
* goal documents or goal numbering;
* handoff or architecture section numbers;
* named gates or phases;
* archived proposals or ADR details;
* undefined vocabulary;
* implicit knowledge of the current Rust implementation;
* unstated assumptions about profiles, generations, identities, ownership, or evidence tiers.

For each finding:

* cite the exact normative wording;
* identify the missing canonical fact;
* identify the non-canonical source if present;
* choose the canonical owner;
* provide replacement wording or an exact remediation;
* classify whether semantics change;
* state whether the old reference should be removed or retained only as explicitly non-normative rationale after milestone renaming.

A canonical requirement must remain understandable after historical artifacts and conversation context are removed.

## 5. What shadow architecture exists outside the canonical specs?

Identify consequential architecture currently expressed only in:

* Rust types or behavior;
* tests and fixtures;
* verification manifests or evidence reports;
* ADRs;
* active or archived OpenSpec changes;
* architecture handoffs;
* operational scripts;
* CLI behavior;
* deployment assumptions;
* documentation tooling;
* the supplied code review;
* other review or handoff material included in the ZIP.

Include only behavior affecting:

* correctness;
* data integrity;
* recovery;
* durability;
* authority;
* lifecycle or ownership;
* portability;
* compatibility;
* security;
* dependency direction;
* persistent interpretation;
* externally observable outcomes.

Do not promote ordinary implementation choices into architecture.

For each shadow rule:

1. Cite the exact source.
2. State the behavior or invariant.
3. Explain why it is architectural rather than a local implementation choice.
4. Identify the canonical capability and requirement that should own it.
5. Choose one disposition: add a requirement, modify an existing requirement, mark it explicitly non-normative, preserve it as implementation choice, or delete it as accidental architecture.
6. Classify whether the proposed normative edit changes semantics.
7. Provide the exact spec change and required evidence.

Prioritize omissions that could cause data loss, incompatible implementations, unsafe authority, exaggerated evidence, or incorrect work by a future agent.

## 6. Which specs or artifacts should be merged, split, retired, or deleted?

Do not assume the current spec decomposition is correct or that consolidation is automatically better.

Look for:

* one-time experiments retained as permanent product requirements;
* two capabilities that differ only by historical implementation sequence;
* cross-cutting specifications that have absorbed capability-specific behavior;
* capability specs that restate rather than refine another owner;
* obsolete evidence comparisons after a decision was made;
* requirements whose remaining value is only historical rationale;
* files that exist solely because the goal-based workflow accumulated them.

For every proposed structural change, give:

* source and destination requirements;
* stable-ID consequences;
* evidence and implementation references that must migrate;
* deletion criteria;
* why the new boundary is easier to implement and verify.

Prefer no structural change where the existing decomposition is already coherent.

## 7. How should the specification system become easier for implementation agents?

Evaluate the workflow for an implementation agent receiving a bounded task.

Determine how that agent discovers:

* relevant requirement IDs;
* canonical owners and prerequisites;
* required reading order;
* current authority versus historical rationale;
* associated code, tests, evidence, and active changes;
* that another capability owns a policy;
* a contradiction or missing contract before inventing behavior.

Recommend only small changes that materially improve execution.

Consider:

* typed forward relationships;
* generated capability/dependency views;
* requirement search;
* bounded context by capability;
* lightweight linting;
* clearer ownership and non-goal sections.

Reject speculative knowledge systems, embeddings, duplicate registries, semantic databases, generalized graph infrastructure, and checks that pretend to automate semantic judgment.

## 8. Which automated checks would catch real defects?

Recommend a minimal deterministic check set.

For each check, state:

* its input;
* exact mechanical failure condition;
* expected diagnostic;
* blocking or advisory status;
* where it belongs in existing tooling;
* one current defect it would catch;
* what it deliberately does not prove.

Consider:

* unknown current requirement references;
* active-change requirement leakage into current manifests;
* normative OS, VE, goal, gate, phase, or handoff references;
* invalid or cyclic hard dependencies;
* orphaned or superseded requirements;
* duplicated semantic ownership declarations where ownership is mechanically declared;
* requirements dependent on removed historical sources;
* stale paths after goal-to-milestone migration.

Do not propose an automated semantic-consistency verdict without a precise mechanical invariant.

## 9. How should goal nomenclature be evicted retroactively?

Inventory every project-planning use of:

* `goal-vN` and filename variants;
* `Goal vN` and textual variants;
* goal documents and directories;
* links to goal paths;
* goal-specific tooling, prompts, tests, schemas, instructions, comments, and generated inputs;
* historical or archived goal references;
* assumptions that goal numbering defines dependency order.

Design a clean retroactive migration to milestone terminology.

Requirements:

* All retained project-planning artifacts use milestone names and paths.
* Historical artifacts are renamed and internally rewritten when they remain useful.
* Useless historical planning artifacts are deleted rather than preserved ceremonially.
* No `goal-vN` files, references, aliases, redirects, or compatibility lookup tables remain.
* Broken inbound links, indexes, prompts, tests, fixtures, and scripts are updated.
* Do not mechanically rewrite unrelated ordinary English uses of “goal” unless they refer to the retired project-planning concept.
* Preserve stable requirement IDs.
* Milestones may reference requirements but never own normative behavior.
* If previous goal numbers map cleanly to milestone numbers, use that mapping.
* If they do not, define an exact collision-free mapping and explain every exception.
* The next new work begins at `docs/milestones/m8.md` regardless of how earlier historical artifacts are mapped.

Identify every file move, deletion, content rewrite, reference update, tooling change, and validation required for the supplied ZIP.

This migration manifest must be exhaustive with respect to the supplied snapshot.

## 10. What cleanup milestones should an implementation agent execute next?

Turn the complete review into one or more bounded milestones beginning with m8.

Milestone 8 must establish the first coherent cleanup slice after the current OS-031 state.

Prioritize:

* confirmed correctness contradictions;
* canonical ownership gaps that would misdirect subsequent implementation;
* high-risk shadow architecture;
* removal of non-canonical normative dependencies;
* the retroactive goal-to-milestone cutover;
* enough relationship/navigation structure for the next agent to work safely.

Produce m9 or later milestones only where required to keep work executable, reviewable, or safely sequenced.

Do not force every finding into m8.

Choose boundaries based on semantic dependencies and completion gates, not equal document size or finding category.

Explain:

* why m8 is next;
* what it deliberately excludes;
* what later milestones it unlocks;
* where unresolved implementation findings from the supplied code review belong;
* how the sequence interacts with the observed OS-031 state;
* which work, if any, must wait for OS-031 completion.

# Supplied code-review reconciliation

Reconcile the supplied mid-OS-031 code review against the current archive.

For every consequential review finding, assign a `CR-###` identifier and determine:

* the original finding;
* the code/spec/evidence location it referred to;
* whether the current snapshot still exhibits it;
* whether it has been fixed;
* whether subsequent OS-031 work superseded it;
* whether the original finding was incorrect or no longer relevant;
* whether the remaining problem is an implementation defect, canonical-spec defect, evidence/test defect, or some combination;
* the associated canonical requirement IDs;
* whether fixing it changes product semantics;
* exact execution disposition.

Allowed reconciliation states are:

* **still present**;
* **partially fixed**;
* **fixed in current snapshot**;
* **superseded by later design**;
* **not reproducible from supplied snapshot**;
* **review finding rejected**.

Do not carry a code-review item into m8 or later merely because the earlier reviewer reported it.

Do not discard a finding merely because current tests pass if the underlying contract remains violated or unspecified.

# Finding classification

Classify every `F-###` finding as exactly one of:

* **Correctness blocker**
* **Canonical ownership gap**
* **Shadow architecture**
* **Dependency/navigation gap**
* **Historical-authority leakage**
* **Milestone-migration work**
* **Agent-usability improvement**
* **Optional cleanup**
* **No change recommended**

Also assign:

* priority: `P0`, `P1`, `P2`, or `P3`;
* confidence: `confirmed`, `strong inference`, or `needs owner decision`;
* execution disposition: `OS-031`, `m8`, `m9+`, `post-OS-031 code review`, or `no action`.

Do not inflate severity.

A stylistic overlap or alternative wording is not a correctness defect unless it can lead to meaningfully incompatible behavior.

# Required deliverables

## A. Archive-grounded current-state assessment

Provide:

* the archive contents inspected;
* expected inputs absent from the ZIP;
* observed OS-031 state;
* counts and structure of canonical capabilities, requirements, and scenarios;
* organization, consistency, dependency, and agent-usability assessments;
* local inspection/searches you actually performed, if any;
* additional commands an implementation agent should later run;
* a clear distinction between observed results and recommended future commands.

Also state whether the supplied material was sufficient to perform the requested review and identify any conclusions weakened by missing evidence.

## B. Dependency map

Provide:

* a layered capability map;
* a requirement-level table for consequential dependencies;
* explicit versus implicit classification;
* cycles;
* missing edges;
* obsolete historical edges;
* the minimal recommended relationship model.

Do not attempt to enumerate meaningless dependencies simply for completeness.

## C. Specification issue ledger

| ID | Priority | Classification | Confidence | Canonical requirements | Problem | Archive evidence | Canonical owner | Semantic effect | Exact disposition | Required verification | Execution |
| -- | -------- | -------------- | ---------- | ---------------------- | ------- | ---------------- | --------------- | --------------- | ----------------- | --------------------- | --------- |

Include confirmed non-problems where an apparent overlap is intentionally valid and recording that conclusion will prevent future agents from repeatedly reopening it.

Treat this ledger as the canonical record for `F-###` findings.

## D. Shadow architecture ledger

| Finding | Priority | Shadow rule | Current source | Why architectural | Canonical owner | Semantic effect | Exact spec change | Evidence required | Execution |
| ------- | -------- | ----------- | -------------- | ----------------- | --------------- | --------------- | ----------------- | ----------------- | --------- |

## E. Code-review reconciliation ledger

| Review ID | Original finding | Current evidence | Reconciliation state | Canonical requirements | Remaining problem | Exact disposition | Execution |
| --------- | ---------------- | ---------------- | -------------------- | ---------------------- | ----------------- | ----------------- | --------- |

Every consequential finding from the supplied review must appear here once.

## F. Retroactive goal-to-milestone migration manifest

| Existing path/reference | Current purpose | Rename, rewrite, or delete | New path or canonical owner | Required inbound updates | Verification |
| ----------------------- | --------------- | -------------------------- | --------------------------- | ------------------------ | ------------ |

This manifest must be exhaustive for the supplied ZIP and leave no retained project-planning `goal-vN` nomenclature.

## G. Complete milestone documents

Produce the complete proposed contents of:

`docs/milestones/m8.md`

If the dependency structure genuinely requires further milestones, also provide complete contents for:

`docs/milestones/m9.md`
`docs/milestones/m10.md`
...

Do not create additional milestones unless they have a distinct completion gate or dependency reason.

Each milestone must be directly executable by another implementation agent and contain:

1. Directive
2. Starting state and prerequisites
3. Authority and source-inspection order
4. Exact scope
5. Canonical requirements affected
6. Ordered work packages
7. Exact files or file families to inspect and change
8. Required additions, rewrites, moves, and deletions
9. Goal-to-milestone migration work assigned to that milestone
10. Acceptance criteria
11. Focused validation commands
12. Evidence that must be retained
13. Failure and ambiguity handling
14. Explicit non-goals
15. Forbidden shortcuts
16. Completion gate
17. Final handoff requirements

Every work item must name:

* input;
* exact intended change;
* dependency on earlier work;
* observable acceptance result;
* proof or validation method.

Reference `F-###` and `CR-###` IDs rather than restating parallel versions of findings.

A milestone may direct an agent to change canonical requirements, but the milestone itself must not redefine those requirements or become their semantic owner.

Do not use unresolved phrases such as:

* “consider”;
* “possibly”;
* “as appropriate”;
* “decide later”;
* “clean up related items”;
* “update tests as needed.”

If a genuine product-owner decision is unavoidable, isolate it as a blocking decision and provide:

* exact alternatives;
* semantic consequences of each;
* evidence favoring each alternative;
* the safest default;
* all work that can proceed independently.

Do not manufacture owner decisions where repository evidence supports a clear answer.

## H. Execution order

Finish with one ordered execution sequence an implementation lead can follow:

1. remaining OS-031 work, if any, or exact safe prerequisite work;
2. m8;
3. any m9+ milestones produced;
4. unresolved implementation work from the reconciled code review at the correct point in the sequence;
5. final specification, evidence, dependency, documentation, and agent-readiness verification.

# Constraints

* Identify missing inputs explicitly.
* Do not modify or return repository files.
* Treat the repository ZIP as the current snapshot and older review material as potentially stale.
* Do not treat historical architecture as current authority.
* Do inspect historical material for shadow architecture, lost contracts, rationale, and cleanup needs.
* Use architecture v0.8 as an architectural coherence lens, not a normative override.
* Do not leave historical project-goal nomenclature merely because it is historical.
* Do not assume the existing spec split is correct.
* Do not assume consolidation is automatically better.
* Do not create a second semantic registry.
* Do not make milestones normative.
* Do not preserve retired goal paths through aliases, redirects, compatibility files, or lookup tables.
* Do not turn ordinary implementation details into requirements.
* Do not hide contradictions behind generic cross-references.
* Prefer deletion, canonical ownership, and generated views over new frameworks.
* Preserve stable requirement IDs when semantics remain materially the same.
* If semantics materially change, state the identity and migration consequence explicitly.
* Do not repeatedly restate the same finding across the report; use `F-###` and `CR-###` references.
* Distinguish repository facts from inference.
* Distinguish semantic defects from documentation/navigation defects.
* Distinguish implementation nonconformance from missing or incorrect canonical semantics.
* Prefer the smallest cleanup that leaves future implementation agents with an unambiguous canonical system.
* Finish with a single recommended course of action and complete executable milestone artifacts, not competing plans.
