/**
 * Bounded Quint Connect spike for the VE-002 delegated relation.
 *
 * The bridge tests the observable normal lifecycle against
 * dwv-transaction-ref. It excludes abstract fence and home-reconciliation
 * outcomes whose concrete evidence is owned outside this projection.
 * The Rust machine may consume several semantic results for one abstract
 * action; private layout and non-delegated evidence remain outside this test.
 */
module RecoveryProtocolConnect {
  import RecoveryProtocol(
    Regions = Set("data"),
    Stores = Set(),
  ).* from "../../models/quint/RecoveryProtocol"

  action connectStep: bool = any {
    begin,
    acceptIntent,
    mutate,
    makeHomeDurable,
    checkpoint,
    release,
  }

  run normalConnectPath =
    init
      .then(begin)
      .then(acceptIntent)
      .then(mutate)
      .then(makeHomeDurable)
      .then(checkpoint)
      .then(release)

}
