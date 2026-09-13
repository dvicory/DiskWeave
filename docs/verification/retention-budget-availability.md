# Retention-budget availability (bounded)

Date: 2026-09-04

Sources:

- `models/quint/RetentionBudgetAvailability.qnt`
- `verification/quint/RetentionBudgetAvailabilityAnalysis.qnt`
- `verification/quint/RetentionBudgetAvailabilityCompact.qnt`
- `verification/quint/RetentionBudgetAvailabilityCompactRefusal.qnt`
- `verification/quint/RetentionBudgetAvailabilityCompactRetirement.qnt`
- `verification/quint/RetentionBudgetAvailabilityRefusalCycle.qnt`
- `verification/quint/RetentionBudgetAvailabilityRetirementCycle.qnt`
- `verification/quint/RetentionBudgetAvailabilityMutants.qnt`
- `verification/quint/RetentionBudgetAvailabilityComposition.qnt`
- `verification/quint/RetentionBudgetAvailabilityConnect.qnt`
- `crates/dwv-service/src/service/tests/retention_budget_availability_connect.rs`
- `crates/dwv-service/src/service/release_authorization.rs`
- `models/quint/RetentionBudgetAvailability.qnt`:
  `84ddfd46b7e496c3b5836c82b94e40b6377c6e3675d6861f44f905866dc495ec`

- `verification/quint/RetentionBudgetAvailabilityAnalysis.qnt`:
  `345e8e5e89892f7d2558925af363fd39d364b25a291be0b2885a12d75900261f`
- `verification/quint/RetentionBudgetAvailabilityCompact.qnt`:
  `0cca2397b537a68fe88417691e45fea6d46c08a7332944982d2ec9029c27c773`
- `verification/quint/RetentionBudgetAvailabilityCompactRefusal.qnt`:
  `6c184c30a969aff57b112ad2f727e2ea7e24d0069526383f50f5912eff961401`
- `verification/quint/RetentionBudgetAvailabilityCompactRetirement.qnt`:
  `a78cdc7f71187f74c95cbd9d8b5079cf259049285d650e3ccb2496725d13aa57`
- `verification/quint/RetentionBudgetAvailabilityRefusalCycle.qnt`:
  `804f8d5e1c1f226a501909ad8d3411ff3bd5b716636b9cff2213cf8b7abc84b5`
- `verification/quint/RetentionBudgetAvailabilityRetirementCycle.qnt`:
  `7d04435caf2e249ec8b75bdbb1ff72980211b3620f3270d05d52c617e40b29a7`
- `verification/quint/RetentionBudgetAvailabilityMutants.qnt`:
  `50bf3493d47dfe1f62ebf03b29987182a1c06d8d8b8ff9e7329726ae9ab80d06`
- `verification/quint/RetentionBudgetAvailabilityComposition.qnt`:
  `af46c0cdce4ebcd7d956a2d6a295a28bdecc02c90d2b764d613f500f64280ff4`
- `verification/quint/RetentionBudgetAvailabilityConnect.qnt`:
  `32471df8907ac74af1051286ad5e31376787cade3a1f9648e5b23741e02994cd`
- `crates/dwv-service/src/service/tests/retention_budget_availability_connect.rs`:
  `97d51c0e0142bc0661509ddf9d1b6db3822b28c57c8ed1c12c48a811a9e6de92`

The delegated executable relation is
`RetentionBudgetAvailability.qnt`. It covers the bounded retention-budget
availability relation over slot-indexed generation sets: exact-token observe,
full-and-consumed blocking with refusal that changes nothing, lowest
discharged-first retirement inside every remember, occupied replace without
sweep, sweep-and-insert into room, failure-with-sweep exhaustion, exact
retirement, and exact verdict identity across six invariants
(`BoundedCapacity`, `ConsumerEntriesRetained`, `RefusalValid`,
`ExhaustionValid`, `AdmissionHonest`, `RetentionValid`). Budgets are
parameters; consumer flags are opaque owner observations. It has no
authorization composition, slot lifecycle, coded capture internals,
authorization payloads, u32 range or index bounds, transaction or recovery
semantics, WriteDriver continuation, physical stores, or governor capacity
policy.

`verification/quint/RetentionBudgetAvailabilityComposition.qnt` is
evidence-only handoff composition. It drives the release-owner
`LifecycleRelease` relation through two exact-generation establishments and
feeds the established authorizations into the budget relation with trusted
fixture identity inputs, proving every retained entry was established and
that an established authorization arriving at a saturated consumed budget
fails clean. Trusted inputs establish only exact finite-instance
correlation; they do not establish upstream truth or owner authority. The
Connect bridge separately maps real `ReleaseAuthorizationLedger` retention,
exact-token observe, blocking probes, discharge sweep, occupied replacement,
exhaustion failure, and exact retirement through staged deterministic paths
with real slot-table tokens, real lifecycle-owner authorizations, and a
fixture outstanding-consumer set. It does not claim slot admission,
authorization composition, or physical durability.

The service `remember` composition retires exactly the lowest discharged
entry at the index before the cleanup path, replaces an occupied entry
without sweeping, and sweeps the rest before inserting into room; failure
still sweeps while refusing the new entry. Refusal before coded admission
leaves existing records and owners untouched. These production transitions
align with the delegated relation without changing persisted formats.

Checks:

```text
quint typecheck models/quint/RetentionBudgetAvailability.qnt
quint typecheck verification/quint/RetentionBudgetAvailabilityAnalysis.qnt
quint typecheck verification/quint/RetentionBudgetAvailabilityCompact.qnt
quint typecheck verification/quint/RetentionBudgetAvailabilityCompactRefusal.qnt
quint typecheck verification/quint/RetentionBudgetAvailabilityCompactRetirement.qnt
quint typecheck verification/quint/RetentionBudgetAvailabilityRefusalCycle.qnt
quint typecheck verification/quint/RetentionBudgetAvailabilityRetirementCycle.qnt
quint typecheck verification/quint/RetentionBudgetAvailabilityMutants.qnt
quint typecheck verification/quint/RetentionBudgetAvailabilityComposition.qnt
quint typecheck verification/quint/RetentionBudgetAvailabilityConnect.qnt
=> all completed with no output

quint test verification/quint/RetentionBudgetAvailabilityAnalysis.qnt \
  --main RetentionBudgetAvailabilityAnalysis \
  --match '^(productionShapeTest|admitWhenFree|eagerDropOnRoom|refuseConsumedSaturation|freeIndexAdmitsDespiteSaturation|dischargeSweepRetains|exhaustedBeforeDischarge|refusalLeavesWholeLedgerUntouched|exhaustionLeavesWholeLedgerUntouched|replaceWithoutGrowth|retireOnlyWhenDischarged|generationReuseAfterSweep)$'
=> 12 passing

quint test verification/quint/RetentionBudgetAvailabilityAnalysis.qnt \
  --main RetentionBudgetAvailabilityOccupiedSweep \
  --match '^(occupiedKeepsOtherDischarged|retireAllOrphansConsumersMutantTest)$'
=> 2 passing

quint test verification/quint/RetentionBudgetAvailabilityMutants.qnt \
  --main RetentionBudgetAvailabilityMutants --match 'MutantTest'
=> 4 passing

quint test verification/quint/RetentionBudgetAvailabilityRefusalCycle.qnt \
  --main RetentionBudgetAvailabilityRefusalCycle \
  --match '^(refusalShapeTest|stagedRefusalPath)$'
quint test verification/quint/RetentionBudgetAvailabilityRefusalCycle.qnt \
  --main RetentionBudgetAvailabilityRefusalCrossTraffic \
  --match '^stagedRefusalCrossTraffic$'
quint test verification/quint/RetentionBudgetAvailabilityRetirementCycle.qnt \
  --main RetentionBudgetAvailabilityRetirementCycle \
  --match '^(retirementShapeTest|stagedRetirementPath)$'
quint test verification/quint/RetentionBudgetAvailabilityRetirementCycle.qnt \
  --main RetentionBudgetAvailabilityRetirementIsolation \
  --match '^stagedRetirementIsolation$'
quint test verification/quint/RetentionBudgetAvailabilityComposition.qnt \
  --main RetentionBudgetAvailabilityComposition \
  --match '^(compositionShapeTest|establishedThenRetainedThenExhausted)$'
=> all passing

quint verify verification/quint/RetentionBudgetAvailabilityRefusalCycle.qnt \
  --main RetentionBudgetAvailabilityRefusalCycle --init refusalInit \
  --step refusalStep --max-steps 7 --invariant AvailabilityInvariants
quint verify verification/quint/RetentionBudgetAvailabilityRefusalCycle.qnt \
  --main RetentionBudgetAvailabilityRefusalCrossTraffic --init refusalCrossInit \
  --step refusalCrossStep --max-steps 7 --invariant AvailabilityInvariants
quint verify verification/quint/RetentionBudgetAvailabilityRetirementCycle.qnt \
  --main RetentionBudgetAvailabilityRetirementCycle --init retirementInit \
  --step retirementStep --max-steps 7 --invariant AvailabilityInvariants
quint verify verification/quint/RetentionBudgetAvailabilityRetirementCycle.qnt \
  --main RetentionBudgetAvailabilityRetirementIsolation --init retirementIsoInit \
  --step retirementIsoStep --max-steps 8 --invariant AvailabilityInvariants
quint verify verification/quint/RetentionBudgetAvailabilityAnalysis.qnt \
  --main RetentionBudgetAvailabilityOccupiedSweep --init occupiedInit \
  --step occupiedStep --max-steps 9 --invariant AvailabilityInvariants
quint verify verification/quint/RetentionBudgetAvailabilityComposition.qnt \
  --main RetentionBudgetAvailabilityComposition --init compositionInit \
  --step compositionStep --max-steps 22 \
  --invariant BudgetInvariants,EveryRetainedWasEstablished
=> no violation found

quint verify verification/quint/RetentionBudgetAvailabilityCompact.qnt \
  --main RetentionBudgetAvailabilityCompact --init compactInit \
  --step compactStep --max-steps 4 --invariant AvailabilityInvariants
=> five-invariant set: no violation found; six-invariant set (with
RetentionValid): not completed on the verification box (swap-starved;
recorded open in the change tasks, carried by the staged, sampled,
scenario, mutant, and single-invariant evidence)

quint run verification/quint/RetentionBudgetAvailabilityAnalysis.qnt \
  --main RetentionBudgetAvailabilityAnalysis --init analysisInit \
  --step analysisStep --max-steps 15 --max-samples 1000 \
  --invariant AvailabilityInvariants --seed 42
=> no violation found

cargo test -p dwv-service --all-features --lib retention_
=> 8 passed (seven Connect paths plus the exact-retention ledger unit test)

cargo test --workspace --all-features -- --test-threads=1
=> 605 passed, 0 failed, 1 ignored across 42 test targets
```

The canaries inject sweep dropping a consumed entry, refusal despite space,
retirement with an outstanding consumer, token-blind refusal, and whole
bucket retirement orphaning surviving consumer flags. Each injected defect
violates its named safety predicate.

Adversarial model review returned `GO` at confidence `0.96` after nine
repaired findings (room-sweep eager drop, occupied single-drop rule,
failure-with-sweep exhaustion, whole-ledger purity scenarios,
verdict-identity pins, `RetentionValid` invariant, retire-all canary,
outcome-stutter justification, raw-retain scope edge). Adversarial
delegation review returned `GO` at confidence `0.97` after prose-reduction
repairs (waits and timing outside the delegated surface, service-owned
retention paragraph with pointer, three service-level scenarios, honest
task evidence). Adversarial Connect review returned `GO` at confidence
`0.97` after the two-step remember mirror and occupied-path repair.

Non-claims:

- The relation and handoff composition do not own authorization
  composition, budget values, slot-table occupancy, slot lifecycle, coded
  capture mechanics, transaction or recovery semantics, WriteDriver
  continuation, physical stores, governor capacity, u32 range or index
  bounds, or authorization payloads.
- Trusted fixture inputs (operation identity, consumer flags, probe token
  values) do not prove upstream truth.
- Bounded, staged, and Connect traces do not prove arbitrary-width
  concurrency or liveness. The two-slot profiles are the checked finite
  domains, not proof of every slot count, budget, or generation shape.
- Real slot-table and lifecycle-owner fixtures are not deployed-backend,
  hardware, or physical-durability certification.
- Mutants prove sensitivity only to named defects, not model completeness or
  implementation correctness.

## Affected-owner review

The pre-marker affected-owner query for the model file reports no
relationships (`review_required: false`); the source-local delegation
marker lands only at atomic currentization. The owning requirement and its
directly composed owners were reviewed individually through the delegation
review instead; no requirement identity or reviewed state was bulk
accepted:

- `req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts`:
  gains only the bounded availability delegation, service-owned retention
  paragraph, and three service-level scenarios; composition surface,
  seven observations, and all eight prior scenarios are unchanged.
- `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`:
  slot lifecycle, child outcomes, reconciliation, `Reclaimable`, and
  generation reuse remain canonical; the availability relation consumes
  exact tokens without redefining them.
- `req.explicit-transaction-machine.*`: transaction and release-relation
  ownership is unchanged and is not consulted by the availability relation.
- `req.recovery-state-semantics.*`: commit observations and reconciliation
  ownership are unchanged.
- Coded-capture owners (`CodedRangeClean` delegation): claim removal
  consumes established authorizations; capture discharge mechanics remain
  coded-owned and enter the relation only as opaque consumer flags.
- File-backed, Linux frontend, operator, dirty-integrity, checksum, and
  inference owners: untouched; no retention, claim, or evidence semantics
  move.

`LifecycleRelease` remains the exact current owner of applicability, its
seven independent owner facts, exact-generation monotonic `ReleaseAllowed`,
and post-authorization cleanup. `PortableOperationExecutionCore` remains
the exact current owner of retained child correlation through slot
reclamation.

The active `define-portable-writable-session-lifecycle` target still owns
future durable session begin/close meaning.
`add-portable-shutdown-claim-release` still owns future shutdown ordering
and clean-close composition. `add-scan-independent-writable-startup`
still owns future stabilization, topology/store claims, epoch admission,
and publication. None is current authority here, and no unsynchronized
session, shutdown, or startup semantics entered this model,
implementation, delta, or evidence.

Post-synchronization dependent review (each reviewed individually against
its consumed prerequisite facts; the sync adds only the availability
paragraph, delegation clause, and three service-level scenarios while
every prior composition sentence and scenario stays verbatim):

- `req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set`:
  consumes monotonic `ReleaseAllowed` and unresolved-`CLEAN` independence;
  both unchanged; retention budgets touch neither capture roots nor close
  composition.
- `req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts`:
  consumes established authorization downstream of claim removal;
  composition and the seven facts unchanged; retention does not alter coded
  policy or capture membership.
- `req.healthy-portable-io.durable-completion-and-recovery-clean-require-persistence-evidence`:
  persistence and recovery-`CLEAN` admission stay independent owner facts;
  implementation disposition stays deferred with no new endpoint claimed.
- `req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts`:
  the synced requirement itself; delegation reviewed `GO` with bounded
  budgets parametric and consumers opaque.
- `req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity`:
  transaction and parity-write path consumes authorization semantics;
  unchanged; budget retention alters neither write admission nor parity
  geometry.
- `req.recovery-state-semantics.exact-persistence-evidence-follows-owner-qualified-claim-liveness`:
  owner-qualified claim liveness stays independent; unchanged.
- `req.recovery-state-semantics.fence-retirement-preserves-an-exact-durable-predecessor`:
  predecessor durability stays independent; unchanged.
- `req.recovery-state-semantics.legacy-fence-retention-migrates-explicitly`:
  explicit legacy conversion stays independent; unchanged.

## Currentization checks

```text
openspec validate --strict --all
=> 33 passed, 0 failed

cargo test -p dwv-service --all-features --lib retention_
=> 8 passed

cargo xtask docs check
=> success; ready: true, 175 requirements, no diagnostics or review-required entries

cargo xtask docs knowledge readiness
=> success; ready: true, 175 requirements

cargo xtask docs build
=> success; 175 objects
```
