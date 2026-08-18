## Context
The last-synced delegated source is `verification/quint/RecoveryProtocol.qnt`. The active target relocates and repairs that relation at `models/quint/RecoveryProtocol.qnt`. It models one abstract obligation over parameterized region and store sets, while `verification/quint/RecoveryProtocolAnalysis.qnt` supplies the two-region, two-store finite evidence instance. The target adds explicit range release, blocks `begin` from an unreleased terminal state, and prevents home reconciliation from reporting a durable whole-home effect after only partial mutation coverage. The Rust `dwv-transaction-ref` machine already exposes explicit `ReleaseRange`, rejects duplicate or out-of-order results without mutating state, and routes uncertain effects to reconciliation; it is a related implementation seam, not a mechanically identical model.

The knowledge scanner SHALL recognize marked delegated `.qnt` sources by their source-local requirement marker, regardless of whether they are under `verification/quint` or `models/quint`. The canonical knowledge contract does not let a filename create an owner. The active U11 change is target-only and remains blocked on a separate writable-session authority issue; this repair must not currentize or implement U11.

## Goals / Non-Goals

**Goals:**

- Make the delegated Quint relation complete for explicit ownership release, terminal and aborted outcomes, invalid/repeated transition behavior, and conservative home uncertainty.
- Keep the model parameterized and model-only. Preserve the distinction between the canonical source and finite analysis assumptions.
- Produce bounded positive, uncertainty, terminal-release/reuse, and negative/mutation evidence with deterministic reruns.
- Make delegated-source path changes participate in bounded knowledge impact and dependent review.
- Record the Rust seam assessment with two seeded Quint Connect projections, without introducing a fragile model-to-production adapter.
- Keep the repair active and proposed until verification; preserve historical source and archive records.

**Non-Goals:**

- No Rust transaction-machine redesign, production API change, persistent-format migration, or recovery-adapter change.
- No exhaustive arbitrary-width proof, physical durability claim, Rust implementation conformance claim, or repo-wide Quint migration.
- No delegation of region mapping, checksum semantics, topology, typed fence admissibility, persistence, operation-slot lifetime, frontend ownership, or production recovery authority.
- No implementation or canonicalization of the active U11 shutdown change.

## Decisions

1. **Represent release explicitly.** Add range-held and terminal-pending-release state to the target model. `begin` is enabled only for an unowned obligation. Successful checkpoint and pre-mutation intent rejection create owned terminal/aborted states; `release` is the only transition that returns ownership to `Unowned`, after which a new `begin` may acquire the released range. This matches the existing Rust action vocabulary and prevents terminal state from being silently overwritten.

2. **Use partial-transition semantics for invalid results.** Do not add a second error-model state machine. The Quint `step` relation contains only enabled abstract actions; an invalid, repeated, or out-of-order action has no successor. Repeated abandonment and repeated home-reconciliation observations are explicitly guarded. Result correlation across release and range reuse remains owned by the applicable evidence requirement; Quint owns only ordering and guards for observations already admitted to the current obligation. State invariants and disposable guard mutations make these boundaries observable without duplicating Rust error classes.

3. **Guard durable home resolution by represented coverage.** `HomeEffectDurable` is a whole represented home-effect outcome. The model admits it from unknown home state only when `attempted == Regions` and `mutated == Regions`. Indeterminate reconciliation records that the current observation was acknowledged; a later mutation clears that acknowledgement so a new observation is required. After volatile abandonment with incomplete coverage, reconciliation completes the currently represented mutation set while the remaining regions proceed through the abstract mutation path. A new invariant rejects any durable home state without complete mutation coverage.

4. **Keep uncertainty explicit and owned.** Unknown intent and home effects remain non-clean and owned by handoff/reconciliation states. Rejected intent becomes an owned pre-mutation-aborted state until release; durable intent remains dirty until complete home mutation, fence coverage, checkpoint, and release. The model does not decide how a concrete adapter proves prior/proposed persistence facts.

5. **Separate canonical relation from evidence.** The analysis module gains direct scenarios for release/reuse, rejected intent release, volatile-abandonment continuation, repeated-action rejection, and durable-home reachability. It retains finite sets and depth as evidence metadata only. The evidence record reports sampled and bounded checks, mutation counterexamples, deterministic replay, and non-claims; it does not promote analysis values to protocol semantics.

6. **Assess, do not force, Rust conformance.** Compare the model's semantic phases and outcomes with the Rust machine's public action/result seam. A bounded Quint Connect driver projects only the observable normal lifecycle through release using one mapped region and no abstract stores; reuse is covered by direct Quint and Rust checks rather than by Connect. It intentionally excludes abstract fence and home-reconciliation transitions whose concrete evidence is owned elsewhere. Because the model mutates one abstract region at a time while Rust batches exact reads, parity computation, writes, watermarks, and typed fence evidence, record the mismatch and retain independent focused Rust tests instead of adding a general translation layer.

7. **Make delegated-source review contractual.** Extend the change-boundary requirement and focused scanner tests so a changed marked delegated source produces `delegated_canonical_source_changed`, reviews marked current owners, and propagates review through the requirement dependent closure. Removing a prior marker is a removed or reassigned relationship; an unmarked model path is context only. A filename, historical artifact, or run output cannot create an owner.

## Risks / Trade-offs

- **[Risk]** The abstract relation could be mistaken for production recovery policy. **Mitigation:** retain explicit non-delegated owners, finite-bound wording, and the Rust/conformance mismatch in the ADR and evidence record.
- **[Risk]** Sampled runs may miss the release or partial-abandonment paths. **Mitigation:** add direct witnesses, deterministic seed reruns, bounded test checks, and disposable guard mutations that must fail an invariant.
- **[Risk]** Adding release state may reduce witness frequency or expose dead ends. **Mitigation:** test the complete checkpoint-release path and the rejected-intent release path before updating evidence claims.
- **[Risk]** Delegated-source review may over-stale unrelated documentation. **Mitigation:** classify only marked current sources, then use existing requirement dependent closure; historical and unmarked paths remain non-authoritative context.

## Migration Plan

1. Keep the active change delta and target artifacts separate from current OpenSpec semantics.
2. Update the target Quint model and finite analysis, then run typecheck, bounded Quint verification, bounded tests, seeded simulations, deterministic replay, disposable negative mutations, and the two seeded Quint Connect projections.
3. Review the Rust transaction seam and update the ADR, evidence log, manifest, maintained roadmap, and knowledge contract with exact claim boundaries.
4. Run focused Rust tests, knowledge scanner tests, strict OpenSpec validation, docs readiness/check/build, and affected dependent review.
5. Leave the change unsynced and unarchived for the next authority-transition step. Canonical sync and archive are intentionally outside this repair execution.
6. Rollback is deletion/reversion of the active repair edits; no payload, runtime, or persistent-state migration is required.
