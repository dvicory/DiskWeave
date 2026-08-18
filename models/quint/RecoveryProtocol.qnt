// -*- mode: Bluespec; -*-
/**
 * VE-002 bounded recovery protocol model.
 * The model is model-only. It defines one active admitted write obligation at
 * a time. Explicit release permits sequential range reuse within the
 * represented relation.
 * Exact region mapping, typed fence evidence, topology, persistence, resource
 * lifetime, and production recovery authority remain owned by their current
 * OpenSpec requirements.
 */
// dwv:req req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent
module RecoveryProtocol {
  const Regions: Set[str]
  const Stores: Set[str]

  type IntentState = NoIntent | IntentPending | IntentDurable | IntentUnknown
  type HomeState = HomeUnmodified | HomeVolatile | HomeDurable | HomeUnknown
  type RecoveryState = RecoveryClean | RecoveryDirty | RecoveryIndeterminate
  type ObligationState = Unowned | InFlight | Handoff | Terminal | Aborted
  type IntentObservation =
    IntentCommitUnknown
    | IntentCommitRejected
    | IntentCommitDurable
  type HomeObservation =
    HomeEffectUnknown
    | HomeEffectIndeterminate
    | HomeEffectDurable

  type ProtocolState = {
    intent: IntentState,
    intentObservation: IntentObservation,
    home: HomeState,
    homeObservation: HomeObservation,
    homeReconciled: bool,
    recovery: RecoveryState,
    obligation: ObligationState,
    invalidated: bool,
    mutated: Set[str],
    attempted: Set[str],
    fenced: Set[str],
    checkpointed: Set[str],
    rangeHeld: bool,
    terminalPendingRelease: bool,
    rangeReleased: bool,
  }

  var state: ProtocolState

  pure def active(s: ProtocolState): bool =
    s.obligation == InFlight or s.obligation == Handoff

  pure def abortedUnreleased(
    s: ProtocolState,
    observation: IntentObservation,
  ): ProtocolState = {
    ...s,
    intent: NoIntent,
    intentObservation: observation,
    home: HomeUnmodified,
    homeObservation: HomeEffectUnknown,
    homeReconciled: false,
    recovery: RecoveryClean,
    obligation: Aborted,
    invalidated: false,
    mutated: Set(),
    attempted: Set(),
    fenced: Set(),
    checkpointed: Set(),
    rangeHeld: true,
    terminalPendingRelease: true,
    rangeReleased: false,
  }
  
  action init: bool = all {
    state' = {
      intent: NoIntent,
      intentObservation: IntentCommitUnknown,
      home: HomeUnmodified,
      homeObservation: HomeEffectUnknown,
      homeReconciled: false,
      recovery: RecoveryClean,
      obligation: Unowned,
      invalidated: false,
      mutated: Set(),
      attempted: Set(),
      fenced: Set(),
      checkpointed: Set(),
      rangeHeld: false,
      terminalPendingRelease: false,
      rangeReleased: false,
    },
  }
  action begin: bool = all {
    state.obligation == Unowned,
    state' = {
      ...state,
      intent: IntentPending,
      intentObservation: IntentCommitUnknown,
      home: HomeUnmodified,
      homeObservation: HomeEffectUnknown,
      homeReconciled: false,
      recovery: RecoveryDirty,
      obligation: InFlight,
      invalidated: false,
      attempted: Set(),
      mutated: Set(),
      fenced: Set(),
      checkpointed: Set(),
      rangeHeld: true,
      terminalPendingRelease: false,
      rangeReleased: false,
    },
  }

  action acceptIntent: bool = all {
    state.intent == IntentPending,
    state' = {
      ...state,
      intent: IntentDurable,
      intentObservation: IntentCommitDurable,
      invalidated: true,
    },
  }

  action loseIntent: bool = all {
    state.intent == IntentPending,
    state' = {
      ...state,
      intent: IntentUnknown,
      intentObservation: IntentCommitUnknown,
      recovery: RecoveryIndeterminate,
      obligation: Handoff,
    },
  }

  action reconcileIntent(observation: IntentObservation): bool = all {
    observation == IntentCommitRejected or observation == IntentCommitDurable,
    state.intent == IntentUnknown,
    state.home == HomeUnmodified,
    state.mutated == Set(),
    state' = if (observation == IntentCommitRejected)
      abortedUnreleased(state, observation)
    else
      {
        ...state,
        intent: IntentDurable,
        intentObservation: observation,
        recovery: RecoveryDirty,
        obligation: Handoff,
        invalidated: true,
      },
  }

  action mutate: bool = {
    nondet region = Regions.exclude(state.attempted).oneOf()
    all {
      active(state),
      state.intent == IntentDurable,
      state.attempted != Regions,
      state' = {
        ...state,
        home: if (state.home == HomeUnknown) HomeUnknown else HomeVolatile,
        homeReconciled: if (state.home == HomeUnknown) false else state.homeReconciled,
        recovery: if (state.home == HomeUnknown)
          RecoveryIndeterminate
        else
          state.recovery,
        obligation: if (state.home == HomeUnknown) Handoff else state.obligation,
        attempted: state.attempted.union(Set(region)),
        mutated: if (state.home == HomeUnknown)
          state.mutated
        else
          state.mutated.union(Set(region)),
      },
    }
  }

  action reconcileMutation: bool = all {
    state.intent == IntentDurable,
    state.home == HomeUnknown,
    state.recovery == RecoveryIndeterminate,
    state.obligation == Handoff,
    state.mutated != state.attempted,
    state' = {
      ...state,
      mutated: state.attempted,
    },
  }

  action makeHomeDurable: bool = all {
    active(state),
    state.intent == IntentDurable,
    state.home == HomeVolatile,
    state.attempted == Regions,
    state.mutated == Regions,
    state' = {
      ...state,
      home: HomeDurable,
      homeObservation: HomeEffectDurable,
      homeReconciled: true,
    },
  }

  action fence: bool = {
    nondet store = Stores.exclude(state.fenced).oneOf()
    all {
      active(state),
      state.intent == IntentDurable,
      state.home == HomeDurable,
      state.fenced != Stores,
      state' = {
        ...state,
        fenced: state.fenced.union(Set(store)),
      },
    }
  }

  action checkpoint: bool = all {
    active(state),
    state.recovery == RecoveryDirty,
    state.intent == IntentDurable,
    state.home == HomeDurable,
    state.mutated == Regions,
    state.fenced == Stores,
    state' = {
      ...state,
      recovery: RecoveryClean,
      checkpointed: Regions,
      obligation: Terminal,
      rangeHeld: true,
      terminalPendingRelease: true,
      rangeReleased: false,
    },
  }

  action release: bool = all {
    state.rangeHeld,
    state.terminalPendingRelease,
    state.obligation == Terminal or state.obligation == Aborted,
    state' = {
      intent: NoIntent,
      intentObservation: IntentCommitUnknown,
      home: HomeUnmodified,
      homeObservation: HomeEffectUnknown,
      homeReconciled: false,
      recovery: RecoveryClean,
      obligation: Unowned,
      invalidated: false,
      mutated: Set(),
      attempted: Set(),
      fenced: Set(),
      checkpointed: Set(),
      rangeHeld: false,
      terminalPendingRelease: false,
      rangeReleased: true,
    },
  }
  action loseHome: bool = all {
    active(state),
    state.intent == IntentDurable,
    state.home == HomeVolatile or state.home == HomeDurable,
    state' = {
      ...state,
      home: HomeUnknown,
      homeObservation: HomeEffectIndeterminate,
      homeReconciled: false,
      recovery: RecoveryIndeterminate,
      obligation: Handoff,
      attempted: state.attempted.union(state.mutated),
      mutated: Set(),
    },
  }

  action reconcileHomeIndeterminate: bool = all {
    state.intent == IntentDurable,
    state.home == HomeUnknown,
    state.recovery == RecoveryIndeterminate,
    state.obligation == Handoff,
    (
      state.homeObservation != HomeEffectIndeterminate
      or not(state.homeReconciled)
    ),
    state' = {
      ...state,
      home: HomeUnknown,
      homeObservation: HomeEffectIndeterminate,
      homeReconciled: true,
      recovery: RecoveryIndeterminate,
      obligation: Handoff,
    },
  }

  action reconcileHomeDurable: bool = all {
    state.intent == IntentDurable,
    state.home == HomeUnknown,
    state.recovery == RecoveryIndeterminate,
    state.obligation == Handoff,
    state.attempted == Regions,
    state.mutated == Regions,
    (
      state.homeObservation != HomeEffectDurable
      or not(state.homeReconciled)
    ),
    state' = {
      ...state,
      home: HomeDurable,
      homeObservation: HomeEffectDurable,
      homeReconciled: true,
      recovery: RecoveryDirty,
      obligation: Handoff,
    },
  }


  action abandon: bool = all {
    state.obligation == InFlight,
    state' = if (state.intent == IntentPending and state.home == HomeUnmodified)
      {
        ...state,
        intent: IntentUnknown,
        intentObservation: IntentCommitUnknown,
        recovery: RecoveryIndeterminate,
        obligation: Handoff,
      }
    else
      {
        ...state,
        home: if (state.home == HomeVolatile) HomeUnknown else state.home,
        homeObservation: if (state.home == HomeVolatile)
          HomeEffectIndeterminate
        else
          state.homeObservation,
        homeReconciled: if (state.home == HomeVolatile)
          false
        else
          state.homeReconciled,
        recovery: if (
          state.intent == IntentUnknown
          or state.home == HomeUnknown
          or state.home == HomeVolatile
        ) RecoveryIndeterminate else RecoveryDirty,
        obligation: Handoff,
        attempted: if (state.home == HomeVolatile)
          state.attempted.union(state.mutated)
        else
          state.attempted,
        mutated: if (state.home == HomeVolatile) Set() else state.mutated,
      },
  }

  action step: bool = any {
    begin,
    acceptIntent,
    loseIntent,
    reconcileIntent(IntentCommitRejected),
    reconcileIntent(IntentCommitDurable),
    mutate,
    reconcileMutation,
    makeHomeDurable,
    fence,
    checkpoint,
    release,
    loseHome,
    reconcileHomeIndeterminate,
    reconcileHomeDurable,
    abandon,
  }

  val TypeInvariant: bool =
    state.attempted.subseteq(Regions)
    and state.mutated.subseteq(Regions)
    and state.mutated.subseteq(state.attempted)
    and state.fenced.subseteq(Stores)
    and state.checkpointed.subseteq(Regions)
    and (
      (state.obligation == Unowned) implies not(state.rangeHeld)
    )
    and (
      (state.obligation != Unowned) implies state.rangeHeld
    )
    and (
      state.rangeReleased implies (
        state.obligation == Unowned and not(state.rangeHeld)
      )
    )

  val TerminalRequiresRelease: bool =
    (state.obligation == Terminal or state.obligation == Aborted) implies (
      state.rangeHeld and state.terminalPendingRelease
    )

  val DurableHomeRequiresCoverage: bool =
    state.home == HomeDurable implies (
      state.attempted == Regions and state.mutated == Regions
    )
  val NoFalseClean: bool =
    (state.recovery == RecoveryClean) implies (
      (
        state.obligation == Unowned
        or state.obligation == Terminal
        or state.obligation == Aborted
      )
      and (
        state.intent == NoIntent
        or (
          state.intent == IntentDurable
          and state.invalidated
          and state.home == HomeDurable
        )
      )
      and (state.obligation == Terminal implies state.checkpointed == Regions)
      and (
        state.obligation == Aborted
        implies state.attempted == Set() and state.mutated == Set()
      )
    )

  val MutationRequiresIntent: bool =
    state.attempted != Set() implies state.intent == IntentDurable

  val UncertaintyIsVisible: bool =
    (
      state.intent == IntentUnknown
      or state.home == HomeUnknown
      or state.recovery == RecoveryIndeterminate
    ) implies (
      state.recovery != RecoveryClean
      and state.obligation != Terminal
      and state.obligation != Aborted
    )

  val UncertaintyIsOwned: bool =
    (
      state.intent == IntentUnknown
      or state.home == HomeUnknown
      or state.recovery == RecoveryIndeterminate
    ) implies state.obligation != Unowned

  val DurableWorkIsOwned: bool =
    (
      state.intent == IntentDurable
      or state.intent == IntentUnknown
      or state.home != HomeUnmodified
    ) implies state.obligation != Unowned

  val TerminalRequiresEvidence: bool =
    (state.obligation == Terminal) implies (
      state.recovery == RecoveryClean
      and state.intent == IntentDurable
      and state.invalidated
      and state.home == HomeDurable
      and state.attempted == Regions
      and state.mutated == Regions
      and state.fenced == Stores
      and state.checkpointed == Regions
      and state.rangeHeld
      and state.terminalPendingRelease
      and not(state.rangeReleased)
    )

  val FenceAndCheckpointCoverage: bool =
    state.fenced.subseteq(Stores)
    and state.checkpointed.subseteq(state.mutated)
    and (state.checkpointed != Set() implies state.fenced == Stores)
}

