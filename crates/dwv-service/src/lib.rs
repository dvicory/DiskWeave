//! Healthy, single-parity portable I/O over ordinary file-backed members.
//!
//! This crate is deliberately synchronous and runtime-independent.  It is a
//! semantic service for the portable/macOS reference path; FSKit, DiskImages,
//! Linux adapters, degraded repair, and physical power-loss claims remain
//! outside its boundary.

mod admission;
mod evidence;
mod failure;
mod lifecycle;
mod metadata_loss;
mod range;
mod read;
mod request;
mod service;
mod write;

pub use admission::{AdmissionConfig, OperationAdmission};
pub use evidence::{CompletionEvidence, OperationEvidence, PersistenceClaim};
pub use failure::{FailureClass, ServiceError};
pub use lifecycle::ServiceState;
pub use metadata_loss::classify_metadata_loss_verification;
pub use range::{RangePlan, split_range};
pub use request::{PortableRequest, RequestOperation};
pub use service::{HealthyPortableService, MemberStore, ServiceConfig};
