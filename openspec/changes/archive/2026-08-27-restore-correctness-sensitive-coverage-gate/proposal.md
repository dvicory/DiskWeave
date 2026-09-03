## Why

The current knowledge readiness gate validates canonical identities, fingerprints, and reference shape, but it can still report ready when a correctness-sensitive requirement has no meaningful production-Rust owner or applicable verification evidence. The archived `2026-08-09-open-traceable-knowledge` change records the intended sparse ownership/evidence boundary, but it is historical evidence rather than current authority and has no current canonical `req.*` identity to preserve.

## What Changes

- Modify the existing canonical-requirement discovery requirement so it owns only current extraction and identity; move its existing uncovered/check consequence to the new coverage owner rather than leaving duplicate policy ownership.
- Add one current `documentation-knowledge-architecture` requirement that puts every current canonical `Requirement` in the coverage gate without a second requirement/classification registry.
- Require derived meaningful implementation ownership and applicable test, model, executable-scenario, or evidence coverage for each target, unless that target has its own validated explicit absence/deferment disposition.
- Define meaningful Rust ownership as a current `dwv:req` relationship on a real semantic production scope, not a marker on every helper; keep tests and evidence as separate endpoint classes.
- Require the full evidence-boundaries contract for counted verification: executable input, exercised mechanism or invariant, observed outcome, evidence tier, closest unsupported claim boundary, bounded scope, and explicit non-claims.
- Persist a deterministic per-target digest of the typed owner/evidence endpoint set that was semantically reviewed in the existing reviewed-requirement entry; endpoint-set changes become stale even when requirement fingerprints do not change.
- Define independent `(requirement identity, target)` `not-applicable` and `deferred` dispositions with bounded scope, non-claims, reason, current fingerprints, and machine-evaluable deferred triggers; a target disposition cannot conceal a different target’s omission.
- Extend `knowledge ownership`, export, readiness, and `docs check` so target states, endpoint digests, dispositions, omissions, and trigger transitions are visible and fail closed without treating a disposition as coverage proof.
- Make currentization atomic: `dwv-6c0.1` depends on `dwv-6c0.2`; one coherent cutover snapshot migrates reviewed state, syncs the modified/new requirements, activates the gate, self-covers the new requirement, backfills every current target with real coverage or independent dispositions, and then proves live readiness/check. Any `dwv-6c0.3` audit/repair is an external post-`.1` handoff and is not executed by this change.
- Keep source-root traversal and the meaning of repository `src/` and `tests/` roots in `dwv-6c0.2`; this change consumes the resulting typed endpoint model and does not add root traversal mechanics.
- Keep implementation, focused fixtures/evidence, canonicalization, and Bead closure as later tasks; this change edits no product code, models, Beads, current canonical specs, or maintained documentation.

## Capabilities

### New Capabilities

None. The obligation belongs to the existing documentation-knowledge architecture capability.

### Modified Capabilities

- `documentation-knowledge-architecture`: separate discovery from the canonical coverage owner, add target-level coverage/digest/disposition semantics, expose ownership outcomes, and define the atomic authority transition.

## Impact

The future apply sequence will touch the knowledge extraction/reference model, the existing reviewed-requirement state contract, ownership/export/readiness/`docs check`, focused `xtask` fixtures, and current Rust/evidence relationships only after the contract is canonical. `dwv-6c0.1` is explicitly blocked on complete `dwv-6c0.2` Rust-root traversal and path classification. The full current owner/evidence/disposition backfill and self-coverage occur in one coherent `.1` cutover snapshot; any `dwv-6c0.3` work begins only after `.1` closes and is outside this change’s execution.

Current canonical OpenSpecs remain authoritative until this change is applied, verified, and synced. The archived open-traceable change, its completion evidence, source markers, evidence records, reviewed state, generated export, Beads, and implementation are inputs or projections only; none defines the product rule. After normal OpenSpec synchronization, the new documentation-knowledge requirement becomes the sole semantic owner of coverage and disposition policy, while canonical product requirements remain owners of product behavior, the discovery requirement remains owner of extraction/identity, the fingerprint relationship requirement remains owner of fingerprint propagation, and `evidence-boundaries` remains owner of evidence claim scope and non-claims.
