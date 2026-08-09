//! Deterministic volatile-media simulation for portable correctness tests.
//!
//! The simulator is deliberately byte-oriented.  It separates durable media,
//! acknowledged volatile writes, pending operations, and completion delivery;
//! it does not claim to model a particular filesystem, kernel, or device.

use dwv_core::TopologyEpoch;
use dwv_store::{
    ByteRange, CapabilityEvidenceId, CapabilitySupport, ChildOperationId, StoreCapabilities,
    StoreError, StoreId, StoreOperation, StoreWriteWatermark, WriteIntent,
};
#[cfg(test)]
use dwv_store::{CompletionDisposition, PersistenceEvidence};
use std::fmt;

mod checksum_model;
mod envelope_model;
mod media;
mod recovery_model;

pub use media::{
    CompletionFault, Delivery, MediaConfig, MediaEffect, MediaError, MediaSchedule,
    MediaScheduleStep, MediaSimulator, MediaStateSnapshot, MediaTrace, MediaTraceEvent,
    MediaVolatileWriteSnapshot, PowerLossPolicy,
};

pub use envelope_model::{
    EnvelopeCutPoint, EnvelopeModelOutcome, EnvelopeSchedule, ProfileCost, compare_profile_costs,
    run_envelope_schedule, run_interrupted_migration,
};

pub use checksum_model::{
    ChecksumCutPoint, ChecksumModelEvent, ChecksumModelOutcome, ChecksumModelSchedule,
    ChecksumModelState,
};
pub use recovery_model::{
    RecoveryCutPoint, RecoveryModelEvent, RecoveryModelOutcome, RecoveryModelState,
    RecoverySchedule, RecoveryWrite,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FaultModel {
    pub max_pending: usize,
    pub write_completion: CompletionFault,
    pub write_zeroes_completion: CompletionFault,
    pub discard_completion: CompletionFault,
    pub read_completion: CompletionFault,
    pub flush_completion: CompletionFault,
    pub recovery_commit: CompletionFault,
    pub parity_envelope_commit: CompletionFault,
    pub power_loss: PowerLossPolicy,
}

impl Default for FaultModel {
    fn default() -> Self {
        Self {
            max_pending: 64,
            write_completion: CompletionFault::Success,
            write_zeroes_completion: CompletionFault::Success,
            discard_completion: CompletionFault::Success,
            read_completion: CompletionFault::Success,
            flush_completion: CompletionFault::Success,
            recovery_commit: CompletionFault::Success,
            parity_envelope_commit: CompletionFault::Success,
            power_loss: PowerLossPolicy::DiscardVolatile,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulatorConfig {
    pub store_id: StoreId,
    pub topology_epoch: TopologyEpoch,
    pub capabilities: StoreCapabilities,
    pub fault_model: FaultModel,
}

impl SimulatorConfig {
    pub fn for_size(size: usize) -> Self {
        let size = u64::try_from(size).expect("test and simulator images must fit in u64");
        let mut capabilities =
            StoreCapabilities::portable_demo(size, 1, size.max(1), CapabilityEvidenceId(1));
        capabilities.fua = CapabilitySupport::declared();
        capabilities.write_zeroes = CapabilitySupport::declared();
        capabilities.discard = CapabilitySupport::declared();
        Self {
            store_id: StoreId(1),
            topology_epoch: TopologyEpoch(1),
            capabilities,
            fault_model: FaultModel::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScheduleStep {
    SubmitWrite {
        operation_id: ChildOperationId,
        offset: u64,
        bytes: Vec<u8>,
        intent: WriteIntent,
    },
    SubmitRead {
        operation_id: ChildOperationId,
        offset: u64,
        length: u64,
    },
    SubmitWriteZeroes {
        operation_id: ChildOperationId,
        offset: u64,
        length: u64,
        intent: WriteIntent,
    },
    SubmitDiscard {
        operation_id: ChildOperationId,
        offset: u64,
        length: u64,
    },
    SubmitFlush {
        operation_id: ChildOperationId,
        through: StoreWriteWatermark,
    },
    SubmitRecoveryCommit {
        operation_id: ChildOperationId,
        generation: u64,
        bytes: Vec<u8>,
    },
    SubmitParityEnvelopeCommit {
        operation_id: ChildOperationId,
        copy_index: usize,
        generation: u64,
        bytes: Vec<u8>,
    },
    Deliver {
        pending_index: usize,
    },
    DuplicateDelivery {
        delivery_index: usize,
    },
    InjectLatentCorruption {
        offset: u64,
        xor_mask: Vec<u8>,
    },
    DaemonCrash,
    ControllerReset,
    PowerLoss,
    Disappear,
    Reappear,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Schedule {
    steps: Vec<ScheduleStep>,
}

impl Schedule {
    pub fn new(steps: Vec<ScheduleStep>) -> Self {
        Self { steps }
    }

    pub fn push(&mut self, step: ScheduleStep) {
        self.steps.push(step);
    }

    pub fn as_slice(&self) -> &[ScheduleStep] {
        &self.steps
    }

    pub fn to_reproducer(&self) -> String {
        let mut output = String::from("dwv-sim-v1\n");
        for step in &self.steps {
            match step {
                ScheduleStep::SubmitWrite {
                    operation_id,
                    offset,
                    bytes,
                    intent,
                } => {
                    output.push_str(&format!(
                        "W|{}|{}|{}|{}|{}|{}\n",
                        operation_id.slot.index,
                        operation_id.slot.generation,
                        operation_id.index,
                        offset,
                        hex_encode(bytes),
                        encode_intent(*intent)
                    ));
                }
                ScheduleStep::SubmitRead {
                    operation_id,
                    offset,
                    length,
                } => {
                    output.push_str(&format!(
                        "R|{}|{}|{}|{}|{}\n",
                        operation_id.slot.index,
                        operation_id.slot.generation,
                        operation_id.index,
                        offset,
                        length
                    ));
                }
                ScheduleStep::SubmitWriteZeroes {
                    operation_id,
                    offset,
                    length,
                    intent,
                } => {
                    output.push_str(&format!(
                        "Z|{}|{}|{}|{}|{}|{}\n",
                        operation_id.slot.index,
                        operation_id.slot.generation,
                        operation_id.index,
                        offset,
                        length,
                        encode_intent(*intent)
                    ));
                }
                ScheduleStep::SubmitDiscard {
                    operation_id,
                    offset,
                    length,
                } => {
                    output.push_str(&format!(
                        "K|{}|{}|{}|{}|{}\n",
                        operation_id.slot.index,
                        operation_id.slot.generation,
                        operation_id.index,
                        offset,
                        length
                    ));
                }
                ScheduleStep::SubmitFlush {
                    operation_id,
                    through,
                } => {
                    output.push_str(&format!(
                        "F|{}|{}|{}|{}\n",
                        operation_id.slot.index,
                        operation_id.slot.generation,
                        operation_id.index,
                        through.0
                    ));
                }
                ScheduleStep::SubmitRecoveryCommit {
                    operation_id,
                    generation,
                    bytes,
                } => {
                    output.push_str(&format!(
                        "G|{}|{}|{}|{}|{}\n",
                        operation_id.slot.index,
                        operation_id.slot.generation,
                        operation_id.index,
                        generation,
                        hex_encode(bytes)
                    ));
                }
                ScheduleStep::SubmitParityEnvelopeCommit {
                    operation_id,
                    copy_index,
                    generation,
                    bytes,
                } => {
                    output.push_str(&format!(
                        "E|{}|{}|{}|{}|{}|{}\n",
                        operation_id.slot.index,
                        operation_id.slot.generation,
                        operation_id.index,
                        copy_index,
                        generation,
                        hex_encode(bytes)
                    ));
                }
                ScheduleStep::Deliver { pending_index } => {
                    output.push_str(&format!("D|{pending_index}\n"));
                }
                ScheduleStep::DuplicateDelivery { delivery_index } => {
                    output.push_str(&format!("U|{delivery_index}\n"));
                }
                ScheduleStep::InjectLatentCorruption { offset, xor_mask } => {
                    output.push_str(&format!("L|{}|{}\n", offset, hex_encode(xor_mask)));
                }
                ScheduleStep::DaemonCrash => output.push_str("C\n"),
                ScheduleStep::ControllerReset => output.push_str("R\n"),
                ScheduleStep::PowerLoss => output.push_str("P\n"),
                ScheduleStep::Disappear => output.push_str("X\n"),
                ScheduleStep::Reappear => output.push_str("Y\n"),
            }
        }
        output
    }

    pub fn from_reproducer(input: &str) -> Result<Self, ReproducerError> {
        let mut lines = input.lines();
        if lines.next() != Some("dwv-sim-v1") {
            return Err(ReproducerError::InvalidHeader);
        }

        let mut steps = Vec::new();
        for (line_index, line) in lines.enumerate() {
            let line_number = line_index + 2;
            let fields: Vec<&str> = line.split('|').collect();
            let step = match fields.first().copied() {
                Some("W") if fields.len() == 7 => ScheduleStep::SubmitWrite {
                    operation_id: parse_child(&fields[1..4], line_number)?,
                    offset: parse_u64(fields[4], line_number)?,
                    bytes: hex_decode(fields[5], line_number)?,
                    intent: decode_intent(fields[6], line_number)?,
                },
                Some("R") if fields.len() == 6 => ScheduleStep::SubmitRead {
                    operation_id: parse_child(&fields[1..4], line_number)?,
                    offset: parse_u64(fields[4], line_number)?,
                    length: parse_u64(fields[5], line_number)?,
                },
                Some("Z") if fields.len() == 7 => ScheduleStep::SubmitWriteZeroes {
                    operation_id: parse_child(&fields[1..4], line_number)?,
                    offset: parse_u64(fields[4], line_number)?,
                    length: parse_u64(fields[5], line_number)?,
                    intent: decode_intent(fields[6], line_number)?,
                },
                Some("K") if fields.len() == 6 => ScheduleStep::SubmitDiscard {
                    operation_id: parse_child(&fields[1..4], line_number)?,
                    offset: parse_u64(fields[4], line_number)?,
                    length: parse_u64(fields[5], line_number)?,
                },
                Some("F") if fields.len() == 5 => ScheduleStep::SubmitFlush {
                    operation_id: parse_child(&fields[1..4], line_number)?,
                    through: StoreWriteWatermark(parse_u64(fields[4], line_number)?),
                },
                Some("G") if fields.len() == 6 => ScheduleStep::SubmitRecoveryCommit {
                    operation_id: parse_child(&fields[1..4], line_number)?,
                    generation: parse_u64(fields[4], line_number)?,
                    bytes: hex_decode(fields[5], line_number)?,
                },
                Some("E") if fields.len() == 7 => ScheduleStep::SubmitParityEnvelopeCommit {
                    operation_id: parse_child(&fields[1..4], line_number)?,
                    copy_index: parse_usize(fields[4], line_number)?,
                    generation: parse_u64(fields[5], line_number)?,
                    bytes: hex_decode(fields[6], line_number)?,
                },
                Some("D") if fields.len() == 2 => ScheduleStep::Deliver {
                    pending_index: parse_usize(fields[1], line_number)?,
                },
                Some("U") if fields.len() == 2 => ScheduleStep::DuplicateDelivery {
                    delivery_index: parse_usize(fields[1], line_number)?,
                },
                Some("L") if fields.len() == 3 => ScheduleStep::InjectLatentCorruption {
                    offset: parse_u64(fields[1], line_number)?,
                    xor_mask: hex_decode(fields[2], line_number)?,
                },
                Some("C") if fields.len() == 1 => ScheduleStep::DaemonCrash,
                Some("R") if fields.len() == 1 => ScheduleStep::ControllerReset,
                Some("P") if fields.len() == 1 => ScheduleStep::PowerLoss,
                Some("X") if fields.len() == 1 => ScheduleStep::Disappear,
                Some("Y") if fields.len() == 1 => ScheduleStep::Reappear,
                _ => return Err(ReproducerError::InvalidLine(line_number)),
            };
            steps.push(step);
        }
        Ok(Self { steps })
    }

    pub fn minimize<F>(&self, mut reproduces_failure: F) -> Self
    where
        F: FnMut(&Schedule) -> bool,
    {
        let mut current = self.clone();
        let mut index = 0;
        while index < current.steps.len() {
            let mut candidate = current.clone();
            candidate.steps.remove(index);
            if reproduces_failure(&candidate) {
                current = candidate;
            } else {
                index += 1;
            }
        }
        current
    }

    /// Enumerate valid finite prefixes for a small one-range fault search.
    ///
    /// The generated offsets and lengths fit the eight-byte image used by the
    /// portable simulator fixtures. `max_schedules` is a hard bound so callers
    /// cannot accidentally turn a bounded exhaustive test into an unbounded
    /// generator.
    pub fn bounded_one_range(
        max_steps: usize,
        max_schedules: usize,
    ) -> Result<Vec<Self>, ScheduleBoundError> {
        if max_schedules == 0 {
            return Err(ScheduleBoundError::ZeroLimit);
        }
        let mut schedules = Vec::new();
        let mut prefixes = vec![(Self::default(), BoundedState::default())];
        schedules.push(Self::default());

        for _ in 0..max_steps {
            let mut next = Vec::new();
            for (schedule, state) in prefixes {
                for action in bounded_actions(state) {
                    if schedules.len() + next.len() >= max_schedules {
                        return Err(ScheduleBoundError::LimitExceeded {
                            limit: max_schedules,
                        });
                    }
                    let mut candidate = schedule.clone();
                    candidate.push(action.step.clone());
                    let next_state = action.state;
                    schedules.push(candidate.clone());
                    next.push((candidate, next_state));
                }
            }
            prefixes = next;
            if prefixes.is_empty() {
                break;
            }
        }
        Ok(schedules)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleBoundError {
    ZeroLimit,
    LimitExceeded { limit: usize },
}

impl fmt::Display for ScheduleBoundError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit => write!(formatter, "bounded schedule limit must be non-zero"),
            Self::LimitExceeded { limit } => {
                write!(formatter, "bounded schedule limit {limit} was exceeded")
            }
        }
    }
}

impl std::error::Error for ScheduleBoundError {}

#[derive(Clone, Copy, Debug)]
struct BoundedState {
    pending: usize,
    available: bool,
}

impl Default for BoundedState {
    fn default() -> Self {
        Self {
            pending: 0,
            available: true,
        }
    }
}

struct BoundedAction {
    step: ScheduleStep,
    state: BoundedState,
}

fn bounded_actions(state: BoundedState) -> Vec<BoundedAction> {
    let operation_id = ChildOperationId {
        slot: dwv_store::OperationSlotToken::new(0, 1),
        index: state.pending as u32,
    };
    let mut actions = Vec::new();
    if state.available {
        let mut submitted = state;
        submitted.pending += 1;
        actions.push(BoundedAction {
            step: ScheduleStep::SubmitWrite {
                operation_id,
                offset: 0,
                bytes: vec![1, 2],
                intent: WriteIntent::Ordinary,
            },
            state: submitted,
        });
        actions.push(BoundedAction {
            step: ScheduleStep::SubmitWriteZeroes {
                operation_id,
                offset: 0,
                length: 2,
                intent: WriteIntent::Ordinary,
            },
            state: submitted,
        });
        actions.push(BoundedAction {
            step: ScheduleStep::SubmitDiscard {
                operation_id,
                offset: 0,
                length: 2,
            },
            state: submitted,
        });
        actions.push(BoundedAction {
            step: ScheduleStep::SubmitRead {
                operation_id,
                offset: 0,
                length: 2,
            },
            state: submitted,
        });
        actions.push(BoundedAction {
            step: ScheduleStep::SubmitFlush {
                operation_id,
                through: StoreWriteWatermark(1),
            },
            state: submitted,
        });
        actions.push(BoundedAction {
            step: ScheduleStep::SubmitRecoveryCommit {
                operation_id,
                generation: 1,
                bytes: vec![0xaa, 0x55],
            },
            state: submitted,
        });
        actions.push(BoundedAction {
            step: ScheduleStep::SubmitParityEnvelopeCommit {
                operation_id,
                copy_index: 0,
                generation: 1,
                bytes: vec![0x11, 0x22],
            },
            state: submitted,
        });
        let mut disappeared = state;
        disappeared.available = false;
        actions.push(BoundedAction {
            step: ScheduleStep::Disappear,
            state: disappeared,
        });
    } else {
        let mut reappeared = state;
        reappeared.available = true;
        actions.push(BoundedAction {
            step: ScheduleStep::Reappear,
            state: reappeared,
        });
    }
    if state.pending > 0 {
        let mut delivered = state;
        delivered.pending -= 1;
        actions.push(BoundedAction {
            step: ScheduleStep::Deliver { pending_index: 0 },
            state: delivered,
        });
    }
    actions.push(BoundedAction {
        step: ScheduleStep::InjectLatentCorruption {
            offset: 0,
            xor_mask: vec![0xff, 0],
        },
        state,
    });
    let mut cleared = state;
    cleared.pending = 0;
    actions.push(BoundedAction {
        step: ScheduleStep::DaemonCrash,
        state: cleared,
    });
    actions.push(BoundedAction {
        step: ScheduleStep::ControllerReset,
        state: cleared,
    });
    actions.push(BoundedAction {
        step: ScheduleStep::PowerLoss,
        state: cleared,
    });
    actions
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReproducerError {
    InvalidHeader,
    InvalidLine(usize),
    InvalidNumber(usize),
    InvalidHex(usize),
    InvalidIntent(usize),
}

impl fmt::Display for ReproducerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader => write!(formatter, "unsupported simulator reproducer header"),
            Self::InvalidLine(line) => write!(formatter, "invalid reproducer line {line}"),
            Self::InvalidNumber(line) => {
                write!(formatter, "invalid number on reproducer line {line}")
            }
            Self::InvalidHex(line) => {
                write!(formatter, "invalid byte encoding on reproducer line {line}")
            }
            Self::InvalidIntent(line) => {
                write!(formatter, "invalid write intent on reproducer line {line}")
            }
        }
    }
}

impl std::error::Error for ReproducerError {}

fn parse_u64(value: &str, line: usize) -> Result<u64, ReproducerError> {
    value
        .parse()
        .map_err(|_| ReproducerError::InvalidNumber(line))
}

fn parse_usize(value: &str, line: usize) -> Result<usize, ReproducerError> {
    value
        .parse()
        .map_err(|_| ReproducerError::InvalidNumber(line))
}

fn parse_child(fields: &[&str], line: usize) -> Result<ChildOperationId, ReproducerError> {
    if fields.len() != 3 {
        return Err(ReproducerError::InvalidLine(line));
    }
    Ok(ChildOperationId {
        slot: dwv_store::OperationSlotToken::new(
            fields[0]
                .parse()
                .map_err(|_| ReproducerError::InvalidNumber(line))?,
            fields[1]
                .parse()
                .map_err(|_| ReproducerError::InvalidNumber(line))?,
        ),
        index: fields[2]
            .parse()
            .map_err(|_| ReproducerError::InvalidNumber(line))?,
    })
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn hex_decode(value: &str, line: usize) -> Result<Vec<u8>, ReproducerError> {
    let bytes = value.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(ReproducerError::InvalidHex(line));
    }
    let mut output = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = hex_digit(pair[0]).ok_or(ReproducerError::InvalidHex(line))?;
        let low = hex_digit(pair[1]).ok_or(ReproducerError::InvalidHex(line))?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode_intent(intent: WriteIntent) -> char {
    match intent {
        WriteIntent::Ordinary => 'o',
        WriteIntent::Fua => 'f',
        WriteIntent::Preflush => 'p',
        WriteIntent::FuaAndPreflush => 'b',
    }
}

fn decode_intent(value: &str, line: usize) -> Result<WriteIntent, ReproducerError> {
    match value {
        "o" => Ok(WriteIntent::Ordinary),
        "f" => Ok(WriteIntent::Fua),
        "p" => Ok(WriteIntent::Preflush),
        "b" => Ok(WriteIntent::FuaAndPreflush),
        _ => Err(ReproducerError::InvalidIntent(line)),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationError {
    Store(StoreError),
    StoreUnavailable,
    PendingBound { limit: usize },
    PendingIndex { index: usize },
    DeliveryIndex { index: usize },
    EnvelopeIndex { index: usize },
    EmptyCorruptionMask,
    InvalidFault,
    InvalidPowerLoss,
}

impl fmt::Display for SimulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => write!(formatter, "store contract rejected simulation: {error}"),
            Self::StoreUnavailable => write!(formatter, "simulated store is unavailable"),
            Self::PendingBound { limit } => {
                write!(formatter, "pending-operation bound {limit} reached")
            }
            Self::PendingIndex { index } => {
                write!(formatter, "pending operation {index} does not exist")
            }
            Self::DeliveryIndex { index } => write!(formatter, "delivery {index} does not exist"),
            Self::EnvelopeIndex { index } => {
                write!(formatter, "envelope copy {index} does not exist")
            }
            Self::EmptyCorruptionMask => write!(formatter, "latent corruption mask is empty"),
            Self::InvalidFault => write!(
                formatter,
                "fault model is invalid for the requested operation"
            ),
            Self::InvalidPowerLoss => write!(formatter, "power-loss policy is invalid"),
        }
    }
}

impl std::error::Error for SimulationError {}

impl From<StoreError> for SimulationError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}
impl From<MediaError> for SimulationError {
    fn from(error: MediaError) -> Self {
        match error {
            MediaError::Store(error) => Self::Store(error),
            MediaError::StoreUnavailable => Self::StoreUnavailable,
            MediaError::PendingBound { limit } => Self::PendingBound { limit },
            MediaError::PendingIndex { index } => Self::PendingIndex { index },
            MediaError::DeliveryIndex { index } => Self::DeliveryIndex { index },
            MediaError::EmptyCorruptionMask => Self::EmptyCorruptionMask,
            MediaError::InvalidFault => Self::InvalidFault,
            MediaError::InvalidPowerLoss => Self::InvalidPowerLoss,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VolatileWriteSnapshot {
    pub range: ByteRange,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryDatabaseSnapshot {
    pub committed_generation: u64,
    pub durable_bytes: Vec<u8>,
    pub commit_uncertain: bool,
    pub torn: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParityEnvelopeCopySnapshot {
    pub generation: u64,
    pub bytes: Vec<u8>,
    pub valid: bool,
    pub commit_uncertain: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaSnapshot {
    pub store_id: StoreId,
    pub topology_epoch: TopologyEpoch,
    pub durable_media: Vec<u8>,
    pub volatile_acknowledged_writes: Vec<VolatileWriteSnapshot>,
    pub pending_operations: usize,
    pub available: bool,
    pub recovery_database: RecoveryDatabaseSnapshot,
    pub parity_envelope_copies: Vec<ParityEnvelopeCopySnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateCommitKind {
    RecoveryDatabase,
    ParityEnvelope { copy_index: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateDelivery {
    pub operation_id: ChildOperationId,
    pub kind: StateCommitKind,
    pub fault: CompletionFault,
    pub committed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceEvent {
    Submitted {
        operation_id: ChildOperationId,
        operation: StoreOperation,
        range: ByteRange,
    },
    Delivered(Delivery),
    StateCommitSubmitted {
        operation_id: ChildOperationId,
        kind: StateCommitKind,
    },
    StateCommitDelivered(StateDelivery),
    LatentCorruption {
        range: ByteRange,
    },
    DaemonCrashed,
    ControllerReset,
    PowerLost(PowerLossPolicy),
    StoreDisappeared,
    StoreReappeared,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationTrace {
    pub events: Vec<TraceEvent>,
    pub deliveries: Vec<Delivery>,
    pub state_deliveries: Vec<StateDelivery>,
    pub final_snapshot: MediaSnapshot,
}

#[derive(Clone, Debug)]
enum ParentPending {
    Media(crate::media::MediaPendingId),
    Recovery {
        operation_id: ChildOperationId,
        generation: u64,
        bytes: Vec<u8>,
    },
    ParityEnvelope {
        operation_id: ChildOperationId,
        copy_index: usize,
        generation: u64,
        bytes: Vec<u8>,
    },
}

pub struct Simulator {
    media: MediaSimulator,
    pending: Vec<ParentPending>,
    deliveries: Vec<Delivery>,
    state_deliveries: Vec<StateDelivery>,
    events: Vec<TraceEvent>,
    config: SimulatorConfig,
    recovery_database: RecoveryDatabaseSnapshot,
    parity_envelope_copies: Vec<ParityEnvelopeCopySnapshot>,
    media_event_cursor: usize,
}

impl Simulator {
    pub fn new(initial_media: Vec<u8>, config: SimulatorConfig) -> Result<Self, SimulationError> {
        let media = MediaSimulator::new(
            initial_media,
            MediaConfig {
                store_id: config.store_id,
                topology_epoch: config.topology_epoch,
                store_incarnation: dwv_store::StoreIncarnationId(1),
                capabilities: config.capabilities.clone(),
                max_pending: config.fault_model.max_pending,
                write_completion: config.fault_model.write_completion,
                write_zeroes_completion: config.fault_model.write_zeroes_completion,
                discard_completion: config.fault_model.discard_completion,
                read_completion: config.fault_model.read_completion,
                flush_completion: config.fault_model.flush_completion,
                power_loss: config.fault_model.power_loss,
            },
        )?;
        Ok(Self {
            media,
            pending: Vec::new(),
            deliveries: Vec::new(),
            state_deliveries: Vec::new(),
            events: Vec::new(),
            config,
            recovery_database: RecoveryDatabaseSnapshot {
                committed_generation: 0,
                durable_bytes: Vec::new(),
                commit_uncertain: false,
                torn: false,
            },
            parity_envelope_copies: initial_envelopes(),
            media_event_cursor: 0,
        })
    }

    pub fn reset(&mut self) {
        self.media.reset();
        self.pending.clear();
        self.deliveries.clear();
        self.state_deliveries.clear();
        self.events.clear();
        self.recovery_database = RecoveryDatabaseSnapshot {
            committed_generation: 0,
            durable_bytes: Vec::new(),
            commit_uncertain: false,
            torn: false,
        };
        self.parity_envelope_copies = initial_envelopes();
        self.media_event_cursor = 0;
    }

    pub fn run(&mut self, schedule: &Schedule) -> Result<SimulationTrace, SimulationError> {
        self.reset();
        for step in schedule.as_slice() {
            self.apply(step)?;
        }
        self.check_invariants()?;
        Ok(self.trace())
    }

    pub fn apply(&mut self, step: &ScheduleStep) -> Result<(), SimulationError> {
        match step {
            ScheduleStep::SubmitRecoveryCommit {
                operation_id,
                generation,
                bytes,
            } => self.submit_recovery_commit(*operation_id, *generation, bytes),
            ScheduleStep::SubmitParityEnvelopeCommit {
                operation_id,
                copy_index,
                generation,
                bytes,
            } => self.submit_parity_envelope_commit(*operation_id, *copy_index, *generation, bytes),
            ScheduleStep::Deliver { pending_index } => self.deliver(*pending_index),
            ScheduleStep::DaemonCrash => {
                self.apply_media(MediaScheduleStep::DaemonCrash)?;
                self.clear_pending_after_restart();
                Ok(())
            }
            ScheduleStep::ControllerReset => {
                self.apply_media(MediaScheduleStep::ControllerReset)?;
                self.clear_pending_after_restart();
                Ok(())
            }
            ScheduleStep::PowerLoss => {
                self.apply_media(MediaScheduleStep::PowerLoss)?;
                self.clear_pending_after_restart();
                Ok(())
            }
            ScheduleStep::SubmitWrite { .. }
            | ScheduleStep::SubmitRead { .. }
            | ScheduleStep::SubmitWriteZeroes { .. }
            | ScheduleStep::SubmitDiscard { .. }
            | ScheduleStep::SubmitFlush { .. }
            | ScheduleStep::DuplicateDelivery { .. }
            | ScheduleStep::InjectLatentCorruption { .. }
            | ScheduleStep::Disappear
            | ScheduleStep::Reappear => {
                let media_step = to_media_step(step).expect("matched media schedule step");
                self.apply_media(media_step)
            }
        }
    }

    pub fn snapshot(&self) -> MediaSnapshot {
        let media = self.media.snapshot();
        MediaSnapshot {
            store_id: media.store_id,
            topology_epoch: media.topology_epoch,
            durable_media: media.durable_media,
            volatile_acknowledged_writes: media
                .volatile_acknowledged_writes
                .into_iter()
                .map(|write| VolatileWriteSnapshot {
                    range: write.range,
                    bytes: write.bytes,
                })
                .collect(),
            pending_operations: self.pending.len(),
            available: media.available,
            recovery_database: self.recovery_database.clone(),
            parity_envelope_copies: self.parity_envelope_copies.clone(),
        }
    }

    pub fn trace(&self) -> SimulationTrace {
        SimulationTrace {
            events: self.events.clone(),
            deliveries: self.deliveries.clone(),
            state_deliveries: self.state_deliveries.clone(),
            final_snapshot: self.snapshot(),
        }
    }

    pub fn deliveries(&self) -> &[Delivery] {
        &self.deliveries
    }

    pub fn check_invariants(&self) -> Result<(), SimulationError> {
        self.media.check_invariants()?;
        let media_pending = self
            .pending
            .iter()
            .filter(|pending| matches!(pending, ParentPending::Media(_)))
            .count();
        if media_pending != self.media.pending_operations() || self.parity_envelope_copies.len() < 2
        {
            return Err(SimulationError::InvalidPowerLoss);
        }
        for delivery in &self.deliveries {
            delivery.completion.validate()?;
        }
        Ok(())
    }

    /// dwv:req req.normalized-trace-replay.replay-is-deterministic-across-portable-backends
    pub fn deterministic_replay(
        initial_media: Vec<u8>,
        config: SimulatorConfig,
        schedule: &Schedule,
    ) -> Result<bool, SimulationError> {
        let mut first = Self::new(initial_media.clone(), config.clone())?;
        let first_trace = first.run(schedule)?;
        let mut second = Self::new(initial_media, config)?;
        let second_trace = second.run(schedule)?;
        Ok(first_trace == second_trace)
    }

    fn apply_media(&mut self, step: MediaScheduleStep) -> Result<(), SimulationError> {
        let was_submission = matches!(
            step,
            MediaScheduleStep::SubmitWrite { .. }
                | MediaScheduleStep::SubmitRead { .. }
                | MediaScheduleStep::SubmitWriteZeroes { .. }
                | MediaScheduleStep::SubmitDiscard { .. }
                | MediaScheduleStep::SubmitFlush { .. }
        );
        let previous_media_pending = self.media.pending_operations();
        self.media.apply(&step)?;
        self.sync_media();
        if was_submission {
            let pending_id = self
                .media
                .pending_id_at(previous_media_pending)
                .ok_or(SimulationError::InvalidPowerLoss)?;
            self.pending.push(ParentPending::Media(pending_id));
        }
        Ok(())
    }

    fn sync_media(&mut self) {
        let media_events = self.media.events();
        for event in &media_events[self.media_event_cursor..] {
            self.events.push(match event {
                MediaTraceEvent::Submitted {
                    operation_id,
                    operation,
                    range,
                } => TraceEvent::Submitted {
                    operation_id: *operation_id,
                    operation: *operation,
                    range: *range,
                },
                MediaTraceEvent::Delivered(delivery) => TraceEvent::Delivered(delivery.clone()),
                MediaTraceEvent::LatentCorruption { range } => {
                    TraceEvent::LatentCorruption { range: *range }
                }
                MediaTraceEvent::DaemonCrashed => TraceEvent::DaemonCrashed,
                MediaTraceEvent::ControllerReset => TraceEvent::ControllerReset,
                MediaTraceEvent::PowerLost(policy) => TraceEvent::PowerLost(*policy),
                MediaTraceEvent::StoreDisappeared => TraceEvent::StoreDisappeared,
                MediaTraceEvent::StoreReappeared => TraceEvent::StoreReappeared,
            });
        }
        self.media_event_cursor = media_events.len();
        self.deliveries = self.media.deliveries().to_vec();
    }

    fn clear_pending_after_restart(&mut self) {
        self.pending.clear();
    }

    fn submit_recovery_commit(
        &mut self,
        operation_id: ChildOperationId,
        generation: u64,
        bytes: &[u8],
    ) -> Result<(), SimulationError> {
        self.ensure_pending_capacity()?;
        if !self.media.is_available() {
            return Err(SimulationError::StoreUnavailable);
        }
        self.pending.push(ParentPending::Recovery {
            operation_id,
            generation,
            bytes: bytes.to_vec(),
        });
        self.events.push(TraceEvent::StateCommitSubmitted {
            operation_id,
            kind: StateCommitKind::RecoveryDatabase,
        });
        Ok(())
    }

    fn submit_parity_envelope_commit(
        &mut self,
        operation_id: ChildOperationId,
        copy_index: usize,
        generation: u64,
        bytes: &[u8],
    ) -> Result<(), SimulationError> {
        if copy_index >= self.parity_envelope_copies.len() {
            return Err(SimulationError::EnvelopeIndex { index: copy_index });
        }
        self.ensure_pending_capacity()?;
        if !self.media.is_available() {
            return Err(SimulationError::StoreUnavailable);
        }
        self.pending.push(ParentPending::ParityEnvelope {
            operation_id,
            copy_index,
            generation,
            bytes: bytes.to_vec(),
        });
        self.events.push(TraceEvent::StateCommitSubmitted {
            operation_id,
            kind: StateCommitKind::ParityEnvelope { copy_index },
        });
        Ok(())
    }

    fn ensure_pending_capacity(&self) -> Result<(), SimulationError> {
        if self.pending.len() >= self.config.fault_model.max_pending {
            return Err(SimulationError::PendingBound {
                limit: self.config.fault_model.max_pending,
            });
        }
        Ok(())
    }

    fn deliver(&mut self, pending_index: usize) -> Result<(), SimulationError> {
        let pending =
            self.pending
                .get(pending_index)
                .cloned()
                .ok_or(SimulationError::PendingIndex {
                    index: pending_index,
                })?;
        self.pending.remove(pending_index);
        match pending {
            ParentPending::Media(id) => {
                self.media.deliver_pending(id)?;
                self.sync_media();
            }
            ParentPending::Recovery {
                operation_id,
                generation,
                bytes,
            } => self.deliver_recovery_commit(operation_id, generation, &bytes),
            ParentPending::ParityEnvelope {
                operation_id,
                copy_index,
                generation,
                bytes,
            } => {
                self.deliver_parity_envelope_commit(operation_id, copy_index, generation, &bytes)?
            }
        }
        Ok(())
    }

    fn deliver_recovery_commit(
        &mut self,
        operation_id: ChildOperationId,
        generation: u64,
        bytes: &[u8],
    ) {
        let fault = self.config.fault_model.recovery_commit;
        let mut committed = false;
        match fault {
            CompletionFault::Success => {
                self.recovery_database = RecoveryDatabaseSnapshot {
                    committed_generation: generation,
                    durable_bytes: bytes.to_vec(),
                    commit_uncertain: false,
                    torn: false,
                };
                committed = true;
            }
            CompletionFault::Failed(_) => {}
            CompletionFault::Uncertain { .. } => {
                self.recovery_database.commit_uncertain = true;
            }
            CompletionFault::Torn(prefix) => {
                self.recovery_database.durable_bytes = bytes[..prefix.min(bytes.len())].to_vec();
                self.recovery_database.commit_uncertain = true;
                self.recovery_database.torn = true;
            }
            CompletionFault::Short(_) => {
                self.recovery_database.commit_uncertain = true;
            }
        }
        let delivery = StateDelivery {
            operation_id,
            kind: StateCommitKind::RecoveryDatabase,
            fault,
            committed,
        };
        self.events
            .push(TraceEvent::StateCommitDelivered(delivery.clone()));
        self.state_deliveries.push(delivery);
    }

    fn deliver_parity_envelope_commit(
        &mut self,
        operation_id: ChildOperationId,
        copy_index: usize,
        generation: u64,
        bytes: &[u8],
    ) -> Result<(), SimulationError> {
        let copy = self
            .parity_envelope_copies
            .get_mut(copy_index)
            .ok_or(SimulationError::EnvelopeIndex { index: copy_index })?;
        let fault = self.config.fault_model.parity_envelope_commit;
        let mut committed = false;
        match fault {
            CompletionFault::Success => {
                copy.generation = generation;
                copy.bytes = bytes.to_vec();
                copy.valid = true;
                copy.commit_uncertain = false;
                committed = true;
            }
            CompletionFault::Failed(_) => {}
            CompletionFault::Uncertain { .. } => {
                copy.valid = false;
                copy.commit_uncertain = true;
            }
            CompletionFault::Torn(prefix) => {
                copy.bytes = bytes[..prefix.min(bytes.len())].to_vec();
                copy.valid = false;
                copy.commit_uncertain = true;
            }
            CompletionFault::Short(_) => {
                copy.valid = false;
                copy.commit_uncertain = true;
            }
        }
        let delivery = StateDelivery {
            operation_id,
            kind: StateCommitKind::ParityEnvelope { copy_index },
            fault,
            committed,
        };
        self.events
            .push(TraceEvent::StateCommitDelivered(delivery.clone()));
        self.state_deliveries.push(delivery);
        Ok(())
    }
}

fn initial_envelopes() -> Vec<ParityEnvelopeCopySnapshot> {
    vec![
        ParityEnvelopeCopySnapshot {
            generation: 0,
            bytes: Vec::new(),
            valid: true,
            commit_uncertain: false,
        },
        ParityEnvelopeCopySnapshot {
            generation: 0,
            bytes: Vec::new(),
            valid: true,
            commit_uncertain: false,
        },
    ]
}

fn to_media_step(step: &ScheduleStep) -> Option<MediaScheduleStep> {
    Some(match step {
        ScheduleStep::SubmitWrite {
            operation_id,
            offset,
            bytes,
            intent,
        } => MediaScheduleStep::SubmitWrite {
            operation_id: *operation_id,
            offset: *offset,
            bytes: bytes.clone(),
            intent: *intent,
        },
        ScheduleStep::SubmitRead {
            operation_id,
            offset,
            length,
        } => MediaScheduleStep::SubmitRead {
            operation_id: *operation_id,
            offset: *offset,
            length: *length,
        },
        ScheduleStep::SubmitWriteZeroes {
            operation_id,
            offset,
            length,
            intent,
        } => MediaScheduleStep::SubmitWriteZeroes {
            operation_id: *operation_id,
            offset: *offset,
            length: *length,
            intent: *intent,
        },
        ScheduleStep::SubmitDiscard {
            operation_id,
            offset,
            length,
        } => MediaScheduleStep::SubmitDiscard {
            operation_id: *operation_id,
            offset: *offset,
            length: *length,
        },
        ScheduleStep::SubmitFlush {
            operation_id,
            through,
        } => MediaScheduleStep::SubmitFlush {
            operation_id: *operation_id,
            through: *through,
        },
        ScheduleStep::Deliver { pending_index } => MediaScheduleStep::Deliver {
            pending_index: *pending_index,
        },
        ScheduleStep::DuplicateDelivery { delivery_index } => {
            MediaScheduleStep::DuplicateDelivery {
                delivery_index: *delivery_index,
            }
        }
        ScheduleStep::InjectLatentCorruption { offset, xor_mask } => {
            MediaScheduleStep::InjectLatentCorruption {
                offset: *offset,
                xor_mask: xor_mask.clone(),
            }
        }
        ScheduleStep::DaemonCrash => MediaScheduleStep::DaemonCrash,
        ScheduleStep::ControllerReset => MediaScheduleStep::ControllerReset,
        ScheduleStep::PowerLoss => MediaScheduleStep::PowerLoss,
        ScheduleStep::Disappear => MediaScheduleStep::Disappear,
        ScheduleStep::Reappear => MediaScheduleStep::Reappear,
        ScheduleStep::SubmitRecoveryCommit { .. }
        | ScheduleStep::SubmitParityEnvelopeCommit { .. } => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child(index: u32) -> ChildOperationId {
        ChildOperationId {
            slot: dwv_store::OperationSlotToken::new(0, 1),
            index,
        }
    }

    fn write(offset: u64, bytes: &[u8], index: u32) -> ScheduleStep {
        ScheduleStep::SubmitWrite {
            operation_id: child(index),
            offset,
            bytes: bytes.to_vec(),
            intent: WriteIntent::Ordinary,
        }
    }

    #[test]
    fn ordinary_write_is_volatile_until_flush() {
        let mut simulator = Simulator::new(vec![0; 8], SimulatorConfig::for_size(8)).unwrap();
        let schedule = Schedule::new(vec![
            write(0, &[1, 2, 3, 4], 0),
            ScheduleStep::Deliver { pending_index: 0 },
        ]);
        let trace = simulator.run(&schedule).unwrap();
        assert_eq!(trace.final_snapshot.durable_media, vec![0; 8]);
        assert_eq!(trace.final_snapshot.volatile_acknowledged_writes.len(), 1);
        assert_eq!(
            trace.deliveries[0].completion.disposition,
            CompletionDisposition::Success
        );
        assert!(!trace.deliveries[0].completion.persistence.is_durable());
    }

    #[test]
    fn flush_promotes_volatile_write_to_fence_evidence() {
        let mut simulator = Simulator::new(vec![0; 8], SimulatorConfig::for_size(8)).unwrap();
        let schedule = Schedule::new(vec![
            write(0, &[1, 2, 3, 4], 0),
            ScheduleStep::Deliver { pending_index: 0 },
            ScheduleStep::SubmitFlush {
                operation_id: child(1),
                through: StoreWriteWatermark(1),
            },
            ScheduleStep::Deliver { pending_index: 0 },
        ]);
        let trace = simulator.run(&schedule).unwrap();
        assert_eq!(&trace.final_snapshot.durable_media[..4], &[1, 2, 3, 4]);
        assert!(trace.final_snapshot.volatile_acknowledged_writes.is_empty());
        assert!(trace.deliveries[1].completion.persistence.is_durable());
    }

    #[test]
    fn fua_write_is_durable_at_delivery() {
        let mut simulator = Simulator::new(vec![0; 8], SimulatorConfig::for_size(8)).unwrap();
        let schedule = Schedule::new(vec![
            ScheduleStep::SubmitWrite {
                operation_id: child(0),
                offset: 0,
                bytes: vec![7, 6, 5, 4],
                intent: WriteIntent::Fua,
            },
            ScheduleStep::Deliver { pending_index: 0 },
        ]);
        let trace = simulator.run(&schedule).unwrap();
        assert_eq!(&trace.final_snapshot.durable_media[..4], &[7, 6, 5, 4]);
        assert!(trace.final_snapshot.volatile_acknowledged_writes.is_empty());
        assert!(matches!(
            trace.deliveries[0].completion.persistence,
            PersistenceEvidence::DurableByFua { .. }
        ));
    }

    #[test]
    fn short_failed_and_uncertain_effects_are_explicit() {
        let mut config = SimulatorConfig::for_size(8);
        config.fault_model.write_completion = CompletionFault::Short(2);
        let mut simulator = Simulator::new(vec![0; 8], config.clone()).unwrap();
        let short = Schedule::new(vec![
            write(0, &[1, 2, 3, 4], 0),
            ScheduleStep::Deliver { pending_index: 0 },
        ]);
        let trace = simulator.run(&short).unwrap();
        assert_eq!(
            trace.deliveries[0].completion.disposition,
            CompletionDisposition::Short
        );
        assert_eq!(
            trace.deliveries[0].completion.completed.as_slice(),
            &[ByteRange::new(0, 2).unwrap()]
        );
        assert_eq!(
            trace.final_snapshot.volatile_acknowledged_writes[0].bytes,
            vec![1, 2]
        );

        config.fault_model.write_completion = CompletionFault::Failed(5);
        simulator = Simulator::new(vec![0; 8], config.clone()).unwrap();
        let failed = Schedule::new(vec![
            write(0, &[1, 2, 3, 4], 0),
            ScheduleStep::Deliver { pending_index: 0 },
        ]);
        let trace = simulator.run(&failed).unwrap();
        assert!(matches!(
            trace.deliveries[0].completion.disposition,
            CompletionDisposition::Failed(StoreError::BackendFailure { code: 5 })
        ));
        assert_eq!(trace.final_snapshot.durable_media, vec![0; 8]);

        config.fault_model.write_completion = CompletionFault::Uncertain {
            effect: MediaEffect::VolatileAll,
        };
        simulator = Simulator::new(vec![0; 8], config).unwrap();
        let uncertain = Schedule::new(vec![
            write(0, &[1, 2, 3, 4], 0),
            ScheduleStep::Deliver { pending_index: 0 },
        ]);
        let trace = simulator.run(&uncertain).unwrap();
        assert_eq!(
            trace.deliveries[0].completion.disposition,
            CompletionDisposition::Uncertain
        );
        assert!(!trace.deliveries[0].completion.persistence.is_durable());
    }

    #[test]
    fn daemon_crash_and_power_loss_have_distinct_effects() {
        let mut config = SimulatorConfig::for_size(8);
        config.fault_model.power_loss = PowerLossPolicy::DiscardVolatile;
        let mut simulator = Simulator::new(vec![0; 8], config).unwrap();
        let crash = Schedule::new(vec![
            write(0, &[9, 8, 7, 6], 0),
            ScheduleStep::Deliver { pending_index: 0 },
            ScheduleStep::DaemonCrash,
        ]);
        let trace = simulator.run(&crash).unwrap();
        assert_eq!(trace.final_snapshot.durable_media, vec![0; 8]);
        assert_eq!(trace.final_snapshot.volatile_acknowledged_writes.len(), 1);

        let power_loss = Schedule::new(vec![
            write(0, &[9, 8, 7, 6], 0),
            ScheduleStep::Deliver { pending_index: 0 },
            ScheduleStep::PowerLoss,
        ]);
        let trace = simulator.run(&power_loss).unwrap();
        assert_eq!(trace.final_snapshot.durable_media, vec![0; 8]);
        assert!(trace.final_snapshot.volatile_acknowledged_writes.is_empty());
    }

    #[test]
    fn delivery_order_disappearance_and_duplicate_are_visible() {
        let mut simulator = Simulator::new(vec![0; 8], SimulatorConfig::for_size(8)).unwrap();
        let schedule = Schedule::new(vec![
            write(0, &[1, 1, 1, 1], 0),
            write(4, &[2, 2, 2, 2], 1),
            ScheduleStep::Deliver { pending_index: 1 },
            ScheduleStep::Deliver { pending_index: 0 },
            ScheduleStep::DuplicateDelivery { delivery_index: 0 },
            ScheduleStep::Disappear,
            ScheduleStep::Reappear,
        ]);
        let trace = simulator.run(&schedule).unwrap();
        assert_eq!(trace.deliveries[0].completion.operation_id, child(1));
        assert!(trace.deliveries[2].duplicate);
        assert!(trace.final_snapshot.available);
    }

    #[test]
    fn disappearance_rejects_new_submissions_until_reappearance() {
        let mut simulator = Simulator::new(vec![0; 8], SimulatorConfig::for_size(8)).unwrap();
        simulator.apply(&ScheduleStep::Disappear).unwrap();
        assert_eq!(
            simulator.apply(&write(0, &[1, 2, 3, 4], 0)),
            Err(SimulationError::StoreUnavailable)
        );
        simulator.apply(&ScheduleStep::Reappear).unwrap();
        simulator.apply(&write(0, &[1, 2, 3, 4], 0)).unwrap();
    }

    #[test]
    fn reproducer_round_trips_and_minimizes_deterministically() {
        let schedule = Schedule::new(vec![
            ScheduleStep::DaemonCrash,
            write(0, &[1, 2], 0),
            ScheduleStep::Deliver { pending_index: 0 },
            ScheduleStep::PowerLoss,
            ScheduleStep::DaemonCrash,
        ]);
        let encoded = schedule.to_reproducer();
        assert_eq!(Schedule::from_reproducer(&encoded).unwrap(), schedule);

        let minimized = schedule.minimize(|candidate| {
            candidate
                .as_slice()
                .iter()
                .any(|step| matches!(step, ScheduleStep::PowerLoss))
        });
        assert_eq!(minimized.as_slice(), &[ScheduleStep::PowerLoss]);
        assert_eq!(
            Schedule::from_reproducer(&minimized.to_reproducer()).unwrap(),
            minimized
        );
    }

    /// dwv:req req.normalized-trace-replay.replay-is-deterministic-across-portable-backends
    #[test]
    fn replay_is_stable_and_read_sees_acknowledged_volatile_bytes() {
        let mut simulator = Simulator::new(vec![0; 8], SimulatorConfig::for_size(8)).unwrap();
        let schedule = Schedule::new(vec![
            write(0, &[5, 4, 3, 2], 0),
            ScheduleStep::Deliver { pending_index: 0 },
            ScheduleStep::SubmitRead {
                operation_id: child(1),
                offset: 0,
                length: 4,
            },
            ScheduleStep::Deliver { pending_index: 0 },
        ]);
        assert!(
            Simulator::deterministic_replay(vec![0; 8], SimulatorConfig::for_size(8), &schedule)
                .unwrap()
        );
        let trace = simulator.run(&schedule).unwrap();
        assert_eq!(
            trace.deliveries[1].read_data.as_deref(),
            Some(&[5, 4, 3, 2][..])
        );
    }

    #[test]
    fn write_zeroes_and_discard_are_distinct_logical_operations() {
        let mut simulator = Simulator::new(vec![9; 8], SimulatorConfig::for_size(8)).unwrap();
        let schedule = Schedule::new(vec![
            ScheduleStep::SubmitWriteZeroes {
                operation_id: child(0),
                offset: 0,
                length: 4,
                intent: WriteIntent::Fua,
            },
            ScheduleStep::Deliver { pending_index: 0 },
            ScheduleStep::SubmitDiscard {
                operation_id: child(1),
                offset: 4,
                length: 2,
            },
            ScheduleStep::Deliver { pending_index: 0 },
        ]);
        let trace = simulator.run(&schedule).unwrap();
        assert_eq!(
            trace.final_snapshot.durable_media,
            vec![0, 0, 0, 0, 0, 0, 9, 9]
        );
        assert_eq!(
            trace.events[0],
            TraceEvent::Submitted {
                operation_id: child(0),
                operation: StoreOperation::WriteZeroes,
                range: ByteRange::new(0, 4).unwrap(),
            }
        );
        assert_eq!(
            trace.events[2],
            TraceEvent::Submitted {
                operation_id: child(1),
                operation: StoreOperation::Discard,
                range: ByteRange::new(4, 2).unwrap(),
            }
        );
    }

    #[test]
    fn torn_write_is_uncertain_and_only_changes_modeled_prefix() {
        let mut config = SimulatorConfig::for_size(8);
        config.fault_model.write_completion = CompletionFault::Torn(2);
        let mut simulator = Simulator::new(vec![8; 8], config).unwrap();
        let trace = simulator
            .run(&Schedule::new(vec![
                write(0, &[1, 2, 3, 4], 0),
                ScheduleStep::Deliver { pending_index: 0 },
            ]))
            .unwrap();
        assert_eq!(trace.deliveries[0].fault, CompletionFault::Torn(2));
        assert_eq!(
            trace.deliveries[0].completion.disposition,
            CompletionDisposition::Uncertain
        );
        assert_eq!(
            trace.final_snapshot.volatile_acknowledged_writes[0].bytes,
            vec![1, 2]
        );
    }

    #[test]
    fn controller_reset_preserves_media_and_latent_corruption_is_exact() {
        let mut simulator = Simulator::new(vec![1, 2, 3, 4], SimulatorConfig::for_size(4)).unwrap();
        let trace = simulator
            .run(&Schedule::new(vec![
                write(0, &[9, 9], 0),
                ScheduleStep::ControllerReset,
                ScheduleStep::InjectLatentCorruption {
                    offset: 1,
                    xor_mask: vec![0xff, 0],
                },
            ]))
            .unwrap();
        assert_eq!(trace.final_snapshot.durable_media, vec![1, 0xfd, 3, 4]);
        assert!(trace.final_snapshot.volatile_acknowledged_writes.is_empty());
        assert_eq!(trace.final_snapshot.pending_operations, 0);
        assert!(
            trace
                .events
                .iter()
                .any(|event| matches!(event, TraceEvent::ControllerReset))
        );
    }

    #[test]
    fn recovery_failure_and_envelope_tear_are_conservative_per_copy() {
        let mut config = SimulatorConfig::for_size(8);
        config.fault_model.recovery_commit = CompletionFault::Failed(5);
        config.fault_model.parity_envelope_commit = CompletionFault::Torn(1);
        let mut simulator = Simulator::new(vec![0; 8], config).unwrap();
        let trace = simulator
            .run(&Schedule::new(vec![
                ScheduleStep::SubmitRecoveryCommit {
                    operation_id: child(0),
                    generation: 7,
                    bytes: vec![1, 2, 3],
                },
                ScheduleStep::Deliver { pending_index: 0 },
                ScheduleStep::SubmitParityEnvelopeCommit {
                    operation_id: child(1),
                    copy_index: 0,
                    generation: 4,
                    bytes: vec![8, 9],
                },
                ScheduleStep::Deliver { pending_index: 0 },
            ]))
            .unwrap();
        assert_eq!(
            trace.final_snapshot.recovery_database.committed_generation,
            0
        );
        assert!(
            trace
                .final_snapshot
                .recovery_database
                .durable_bytes
                .is_empty()
        );
        assert_eq!(
            trace.final_snapshot.parity_envelope_copies[0].bytes,
            vec![8]
        );
        assert!(!trace.final_snapshot.parity_envelope_copies[0].valid);
        assert!(trace.final_snapshot.parity_envelope_copies[0].commit_uncertain);
        assert_eq!(
            trace.final_snapshot.parity_envelope_copies[1],
            ParityEnvelopeCopySnapshot {
                generation: 0,
                bytes: Vec::new(),
                valid: true,
                commit_uncertain: false,
            }
        );
        assert_eq!(trace.state_deliveries.len(), 2);
        assert!(!trace.state_deliveries[0].committed);
        assert!(!trace.state_deliveries[1].committed);
    }

    #[test]
    fn bounded_schedules_are_capped_serializable_and_replay_stable() {
        let schedules = Schedule::bounded_one_range(2, 512).unwrap();
        assert!(!schedules.is_empty());
        for schedule in &schedules {
            assert_eq!(
                Schedule::from_reproducer(&schedule.to_reproducer()).unwrap(),
                *schedule
            );
            assert!(
                Simulator::deterministic_replay(vec![0; 8], SimulatorConfig::for_size(8), schedule)
                    .unwrap()
            );
        }
        assert_eq!(
            Schedule::bounded_one_range(3, 1),
            Err(ScheduleBoundError::LimitExceeded { limit: 1 })
        );
    }
    #[test]
    fn seeded_operation_and_fault_schedules_replay_stably() {
        for (line, seed_text) in
            include_str!("../../../verification/corpus/simulator-schedule-seeds.txt")
                .lines()
                .enumerate()
        {
            let seed = u64::from_str_radix(seed_text.trim(), 16)
                .unwrap_or_else(|_| panic!("invalid simulator seed on line {}", line + 1));
            let (initial, config, schedule) = seeded_schedule(seed);
            let encoded = schedule.to_reproducer();
            assert_eq!(
                Schedule::from_reproducer(&encoded).unwrap(),
                schedule,
                "seed {seed:#x} must preserve its producer witness"
            );
            assert!(
                Simulator::deterministic_replay(initial.clone(), config.clone(), &schedule)
                    .unwrap(),
                "seed {seed:#x} must replay deterministically"
            );

            let mut simulator = Simulator::new(initial, config).unwrap();
            let trace = simulator.run(&schedule).unwrap();
            assert_eq!(trace.final_snapshot.pending_operations, 0);
            assert!(trace.final_snapshot.available);
        }
    }

    fn seeded_schedule(mut seed: u64) -> (Vec<u8>, SimulatorConfig, Schedule) {
        let size = 8 + (next(&mut seed) as usize % 3) * 4;
        let initial = (0..size).map(|_| next(&mut seed) as u8).collect::<Vec<_>>();
        let mut config = SimulatorConfig::for_size(size);
        config.fault_model.max_pending = 4;
        config.fault_model.write_completion = match next(&mut seed) % 4 {
            0 => CompletionFault::Success,
            1 => CompletionFault::Short(1),
            2 => CompletionFault::Torn(1),
            _ => CompletionFault::Uncertain {
                effect: MediaEffect::VolatileAll,
            },
        };
        config.fault_model.recovery_commit = match next(&mut seed) % 3 {
            0 => CompletionFault::Success,
            1 => CompletionFault::Failed(7),
            _ => CompletionFault::Torn(1),
        };
        config.fault_model.parity_envelope_commit = match next(&mut seed) % 3 {
            0 => CompletionFault::Success,
            1 => CompletionFault::Failed(9),
            _ => CompletionFault::Torn(1),
        };
        config.fault_model.power_loss = match next(&mut seed) % 3 {
            0 => PowerLossPolicy::DiscardVolatile,
            1 => PowerLossPolicy::PersistVolatile,
            _ => PowerLossPolicy::PersistPrefix(2),
        };

        let slot = dwv_store::OperationSlotToken::new(0, 1);
        let mut operation_index = 0;
        let mut pending = 0;
        let mut available = true;
        let mut steps = Vec::new();

        for _ in 0..32 {
            let choice = (next(&mut seed) % 12) as usize;
            match choice {
                0 if pending > 0 && available => {
                    steps.push(ScheduleStep::Deliver { pending_index: 0 });
                    pending -= 1;
                }
                1 if available => {
                    steps.push(ScheduleStep::InjectLatentCorruption {
                        offset: next(&mut seed) % size as u64,
                        xor_mask: vec![next(&mut seed) as u8 | 1],
                    });
                }
                2 => {
                    steps.push(ScheduleStep::DaemonCrash);
                    pending = 0;
                }
                3 => {
                    steps.push(ScheduleStep::ControllerReset);
                    pending = 0;
                }
                4 => {
                    steps.push(ScheduleStep::PowerLoss);
                    pending = 0;
                }
                5 if available => {
                    steps.push(ScheduleStep::Disappear);
                    available = false;
                }
                6 if !available => {
                    steps.push(ScheduleStep::Reappear);
                    available = true;
                }
                _ if available && pending < 4 => {
                    let offset = next(&mut seed) % size as u64;
                    let length = 1 + next(&mut seed) % (size as u64 - offset);
                    let operation_id = ChildOperationId {
                        slot,
                        index: operation_index,
                    };
                    operation_index += 1;
                    let intent = match next(&mut seed) % 4 {
                        0 => WriteIntent::Ordinary,
                        1 => WriteIntent::Fua,
                        2 => WriteIntent::Preflush,
                        _ => WriteIntent::FuaAndPreflush,
                    };
                    match next(&mut seed) % 7 {
                        0 => steps.push(ScheduleStep::SubmitWrite {
                            operation_id,
                            offset,
                            bytes: (0..length).map(|_| next(&mut seed) as u8).collect(),
                            intent,
                        }),
                        1 => steps.push(ScheduleStep::SubmitRead {
                            operation_id,
                            offset,
                            length,
                        }),
                        2 => steps.push(ScheduleStep::SubmitWriteZeroes {
                            operation_id,
                            offset,
                            length,
                            intent,
                        }),
                        3 => steps.push(ScheduleStep::SubmitDiscard {
                            operation_id,
                            offset,
                            length,
                        }),
                        4 => steps.push(ScheduleStep::SubmitFlush {
                            operation_id,
                            through: StoreWriteWatermark(next(&mut seed) % 4),
                        }),
                        5 => steps.push(ScheduleStep::SubmitRecoveryCommit {
                            operation_id,
                            generation: 1 + next(&mut seed) % 4,
                            bytes: vec![next(&mut seed) as u8, next(&mut seed) as u8],
                        }),
                        _ => steps.push(ScheduleStep::SubmitParityEnvelopeCommit {
                            operation_id,
                            copy_index: next(&mut seed) as usize % 2,
                            generation: 1 + next(&mut seed) % 4,
                            bytes: vec![next(&mut seed) as u8, next(&mut seed) as u8],
                        }),
                    }
                    pending += 1;
                }
                _ => {
                    steps.push(ScheduleStep::Reappear);
                    available = true;
                }
            }
        }

        if !available {
            steps.push(ScheduleStep::Reappear);
        }
        while pending > 0 {
            steps.push(ScheduleStep::Deliver { pending_index: 0 });
            pending -= 1;
        }
        (initial, config, Schedule::new(steps))
    }

    fn next(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *seed
    }
}
