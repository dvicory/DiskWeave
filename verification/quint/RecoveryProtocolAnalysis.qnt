/**
 * VE-002 finite evidence instance for the canonical recovery protocol.
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

  val beginReachable: bool = state.obligation == InFlight
  val durableIntentReachable: bool = state.intent == IntentDurable
  val mutationReachable: bool = state.mutated != Set()
  val uncertainIntentReachable: bool = state.intent == IntentUnknown
  val uncertainHomeReachable: bool = state.home == HomeUnknown
  val reconciliationReachable: bool = state.obligation == Handoff
  val terminalReachable: bool = state.obligation == Terminal
  val terminalPendingReleaseReachable: bool =
    state.obligation == Terminal
    and state.rangeHeld
    and state.terminalPendingRelease
  val abortedReachable: bool =
    state.obligation == Aborted
    and state.rangeHeld
    and state.terminalPendingRelease
  val releasedReachable: bool =
    state.obligation == Unowned
    and state.rangeReleased
    and not(state.rangeHeld)
  val resumedMutationReachable: bool =
    state.home == HomeVolatile
    and state.mutated != Set()
    and state.mutated != Regions
  val durableHomeCoverage: bool =
    state.home == HomeDurable implies state.mutated == Regions
  run terminalReleaseRequiredTest =
    init
      .then(begin)
      .then(acceptIntent)
      .then(2.reps(i => mutate))
      .then(makeHomeDurable)
      .then(2.reps(i => fence))
      .then(checkpoint)
      .then(release)
      .expect(
        state.obligation == Unowned
        and state.rangeReleased
        and not(state.rangeHeld)
      )

  run terminalBeginBlockedTest =
    init
      .then(begin)
      .then(acceptIntent)
      .then(2.reps(i => mutate))
      .then(makeHomeDurable)
      .then(2.reps(i => fence))
      .then(checkpoint)
      .then(begin.fail())

  run partialHomeCannotBeDurableTest =
    init
      .then(begin)
      .then(acceptIntent)
      .then(mutate)
      .then(loseHome)
      .then(reconcileHome)
      .expect(
        state.home == HomeUnknown
        and state.mutated != Regions
      )
      .then(makeHomeDurable.fail())
}
