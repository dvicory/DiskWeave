/**
 * Evidence profiles for the independent retained-operation core relation.
 * The six-child domain is the delegated production domain; scenario traces and
 * any smaller checker profiles remain evidence rather than semantics.
 */
module PortableOperationExecutionCoreAnalysis {
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

  action analysisInit: bool = all { init }
  action analysisStep: bool = all { step }

  run boundedDomainTest = all {
    CanonicalChildDomain,
    ChildIds.size() == 6,
  }

  run delayedOutOfOrderAndReconcile =
    init
      .then(emitChild("basis-data"))
      .then(acceptChild("basis-data", expectedIdentity("basis-data")))
      .then(emitChild("basis-parity"))
      .then(acceptChild("basis-parity", expectedIdentity("basis-parity")))
      .then(executeChild("basis-parity", expectedIdentity("basis-parity")))
      .then(executeChild("basis-data", expectedIdentity("basis-data")))
      .then(
        deliverChild(
          "basis-parity",
          expectedIdentity("basis-parity"),
          DispositionSuccess,
        )
      )
      .then(
        deliverChild(
          "basis-data",
          expectedIdentity("basis-data"),
          DispositionUncertain,
        )
      )
      .then(
        duplicateChild(
          "basis-parity",
          expectedIdentity("basis-parity"),
          DispositionSuccess,
        )
      )
      .then(abandon)
      .then(reconcile(ReconciliationUncertainRetained))
      .expect(
        state.slot == SlotReclaimable
          and state.reconciliation == ReconciliationUncertainRetained
          and state.children.get("basis-parity").duplicateSeen
      )

  run acceptedWorkSurvivesAbandonment =
    init
      .then(emitChild("basis-data"))
      .then(acceptChild("basis-data", expectedIdentity("basis-data")))
      .then(abandon)
      .then(executeChild("basis-data", expectedIdentity("basis-data")))
      .then(
        deliverChild(
          "basis-data",
          expectedIdentity("basis-data"),
          DispositionSuccess,
        )
      )
      .then(reconcile(ReconciliationDurable))
      .then(reconcile(ReconciliationDurable))
      .expect(
        state.slot == SlotReclaimable
          and state.acceptedAtAbandon.contains("basis-data")
          and state.children.get("basis-data").slotState == SlotChildTerminal
      )
}
