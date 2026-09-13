/**
 * Bounded canonical core for retained-operation child correlation.
 *
 * This relation deliberately has no driver cursor, physical scheduler,
 * transaction marker, lifecycle release state, or post-Reclaimable state.
 * It ends at slot reclamation so those surrounding owners can be checked by
 * a separate correspondence wrapper without becoming model authority here.
 *
 * This core does not own canonical request production, topology/store/
 * incarnation selection, buffers/tags, resources, drain, generation reuse,
 * watermarks, persistence admissibility, physical stores, recovery authority,
 * transaction semantics, ReleaseAllowed, frontend abandonment, shutdown,
 * session, startup, or post-Reclaimable cleanup. Trusted identity inputs do
 * not prove upstream truth. Bounded and Connect traces do not prove
 * arbitrary-width concurrency or liveness; fake or in-memory evidence is not
 * deployed-backend, hardware, or physical-durability certification. Mutants
 * prove sensitivity only to named defects, not model completeness or
 * implementation correctness.
 */
module PortableOperationExecutionCoreTypes {
  type SlotState =
    SlotReserved
    | SlotSubmitted
    | SlotPartiallyCompleted
    | SlotCompletionUncertain
    | SlotAwaitingReconciliation
    | SlotReclaimable
  type SlotChildState =
    SlotChildRegistered
    | SlotChildAccepted
    | SlotChildTerminal
    | SlotChildRefused
  type WorkState =
    WorkPlanned
    | WorkEmitted
    | WorkAccepted
    | WorkExecuted
    | WorkDelivered
    | WorkRefused
  type CompletionDisposition =
    DispositionNone
    | DispositionSuccess
    | DispositionShort
    | DispositionFailed
    | DispositionUncertain
  type ReconciliationState =
    ReconciliationNone
    | ReconciliationDurable
    | ReconciliationUncertainRetained
  type ChildAction =
    ActionBasisData
    | ActionBasisParity
    | ActionDataWrite
    | ActionParityWrite
    | ActionDataFlush
    | ActionParityFlush
    | ActionGeneric
  type OperationGeneration = {
    operation: str,
    generation: int,
  }
  type ChildIdentity = {
    operation: OperationGeneration,
    store: str,
    incarnation: str,
    topology: str,
    range: str,
    kind: ChildAction,
  }
  type Child = {
    slotState: SlotChildState,
    work: WorkState,
    identity: ChildIdentity,
    completion: CompletionDisposition,
    duplicateSeen: bool,
  }
  type CoreState = {
    operation: OperationGeneration,
    slot: SlotState,
    children: str -> Child,
    acceptedAtAbandon: Set[str],
    reconciliation: ReconciliationState,
    abandoned: bool,
  }
}

// dwv:req req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
module PortableOperationExecutionCore {
  import PortableOperationExecutionCoreTypes.*

  const ChildIds: Set[str]
  const Operation: OperationGeneration
  const ChildIdentities: str -> ChildIdentity
  var state: CoreState

  pure def expectedIdentity(id: str): ChildIdentity =
    ChildIdentities.get(id)

  pure def initialChild(id: str): Child = {
    slotState: SlotChildRegistered,
    work: WorkPlanned,
    identity: expectedIdentity(id),
    completion: DispositionNone,
    duplicateSeen: false,
  }

  pure def initialState: CoreState = {
    operation: Operation,
    slot: SlotReserved,
    children: ChildIds.mapBy(id => initialChild(id)),
    acceptedAtAbandon: Set(),
    reconciliation: ReconciliationNone,
    abandoned: false,
  }

  pure def childDone(child: Child): bool =
    child.slotState == SlotChildTerminal or child.slotState == SlotChildRefused

  pure def allDone(s: CoreState): bool =
    ChildIds.forall(id => childDone(s.children.get(id)))

  pure def allDoneAfterDelivery(
    s: CoreState,
    id: str,
    disposition: CompletionDisposition,
  ): bool =
    ChildIds.forall(candidate =>
      if (candidate == id) true
      else if (
        disposition != DispositionSuccess
          and s.children.get(candidate).slotState == SlotChildRegistered
      ) true
      else childDone(s.children.get(candidate))
    )

  pure def allDoneAfterAbandon(s: CoreState): bool =
    ChildIds.forall(id => s.children.get(id).slotState != SlotChildAccepted)

  pure def hasUncertain(s: CoreState): bool =
    ChildIds.exists(id =>
      s.children.get(id).completion == DispositionUncertain
    )

  pure def slotAfterDelivery(
    s: CoreState,
    id: str,
    disposition: CompletionDisposition,
  ): SlotState =
    if (
      allDoneAfterDelivery(s, id, disposition)
        and (disposition == DispositionUncertain or hasUncertain(s))
    ) SlotCompletionUncertain
    else if (allDoneAfterDelivery(s, id, disposition)) SlotAwaitingReconciliation
    else SlotPartiallyCompleted

  pure def slotAfterAbandon(s: CoreState): SlotState =
    if (allDoneAfterAbandon(s) and hasUncertain(s)) SlotCompletionUncertain
    else if (allDoneAfterAbandon(s)) SlotAwaitingReconciliation
    else SlotPartiallyCompleted

  action init: bool = all {
    state' = initialState,
  }

  action emitChild(id: str): bool = all {
    not(state.abandoned),
    state.slot != SlotReclaimable,
    state.children.get(id).slotState == SlotChildRegistered,
    state.children.get(id).work == WorkPlanned,
    state' = {
      ...state,
      children: state.children.put(id, {
        ...state.children.get(id),
        work: WorkEmitted,
      }),
    },
  }

  action acceptChild(id: str, identity: ChildIdentity): bool = all {
    not(state.abandoned),
    state.slot != SlotReclaimable,
    identity == state.children.get(id).identity,
    state.children.get(id).slotState == SlotChildRegistered,
    state.children.get(id).work == WorkEmitted,
    state' = {
      ...state,
      slot: if (state.slot == SlotReserved) SlotSubmitted else state.slot,
      children: state.children.put(id, {
        ...state.children.get(id),
        slotState: SlotChildAccepted,
        work: WorkAccepted,
      }),
    },
  }

  action executeChild(id: str, identity: ChildIdentity): bool = all {
    state.slot != SlotReclaimable,
    identity == state.children.get(id).identity,
    state.children.get(id).slotState == SlotChildAccepted,
    state.children.get(id).work == WorkAccepted,
    state' = {
      ...state,
      children: state.children.put(id, {
        ...state.children.get(id),
        work: WorkExecuted,
      }),
    },
  }

  action deliverChild(
    id: str,
    identity: ChildIdentity,
    disposition: CompletionDisposition,
  ): bool = all {
    state.slot != SlotReclaimable,
    disposition != DispositionNone,
    identity == state.children.get(id).identity,
    state.children.get(id).slotState == SlotChildAccepted,
    state.children.get(id).work == WorkExecuted,
    state' = {
      ...state,
      slot: slotAfterDelivery(state, id, disposition),
      children: ChildIds.mapBy(candidate => {
        if (candidate == id) {
          {
            ...state.children.get(candidate),
            slotState: SlotChildTerminal,
            work: WorkDelivered,
            completion: disposition,
          }
        } else if (
          disposition != DispositionSuccess
            and state.children.get(candidate).slotState == SlotChildRegistered
        ) {
          {
            ...state.children.get(candidate),
            slotState: SlotChildRefused,
            work: WorkRefused,
          }
        } else state.children.get(candidate)
      }),
    },
  }

  action abandon: bool = all {
    not(state.abandoned),
    state.slot != SlotReclaimable,
    state' = {
      ...state,
      slot: slotAfterAbandon(state),
      acceptedAtAbandon: ChildIds.filter(id =>
        state.children.get(id).slotState == SlotChildAccepted
      ),
      abandoned: true,
      children: ChildIds.mapBy(id => {
        val child = state.children.get(id)
        if (child.slotState == SlotChildRegistered) {
          {
            ...child,
            slotState: SlotChildRefused,
            work: WorkRefused,
          }
        } else if (child.work == WorkPlanned or child.work == WorkEmitted) {
          {
            ...child,
            slotState: SlotChildRefused,
            work: WorkRefused,
          }
        } else child
      }),
    },
  }

  // One exact duplicate envelope is retained as a finite idempotence probe.
  action duplicateChild(
    id: str,
    identity: ChildIdentity,
    disposition: CompletionDisposition,
  ): bool = all {
    identity == state.children.get(id).identity,
    state.children.get(id).slotState == SlotChildTerminal,
    state.children.get(id).work == WorkDelivered,
    disposition != DispositionNone,
    not(state.children.get(id).duplicateSeen),
    state' = {
      ...state,
      children: state.children.put(id, {
        ...state.children.get(id),
        duplicateSeen: true,
      }),
    },
  }

  pure def reconciliationState(
    s: CoreState,
    outcome: ReconciliationState,
  ): CoreState =
    if (s.reconciliation == ReconciliationNone) {
      {
        ...s,
        slot: SlotReclaimable,
        reconciliation: outcome,
      }
    } else s

  action reconcile(outcome: ReconciliationState): bool = all {
    outcome == ReconciliationDurable or outcome == ReconciliationUncertainRetained,
    (
      (
        state.reconciliation == ReconciliationNone
          and state.slot != SlotReserved
          and allDone(state)
      )
        or (
          state.slot == SlotReclaimable
            and state.reconciliation == outcome
            and allDone(state)
        )
    ),
    state' = reconciliationState(state, outcome),
  }

  action step: bool = {
    nondet id = ChildIds.oneOf()
    any {
      emitChild(id),
      acceptChild(id, expectedIdentity(id)),
      executeChild(id, expectedIdentity(id)),
      deliverChild(id, expectedIdentity(id), DispositionSuccess),
      deliverChild(id, expectedIdentity(id), DispositionShort),
      deliverChild(id, expectedIdentity(id), DispositionFailed),
      deliverChild(id, expectedIdentity(id), DispositionUncertain),
      abandon,
      duplicateChild(id, expectedIdentity(id), DispositionSuccess),
      duplicateChild(id, expectedIdentity(id), DispositionShort),
      duplicateChild(id, expectedIdentity(id), DispositionFailed),
      duplicateChild(id, expectedIdentity(id), DispositionUncertain),
      reconcile(ReconciliationDurable),
      reconcile(ReconciliationUncertainRetained),
    }
  }

  val BoundedState: bool = all {
    state.children.keys() == ChildIds,
    ChildIdentities.keys() == ChildIds,
    ChildIds.size() > 0,
    state.operation.generation >= 0,
    state.operation.operation != "",
    state.acceptedAtAbandon.subseteq(ChildIds),
    ChildIds.forall(id =>
      state.children.get(id).identity.operation == state.operation
    ),
  }

  val CanonicalChildDomain: bool =
    ChildIds == Set(
      "basis-data",
      "basis-parity",
      "data-write",
      "parity-write",
      "data-flush",
      "parity-flush",
    )

  val PhysicalExecutionRequiresAcceptance: bool =
    ChildIds.forall(id => {
      val child = state.children.get(id)
      child.work != WorkExecuted
        and child.work != WorkDelivered
        or (
          child.slotState == SlotChildAccepted
            or child.slotState == SlotChildTerminal
        )
    })

  val ReclaimRequiresCompleteReconciliation: bool =
    state.slot != SlotReclaimable
      or (state.reconciliation != ReconciliationNone and allDone(state))

  val AbandonmentPreservesAcceptedChildren: bool =
    state.acceptedAtAbandon.subseteq(ChildIds)
      and (
        not(state.abandoned)
        or state.acceptedAtAbandon.forall(id =>
          state.children.get(id).slotState != SlotChildRefused
        )
      )

  val TerminalChildrenCarryDisposition: bool =
    ChildIds.forall(id =>
      state.children.get(id).slotState != SlotChildTerminal
        or state.children.get(id).completion != DispositionNone
    )

}
