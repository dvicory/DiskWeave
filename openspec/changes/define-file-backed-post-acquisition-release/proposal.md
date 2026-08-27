## Why

Current `file-backed-stores` semantics already own crash-releasing descriptor leases, competing-writer refusal, definite cleanup when multi-store acquisition itself fails partway, and fresh revalidation on reacquisition. They do not define what the file-backed owner can truthfully report when a caller that already holds an accepted complete writer-claim set later asks to release it.

The active scan-independent startup target reaches that gap after stabilization and later owner gates. It correctly refuses to invent release, retry, or reacquisition authority, but therefore cannot finish its failure composition. This change adds one file-backed owner observation for that post-acquisition case without making the file-backed layer own startup abandonment policy, writable-session or protection-epoch lifecycle, shutdown, recovery `CLEAN`, checksum state, or future daemon custody policy.

## What Changes

- Add an exact typed `FileBackedReleaseObservation` for an explicit release request over one exact post-acquisition writer-claim set.
- Preserve each acquired claim's exact claim token, store incarnation, stable backing-resource identity, and alias binding from the accepted acquisition. Each claim is observed as `released`, `held`, or `uncertain`; operational cause may be reported separately without changing that ownership disposition.
- Derive the aggregate owner result conservatively: `released-all` only when every exact claim is `released`; `residual` when at least one exact claim is known `held`; otherwise `uncertain` when at least one exact claim cannot be classified.
- Make `released-all` the only result that establishes absence of live file-backed ownership for that exact set. Residual or uncertain results preserve the exact unresolved identities and do not authorize inferred release, same-attempt reacquisition, or reuse of claim-bound evidence.
- Keep process death separate from an observed explicit-release result. Operating-system descriptor cleanup releases the dead process's leases, while a later process begins with fresh acquisition and the existing identity, geometry, topology, and recovery revalidation rules.
- Keep the current partial-acquisition cleanup rule unchanged. This change does not weaken its requirement that every acquired partial lease be released when acquisition itself fails.
- State explicitly that file-backed release changes only ephemeral writer ownership. It does not roll back, close, validate, invalidate, or reinterpret writable-session, protection-epoch, recovery, `CLEAN`, checksum, publication, or other independently owned state.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `file-backed-stores`: add exact post-acquisition release observations while preserving the existing partial-acquisition and process-death rules.

## Impact

- This is a semantic target only; production implementation remains downstream work.
- The active scan-independent startup target can consume this owner observation when its own policy abandons a startup attempt after the complete file-backed claim set was acquired.
- No canonical `openspec/specs/*` file, product source, test, executable model, persistent format, or Bead is changed by this target alone.
