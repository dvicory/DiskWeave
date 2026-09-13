/**
 * Evidence-only composition for one bounded protected-write correspondence.
 *
 * The qualified core, recovery, and release modules remain the owners of their
 * respective state machines. This wrapper retains only a fixed driver cursor
 * and fan-in mode. Trusted identity inputs establish correlation for this
 * finite instance; they do not prove upstream truth.
 *
 * This composition does not own canonical request production, topology/store/
 * incarnation selection, buffers/tags, resources, drain, generation reuse,
 * watermarks, persistence admissibility, physical stores, recovery authority,
 * transaction semantics, ReleaseAllowed, frontend abandonment, shutdown,
 * session, startup, or post-Reclaimable cleanup. Its two-child bounded and
 * Connect traces do not prove arbitrary-width concurrency or liveness; fake
 * or in-memory evidence is not deployed-backend, hardware, or
 * physical-durability certification. Mutants prove sensitivity only to named
 * defects, not model completeness or implementation correctness.
 */
module PortableOperationExecution {
  import PortableOperationExecutionCoreTypes.* from "../../models/quint/PortableOperationExecutionCore"
  import PortableOperationExecutionCore(
    ChildIds = ChildIds,
    Operation = Operation,
    ChildIdentities = ChildIdentities,
  ) as core from "../../models/quint/PortableOperationExecutionCore"
  import RecoveryProtocol(
    Regions = Regions,
    Stores = Stores,
  ) as recovery from "../../models/quint/RecoveryProtocol"
  import LifecycleRelease(
    OperationGenerations = Set(Operation),
    ApplicableGenerations = ApplicableGenerations,
  ) as release from "../../models/quint/LifecycleRelease"


  const ChildIds: Set[str]
  const Operation: OperationGeneration
  const ChildIdentities: str -> ChildIdentity
  const Regions: Set[str]
  const Stores: Set[str]
  const CurrentWriteOperation: OperationGeneration
  const ApplicableGenerations: Set[OperationGeneration]
  const TrustedMediaEffectTerminal: bool
  const TrustedRecoveryAuthoritative: bool
  const TrustedBasisConsumed: bool

  type ExecutionMode = DriverMode | SlotFanInMode
  type DriverPhase =
    DriverUnretained
    | DriverRetained
    | DriverRunning
    | DriverFailed
    | DriverAbandoned

  var mode: ExecutionMode
  var driver: DriverPhase
  var cursor: int

  pure def expectedChild(position: int): str =
    if (position == 0) "basis-data"
    else if (position == 1) "basis-parity"
    else if (position == 2) "data-write"
    else if (position == 3) "parity-write"
    else if (position == 4) "data-flush"
    else "parity-flush"

  pure def expectedIdentity(id: str): ChildIdentity =
    ChildIdentities.get(id)

  pure def coreFixtureChildrenSucceeded: bool = all {
    core::state.operation == Operation,
    ChildIds.forall(id => {
      val child = core::state.children.get(id)
      child.slotState == SlotChildTerminal
        and child.completion == DispositionSuccess
    }),
  }

  action init: bool = all {
    core::init,
    recovery::init,
    release::init,
    mode' = DriverMode,
    driver' = DriverUnretained,
    cursor' = 0,
  }

  action submitDriver: bool = all {
    mode == DriverMode,
    driver == DriverUnretained,
    core::state' = core::state,
    recovery::startWrite,
    release::enterApplicable(Operation),
    mode' = mode,
    driver' = DriverRetained,
    cursor' = 0,
  }

  action enterSlotFanIn: bool = all {
    mode == DriverMode,
    driver == DriverRetained,
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = SlotFanInMode,
    driver' = driver,
    cursor' = cursor,
  }

  action emitDriverChild(id: str): bool = all {
    mode == DriverMode,
    driver == DriverRetained,
    cursor < ChildIds.size(),
    id == expectedChild(cursor),
    core::emitChild(id),
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = DriverRunning,
    cursor' = cursor,
  }

  action acceptDriverChild(id: str, identity: ChildIdentity): bool = all {
    mode == DriverMode,
    driver == DriverRunning,
    core::acceptChild(id, identity),
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action executeDriverChild(id: str, identity: ChildIdentity): bool = all {
    mode == DriverMode,
    driver == DriverRunning or driver == DriverAbandoned,
    core::executeChild(id, identity),
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action deliverDriverChild(
    id: str,
    identity: ChildIdentity,
    disposition: CompletionDisposition,
  ): bool = all {
    mode == DriverMode,
    driver == DriverRunning or driver == DriverFailed or driver == DriverAbandoned,
    core::deliverChild(id, identity, disposition),
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = if (driver == DriverAbandoned) DriverAbandoned
      else if (disposition == DispositionSuccess) DriverRetained
      else DriverFailed,
    cursor' = cursor + 1,
  }

  action abandonDriver: bool = all {
    mode == DriverMode,
    driver == DriverRetained or driver == DriverRunning or driver == DriverFailed,
    core::abandon,
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = DriverAbandoned,
    cursor' = cursor,
  }

  action emitFanInChild(id: str): bool = all {
    mode == SlotFanInMode,
    core::emitChild(id),
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action acceptFanInChild(id: str, identity: ChildIdentity): bool = all {
    mode == SlotFanInMode,
    core::acceptChild(id, identity),
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action executeFanInChild(id: str, identity: ChildIdentity): bool = all {
    mode == SlotFanInMode,
    core::executeChild(id, identity),
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action deliverFanInChild(
    id: str,
    identity: ChildIdentity,
    disposition: CompletionDisposition,
  ): bool = all {
    mode == SlotFanInMode,
    core::deliverChild(id, identity, disposition),
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action duplicateFanInChild(
    id: str,
    identity: ChildIdentity,
    disposition: CompletionDisposition,
  ): bool = all {
    mode == SlotFanInMode,
    core::state.children.get(id).completion == disposition,
    core::duplicateChild(id, identity, disposition),
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action reconcile(outcome: ReconciliationState): bool = all {
    core::reconcile(outcome),
    recovery::state' = recovery::state,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action confirmTransactionWriteRecoveryRecord: bool = all {
    coreFixtureChildrenSucceeded,
    core::state' = core::state,
    recovery::confirmWriteRecoveryRecordDurable,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action attemptTransactionDataParityWrite: bool = all {
    coreFixtureChildrenSucceeded,
    core::state' = core::state,
    recovery::attemptDataParityWrite,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action confirmTransactionDataParityWritesDurable: bool = all {
    coreFixtureChildrenSucceeded,
    core::state' = core::state,
    recovery::confirmDataParityWritesDurable,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action observeTransactionPersistenceEvidence: bool = all {
    coreFixtureChildrenSucceeded,
    core::state' = core::state,
    recovery::observePersistenceEvidence,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action commitTransactionRecoveryClean: bool = all {
    coreFixtureChildrenSucceeded,
    core::state' = core::state,
    recovery::commitRecoveryClean,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action observeMediaEffectTerminal(operation: OperationGeneration): bool = all {
    operation == Operation,
    TrustedMediaEffectTerminal,
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::observeMediaTerminal(operation),
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action observeTerminalChildren(operation: OperationGeneration): bool = all {
    operation == core::state.operation,
    core::allDone(core::state),
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::observeChildrenTerminal(operation),
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action recordRequiredReconciliation(operation: OperationGeneration): bool = all {
    operation == core::state.operation,
    core::state.reconciliation != ReconciliationNone,
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::recordRequiredReconciliation(operation),
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action observeReclaimable(operation: OperationGeneration): bool = all {
    operation == core::state.operation,
    core::state.slot == SlotReclaimable,
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::observeReclaimable(operation),
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action releaseTransactionRange(operation: OperationGeneration): bool = all {
    operation == core::state.operation,
    core::state.slot == SlotReclaimable,
    core::state.reconciliation != ReconciliationNone,
    CurrentWriteOperation == operation,
    core::state' = core::state,
    recovery::releaseRange,
    release::state' = release::state,
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action observeTransactionSatisfied(operation: OperationGeneration): bool = all {
    operation == Operation,
    CurrentWriteOperation == operation,
    recovery::state.rangeReleased,
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::observeTransactionSatisfied(operation),
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action observeRecoveryAuthority(operation: OperationGeneration): bool = all {
    operation == Operation,
    TrustedRecoveryAuthoritative,
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::observeRecoveryAuthoritative(operation),
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action observeBasisConsumed(operation: OperationGeneration): bool = all {
    operation == Operation,
    TrustedBasisConsumed,
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::observeBasisConformance(operation, release::BasisConsumed),
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action observeReleaseAuthorization(operation: OperationGeneration): bool = all {
    operation == Operation,
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::establishReleaseAllowed(operation),
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action requestCleanup(operation: OperationGeneration): bool = all {
    operation == Operation,
    release::ReleaseAllowed.contains(operation),
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::requestCleanup(operation),
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action release: bool = all {
    release::ReleaseAllowed.contains(Operation),
    core::state' = core::state,
    recovery::state' = recovery::state,
    release::observeCleanupSucceeded(Operation),
    mode' = mode,
    driver' = driver,
    cursor' = cursor,
  }

  action step: bool = {
    nondet id = ChildIds.oneOf()
    any {
      submitDriver,
      enterSlotFanIn,
      emitDriverChild(id),
      acceptDriverChild(id, expectedIdentity(id)),
      executeDriverChild(id, expectedIdentity(id)),
      deliverDriverChild(id, expectedIdentity(id), DispositionSuccess),
      deliverDriverChild(id, expectedIdentity(id), DispositionShort),
      deliverDriverChild(id, expectedIdentity(id), DispositionFailed),
      deliverDriverChild(id, expectedIdentity(id), DispositionUncertain),
      abandonDriver,
      emitFanInChild(id),
      acceptFanInChild(id, expectedIdentity(id)),
      executeFanInChild(id, expectedIdentity(id)),
      deliverFanInChild(id, expectedIdentity(id), DispositionSuccess),
      deliverFanInChild(id, expectedIdentity(id), DispositionShort),
      deliverFanInChild(id, expectedIdentity(id), DispositionFailed),
      deliverFanInChild(id, expectedIdentity(id), DispositionUncertain),
      duplicateFanInChild(id, expectedIdentity(id), DispositionSuccess),
      duplicateFanInChild(id, expectedIdentity(id), DispositionShort),
      duplicateFanInChild(id, expectedIdentity(id), DispositionFailed),
      duplicateFanInChild(id, expectedIdentity(id), DispositionUncertain),
      reconcile(ReconciliationDurable),
      reconcile(ReconciliationUncertainRetained),
      confirmTransactionWriteRecoveryRecord,
      attemptTransactionDataParityWrite,
      confirmTransactionDataParityWritesDurable,
      observeTransactionPersistenceEvidence,
      commitTransactionRecoveryClean,
      observeMediaEffectTerminal(Operation),
      observeTerminalChildren(Operation),
      recordRequiredReconciliation(Operation),
      observeReclaimable(Operation),
      releaseTransactionRange(Operation),
      observeTransactionSatisfied(Operation),
      observeRecoveryAuthority(Operation),
      observeBasisConsumed(Operation),
      observeReleaseAuthorization(Operation),
      requestCleanup(Operation),
      release,
    }
  }

  val CoreReclaimable: bool = core::state.slot == SlotReclaimable
  val CoreDurableReconciliation: bool =
    core::state.reconciliation == ReconciliationDurable
  val TransactionRangeReleased: bool = recovery::state.rangeReleased
  val ReleaseAllowed: Set[OperationGeneration] = release::ReleaseAllowed
  def coreDuplicateSeen(id: str): bool =
    core::state.children.get(id).duplicateSeen
  def cleanupSucceeded(operation: OperationGeneration): bool =
    release::state.cleanup.get(operation) == release::CleanupSucceeded

  val BoundedState: bool = all {
    core::BoundedState,
    release::TypeInvariant,
    cursor >= 0,
    cursor <= ChildIds.size(),
  }

  val SubmittedSlotHasNoTerminalChildren: bool =
    core::state.slot != SlotSubmitted
      or ChildIds.forall(id =>
        core::state.children.get(id).slotState != SlotChildTerminal
      )

  val PhysicalExecutionRequiresAcceptance: bool = core::PhysicalExecutionRequiresAcceptance
  val ReclaimRequiresCompleteReconciliation: bool = core::ReclaimRequiresCompleteReconciliation
  val AbandonmentPreservesAcceptedChildren: bool = core::AbandonmentPreservesAcceptedChildren
  val TerminalChildrenCarryDisposition: bool = core::TerminalChildrenCarryDisposition
  val RecoveryProgressRequiresCoreFixtureSuccess: bool =
    (
      recovery::state.writeRecoveryRecord != recovery::WriteRecoveryRecordDurable
        and recovery::state.dataParityWriteState == recovery::NoDataParityWrites
        and recovery::state.storesWithPersistenceEvidence == Set()
    )
      or coreFixtureChildrenSucceeded
  val ReleaseRequiresExplicitOwnerObservations: bool = all {
    release::AuthorizationRequiresApplicability,
    release::AuthorizationRequiresAllOwnerFacts,
    release::MissingOrIncompleteEvidenceWithholds,
  }
  val DriverTransactionPrecedesExecution: bool =
    ChildIds.forall(id => {
      val child = core::state.children.get(id)
      child.work != WorkExecuted
        and child.work != WorkDelivered
        or recovery::state.writeLifecycle != recovery::Unowned
        or recovery::state.rangeReleased
    })
}
