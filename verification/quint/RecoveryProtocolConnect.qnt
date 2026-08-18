/**
 * Bounded Quint Connect spike for the VE-002 delegated relation.
 *
 * The bridge tests a projection of the model against dwv-transaction-ref. It
 * intentionally covers only normal completion and one uncertain home path.
 * The Rust machine may consume several semantic results for one abstract
 * action; private layout and non-delegated evidence remain outside this test.
 */
module RecoveryProtocolConnect {
  import RecoveryProtocol(
    Regions = Set("data", "parity"),
    Stores = Set("data", "parity"),
  ).* from "../../models/quint/RecoveryProtocol"

  action connectStep: bool = any {
    begin,
    acceptIntent,
    mutate,
    makeHomeDurable,
    fence,
    checkpoint,
    release,
    loseHome,
    reconcileHome,
  }
  run normalConnectPath =

    init
      .then(begin)
      .then(acceptIntent)
      .then(2.reps(i => mutate))
      .then(makeHomeDurable)
      .then(2.reps(i => fence))
      .then(checkpoint)
      .then(release)

  run uncertainHomeConnectPath =
    init
      .then(begin)
      .then(acceptIntent)
      .then(mutate)
      .then(loseHome)
      .then(reconcileHome)
}
