/**
 * Deterministic production correspondence paths for the retained-operation
 * core. The wrapper binds six named service children but delegates only to the
 * imported core relation; driver ordering, release, and recovery owners stay
 * outside this model.
 */
module PortableOperationExecutionCoreConnect {
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
    Operation = { operation: "slot-a", generation: 1 },
    ChildIdentities = Map(
      "basis-data" -> {
        operation: { operation: "slot-a", generation: 1 },
        store: "data",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "block-0",
        kind: ActionBasisData,
      },
      "basis-parity" -> {
        operation: { operation: "slot-a", generation: 1 },
        store: "parity",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "block-0",
        kind: ActionBasisParity,
      },
      "data-write" -> {
        operation: { operation: "slot-a", generation: 1 },
        store: "data",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "block-0",
        kind: ActionDataWrite,
      },
      "parity-write" -> {
        operation: { operation: "slot-a", generation: 1 },
        store: "parity",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "block-0",
        kind: ActionParityWrite,
      },
      "data-flush" -> {
        operation: { operation: "slot-a", generation: 1 },
        store: "data",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "empty",
        kind: ActionDataFlush,
      },
      "parity-flush" -> {
        operation: { operation: "slot-a", generation: 1 },
        store: "parity",
        incarnation: "incarnation-1",
        topology: "topology-4",
        range: "empty",
        kind: ActionParityFlush,
      },
    ),
  ).* from "../../models/quint/PortableOperationExecutionCore"

  var stage: int

  action connectInit: bool = all {
    stage' = 0,
    init,
  }

  action emitBasisData: bool = all {
    stage == 0,
    emitChild("basis-data"),
    stage' = 1,
  }
  action acceptBasisData: bool = all {
    stage == 1,
    acceptChild("basis-data", expectedIdentity("basis-data")),
    stage' = 2,
  }
  action executeBasisData: bool = all {
    stage == 2,
    executeChild("basis-data", expectedIdentity("basis-data")),
    stage' = 3,
  }
  action emitBasisParity: bool = all {
    stage == 3,
    emitChild("basis-parity"),
    stage' = 4,
  }
  action acceptBasisParity: bool = all {
    stage == 4,
    acceptChild("basis-parity", expectedIdentity("basis-parity")),
    stage' = 5,
  }
  action executeBasisParity: bool = all {
    stage == 5,
    executeChild("basis-parity", expectedIdentity("basis-parity")),
    stage' = 6,
  }

  action deliverBasisParity: bool = all {
    stage == 6,
    deliverChild(
      "basis-parity",
      expectedIdentity("basis-parity"),
      DispositionSuccess,
    ),
    stage' = 7,
  }
  action deliverBasisData: bool = all {
    stage == 7,
    deliverChild(
      "basis-data",
      expectedIdentity("basis-data"),
      DispositionSuccess,
    ),
    stage' = 8,
  }
  action emitDataWrite: bool = all {
    stage == 8,
    emitChild("data-write"),
    stage' = 9,
  }
  action acceptDataWrite: bool = all {
    stage == 9,
    acceptChild("data-write", expectedIdentity("data-write")),
    stage' = 10,
  }
  action executeDataWrite: bool = all {
    stage == 10,
    executeChild("data-write", expectedIdentity("data-write")),
    stage' = 11,
  }
  action emitParityWrite: bool = all {
    stage == 11,
    emitChild("parity-write"),
    stage' = 12,
  }
  action acceptParityWrite: bool = all {
    stage == 12,
    acceptChild("parity-write", expectedIdentity("parity-write")),
    stage' = 13,
  }
  action executeParityWrite: bool = all {
    stage == 13,
    executeChild("parity-write", expectedIdentity("parity-write")),
    stage' = 14,
  }
  action deliverParityWrite: bool = all {
    stage == 14,
    deliverChild(
      "parity-write",
      expectedIdentity("parity-write"),
      DispositionSuccess,
    ),
    stage' = 15,
  }
  action duplicateParityWrite: bool = all {
    stage == 15,
    duplicateChild(
      "parity-write",
      expectedIdentity("parity-write"),
      DispositionSuccess,
    ),
    stage' = 16,
  }
  action duplicateParityWriteMismatched: bool = all {
    stage == 15,
    duplicateChild(
      "parity-write",
      expectedIdentity("parity-write"),
      DispositionFailed,
    ),
    stage' = 16,
  }
  action deliverDataWrite: bool = all {
    stage == 16,
    deliverChild(
      "data-write",
      expectedIdentity("data-write"),
      DispositionSuccess,
    ),
    stage' = 17,
  }
  action emitDataFlush: bool = all {
    stage == 17,
    emitChild("data-flush"),
    stage' = 18,
  }
  action acceptDataFlush: bool = all {
    stage == 18,
    acceptChild("data-flush", expectedIdentity("data-flush")),
    stage' = 19,
  }
  action executeDataFlush: bool = all {
    stage == 19,
    executeChild("data-flush", expectedIdentity("data-flush")),
    stage' = 20,
  }
  action emitParityFlush: bool = all {
    stage == 20,
    emitChild("parity-flush"),
    stage' = 21,
  }
  action acceptParityFlush: bool = all {
    stage == 21,
    acceptChild("parity-flush", expectedIdentity("parity-flush")),
    stage' = 22,
  }
  action executeParityFlush: bool = all {
    stage == 22,
    executeChild("parity-flush", expectedIdentity("parity-flush")),
    stage' = 23,
  }
  action deliverParityFlush: bool = all {
    stage == 23,
    deliverChild(
      "parity-flush",
      expectedIdentity("parity-flush"),
      DispositionSuccess,
    ),
    stage' = 24,
  }
  action deliverDataFlush: bool = all {
    stage == 24,
    deliverChild(
      "data-flush",
      expectedIdentity("data-flush"),
      DispositionSuccess,
    ),
    stage' = 25,
  }
  action reconcileDurable: bool = all {
    stage == 25,
    reconcile(ReconciliationDurable),
    stage' = 26,
  }

  action abandonOperation: bool = all {
    stage == 7,
    abandon,
    stage' = 8,
  }
  action abandonDeliverBasisData: bool = all {
    stage == 6,
    deliverChild(
      "basis-data",
      expectedIdentity("basis-data"),
      DispositionSuccess,
    ),
    stage' = 7,
  }
  action abandonDeliverBasisParity: bool = all {
    stage == 8,
    deliverChild(
      "basis-parity",
      expectedIdentity("basis-parity"),
      DispositionSuccess,
    ),
    stage' = 9,
  }
  action abandonReconcileDurable: bool = all {
    stage == 9,
    reconcile(ReconciliationDurable),
    stage' = 10,
  }

  // The model has no invalid transition. This no-op path lets Connect verify
  // that a real mismatched acceptance is rejected without changing state.
  action probeIdentityRejection: bool = all {
    stage == 1,
    state' = state,
    stage' = 2,
  }

  action probeCompletionRejection: bool = all {
    stage == 3,
    state' = state,
    stage' = 4,
  }

  action resultIdentityStep: bool = any {
    emitBasisData,
    acceptBasisData,
    executeBasisData,
    probeCompletionRejection,
  }

  action successStep: bool = any {
    emitBasisData,
    acceptBasisData,
    executeBasisData,
    emitBasisParity,
    acceptBasisParity,
    executeBasisParity,
    deliverBasisParity,
    deliverBasisData,
    emitDataWrite,
    acceptDataWrite,
    executeDataWrite,
    emitParityWrite,
    acceptParityWrite,
    executeParityWrite,
    deliverParityWrite,
    duplicateParityWrite,
    deliverDataWrite,
    emitDataFlush,
    acceptDataFlush,
    executeDataFlush,
    emitParityFlush,
    acceptParityFlush,
    executeParityFlush,
    deliverParityFlush,
    deliverDataFlush,
    reconcileDurable,
  }

  action duplicateDispositionStep: bool = any {
    emitBasisData,
    acceptBasisData,
    executeBasisData,
    emitBasisParity,
    acceptBasisParity,
    executeBasisParity,
    deliverBasisParity,
    deliverBasisData,
    emitDataWrite,
    acceptDataWrite,
    executeDataWrite,
    emitParityWrite,
    acceptParityWrite,
    executeParityWrite,
    deliverParityWrite,
    duplicateParityWriteMismatched,
  }

  action abandonmentStep: bool = any {
    emitBasisData,
    acceptBasisData,
    executeBasisData,
    emitBasisParity,
    acceptBasisParity,
    executeBasisParity,
    abandonOperation,
    abandonDeliverBasisData,
    abandonDeliverBasisParity,
    abandonReconcileDurable,
  }

  action deliverShortBasisData: bool = all {
    stage == 3,
    deliverChild(
      "basis-data",
      expectedIdentity("basis-data"),
      DispositionShort,
    ),
    stage' = 4,
  }
  action deliverFailureBasisData: bool = all {
    stage == 3,
    deliverChild(
      "basis-data",
      expectedIdentity("basis-data"),
      DispositionFailed,
    ),
    stage' = 4,
  }
  action deliverUncertaintyBasisData: bool = all {
    stage == 3,
    deliverChild(
      "basis-data",
      expectedIdentity("basis-data"),
      DispositionUncertain,
    ),
    stage' = 4,
  }

  action deliverFailureBasisDataAfterParity: bool = all {
    stage == 6,
    deliverChild(
      "basis-data",
      expectedIdentity("basis-data"),
      DispositionFailed,
    ),
    stage' = 7,
  }
  action deliverFailureBasisParityAfterFailure: bool = all {
    stage == 7,
    deliverChild(
      "basis-parity",
      expectedIdentity("basis-parity"),
      DispositionFailed,
    ),
    stage' = 8,
  }
  action reconcileAfterFailureSibling: bool = all {
    stage == 8,
    reconcile(ReconciliationDurable),
    stage' = 9,
  }

  action shortStep: bool = any {
    emitBasisData,
    acceptBasisData,
    executeBasisData,
    deliverShortBasisData,
  }
  action failureStep: bool = any {
    emitBasisData,
    acceptBasisData,
    executeBasisData,
    deliverFailureBasisData,
  }
  action failureSiblingStep: bool = any {
    emitBasisData,
    acceptBasisData,
    executeBasisData,
    emitBasisParity,
    acceptBasisParity,
    executeBasisParity,
    deliverFailureBasisDataAfterParity,
    deliverFailureBasisParityAfterFailure,
    reconcileAfterFailureSibling,
  }
  action uncertaintyStep: bool = any {
    emitBasisData,
    acceptBasisData,
    executeBasisData,
    deliverUncertaintyBasisData,
  }
  action identityStep: bool = any {
    emitBasisData,
    probeIdentityRejection,
  }
  run success =
    connectInit
      .then(emitBasisData)
      .then(acceptBasisData)
      .then(executeBasisData)
      .then(emitBasisParity)
      .then(acceptBasisParity)
      .then(executeBasisParity)
      .then(deliverBasisParity)
      .then(deliverBasisData)
      .then(emitDataWrite)
      .then(acceptDataWrite)
      .then(executeDataWrite)
      .then(emitParityWrite)
      .then(acceptParityWrite)
      .then(executeParityWrite)
      .then(deliverParityWrite)
      .then(duplicateParityWrite)
      .then(deliverDataWrite)
      .then(emitDataFlush)
      .then(acceptDataFlush)
      .then(executeDataFlush)
      .then(emitParityFlush)
      .then(acceptParityFlush)
      .then(executeParityFlush)
      .then(deliverParityFlush)
      .then(deliverDataFlush)
      .then(reconcileDurable)
      .expect(
        state.slot == SlotReclaimable
          and state.children.get("parity-write").duplicateSeen
      )

  run duplicateDispositionProbe =
    connectInit
      .then(emitBasisData)
      .then(acceptBasisData)
      .then(executeBasisData)
      .then(emitBasisParity)
      .then(acceptBasisParity)
      .then(executeBasisParity)
      .then(deliverBasisParity)
      .then(deliverBasisData)
      .then(emitDataWrite)
      .then(acceptDataWrite)
      .then(executeDataWrite)
      .then(emitParityWrite)
      .then(acceptParityWrite)
      .then(executeParityWrite)
      .then(deliverParityWrite)
      .then(duplicateParityWriteMismatched)
      .expect(
        state.children.get("parity-write").duplicateSeen
          and state.children.get("parity-write").completion == DispositionSuccess
      )

  run acceptedWorkAfterAbandonment =
    connectInit
      .then(emitBasisData)
      .then(acceptBasisData)
      .then(executeBasisData)
      .then(emitBasisParity)
      .then(acceptBasisParity)
      .then(executeBasisParity)
      .then(abandonDeliverBasisData)
      .then(abandonOperation)
      .then(abandonDeliverBasisParity)
      .then(abandonReconcileDurable)
      .expect(
        state.slot == SlotReclaimable
          and state.acceptedAtAbandon == Set("basis-parity")
      )

  run failureProbe =
    connectInit
      .then(emitBasisData)
      .then(acceptBasisData)
      .then(executeBasisData)
      .then(deliverFailureBasisData)
      .expect(
        state.slot == SlotAwaitingReconciliation
          and state.children.get("basis-data").work == WorkDelivered
          and state.children.get("basis-data").completion == DispositionFailed
      )

  run failurePreservesAcceptedSibling =
    connectInit
      .then(emitBasisData)
      .then(acceptBasisData)
      .then(executeBasisData)
      .then(emitBasisParity)
      .then(acceptBasisParity)
      .then(executeBasisParity)
      .then(deliverFailureBasisDataAfterParity)
      .then(deliverFailureBasisParityAfterFailure)
      .then(reconcileAfterFailureSibling)
      .expect(
        state.slot == SlotReclaimable
          and state.children.get("basis-data").completion == DispositionFailed
          and state.children.get("basis-parity").completion == DispositionFailed
      )

  run shortProbe =
    connectInit
      .then(emitBasisData)
      .then(acceptBasisData)
      .then(executeBasisData)
      .then(deliverShortBasisData)
      .expect(
        state.slot == SlotAwaitingReconciliation
          and state.children.get("basis-data").work == WorkDelivered
          and state.children.get("basis-data").completion == DispositionShort
      )

  run uncertaintyProbe =
    connectInit
      .then(emitBasisData)
      .then(acceptBasisData)
      .then(executeBasisData)
      .then(deliverUncertaintyBasisData)
      .expect(
        state.slot == SlotCompletionUncertain
          and state.children.get("basis-data").work == WorkDelivered
          and state.children.get("basis-data").completion == DispositionUncertain
      )

  run identityProbe =
    connectInit
      .then(emitBasisData)
      .then(probeIdentityRejection)
      .expect(state.children.get("basis-data").work == WorkEmitted)

  run resultIdentityProbe =
    connectInit
      .then(emitBasisData)
      .then(acceptBasisData)
      .then(executeBasisData)
      .then(probeCompletionRejection)
      .expect(state.children.get("basis-data").work == WorkExecuted)
}
