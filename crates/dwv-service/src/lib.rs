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
    CodedCapturePhase, CodedCaptureSnapshot, ValidatedCodedCaptureScope,
};
pub use dwv_transaction_ref::{
    CodedAdmissionOutcome, CodedAuthorityError, CodedClaim, CodedClaimInput, CodedClaimRelease,
    CodedOperationPhase, CodedRangeAuthority,
};
#[doc = r#"
An exact-generation release capability produced by the service lifecycle owner.

Ordinary consumers cannot mint one from a slot token or promote the lower
lifecycle signing primitive:

```compile_fail
use dwv_service::ReleaseAuthorization;
use dwv_store::OperationSlotToken;

fn mint(operation: OperationSlotToken) -> ReleaseAuthorization {
    ReleaseAuthorization { operation }
}
```

```compile_fail
fn promote(
    authorization: dwv_lifecycle_authority::ReleaseAuthorization,
) -> dwv_service::ReleaseAuthorization {
    authorization
}
```

```compile_fail
use dwv_service::IncludedLifecycleAuthorization;
```
"#]
pub use evidence::ReleaseAuthorization;
pub use evidence::{CompletionEvidence, OperationEvidence, PersistenceClaim};
pub use failure::{FailureClass, ServiceError};
pub use lifecycle::ServiceState;
pub use metadata_loss::classify_metadata_loss_verification;
pub use range::{RangePlan, split_range};
pub use rebuild::{
    OfflineRebuildCommitError, RebuildStore, commit_verified_rebuild_chunk,
    commit_verified_rebuild_completion, validate_rebuild_resume,
};
#[doc = r#"
Proof that one exact emitted action crossed the backend-acceptance boundary.

Ordinary executors cannot forge the proof or report an unaccepted action:

```compile_fail
use dwv_service::{AcceptedPortableWriteWork, PortableWriteWork};

fn forge(work: PortableWriteWork) -> AcceptedPortableWriteWork {
    AcceptedPortableWriteWork { work }
}
```

```compile_fail
use dwv_service::{PortableWriteResult, PortableWriteWork};
use dwv_store::StoreCompletion;

fn report(work: PortableWriteWork, completion: StoreCompletion) -> PortableWriteResult {
    PortableWriteResult::new(work, completion, None)
}
```
"#]
pub use service::AcceptedPortableWriteWork;
pub use service::{
    BasisReadPermission, HealthyPortableService, MemberBinding, PortableOperationSubmission,
    PortableReadPayload, PortableWriteAction, PortableWriteDrive, PortableWriteResult,
    PortableWriteSubmission, PortableWriteWait, PortableWriteWork, PublicationIdentity,
    PublicationIdentityError, ServiceConfig, WritableStartAssessment, assess_writable_start,
    publication_identity,
};
