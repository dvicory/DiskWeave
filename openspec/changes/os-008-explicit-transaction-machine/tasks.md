## 1. Artifact and module contract

- [x] 1.1 Cite archived OS-003–005 prerequisites, handoff Sections 9.8–9.10 and 22.2, and the transaction decision IDs.
- [x] 1.2 Define the handoff-required 20-section design with semantic actions/results separated from backend child I/O.
- [x] 1.3 Define the explicit transaction capability delta, forbidden orderings, and OS-009/OS-010/OS-013 successors.

## 2. Reference machine implementation

- [x] 2.1 Add `dwv-transaction-ref` and split the implementation into action, machine, trace, and error modules.
- [x] 2.2 Implement bounded transaction plans, stage transitions, semantic actions/results, and topology/recovery generation capture.
- [x] 2.3 Enforce durable dirty/integrity intent before home mutation and fence evidence before checkpoint/clear/release.
- [x] 2.4 Implement failure, uncertainty, abandonment, daemon-crash, duplicate-result, and reconciliation-required handling.

## 3. Trace and verification

- [x] 3.1 Implement versioned normalized action traces and deterministic replay without private backend/runtime types.
- [x] 3.2 Add transition, ordering, mutation, crash, abandonment, and simulator-composition tests.
- [x] 3.3 Run focused/workspace tests, formatting, dependency inspection, and OpenSpec validation.
- [x] 3.4 Record OS-008 complete only after every forbidden transition is rejected and every legal path has evidence; leave OS-009 selection to its own change.
