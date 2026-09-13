## Context

`ReleaseAuthorizationLedger` retains established exact-generation authorizations per slot index under an explicit per-slot budget. `remember_release_authorization` composes retention. Two service paths consult the same ledger predicate with different consumer closures: the `reserve_if` allow-callback refuses new admissions before coded work using the coded-capture-only outstanding check, while `release_authorization_blocked` and `ensure_release_authorization_available` run at authorization establishment (after basis conformance, before composition) with the full admission, coded-capture, and driver check, preserving the accepted driver through an owner-reconciliation wait instead of failing it. The behavior is implemented and adversarially reviewed, and a bounded Quint relation now states its exact transitions with staged, compact, composition, scenario, and mutant evidence. The canonical requirement composes authorizations but never states this availability contract.

See `proposal.md` for motivation. This design follows the reviewed model boundary and does not replace authorization composition, slot lifecycle, coded capture mechanics, transaction/recovery semantics, or capacity governance.

## Goals / Non-Goals

**Goals:**

- Name the exact availability relation (observe, block, sweep, replace, insert, exhaust, retire) as model-owned finite semantics.
- Keep budgets parametric and consumer observations opaque so checklist/garbage-collection and governor work stays unconstrained.
- Preserve accepted-work waiting (reconciliation-required, not failure) when retention is the only blocker.
- Keep every surrounding owner exactly where it is.

**Non-Goals:**

- No change to ledger code, service composition, budget values, or slot admission.
- No model of authorization composition (seven observations), coded claim removal, capture discharge mechanics, slot lifecycle, transaction/recovery commit, WriteDriver continuation, or physical durability.
- No canonicalization of the analysis, phase-cut, composition, or mutant profiles as a replacement for the model relation.
- No new persistent state, configuration surface, or operator policy.

## Decisions

1. **Delegate the availability subrelation only.** `RetentionBudgetAvailability.qnt` owns exact-token observe, full-and-consumed blocking, lowest-discharged-first retirement inside every remember, occupied replace without sweep, sweep-and-insert into room, failure-with-sweep exhaustion, exact retirement, and verdict identity. Everything else stays canonical.
2. **Receive composition and consumers as opaque inputs.** Established authorizations arrive from the LifecycleRelease owner; outstanding-consumer flags arrive from admission, coded-capture, and driver owners. The model never manufactures either.
3. **Keep budgets parametric.** The per-slot budget is a model constant, never a fixed semantic fact, so capacity governance can unify budgets later without contradicting delegated semantics.
4. **Keep evidence profiles out of delegation.** Staged paths, the LifecycleRelease handoff composition, scenarios, and mutants are correspondence and sensitivity evidence, not product semantics.
5. **Mark authority at currentization only.** The source-local `dwv:req` marker lands on the model module atomically with canonical sync, never before.

## Risks / Trade-offs

- The failure-with-sweep path is unreachable at fixed capacity (exhaustion needs a full consumed bucket, which holds no discharged entries); it is delegated for exactness and future capacity changes, not for current behavior.
- u32 range/overflow and out-of-range index handling stay with Rust bounds checks and unit tests, outside the finite model domain.
- The handoff composition's trusted fixture mapping (operation to slot identity) is evidence scope, not upstream truth.
