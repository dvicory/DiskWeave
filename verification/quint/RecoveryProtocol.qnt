// -*- mode: Bluespec; -*-
/**
 * VE-002 bounded recovery protocol model.
 *
 * The model is model-only. It defines one admitted write obligation over two
 * affected regions and two stores. Exact region mapping, typed fence evidence,
 * topology, persistence, resource lifetime, and production recovery authority
 * remain owned by their current OpenSpec requirements.
 */
module RecoveryProtocol {
  const MaxDepth: int
  const Regions: Set[str]
  const Stores: Set[str]

  type IntentState = NoIntent | IntentPending | IntentDurable | IntentUnknown
  type HomeState = HomeUnmodified | HomeVolatile | HomeDurable | HomeUnknown
  type RecoveryState = RecoveryClean | RecoveryDirty | RecoveryIndeterminate
  type ObligationState = Unowned | InFlight | Handoff | Terminal

  type ProtocolState = {
    intent: IntentState,
    home: HomeState,
    recovery: RecoveryState,
    obligation: ObligationState,
    invalidated: bool,
    mutated: Set[str],
    fenced: Set[str],
    checkpointed: Set[str],
    steps: int,
  }

  var state: ProtocolState

  pure def withinBudget(s: ProtocolState): bool = s.steps < MaxDepth

  pure def active(s: ProtocolState): bool =
    s.obligation == InFlight or s.obligation == Handoff

  pure def cleanUnowned(s: ProtocolState): ProtocolState =
    {
      ...s,
      intent: NoIntent,
      home: HomeUnmodified,
      recovery: RecoveryClean,
      obligation: Unowned,
      invalidated: false,
      mutated: Set(),
      fenced: Set(),
      checkpointed: Set(),
      steps: s.steps + 1,
    }

  action init: bool = all {
    state' = {
      intent: NoIntent,
      home: HomeUnmodified,
      recovery: RecoveryClean,
      obligation: Unowned,
      invalidated: false,
      mutated: Set(),
      fenced: Set(),
      checkpointed: Set(),
      steps: 0,
    },
  }

  action begin: bool = all {
    withinBudget(state),
    state.obligation == Unowned or state.obligation == Terminal,
    state' = {
      ...state,
      intent: IntentPending,
      home: HomeUnmodified,
      recovery: RecoveryDirty,
      obligation: InFlight,
      invalidated: false,
      mutated: Set(),
      fenced: Set(),
      checkpointed: Set(),
      steps: state.steps + 1,
    },
  }

  action acceptIntent: bool = all {
    withinBudget(state),
    state.intent == IntentPending,
    state' = {
      ...state,
      intent: IntentDurable,
      invalidated: true,
      steps: state.steps + 1,
    },
  }

  action loseIntent: bool = all {
    withinBudget(state),
    state.intent == IntentPending,
    state' = {
      ...state,
      intent: IntentUnknown,
      recovery: RecoveryIndeterminate,
      obligation: Handoff,
      steps: state.steps + 1,
    },
  }

  action reconcileIntentPrior: bool = all {
    withinBudget(state),
    state.intent == IntentUnknown,
    state.home == HomeUnmodified,
    state.mutated == Set(),
    state' = cleanUnowned(state),
  }

  action reconcileIntentProposed: bool = all {
    withinBudget(state),
    state.intent == IntentUnknown,
    state.home == HomeUnmodified,
    state.mutated == Set(),
    state' = {
      ...state,
      intent: IntentDurable,
      recovery: RecoveryDirty,
      obligation: Handoff,
      invalidated: true,
      steps: state.steps + 1,
    },
  }

  action mutate: bool = {
    nondet region = Regions.exclude(state.mutated).oneOf()
    all {
      withinBudget(state),
      active(state),
      state.intent == IntentDurable,
      state.mutated != Regions,
      state' = {
        ...state,
        home: HomeVolatile,
        mutated: state.mutated.union(Set(region)),
        steps: state.steps + 1,
      },
    }
  }

  action makeHomeDurable: bool = all {
    withinBudget(state),
    active(state),
    state.intent == IntentDurable,
    state.home == HomeVolatile,
    state.mutated == Regions,
    state' = {
      ...state,
      home: HomeDurable,
      steps: state.steps + 1,
    },
  }

  action fence: bool = {
    nondet store = Stores.exclude(state.fenced).oneOf()
    all {
      withinBudget(state),
      state.home == HomeDurable,
      state.fenced != Stores,
      state' = {
        ...state,
        fenced: state.fenced.union(Set(store)),
        steps: state.steps + 1,
      },
    }
  }

  action checkpoint: bool = all {
    withinBudget(state),
    state.recovery == RecoveryDirty,
    state.home == HomeDurable,
    state.intent == IntentDurable,
    state.mutated == Regions,
    state.fenced == Stores,
    state' = {
      ...state,
      recovery: RecoveryClean,
      checkpointed: Regions,
      obligation: Terminal,
      steps: state.steps + 1,
    },
  }

  action loseHome: bool = all {
    withinBudget(state),
    active(state),
    state.intent == IntentDurable,
    state.home == HomeVolatile or state.home == HomeDurable,
    state' = {
      ...state,
      home: HomeUnknown,
      recovery: RecoveryIndeterminate,
      obligation: Handoff,
      steps: state.steps + 1,
    },
  }

  action reconcileHome: bool = all {
    withinBudget(state),
    state.intent == IntentDurable,
    state.home == HomeUnknown,
    state.recovery == RecoveryIndeterminate,
    state.obligation == Handoff,
    state' = {
      ...state,
      home: HomeDurable,
      recovery: RecoveryDirty,
      obligation: Handoff,
      steps: state.steps + 1,
    },
  }

  action abandon: bool = all {
    withinBudget(state),
    active(state),
    state' = if (state.intent == IntentPending and state.home == HomeUnmodified)
      cleanUnowned(state)
    else
      {
        ...state,
        home: if (state.home == HomeVolatile) HomeUnknown else state.home,
        recovery: if (
          state.intent == IntentUnknown or state.home == HomeUnknown
        ) RecoveryIndeterminate else RecoveryDirty,
        obligation: Handoff,
        steps: state.steps + 1,
      },
  }

  action step: bool = any {
    begin,
    acceptIntent,
    loseIntent,
    reconcileIntentPrior,
    reconcileIntentProposed,
    mutate,
    makeHomeDurable,
    fence,
    checkpoint,
    loseHome,
    reconcileHome,
    abandon,
  }

  val TypeInvariant: bool =
    state.mutated.subseteq(Regions)
    and state.fenced.subseteq(Stores)
    and state.checkpointed.subseteq(Regions)
    and state.steps >= 0
    and state.steps <= MaxDepth

  val NoFalseClean: bool =
    (state.recovery == RecoveryClean) implies (
      (state.obligation == Unowned or state.obligation == Terminal)
      and (
        state.intent == NoIntent
        or (
          state.intent == IntentDurable
          and state.invalidated
          and state.home == HomeDurable
        )
      )
      and (state.obligation == Terminal implies state.checkpointed == Regions)
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
    )

  val FenceAndCheckpointCoverage: bool =
    state.fenced.subseteq(Stores)
    and state.checkpointed.subseteq(state.mutated)
    and (state.checkpointed != Set() implies state.fenced == Stores)
}

module RecoveryProtocolAnalysis {
  import RecoveryProtocol(
    MaxDepth = 8,
    Regions = Set("data", "parity"),
    Stores = Set("data", "parity"),
  ).*

  run boundedAssumptions = all {
    MaxDepth == 8,
    Regions == Set("data", "parity"),
    Stores == Set("data", "parity"),
  }

  val beginReachable: bool = state.obligation == InFlight
  val durableIntentReachable: bool = state.intent == IntentDurable
  val mutationReachable: bool = state.mutated != Set()
  val uncertainIntentReachable: bool = state.intent == IntentUnknown
  val uncertainHomeReachable: bool = state.home == HomeUnknown
  val reconciliationReachable: bool = state.obligation == Handoff
  val terminalReachable: bool = state.obligation == Terminal
}
