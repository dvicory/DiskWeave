## Context

The bounded current context for U11 contains canonical frontend lifecycle, operation-slot, transaction, dirty/restart, store-watermark, recovery-fence, portable-service, and Linux assembly/shutdown owners. Those owners define abandonment, terminal ownership, conservative failure, and evidence predicates. They do not yet compose a service-owned durable writable-session begin/close path: `RecoveryMutation` exposes `BeginWritableSession` and `CloseWritableSession`, but `HealthyPortableService` does not invoke them. The target ordering remains useful, but implementation cannot prove owner-approved close-session evidence until that ownership path is reconciled.

See `proposal.md` for the capability selection, rejected candidates, and the post-planning blocker. This change remains planning-only: current canonical specs, product implementation, evidence, and the independent recovery-inspection capability remain untouched.

## Goals / Non-Goals

**Goals:**

- Give the portable service one canonical owner for shutdown ordering and the clean/non-clean claim boundary.
- Preserve operation, transaction, dirty, watermark, fence, and claim-lifetime ownership in their existing requirements.
- Make endpoint withdrawal a prerequisite to releasing writable claims and make incomplete evidence observable as failure or reconciliation-required state.
- Keep the Linux requirement as a local adapter/refinement with its OS descriptor and ublk ownership behavior.
- Preserve a bounded stop outcome across clean, incomplete, forced, and process-loss paths without authorizing recovery or currentization.

**Non-Goals:**

- Startup, publication admission, deployment or mount-namespace ordering, post-gap epochs, currentization, rollover, retention, history, recovery mutation, repair, migration, or stable-format graduation.
- New compatibility promises, forced destructive cleanup, automatic retries of uncertain writes, or a second recovery/inspection tool.
- A generic lifecycle framework or a new semantic registry.

## Decisions

### Portable owner and refinement boundary

Add `req.healthy-portable-io.portable-shutdown-preserves-operation-ownership-and-claim-release-ordering` as the detailed portable policy owner. It owns only the service-level ordering and clean-claim predicate:

1. close admission;
2. quiesce frontends and namespace writers;
3. drain or durably hand off admitted operations;
4. reconcile indeterminate completions where possible;
5. obtain owner-approved exact checkpoint and close-session evidence;
6. withdraw every exported writable endpoint;
7. release claims only when no writable alias remains.

The existing operation-slot, frontend lifecycle, transaction, dirty/restart, store-watermark, and recovery-fence requirements remain owners of their predicates. The new owner composes them and does not repeat their internal transitions.

The existing Linux requirement is modified to reference the portable owner and retain only Linux-specific validation, OS descriptor claims, owned ublk endpoint removal, stale-endpoint handling, process-death behavior, and platform failure mapping. This is an ownership-preserving refinement, not a second portable shutdown policy.

### Conservative result boundary

A clean result requires successful completion of every applicable stage and accepted exact evidence. A missing, stale, partial, volatile, mismatched, or unavailable prerequisite produces failure or reconciliation-required state. Forced stop and process loss never manufacture a clean close certificate; dirty or indeterminate state and stale endpoint evidence remain inputs for later safe handling.

No new enum or wire format is prescribed here. The implementation must expose the existing repository's bounded human/structured result boundary with enough distinction to tell clean completion from failed or reconciliation-required stop. Concrete interface and status representation are delegated once these semantic distinctions are preserved.

### Implementation readiness blocker

Bead `dwv-hg0.4` owns one prerequisite reconciliation: identify the canonical owner and durable OpenSpec boundary for beginning a writable session before service, closing it only with exact admissible evidence, and preserving open/dirty or indeterminate consequences after failure or process loss. It may amend this change or produce a separate bounded prerequisite change as repository authority requires.

The blocker does not preselect an implementation shape or declare another campaign row to be the owner. U03 durable epoch admission, U08 closed-mutation checkpointing, and U25 deployment ordering are review inputs because they touch adjacent transitions; none is hard-coded as an unblocker. `dwv-hg0.1` remains blocked until this reconciliation is implementation-ready.

### Transition inheritance

Shutdown can begin only from an already admitted service. It does not bypass current admission, operation ownership, dirty evidence, fence admissibility, or endpoint ownership obligations. Frontend abandonment suppresses delivery interest only; it cannot release the operation slot or claim early. A clean checkpoint cannot be inferred from a timeout, process exit, unmount success, or absence of a completion. A later startup or recovery action remains governed by its own current requirements and is not authorized by a stop result.

### Ownership challenge and dependent review

The change-local reconciliation is:

| Current owner | Disposition | Semantic effect |
|---|---|---|
| `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics` | Requires; preserve | Supplies quiescence, loss, and abandonment meaning; no shutdown policy relocation. |
| `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations` | Requires; preserve | Retains admitted resources until terminal/reconciliation state. |
| `req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative` | Requires; preserve | Keeps transaction uncertainty and release ordering conservative. |
| `req.dirty-integrity-invalidation.failures-and-restart-are-conservative` | Requires; preserve | Keeps dirty/indeterminate consequences after incomplete stop. |
| `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence` and `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence` | Requires; preserve | Continue to decide whether clean/checkpoint claims are admissible. |
| `req.linux-ublk-frontend.assembly-and-shutdown-preserve-ownership-and-recovery-authority` | Refine | Linux keeps endpoint/descriptor/platform rules; portable ordering becomes the single generic policy owner. |
| `req.operator-recovery.start-composes-admission-and-actual-publication` | Dependent review only | Start remains unchanged; shutdown never creates publication authority. |

The dependent-review task must inspect every direct dependent of the new owner and the modified Linux requirement individually. No bulk reviewed-state acceptance is allowed.

### Focused evidence boundary

Evidence must observe the portable contract rather than prove unrelated recovery meaning: clean completion only after endpoint withdrawal and exact evidence; no clean result after incomplete drain/checkpoint/withdrawal; operation ownership retained after abandonment; forced/process-loss conservative state; and Linux-specific owned-endpoint/descriptor behavior where the platform is available. It must not claim custody continuity, current protection, historical continuity, payload integrity, or hardware durability.

## Risks / Trade-offs

- A portable owner can accidentally become an umbrella requirement. The explicit requires/refinement boundary and ownership challenge prevent it from redefining operation or fence semantics.
- A forced stop may leave stale endpoint or indeterminate state. That is intentional: underclaiming is safer than fabricating clean ownership, and later reconciliation remains explicit.
- Some frontends may lack a native endpoint-withdrawal primitive. They must report unsupported or reconciliation-required shutdown rather than silently release claims; interface-specific choices remain implementation work.
- The change does not settle startup/deployment ordering or recovery mutation. Those remain visible campaign rows rather than hidden dependencies of shutdown.
- Treating flush completion, unmount, process exit, endpoint removal, or an in-memory state transition as close-session evidence would manufacture a clean claim. The explicit prerequisite prevents that shortcut.
