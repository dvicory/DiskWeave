/**
 * Negative evidence for the delegated retained-operation core.
 *
 * Each action injects one defect. Its deterministic witness must violate the
 * named safety predicate, proving that the corresponding oracle is effective.
 */
module PortableOperationExecutionCoreMutants {
  import PortableOperationExecutionCoreTypes.* from "../../models/quint/PortableOperationExecutionCore"
  import PortableOperationExecutionCore(
    ChildIds = Set(
      "basis-data",
      "basis-parity",
      "data-write",
      "parity-write",
      "data-flush",
      "parity-flush",
    ),
    Operation = { operation: "slot-a", generation: 0 },
    ChildIdentities = Map(
      "basis-data" -> {
        operation: { operation: "slot-a", generation: 0 },
        store: "data",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "block-0",
        kind: ActionBasisData,
      },
      "basis-parity" -> {
        operation: { operation: "slot-a", generation: 0 },
        store: "parity",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "block-0",
        kind: ActionBasisParity,
      },
      "data-write" -> {
        operation: { operation: "slot-a", generation: 0 },
        store: "data",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "block-0",
        kind: ActionDataWrite,
      },
      "parity-write" -> {
        operation: { operation: "slot-a", generation: 0 },
        store: "parity",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "block-0",
        kind: ActionParityWrite,
      },
      "data-flush" -> {
        operation: { operation: "slot-a", generation: 0 },
        store: "data",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "empty",
        kind: ActionDataFlush,
      },
      "parity-flush" -> {
        operation: { operation: "slot-a", generation: 0 },
        store: "parity",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "empty",
        kind: ActionParityFlush,
      },
    ),
  ).* from "../../models/quint/PortableOperationExecutionCore"

  val ExactChildIdentity: bool =
    ChildIds.forall(id => state.children.get(id).identity == expectedIdentity(id))

  val NonSuccessRefusesRegisteredSiblings: bool =
    ChildIds.forall(completed => {
      val child = state.children.get(completed)
      child.slotState != SlotChildTerminal
        or child.completion == DispositionSuccess
        or ChildIds.forall(sibling =>
          sibling == completed
            or state.children.get(sibling).slotState != SlotChildRegistered
        )
    })

  action executeBeforeAcceptanceMutant(id: str): bool = all {
    state.children.get(id).slotState == SlotChildRegistered,
    state' = {
      ...state,
      children: state.children.put(id, {
        ...state.children.get(id),
        work: WorkExecuted,
      }),
    },
  }

  action acceptWrongIdentityMutant(id: str): bool = all {
    state' = {
      ...state,
      slot: SlotSubmitted,
      children: state.children.put(id, {
        ...state.children.get(id),
        slotState: SlotChildAccepted,
        work: WorkAccepted,
        identity: {
          ...state.children.get(id).identity,
          store: "wrong-store",
        },
      }),
    },
  }

  action deliverWithoutDispositionMutant(id: str): bool = all {
    state' = {
      ...state,
      slot: SlotPartiallyCompleted,
      children: state.children.put(id, {
        ...state.children.get(id),
        slotState: SlotChildTerminal,
        work: WorkDelivered,
      }),
    },
  }

  action keepRegisteredSiblingAfterFailureMutant(id: str): bool = all {
    state.children.get(id).slotState == SlotChildAccepted,
    state.children.get(id).work == WorkExecuted,
    state' = {
      ...state,
      slot: SlotPartiallyCompleted,
      children: state.children.put(id, {
        ...state.children.get(id),
        slotState: SlotChildTerminal,
        work: WorkDelivered,
        completion: DispositionFailed,
      }),
    },
  }

  action refuseAcceptedChildOnAbandonMutant(id: str): bool = all {
    state.children.get(id).slotState == SlotChildAccepted,
    state' = {
      ...state,
      slot: SlotAwaitingReconciliation,
      acceptedAtAbandon: Set(id),
      abandoned: true,
      children: state.children.put(id, {
        ...state.children.get(id),
        slotState: SlotChildRefused,
        work: WorkRefused,
      }),
    },
  }

  action reclaimWithoutReconciliationMutant: bool = all {
    state' = {
      ...state,
      slot: SlotReclaimable,
    },
  }

  run executionBeforeAcceptanceMutantTest =
    init
      .then(executeBeforeAcceptanceMutant("basis-data"))
      .expect(not(PhysicalExecutionRequiresAcceptance))

  run wrongIdentityAcceptanceMutantTest =
    init
      .then(acceptWrongIdentityMutant("basis-data"))
      .expect(not(ExactChildIdentity))

  run terminalWithoutDispositionMutantTest =
    init
      .then(deliverWithoutDispositionMutant("basis-data"))
      .expect(not(TerminalChildrenCarryDisposition))

  run unacceptedSiblingSurvivesFailureMutantTest =
    init
      .then(emitChild("basis-data"))
      .then(acceptChild("basis-data", expectedIdentity("basis-data")))
      .then(executeChild("basis-data", expectedIdentity("basis-data")))
      .then(keepRegisteredSiblingAfterFailureMutant("basis-data"))
      .expect(not(NonSuccessRefusesRegisteredSiblings))

  run acceptedChildRefusedAtAbandonMutantTest =
    init
      .then(emitChild("basis-data"))
      .then(acceptChild("basis-data", expectedIdentity("basis-data")))
      .then(refuseAcceptedChildOnAbandonMutant("basis-data"))
      .expect(not(AbandonmentPreservesAcceptedChildren))

  run prematureReclaimMutantTest =
    init
      .then(reclaimWithoutReconciliationMutant)
      .expect(not(ReclaimRequiresCompleteReconciliation))
}
