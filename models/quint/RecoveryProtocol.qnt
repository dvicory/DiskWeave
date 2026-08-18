// -*- mode: Bluespec; -*-
/**
 * VE-002 bounded recovery protocol model.
 *
 * The model is model-only. It defines one admitted write obligation over two
 * affected regions and two stores. Exact region mapping, typed fence evidence,
 * topology, persistence, resource lifetime, and production recovery authority
 * remain owned by their current OpenSpec requirements.
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
    recovery: RecoveryState,
    obligation: ObligationState,
    invalidated: bool,
    mutated: Set[str],
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
    recovery: RecoveryClean,
    obligation: Aborted,
    invalidated: false,
    mutated: Set(),
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
      recovery: RecoveryClean,
      obligation: Unowned,
      invalidated: false,
      mutated: Set(),
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
      recovery: RecoveryDirty,
      obligation: InFlight,
      invalidated: false,
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

  action reconcileIntent: bool = {
    nondet observation = Set(
      IntentCommitRejected,
      IntentCommitDurable,
    ).oneOf()
    all {
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
  }

  action mutate: bool = {
    nondet region = Regions.exclude(state.mutated).oneOf()
    all {
      active(state),
      state.intent == IntentDurable,
      state.mutated != Regions,
      state' = {
        ...state,
        home: HomeVolatile,
        mutated: state.mutated.union(Set(region)),
      },
    }
  }

  action makeHomeDurable: bool = all {
    active(state),
    state.intent == IntentDurable,
    state.home == HomeVolatile,
    state.mutated == Regions,
    state' = {
      ...state,
      home: HomeDurable,
      homeObservation: HomeEffectDurable,
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
      ...state,
      intent: NoIntent,
      intentObservation: IntentCommitUnknown,
      home: HomeUnmodified,
      homeObservation: HomeEffectUnknown,
      recovery: RecoveryClean,
      obligation: Unowned,
      invalidated: false,
      mutated: Set(),
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
      recovery: RecoveryIndeterminate,
      obligation: Handoff,
    },
  }

  action reconcileHome: bool = {
    nondet observation = Set(
      HomeEffectIndeterminate,
      HomeEffectDurable,
    ).filter(effect =>
      state.mutated == Regions or effect == HomeEffectIndeterminate
    ).oneOf()
    all {
      state.intent == IntentDurable,
      state.home == HomeUnknown,
      state.recovery == RecoveryIndeterminate,
      state.obligation == Handoff,
      state' = if (observation == HomeEffectDurable)
        {
          ...state,
          home: HomeDurable,
          homeObservation: observation,
          recovery: RecoveryDirty,
          obligation: Handoff,
        }
      else
        {
          ...state,
          home: HomeUnknown,
          homeObservation: observation,
          recovery: RecoveryIndeterminate,
          obligation: Handoff,
        },
    }
  }

  action abandon: bool = all {
    active(state),
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
        recovery: if (
          state.intent == IntentUnknown or state.home == HomeUnknown
        ) RecoveryIndeterminate else RecoveryDirty,
        obligation: Handoff,
      },
  }

  action step: bool = any {
    begin,
    acceptIntent,
    loseIntent,
    reconcileIntent,
    mutate,
    makeHomeDurable,
    fence,
    checkpoint,
    release,
    loseHome,
    reconcileHome,
    abandon,
  }

  val TypeInvariant: bool =
    state.mutated.subseteq(Regions)
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
    state.home == HomeDurable implies state.mutated == Regions
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
      and (state.obligation == Aborted implies state.mutated == Set())
    )

  val MutationRequiresIntent: bool =
    state.mutated != Set() implies state.intent == IntentDurable

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

