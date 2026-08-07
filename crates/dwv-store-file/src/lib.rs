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
    FileStore, FileStoreConfig, FileStoreError, FileSyncMode, ReadProgress, SyncEvidence,
};
pub use lease::{
    AliasError, FileIdentityError, FileLease, FileLeaseError, IdentityComparison,
    observe_file_identity, reject_backing_export_alias,
};

