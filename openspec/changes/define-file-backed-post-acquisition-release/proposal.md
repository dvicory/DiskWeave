## Why

Current `file-backed-stores` semantics already own crash-releasing descriptor leases, competing-writer refusal, definite cleanup when multi-store acquisition itself fails partway, and fresh revalidation on reacquisition. They do not define what the file-backed owner can truthfully report when a caller that already holds an accepted complete writer-claim set later asks to release it.

The active scan-independent startup target reaches that gap after stabilization and later owner gates. It correctly refuses to invent release, retry, or reacquisition authority, but therefore cannot finish its failure composition. This change adds one file-backed owner observation for that post-acquisition case without making the file-backed layer own startup abandonment policy, writable-session or protection-epoch lifecycle, shutdown, recovery `CLEAN`, checksum state, or future daemon custody policy.

## What Changes

- Add an exact typed `FileBackedReleaseObservation` for an explicit release request over one exact post-acquisition writer-claim set. The observation is not the live ownership capability.
- Consume the original writable stores into one opaque owner-issued accepted ownership value when the complete set is accepted. It retains every exact claim binding and store, exposes store operations without permitting member extraction, and is the only normal live-process source of explicit release for that set. Dropping it transfers still-held claims into bounded owner quarantine rather than silently unlocking them. A same-`StoreId` replacement, remap, or reacquisition cannot enter or release the original accepted set.
- Preserve each acquired claim's exact claim token, including a file-backed-owner acquisition identity that is unique within one live owner process, expires at process death, and does not repeat in that process if disposable marker state is removed or recreated, plus its store incarnation, stable backing-resource identity, and alias binding. A claim is `released` only when every ownership resource or lock constituting that claim is definitely relinquished. The current file-backed claim includes both the backing-payload lock and the `.dwv-lease` marker lock. Partial release within one claim is `held` or `uncertain`, never `released`.
- Preserve ownership disposition separately from operational cause. Each claim is observed as `released`, `held`, or `uncertain`; when a concrete operational failure or lost or unclassifiable observation exists, the owner also preserves that cause.
- Derive the aggregate owner result conservatively: `released-all` only when every exact claim is `released`; `residual` when at least one exact claim is known `held`; otherwise `uncertain` when at least one exact claim cannot be classified.
- Make `released-all` the only structurally owner-issued result that establishes absence of live file-backed ownership for that exact set. The accepted ownership value retains all original stores until explicit release or quarantine transfer. Residual or uncertain release retains unresolved resources plus each original member's existing lightweight exact-resource process reservation until the aggregate resolves. Definitely released payload and marker descriptors close normally; only the process reservation remains to block premature same-process reacquisition. Retry reuses bounded live claim/set records rather than appending history, and final `released-all` removes all reservations. Dropping either live owner value transfers its exact resources and reservations into bounded owner-managed quarantine that supports deadlock-safe retry and never forgets or intentionally leaks descriptors.
- Keep process death separate from an observed explicit-release result. Operating-system descriptor cleanup releases the dead process's leases and expires all process-local acquisition and quarantine identities, while a later process begins with fresh acquisition and the existing identity, geometry, topology, and recovery revalidation rules.
- Keep the current partial-acquisition cleanup rule unchanged and structurally separate from the post-accepted-set release protocol. This change does not weaken its requirement that every acquired partial lease be released when acquisition itself fails.
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
