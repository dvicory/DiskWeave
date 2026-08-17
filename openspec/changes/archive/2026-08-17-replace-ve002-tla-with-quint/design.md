## Context

The current VE-002 model is a model-only PlusCal state machine with two abstract regions and stores. Current canonical requirements split ownership: dirty/integrity semantics own invalidation and restart consequences, recovery-state semantics own commit observations and typed fence authority, store semantics own persistence evidence and lifetimes, and the reference transaction owns action ordering and normalized traces. The change must preserve those boundaries while making one exact bounded transition relation executable.

## Goals / Non-Goals

**Goals:**

- Make the bounded reference transaction/recovery transition relation executable with the repository's installed Quint CLI.
- Expose intent-commit uncertainty and home-effect uncertainty as explicit states requiring explicit reconciliation.
- Preserve the old model's finite state bound, two-region/two-store symmetry, safety invariants, reachability checks, and mutation-test bar.
- Make the authority boundary readable from the owning OpenSpec requirement and the model itself.
- Keep the model independent of Rust types, production state, database layout, and runtime behavior.

**Non-Goals:**

- Modeling exact dirty-region geometry, checksum extents, topology identity, generation arithmetic, typed watermarks, adapter persistence, operation-slot resources, or frontend delivery.
- Encoding production recovery authority, operator interpretation, compatibility policy, or physical durability claims.
- Migrating other verification models or adding a general Quint framework.
- Exhaustive model checking. The canary uses `quint run` simulations and bounded mutation checks; exhaustive `quint verify` is outside this request.

## Decisions

1. **Use one plain Quint state machine, not Choreo.** VE-002 has one abstract actor and shared protocol state, not message-passing participants. A single `State` record keeps all coupled fields together and avoids translating TLA+'s flat variables into a second abstraction layer.

2. **Keep the model in a parameterized module with an analysis module.** `RecoveryProtocol` declares `MaxDepth`, `Regions`, and `Stores`; `RecoveryProtocolAnalysis` binds a two-region/two-store instance and supplies assumptions, witnesses, and simulation entry points. This preserves the old finite symmetry while making the model runnable without uninitialized constants.

3. **Make uncertainty explicit instead of recovering it implicitly.** Intent has `none`, `pending`, `durable`, and `unknown`; home effects have `unmodified`, `volatile`, `durable`, and `unknown`; recovery has `clean`, `dirty`, and `indeterminate`; ownership has `none`, `inflight`, `handoff`, and `terminal`. `unknown` states cannot reach clean or terminal ownership. Explicit reconciliation actions represent owner-approved resolution. The model intentionally covers the positive durable-resolution branch after an unknown home effect; exact prior/proposed artifact comparison remains a recovery-adapter responsibility outside the model.

4. **Delegate the transition relation, not every recovery predicate.** Quint owns the exact enabled-action/post-state relation for the bounded abstract state and its invariants. The current requirements continue to own typed evidence, exact region/checksum coverage, topology, persistence, resource lifetime, and product-facing claims. Removing those boundaries would turn a small canary into a second recovery implementation.

5. **Replace, do not retain, the TLA checker path.** Keeping both TLA+ and Quint as accepted VE-002 authorities would create duplicate semantic sources and hide drift. The old TLA files and TLC/tla-rs evidence are removed from the active evidence path; the archived OpenSpec change remains historical provenance, not current authority.

6. **Use a single mutation target.** Remove the durable-intent guard from the Quint `mutate` action in a temporary copy and run the same invariant simulation. The canary passes only if `MutationRequiresIntent` reports a counterexample. The mutant is disposable and never becomes repository evidence.

## Risks / Trade-offs

- [Risk] The finite model can be mistaken for the whole recovery contract. → The OpenSpec delegation names its exact state/action surface and lists non-delegated owners; ADR and evidence log repeat the finite bounds and non-claims.
- [Risk] Quint simulation is sampled rather than exhaustive. → Run multiple seeds with witnesses and invariants, retain commands and observations, and keep the change's claim at bounded executable evidence.
- [Risk] Removing TLA+ loses a second checker cross-check. → Require a seeded mutation failure, explicit uncertainty witnesses, deterministic reruns, and independent focused OpenSpec/docs checks before sync.
- [Risk] The model's positive home-reconciliation branch does not encode exact prior/proposed artifact comparison. → Keep that decision explicitly owned by `recovery-adapters-report-conservative-commit-observations`; do not use Quint to authorize it.

## Migration Plan

1. Implement and run the Quint model and disposable mutation check while the change delta remains proposed.
2. Update the ADR, evidence log, milestone status, and verification manifest only after the focused runs pass.
3. Run `cargo xtask docs check`, focused Quint commands, OpenSpec strict validation, and the required documentation build.
4. Sync the modified `explicit-transaction-machine` requirement into current specs only after verification confirms the delegation boundary and all direct dependents have been reviewed.
5. Archive the completed change. Rollback is a clean revert of the change and restoration of the historical TLA artifacts only if a later review finds the Quint canary insufficient; no product data or runtime migration is involved.

## Open Questions

None. The bounded authority surface and non-delegated recovery decisions are settled by this design.
