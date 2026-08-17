## 1. Quint canary model

- [x] 1.1 Add the parameterized model-only `RecoveryProtocol` Quint module and concrete analysis module with explicit intent, home, recovery, ownership, fence, checkpoint, crash/loss, abandonment, and reconciliation states.
- [x] 1.2 Preserve the seven VE-002 invariant names, add witnesses for each major transition, and bind the two-region/two-store finite analysis instance.
- [x] 1.3 Run typecheck and sampled Quint simulations at the declared bound, recording deterministic rerun and witness observations.

## 2. Mutation and evidence replacement

- [x] 2.1 Run the disposable durable-intent mutation against a temporary Quint copy and retain the failing invariant output as verification evidence without retaining the mutant.
- [x] 2.2 Remove the active TLA+ model and checker configuration, and update the VE-002 ADR to name Quint as the sole current model authority.
- [x] 2.3 Replace the VE-002 section in the portable verification evidence log and add a bounded Quint evidence entry to the verification manifest.
- [x] 2.4 Update the active architecture roadmap's VE-002 decision, invariants, bounds, and status without changing unrelated verification queues.

## 3. Authority transition

- [x] 3.1 Review every direct dependent of the modified reference-transaction requirement and record why typed evidence, topology, persistence, lifecycle, and product claims remain outside Quint authority.
- [x] 3.2 Verify the proposal, delta spec, design, tasks, model, evidence, and implementation agree on the current-versus-proposed authority transition.
- [x] 3.3 Sync the verified delta into the current `explicit-transaction-machine` specification and archive the completed change.

## 4. Boundary verification

- [x] 4.1 Run focused Quint checks, mutation evidence, and the affected recovery/transaction Rust tests without broadening into a repo-wide Quint migration.
- [x] 4.2 Run strict OpenSpec validation and the documentation knowledge boundary checks; build maintained documentation when links or evidence changed.
- [x] 4.3 Complete final authority, evidence-scope, non-claim, and next-campaign review, then close the Beads work item.
