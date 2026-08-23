## Context

The bounded current context for **portable shutdown, endpoint withdrawal, and claim-release ordering** contains canonical frontend lifecycle, operation-slot, operation/media-effect, transaction, dirty/restart, store-watermark, persistence-evidence, release-authorization, portable-service, and Linux assembly/shutdown owners. Those owners define abandonment, safe generation-qualified `Reclaimable`, terminal or authoritatively reconciled effects, conservative failure, and evidence predicates. They do not currently include the durable writable-session lifecycle defined by the recovered `define-portable-writable-session-lifecycle` target. That target remains non-canonical until its normal authority transition completes, so this shutdown change records it as an explicit external prerequisite without adding a false current `requires` edge.

See `proposal.md` for the capability selection, rejected candidates, and the post-planning blocker. This change remains planning-only: current canonical specs, product implementation, evidence, and the independent recovery-inspection capability remain untouched.

## Goals / Non-Goals

**Goals:**

- Give the portable service one canonical owner for shutdown ordering and the clean/non-clean claim boundary.
- Preserve operation, transaction, dirty, watermark, persistence-evidence, and claim-lifetime ownership in their existing requirements.
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
3. establish the shutdown close frontier and require every operation generation still owned at that frontier to reach the operation-slot owner's safe `Reclaimable` state after every child is terminal and required reconciliation is recorded;
4. require each applicable operation or media effect to be terminal or authoritatively reconciled under its existing owner;
5. obtain owner-approved exact recovery `CLEAN` and durable session-close evidence from owners that consume, but do not manufacture, those operation-slot and effect facts;
6. withdraw every exported writable endpoint;
7. release each claim only when its independent owner permits release and no writable alias remains.

The existing operation-slot, operation/media-effect, frontend lifecycle, transaction, dirty/restart, store-watermark, persistence-evidence, and generation-qualified release-authorization requirements remain owners of their predicates. After its normal authority transition, the recovered writable-session lifecycle remains owner of durable begin/close meaning and consumes those predicates without redefining them. The new shutdown owner composes the predicates only into service-level ordering and result meaning.

The existing Linux requirement is modified to reference the portable owner and retain only Linux-specific validation, OS descriptor claims, owned ublk endpoint removal, stale-endpoint handling, process-death behavior, and platform failure mapping. This is an ownership-preserving refinement, not a second portable shutdown policy.

### Conservative result boundary

A clean result requires successful completion of every applicable stage and accepted exact evidence. A missing, stale, partial, volatile, mismatched, or unavailable prerequisite produces failure or reconciliation-required state. Forced stop and process loss never manufacture a clean close certificate; dirty or indeterminate state and stale endpoint evidence remain inputs for later safe handling.

No new enum or wire format is prescribed here. The implementation must expose the existing repository's bounded human/structured result boundary with enough distinction to tell clean completion from failed or reconciliation-required stop. Concrete interface and status representation are delegated once these semantic distinctions are preserved.

### Implementation readiness blocker

The recovered `define-portable-writable-session-lifecycle` change is the explicit external target prerequisite under Bead `dwv-hg0.4`. It defines durable writable-session begin/close meaning, accepts close only for the same captured authority and exact owner evidence, and preserves open/dirty or indeterminate consequences after failure or process loss. Until that target completes its normal canonical transition, this shutdown change records no current `requires` edge to its proposed requirement.

The blocker does not preselect an implementation shape or declare another campaign boundary to be the owner. Typed stabilization and durable epoch admission, closed-mutation-set recovery `CLEAN`, and baseline deployment ordering are review inputs because they touch adjacent transitions; none is hard-coded as an unblocker. `dwv-hg0.1` remains blocked until the recovered lifecycle is canonical and implementation-ready.

### Transition inheritance

Shutdown can begin only from an already admitted service. It does not bypass current admission, operation ownership, dirty evidence, persistence-evidence admissibility, or endpoint ownership obligations. Frontend abandonment suppresses delivery interest only; it cannot release the operation slot or claim early. Recovery CLEAN cannot be inferred from a timeout, process exit, unmount success, or absence of a completion. A later startup or recovery action remains governed by its own current requirements and is not authorized by a stop result.

### Ownership challenge and dependent review

The change-local reconciliation is:

| Current owner | Disposition | Semantic effect |
|---|---|---|
| `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics` | Requires; preserve | Supplies quiescence, loss, and abandonment meaning; no shutdown policy relocation. |
| `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations` | Requires; preserve | Owns exact operation generations, terminal children, recorded required reconciliation, and safe `Reclaimable`; no shutdown or recovery fact may manufacture them. |
| `req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts` | Requires; preserve | Keeps operation/media-effect disposition, child terminality, recorded reconciliation, safe `Reclaimable`, and other applicable release observations independent before exact-generation `ReleaseAllowed`. |
| `req.recovery-state-semantics.durable-writable-session-lifecycle-binds-authority-and-close-evidence` from `define-portable-writable-session-lifecycle` | External target prerequisite pending normal canonical transition | Owns durable session begin/close meaning after transition; close consumes exact operation-slot and effect facts and cannot create them. |
| `req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent` | Requires; preserve | Supplies the canonical bounded transaction relation and normalized comparison boundary; exact states, outcomes, ordering, and release sequencing remain in Quint. |
| `req.dirty-integrity-invalidation.failures-and-restart-are-conservative` | Requires; preserve | Keeps dirty/indeterminate consequences after incomplete stop. |
| `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence` and `req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence` | Requires; preserve | Continue to decide whether recovery CLEAN or close-session claims are admissible. |
| `req.linux-ublk-frontend.assembly-and-shutdown-preserve-ownership-and-recovery-authority` | Refine | Linux keeps endpoint/descriptor/platform rules; portable ordering becomes the single generic policy owner. |
| `req.operator-recovery.start-composes-admission-and-actual-publication` | Dependent review only | Start remains unchanged; shutdown never creates publication authority. |

The dependent-review task must inspect every direct dependent of the new owner and the modified Linux requirement individually. No bulk reviewed-state acceptance is allowed.

### Focused evidence boundary
Evidence must observe the portable contract rather than prove unrelated recovery meaning: clean completion only after every operation generation still owned when the shutdown close frontier is established is safely `Reclaimable`, every child is terminal, required reconciliation is recorded, and each such operation's applicable operation/media effect is terminal or authoritatively reconciled; already released mutation history is covered by the closed-mutation-set owner's exact bounded lower-frontier/coverage evidence rather than an unbounded retained operation-slot ledger; exact recovery-`CLEAN` and session-close evidence is accepted; endpoint withdrawal completes; and independent claim-release predicates hold. It must also observe conservative outcomes when any predicate is missing and Linux-specific owned-endpoint/descriptor behavior where the platform is available. It must not claim custody continuity, current protection, historical continuity, payload integrity, or hardware durability.

## Risks / Trade-offs

- A portable owner can accidentally become an umbrella requirement. The explicit requires/refinement boundary and ownership challenge prevent it from redefining operation or persistence-evidence semantics.
- A forced stop may leave stale endpoint or indeterminate state. That is intentional: underclaiming is safer than fabricating clean ownership, and later reconciliation remains explicit.
- Some frontends may lack a native endpoint-withdrawal primitive. They must report unsupported or reconciliation-required shutdown rather than silently release claims; interface-specific choices remain implementation work.
- The change does not settle startup/deployment ordering or recovery mutation. Those remain visible campaign rows rather than hidden dependencies of shutdown.
- Treating a generic handoff, recovery fact, flush completion, unmount, process exit, endpoint removal, or in-memory state transition as terminal children, safe `Reclaimable`, an authoritatively reconciled effect, or close-session evidence would manufacture an owner fact. The explicit owner composition and external lifecycle prerequisite prevent that shortcut.
