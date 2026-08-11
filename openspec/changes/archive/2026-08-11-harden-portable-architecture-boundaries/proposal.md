## Why

DiskWeave's canonical architecture requires portable semantics to remain independent of storage, recovery, and frontend mechanisms, but the current Rust graph embeds the file-store adapter in the portable service, names SQLite in live ublk composition, and exposes SQLite evaluation types from portable recovery. The same audit found an unused recovery marker trait that adds no semantic contract.

## What Changes

- **BREAKING (internal Rust API):** make the healthy portable service execute through the semantic store-operation boundary instead of embedding `FileStore`; migrate every caller without compatibility shims.
- Preserve exact ranges, completion dispositions, persistence evidence, identity, watermarks, generational buffers, abandonment, and reconciliation across the service/store boundary, with file-backed and deterministic non-file conformance evidence.
- Move SQLite journal/synchronization/checkpoint/reset evaluation vocabulary into the SQLite adapter/evidence boundary and remove the unused `RecoveryStateAdapter` marker trait.
- Make live ublk publication consume an admitted service without selecting a concrete recovery adapter; retain disposable fixture composition as an explicitly bounded acceptance path.
- Preserve all persistent formats, recovery authority, durability claims, request semantics, supported frontend profiles, and production/demo separation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `architecture-contract`: Preserve the existing portable-boundary requirement unchanged while repairing implementation conformance.

## Impact

- Affected packages: `dwv-store`, `dwv-store-file`, `dwv-service`, `dwv-recovery`, `dwv-recovery-sqlite`, `dwv-frontend-ublk`, and the root `dwv` binary/tests.
- Internal service/member-binding and store payload APIs change in one clean cutover; no stable compatibility boundary has been declared.
- Cargo dependency direction changes so portable service and live frontend composition no longer select concrete outbound adapters.
- No persistent data migration, new dependency, new runtime, new product backend, or broader platform/durability claim is introduced.
