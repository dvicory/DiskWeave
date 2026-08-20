/**
 * Bounded verify.write-recovery-lifecycle Quint Connect spike.
 *
 * The bridge tests the observable normal lifecycle against
 * dwv-transaction-ref. The retained seeds reach release and then a new
 * startWrite, so this projection covers release-followed-by-reuse for the
 * mapped lifecycle. It excludes abstract persistence-evidence and
 * data/parity-write reconciliation outcomes whose concrete evidence is owned
 * outside this projection.
 */
module RecoveryProtocolConnect {
  import RecoveryProtocol(
    Regions = Set("data"),
    Stores = Set(),
  ).* from "../../models/quint/RecoveryProtocol"

  action connectStep: bool = any {
    startWrite,
    confirmWriteRecoveryRecordDurable,
    attemptDataParityWrite,
    confirmDataParityWritesDurable,
    commitRecoveryClean,
    releaseRange,
  }

  run normalConnectPath =
    init
      .then(startWrite)
      .then(confirmWriteRecoveryRecordDurable)
      .then(attemptDataParityWrite)
      .then(confirmDataParityWritesDurable)
      .then(commitRecoveryClean)
      .then(releaseRange)
}
