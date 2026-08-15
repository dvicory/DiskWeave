## Context

The canonical specs already contain capability-specific output obligations. This change adds one architecture-contract owner for the shared projection rule and moves duplicated generic projection wording under that owner. The canonical specs are restored to the parent revision; this directory contains the proposed canonical delta and its ownership record.

## Goals / Non-Goals

**Goals:**

- Give one canonical requirement ownership of consequential meaning across external representations.
- Require structured output to retain the complete bounded result required by the applicable capability contract.
- Allow human output to select, summarize, aggregate, reorder, or omit only detail that is both non-consequential and not otherwise required by that capability contract.
- Retain every existing capability-specific obligation while making ownership and dependency edges explicit.

**Non-Goals:**

- No change to existing operator-recovery, recovery-inspection, or demo-CLI behavior.
- No recovery, repair, migration, publication, compatibility, or persistent-format decision.
- No new renderer abstraction, output schema, serialization format, or implementation API.

## Decisions

1. **Architecture contract owns the shared rule.** The cross-cutting requirement defines consequential meaning and prohibits stronger or contradictory projections. Repeating generic wording in capability specs would create competing owners; moving capability facts into the architecture contract would create a second capability specification.

2. **Use semantic preservation, not field equality.** Structured output SHALL retain the complete bounded result required by its capability contract. Subject to that contract, human output MAY select, summarize, aggregate, reorder, or omit detail that is both non-consequential and not otherwise required. The shared rule does not require identical fields.

3. **Preserve existing local obligations.** The consumer deltas retain C0a role-local coverage and exact bounded-exception requirements, C8a bounded-manifest requirements, and demo command identity and diagnostics. They remove only generic sameness or separate-truth-source wording that the architecture owner now supplies.

4. **Apply through OpenSpec artifacts.** The implementation workflow may sync the reviewed architecture and consumer deltas into canonical specs only after adversarial review, focused evidence planning, and knowledge gates pass. This proposal does not authorize direct canonical edits or product-code changes.

## Ownership Reconciliation

| Semantic rule | Canonical owner | Consumer refinement | Retirement and evidence |
| --- | --- | --- | --- |
| Consequential meaning across external representations | `req.architecture-contract.operator-result-projections-preserve-consequential-meaning` | Operator, independent inspection, and demo requirements depend on it | Retire only duplicated generic projection wording; stable requirement IDs remain unchanged; review renderer and evidence links |
| Role-local coverage, exact bounded exceptions, causal dimensions, and non-authorization | `req.operator-recovery.production-assessment-is-observational-and-multidimensional` | Defines the facts an operator assessment must retain in both representations | No semantic narrowing; existing scenarios and focused evidence remain authoritative |
| Command result identity, outcome, reason, and multidimensional assessment | `req.operator-recovery.production-commands-share-one-semantic-result-boundary` | Defines the local result envelope after parser entry | No semantic narrowing; the shared owner supplies representation equivalence |
| Bounded recovery manifest and inspection classification | `req.independent-recovery-inspection.independent-recovery-state-inspection-is-bounded-and-non-authorizing` | Retains the bounded manifest and all local non-authorization/status facts | No human-manifest relaxation; existing manifest evidence remains required |
| Demo command identity, outcome, and bounded diagnostics | `req.macos-demo-cli.the-demo-cli-has-a-versioned-observable-contract` | Retains the local command contract | Remove only generic separate-truth-source wording; command evidence remains required |

## Risks / Trade-offs

- **[Risk]** A consumer may interpret non-consequential detail too broadly. **Mitigation:** the applicable capability contract takes precedence; preserve every current action-relevant exception, refusal, uncertainty boundary, and next action.
- **[Risk]** Human and structured renderers may drift while remaining syntactically valid. **Mitigation:** future focused evidence compares consequential fields and bounded findings; it does not use byte-for-byte equality as the generic rule.
- **[Risk]** A future request to summarize the recovery manifest may hide an existing compatibility or operator contract. **Mitigation:** keep the current bounded-manifest obligation unchanged and require a separate semantic decision before relaxing it.

## Migration Plan

1. Have the adversarial reviewer compare every consumer delta with the restored canonical specs and record any lost meaning.
2. Apply the architecture and semantic-preserving consumer deltas through the OpenSpec workflow, then refresh documentation knowledge and inspect affected dependents.
3. Add focused semantic-preservation evidence only after the contract review; do not change recovery state, payload, or persistent formats.
4. Run the prescribed documentation checks and focused product checks. Archive only after implementation evidence proves the contract.
5. Before apply, abandon this unarchived change to roll back. After apply, use a new reviewed OpenSpec change; do not overwrite canonical specs directly.
