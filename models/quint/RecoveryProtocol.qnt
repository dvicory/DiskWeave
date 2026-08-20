// -*- mode: Bluespec; -*-
/**
 * verify.write-recovery-lifecycle bounded recovery protocol model.
 * The model is model-only. It defines one active write lifecycle at a
 * time. Explicit release permits sequential range reuse within the
 * represented relation.
 * Exact region mapping, persistence-evidence admissibility, topology,
 * persistence, resource lifetime, and production recovery authority remain
 * owned by their current OpenSpec requirements.
 */
// dwv:req req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent
module RecoveryProtocol {
  const Regions: Set[str]
  const Stores: Set[str]

  type WriteRecoveryRecordState = NoWriteRecoveryRecord | WriteRecoveryRecordPending | WriteRecoveryRecordDurable | WriteRecoveryRecordUnknown
  type DataParityWriteState = NoDataParityWrites | DataParityWritesAwaitingDurability | DataParityWritesDurable | DataParityWritesUnknown
  type RecoveryState = RecoveryClean | RecoveryDirty | RecoveryIndeterminate
  type WriteLifecycleState = Unowned | NormalProcessing | InterruptedProcessing | CompletedAwaitingRelease | AbortedAwaitingRelease
  type WriteRecoveryRecordCommitObservation =
    CommitUnknown
    | CommitRejected
    | CommitDurable
  type DataParityWriteObservation =
    NoDataParityWriteObservation
    | DataParityWriteEffectIndeterminate
    | DataParityWriteEffectDurable

  type ProtocolState = {
    writeRecoveryRecord: WriteRecoveryRecordState,
    writeRecoveryRecordCommitObservation: WriteRecoveryRecordCommitObservation,
    dataParityWriteState: DataParityWriteState,
    dataParityWriteObservation: DataParityWriteObservation,
    dataParityWriteObservationFinalized: bool,
    recoveryState: RecoveryState,
    writeLifecycle: WriteLifecycleState,
    integrityClaimsInvalidated: bool,
    writeKnownAppliedRegions: Set[str],
    writeAttemptedRegions: Set[str],
    storesWithPersistenceEvidence: Set[str],
    recoveryCleanRegions: Set[str],
    rangeOwned: bool,
    releasePending: bool,
    rangeReleased: bool,
  }

  var state: ProtocolState

  pure def active(s: ProtocolState): bool =
    s.writeLifecycle == NormalProcessing or s.writeLifecycle == InterruptedProcessing

  pure def abortedWriteAwaitingRelease(
    s: ProtocolState,
    observation: WriteRecoveryRecordCommitObservation,
  ): ProtocolState = {
    ...s,
    writeRecoveryRecord: NoWriteRecoveryRecord,
    writeRecoveryRecordCommitObservation: observation,
    dataParityWriteState: NoDataParityWrites,
    dataParityWriteObservation: NoDataParityWriteObservation,
    dataParityWriteObservationFinalized: false,
    recoveryState: RecoveryClean,
    writeLifecycle: AbortedAwaitingRelease,
    integrityClaimsInvalidated: false,
    writeKnownAppliedRegions: Set(),
    writeAttemptedRegions: Set(),
    storesWithPersistenceEvidence: Set(),
    recoveryCleanRegions: Set(),
    rangeOwned: true,
    releasePending: true,
    rangeReleased: false,
  }
  
  action init: bool = all {
    state' = {
      writeRecoveryRecord: NoWriteRecoveryRecord,
      writeRecoveryRecordCommitObservation: CommitUnknown,
      dataParityWriteState: NoDataParityWrites,
      dataParityWriteObservation: NoDataParityWriteObservation,
      dataParityWriteObservationFinalized: false,
      recoveryState: RecoveryClean,
      writeLifecycle: Unowned,
      integrityClaimsInvalidated: false,
      writeKnownAppliedRegions: Set(),
      writeAttemptedRegions: Set(),
      storesWithPersistenceEvidence: Set(),
      recoveryCleanRegions: Set(),
      rangeOwned: false,
      releasePending: false,
      rangeReleased: false,
    },
  }
  action startWrite: bool = all {
    state.writeLifecycle == Unowned,
    state' = {
      ...state,
      writeRecoveryRecord: WriteRecoveryRecordPending,
      writeRecoveryRecordCommitObservation: CommitUnknown,
      dataParityWriteState: NoDataParityWrites,
      dataParityWriteObservation: NoDataParityWriteObservation,
      dataParityWriteObservationFinalized: false,
      recoveryState: RecoveryDirty,
      writeLifecycle: NormalProcessing,
      integrityClaimsInvalidated: false,
      writeAttemptedRegions: Set(),
      writeKnownAppliedRegions: Set(),
      storesWithPersistenceEvidence: Set(),
      recoveryCleanRegions: Set(),
      rangeOwned: true,
      releasePending: false,
      rangeReleased: false,
    },
  }

  action confirmWriteRecoveryRecordDurable: bool = all {
    state.writeRecoveryRecord == WriteRecoveryRecordPending,
    state' = {
      ...state,
      writeRecoveryRecord: WriteRecoveryRecordDurable,
      writeRecoveryRecordCommitObservation: CommitDurable,
      integrityClaimsInvalidated: true,
    },
  }

  action observeWriteRecoveryRecordCommitUnknown: bool = all {
    state.writeRecoveryRecord == WriteRecoveryRecordPending,
    state' = {
      ...state,
      writeRecoveryRecord: WriteRecoveryRecordUnknown,
      writeRecoveryRecordCommitObservation: CommitUnknown,
      recoveryState: RecoveryIndeterminate,
      writeLifecycle: InterruptedProcessing,
    },
  }

  action reconcileWriteRecoveryRecordCommitObservation(observation: WriteRecoveryRecordCommitObservation): bool = all {
    observation == CommitRejected or observation == CommitDurable,
    state.writeRecoveryRecord == WriteRecoveryRecordUnknown,
    state.dataParityWriteState == NoDataParityWrites,
    state.writeKnownAppliedRegions == Set(),
    state' = if (observation == CommitRejected)
      abortedWriteAwaitingRelease(state, observation)
    else
      {
        ...state,
        writeRecoveryRecord: WriteRecoveryRecordDurable,
        writeRecoveryRecordCommitObservation: observation,
        recoveryState: RecoveryDirty,
        writeLifecycle: InterruptedProcessing,
        integrityClaimsInvalidated: true,
      },
  }

  action attemptDataParityWrite: bool = {
    nondet region = Regions.exclude(state.writeAttemptedRegions).oneOf()
    all {
      active(state),
      state.writeRecoveryRecord == WriteRecoveryRecordDurable,
      state.writeAttemptedRegions != Regions,
      state' = {
        ...state,
        dataParityWriteState: if (state.dataParityWriteState == DataParityWritesUnknown) DataParityWritesUnknown else DataParityWritesAwaitingDurability,
        dataParityWriteObservationFinalized: if (state.dataParityWriteState == DataParityWritesUnknown) false else state.dataParityWriteObservationFinalized,
        recoveryState: if (state.dataParityWriteState == DataParityWritesUnknown)
          RecoveryIndeterminate
        else
          state.recoveryState,
        writeLifecycle: if (state.dataParityWriteState == DataParityWritesUnknown) InterruptedProcessing else state.writeLifecycle,
        writeAttemptedRegions: state.writeAttemptedRegions.union(Set(region)),
        writeKnownAppliedRegions: if (state.dataParityWriteState == DataParityWritesUnknown)
          state.writeKnownAppliedRegions
        else
          state.writeKnownAppliedRegions.union(Set(region)),
      },
    }
  }

  action reconcileDataParityWriteCoverage: bool = all {
    state.writeRecoveryRecord == WriteRecoveryRecordDurable,
    state.dataParityWriteState == DataParityWritesUnknown,
    state.recoveryState == RecoveryIndeterminate,
    state.writeLifecycle == InterruptedProcessing,
    state.writeKnownAppliedRegions != state.writeAttemptedRegions,
    state' = {
      ...state,
      writeKnownAppliedRegions: state.writeAttemptedRegions,
    },
  }

  action confirmDataParityWritesDurable: bool = all {
    active(state),
    state.writeRecoveryRecord == WriteRecoveryRecordDurable,
    state.dataParityWriteState == DataParityWritesAwaitingDurability,
    state.writeAttemptedRegions == Regions,
    state.writeKnownAppliedRegions == Regions,
    state' = {
      ...state,
      dataParityWriteState: DataParityWritesDurable,
      dataParityWriteObservation: DataParityWriteEffectDurable,
      dataParityWriteObservationFinalized: true,
    },
  }

  action observePersistenceEvidence: bool = {
    nondet store = Stores.exclude(state.storesWithPersistenceEvidence).oneOf()
    all {
      active(state),
      state.writeRecoveryRecord == WriteRecoveryRecordDurable,
      state.dataParityWriteState == DataParityWritesDurable,
      state.storesWithPersistenceEvidence != Stores,
      state' = {
        ...state,
        storesWithPersistenceEvidence: state.storesWithPersistenceEvidence.union(Set(store)),
      },
    }
  }

  action commitRecoveryClean: bool = all {
    active(state),
    state.recoveryState == RecoveryDirty,
    state.writeRecoveryRecord == WriteRecoveryRecordDurable,
    state.dataParityWriteState == DataParityWritesDurable,
    state.writeKnownAppliedRegions == Regions,
    state.storesWithPersistenceEvidence == Stores,
    state' = {
      ...state,
      recoveryState: RecoveryClean,
      recoveryCleanRegions: Regions,
      writeLifecycle: CompletedAwaitingRelease,
      rangeOwned: true,
      releasePending: true,
      rangeReleased: false,
    },
  }

  action releaseRange: bool = all {
    state.rangeOwned,
    state.releasePending,
    state.writeLifecycle == CompletedAwaitingRelease or state.writeLifecycle == AbortedAwaitingRelease,
    state' = {
      writeRecoveryRecord: NoWriteRecoveryRecord,
      writeRecoveryRecordCommitObservation: CommitUnknown,
      dataParityWriteState: NoDataParityWrites,
      dataParityWriteObservation: NoDataParityWriteObservation,
      dataParityWriteObservationFinalized: false,
      recoveryState: RecoveryClean,
      writeLifecycle: Unowned,
      integrityClaimsInvalidated: false,
      writeKnownAppliedRegions: Set(),
      writeAttemptedRegions: Set(),
      storesWithPersistenceEvidence: Set(),
      recoveryCleanRegions: Set(),
      rangeOwned: false,
      releasePending: false,
      rangeReleased: true,
    },
  }
  action observeDataParityWriteEffectIndeterminate: bool = all {
    active(state),
    state.writeRecoveryRecord == WriteRecoveryRecordDurable,
    state.dataParityWriteState == DataParityWritesAwaitingDurability or state.dataParityWriteState == DataParityWritesDurable,
    state' = {
      ...state,
      dataParityWriteState: DataParityWritesUnknown,
      dataParityWriteObservation: DataParityWriteEffectIndeterminate,
      dataParityWriteObservationFinalized: false,
      recoveryState: RecoveryIndeterminate,
      writeLifecycle: InterruptedProcessing,
      writeAttemptedRegions: state.writeAttemptedRegions.union(state.writeKnownAppliedRegions),
      writeKnownAppliedRegions: Set(),
    },
  }

  action reconcileDataParityWriteEffectIndeterminate: bool = all {
    state.writeRecoveryRecord == WriteRecoveryRecordDurable,
    state.dataParityWriteState == DataParityWritesUnknown,
    state.recoveryState == RecoveryIndeterminate,
    state.writeLifecycle == InterruptedProcessing,
    (
      state.dataParityWriteObservation != DataParityWriteEffectIndeterminate
      or not(state.dataParityWriteObservationFinalized)
    ),
    state' = {
      ...state,
      dataParityWriteState: DataParityWritesUnknown,
      dataParityWriteObservation: DataParityWriteEffectIndeterminate,
      dataParityWriteObservationFinalized: true,
      recoveryState: RecoveryIndeterminate,
      writeLifecycle: InterruptedProcessing,
    },
  }

  action reconcileDataParityWriteEffectDurable: bool = all {
    state.writeRecoveryRecord == WriteRecoveryRecordDurable,
    state.dataParityWriteState == DataParityWritesUnknown,
    state.recoveryState == RecoveryIndeterminate,
    state.writeLifecycle == InterruptedProcessing,
    state.writeAttemptedRegions == Regions,
    state.writeKnownAppliedRegions == Regions,
    (
      state.dataParityWriteObservation != DataParityWriteEffectDurable
      or not(state.dataParityWriteObservationFinalized)
    ),
    state' = {
      ...state,
      dataParityWriteState: DataParityWritesDurable,
      dataParityWriteObservation: DataParityWriteEffectDurable,
      dataParityWriteObservationFinalized: true,
      recoveryState: RecoveryDirty,
      writeLifecycle: InterruptedProcessing,
    },
  }


  action enterInterruptedProcessing: bool = all {
    state.writeLifecycle == NormalProcessing,
    state' = if (state.writeRecoveryRecord == WriteRecoveryRecordPending and state.dataParityWriteState == NoDataParityWrites)
      {
        ...state,
        writeRecoveryRecord: WriteRecoveryRecordUnknown,
        writeRecoveryRecordCommitObservation: CommitUnknown,
        recoveryState: RecoveryIndeterminate,
        writeLifecycle: InterruptedProcessing,
      }
    else
      {
        ...state,
        dataParityWriteState: if (state.dataParityWriteState == DataParityWritesAwaitingDurability) DataParityWritesUnknown else state.dataParityWriteState,
        dataParityWriteObservation: if (state.dataParityWriteState == DataParityWritesAwaitingDurability)
          DataParityWriteEffectIndeterminate
        else
          state.dataParityWriteObservation,
        dataParityWriteObservationFinalized: if (state.dataParityWriteState == DataParityWritesAwaitingDurability)
          false
        else
          state.dataParityWriteObservationFinalized,
        recoveryState: if (
          state.writeRecoveryRecord == WriteRecoveryRecordUnknown
          or state.dataParityWriteState == DataParityWritesUnknown
          or state.dataParityWriteState == DataParityWritesAwaitingDurability
        ) RecoveryIndeterminate else RecoveryDirty,
        writeLifecycle: InterruptedProcessing,
        writeAttemptedRegions: if (state.dataParityWriteState == DataParityWritesAwaitingDurability)
          state.writeAttemptedRegions.union(state.writeKnownAppliedRegions)
        else
          state.writeAttemptedRegions,
        writeKnownAppliedRegions: if (state.dataParityWriteState == DataParityWritesAwaitingDurability) Set() else state.writeKnownAppliedRegions,
      },
  }

  action step: bool = any {
    startWrite,
    confirmWriteRecoveryRecordDurable,
    observeWriteRecoveryRecordCommitUnknown,
    reconcileWriteRecoveryRecordCommitObservation(CommitRejected),
    reconcileWriteRecoveryRecordCommitObservation(CommitDurable),
    attemptDataParityWrite,
    reconcileDataParityWriteCoverage,
    confirmDataParityWritesDurable,
    observePersistenceEvidence,
    commitRecoveryClean,
    releaseRange,
    observeDataParityWriteEffectIndeterminate,
    reconcileDataParityWriteEffectIndeterminate,
    reconcileDataParityWriteEffectDurable,
    enterInterruptedProcessing,
  }

  val TypeInvariant: bool =
    state.writeAttemptedRegions.subseteq(Regions)
    and state.writeKnownAppliedRegions.subseteq(Regions)
    and state.writeKnownAppliedRegions.subseteq(state.writeAttemptedRegions)
    and state.storesWithPersistenceEvidence.subseteq(Stores)
    and state.recoveryCleanRegions.subseteq(Regions)
    and (
      (state.writeLifecycle == Unowned) implies not(state.rangeOwned)
    )
    and (
      (state.writeLifecycle != Unowned) implies state.rangeOwned
    )
    and (
      state.rangeReleased implies (
        state.writeLifecycle == Unowned and not(state.rangeOwned)
      )
    )

  val CompletedOrAbortedRequiresRelease: bool =
    (state.writeLifecycle == CompletedAwaitingRelease or state.writeLifecycle == AbortedAwaitingRelease) implies (
      state.rangeOwned and state.releasePending
    )

  val DurableDataParityWriteRequiresCoverage: bool =
    state.dataParityWriteState == DataParityWritesDurable implies (
      state.writeAttemptedRegions == Regions and state.writeKnownAppliedRegions == Regions
    )
  val NoFalseClean: bool =
    (state.recoveryState == RecoveryClean) implies (
      (
        state.writeLifecycle == Unowned
        or state.writeLifecycle == CompletedAwaitingRelease
        or state.writeLifecycle == AbortedAwaitingRelease
      )
      and (
        state.writeRecoveryRecord == NoWriteRecoveryRecord
        or (
          state.writeRecoveryRecord == WriteRecoveryRecordDurable
          and state.integrityClaimsInvalidated
          and state.dataParityWriteState == DataParityWritesDurable
        )
      )
      and (state.writeLifecycle == CompletedAwaitingRelease implies state.recoveryCleanRegions == Regions)
      and (
        state.writeLifecycle == AbortedAwaitingRelease
        implies state.writeAttemptedRegions == Set() and state.writeKnownAppliedRegions == Set()
      )
    )

  val DataParityWriteRequiresWriteRecoveryRecord: bool =
    state.writeAttemptedRegions != Set() implies state.writeRecoveryRecord == WriteRecoveryRecordDurable

  val UncertaintyIsVisible: bool =
    (
      state.writeRecoveryRecord == WriteRecoveryRecordUnknown
      or state.dataParityWriteState == DataParityWritesUnknown
      or state.recoveryState == RecoveryIndeterminate
    ) implies (
      state.recoveryState != RecoveryClean
      and state.writeLifecycle != CompletedAwaitingRelease
      and state.writeLifecycle != AbortedAwaitingRelease
    )

  val UncertaintyIsOwned: bool =
    (
      state.writeRecoveryRecord == WriteRecoveryRecordUnknown
      or state.dataParityWriteState == DataParityWritesUnknown
      or state.recoveryState == RecoveryIndeterminate
    ) implies state.writeLifecycle != Unowned

  val DurableWorkIsOwned: bool =
    (
      state.writeRecoveryRecord == WriteRecoveryRecordDurable
      or state.writeRecoveryRecord == WriteRecoveryRecordUnknown
      or state.dataParityWriteState != NoDataParityWrites
    ) implies state.writeLifecycle != Unowned

  val CompletedWriteRequiresEvidence: bool =
    (state.writeLifecycle == CompletedAwaitingRelease) implies (
      state.recoveryState == RecoveryClean
      and state.writeRecoveryRecord == WriteRecoveryRecordDurable
      and state.integrityClaimsInvalidated
      and state.dataParityWriteState == DataParityWritesDurable
      and state.writeAttemptedRegions == Regions
      and state.writeKnownAppliedRegions == Regions
      and state.storesWithPersistenceEvidence == Stores
      and state.recoveryCleanRegions == Regions
      and state.rangeOwned
      and state.releasePending
      and not(state.rangeReleased)
    )

  val PersistenceEvidenceAndRecoveryCleanCoverage: bool =
    state.storesWithPersistenceEvidence.subseteq(Stores)
    and state.recoveryCleanRegions.subseteq(state.writeKnownAppliedRegions)
    and (state.recoveryCleanRegions != Set() implies state.storesWithPersistenceEvidence == Stores)
}

