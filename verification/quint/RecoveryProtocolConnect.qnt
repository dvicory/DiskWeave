/**
 * Bounded Quint Connect spike for the VE-002 delegated relation.
 *
 * The bridge tests the observable normal lifecycle against
 * dwv-transaction-ref. The retained seeds reach release and then a new
 * begin, so this projection covers release-followed-by-reuse for the mapped
 * lifecycle. It excludes abstract fence and home-reconciliation outcomes
 * whose concrete evidence is owned outside this projection.
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
