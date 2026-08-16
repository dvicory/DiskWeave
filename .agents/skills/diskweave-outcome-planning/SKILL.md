---
name: diskweave-outcome-planning
description: Plan substantial DiskWeave work for autonomous execution using Beads rather than new milestone documents. Preserve outcomes, decisions, dependencies, acceptance evidence, and important boundaries while leaving product semantics to OpenSpec and reversible implementation choices to the executing agent.
---

# DiskWeave outcome planning

Use this skill to plan substantial work, including work the user informally calls a milestone.

A milestone is a planning scale, not a repository document type.

Do not create new `docs/milestones/m*.md` files. Existing milestone files are history, not templates.

Use **Beads** for shared work and dependencies. OpenSpec owns product semantics.

If the environment provides guidance for agent-managed local work, load and follow it for scratch work, experiments, handoffs, or continuity that does not belong in Beads. This skill does not prescribe a local-work layout.

## Plan the outcome

A good plan should make clear:

- what useful result DiskWeave should gain;
- what decisions are already settled;
- what is deliberately out of scope;
- what work can proceed independently;
- what actually blocks other work;
- what each piece of work should prove or make possible;
- what evidence closes the work;
- what important failure, recovery, preservation, or claim boundaries matter;
- what reversible engineering choices remain open.

Use only as much structure as the work needs.

## Keep authority clear

Follow the repository's semantic authority rules.

Beads track work, not product behavior. If planning exposes a semantic gap, contradiction, or behavior change, use semantic reconciliation and the normal OpenSpec workflow. Do not put the missing product rule only in a Bead.

When repository guidance requires human judgment, represent the blocked decision with a Beads human gate. Do not create human gates for ordinary reversible choices or merely because current semantics need investigation.

## Investigation may come first

A Bead may begin before an OpenSpec change exists when the work is to determine whether product semantics need to change.

The Bead owns the investigation, not provisional product semantics.

If a semantic change is needed, create or amend the appropriate OpenSpec change. The Bead may continue tracking the work, but behavior-changing implementation must follow the target contract defined through the normal semantic workflow.

## Use the minimum useful Bead structure

Start with the work that needs durable shared tracking. This may be an outcome, an investigation, or another independently useful unit of work.

Create separate Beads when work benefits from its own status, ownership, acceptance criteria, or dependencies. Do not build a complete task tree before execution needs one.

Use a parent when several Beads contribute to one larger outcome and that outcome has integrated decisions or acceptance criteria worth tracking. The parent describes the overall result; children describe the work they independently own.

Use relationships according to their meaning:

- **parent/child** — work contributes to a larger tracked outcome;
- **blocks** — one piece cannot safely or usefully proceed before another;
- **related** — useful association without execution ordering.

Do not add relationships merely to make the graph look complete.

A Bead should usually say:

- **title** — the result or problem;
- **description** — what needs to change or be determined, with important scope;
- **design** — settled decisions or constraints future agents should preserve;
- **acceptance criteria** — how to tell when the work is done.

Include only the fields and detail the work actually needs.

Reference canonical semantics when an exact product rule matters. Do not copy requirement prose into Beads.

Do not mechanically mirror OpenSpec tasks into Beads. Create Beads when the work itself benefits from durable shared tracking.

## Keep Beads and local work separate

Beads contain shared work that other agents need to find: outcomes, executable work, status, blockers, dependencies, and completion criteria.

Agent-managed local work may hold scratch notes, experiments, temporary evidence, hypotheses, or handoff context.

Do not leave shared obligations only in local agent state. Do not turn local agent state into another issue tracker or semantic registry.

## Leave reversible choices open

Include a planning detail when omitting it would likely cause an agent to:

- build the wrong thing;
- cross a correctness or authority boundary;
- miss a real dependency;
- make a claim stronger than the evidence;
- lose important failure or recovery behavior;
- reopen a decision that is already settled;
- or misunderstand what completion means.

Otherwise, let the executing agent decide.

Do not normally prescribe:

- the number or names of OpenSpec changes;
- speculative `req.*` IDs;
- exact OpenSpec task structure;
- a `design.md` outline;
- a Bead hierarchy beyond what current dependencies justify;
- Rust files, modules, types, or crates;
- reversible implementation choices.

## Completion

A good outcome plan lets a fresh agent answer:

> What result are we trying to achieve, what work remains, what blocks what, which decisions are already settled, what evidence closes the work, and where are the product semantics?

It should not require a new milestone document or conversation history to recover the plan.
