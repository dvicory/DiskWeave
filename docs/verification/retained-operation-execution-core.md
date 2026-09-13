# Retained-operation execution core (bounded)

Date: 2026-09-03

Sources:

- `models/quint/PortableOperationExecutionCore.qnt`
- `verification/quint/PortableOperationExecution.qnt`
- `verification/quint/PortableOperationExecutionCoreAnalysis.qnt`
- `verification/quint/PortableOperationExecutionCoreConnect.qnt`
- `verification/quint/PortableOperationExecutionCoreMutants.qnt`
- `verification/quint/PortableOperationExecutionAnalysis.qnt`
- `verification/quint/PortableOperationExecutionFanInAnalysis.qnt`
- `crates/dwv-service/src/service/tests/retained_operation_core_connect.rs`
- `crates/dwv-store/src/lib.rs`

Verified source SHA-256 digests:

- `models/quint/PortableOperationExecutionCore.qnt`:
  `4a633c9a3344d879cf2ddf6df53a4e1349f0f095b53b482cc6d2cc967fa7a70e`
- `verification/quint/PortableOperationExecution.qnt`:
  `f87d15399986500422245cfae58d149bba09b484293134fec037ecba468cd498`
- `verification/quint/PortableOperationExecutionCoreAnalysis.qnt`:
  `f936c243b7fd35cf7c2f6ec3da6ac8794e2133fef7a2f02a8689ae70ec87a8f3`
- `verification/quint/PortableOperationExecutionCoreMutants.qnt`:
  `55be2aafabf96837930d8270f1af713c85fbfa280de9cbb5b2ccde8b432379ef`
- `verification/quint/PortableOperationExecutionCoreConnect.qnt`:
  `8a5db97285d752721cc7889b3da920915b238f882d7fc5d08b79589fe543671c`
- `verification/quint/PortableOperationExecutionAnalysis.qnt`:
  `842646465c859b1fb20e2dd5bdf592d573a722e76acfee9ea73fd6642f4cc08c`
- `verification/quint/PortableOperationExecutionFanInAnalysis.qnt`:
  `0f73a3b502f54a1fcb6a33323d06c9ea5769fd4a30895a085087eac96fe42a3d`

The delegated executable relation is
`PortableOperationExecutionCore.qnt`. It covers the fixed six-child production
identity domain from registered work through `SlotReclaimable`: exact
operation-generation/store/incarnation/topology/range/action correlation,
acceptance before execution, delayed and out-of-order delivery, duplicate
disposition, conservative outcomes including refusal of every not-yet-accepted
sibling after a short, failed, or uncertain completion while accepted siblings
remain owned, one-way abandonment with accepted-work retention, and complete
reconciliation with identical-outcome retry idempotence and outcome-change
rejection. A mismatched completion identity is rejected before slot mutation.
It has no driver cursor, physical scheduler, transaction marker, release
authorization, or post-reclaim state.

`verification/quint/PortableOperationExecution.qnt` is evidence-only
composition. It drives qualified instances of the retained-child
`PortableOperationExecutionCore`, transaction-owner `RecoveryProtocol`, and
release-owner `LifecycleRelease` relations. It retains only the fixed
protected-write cursor, driver phase, and fan-in entry needed by the two-child
correspondence fixtures. Trusted operation, child identity, region/store,
current-write correlation, applicability, recovery, and basis inputs establish
only exact finite-instance correlation; they do not establish upstream truth
or owner authority. The Connect bridge separately maps real
`HealthyPortableService`, retained `WriteDriver`, `OperationSlotTable`, and
checked `StoreCompletion` observations through the core's `Reclaimable`
boundary. It does not claim service release or physical durability.
Recovery progress requires the exact fixture's core children to have completed
successfully, and transaction-range release additionally requires the exact
core generation to be reconciled and `Reclaimable`.

The operation-slot owner preserves a partial aggregate state when a later
child is accepted after an earlier child completed, preserves uncertain
aggregate state when all children become terminal, atomically refuses every
unaccepted sibling after a non-success delivery, and forbids post-abandonment
child registration or acceptance. Accepted children remain owned through
abandonment and reconciliation. The blocking flush facade stops issuing work
after the owner refuses its remaining children. These production transitions
align with the core relation without changing persisted formats.

Checks:

```text
quint typecheck models/quint/PortableOperationExecutionCore.qnt
quint typecheck verification/quint/PortableOperationExecutionCoreAnalysis.qnt
quint typecheck verification/quint/PortableOperationExecutionCoreConnect.qnt
quint typecheck verification/quint/PortableOperationExecution.qnt
quint typecheck verification/quint/PortableOperationExecutionAnalysis.qnt
quint typecheck verification/quint/PortableOperationExecutionFanInAnalysis.qnt
quint typecheck models/quint/RecoveryProtocol.qnt
quint typecheck verification/quint/RecoveryProtocolAnalysis.qnt
quint typecheck verification/quint/RecoveryProtocolConnect.qnt
quint typecheck models/quint/LifecycleRelease.qnt
quint typecheck verification/quint/LifecycleReleaseAnalysis.qnt
quint typecheck verification/quint/LifecycleReleaseMutants.qnt
quint typecheck verification/quint/LifecycleReleaseConnect.qnt
=> all completed with no output

quint test verification/quint/PortableOperationExecutionCoreAnalysis.qnt \
  --main PortableOperationExecutionCoreAnalysis \
  --match '^(boundedDomainTest|delayedOutOfOrderAndReconcile|acceptedWorkSurvivesAbandonment)$'
=> 3 passing

quint test verification/quint/PortableOperationExecutionCoreConnect.qnt \
  --main PortableOperationExecutionCoreConnect \
  --match '^(success|duplicateDispositionProbe|acceptedWorkAfterAbandonment|failureProbe|failurePreservesAcceptedSibling|shortProbe|uncertaintyProbe|identityProbe|resultIdentityProbe)$'
=> 9 passing

quint typecheck verification/quint/PortableOperationExecutionCoreMutants.qnt
quint test verification/quint/PortableOperationExecutionCoreMutants.qnt \
  --main PortableOperationExecutionCoreMutants \
  --match '^(executionBeforeAcceptanceMutantTest|wrongIdentityAcceptanceMutantTest|terminalWithoutDispositionMutantTest|unacceptedSiblingSurvivesFailureMutantTest|acceptedChildRefusedAtAbandonMutantTest|prematureReclaimMutantTest)$' \
  --max-samples 1 --seed 22082026
=> 6 mutation canaries passing

quint verify verification/quint/PortableOperationExecutionCoreAnalysis.qnt \
  --main PortableOperationExecutionCoreAnalysis --init analysisInit \
  --step analysisStep --max-steps 4 --invariants BoundedState \
  CanonicalChildDomain PhysicalExecutionRequiresAcceptance \
  ReclaimRequiresCompleteReconciliation AbandonmentPreservesAcceptedChildren \
  TerminalChildrenCarryDisposition
=> no violation found

quint test verification/quint/PortableOperationExecutionAnalysis.qnt \
  --main PortableOperationExecutionAnalysis --match '^driverSuccess$' \
  --max-samples 1
=> 1 passing
quint test verification/quint/PortableOperationExecutionFanInAnalysis.qnt \
  --main PortableOperationExecutionFanInAnalysis --match '^fanInOutOfOrder$' \
  --max-samples 1
=> 1 passing

quint verify verification/quint/PortableOperationExecutionAnalysis.qnt \
  --main PortableOperationExecutionAnalysis --init analysisInit \
  --step analysisStep --max-steps 4 --invariants BoundedState \
  RecoveryProgressRequiresCoreFixtureSuccess \
  SubmittedSlotHasNoTerminalChildren PhysicalExecutionRequiresAcceptance \
  DriverTransactionPrecedesExecution ReclaimRequiresCompleteReconciliation \
  ReleaseRequiresExplicitOwnerObservations AbandonmentPreservesAcceptedChildren \
  TerminalChildrenCarryDisposition
=> no violation found

quint verify verification/quint/PortableOperationExecutionFanInAnalysis.qnt \
  --main PortableOperationExecutionFanInAnalysis --init analysisInit \
  --step analysisStep --max-steps 4 --invariants BoundedState \
  RecoveryProgressRequiresCoreFixtureSuccess \
  SubmittedSlotHasNoTerminalChildren PhysicalExecutionRequiresAcceptance \
  DriverTransactionPrecedesExecution ReclaimRequiresCompleteReconciliation \
  ReleaseRequiresExplicitOwnerObservations AbandonmentPreservesAcceptedChildren \
  TerminalChildrenCarryDisposition
=> no violation found

cargo test -p dwv-service portable_operation_core_connect -- --nocapture
=> 9 passed
cargo test -p dwv-store accepting_late_child_preserves_partial_slot_state
=> 1 passed
cargo test -p dwv-store accepted_child_survives_abandonment_and_refused_submission_is_distinct
=> 1 passed
cargo test -p dwv-store reconciliation_is_idempotent_for_same_outcome_and_rejects_changes
=> 1 passed

cargo test -p dwv-store --lib -- --test-threads=1
=> 25 passed
cargo test -p dwv-service --all-features -- --test-threads=1
=> 168 passed; 1 ignored
cargo test -p dwv-service --lib lifecycle_release_connect
=> 3 passed

quint test verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis \
  --match '^(completedReleaseRequiredTest|releasePermitsReuseTest)$' \
  --max-samples 1
=> 2 passing
quint test verification/quint/RecoveryProtocolConnect.qnt \
  --main RecoveryProtocolConnect --match '^normalConnectPath$' --max-samples 1
=> 1 passing

quint test verification/quint/LifecycleReleaseAnalysis.qnt \
  --main LifecycleReleaseAnalysis \
  --match '^(canonicalPrefixAuthorizationTest|incompleteFactsWithholdTest|basisConformanceIsRequiredTest|cleanupRequestIsPendingTest|canonicalPrefixCleanupTest)$' \
  --max-samples 1
=> 5 passing
quint test verification/quint/LifecycleReleaseMutants.qnt \
  --main LifecycleReleaseMutants \
  --match '^(missingTransactionMutantTest|omittedBasisEvidenceMutantTest|reclaimableProxyMutantTest|rangeReleasedProxyMutantTest|actionImpliesSuccessMutantTest)$' \
  --max-samples 1
=> 5 passing
```

The canaries inject execution before acceptance, wrong accepted identity,
terminal state without a disposition, failure without refusing unaccepted
siblings, abandonment that refuses accepted work, and reclaim before complete
reconciliation. Each injected defect violates its named safety predicate.

Adversarial mutant review returned `GO` at confidence `0.93`. The reviewer
confirmed that every witness starts from a satisfying state and violates its
named predicate for the injected defect. Duplicate-result immutability and
reconciliation-outcome replacement remain separately exercised by the
Connect probe and focused store regression; dedicated mutants for those two
properties are optional hardening, not part of the declared six-canary claim.


Non-claims:

- The core and composition do not own canonical request production,
  topology/store/incarnation selection, buffers/tags, resources, drain,
  generation reuse, watermarks, persistence admissibility, physical stores,
  recovery authority, transaction semantics, `ReleaseAllowed`, frontend
  abandonment, shutdown, session, startup, or post-`Reclaimable` cleanup.
- Trusted identity inputs do not prove upstream truth.
- Two-child, bounded, and Connect traces do not prove arbitrary-width
  concurrency or liveness. The six-child profile is the delegated production
  identity domain, not proof of every operation shape.
- `FakeStore` and in-memory recovery evidence is not deployed-backend,
  hardware, or physical-durability certification.
- Mutants prove sensitivity only to named defects, not model completeness or
  implementation correctness.

## Affected-owner review

The normal affected-owner query reported the following current requirements.
Each was reviewed individually; no requirement identity or reviewed state was
bulk accepted:

- `req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set`:
  its closed-set, implementation, and evidence ownership is unchanged and does
  not inherit child-correlation authority.
- `req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts`:
  coded-range conflict ownership and its endpoints are unchanged.
- `req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety`:
  abandonment/restart policy remains with this owner and is not inferred from
  the bounded wrapper.
- `req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe`:
  request admission and identity ownership remain upstream of trusted fixtures.
- `req.healthy-portable-io.durable-completion-and-recovery-clean-require-persistence-evidence`:
  persistence and recovery-`CLEAN` admission remain independent owner facts.
- `req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts`:
  `LifecycleRelease` still owns all seven facts, exact applicability,
  monotonic authorization, and later cleanup.
- `req.healthy-portable-io.publication-identity-is-derived-from-admitted-semantics`:
  publication identity remains derived from real admission, not model fixtures.
- `req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity`:
  transaction and parity-write ownership remains unchanged.
- `req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics`:
  normalized kernel-request ownership and adapters are unchanged.
- `req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate`:
  live-platform evidence scope is unchanged by portable model evidence.
- `req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow`:
  Linux publication remains a narrow platform owner.
- `req.operator-recovery.start-composes-admission-and-actual-publication`:
  operator start and publication composition remain unchanged.
- `req.recovery-state-semantics.exact-persistence-evidence-follows-owner-qualified-claim-liveness`:
  persistence-evidence liveness and endpoints remain independently owned.
- `req.recovery-state-semantics.fence-retirement-preserves-an-exact-durable-predecessor`:
  fence-retirement ownership and endpoints remain unchanged.
- `req.recovery-state-semantics.legacy-fence-retention-migrates-explicitly`:
  legacy-fence migration remains independently owned.
- `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`:
  the stable requirement retains the sole delegated core owner and unchanged
  Rust endpoint; only fixture input and evidence composition changed.

`RecoveryProtocol` remains the exact current owner of bounded write lifecycle,
outcomes, ordering, and `releaseRange`. `LifecycleRelease` remains the exact
current owner of applicability, its seven independent owner facts,
exact-generation monotonic `ReleaseAllowed`, and post-authorization cleanup.

The active `define-portable-writable-session-lifecycle` target still owns future
durable session begin/close meaning. `add-portable-shutdown-claim-release`
still owns future shutdown ordering and clean-close composition.
`add-scan-independent-writable-startup` still owns future stabilization,
topology/store claims, epoch admission, and publication. None is current
authority here, and no unsynchronized session, shutdown, or startup semantics
entered this model, implementation, delta, or evidence.

## Currentization checks

```text
openspec validate clarify-portable-operation-model-ownership --strict
=> valid

cargo xtask docs knowledge affected \
 --path models/quint/PortableOperationExecutionCore.qnt
=> review required; 16 dependent requirements reviewed individually

cargo xtask docs check
cargo xtask docs knowledge readiness
=> success; ready=true, 175 requirements, all gate counts zero

cargo xtask docs build
=> success; 175 objects
```
