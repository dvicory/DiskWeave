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

  run boundedAssumptions = all {
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
