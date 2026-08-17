## Why

DiskWeave's current canonical shutdown behavior is complete for the Linux frontend and for individual operation owners, but the portable service boundary does not yet own one explicit ordering contract for admission closure, quiescence, drain/reconciliation, checkpoint evidence, endpoint withdrawal, and claim release. The v0.9 target makes that ordering a reusable product guarantee; settling it now is useful on its own because an operator can stop a running service without confusing a clean stop with recovery authority, and it does not require the still-open recovery, publication, or currentization capabilities.

## What Changes

- Add one portable shutdown contract to the existing `healthy-portable-io` semantic owner.
- Require shutdown to stop new admission, quiesce frontends and namespace writers, drain or durably hand off admitted work, reconcile indeterminate completions where possible, obtain only owner-approved checkpoint/close evidence, withdraw exported endpoints, and release claims only after writable aliases cannot remain.
- Define conservative outcomes: a clean shutdown is reported only after every required step and exact fence evidence succeeds; failure or reconciliation-required state never receives a clean-close claim, and forced shutdown preserves dirty or indeterminate state.
- Keep lifecycle, operation-slot, dirty/restart, store-watermark, and recovery-fence ownership with their current requirements; use the delegated reference-trace requirement for exact transaction states, outcomes, ordering, and release sequencing without redefining that machine.
- Narrow the Linux frontend requirement to a platform refinement that maps OS descriptor claims, ublk endpoint ownership, cleanup, and process-death behavior onto the portable shutdown owner rather than creating a second generic shutdown policy.
- Leave startup/publication admission, deployment/mount ordering, post-gap currentization, recovery-state mutation, retention, historical recovery, and the independent recovery-inspection change out of scope.

## Capabilities

### New Capabilities

None. The operator capability is an additional requirement in the existing portable service contract; creating a parallel capability spec would split ownership.

### Modified Capabilities

- `healthy-portable-io`: own portable shutdown, endpoint withdrawal, and claim-release ordering and its conservative result boundary.
- `linux-ublk-frontend`: retain Linux-specific acquisition, endpoint, descriptor-lock, process-death, and bounded cleanup behavior as a refinement of the portable owner.

## Selection and Independence

The selected capability is **U11 / C1-C2 — Stop a running service without releasing live ownership early**. Its first observable outcome is a bounded shutdown result: clean only when the portable sequence and exact fence evidence complete, otherwise failure or reconciliation-required with dirty/indeterminate state preserved. It authorizes no recovery decision, publication, currentization, destructive action, compatibility change, or stable-format claim.

Candidate dispositions from the campaign's unresolved rows:

- **Selected planning boundary; implementation blocked — U11 portable shutdown and claim release:** current operation, transaction, dirty, watermark, fence, and Linux lifecycle owners define the required pieces, and the target delta defines their portable ordering. Implementation inspection established that `HealthyPortableService` does not durably begin or close the existing recovery writable-session state, so no implementation may claim owner-approved close-session evidence until Bead `dwv-hg0.4` reconciles that missing ownership path.
- **Rejected — U02/C0b broader recovery and claim status:** depends on unresolved U13-U17 history, claim-lifetime, retention, and release semantics; it cannot produce a truthful standalone status contract yet.
- **Rejected — U03-U08/C2 foreground post-gap publication/currentization:** requires unresolved coded-range coordination, closed checkpoint admission, fresh-basis writes, indeterminate reconciliation, and destructive-authority boundaries; it would authorize mutation.
- **Rejected — U26a/C8a independent recovery inspection:** already has an open implementation-ready change and Bead chain; selecting it again would duplicate an earlier open capability.
- **Rejected — U26b/C8 broader recovery tooling:** remains a mixed open row containing equation verification, parity construction, export, recovery-plan explanation, migration, and damaged/unknown drills; several depend on U26a or unresolved recovery authority and format decisions.
- **Rejected — U25 deployment ordering:** startup/publication and consumer/mount ordering depend on the still-open C1/C2 target and introduce a broader deployment boundary; it is not needed to stop an already admitted service.
- **Rejected — U09/U10/U12 background rollover, fairness, and generic job lifecycle:** each depends on later mutation/recovery capabilities or remains unresolved retained intent rather than an operator-closed contract.
- **Rejected — U13-U20 history, retention, adoption, and topology transitions:** they require unresolved claim-lifetime, authorization, preservation, or destructive/irreversible decisions.
- **Rejected — U21-U24 namespace, mover, staging, and encryption:** campaign posture is ownership-or-scope-unresolved or optional product scope, and none is required for a safe stop.
- **Rejected — U35 live macOS bridge:** evidence-only and explicitly not a product semantic capability; this planning slice does not add evidence or implementation results.

No user decision is inferred for the blocked path. Before implementation, `dwv-hg0.4` must establish the canonical owner and durable OpenSpec boundary for writable-session begin/close, exact admissible close evidence, and failure/restart consequences. U03, U08, and U25 are review inputs only; this plan does not declare any of them the unblocker.

## Impact

- The delta changes only planning artifacts under this change; current canonical specs, product code, tests, evidence, and the existing recovery-inspection change remain untouched.
- Product implementation is blocked on `dwv-hg0.4`; the discarded local implementation attempt is not evidence and is not part of this change.
- Implementation must preserve all current owner boundaries and make no claim that shutdown establishes custody continuity, current protection, historical continuity, recovery authority, or payload integrity.
- Focused evidence must observe clean completion, incomplete drain/checkpoint/withdrawal, forced or owner-death shutdown, no premature claim release, and preservation of dirty/indeterminate state without depending on the recovery-command implementation or evidence.
- Canonicalization must review the new portable owner and every Linux dependent individually, archive only after strict verification, update the temporary campaign, export the Bead viewer state, and leave all unrelated open rows open.
