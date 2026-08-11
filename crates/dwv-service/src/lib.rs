//! Healthy, single-parity portable I/O over mechanism-neutral stores.
//!
//! This crate is deliberately synchronous and runtime-independent. It owns
//! portable service semantics; file, FSKit, DiskImages, Linux, and other
//! mechanism adapters remain outside its boundary.

mod admission;
mod degraded;
mod evidence;
mod failure;
mod lifecycle;
mod metadata_loss;
mod range;
mod read;
mod rebuild;
mod service;
mod write;

pub use admission::{AdmissionConfig, OperationAdmission};
pub use degraded::{OfflineAuthorizationError, RebuildSource, authorize_known_erasure_from_stores};
pub use evidence::{CompletionEvidence, OperationEvidence, PersistenceClaim};
pub use failure::{FailureClass, ServiceError};
pub use lifecycle::ServiceState;
pub use metadata_loss::classify_metadata_loss_verification;
pub use range::{RangePlan, split_range};
pub use rebuild::{
    OfflineRebuildCommitError, RebuildStore, commit_verified_rebuild_chunk,
    commit_verified_rebuild_completion, validate_rebuild_resume,
};
pub use service::{
    HealthyPortableService, MemberBinding, PublicationIdentity, PublicationIdentityError,
    ServiceConfig, WritableStartAssessment, assess_writable_start, publication_identity,
};
