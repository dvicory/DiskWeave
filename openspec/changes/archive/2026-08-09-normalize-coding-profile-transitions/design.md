## Context

The canonical topology-transition requirement covers staged member and role changes but does not retain every bounded-plan field or operation-specific condition from v0.8 §§6.6 and 7.8, and it does not name transitions that alter the mathematical interpretation of parity. The v0.8 audit assigns those statements to capability specs only at section granularity.

## Goals / Non-Goals

**Goals:**

- Preserve the old profile's complete recovery authority until a verified target profile is durably committed.
- Preserve exact plan bindings and the safety preconditions for add, remove, resize, and coding-profile transitions.
- Make unsupported profile transitions observably non-mutating.
- Give the v0.8 clauses an exact canonical owner and evidence disposition.

**Non-Goals:**

- Implement a topology or coding-profile migration.
- Define P/Q mathematics, storage layout, online reconciliation, or quiescence policy for a specific migration.
- Claim that any coding-profile transition is currently supported.

## Decisions

- Extend the existing staged topology-transition requirement rather than introduce a second transition state machine. Profile changes are topology transitions with an additional byte-interpretation hazard.
- Bind source and target generations, geometry, and profiles explicitly. A generation alone cannot prove which parity equation or byte range was verified.
- Require target parity establishment and independent verification before commit while leaving storage layout unspecified. This preserves the safety invariant without inventing a migration implementation.
- Refuse unsupported transitions before mutation. This lets current implementations remain unsupported without weakening the future contract.

## Risks / Trade-offs

- The stronger requirement changes the semantic fingerprint and makes linked relationships suspect until reviewed.
- The contract intentionally leaves online versus quiesced execution to a future capability change; it prevents unsafe implementation but does not make migration executable.
