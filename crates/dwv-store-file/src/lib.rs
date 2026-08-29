//! A fixed-geometry, Unix regular-file implementation of `RandomAccessStore`.
//!
//! Payload files contain only payload bytes.  Leases and the control database
//! are separate, disposable host-side resources and never participate in
//! interpreting the file contents.

mod capabilities;
mod control;
mod file_store;
mod lease;

pub use capabilities::{FileCapabilityReport, SparseBehavior, probe_file_capabilities};
pub use control::{
    CONTROL_SCHEMA_VERSION, ControlEntry, ControlError, ControlExport, ControlProjection,
};
pub use file_store::{
    AcceptedFileStore, AcceptedFileWriterSet, FileBackedReleaseDisposition,
    FileBackedReleaseObservation, FileOwnershipResource, FileReleaseCause, FileReleaseCauseKind,
    FileStore, FileStoreConfig, FileStoreError, FileSyncMode, FileWriterAcquisitionId,
    FileWriterAliasBinding, FileWriterClaimBinding, FileWriterClaimDisposition,
    FileWriterClaimReleaseObservation, FileWriterClaimToken, FileWriterQuarantineId,
    FileWriterQuarantineRetryError, FileWriterQuarantineRetryFailure, FileWriterQuarantineStatus,
    FileWriterSetAcceptanceError, FileWriterSetAcceptanceFailure, FileWriterSetReleaseResult,
    ReadProgress, ReleasedFileWriterSet, SyncEvidence, UnresolvedFileWriterSet,
    quarantined_file_writer_sets, retry_quarantined_file_writer_set,
};
pub use lease::{
    AliasError, FileIdentityError, FileLease, FileLeaseError, IdentityComparison,
    observe_file_identity, reject_backing_export_alias,
};
