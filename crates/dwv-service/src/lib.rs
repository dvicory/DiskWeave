//! Healthy, single-parity portable I/O over mechanism-neutral stores.
//!
//! This crate is runtime-independent. It owns portable service semantics;
//! physical acceptance and normalized completion delivery cross an explicit
//! boundary, while file, FSKit, DiskImages, Linux, and other mechanism
//! adapters remain outside its semantic ownership.

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
pub use dwv_recovery::{
    CodedCaptureCoordinator, CodedCaptureCut, CodedCaptureDecision, CodedCaptureError,
    CodedCaptureFrontier, CodedCaptureId, CodedCaptureMembership, CodedCaptureOwnerFacts,
    CodedCapturePhase, CodedCaptureRetentionSummary, CodedCaptureRetirement,
    CodedCaptureScopeInput, CodedCaptureSnapshot, CodedCleanCommitObservation,
    CodedCleanReconciliation, CodedLaterCutObservation, CodedLaterCutReconciliation,
};
pub use dwv_transaction_ref::{
    CodedAdmissionOutcome, CodedAuthorityError, CodedClaim, CodedClaimInput, CodedClaimRelease,
    CodedOperationPhase, CodedRangeAuthority,
};
pub use evidence::{CompletionEvidence, OperationEvidence, PersistenceClaim, ReleaseAuthorization};
pub use failure::{FailureClass, ServiceError};
pub use lifecycle::ServiceState;
pub use metadata_loss::classify_metadata_loss_verification;
pub use range::{RangePlan, split_range};
pub use rebuild::{
    OfflineRebuildCommitError, RebuildStore, commit_verified_rebuild_chunk,
    commit_verified_rebuild_completion, validate_rebuild_resume,
};
pub use service::{
    BasisReadPermission, HealthyPortableService, MemberBinding, PortableOperationSubmission,
    PortableReadPayload, PortableWriteAction, PortableWriteDrive, PortableWriteResult,
    PortableWriteSubmission, PortableWriteWait, PortableWriteWork, PublicationIdentity,
    PublicationIdentityError, ServiceConfig, WritableStartAssessment, assess_writable_start,
    publication_identity,
};
