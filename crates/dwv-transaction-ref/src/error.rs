use crate::action::{ActionKind, ResultKind};
use dwv_core::TopologyEpoch;
use dwv_recovery::RecoveryGeneration;
use std::fmt;

/// Stable classes used in normalized traces and semantic failures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ErrorClass {
    RangeAcquisitionFailed,
    WriteRecoveryRecordRejected,
    WriteRecoveryRecordLost,
    WriteRecoveryRecordCorrupt,
    ReadFailed,
    ReadUncertain,
    ParityComputationFailed,
    WriteFailed,
    WriteUncertain,
    FenceIncomplete,
    FenceUncertain,
    RecoveryCleanFailed,
    ReleaseFailed,
    DaemonCrash,
    ReconciliationRequired,
}

impl ErrorClass {
    pub const fn is_uncertain(self) -> bool {
        matches!(
            self,
            Self::WriteRecoveryRecordLost
                | Self::WriteRecoveryRecordCorrupt
                | Self::ReadUncertain
                | Self::WriteUncertain
                | Self::FenceUncertain
                | Self::DaemonCrash
        )
    }
}

impl fmt::Display for ErrorClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::RangeAcquisitionFailed => "range acquisition failed",
            Self::WriteRecoveryRecordRejected => "write-recovery record rejected",
            Self::WriteRecoveryRecordLost => "write-recovery record lost",
            Self::WriteRecoveryRecordCorrupt => "write-recovery record corrupt",
            Self::ReadFailed => "read failed",
            Self::ReadUncertain => "read uncertain",
            Self::ParityComputationFailed => "parity computation failed",
            Self::WriteFailed => "write failed",
            Self::WriteUncertain => "write uncertain",
            Self::FenceIncomplete => "fence incomplete",
            Self::FenceUncertain => "fence uncertain",
            Self::RecoveryCleanFailed => "recovery CLEAN commit failed",
            Self::ReleaseFailed => "release failed",
            Self::DaemonCrash => "daemon crashed",
            Self::ReconciliationRequired => "reconciliation required",
        };
        formatter.write_str(name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanError {
    NoRanges,
    NoStores,
    NoReads,
    NoWrites,
    EmptyRange,
    RangeCountExceeded { actual: usize, maximum: usize },
    ReadCountExceeded { actual: usize, maximum: usize },
    WriteCountExceeded { actual: usize, maximum: usize },
    StoreCountExceeded { actual: usize, maximum: usize },
    ExtentCountExceeded { actual: usize, maximum: usize },
    TraceLimitTooSmall,
    DuplicateStore,
    MissingStore,
    MissingWatermark,
    TopologyGenerationOverflow,
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRanges => write!(formatter, "a transaction plan needs at least one range"),
            Self::NoStores => write!(formatter, "a transaction plan needs at least one store"),
            Self::NoReads => write!(formatter, "a transaction plan needs at least one read"),
            Self::NoWrites => write!(formatter, "a transaction plan needs at least one write"),
            Self::EmptyRange => write!(formatter, "transaction ranges must not be empty"),
            Self::RangeCountExceeded { actual, maximum } => {
                write!(formatter, "range count {actual} exceeds bound {maximum}")
            }
            Self::ReadCountExceeded { actual, maximum } => {
                write!(formatter, "read count {actual} exceeds bound {maximum}")
            }
            Self::WriteCountExceeded { actual, maximum } => {
                write!(formatter, "write count {actual} exceeds bound {maximum}")
            }
            Self::StoreCountExceeded { actual, maximum } => {
                write!(formatter, "store count {actual} exceeds bound {maximum}")
            }
            Self::ExtentCountExceeded { actual, maximum } => {
                write!(formatter, "extent count {actual} exceeds bound {maximum}")
            }
            Self::TraceLimitTooSmall => write!(formatter, "trace bound cannot record the plan"),
            Self::DuplicateStore => write!(formatter, "transaction stores must be unique"),
            Self::MissingStore => write!(formatter, "an action refers to an unplanned store"),
            Self::MissingWatermark => write!(formatter, "every planned store needs a watermark"),
            Self::TopologyGenerationOverflow => {
                write!(formatter, "transaction generation cannot be advanced")
            }
        }
    }
}

impl std::error::Error for PlanError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransactionError {
    InvalidPlan(PlanError),
    NoPendingAction,
    UnexpectedResult {
        expected: ActionKind,
        received: ResultKind,
    },
    InvalidResult(&'static str),
    DuplicateAbandonment,
    AlreadyTerminal,
    DaemonAlreadyCrashed,
    TraceLimitExceeded,
    TopologyMismatch {
        expected: TopologyEpoch,
        actual: TopologyEpoch,
    },
    RecoveryGenerationMismatch {
        expected: RecoveryGeneration,
        actual: RecoveryGeneration,
    },
}

impl fmt::Display for TransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPlan(error) => write!(formatter, "invalid transaction plan: {error}"),
            Self::NoPendingAction => write!(formatter, "transaction has no pending action"),
            Self::UnexpectedResult { expected, received } => {
                write!(
                    formatter,
                    "expected {expected:?} result, received {received:?}"
                )
            }
            Self::InvalidResult(message) => write!(formatter, "invalid action result: {message}"),
            Self::DuplicateAbandonment => write!(formatter, "transaction was already abandoned"),
            Self::AlreadyTerminal => write!(formatter, "transaction is already terminal"),
            Self::DaemonAlreadyCrashed => write!(formatter, "daemon crash was already recorded"),
            Self::TraceLimitExceeded => write!(formatter, "transaction trace limit exceeded"),
            Self::TopologyMismatch { expected, actual } => {
                write!(
                    formatter,
                    "topology mismatch: expected {expected:?}, got {actual:?}"
                )
            }
            Self::RecoveryGenerationMismatch { expected, actual } => write!(
                formatter,
                "recovery generation mismatch: expected {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl std::error::Error for TransactionError {}
