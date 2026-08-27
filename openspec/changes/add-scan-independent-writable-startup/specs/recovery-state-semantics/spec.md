## ADDED Requirements

### Requirement: New protection-epoch admission binds current authority and stabilization
<!-- dwv:req req.recovery-state-semantics.new-protection-epoch-admission-binds-current-authority-and-stabilization -->
<!-- dwv:requires req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic -->
<!-- dwv:requires req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->
<!-- dwv:requires req.recovery-state-semantics.writable-recovery-ownership-is-crash-releasing -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->
<!-- External reviewed-target prerequisite: `req.recovery-state-semantics.durable-writable-session-lifecycle-binds-authority-and-close-evidence` from the separately recovered `define-portable-writable-session-lifecycle` target remains external until integrated, reviewed in this lineage, and canonicalized; restore a normal `dwv:requires` edge only after that transition and before implementation, validation, or canonical sync. -->
<!-- dwv:requires req.store-operation-contracts.admission-stabilization-evidence-is-typed-and-store-scoped -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:requires req.checksum-plane.current-baseline-completion-is-persisted-and-exact -->
<!-- dwv:requires req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->
<!-- External target prerequisite: req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts from define-coded-range-clean-coordination; restore the normal owner edge after integration and before implementation, validation, or canonical sync. -->
<!-- External target prerequisite: req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set from define-coded-range-clean-coordination; restore the normal owner edge after integration and before implementation, validation, or canonical sync. -->
<!-- dwv:requires req.operator-recovery.production-assessment-is-observational-and-multidimensional -->

The recovery-state owner SHALL define one durable protection-epoch admission for a writable service reopening after a custody gap. A successful epoch admission SHALL create a fresh protection-epoch identity and bind the accepted array and immutable topology snapshot, complete assignment identities and generations, protected geometry and profile, current writer-authority identity, writable-recovery authority held and freshly revalidated through the epoch commit and observation of its result, durable writable-session-begin evidence from the reviewed active `define-portable-writable-session-lifecycle` target, recovery generation, one successful and exactly bound `AdmissionStabilizationEvidence` for every required store, and checksum-baseline evidence compared under the checksum owner's exact target/range, topology epoch, digest profile, checksum-set generation, target content generation, and persistence-evidence bindings. The recovery-state owner SHALL itself produce and durably preserve a range-role basis snapshot from accepted current recovery inspection, bound to the exact accepted recovery generation, topology epoch, role, range, and source-owner evidence. The snapshot SHALL preserve the canonical values without promotion or inference: `current` names the exact active epoch, range, participating generations, and committed durable transition; `prior` names the exact earlier protected generation; `unprotected` requires affirmative accepted recovery inspection and source-owner evidence that the range is unprotected; `indeterminate` and `not-yet-interpretable` remain unresolved as named. Absent, stale, conflicting, insufficient, or mismatched source inspection or evidence—including wrong recovery generation, topology epoch, role, or range—SHALL refuse epoch admission rather than manufacture a basis classification. The store-operation owner is the sole producer of the physical no-late fact and the file-backed/store ownership owner is the sole claim allocator and resource/alias revalidator. Each accepted stabilization result SHALL bind the opaque current claim token, current store incarnation, current ordering domain, stable backing-resource identity and alias set, selected capability/profile and its declared and evidence-supported physical reachability universe, assignment/topology context, monotonic boundary, and exact predecessor physical-reachability/closure scope within that universe. Recovery SHALL consume these exact owner facts and SHALL NOT derive substitutes from recovery history, operation history, writer-claim presence, process loss, endpoint withdrawal, flush completion, elapsed time, a new incarnation, a current-domain-only fence, or a payload scan.

Still-owned in-process requests, children, resources, completions, and reconciliation remain under `OperationSlotTable` and its related owners. Their facts can constrain whether a store result may succeed, but recovery SHALL NOT copy their lifecycle policy, decide terminality or release, or require a persistent per-operation history across reboot. A successful stabilization result means only that every covered pre-boundary effect in the exact physical predecessor reachability/closure scope within the selected profile's declared universe has completed or ceased before the boundary, so none can mutate the stable backing resource afterward. It is physical quiescence, not a persistence fence, and makes no claim about routes or actors explicitly outside that universe. Recovery SHALL separately preserve the coded-range-authority observation from its owner and the recovery `CLEAN` capture, state, or uncertainty from its distinct owner. It SHALL NOT combine or relabel those observations, turn either into a new startup input, reconstruct coded membership or `CLEAN` capture membership, capture frontier, or future exclusion, or claim `CLEAN`. Recovery SHALL use `req.operator-recovery.production-assessment-is-observational-and-multidimensional` only as canonical semantic vocabulary and evidence predicates for its own range-role basis production; it SHALL NOT invoke or consume an operator command path. No untouched range becomes `Current`, and no classification is relabeled, replayed, or promoted. Stabilization SHALL NOT imply operation outcome, persistence or durability, payload readability or correctness, parity/checksum validity, recovery `CLEAN`, custody continuity, range basis, `Current`, current protection, operation-slot reclamation, transaction release, or any other independent owner fact.

Epoch admission SHALL require current identity/topology acceptance, all-or-nothing prerequisite store-writer and writable-recovery authority, acquisition and continuous holding of each exact current claim token, stable-resource/alias binding, and writable-recovery authority through successful stabilization, checksum-owner comparison, recovery basis-snapshot production, separate coded-range-authority and recovery `CLEAN` owner checks, durable session begin, the epoch commit, and observation of that commit result, with fresh owner revalidation at each boundary through that observation. `unsupported`, `failed`, `uncertain`, stale, conflicting, incomplete, non-monotonic, or mismatched stabilization; inability to identify and close or otherwise certify closure for any route/effect within the selected profile's declared physical reachability universe that remains capable of reaching a stable backing resource; claim/resource/alias or writable-recovery-authority loss, release, reacquisition, mismatch, or uncertain observation; checksum evidence absent, partial, stale, duplicated, conflicting, or mismatched in exact target/range, topology epoch, digest profile, checksum-set generation, target content generation, or persistence evidence; invalid range-role source inspection or evidence; unresolved coded-range-authority observation; or unresolved recovery `CLEAN` capture/state SHALL block the baseline profile under the applicable canonical owner contract. Routes and actors explicitly outside the selected universe remain profile assumptions and non-claims. A new incarnation, process loss, lease acquisition, or current-domain-only fence is not closure evidence. A lost or interrupted observation remains uncertain; a retry must begin only after exact claim/resource/alias and writable-recovery authority reacquisition and fresh stabilization. Known partial multi-store acquisition remains under the file-backed owner’s current definite release-all rule. If a higher-level startup attempt abandons after accepted complete-set acquisition, healthy-startup composition separately consumes `req.file-backed-stores.post-acquisition-writer-claim-release-is-exact-and-owner-observed`; recovery does not produce, infer, or reinterpret that file-backed result. The writable-recovery owner separately owns release, retention, or reconciliation. Recovery retains authority through the durable epoch commit and observation of its result, does not erase or roll back durable session/epoch facts after later publication failure, and does not decide terminality, child reconciliation, coded membership, `CLEAN` capture, transaction release, history compaction, or store stabilization internals.
Recovery does not require or certify future authority continuity through admission return, frontend handoff, or publication; healthy service and operator start own those post-commit gates separately, and later authority loss blocks publication without erasing durable session/epoch facts.

A durably admitted epoch authorizes only the exact current authority and owner evidence named by its record, including the separately preserved coded-range-authority observation, recovery `CLEAN` capture/state or uncertainty, and recovery-produced range-role basis snapshot. It SHALL NOT claim semantic correctness of present bytes, custody continuity across the gap, complete current integrity or protection, full degraded recoverability, historical equality, or a successful protected write to any range without the separately owned action/reconciliation gate.

#### Scenario: A complete present assignment set opens after a custody gap

- **WHEN** accepted lineage and topology identify every required assignment with bounded availability, prerequisite store-writer and writable-recovery authorities are current and continuously held, the exact current claim token and stable backing-resource/alias binding for every required store remain held and freshly revalidated through exact checksum comparison, recovery basis-snapshot production, separate coded-range-authority and recovery `CLEAN` owner checks, durable session begin, the epoch commit, and observation of that commit result, the store-operation owner supplies successful exactly current-bound stabilization evidence for every required store binding stable backing-resource identity, selected capability/profile and declared physical reachability universe, and an exact predecessor physical-reachability/closure scope that covers every in-profile route/effect still capable of reaching that resource, and every other named owner gate succeeds
- **THEN** recovery durably admits one fresh protection epoch bound to those facts and exposes the bounded epoch/basis record as an admission result; actual frontend publication remains separate

#### Scenario: Authority acquisition is known partial

- **WHEN** a required store writer, writable-recovery claim, identity binding, or topology assignment is unavailable or rejected after another prerequisite lease or claim was acquired
- **THEN** no epoch is admitted; known partial file-backed claims follow the current definite partial-acquisition cleanup rule and writable-recovery authority follows its separate owner, with no automatic reacquisition, retry, session begin, or epoch admission

#### Scenario: Authority acquisition is unknown

- **WHEN** process loss or an unclassifiable observation leaves partial claim acquisition uncertain or mismatched
- **THEN** the prior/proposed authority evidence remains unresolved, no epoch or writable endpoint is admitted, and recovery requires owner reconciliation without automatic retry

#### Scenario: Writer claim is held but stabilization is absent

- **WHEN** the exact current claim token and stable-resource/alias binding are held but any required store has no successful exact stabilization evidence covering every route/effect within the selected profile's declared physical reachability universe that remains capable of reaching its stable backing resource
- **THEN** no epoch is admitted because current writer exclusion, a new incarnation, or current-domain identity alone does not prove predecessor or unattributed physical effects have completed and the binding remains valid

#### Scenario: Recovery history is present without physical evidence

- **WHEN** recovery or operation history records prior identity, intent, disposition, or uncertainty but a required store lacks successful exactly current-bound stabilization evidence with exact physical predecessor reachability/closure coverage within the selected profile's declared universe
- **THEN** history does not substitute for physical no-late evidence, no payload scan or persistent operation ledger is introduced, and epoch admission is refused

#### Scenario: Stabilization is incomplete after claim acquisition

- **WHEN** any required store has `unsupported`, `failed`, `uncertain`, stale, mismatched, non-monotonic, or missing admission-stabilization evidence, cannot identify and close or otherwise certify closure for every route/effect within the selected profile's declared physical reachability universe that remains capable of reaching its stable backing resource, the exact claim/resource/alias binding or writable-recovery authority was lost, released, reacquired, mismatched, or became uncertain at any pre-commit gate through epoch-commit observation, checksum evidence is absent, partial, stale, duplicated, conflicting, or mismatched in exact target/range, topology epoch, digest profile, checksum-set generation, target content generation, or persistence evidence, or the recovery basis source inspection/evidence is absent, stale, insufficient, conflicting, or mismatched
- **THEN** no new epoch authority is created and the owning observation determines unavailable or reconciliation-required status; any higher-level abandonment after accepted complete-set acquisition is handled separately by startup through the exact file-backed release owner, while writable-recovery authority follows its separate owner

#### Scenario: A predecessor domain remains physically reachable

- **WHEN** a store adapter can bind the current claim, incarnation, stable resource, selected capability/profile, and ordering domain but cannot identify and close, or otherwise certify closure of, a predecessor or unattributed route/effect within that profile's declared physical reachability universe that may still reach the same stable backing resource
- **THEN** epoch admission is refused with `unsupported`, `failed`, or `uncertain` as applicable; predecessor coverage supports only the current bound admission and grants no authority in the predecessor domain

#### Scenario: The stabilization premise is lost before epoch commit

- **WHEN** a current claim or writable-recovery authority is lost, released, reacquired, mismatched, or becomes uncertain, or the stable backing resource is replaced, remapped, or gains a different alias after stabilization and before or during checksum comparison, basis-snapshot production, coded-range-authority or recovery `CLEAN` owner checks, durable session begin, the epoch commit, or observation of that commit result
- **THEN** no new epoch is admitted or endpoint published; recovery does not infer file-backed release from this later-gate loss, and higher-level startup handles any abandonment through the exact file-backed release owner; the writable-recovery owner separately supplies fresh authority or its release, retention, or reconciliation result

#### Scenario: Coded-range authority remains separately owned

- **WHEN** the coded-range owner reports an unresolved or refusing coded-range-authority observation at the proposed epoch boundary
- **THEN** recovery consumes and preserves that exact owner observation and its consequences as a separately named prerequisite, creates no new epoch authority, and does not combine it with recovery `CLEAN`, derive a startup-owned input or claim, or reconstruct coded membership

#### Scenario: Recovery CLEAN remains separately owned

- **WHEN** the recovery `CLEAN` owner reports unresolved capture, state, or uncertainty at the proposed epoch boundary
- **THEN** recovery consumes and preserves that exact owner observation and its consequences as a separately named prerequisite, creates no new epoch authority, and does not relabel it, derive a startup-owned input or claim, reconstruct capture membership, frontier, or future-exclusion policy, or claim `CLEAN`

#### Scenario: A mandatory checksum baseline is outstanding

- **WHEN** the checksum owner reports evidence that is absent, partial, invalid, unsupported, stale, duplicated, conflicting, or mismatched in exact target/range, topology epoch, digest profile, checksum-set generation, target content generation, or persistence evidence
- **THEN** epoch admission and baseline writable publication are refused under the checksum owner's canonical contract; a custody-epoch transition does not bypass, replay, or redefine that obligation

#### Scenario: Session begin is durable but epoch admission is rejected

- **WHEN** session begin is durably observed but epoch preconditions are rejected
- **THEN** the durable session-begin fact is preserved separately, the prior epoch remains authoritative, and no new epoch or writable admission is created; recovery does not infer or own file-backed cleanup, and higher-level startup may separately request exact post-acquisition release while lifecycle, recovery, and writable-recovery owners preserve their own close, release, retention, or reconciliation decisions

#### Scenario: The epoch commit observation is uncertain

- **WHEN** an unknown epoch outcome may have become durable after durable session begin and the recovery adapter reports a lost, corrupt, or unclassifiable observation
- **THEN** prior and proposed epoch authority remain unresolved, durable session evidence is preserved, and no writable endpoint or current claim is published; recovery does not infer file-backed release or rollback, and higher-level startup may separately request exact post-acquisition release while writable-recovery authority follows its separate owner

#### Scenario: Restart observes an epoch bound to superseded authority

- **WHEN** restart observes a durably admitted epoch whose bound session or writer/recovery authority is no longer current, even if the epoch commit itself is durable
- **THEN** that epoch remains prior lifecycle evidence only; the recovery and session owners require a new durable session and a new epoch bound to newly current authority before any writable publication

#### Scenario: Range-role basis remains recovery-owned

- **WHEN** accepted current recovery inspection supplies source-owner evidence bound to the exact accepted recovery generation, topology epoch, role, and range
- **THEN** the recovery-state owner produces and durably records the range-role basis snapshot without invoking an operator command path: `current` names the exact active epoch, range, participating generations, and committed durable transition; `prior` names the exact earlier protected generation; `unprotected` records only affirmative source-owner evidence that the range is unprotected; and `indeterminate` or `not-yet-interpretable` ranges remain so

#### Scenario: Range-role basis source evidence is unusable

- **WHEN** current recovery inspection or its source-owner evidence is absent, stale, conflicting, insufficient, or mismatched in recovery generation, topology epoch, role, or range
- **THEN** recovery refuses epoch admission and preserves the owner observation without inferring `current`, `prior`, `unprotected`, or any other basis classification

#### Scenario: Epoch admission does not promote unrelated claims

- **WHEN** a fresh epoch commit is durable but no owner-produced durable range transition or independent integrity, historical-continuity, or release evidence covers a range
- **THEN** no untouched range becomes `Current`; recovery preserves the exact range-role classification without replay, relabeling, or promotion and leaves integrity, correctness, continuity, current protection, and release unclaimed for that range
