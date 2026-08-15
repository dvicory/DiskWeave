## ADDED Requirements

### Requirement: Operator-result projections preserve consequential meaning
<!-- dwv:req req.architecture-contract.operator-result-projections-preserve-consequential-meaning -->
<!-- dwv:requires req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred -->

When a capability exposes the same evaluated semantic result through more than one external representation, each representation SHALL preserve the consequential meaning established by that result and the applicable capability contracts.

A meaning is consequential when changing or omitting it would materially change safe interpretation or action. This includes, as applicable, the established outcome or disposition, causal reason or blocker, authority or uncertainty limits, refusal or operational consequence, and next action.

Representations MAY use audience-specific vocabulary or structure. Structured output SHALL retain the complete bounded result required by the applicable capability contract. Subject to that contract, human output MAY be selective and MAY summarize, aggregate, reorder, or omit detail that is both non-consequential and not otherwise required by the contract. The representations need not contain the same fields. No representation SHALL state or imply a consequential meaning that changes the result's safe interpretation or action, including by strengthening a claim, weakening a refusal or uncertainty boundary, or otherwise contradicting the result.

#### Scenario: Human output selects detail
- **WHEN** one evaluated semantic result is rendered as complete, bounded structured output and as human output
- **THEN** subject to the applicable capability contract, human output MAY summarize, aggregate, reorder, or omit detail that is both non-consequential and not otherwise required by the contract, but SHALL preserve every consequential distinction established by the result and applicable capability contract

#### Scenario: Observation and operation outcomes remain distinct
- **WHEN** a result contains an observed subject classification or authorization boundary that differs from the operation outcome, or an operation is refused, blocked, unsupported, reconciliation-required, or failed
- **THEN** each representation SHALL preserve the established operation outcome separately from subject state and SHALL NOT state or imply stronger health, authority, completion, or actionable meaning
