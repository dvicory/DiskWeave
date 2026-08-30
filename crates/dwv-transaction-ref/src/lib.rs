//! A small, explicit transaction-ordering oracle.
//!
//! This crate contains semantic actions and results only.  An executor may
//! fan an action out into child operations, but those child operations never
//! enter this API.  In particular, this crate has no runtime, database, or
//! operating-system dependency.

mod action;
mod coded;
mod error;
mod machine;
mod trace;

pub use action::{
    ActionKind, ActionResult, CommittedRecoveryGeneration, ComputationResult,
    ParityComputationPlan, ParityRange, PlannedRead, PlannedWrite, RangeGuardToken, ResultKind,
    SemanticFailure, SemanticIoResult, StoreWatermark, StoreWatermarks, TransactionAction,
    TransactionIdentity, TransactionPersistenceEvidence, WriteRecoveryRecordRequirement,
};
pub use coded::{
    CodedAdmission, CodedAdmissionOutcome, CodedAuthorityError, CodedClaim, CodedClaimInput,
    CodedClaimRelease, CodedEffectPermit, CodedOperationPhase, CodedRangeAuthority,
};
pub use error::{ErrorClass, PlanError, TransactionError};
pub use machine::{
    Disposition, Stage, TerminalDisposition, TransactionLimits, TransactionMachine,
    TransactionPlan, TransactionState,
};
pub use trace::{TRACE_VERSION, Trace, TraceEvent};
