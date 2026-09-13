/**
 * Finite analysis profile for delayed and out-of-order OperationSlotTable
 * child delivery after the same operation has returned with a retained
 * service driver. This does not claim concurrent WriteDriver scheduling.
 */
module PortableOperationExecutionFanInAnalysis {
  import PortableOperationExecutionCoreTypes.* from "../../models/quint/PortableOperationExecutionCore"
  import PortableOperationExecution(
    ChildIds = Set("basis-data", "basis-parity"),
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
    ),
    Regions = Set("block-0"),
    Stores = Set("data", "parity"),
    CurrentWriteOperation = { operation: "slot-a", generation: 0 },
    ApplicableGenerations = Set({ operation: "slot-a", generation: 0 }),
    TrustedMediaEffectTerminal = true,
    TrustedRecoveryAuthoritative = true,
    TrustedBasisConsumed = true,
  ).* from "./PortableOperationExecution"

  action analysisInit: bool = all { init }
  action analysisStep: bool = all { step }

  run fanInOutOfOrder =
    init
      .then(submitDriver)
      .then(enterSlotFanIn)
      .then(emitFanInChild("basis-data"))
      .then(acceptFanInChild("basis-data", expectedIdentity("basis-data")))
      .then(emitFanInChild("basis-parity"))
      .then(acceptFanInChild("basis-parity", expectedIdentity("basis-parity")))
      .then(executeFanInChild("basis-parity", expectedIdentity("basis-parity")))
      .then(executeFanInChild("basis-data", expectedIdentity("basis-data")))
      .then(
        deliverFanInChild(
          "basis-parity",
          expectedIdentity("basis-parity"),
          DispositionSuccess,
        )
      )
      .then(
        deliverFanInChild(
          "basis-data",
          expectedIdentity("basis-data"),
          DispositionSuccess,
        )
      )
      .then(
        duplicateFanInChild(
          "basis-parity",
          expectedIdentity("basis-parity"),
          DispositionSuccess,
        )
      )
      .then(confirmTransactionWriteRecoveryRecord)
      .then(attemptTransactionDataParityWrite)
      .then(confirmTransactionDataParityWritesDurable)
      .then(observeTransactionPersistenceEvidence)
      .then(observeTransactionPersistenceEvidence)
      .then(commitTransactionRecoveryClean)
      .then(reconcile(ReconciliationDurable))
      .then(observeMediaEffectTerminal(Operation))
      .then(observeTerminalChildren(Operation))
      .then(recordRequiredReconciliation(Operation))
      .then(observeReclaimable(Operation))
      .then(releaseTransactionRange(Operation))
      .then(observeTransactionSatisfied(Operation))
      .then(observeRecoveryAuthority(Operation))
      .then(observeBasisConsumed(Operation))
      .then(observeReleaseAuthorization(Operation))
      .then(requestCleanup(Operation))
      .then(release)
      .expect(
        CoreReclaimable
          and coreDuplicateSeen("basis-parity")
          and TransactionRangeReleased
          and ReleaseAllowed.contains(Operation)
          and cleanupSucceeded(Operation)
      )
}
