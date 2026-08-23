## Why

The recovery contract exposes durable writable-session mutations and clean-session evidence, but it does not yet say what authority a session binds, when its begin is durable relative to writable publication, or what evidence distinguishes a clean close from an ordinary stop. Healthy service startup therefore has no owner-approved begin/close path, and the active `add-portable-shutdown-claim-release` change cannot truthfully consume close-session evidence without inventing a second policy.

This bounded prerequisite settles the recovery-owned lifecycle now so shutdown and scan-independent startup work can compose one contract without defining their local admission, ordering, or platform policy.
Bead `dwv-hg0.4` is the planning and reconciliation anchor for this prerequisite; its existing downstream dependency remains visible without changing shared Beads state here.

## What Changes

- Add one recovery-state requirement that owns durable writable-session begin/close meaning and its conservative restart/reconciliation boundary.
- Refine parity-envelope session representation so prepared/closing state may precede recovery `CLEAN`, while clean/closed representation follows accepted `CLEAN` or is atomic with it.
- Require begin to durably capture the accepted session authority and complete writable bindings before any writable publication or protected mutation is admitted.
- Require a clean close to consume exact owner-approved evidence for the session's closed mutation set, including applicable persistence/fence and recovery-`CLEAN` acceptance, without redefining those owners' predicates.
- Keep service stop, endpoint withdrawal, durable session close, and store/recovery claim release as separate facts with explicit incomplete, forced, process-loss, rejected, and uncertain outcomes.
- Expose durable begin and clean-close proof as prerequisites that downstream shutdown and startup changes compose; do not duplicate their local ordering or admission policy.
- Leave scan-independent stabilization and epoch admission, fresh-basis currentization, background rollover, prior-claim retention, coded-range coordination, shutdown ordering, Linux behavior, lock mechanisms, schemas, Rust types, and delegated-model semantics to their owning or later changes.

## Capabilities

### New Capabilities

None. The lifecycle is a missing requirement within the existing recovery-state semantic boundary; a new capability would split recovery authority.

### Modified Capabilities
- `recovery-state-semantics`: add the canonical durable writable-session begin/close owner and its relationship to recovery `CLEAN`, persistence evidence, and restart reconciliation.
- `parity-envelope-profiles`: refine session representation so clean/closed envelope meaning follows the lifecycle and recovery `CLEAN` ordering without becoming an authority owner.

## Impact

The planning artifacts give `add-portable-shutdown-claim-release` and the active `add-scan-independent-writable-startup` change one prerequisite contract; both downstream changes are intentionally untouched. Future implementation work will cross the recovery adapter, healthy service, operator start, store/fence, ownership, and lifecycle seams while preserving each current predicate owner. Future executable-model work is limited to the settled lifecycle boundary and remains separate from implementation and evidence configuration. No current canonical spec, product/model code, Beads state, or maintained documentation is changed by this proposal.
