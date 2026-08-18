## Why

The completed VE-002 canary moved the bounded reference relation to Quint, but the proposed target relation still has authority gaps: it can begin a new obligation directly from an unreleased terminal state, and abandonment after a volatile home effect can classify a partial mutation as durably complete. The target also leaves invalid or repeated transition handling implicit while the knowledge contract does not state that a changed marked delegated source requires owner and dependent review. Repair these boundaries before applying the canary's recommended U11 campaign.

## What Changes

- Complete the canonical `RecoveryProtocol` relation with explicit range ownership and release, an aborted pre-mutation outcome, and a guard that prevents a new begin until the prior terminal or aborted outcome releases its range.
- Make volatile-home abandonment and home reconciliation preserve incomplete mutation coverage; permit durable home resolution only when all affected regions are represented.
- Make invalid, repeated, and out-of-order actions partial relation attempts with no state transition, and add invariants and mutation checks for terminal release and conservative uncertainty.
- Extend the finite analysis with witnesses for release, released-range reuse, volatile abandonment, resumed mutation, and terminal-release ordering while keeping all region, store, and depth values evidence-only.
- Add bounded Quint verification, negative/mutation evidence, deterministic rerun evidence, and two seeded Quint Connect projections for the repaired transferred semantics.
- Record the exact mismatch between the abstract Quint relation and the Rust transaction seam; the Connect checks cover only the mapped state/action subset and do not claim a general conformance bridge.
- Make delegated canonical model changes a first-class knowledge change-boundary input that triggers review of the marked owner and its dependent closure.
- Update the active authority delta, ADR, portable evidence log, verification manifest, and maintained roadmap references while leaving historical TLA+ and archived change records unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `explicit-transaction-machine`: complete the delegated reference relation's release, invalid-transition, and conservative uncertainty surface.
- `documentation-knowledge-architecture`: classify changed marked delegated sources as semantic changes requiring owner and dependent review.

## Impact

The change updates the model-only Quint source, its finite evidence module, verification commands and records, documentation knowledge behavior, and maintained authority references. It changes no Rust production API, persistent format, runtime dependency, or recovery adapter. The active change remains proposed until focused model, dependent, OpenSpec, and documentation checks pass; historical artifacts remain historical provenance.
