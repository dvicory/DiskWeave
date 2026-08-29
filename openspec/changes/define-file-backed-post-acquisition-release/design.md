## Context

The current file-backed owner uses operating-system advisory locks on open descriptors. Canonical semantics already distinguish three surrounding cases:

1. a competing live writer is refused;
2. a partial multi-store acquisition failure releases every lease acquired by that failed acquisition;
3. process death releases descriptor leases and a later process must reacquire under fresh validation.

The uncovered case starts only after a higher-level acquisition has accepted the complete required file-backed writer-claim set. A later owner gate may fail or become uncertain, and the caller may then ask the file-backed owner to give that exact set back. The caller needs a truthful ownership observation. It does not need the file-backed layer to understand why startup failed or what durable protocol state means afterward.

## Goals and non-goals

**Goals:**

- Give `file-backed-stores` one exact owner observation for explicit release after accepted complete-set acquisition.
- Preserve exact ownership identity and any concrete operational cause through success, known residual ownership, uncertainty, binding change, and process loss.
- Keep the observation separate from the live linear ownership capability; unresolved release retains every ownership resource needed for same-owner reconciliation.
- Give consumers enough evidence to distinguish absence of live file-backed ownership from a state that still requires blocking or reconciliation.
- Preserve fresh-acquisition semantics after any completed release or process loss.

**Non-goals:**

- No change to the current definite partial-acquisition cleanup rule, and no unification of that cleanup path with post-accepted-set release.
- No startup-gate policy, automatic retry policy, future daemon custody-retention policy, portable shutdown policy, or generic claim framework.
- No writable-recovery, writable-session, protection-epoch, recovery `CLEAN`, checksum, transaction, operation-slot, or frontend lifecycle semantics.
- No durable residual-lease ledger. File-backed leases remain process-scoped and crash-releasing.
- No external typestate framework or general state-machine abstraction.

## Decisions

### 1. Release is bound to the accepted acquired set

An explicit post-acquisition release request names the exact writer claims that were accepted as the complete file-backed set by the caller. The resulting `FileBackedReleaseObservation` preserves, for each claim, the exact opaque claim token, store incarnation, stable backing-resource identity, and alias binding from that acquisition. The claim token includes a private owner-issued acquisition identity that is unique within one live file-backed owner process and does not depend on disposable marker existence or content. That identity and capabilities containing it expire wholesale at process death; no persisted or cross-process consumer interprets it.

At complete-set acceptance, the file-backed owner atomically marks every exact live claim as accepted and consumes the original writable stores into one non-duplicable `AcceptedFileWriterSet`. That value owns both the exact bindings and live stores, exposes immutable store references and a non-extractable mutable operation guard, and cannot be separated into an independent capability and raw stores. Any already-accepted member rejects the entire second acceptance. Explicit release consumes the accepted owner value itself. Ordinary Drop converts all still-held members into the same bounded file-owner quarantine used for unresolved release, so losing the value neither unlocks claims silently nor permits fresh same-process acquisition.

A later marker replacement, path replacement, alias change, remap, token loss by a consumer, or caller abandonment does not rewrite or recreate those identities. If the file-backed owner cannot classify an exact claim's current disposition, it reports uncertainty for that acquisition binding rather than inventing a replacement identity or absence of ownership.

### 2. Per-claim disposition owns the aggregate result

Each exact acquired claim has one ownership disposition in the observation:

| Per-claim disposition | Meaning |
|---|---|
| `released` | The live file-backed owner establishes that every ownership resource or lock constituting this exact claim is relinquished. |
| `held` | The live file-backed owner establishes that at least one ownership resource or lock constituting this exact claim remains held. |
| `uncertain` | The live file-backed owner cannot establish whether every ownership resource or lock constituting this exact claim is relinquished. |

For the current `FileStore`, one writable claim includes both the advisory lock on the backing payload descriptor and the `.dwv-lease` marker descriptor lock. Releasing only one is never `released`; it is `held` when remaining ownership is established and otherwise `uncertain`.

The aggregate result is derived, not guessed:

- `released-all` only if every exact claim is `released`;
- `residual` if at least one exact claim is `held`, while preserving any other `released` or `uncertain` members;
- `uncertain` if no exact claim is known `held` but at least one is `uncertain`.

Ownership disposition and operational cause are separate. When the owner observes a concrete unlock or close failure, loses the release result, or cannot classify an observation, the result preserves that cause even when the ownership disposition is independently known. An error is not automatically proof that a claim remains held, and loss of an operation result is not proof that it was released.

### 3. Observation is not the live ownership capability

A consuming release of a held writer set may return a success value only for `released-all`, when every claim's complete ownership-resource set is definitely relinquished. That success value has private owner construction, so a consumer cannot wrap or relabel a cloned residual or uncertain observation as success. A `residual` or `uncertain` result returns or retains a non-duplicable same-owner ownership value containing every unresolved descriptor and resource together with its observation and cause. If one member is definitely released before the aggregate resolves, its payload and marker descriptors close immediately while its existing `ProcessFileClaim` remains as a lightweight exact-resource set reservation.

The accepted ownership value and unresolved ownership value authorize only operations on their same exact claims, owner-directed reconciliation, or explicit release. They do not authorize fresh claim acquisition, transfer to changed paths or resources, or reuse of claim-bound stabilization or admission evidence. Their fields and constructors remain private to the file-backed owner. Ordinary Drop of either value transfers exact still-held resources and released-member process reservations into a bounded process-local owner quarantine keyed by acquisition identity. The quarantine owns every retained descriptor and reservation, rejects duplicate custody, and keeps every original accepted physical resource blocked through the existing live-claim owner registry until aggregate `released-all`. Retry removes the entry under the quarantine lock, releases that lock before filesystem unlock or claim teardown, and reinserts the same unresolved claim and reservation records if the aggregate remains unresolved. It does not create retry tombstones or retain released descriptors: live bookkeeping remains proportional to currently admitted, accepted, or unresolved claims, independent of retry count. Aggregate success drops every set reservation and released resources normally; process death remains the final operating-system cleanup boundary. No descriptor or reservation is forgotten or intentionally leaked.

A consumer may use the owner-issued `released-all` success only for the narrow fact that this live file-backed owner no longer holds any claim from the exact requested set. The observation does not authorize a retry by itself.

`residual` and `uncertain` remain non-successful set-level ownership results. A live process may continue owner-directed reconciliation or retry release of the same exact unresolved claims, or it may terminate. Even when one member's descriptors are definitely released, its process reservation remains in the unresolved set; the process may not infer aggregate release, reacquire any original physical resource as a fresh set, clone the observation and drop ownership, or reuse claim-bound stabilization/admission evidence. A genuinely replaced or remapped physical identity remains separate and cannot satisfy the old set.

### 4. Nested ownership releases in reverse order

The current nested ownership releases the backing-payload lock first while the `.dwv-lease` marker lock remains held, then releases the marker last. If payload release fails or is uncertain, the owner retains the marker and the unresolved payload ownership rather than opening a fresh-acquisition window.

An unlock attempt must not take a descriptor out of the live ownership value before its result is classified. In particular, an unlock error must not implicitly close/drop that descriptor while returning only an error or observation.

### 5. Process death is cleanup, not a returned release observation

Process death releases the process's advisory descriptor locks under the existing canonical rule. Because the process cannot return an observation after it has died, that event does not retroactively turn an interrupted explicit request into `released-all`.

A later process starts from fresh acquisition. It performs the existing identity, geometry, topology, and recovery revalidation and obtains new claim-bound evidence. No release observation from the dead process is needed to reuse the physical files after operating-system cleanup.

### 6. Release does not roll back independent durable state

The release observation concerns only ephemeral file-backed writer ownership. A `released-all` result does not erase a durable writable-session begin, undo or reject a protection epoch, certify recovery `CLEAN`, validate or invalidate checksums, prove payload correctness, close writable-recovery authority, or change frontend publication state.

Those facts remain with their existing owners. This is especially important for startup failures after durable session or epoch mutations: file-backed claims may be released while those durable facts remain true and must be interpreted on the next action or restart.

### 7. Startup decides when to request release

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
- treats release of only the payload or marker resource as release of the complete claim;
- drops unresolved descriptors or the marker exclusion while returning only a residual or uncertain observation;
- unifies partial-acquisition cleanup with post-accepted-set release solely to share an API;
- makes `file-backed-stores` own which startup gates abandon or whether a future daemon retains custody.
