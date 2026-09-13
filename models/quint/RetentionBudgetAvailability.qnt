/**
 * Bounded availability relation for retention-budget admission.
 *
 * Owns exactly the ledger-level availability transitions of
 * `ReleaseAuthorizationLedger` as composed by the service retention path:
 * exact-token observe, full-and-consumed blocking, cleanup-sweep retention,
 * exact retirement, and admission refusal that changes nothing.
 *
 * State is shaped slot-keyed (slot index maps to retained generation sets)
 * so the bounded checker encodes sets of integers rather than quantified
 * record maps. This is an isomorphism on the modeled domain, not an
 * abstraction: OperationSlotToken is exactly (index, generation) with
 * derived ordering, so per-index token maps correspond 1:1 with per-slot
 * generation sets and exact-token lookup is exact (slot, generation)
 * membership. Out of scope by construction: u32 range/overflow behavior
 * and out-of-range index handling, which stay with Rust bounds checks and
 * unit tests.
 *
 * Correspondence map (symbols only, never lines):
 * - retained / consumers / isBlocked <-> ReleaseAuthorizationLedger
 *   (observe, is_full_and_consumed: exact-key lookup only; the
 *   authorization payload behind a key is outside this relation)
 * - retainAuth <-> retain_with_cleanup as called by
 *   remember_release_authorization: every remember first retires exactly
 *   the lowest discharged entry at the index, then replaces an occupied
 *   entry with no sweep or sweeps the rest and inserts into room
 * - retireOp <-> ReleaseAuthorizationLedger retire and the
 *   recovery-driven retire paths
 * - requestAuth admit/refuse <-> the shared is_full_and_consumed predicate
 *   at both service compositions: the reserve_if allow-callback before
 *   coded work (coded-capture-only consumer flags) and
 *   ensure_release_authorization_available /
 *   release_authorization_blocked at authorization establishment (full
 *   admission, coded-capture, and driver flags); slot-table occupancy
 *   stays with OperationAdmission::reserve_if and is outside this relation
 * - Authorization composition (seven observations), coded-capture consumer
 *   mechanics, slot lifecycle, transaction/recovery, WriteDriver
 *   continuation, physical stores, and governor capacity policy are opaque
 *   inputs or non-claims; budgets are parameters, not fixed semantic facts.
 *   Raw retain without cleanup is ledger-internal (its only production
 *   caller is retain_with_cleanup; the rest is unit tests), so only the
 *   cleanup-composed behavior is modeled.
 *
 * Non-claims: no authorization-composition, coded, slot-lifecycle,
 * transaction, recovery, driver, physical-durability, or governor semantics;
 * no liveness or arbitrary-width concurrency; sampled and bounded traces do
 * not prove implementation correctness; mutant witnesses prove oracle
 * sensitivity only. Delegated by the reviewed retention-budget availability
 * change; the marker below names the owning requirement.
 */
module RetentionBudgetAvailabilityTypes {
  type Operation = { slot: int, generation: int }
  type RequestOutcome =
    OutcomeNone
    | OutcomeAdmitted
    | OutcomeRefused
    | OutcomeRetained
    | OutcomeExhausted
  type OutcomeRef =
    NoOutcome
    | ForOperation(Operation)
  type BudgetState = {
    retained: int -> Set[int],
    consumers: int -> Set[int],
    outcome: RequestOutcome,
    outcomeOp: OutcomeRef,
  }
}

// dwv:req req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts
module RetentionBudgetAvailability {
  import RetentionBudgetAvailabilityTypes.*

  const SlotIndices: Set[int]
  const Generations: Set[int]
  const Operations: Set[Operation]
  const CapacityPerSlot: int
  var state: BudgetState

  pure def emptyGenSet: Set[int] = Generations.filter(g => g != g)

  pure def bucketHas(s: BudgetState, op: Operation): bool =
    s.retained.get(op.slot).contains(op.generation)

  pure def bucketSize(s: BudgetState, slot: int): int =
    s.retained.get(slot).size()

  pure def hasConsumer(s: BudgetState, op: Operation): bool =
    s.consumers.get(op.slot).contains(op.generation)

  pure def bucketAllConsumed(s: BudgetState, slot: int): bool =
    s.retained.get(slot).forall(g =>
      hasConsumer(s, { slot: slot, generation: g }))

  pure def isBlocked(s: BudgetState, op: Operation): bool =
    not(bucketHas(s, op))
      and bucketSize(s, op.slot) > 0
      and bucketSize(s, op.slot) >= CapacityPerSlot
      and bucketAllConsumed(s, op.slot)

  // remember first retires exactly the lowest discharged entry at the index
  // (BTreeMap order), then either replaces an occupied entry with no sweep
  // or sweeps the rest and inserts into room. An occupied incoming entry
  // keeps every other entry except that one drop; a vacant one sweeps all
  // remaining discharged entries before the bounded recheck.
  pure def droppedGen(s: BudgetState, op: Operation): Set[int] =
    s.retained.get(op.slot).filter(g =>
      not(hasConsumer(s, { slot: op.slot, generation: g }))
      and s.retained.get(op.slot).forall(h =>
        hasConsumer(s, { slot: op.slot, generation: h }) or h >= g))

  pure def postDropBucket(s: BudgetState, op: Operation): Set[int] =
    s.retained.get(op.slot).filter(g => not(droppedGen(s, op).contains(g)))

  pure def sweptPostDrop(s: BudgetState, op: Operation): Set[int] =
    postDropBucket(s, op).filter(g =>
      hasConsumer(s, { slot: op.slot, generation: g }))

  pure def retainAdmits(s: BudgetState, op: Operation): bool =
    postDropBucket(s, op).contains(op.generation)
      or sweptPostDrop(s, op).size() < CapacityPerSlot

  // The cleanup sweep runs before the bounded recheck even when the recheck
  // fails, so exhaustion still drops discharged entries while refusing the
  // new one. Unobservable at fixed capacity (failure needs a full consumed
  // bucket, which holds no discharged entries), but exact for capacity
  // changes and costless to state.
  pure def retainedAfterRetain(s: BudgetState, op: Operation): int -> Set[int] =
    if (postDropBucket(s, op).contains(op.generation))
      s.retained.put(op.slot, postDropBucket(s, op))
    else if (sweptPostDrop(s, op).size() < CapacityPerSlot)
      s.retained.put(op.slot, sweptPostDrop(s, op).union(Set(op.generation)))
    else s.retained.put(op.slot, sweptPostDrop(s, op))
  action init: bool = all {
    state' = {
      retained: SlotIndices.mapBy(s => emptyGenSet),
      consumers: SlotIndices.mapBy(s => emptyGenSet),
      outcome: OutcomeNone,
      outcomeOp: NoOutcome,
    },
  }

  // Availability probe only: the slot-table occupancy half of admission stays
  // outside; refusal changes nothing.
  action requestAuth(op: Operation): bool = all {
    Operations.contains(op),
    state' = {
      ...state,
      outcome: if (isBlocked(state, op)) OutcomeRefused else OutcomeAdmitted,
      outcomeOp: ForOperation(op),
    },
  }
  // remember_release_authorization as one atomic transition: every call
  // drops the lowest discharged entry at the index, then replaces an
  // occupied entry with no sweep or sweeps the rest and inserts into room;
  // exhaustion still sweeps while refusing the new entry.
  action retainAuth(op: Operation): bool = all {
    Operations.contains(op),
    state' = {
      ...state,
      retained: retainedAfterRetain(state, op),
      outcome: if (retainAdmits(state, op)) OutcomeRetained else OutcomeExhausted,
      outcomeOp: ForOperation(op),
    },
  }

  action retireOp(op: Operation): bool = all {
    Operations.contains(op),
    bucketHas(state, op),
    not(hasConsumer(state, op)),
    state' = {
      ...state,
      retained: state.retained.put(
        op.slot,
        state.retained.get(op.slot).filter(g => g != op.generation)),
      outcome: OutcomeNone,
      outcomeOp: NoOutcome,
    },
  }

  // Consumer arrivals reference established authorizations; discharge is the
  // only path that clears a consumer flag.
  action setConsumer(op: Operation): bool = all {
    Operations.contains(op),
    bucketHas(state, op),
    state' = {
      ...state,
      consumers: state.consumers.put(
        op.slot,
        state.consumers.get(op.slot).union(Set(op.generation))),
      outcome: OutcomeNone,
      outcomeOp: NoOutcome,
    },
  }

  action clearConsumer(op: Operation): bool = all {
    Operations.contains(op),
    state' = {
      ...state,
      consumers: state.consumers.put(
        op.slot,
        state.consumers.get(op.slot).filter(g => g != op.generation)),
      outcome: OutcomeNone,
      outcomeOp: NoOutcome,
    },
  }

  action step: bool = {
    nondet op = Operations.oneOf()
    any {
      requestAuth(op),
      retainAuth(op),
      retireOp(op),
      setConsumer(op),
      clearConsumer(op),
    }
  }

  val BoundedCapacity: bool =
    SlotIndices.forall(slot => bucketSize(state, slot) <= CapacityPerSlot)

  val ConsumerEntriesRetained: bool =
    Operations.forall(op => not(hasConsumer(state, op)) or bucketHas(state, op))

  val RefusalValid: bool = state.outcome != OutcomeRefused or match state.outcomeOp {
    | NoOutcome => false
    | ForOperation(op) => isBlocked(state, op)
  }

  val ExhaustionValid: bool = state.outcome != OutcomeExhausted or match state.outcomeOp {
    | NoOutcome => false
    | ForOperation(op) =>
      not(bucketHas(state, op))
        and bucketSize(state, op.slot) >= CapacityPerSlot
        and bucketAllConsumed(state, op.slot)
  }

  val AdmissionHonest: bool = state.outcome != OutcomeAdmitted or match state.outcomeOp {
    | NoOutcome => false
    | ForOperation(op) => not(isBlocked(state, op))
  }

  val RetentionValid: bool = state.outcome != OutcomeRetained or match state.outcomeOp {
    | NoOutcome => false
    | ForOperation(op) => bucketHas(state, op)
  }

  val AvailabilityInvariants: bool = all {
    BoundedCapacity,
    ConsumerEntriesRetained,
    RefusalValid,
    ExhaustionValid,
    AdmissionHonest,
    RetentionValid,
  }
}
