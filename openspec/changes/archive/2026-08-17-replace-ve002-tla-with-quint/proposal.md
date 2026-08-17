## Why

VE-002 currently relies on a PlusCal/TLA+ model checked by TLC and a Rust checker. The model is independent, but it hides an important uncertainty boundary: an unclassified recovery-intent or home effect can be advanced by an implicit recovery step instead of an explicit reconciliation decision. Its transition policy is also repeated across the reference transaction, dirty/integrity, and recovery-state requirements. Quint provides a smaller executable authority surface that can expose those transitions and make seeded mutation failures observable without starting a repo-wide migration.

## What Changes

- Replace the VE-002 `RecoveryProtocol.tla` and `.cfg` artifacts with a model-only Quint specification and a concrete bounded analysis module.
- Model one admitted write obligation over two affected regions and two stores, with explicit durable-intent observation, home effect, recovery disposition, ownership, fence, checkpoint, crash/loss, abandonment, and reconciliation transitions.
- Delegate only the exact bounded transition and invariant semantics named by the reference transaction requirement to Quint. Keep typed fence admissibility, region mapping, topology identity, persistence implementation, and production recovery authority owned by their existing requirements.
- Simplify duplicate reference-machine transition prose so it names the Quint authority and retains only scope, composition, and non-claims.
- Replace the VE-002 ADR and focused evidence record, add the model to the verification manifest, and update the milestone's tool decision and status.
- Keep current canonical semantics authoritative until this change is verified and synced; the Quint model is proposed authority while the change is active.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `explicit-transaction-machine`: delegate the bounded reference transaction/recovery transition relation and its safety invariants to the canonical Quint model while preserving owner boundaries and implementation-independent traces.

## Impact

This is a verification and semantic-authority change. It changes no Rust API, persistent format, runtime dependency, or production behavior. It removes the TLA+/TLC tool input, adds a Quint CLI verification dependency for the canary, changes the source of VE-002 evidence, and requires dependent OpenSpec and evidence review before canonical sync.
