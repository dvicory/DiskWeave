## Context

The current file-backed owner uses operating-system advisory locks on open descriptors. Canonical semantics already distinguish three surrounding cases:

1. a competing live writer is refused;
2. a partial multi-store acquisition failure releases every lease acquired by that failed acquisition;
3. process death releases descriptor leases and a later process must reacquire under fresh validation.

The uncovered case starts only after a higher-level acquisition has accepted the complete required file-backed writer-claim set. A later owner gate may fail or become uncertain, and the caller may then ask the file-backed owner to give that exact set back. The caller needs a truthful ownership observation. It does not need the file-backed layer to understand why startup failed or what durable protocol state means afterward.

## Goals and non-goals

**Goals:**

- Give `file-backed-stores` one exact owner observation for explicit release after accepted complete-set acquisition.
- Preserve exact ownership identity through success, known residual ownership, uncertainty, binding change, and process loss.
- Give consumers enough evidence to distinguish absence of live file-backed ownership from a state that still requires blocking or reconciliation.
- Preserve fresh-acquisition semantics after any completed release or process loss.

**Non-goals:**

- No change to the current definite partial-acquisition cleanup rule.
- No startup-gate policy, automatic retry policy, future daemon custody-retention policy, portable shutdown policy, or generic claim framework.
- No writable-recovery, writable-session, protection-epoch, recovery `CLEAN`, checksum, transaction, operation-slot, or frontend lifecycle semantics.
- No durable residual-lease ledger. File-backed leases remain process-scoped and crash-releasing.
- No Rust API, enum, module, schema, or implementation decomposition.

## Decisions

### 1. Release is bound to the accepted acquired set

An explicit post-acquisition release request names the exact writer claims that were accepted as the complete file-backed set by the caller. The resulting `FileBackedReleaseObservation` preserves, for each claim, the exact opaque claim token, store incarnation, stable backing-resource identity, and alias binding from that acquisition.

A later path replacement, alias change, remap, token loss by a consumer, or caller abandonment does not rewrite those identities. If the file-backed owner cannot classify an exact claim's current disposition, it reports uncertainty for that acquisition binding rather than inventing a replacement identity or absence of ownership.

### 2. Per-claim disposition owns the aggregate result

Each exact acquired claim has one ownership disposition in the observation:

| Per-claim disposition | Meaning |
|---|---|
| `released` | The live file-backed owner establishes that it no longer holds this exact claim. |
| `held` | The live file-backed owner establishes that it still holds this exact claim after the release request. |
| `uncertain` | The live file-backed owner cannot establish whether this exact claim remains held. |

The aggregate result is derived, not guessed:

- `released-all` only if every exact claim is `released`;
- `residual` if at least one exact claim is `held`, while preserving any other `released` or `uncertain` members;
- `uncertain` if no exact claim is known `held` but at least one is `uncertain`.

Operational errors may be preserved as owner/cause information, but they do not substitute for the ownership disposition. In particular, an error is not automatically proof that a claim remains held, and loss of an operation result is not proof that it was released.

### 3. Only `released-all` proves absence of live ownership for the set

A consumer may use `released-all` only for the narrow fact that this live file-backed owner no longer holds any claim from the exact requested set. The observation does not authorize a retry by itself.

`residual` and `uncertain` remain non-successful ownership results. A live process may continue owner-directed reconciliation or retry release of the same exact unresolved claims, or it may terminate. It may not infer release, reacquire the same resources as a fresh set, or reuse claim-bound stabilization/admission evidence while an exact claim remains held or unresolved.

### 4. Process death is cleanup, not a returned release observation

Process death releases the process's advisory descriptor locks under the existing canonical rule. Because the process cannot return an observation after it has died, that event does not retroactively turn an interrupted explicit request into `released-all`.

A later process starts from fresh acquisition. It performs the existing identity, geometry, topology, and recovery revalidation and obtains new claim-bound evidence. No release observation from the dead process is needed to reuse the physical files after operating-system cleanup.

### 5. Release does not roll back independent durable state

The release observation concerns only ephemeral file-backed writer ownership. A `released-all` result does not erase a durable writable-session begin, undo or reject a protection epoch, certify recovery `CLEAN`, validate or invalidate checksums, prove payload correctness, close writable-recovery authority, or change frontend publication state.

Those facts remain with their existing owners. This is especially important for startup failures after durable session or epoch mutations: file-backed claims may be released while those durable facts remain true and must be interpreted on the next action or restart.

### 6. Startup decides when to request release

This change does not enumerate startup gates or decide that every future daemon failure must relinquish custody. The active startup owner decides when its attempt is abandoned and requests post-acquisition release when its own policy requires cleanup. A future long-lived daemon can define a deliberate retained-custody state by not making that request; no such policy is added here.

## Failure and restart consequences

- `released-all`: the exact old file-backed set has no live ownership from that owner. A later attempt may reacquire only as a fresh acquisition with normal revalidation and new claim-bound evidence; no automatic retry is implied.
- `residual`: exact held claims remain identified. Higher-level use that requires relinquished file-backed ownership blocks until the same owner releases/reconciles them or the process terminates.
- `uncertain`: exact unresolved claims remain identified. The caller cannot distinguish absence from continued ownership and therefore blocks or terminates rather than reacquiring by inference.
- process loss: descriptor cleanup follows the existing canonical crash-release rule. A new process reacquires fresh; durable protocol state is independently inspected and reconciled.

## Review boundaries

Review should reject any interpretation that:

- weakens the current partial-acquisition `SHALL release every acquired lease` rule;
- lets aggregate release success hide an exact held or uncertain claim;
- treats path/token disappearance or a later successful acquisition attempt as retrospective proof of release;
- makes process death equivalent to a returned successful explicit-release result;
- lets release imply rollback, clean shutdown, integrity, `CLEAN`, current protection, or publication;
- makes `file-backed-stores` own which startup gates abandon or whether a future daemon retains custody.
