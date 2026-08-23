## Why

DiskWeave's current canonical shutdown behavior is complete for the Linux frontend and for individual operation owners, but the portable service boundary does not yet own one explicit ordering contract for admission closure, quiescence, drain/reconciliation, recovery CLEAN evidence, endpoint withdrawal, and claim release. The v0.9 target makes that ordering a reusable product guarantee; settling it now is useful on its own because an operator can stop a running service without confusing a clean stop with recovery authority, and it does not require the still-open recovery, publication, or currentization capabilities.

## What Changes

- Add one portable shutdown contract to the existing `healthy-portable-io` semantic owner.
- Require shutdown to stop new admission, quiesce frontends and namespace writers, establish a shutdown close frontier, require every operation generation still owned at that frontier to reach the operation-slot owner's safe `Reclaimable` state with every child terminal and required reconciliation recorded, require each such operation's applicable operation or media effect to be terminal or authoritatively reconciled under its existing owner, cover already released mutation history with the closed-mutation-set owner's exact bounded lower-frontier/coverage evidence rather than an unbounded retained operation-slot ledger, obtain only owner-approved recovery `CLEAN` and durable session-close evidence that consumes rather than manufactures those facts, withdraw exported endpoints, and release claims only after their independent owner predicates hold and writable aliases cannot remain.
- Define conservative outcomes: a clean shutdown is reported only after every required step and exact persistence evidence succeeds; failure or reconciliation-required state never receives a clean-close claim, and forced shutdown preserves dirty or indeterminate state.
- Keep frontend lifecycle, operation-slot, operation/media-effect, transaction, dirty/restart, store-watermark, persistence-evidence, and generation-qualified release-authorization ownership with their current requirements; consume the recovered writable-session lifecycle only after its normal authority transition, and use the delegated reference-trace requirement for exact transaction states, outcomes, ordering, and release sequencing without redefining that machine.
- Narrow the Linux frontend requirement to a platform refinement that maps OS descriptor claims, ublk endpoint ownership, cleanup, and process-death behavior onto the portable shutdown owner rather than creating a second generic shutdown policy.
- Leave startup/publication admission, deployment/mount ordering, post-gap currentization, recovery-state mutation, retention, historical recovery, and the independent recovery-inspection change out of scope.

## Capabilities

### New Capabilities

None. The operator capability is an additional requirement in the existing portable service contract; creating a parallel capability spec would split ownership.

### Modified Capabilities

- `healthy-portable-io`: own portable shutdown, endpoint withdrawal, and claim-release ordering and its conservative result boundary.
- `linux-ublk-frontend`: retain Linux-specific acquisition, endpoint, descriptor-lock, process-death, and bounded cleanup behavior as a refinement of the portable owner.

## Selection and Independence

The selected planning boundary is **portable shutdown, endpoint withdrawal, and claim-release ordering**, across the capabilities **Use ordinary members through one mediated service** and **Restart a complete array after a custody gap**. Its first observable outcome is a bounded shutdown result: clean only when the portable sequence and exact persistence evidence complete, otherwise failure or reconciliation-required with dirty/indeterminate state preserved. It authorizes no recovery decision, publication, currentization, destructive action, compatibility change, or stable-format claim.

Candidate dispositions from the campaign's unresolved rows:

- **Selected planning boundary; implementation blocked — portable shutdown, endpoint withdrawal, and claim-release ordering:** current operation-slot, operation/media-effect, transaction, dirty, watermark, persistence-evidence, release-authorization, and Linux lifecycle owners define the required pieces, and the target delta defines their portable ordering. The recovered `define-portable-writable-session-lifecycle` target defines the missing durable begin/close lifecycle contract under Bead `dwv-hg0.4`, but it is not current authority until its normal canonical transition completes. This change therefore records that target as an external prerequisite rather than a false current `requires` edge.
- **Rejected — broader epoch/basis/history/unknown/claim-lifetime status and full recovery/claim explanation:** depends on unresolved historical artifact roles, claim-lifetime, positive retention, and historical restore boundaries; it cannot produce a truthful standalone status contract yet.
- **Rejected — typed stabilization/admission, fresh-basis currentization, indeterminate-basis reconciliation, coded-range coordination, and closed-mutation-set recovery CLEAN:** requires unresolved coded-range coordination, closed recovery-state admission, fresh-basis writes, indeterminate reconciliation, and destructive-authority boundaries; it would authorize mutation.
- **Rejected — independent read-only recovery-state inspection:** already has an open implementation-ready change and Bead chain; selecting it again would duplicate an earlier open capability.
- **Rejected — independent equation verification, parity construction, candidate export, recovery-plan explanation, migration, and damaged/unknown drills:** remains a mixed open row containing those operations; several depend on independent recovery-state inspection or unresolved recovery authority and format decisions.
- **Rejected — baseline deployment/startup/shutdown dependency ordering:** startup/publication and consumer/mount ordering depend on the still-open mediated-service and post-gap-restart capabilities and introduce a broader deployment boundary; it is not needed to stop an already admitted service.
- **Rejected — background rollover, cross-operation fairness, and long-running job identity/resume/cancel/cursor:** each depends on later mutation/recovery capabilities or remains unresolved retained intent rather than an operator-closed contract.
- **Rejected — historical artifacts, claim lifetime, retention, adoption, and topology/profile transitions:** these require unresolved claim-lifetime, authorization, preservation, or destructive/irreversible decisions.
- **Rejected — whole-file namespace, mover, staging, and encryption:** campaign posture is ownership-or-scope-unresolved or optional product scope, and none is required for a safe stop.
- **Rejected — live macOS bridge:** evidence-only and explicitly not a product semantic capability; this planning slice does not add evidence or implementation results.

No user decision is inferred for the blocked path. Before implementation, `define-portable-writable-session-lifecycle` must complete its normal authority transition under Bead `dwv-hg0.4`, providing durable writable-session begin/close meaning, exact admissible close evidence, and failure/restart consequences. Typed stabilization/admission, closed-mutation-set recovery `CLEAN`, and baseline deployment ordering remain review inputs only; this plan does not declare any of them the unblocker.

## Impact

- The delta changes only planning artifacts under this change; current canonical specs, product code, tests, evidence, and the existing recovery-inspection change remain untouched.
- Product implementation is blocked on the recovered `define-portable-writable-session-lifecycle` external target prerequisite completing its normal canonical transition under Bead `dwv-hg0.4`; the discarded local implementation attempt is not evidence and is not part of this change.
- Implementation must preserve all current owner boundaries and make no claim that shutdown establishes custody continuity, current protection, historical continuity, recovery authority, or payload integrity.
- Focused evidence must observe clean completion, incomplete operation-slot reclamation predicates, unresolved operation/media effects, unavailable recovery-`CLEAN` or session-close evidence, incomplete endpoint withdrawal, forced or owner-death shutdown, no premature claim release, and preservation of dirty/indeterminate state without depending on the recovery-command implementation or evidence.
- Canonicalization must review the new portable owner and every Linux dependent individually, archive only after strict verification, update the temporary campaign, export the Bead viewer state, and leave all unrelated open rows open.
