## Context

`verification/quint/RecoveryProtocol.qnt` contains the canonical `RecoveryProtocol` module, which is the semantic authority for the parameterized protocol relation. Its separate `RecoveryProtocolAnalysis` module instantiates that relation for the VE-002 evidence run with one represented obligation, `Regions = {"data", "parity"}`, `Stores = {"data", "parity"}`, and `MaxDepth = 8`. Those finite values bound exploration and witness evidence only; they are not product limits or additional protocol semantics. The analysis module, sampled traces, manifest, and evidence record verify and report the relation but do not own its meaning.

The current `explicit-transaction-machine` specification repeats model-specific guards, states, outcomes, action order, and release sequencing in three requirements and also repeats part of that relation in the normalized-action and reference-trace requirements. The independent product contracts already live with dirty/integrity, recovery-state, store-operation, lifecycle, and healthy-service owners.

## Goals / Non-Goals

**Goals:**

- Keep the normalized-action requirement responsible for the semantic action vocabulary, semantic evidence fields, and backend-fanout boundary.
- Make the reference-trace requirement the only OpenSpec requirement that delegates the exact machine relation to Quint.
- Remove requirement blocks whose only remaining responsibility would duplicate delegated transition semantics.
- Preserve independent owner requirements for dirty-region mapping, typed fence evidence, persistence, operation lifetime, frontend abandonment, restart consequences, and production recovery authority.
- Distinguish the parameterized protocol model from the finite VE-002 verification instance.
- Rewire current and active dependents, then sync and archive only after focused verification passes.

**Non-Goals:**

- No change to `RecoveryProtocol.qnt`, its invariants, witnesses, or evidence commands.
- No claim that `MaxDepth = 8` or the two-region/two-store analysis instance is exhaustive.
- No change to production code, runtime behavior, persistent formats, or historical VE-002 records.
- No new general Quint migration or model authority outside the named VE-002 protocol relation.

## Decisions

1. **Remove the durable-intent requirement block.** Its exact protected-mutation guard and blocked outcome belong to the delegated Quint relation; durable dirty/integrity invalidation and recovery requirements own the independent product predicates.

2. **Remove the clean/checkpoint requirement block.** Its exact fence, checkpoint, clear, terminal, and release ordering belongs to Quint; store, recovery-state, dirty/integrity, and healthy-service requirements own independent evidence and clean-claim admissibility.

3. **Remove the failure/abandonment requirement block.** Its exact uncertainty, handoff, terminal, and abandonment outcomes belong to Quint; lifecycle, operation-slot, dirty/restart, and healthy-service requirements own independent custody, lifetime, and conservative-failure behavior.

4. **Modify normalized action emission.** Retain the semantic action vocabulary, semantic ranges/generations/identities/evidence, and one-action-per-fanout boundary. Remove deterministic sequence wording and scenarios that prescribe the delegated machine's ordering.

5. **Retain the reference-trace requirement as the sole delegation boundary.** It owns normalized trace versioning, stable comparison/error representation, and implementation-independent comparison. It names the canonical `RecoveryProtocol` module in `verification/quint/RecoveryProtocol.qnt` as the authority for the exact parameterized state/action/invariant relation. The separate `RecoveryProtocolAnalysis` module binds the finite evidence instance only; its runs, witnesses, and evidence records are not semantic authority.

6. **Rewire dependents.** Remove references to deleted IDs from the healthy portable I/O contract, the active portable-shutdown change, evidence manifest, curriculum, maintained guide, and campaign references. Use the actual dirty/integrity, recovery-state, store-operation, lifecycle, healthy-service, or retained reference-trace owner instead.

7. **Keep the authority transition staged until verification.** The active delta remains proposed behavior while checks run. After dependent rewiring and documentation verification, sync the canonical specs and archive the change; the evidence instance never becomes a semantic owner.

## Risks / Trade-offs

- Removing three requirement IDs is intentional, not cosmetic: retaining them would leave no independent product decision after exact machine semantics move to Quint.
- The finite evidence instance is concrete and reproducible but does not establish unbounded reachability, arbitrary region/store cardinality, or production implementation correctness.
- A future expansion of Quint authority must change the single reference-trace delegation boundary and review affected owner requirements; it must not restore duplicated machine scenarios.
