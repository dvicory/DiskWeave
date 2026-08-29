## ADDED Requirements

### Requirement: Scan-independent writable startup admission is complete and conservative
<!-- dwv:req req.healthy-portable-io.scan-independent-writable-startup-admission-is-complete-and-conservative -->
<!-- dwv:requires req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:requires req.anchorless-topology-identity.writable-assembly-fails-closed-on-unresolved-identity -->
<!-- dwv:requires req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit -->
<!-- External target prerequisite: `req.file-backed-stores.post-acquisition-writer-claim-release-is-exact-and-owner-observed` from `define-file-backed-post-acquisition-release`; restore a normal `dwv:requires` edge only after that transition and before implementation, validation, or canonical sync. -->
<!-- dwv:requires req.store-operation-contracts.capability-evidence-determines-the-allowed-safety-profile -->
<!-- dwv:requires req.store-operation-contracts.resource-admission-and-identity-remain-bounded-and-explicit -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.store-operation-contracts.store-failures-are-conservative-and-testable -->
<!-- dwv:requires req.store-operation-contracts.admission-stabilization-evidence-is-typed-and-store-scoped -->
<!-- dwv:requires req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->
<!-- dwv:requires req.recovery-state-semantics.writable-recovery-ownership-is-crash-releasing -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->
<!-- External reviewed-target prerequisite: `req.recovery-state-semantics.durable-writable-session-lifecycle-binds-authority-and-close-evidence` from the separately recovered `define-portable-writable-session-lifecycle` target remains external until integrated, reviewed in this lineage, and canonicalized; restore a normal `dwv:requires` edge only after that transition and before implementation, validation, or canonical sync. -->
<!-- dwv:requires req.recovery-state-semantics.new-protection-epoch-admission-binds-current-authority-and-stabilization -->
<!-- dwv:requires req.checksum-plane.current-baseline-completion-is-persisted-and-exact -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->
<!-- External target prerequisite: req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts from define-coded-range-clean-coordination; restore the normal owner edge after integration and before implementation, validation, or canonical sync. -->
<!-- External target prerequisite: req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set from define-coded-range-clean-coordination; restore the normal owner edge after integration and before implementation, validation, or canonical sync. -->
<!-- dwv:requires req.operator-recovery.production-assessment-is-observational-and-multidimensional -->

The healthy portable service SHALL define one baseline writable-startup admission for a complete stable present assignment set reopening after a custody gap. It SHALL re-observe and validate current array identity, topology, assignment bindings, geometry, store capabilities, recovery health, checksum obligations, and selected service profile; reacquire every required store-writer claim or equivalent exclusion and writable-recovery authority as one all-or-nothing current premise before the reviewed external `define-portable-writable-session-lifecycle` target establishes durable session begin; acquire no separate pre-begin session authority; and obtain typed `AdmissionStabilizationEvidence` from the store-operation owner for every required store while both each exact current claim/resource/alias binding and writable-recovery authority are continuously held and freshly revalidated through every pre-commit gate, the epoch commit, and observation of its result. After that observed commit, healthy service independently holds and freshly revalidates those authorities through admission return; operator start separately owns continued revalidation through frontend handoff and successful publication.

The store-operation owner is the sole canonical producer of the cross-custody physical no-late fact, while the file-backed/store ownership owner is the sole claim allocator and resource/alias revalidator. Each successful result SHALL bind the opaque exact current claim token, current store incarnation, current ordering domain, stable backing-resource identity and alias set, selected capability/profile and its declared and evidence-supported physical reachability universe, assignment/topology context, monotonic stabilization boundary, and exact predecessor physical-reachability/closure scope within that universe. Success means only that every covered pre-boundary effect in that exact scope has completed or ceased before the boundary; a new incarnation, process loss, lease acquisition, current-domain-only fence, or generic flush is insufficient. Old kernel/device queues, DMA, raw or out-of-band paths, aliases, and remaps are required only when included by the selected profile's universe. Routes and actors explicitly outside it remain profile assumptions and non-claims. Stabilization is not a persistence fence and does not prove already-visible bytes durable.

The service SHALL aggregate successful, correctly current-bound stabilization evidence for every required store before consuming checksum-owner evidence compared under exact target/range, topology epoch, digest profile, checksum-set generation, target content generation, and persistence evidence; the recovery-state owner-produced range-role basis snapshot from accepted current recovery inspection bound to exact recovery generation, topology epoch, role, range, and source-owner evidence; and separate coded-range-authority and recovery `CLEAN` owner observations. It SHALL not combine, relabel, reconstruct, or turn either owner observation into a startup input. It then obtains durable writable-session begin only from the reviewed active lifecycle target and requests recovery's durable new protection-epoch commit. Recovery holds and freshly revalidates writable-recovery authority through the epoch commit and observation of its result only. Healthy service independently revalidates every exact claim/resource/alias binding and writable-recovery authority from that observed commit through admission return; operator start separately continues those checks through frontend handoff and publication.

Known partial authority-acquisition failure SHALL invoke owner-directed handling for both store claims and writable-recovery authority. For file-backed leases, startup SHALL consume the file-backed owner’s current definite release-all rule for every acquired partial lease; the separate post-acquisition owner result SHALL NOT weaken or replace that partial-acquisition contract. The writable-recovery owner separately determines its authority disposition. An unknown or mismatched partial acquisition SHALL remain unresolved rather than be treated as no claim.

After all required file-backed claims have been acquired and accepted as the complete set, startup SHALL retain the file-backed owner's opaque accepted ownership value, which owns the exact original stores and exposes store operations only by non-extracting borrow, through every later gate. Any startup path that abandons the attempt before successful endpoint publication SHALL consume that value itself to request the file-backed owner release result. A replacement, remapped, or reacquired same-store-ID claim SHALL NOT enter or release the accepted value. `released-all` SHALL supply its owner-issued observation, preserve the original startup refusal/failure and any durable session/epoch or other owner facts, and establish only that the old file-backed set is no longer held; it SHALL NOT trigger automatic retry, and any later attempt SHALL reacquire fresh under normal revalidation and fresh claim-bound evidence. `residual` or `uncertain` SHALL supply its observation together with retained non-duplicable unresolved ownership, make file-backed cleanup reconciliation-required for the live process, and preserve the exact held or unresolved acquisition bindings. Startup SHALL retain live ownership for same-owner reconciliation or retry, or until process termination; dropping accepted or unresolved ownership SHALL transfer still-held resources into file-owner quarantine, and startup SHALL NOT retain only the observation, infer release, freshly reacquire in the same process, or reuse claim-bound evidence. Process loss SHALL use the existing crash-release rule and a later process SHALL reacquire fresh. Writable-recovery authority remains under its separate owner throughout.

The service SHALL return a bounded admission result containing stable assignment and authority bindings, exact per-store current-claim/resource/alias and predecessor-closure stabilization evidence, writable-recovery authority held and freshly revalidated from the observed epoch commit through admission return, exact checksum-owner evidence, the recovery-produced range-role basis snapshot and source bindings, separate coded-range-authority and recovery `CLEAN` owner observations, lifecycle-target session-begin evidence, epoch identity and durable observation, allowed claims, non-claims, owner/cause information, and independently gated ranges. The basis snapshot preserves canonical `current`, `prior`, `unprotected`, `indeterminate`, and `not-yet-interpretable` values without promotion or inference. The service SHALL NOT treat its orchestration result as identity, topology, store persistence, session lifecycle, recovery `CLEAN`, coded-range authority, checksum validity, current protection, payload correctness, custody continuity, range basis beyond that owner snapshot, operation outcome, operation-slot release, transaction release, or any other owner decision.

The baseline admission SHALL refuse or return the owning conservative outcome for ambiguous or stale identity/topology, missing or unavailable required stores, alias or geometry mismatch, known or unknown/mismatched partial authority acquisition, missing or lost exact current claim token or writable-recovery authority, resource replacement/remap or alias-set change, unsupported/failed/uncertain/incomplete/stale/mismatched/non-monotonic/predecessor-scope-incomplete stabilization, a new incarnation with unclosed predecessor effects, checksum evidence absent, partial, stale, duplicated, conflicting, or mismatched in exact target/range, topology epoch, digest profile, checksum-set generation, target content generation, or persistence evidence, absent/stale/conflicting/insufficient/mismatched recovery basis source evidence, unresolved coded-range-authority observation, unresolved recovery `CLEAN` capture/state, a missing or mandatory incomplete checksum baseline, or an uncertain session/epoch outcome. After accepted complete-set acquisition, `residual` or `uncertain` post-acquisition file-backed release blocks as reconciliation-required without inferred release, same-process fresh reacquisition, or claim-bound evidence reuse.

A successful baseline admission SHALL claim only that the selected assignments are stably and unambiguously bound, prerequisite writer/recovery authority and required store stabilization are in force through the observed epoch commit and admission return, exact checksum-owner evidence and the recovery-produced range-role basis snapshot are accepted, separate coded-range-authority and recovery `CLEAN` observations are preserved without combination or relabeling, the reviewed lifecycle target's session begin is durable, and the new protection epoch is durable. Its physical no-late claim is limited to the exact current claim/incarnation/domain, selected capability/profile and declared physical reachability universe, stable backing-resource/alias binding, and predecessor physical-reachability/closure scope carried by each store result; that predecessor coverage grants no authority in a predecessor domain or over routes or actors explicitly outside the selected universe. It does not prove operation outcome, persistence or durability, payload readability or correctness, parity/checksum validity, custody continuity, range basis beyond the owner-produced snapshot, `Current`, current protection, release, or full degraded recoverability.

#### Scenario: Complete stable assignments yield an admission candidate without a payload scan

- **WHEN** every required role and store has one unambiguous current binding and bounded availability observation, prerequisite writer and writable-recovery authority is current and continuously held, the exact current claim token and stable backing-resource/alias binding for every required store remain held and are freshly revalidated after stabilization, after exact checksum comparison and basis-source checks, through the reviewed lifecycle target's durable session begin, the epoch commit and observation of its result, and the healthy service's admission-return gate, the store-operation owner supplies successful exactly current-bound stabilization evidence for every required store binding a stable backing-resource identity, selected capability/profile and declared physical reachability universe, and exact predecessor physical-reachability/closure scope that covers every in-profile route/effect still capable of reaching it, separate coded-range-authority and recovery `CLEAN` owner observations are accepted without combination, and the service admission gates pass
- **THEN** the service returns a successful scan-independent admission candidate without enumerating all payload bytes; actual frontend publication remains a separate composition

#### Scenario: Writer exclusion exists without physical stabilization

- **WHEN** the service holds every required current claim token and stable-resource/alias binding, including for a new store incarnation or ordering domain, but any store lacks successful exact stabilization covering every route/effect within the selected profile's declared physical reachability universe that is still capable of reaching the same stable backing resource
- **THEN** admission refuses because current exclusion and identity alone do not prove that predecessor or unattributed physical effects have completed or that the binding remains valid

#### Scenario: Recovery history exists without physical stabilization

- **WHEN** recovery or operation history records prior identity, intent, disposition, or uncertainty but any required store lacks successful exact current-bound stabilization evidence with exact physical predecessor reachability/closure coverage within the selected profile's declared physical reachability universe
- **THEN** admission refuses because history does not establish physical quiescence; the service does not scan payload or create a persistent operation ledger

#### Scenario: An unknown physical route remains reachable

- **WHEN** the service can bind the exact current claim token, stable resource, and selected capability/profile but the adapter cannot identify and close, or otherwise certify closure of, an old kernel/device queue, DMA, raw or out-of-band path, alias/remap, unattributed effect, or any other route within that profile's declared physical reachability universe that may still reach the resource
- **THEN** the result is `unsupported` or `uncertain` as applicable, no admission candidate is returned, and no current authority is inferred from predecessor coverage

#### Scenario: A predecessor domain remains physically reachable

- **WHEN** the service can bind the exact current claim token, incarnation, and ordering domain but cannot close, or otherwise certify closure of, a predecessor claim, incarnation, or ordering domain whose effects may still reach the same stable backing resource

- **THEN** the result is `unsupported`, `failed`, or `uncertain` as applicable, no admission candidate is returned, and predecessor coverage supports only the current bound admission without transferring authority

#### Scenario: Stabilization is interrupted or the claim is lost

- **WHEN** process loss, store disappearance, timeout, claim loss, writable-recovery-authority loss, resource replacement or remap, alias-set change, or an unclassifiable adapter result interrupts stabilization or later premise revalidation
- **THEN** no writable admission candidate is returned; if the owner process remains live and the attempt is abandoned after complete-set acquisition, the full-set file-backed release rule applies, while process loss uses the existing descriptor crash-release rule; any later attempt reacquires and continuously holds/freshly revalidates the exact current claim/resource/alias binding and writable-recovery authority and produces fresh evidence rather than reusing or automatically upgrading the interrupted result

#### Scenario: Authority acquisition is known partial

- **WHEN** one required store writer or writable-recovery claim fails after another prerequisite claim has been acquired
- **THEN** no admission candidate is returned; startup consumes the file-backed owner’s current definite release-all rule for every exact acquired partial lease and leaves writable-recovery authority to its separate owner without inferring session authority or a writable endpoint

#### Scenario: Full-set startup abandonment consumes exact file-backed release

- **WHEN** all required file-backed claims were acquired and accepted as the complete set but stabilization or any later startup owner gate causes the attempt to be abandoned before successful endpoint publication
- **THEN** startup consumes the retained owner-issued accepted ownership value containing the exact original stores; `released-all` supplies owner-issued observation, preserves the original refusal/failure and any independently durable facts, and allows only a later fresh acquisition, whereas `residual` or `uncertain` supplies observation plus retained unresolved ownership for same-owner reconciliation or retry or until process termination; dropping either live owner value transfers still-held resources into file-owner quarantine, without inferred release, same-process fresh reacquisition, observation-only retention, or claim-bound evidence reuse; writable-recovery authority follows its separate owner

#### Scenario: The binding changes after owner gates

- **WHEN** the exact claim token or writable-recovery authority is lost, released, reacquired, or mismatched, or the stable resource is replaced, remapped, or gains a different alias after stabilization and before or during exact checksum or range-role basis checks, separate coded-range-authority or recovery `CLEAN` checks, session begin, epoch commit, admission return, frontend handoff, or publication
- **THEN** the evidence is invalid and no admission candidate or endpoint is returned; if the live startup attempt is abandoned, it consumes the retained owner-issued accepted ownership value for the exact originally accepted set, so changed claims and changed paths/resources cannot enter that value or receive transferred release authority; any unclassifiable exact claim remains `uncertain`, and the writable-recovery owner provides its separate fresh authority or reconciliation result

#### Scenario: Authority acquisition is unknown or mismatched

- **WHEN** process loss, a mismatched observation, or an unclassifiable result leaves partial claim acquisition uncertain
- **THEN** startup preserves the uncertainty, performs no automatic retry or second begin, and requires owner reconciliation before writable admission

#### Scenario: One required store is not stabilized

- **WHEN** all but one required store have successful exactly bound stabilization evidence under the captured startup premise
- **THEN** the complete baseline set remains unavailable and partial evidence does not authorize writable admission or publication

#### Scenario: Session begin is durable but epoch admission is rejected

- **WHEN** the lifecycle target has durably established session begin but recovery rejects epoch admission as a known precondition failure
- **THEN** the session-begin fact remains durable and separate, the prior epoch remains authoritative, no new writable epoch or endpoint is admitted, and lifecycle/recovery owners preserve their exact close, abort, release, retention, or reconciliation mechanisms; startup applies the full-set file-backed release rule without treating `released-all` as rollback or allowing `residual`/`uncertain` to authorize retry, while writable-recovery authority follows its separate owner disposition

#### Scenario: Session or epoch outcome is unresolved

- **WHEN** an unknown session or epoch outcome may have become durable and its observation is lost, corrupt, or unclassifiable
- **THEN** the service preserves separate prior/proposed evidence, remains unpublished, grants no new writable epoch or current claim, and requires owner reconciliation without automatic retry or a second begin

#### Scenario: Publication fails after durable admission

- **WHEN** session begin and epoch admission are durable but the selected frontend is unsupported, refuses publication, or fails after admission
- **THEN** the service reports the frontend outcome, preserves durable session/epoch evidence and owner next action, applies the full-set file-backed release rule to relinquish the exact accepted set without rollback or auto-close, leaves writable-recovery disposition to its separate owner, performs no auto-reacquire or auto-rebegin, and does not report an online endpoint

#### Scenario: Admission does not prove independent owner claims

- **WHEN** stable assignments, authority, stabilization, session begin, and epoch commit succeed but no operation outcome, durability, payload equality, semantic validation, parity/checksum validity, `CLEAN`, custody-continuity evidence, range basis, `Current`, current protection, or release evidence exists
- **THEN** the admission result claims only its bounded assignment, authority, and physical no-late facts and preserves all other dimensions as owner-produced non-claims
