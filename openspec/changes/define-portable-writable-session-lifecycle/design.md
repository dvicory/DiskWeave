## Context

The canonical recovery boundary already owns generation-checked durable mutations, conservative commit observations, persistence evidence for recovery `CLEAN` and clean-session claims, and a recovery writable claim. Its semantic snapshot has a writable-session record and its mutation vocabulary can record begin and close, but the healthy service does not yet drive those transitions. The current healthy-service, operator-start, store-watermark, operation-lifetime, dirty/restart, ownership, and Linux requirements each retain their own predicates; none is the right owner for the meaning of a durable session record.

The active `add-portable-shutdown-claim-release` change is a target contract, not current authority, and explicitly records this missing prerequisite. The active `add-scan-independent-writable-startup` change also consumes durable session begin as an external target prerequisite but retains its own admission, stabilization, epoch, and publication policy. The v0.9 roadmap and normalization campaign are used only as reconciliation input; they do not define this design.
Bead `dwv-hg0.4` is the bounded reconciliation anchor for the missing lifecycle owner; its downstream dependencies remain coordination inputs rather than additional semantic authority.
**Non-normative recovery provenance:** This candidate was recovered from reviewed source `ntpooqkpnypwwtxqoozlutxvoqspxylr` / `2145e1649ea1934555c8dff396b083e82d3fe792`, and the current child contains explicit reviewed repairs.

## Goals / Non-Goals

**Goals:**

- Give one current-capability owner a durable begin/close contract that a fresh shutdown or startup author can consume.
- Bind a session to the exact accepted authority and writable assignment evidence that its later close must match.
- Make durable-before-publication ordering and uncertain commit handling observable without prescribing an implementation representation.
- Make clean close a composed evidence claim rather than a synonym for process stop, endpoint withdrawal, or claim release.
- Preserve explicit owner boundaries for operation lifetime, dirty/integrity, store watermark/fence, recovery `CLEAN`, identity/topology, and ownership requirements.
- Leave the downstream proof and model work small enough to implement and verify independently.

**Non-Goals:**

- No scan-independent stabilization, durable protection-epoch admission, fresh-basis currentization, background rollover, prior-claim retention, or post-gap publication policy.
- No portable shutdown sequence, endpoint-specific ordering, operator result mapping, or Linux behavior.
- No new lock mechanism, persistence schema, Rust API/type, database choice, or delegated-model transition design.
- No closed-mutation-set algorithm or fence supersession rule; the lifecycle consumes those owner-approved decisions.
- No claim that a clean session proves custody continuity, payload integrity, historical continuity, current protection, or later absence of out-of-band writes.

## Decisions

### One recovery-state owner

Add one requirement under `recovery-state-semantics` rather than creating a service-lifecycle capability or moving ownership into healthy service. Recovery state already owns durable generation transitions, session claims, and admissibility of persistence evidence. The lifecycle requirement has a one-way external-target dependency on `req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set` from `define-coded-range-clean-coordination` until that requirement completes its normal authority transition; the closed-set owner does not depend on this service-facing consumer. The parity-envelope requirement refines the lifecycle only for envelope representation and copy agreement. Healthy service, operator start, and shutdown will compose these owners and retain their local orchestration or publication policies.

**Alternative rejected:** make healthy service own the session record. That would duplicate recovery's durable authority and leave non-service recovery or startup paths without a canonical session meaning.

**Alternative rejected:** make the closed-set or envelope owners depend on the lifecycle consumer. That would reverse semantic ownership and make lower-level recovery facts depend on a high-level service.

### Begin captures authority, not mechanisms

A begin records one fresh session identity, the accepted array/topology authority, the topology epoch, every required assignment and its stable identity/generation and store incarnation, protected geometry, the recovery generation, the session/dirty-envelope generation, each bound store's ordering-domain identity, the accepted capability/profile binding, and any optional baseline watermark that is already current and owner-approved. It deliberately does not record future write watermarks or close fences as if they already existed. It consumes current decisions from identity, topology, recovery health, capability, and store-ownership owners; it does not acquire or release claims or reinterpret their predicates.

**Alternative rejected:** record only a session identifier and topology epoch. That would let a later close certificate be applied to a changed assignment, store incarnation, geometry, or recovery generation.

### Durable begin is the writable boundary

The begin must be durably observed before any writable endpoint publication, writable request admission, or protected payload mutation. A known precondition rejection leaves the exact prior recovery state authoritative and is a definite refusal; a durable observation makes the proposed session state authoritative; a lost, corrupt, or unclassifiable observation leaves prior-versus-proposed authority unresolved, blocks publication, and requires reconciliation. This is a semantic ordering boundary; it does not select the adapter, transaction representation, or persistence engine.

**Alternative rejected:** publish first and repair the session record afterward. That would make a live writable service exist without durable recovery authority and could turn process loss into an unowned interval.

### Clean close consumes owner-approved evidence

The lifecycle owner accepts a close only for the same session and captured authority. A clean-close claim requires exact-generation owner evidence for every operation generation still owned when the close frontier is established: each such operation must reach the operation-slot owner's safe `Reclaimable` state with terminal children and recorded required reconciliation, and its applicable operation or media effect must be terminal or authoritatively reconciled under its existing owner. Already released mutation history SHALL NOT require an unbounded retained operation-slot ledger; its inclusion in the session's closed mutation set is represented only by the closed-set owner's exact bounded lower-frontier/coverage evidence. Neither that closed-set evidence nor recovery `CLEAN` or session-close acceptance can establish terminal children, recorded reconciliation, safe `Reclaimable`, or an effect disposition for an operation still owned at the frontier. The durability/store/fence owners must accept exact evidence for every store and participating ordering domain, and recovery, dirty/integrity, and closed-mutation-set owners must accept recovery `CLEAN` for the exact closed mutation set. A generic handoff observation cannot substitute for any of those facts. The lifecycle owner does not define operation terminality, child reconciliation, the closed set, its admission frontier, historical compaction, whether a global fence supersedes per-write evidence, or any owner predicate. The close transition itself must also be durably observed.

A known close precondition rejection leaves the exact prior session and recovery state authoritative and is not reconciliation-required. A durable close observation makes only the proposed durable session transition authoritative. A clean-close result is available only after accepted recovery `CLEAN` and the parity-envelope owner's clean/closed representation transition follow that evidence or occur atomically with it; a prepared/closing representation may precede `CLEAN` but is not a clean claim. A lost, corrupt, or unclassifiable close observation leaves prior-versus-proposed session authority unresolved and authorizes no clean close; it neither authorizes nor revokes independently owned recovery-`CLEAN` or claim-release decisions.

The one-way external-target dependency on `req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set` prevents a session from consuming the old race-open `CLEAN` predicate. Exact capture membership, frontier, generation coverage, and any fence supersession remain owned by `define-coded-range-clean-coordination` and the applicable durability owners until their normal authority transitions complete.

**Alternative rejected:** treat a flush, timeout, unmount, endpoint removal, process exit, or in-memory close flag as a certificate. Those observations do not cover all mutations or prove durable recovery authority.

### Keep lifecycle facts separate

Service stop is a frontend/service outcome. Endpoint withdrawal is publication ownership. Session close is a durable recovery fact. Store and recovery claim release is owned by the applicable ownership boundary, and `req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts` remains the sole owner of generation-qualified `ReleaseAllowed` composition. These facts can be observed in different orders, and none is inferred from another. A clean or uncertain session close neither establishes nor revokes that independent authorization. Downstream shutdown may require a particular order, but that order remains outside this change.

### Restart is conservative and requires a new begin

A durably clean-closed session is prior lifecycle evidence only. A later writable service must establish a fresh begin against currently accepted authority before publication. A prior definite begin or close rejection leaves the prior state authoritative and may be retried only after its precondition is corrected; an open, partially closed, uncertain, or binding-mismatched session blocks writable admission until recovery and store authorities reconcile it. Process loss may release crash-releasing claims under the ownership contract, but it never manufactures a clean close or proves continuity of the old claim.

**Alternative rejected:** treat process survival, process exit, endpoint absence, or a clean database observation as a reusable session certificate. None is sufficient to establish current bindings or the absence of unresolved effects.

### Downstream composition boundary

The active `add-portable-shutdown-claim-release` change will use this requirement as a prerequisite after it closes admission, quiesces, and obtains the exact operation-slot and recovery owner dispositions required by its own endpoint and claim policy. The active `add-scan-independent-writable-startup` change uses durable begin after its own authority, no-late, stabilization, checksum, and range-basis decisions and before durable protection-epoch admission and publication. This change supplies dependency and affected-review handoffs; it does not edit either downstream change or copy their local policy.

The existing shutdown-model work in `dwv-hg0.7` may consume these settled lifecycle observations and outcomes. This change creates no new Quint module and no second session model; model correspondence remains a downstream handoff rather than a parallel implementation path.

## Risks / Trade-offs

- **Risk:** Exact closed-mutation-set coverage is supplied by multiple owners and could be interpreted as a hidden session policy. **Mitigation:** the requirement names the evidence it consumes and explicitly leaves set construction and fence supersession to coded-range, durability, dirty, and recovery owners.
- **Risk:** A conservative open or uncertain session can delay restart. **Mitigation:** preserve a bounded reconciliation disposition and require current owner evidence rather than silently upgrading uncertainty.
- **Risk:** Implementations may collapse stop, withdrawal, close, and release into one cleanup path. **Mitigation:** keep each fact and its non-implication explicit in the requirement and make every downstream task test the distinctions.
- **Risk:** Adding a requirement under recovery could be mistaken for a startup or shutdown policy. **Mitigation:** downstream composition is named as a prerequisite only; all scan-independent admission and shutdown ordering remain non-goals.

## Migration Plan

No implementation or canonical-spec migration occurs in this planning change. A future apply sequence is strictly one-at-a-time: complete `dwv-hg0.5`, `dwv-hg0.6`, and the coded-range/CLEAN delegated-model gate; implement the recovery-owned begin/close seam; then hand the settled lifecycle to healthy service, parity-envelope, `add-portable-shutdown-claim-release`, and `add-scan-independent-writable-startup` through their affected-owner reviews. The existing `dwv-hg0.7` shutdown model may consume the lifecycle observations; this change adds no Quint module or second session model. Focused evidence and normal verification precede syncing the deltas into canonical specs. Before that verification, the current canonical requirements and both downstream active changes remain unchanged, so abandoning this plan requires no rollback migration.

## Open Questions

None at the semantic level. Representation, adapter sequencing, and model encoding are intentionally implementation decisions after this contract is accepted.
