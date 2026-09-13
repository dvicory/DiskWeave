/**
 * Finite analysis profile for the portable operation/service-driver relation.
 * The production Connect profile is deliberately separate so model checking
 * does not depend on Rust execution or a hand-shaped trace.
 */
module PortableOperationExecutionAnalysis {
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

  run driverSuccess =
    init
      .then(submitDriver)
      .then(emitDriverChild("basis-data"))
      .then(acceptDriverChild("basis-data", expectedIdentity("basis-data")))
      .then(executeDriverChild("basis-data", expectedIdentity("basis-data")))
      .then(
        deliverDriverChild(
          "basis-data",
          expectedIdentity("basis-data"),
          DispositionSuccess,
        )
      )
      .then(emitDriverChild("basis-parity"))
      .then(acceptDriverChild("basis-parity", expectedIdentity("basis-parity")))
      .then(executeDriverChild("basis-parity", expectedIdentity("basis-parity")))
      .then(
        deliverDriverChild(
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
          and CoreDurableReconciliation
          and TransactionRangeReleased
          and ReleaseAllowed.contains(Operation)
          and cleanupSucceeded(Operation)
      )
}
