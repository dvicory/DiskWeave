## Why

M9's first implementation allowed declarative policy plus algebraic parity agreement to manufacture lost topology authority, detached frontend publication from the admitted service object, left persisted checksum evidence retrospectively relabelable, and did not exercise the concrete SQLite uncertain-commit boundary. These defects can authorize or report a different semantic object than the one actually proven.

## What Changes

- **BREAKING**: classify total recovery-metadata loss with surviving payload as a non-executable new-lineage operation until independent topology/identity authority and verified fresh parity/checksum construction exist.
- Bind frontend publication and discovery to a digest derived by the admitted service from its exact topology and member identity observations.
- Persist checksum evidence with its exact target/range, topology epoch, profile, checksum-set generation, and content generation; reject stale, mixed, duplicate, unsupported, or relabeled records.
- Persist prior/proposed SQLite commit reconciliation evidence before publication and reconcile exact semantic state after uncertain acknowledgement.
- Return semantic production-command failures through the shared `dwv.operator.v1` result in both human and JSON modes.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `metadata-loss-recovery`: total metadata loss with surviving data is explicitly new-lineage creation, not recovery of prior topology.
- `operator-recovery`: production apply refuses that currently unsupported new-lineage path; production outcomes share one result boundary.
- `healthy-portable-io`: admitted services expose a publication identity bound to exact admitted topology and observed member identities.
- `linux-ublk-frontend`: publication metadata and discovery consume the admitted publication identity rather than fixture coincidence.
- `checksum-plane`: durable validity records carry and revalidate exact checksum evidence bindings.
- `recovery-state-semantics`: the concrete SQLite adapter persists and reconciles exact prior/proposed states around uncertain commits.

## Impact

Affected code: `src/operator.rs`, `src/cli.rs`, `crates/dwv-service`, `crates/dwv-frontend-ublk`, `crates/dwv-recovery`, and `crates/dwv-recovery-sqlite`. Affected evidence: focused operator, service, SQLite failure-injection, workspace, and Linux publication acceptance checks. No compatibility shim is retained; early-development recovery records may require recreation.
