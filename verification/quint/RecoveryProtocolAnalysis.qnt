/**
 * verify.write-recovery-lifecycle finite evidence instance for the canonical
 * recovery protocol.
 *
 * The imported model owns the parameterized state/action/invariant relation.
 * These concrete values, witnesses, and bounds belong to verification only.
 */
module RecoveryProtocolAnalysis {
  import RecoveryProtocol(
    Regions = Set("data", "parity"),
    Stores = Set("data", "parity"),
  ).* from "../../models/quint/RecoveryProtocol"

  run boundedAssumptionsTest = all {
    Regions == Set("data", "parity"),
    Stores == Set("data", "parity"),
  }

  val startWriteReachable: bool = state.writeLifecycle == NormalProcessing
  val durableWriteRecoveryRecordReachable: bool =
    state.writeRecoveryRecord == WriteRecoveryRecordDurable
  val dataParityWriteReachable: bool =
    state.writeKnownAppliedRegions != Set()
      or state.writeAttemptedRegions != Set()
  val unknownWriteRecoveryRecordReachable: bool =
    state.writeRecoveryRecord == WriteRecoveryRecordUnknown
  val unknownDataParityWriteReachable: bool =
    state.dataParityWriteState == DataParityWritesUnknown
  val interruptedProcessingReachable: bool =
    state.writeLifecycle == InterruptedProcessing
  val completedAwaitingReleaseReachable: bool =
    state.writeLifecycle == CompletedAwaitingRelease
  val releasePendingReachable: bool =
    state.writeLifecycle == CompletedAwaitingRelease
    and state.rangeOwned
    and state.releasePending
  val abortedAwaitingReleaseReachable: bool =
    state.writeLifecycle == AbortedAwaitingRelease
    and state.rangeOwned
    and state.releasePending
  val releasedReachable: bool =
    state.writeLifecycle == Unowned
    and state.rangeReleased
    and not(state.rangeOwned)
  val resumedDataParityWriteReachable: bool =
    state.dataParityWriteState == DataParityWritesUnknown
    and state.writeKnownAppliedRegions != Set()
    and state.writeKnownAppliedRegions != Regions
  val reconciledDataParityWriteReachable: bool =
    state.dataParityWriteState == DataParityWritesUnknown
    and state.writeAttemptedRegions == Regions
    and state.writeKnownAppliedRegions == Regions
  val durableDataParityWriteReachable: bool =
    state.dataParityWriteState == DataParityWritesDurable
    and state.writeAttemptedRegions == Regions
    and state.writeKnownAppliedRegions == Regions

  run completedReleaseRequiredTest =
    init
      .then(startWrite)
      .then(confirmWriteRecoveryRecordDurable)
      .then(2.reps(i => attemptDataParityWrite))
      .then(confirmDataParityWritesDurable)
      .expect(
        durableDataParityWriteReachable
        and state.recoveryState == RecoveryDirty
      )
      .then(2.reps(i => observePersistenceEvidence))
      .then(commitRecoveryClean)
      .then(releaseRange)
      .expect(
        state.writeLifecycle == Unowned
        and state.rangeReleased
        and not(state.rangeOwned)
      )

  run completedWriteStartBlockedTest =
    init
      .then(startWrite)
      .then(confirmWriteRecoveryRecordDurable)
      .then(2.reps(i => attemptDataParityWrite))
      .then(confirmDataParityWritesDurable)
      .then(2.reps(i => observePersistenceEvidence))
      .then(commitRecoveryClean)
      .then(startWrite.fail())

  run releasePermitsReuseTest =
    init
      .then(startWrite)
      .then(confirmWriteRecoveryRecordDurable)
      .then(2.reps(i => attemptDataParityWrite))
      .then(confirmDataParityWritesDurable)
      .then(2.reps(i => observePersistenceEvidence))
      .then(commitRecoveryClean)
      .then(releaseRange)
      .then(startWrite)
      .expect(
        state.writeLifecycle == NormalProcessing
        and state.rangeOwned
        and not(state.rangeReleased)
      )

  run rejectedWriteRecoveryRecordReleaseTest =
    init
      .then(startWrite)
      .then(observeWriteRecoveryRecordCommitUnknown)
      .then(
        reconcileWriteRecoveryRecordCommitObservation(
          CommitRejected
        )
      )
      .expect(
        state.writeLifecycle == AbortedAwaitingRelease
        and state.rangeOwned
        and state.releasePending
      )
      .then(releaseRange)
      .expect(
        state.writeLifecycle == Unowned
        and state.rangeReleased
        and not(state.rangeOwned)
      )

  run volatileAbandonmentContinuationTest =
    init
      .then(startWrite)
      .then(confirmWriteRecoveryRecordDurable)
      .then(attemptDataParityWrite)
      .then(enterInterruptedProcessing)
      .expect(
        state.dataParityWriteState == DataParityWritesUnknown
        and state.recoveryState == RecoveryIndeterminate
        and state.writeLifecycle == InterruptedProcessing
        and state.writeAttemptedRegions != Set()
        and state.writeKnownAppliedRegions == Set()
      )
      .then(reconcileDataParityWriteCoverage)
      .then(attemptDataParityWrite)
      .then(reconcileDataParityWriteCoverage)
      .expect(
        state.dataParityWriteState == DataParityWritesUnknown
        and state.writeAttemptedRegions == Regions
        and state.writeKnownAppliedRegions == Regions
      )
      .then(reconcileDataParityWriteEffectDurable)
      .expect(
        state.dataParityWriteState == DataParityWritesDurable
        and state.recoveryState == RecoveryDirty
      )
      .then(2.reps(i => observePersistenceEvidence))
      .then(commitRecoveryClean)
      .then(releaseRange)
      .expect(state.writeLifecycle == Unowned and state.rangeReleased)

  run partialDataParityWriteCannotBeDurableTest =
    init
      .then(startWrite)
      .then(confirmWriteRecoveryRecordDurable)
      .then(attemptDataParityWrite)
      .then(observeDataParityWriteEffectIndeterminate)
      .then(reconcileDataParityWriteEffectIndeterminate)
      .then(attemptDataParityWrite)
      .expect(
        state.dataParityWriteState == DataParityWritesUnknown
        and state.writeAttemptedRegions == Regions
        and state.writeKnownAppliedRegions != Regions
      )
      .then(reconcileDataParityWriteEffectDurable.fail())

  run repeatedAbandonBlockedTest =
    init
      .then(startWrite)
      .then(confirmWriteRecoveryRecordDurable)
      .then(attemptDataParityWrite)
      .then(enterInterruptedProcessing)
      .then(enterInterruptedProcessing.fail())

  run repeatedDataParityWriteReconciliationBlockedTest =
    init
      .then(startWrite)
      .then(confirmWriteRecoveryRecordDurable)
      .then(attemptDataParityWrite)
      .then(observeDataParityWriteEffectIndeterminate)
      .then(reconcileDataParityWriteEffectIndeterminate)
      .then(reconcileDataParityWriteEffectIndeterminate.fail())
}
