//! Role-neutral byte media and completion simulation.
//!
//! This module owns media effects and operation delivery only. The parent
//! simulator owns DiskWeave parity, recovery, envelope, and integrity state.

use dwv_core::TopologyEpoch;

use dwv_store::{
    BufferToken, ByteRange, CapabilityEvidenceId, CapabilitySupport, ChildOperationId,
    CompletedRangeSet, CompletionDisposition, Evidence, FenceId, PersistenceEvidence,
    StoreCapabilities, StoreCompletion, StoreError, StoreId, StoreOperation, StoreRequest,
    StoreRequestKind, StoreWriteWatermark, WriteIntent,
};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaEffect {
    None,
    VolatileAll,
    DurableAll,
    VolatilePrefix(usize),
    DurablePrefix(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerLossPolicy {
    DiscardVolatile,
    PersistVolatile,
    PersistPrefix(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionFault {
    Success,
    Short(usize),
    Torn(usize),
    Failed(u16),
    Uncertain { effect: MediaEffect },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaConfig {
    pub store_id: StoreId,
    pub topology_epoch: TopologyEpoch,
    pub store_incarnation: dwv_store::StoreIncarnationId,
    pub capabilities: StoreCapabilities,
    pub max_pending: usize,
    pub write_completion: CompletionFault,
    pub write_zeroes_completion: CompletionFault,
    pub discard_completion: CompletionFault,
    pub read_completion: CompletionFault,
    pub flush_completion: CompletionFault,
    pub power_loss: PowerLossPolicy,
}

impl MediaConfig {
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
            store_incarnation: dwv_store::StoreIncarnationId(1),
            capabilities,
            max_pending: 64,
            write_completion: CompletionFault::Success,
            write_zeroes_completion: CompletionFault::Success,
            discard_completion: CompletionFault::Success,
            read_completion: CompletionFault::Success,
            flush_completion: CompletionFault::Success,
            power_loss: PowerLossPolicy::DiscardVolatile,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MediaScheduleStep {
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
pub struct MediaSchedule {
    steps: Vec<MediaScheduleStep>,
}

impl MediaSchedule {
    pub fn new(steps: Vec<MediaScheduleStep>) -> Self {
        Self { steps }
    }

    pub fn push(&mut self, step: MediaScheduleStep) {
        self.steps.push(step);
    }

    pub fn as_slice(&self) -> &[MediaScheduleStep] {
        &self.steps
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaVolatileWriteSnapshot {
    pub range: ByteRange,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaStateSnapshot {
    pub store_id: StoreId,
    pub topology_epoch: TopologyEpoch,
    pub durable_media: Vec<u8>,
    pub volatile_acknowledged_writes: Vec<MediaVolatileWriteSnapshot>,
    pub pending_operations: usize,
    pub available: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Delivery {
    pub completion: StoreCompletion,
    pub read_data: Option<Vec<u8>>,
    pub duplicate: bool,
    pub fault: CompletionFault,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MediaTraceEvent {
    Submitted {
        operation_id: ChildOperationId,
        operation: StoreOperation,
        range: ByteRange,
    },
    Delivered(Delivery),
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
pub struct MediaTrace {
    pub events: Vec<MediaTraceEvent>,
    pub deliveries: Vec<Delivery>,
    pub final_snapshot: MediaStateSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MediaError {
    Store(StoreError),
    StoreUnavailable,
    PendingBound { limit: usize },
    PendingIndex { index: usize },
    DeliveryIndex { index: usize },
    EmptyCorruptionMask,
    InvalidFault,
    InvalidPowerLoss,
}

impl fmt::Display for MediaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => write!(
                formatter,
                "store contract rejected media simulation: {error}"
            ),
            Self::StoreUnavailable => formatter.write_str("simulated store is unavailable"),
            Self::PendingBound { limit } => {
                write!(formatter, "pending-operation bound {limit} reached")
            }
            Self::PendingIndex { index } => {
                write!(formatter, "pending operation {index} does not exist")
            }
            Self::DeliveryIndex { index } => write!(formatter, "delivery {index} does not exist"),
            Self::EmptyCorruptionMask => formatter.write_str("latent corruption mask is empty"),
            Self::InvalidFault => {
                formatter.write_str("fault model is invalid for the requested operation")
            }
            Self::InvalidPowerLoss => formatter.write_str("power-loss policy is invalid"),
        }
    }
}

impl std::error::Error for MediaError {}

impl From<StoreError> for MediaError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MediaPendingId(u64);

#[derive(Clone, Debug)]
enum PendingKind {
    Write {
        range: ByteRange,
        bytes: Vec<u8>,
        intent: WriteIntent,
    },
    Read {
        range: ByteRange,
    },
    WriteZeroes {
        range: ByteRange,
        intent: WriteIntent,
    },
    Discard {
        range: ByteRange,
    },
    Flush {
        through: StoreWriteWatermark,
    },
}

#[derive(Clone, Debug)]
struct PendingOperation {
    id: MediaPendingId,
    operation_id: ChildOperationId,
    kind: PendingKind,
}

#[derive(Clone, Debug)]
struct VolatileWrite {
    range: ByteRange,
    bytes: Vec<u8>,
}

pub struct MediaSimulator {
    initial_media: Vec<u8>,
    durable_media: Vec<u8>,
    volatile_acknowledged_writes: Vec<VolatileWrite>,
    pending_operations: Vec<PendingOperation>,
    deliveries: Vec<Delivery>,
    events: Vec<MediaTraceEvent>,
    available: bool,
    config: MediaConfig,
    next_pending_id: u64,
}

impl MediaSimulator {
    pub fn new(initial_media: Vec<u8>, config: MediaConfig) -> Result<Self, MediaError> {
        if let Evidence::Known(length) = config.capabilities.logical_length
            && length != initial_media.len() as u64
        {
            return Err(MediaError::Store(StoreError::RangeOutsideStore {
                end: initial_media.len() as u64,
                length,
            }));
        }
        Ok(Self {
            durable_media: initial_media.clone(),
            initial_media,
            volatile_acknowledged_writes: Vec::new(),
            pending_operations: Vec::new(),
            deliveries: Vec::new(),
            events: Vec::new(),
            available: true,
            config,
            next_pending_id: 0,
        })
    }

    pub fn reset(&mut self) {
        self.durable_media.clone_from(&self.initial_media);
        self.volatile_acknowledged_writes.clear();
        self.pending_operations.clear();
        self.deliveries.clear();
        self.events.clear();
        self.available = true;
        self.next_pending_id = 0;
    }

    pub fn run(&mut self, schedule: &MediaSchedule) -> Result<MediaTrace, MediaError> {
        self.reset();
        for step in schedule.as_slice() {
            self.apply(step)?;
        }
        self.check_invariants()?;
        Ok(self.trace())
    }

    pub fn apply(&mut self, step: &MediaScheduleStep) -> Result<(), MediaError> {
        match step {
            MediaScheduleStep::SubmitWrite {
                operation_id,
                offset,
                bytes,
                intent,
            } => {
                self.submit_write(*operation_id, *offset, bytes, *intent)?;
            }
            MediaScheduleStep::SubmitRead {
                operation_id,
                offset,
                length,
            } => {
                self.submit_read(*operation_id, *offset, *length)?;
            }
            MediaScheduleStep::SubmitWriteZeroes {
                operation_id,
                offset,
                length,
                intent,
            } => {
                self.submit_write_zeroes(*operation_id, *offset, *length, *intent)?;
            }
            MediaScheduleStep::SubmitDiscard {
                operation_id,
                offset,
                length,
            } => {
                self.submit_discard(*operation_id, *offset, *length)?;
            }
            MediaScheduleStep::SubmitFlush {
                operation_id,
                through,
            } => {
                self.submit_flush(*operation_id, *through)?;
            }
            MediaScheduleStep::Deliver { pending_index } => {
                self.deliver_index(*pending_index)?;
            }
            MediaScheduleStep::DuplicateDelivery { delivery_index } => {
                self.duplicate_delivery(*delivery_index)?;
            }
            MediaScheduleStep::InjectLatentCorruption { offset, xor_mask } => {
                self.inject_latent_corruption(*offset, xor_mask)?;
            }
            MediaScheduleStep::DaemonCrash => {
                self.clear_pending();
                self.events.push(MediaTraceEvent::DaemonCrashed);
            }
            MediaScheduleStep::ControllerReset => {
                self.clear_pending();
                self.events.push(MediaTraceEvent::ControllerReset);
            }
            MediaScheduleStep::PowerLoss => {
                self.power_loss()?;
                self.clear_pending();
                self.events
                    .push(MediaTraceEvent::PowerLost(self.config.power_loss));
            }
            MediaScheduleStep::Disappear => {
                self.available = false;
                self.events.push(MediaTraceEvent::StoreDisappeared);
            }
            MediaScheduleStep::Reappear => {
                self.available = true;
                self.events.push(MediaTraceEvent::StoreReappeared);
            }
        }
        Ok(())
    }

    pub fn snapshot(&self) -> MediaStateSnapshot {
        MediaStateSnapshot {
            store_id: self.config.store_id,
            topology_epoch: self.config.topology_epoch,
            durable_media: self.durable_media.clone(),
            volatile_acknowledged_writes: self
                .volatile_acknowledged_writes
                .iter()
                .map(|write| MediaVolatileWriteSnapshot {
                    range: write.range,
                    bytes: write.bytes.clone(),
                })
                .collect(),
            pending_operations: self.pending_operations.len(),
            available: self.available,
        }
    }

    pub fn trace(&self) -> MediaTrace {
        MediaTrace {
            events: self.events.clone(),
            deliveries: self.deliveries.clone(),
            final_snapshot: self.snapshot(),
        }
    }

    pub fn deliveries(&self) -> &[Delivery] {
        &self.deliveries
    }
    pub(crate) fn events(&self) -> &[MediaTraceEvent] {
        &self.events
    }

    pub(crate) fn pending_id_at(&self, index: usize) -> Option<MediaPendingId> {
        self.pending_operations.get(index).map(|pending| pending.id)
    }

    pub fn pending_operations(&self) -> usize {
        self.pending_operations.len()
    }

    pub fn is_available(&self) -> bool {
        self.available
    }

    pub(crate) fn clear_pending(&mut self) {
        self.pending_operations.clear();
    }

    pub(crate) fn submit_write(
        &mut self,
        operation_id: ChildOperationId,
        offset: u64,
        bytes: &[u8],
        intent: WriteIntent,
    ) -> Result<MediaPendingId, MediaError> {
        let range = make_range(offset, bytes.len() as u64)?;
        self.validate_submission(
            operation_id,
            StoreRequestKind::Write { range, intent },
            Some(BufferToken::new(
                operation_id.index,
                operation_id.slot.generation,
            )),
        )?;
        self.ensure_pending_capacity()?;
        let id = self.push_pending(
            operation_id,
            PendingKind::Write {
                range,
                bytes: bytes.to_vec(),
                intent,
            },
        );
        self.events.push(MediaTraceEvent::Submitted {
            operation_id,
            operation: StoreOperation::Write,
            range,
        });
        Ok(id)
    }

    pub(crate) fn submit_read(
        &mut self,
        operation_id: ChildOperationId,
        offset: u64,
        length: u64,
    ) -> Result<MediaPendingId, MediaError> {
        let range = make_range(offset, length)?;
        self.validate_submission(
            operation_id,
            StoreRequestKind::Read { range },
            Some(BufferToken::new(
                operation_id.index,
                operation_id.slot.generation,
            )),
        )?;
        self.ensure_pending_capacity()?;
        let id = self.push_pending(operation_id, PendingKind::Read { range });
        self.events.push(MediaTraceEvent::Submitted {
            operation_id,
            operation: StoreOperation::Read,
            range,
        });
        Ok(id)
    }

    pub(crate) fn submit_write_zeroes(
        &mut self,
        operation_id: ChildOperationId,
        offset: u64,
        length: u64,
        intent: WriteIntent,
    ) -> Result<MediaPendingId, MediaError> {
        let range = make_range(offset, length)?;
        self.validate_submission(
            operation_id,
            StoreRequestKind::WriteZeroes { range, intent },
            None,
        )?;
        self.ensure_pending_capacity()?;
        let id = self.push_pending(operation_id, PendingKind::WriteZeroes { range, intent });
        self.events.push(MediaTraceEvent::Submitted {
            operation_id,
            operation: StoreOperation::WriteZeroes,
            range,
        });
        Ok(id)
    }

    pub(crate) fn submit_discard(
        &mut self,
        operation_id: ChildOperationId,
        offset: u64,
        length: u64,
    ) -> Result<MediaPendingId, MediaError> {
        let range = make_range(offset, length)?;
        self.validate_submission(operation_id, StoreRequestKind::Discard { range }, None)?;
        self.ensure_pending_capacity()?;
        let id = self.push_pending(operation_id, PendingKind::Discard { range });
        self.events.push(MediaTraceEvent::Submitted {
            operation_id,
            operation: StoreOperation::Discard,
            range,
        });
        Ok(id)
    }

    pub(crate) fn submit_flush(
        &mut self,
        operation_id: ChildOperationId,
        through: StoreWriteWatermark,
    ) -> Result<MediaPendingId, MediaError> {
        self.validate_submission(operation_id, StoreRequestKind::Flush { through }, None)?;
        self.ensure_pending_capacity()?;
        let id = self.push_pending(operation_id, PendingKind::Flush { through });
        self.events.push(MediaTraceEvent::Submitted {
            operation_id,
            operation: StoreOperation::Flush,
            range: ByteRange::empty(),
        });
        Ok(id)
    }

    pub(crate) fn deliver_pending(&mut self, id: MediaPendingId) -> Result<Delivery, MediaError> {
        let index = self
            .pending_operations
            .iter()
            .position(|pending| pending.id == id)
            .ok_or(MediaError::PendingIndex {
                index: usize::try_from(id.0).unwrap_or(usize::MAX),
            })?;
        self.deliver_index(index)
    }

    pub(crate) fn power_loss(&mut self) -> Result<(), MediaError> {
        match self.config.power_loss {
            PowerLossPolicy::DiscardVolatile => self.volatile_acknowledged_writes.clear(),
            PowerLossPolicy::PersistVolatile => self.flush_volatile(),
            PowerLossPolicy::PersistPrefix(length) => {
                for write in self.volatile_acknowledged_writes.drain(..) {
                    let length = length.min(write.bytes.len());
                    let start = write.range.offset as usize;
                    self.durable_media[start..start + length]
                        .copy_from_slice(&write.bytes[..length]);
                }
            }
        }
        Ok(())
    }

    pub fn check_invariants(&self) -> Result<(), MediaError> {
        if self.durable_media.len() != self.initial_media.len() {
            return Err(MediaError::InvalidPowerLoss);
        }
        for write in &self.volatile_acknowledged_writes {
            let end = write
                .range
                .offset
                .checked_add(write.range.length)
                .ok_or(MediaError::InvalidPowerLoss)?;
            if end > self.durable_media.len() as u64
                || write.bytes.len() as u64 != write.range.length
            {
                return Err(MediaError::InvalidPowerLoss);
            }
        }
        for delivery in &self.deliveries {
            delivery.completion.validate()?;
        }
        Ok(())
    }

    pub fn deterministic_replay(
        initial_media: Vec<u8>,
        config: MediaConfig,
        schedule: &MediaSchedule,
    ) -> Result<bool, MediaError> {
        let mut first = Self::new(initial_media.clone(), config.clone())?;
        let first_trace = first.run(schedule)?;
        let mut second = Self::new(initial_media, config)?;
        let second_trace = second.run(schedule)?;
        Ok(first_trace == second_trace)
    }

    fn push_pending(
        &mut self,
        operation_id: ChildOperationId,
        kind: PendingKind,
    ) -> MediaPendingId {
        let id = MediaPendingId(self.next_pending_id);
        self.next_pending_id = self.next_pending_id.wrapping_add(1);
        self.pending_operations.push(PendingOperation {
            id,
            operation_id,
            kind,
        });
        id
    }

    fn submit_operation(&self, operation_id: ChildOperationId) -> StoreRequest {
        StoreRequest {
            operation_id: dwv_store::OperationId(u64::from(operation_id.index)),
            store_id: self.config.store_id,
            topology_epoch: self.config.topology_epoch,
            kind: StoreRequestKind::Flush {
                through: StoreWriteWatermark(0),
            },
            buffer: None,
        }
    }

    fn validate_submission(
        &self,
        operation_id: ChildOperationId,
        kind: StoreRequestKind,
        buffer: Option<BufferToken>,
    ) -> Result<(), MediaError> {
        if !self.available {
            return Err(MediaError::StoreUnavailable);
        }
        let mut request = self.submit_operation(operation_id);
        request.kind = kind;
        request.buffer = buffer;
        request.validate(&self.config.capabilities)?;
        Ok(())
    }

    fn ensure_pending_capacity(&self) -> Result<(), MediaError> {
        if self.pending_operations.len() >= self.config.max_pending {
            return Err(MediaError::PendingBound {
                limit: self.config.max_pending,
            });
        }
        Ok(())
    }

    fn deliver_index(&mut self, pending_index: usize) -> Result<Delivery, MediaError> {
        if pending_index >= self.pending_operations.len() {
            return Err(MediaError::PendingIndex {
                index: pending_index,
            });
        }
        if !self.available {
            return Err(MediaError::StoreUnavailable);
        }
        let pending = self.pending_operations.remove(pending_index);
        let delivery = match pending.kind {
            PendingKind::Write {
                range,
                bytes,
                intent,
            } => self.deliver_write(pending.operation_id, range, &bytes, intent)?,
            PendingKind::Read { range } => self.deliver_read(pending.operation_id, range)?,
            PendingKind::WriteZeroes { range, intent } => self.deliver_zeroes(
                pending.operation_id,
                range,
                intent,
                self.config.write_zeroes_completion,
                false,
            )?,
            PendingKind::Discard { range } => self.deliver_zeroes(
                pending.operation_id,
                range,
                WriteIntent::Ordinary,
                self.config.discard_completion,
                true,
            )?,
            PendingKind::Flush { through } => self.deliver_flush(pending.operation_id, through)?,
        };
        self.events
            .push(MediaTraceEvent::Delivered(delivery.clone()));
        self.deliveries.push(delivery.clone());
        Ok(delivery)
    }

    fn inject_latent_corruption(&mut self, offset: u64, xor_mask: &[u8]) -> Result<(), MediaError> {
        if xor_mask.is_empty() {
            return Err(MediaError::EmptyCorruptionMask);
        }
        let range = make_range(offset, xor_mask.len() as u64)?;
        let end = range
            .offset
            .checked_add(range.length)
            .ok_or(MediaError::InvalidPowerLoss)?;
        if end > self.durable_media.len() as u64 {
            return Err(MediaError::Store(StoreError::RangeOutsideStore {
                end,
                length: self.durable_media.len() as u64,
            }));
        }
        let start = range.offset as usize;
        for (byte, mask) in self.durable_media[start..start + xor_mask.len()]
            .iter_mut()
            .zip(xor_mask)
        {
            *byte ^= mask;
        }
        self.events
            .push(MediaTraceEvent::LatentCorruption { range });
        Ok(())
    }

    fn deliver_write(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        bytes: &[u8],
        intent: WriteIntent,
    ) -> Result<Delivery, MediaError> {
        let fault = self.config.write_completion;
        let (disposition, effect, persistence, completed_length) = match fault {
            CompletionFault::Success => {
                let effect = if uses_fua(intent) {
                    MediaEffect::DurableAll
                } else {
                    MediaEffect::VolatileAll
                };
                let persistence = if uses_fua(intent) {
                    PersistenceEvidence::DurableByFua {
                        store_id: self.config.store_id,
                        store_incarnation: self.config.store_incarnation,
                        topology_epoch: self.config.topology_epoch,
                        through: StoreWriteWatermark(u64::from(operation_id.index) + 1),
                    }
                } else {
                    PersistenceEvidence::VolatileOrUnknown
                };
                (
                    CompletionDisposition::Success,
                    effect,
                    persistence,
                    Some(bytes.len()),
                )
            }
            CompletionFault::Short(length) => {
                if length >= bytes.len() {
                    return Err(MediaError::InvalidFault);
                }
                (
                    CompletionDisposition::Short,
                    if uses_fua(intent) {
                        MediaEffect::DurablePrefix(length)
                    } else {
                        MediaEffect::VolatilePrefix(length)
                    },
                    PersistenceEvidence::VolatileOrUnknown,
                    Some(length),
                )
            }
            CompletionFault::Torn(length) => {
                if length > bytes.len() {
                    return Err(MediaError::InvalidFault);
                }
                (
                    CompletionDisposition::Uncertain,
                    if uses_fua(intent) {
                        MediaEffect::DurablePrefix(length)
                    } else {
                        MediaEffect::VolatilePrefix(length)
                    },
                    PersistenceEvidence::VolatileOrUnknown,
                    None,
                )
            }
            CompletionFault::Failed(code) => (
                CompletionDisposition::Failed(StoreError::BackendFailure { code }),
                MediaEffect::None,
                PersistenceEvidence::VolatileOrUnknown,
                None,
            ),
            CompletionFault::Uncertain { effect } => (
                CompletionDisposition::Uncertain,
                effect,
                PersistenceEvidence::VolatileOrUnknown,
                None,
            ),
        };
        self.apply_effect(range, bytes, effect)?;
        let completed = completed_for(range, &disposition, completed_length)?;
        Ok(Delivery {
            completion: StoreCompletion::new(
                operation_id,
                range,
                completed,
                disposition,
                persistence,
            )?,
            read_data: None,
            duplicate: false,
            fault,
        })
    }

    fn deliver_zeroes(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        intent: WriteIntent,
        fault: CompletionFault,
        durable_default: bool,
    ) -> Result<Delivery, MediaError> {
        let length = usize::try_from(range.length).map_err(|_| MediaError::InvalidFault)?;
        let bytes = vec![0; length];
        let (disposition, effect, persistence, completed_length) = match fault {
            CompletionFault::Success => {
                let durable = durable_default || uses_fua(intent);
                (
                    CompletionDisposition::Success,
                    if durable {
                        MediaEffect::DurableAll
                    } else {
                        MediaEffect::VolatileAll
                    },
                    if uses_fua(intent) {
                        PersistenceEvidence::DurableByFua {
                            store_id: self.config.store_id,
                            store_incarnation: self.config.store_incarnation,
                            topology_epoch: self.config.topology_epoch,
                            through: StoreWriteWatermark(u64::from(operation_id.index) + 1),
                        }
                    } else {
                        PersistenceEvidence::VolatileOrUnknown
                    },
                    Some(length),
                )
            }
            CompletionFault::Short(completed) => {
                if completed >= length {
                    return Err(MediaError::InvalidFault);
                }
                let durable = durable_default || uses_fua(intent);
                (
                    CompletionDisposition::Short,
                    if durable {
                        MediaEffect::DurablePrefix(completed)
                    } else {
                        MediaEffect::VolatilePrefix(completed)
                    },
                    PersistenceEvidence::VolatileOrUnknown,
                    Some(completed),
                )
            }
            CompletionFault::Torn(torn_length) => {
                if torn_length > length {
                    return Err(MediaError::InvalidFault);
                }
                let durable = durable_default || uses_fua(intent);
                (
                    CompletionDisposition::Uncertain,
                    if durable {
                        MediaEffect::DurablePrefix(torn_length)
                    } else {
                        MediaEffect::VolatilePrefix(torn_length)
                    },
                    PersistenceEvidence::VolatileOrUnknown,
                    None,
                )
            }
            CompletionFault::Failed(code) => (
                CompletionDisposition::Failed(StoreError::BackendFailure { code }),
                MediaEffect::None,
                PersistenceEvidence::VolatileOrUnknown,
                None,
            ),
            CompletionFault::Uncertain { effect } => (
                CompletionDisposition::Uncertain,
                effect,
                PersistenceEvidence::VolatileOrUnknown,
                None,
            ),
        };
        self.apply_effect(range, &bytes, effect)?;
        let completed = completed_for(range, &disposition, completed_length)?;
        Ok(Delivery {
            completion: StoreCompletion::new(
                operation_id,
                range,
                completed,
                disposition,
                persistence,
            )?,
            read_data: None,
            duplicate: false,
            fault,
        })
    }

    fn deliver_read(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
    ) -> Result<Delivery, MediaError> {
        let fault = self.config.read_completion;
        let view = self.read_view();
        let start = usize::try_from(range.offset).map_err(|_| MediaError::InvalidFault)?;
        let end = start
            .checked_add(usize::try_from(range.length).map_err(|_| MediaError::InvalidFault)?)
            .ok_or(MediaError::InvalidFault)?;
        let (disposition, data, completed_length) = match fault {
            CompletionFault::Success => (
                CompletionDisposition::Success,
                Some(view[start..end].to_vec()),
                Some(range.length as usize),
            ),
            CompletionFault::Short(length) => {
                if length >= range.length as usize {
                    return Err(MediaError::InvalidFault);
                }
                (
                    CompletionDisposition::Short,
                    Some(view[start..start + length].to_vec()),
                    Some(length),
                )
            }
            CompletionFault::Torn(_) => (CompletionDisposition::Uncertain, None, None),
            CompletionFault::Failed(code) => (
                CompletionDisposition::Failed(StoreError::BackendFailure { code }),
                None,
                None,
            ),
            CompletionFault::Uncertain { .. } => (CompletionDisposition::Uncertain, None, None),
        };
        let completed = completed_for(range, &disposition, completed_length)?;
        Ok(Delivery {
            completion: StoreCompletion::new(
                operation_id,
                range,
                completed,
                disposition,
                PersistenceEvidence::VolatileOrUnknown,
            )?,
            read_data: data,
            duplicate: false,
            fault,
        })
    }

    fn deliver_flush(
        &mut self,
        operation_id: ChildOperationId,
        through: StoreWriteWatermark,
    ) -> Result<Delivery, MediaError> {
        let range = ByteRange::empty();
        let fault = self.config.flush_completion;
        let persistence = match fault {
            CompletionFault::Success => {
                self.flush_volatile();
                PersistenceEvidence::DurableByFence {
                    fence: dwv_store::StoreFenceRef {
                        fence_id: FenceId(u64::from(operation_id.index)),
                        store_id: self.config.store_id,
                        store_incarnation: self.config.store_incarnation,
                        topology_epoch: self.config.topology_epoch,
                        through,
                        capability_evidence_id: self.config.capabilities.evidence_id,
                    },
                }
            }
            CompletionFault::Short(_) => return Err(MediaError::InvalidFault),
            CompletionFault::Torn(_) | CompletionFault::Uncertain { .. } => {
                PersistenceEvidence::VolatileOrUnknown
            }
            CompletionFault::Failed(code) => {
                return Ok(Delivery {
                    completion: StoreCompletion::new(
                        operation_id,
                        range,
                        CompletedRangeSet::empty(),
                        CompletionDisposition::Failed(StoreError::BackendFailure { code }),
                        PersistenceEvidence::VolatileOrUnknown,
                    )?,
                    read_data: None,
                    duplicate: false,
                    fault,
                });
            }
        };
        Ok(Delivery {
            completion: StoreCompletion::new(
                operation_id,
                range,
                CompletedRangeSet::empty(),
                if matches!(fault, CompletionFault::Success) {
                    CompletionDisposition::Success
                } else {
                    CompletionDisposition::Uncertain
                },
                persistence,
            )?,
            read_data: None,
            duplicate: false,
            fault,
        })
    }

    fn duplicate_delivery(&mut self, delivery_index: usize) -> Result<(), MediaError> {
        let original =
            self.deliveries
                .get(delivery_index)
                .cloned()
                .ok_or(MediaError::DeliveryIndex {
                    index: delivery_index,
                })?;
        let completion = StoreCompletion::new(
            original.completion.operation_id,
            original.completion.requested,
            original.completion.completed.clone(),
            CompletionDisposition::Duplicate,
            original.completion.persistence,
        )?;
        let duplicate = Delivery {
            completion,
            read_data: original.read_data,
            duplicate: true,
            fault: original.fault,
        };
        self.events
            .push(MediaTraceEvent::Delivered(duplicate.clone()));
        self.deliveries.push(duplicate);
        Ok(())
    }

    fn apply_effect(
        &mut self,
        range: ByteRange,
        bytes: &[u8],
        effect: MediaEffect,
    ) -> Result<(), MediaError> {
        let length = match effect {
            MediaEffect::None => 0,
            MediaEffect::VolatileAll | MediaEffect::DurableAll => bytes.len(),
            MediaEffect::VolatilePrefix(length) | MediaEffect::DurablePrefix(length) => {
                if length > bytes.len() {
                    return Err(MediaError::InvalidFault);
                }
                length
            }
        };
        if length == 0 {
            return Ok(());
        }
        let start = usize::try_from(range.offset).map_err(|_| MediaError::InvalidPowerLoss)?;
        let end = start
            .checked_add(length)
            .ok_or(MediaError::InvalidPowerLoss)?;
        let partial = ByteRange::new(range.offset, length as u64)
            .map_err(|_| MediaError::InvalidPowerLoss)?;
        match effect {
            MediaEffect::DurableAll | MediaEffect::DurablePrefix(_) => {
                self.durable_media[start..end].copy_from_slice(&bytes[..length]);
            }
            MediaEffect::VolatileAll | MediaEffect::VolatilePrefix(_) => {
                self.volatile_acknowledged_writes.push(VolatileWrite {
                    range: partial,
                    bytes: bytes[..length].to_vec(),
                });
            }
            MediaEffect::None => {}
        }
        Ok(())
    }

    fn read_view(&self) -> Vec<u8> {
        let mut view = self.durable_media.clone();
        for write in &self.volatile_acknowledged_writes {
            let start = write.range.offset as usize;
            view[start..start + write.bytes.len()].copy_from_slice(&write.bytes);
        }
        view
    }

    fn flush_volatile(&mut self) {
        for write in self.volatile_acknowledged_writes.drain(..) {
            let start = write.range.offset as usize;
            self.durable_media[start..start + write.bytes.len()].copy_from_slice(&write.bytes);
        }
    }
}

fn make_range(offset: u64, length: u64) -> Result<ByteRange, MediaError> {
    ByteRange::new(offset, length).map_err(|error| {
        MediaError::Store(StoreError::RangeOverflow {
            offset: match error {
                dwv_core::RangeError::Overflow { offset, .. } => offset,
            },
            length,
        })
    })
}

fn uses_fua(intent: WriteIntent) -> bool {
    matches!(intent, WriteIntent::Fua | WriteIntent::FuaAndPreflush)
}

fn completed_for(
    range: ByteRange,
    disposition: &CompletionDisposition,
    completed_length: Option<usize>,
) -> Result<CompletedRangeSet, MediaError> {
    match disposition {
        CompletionDisposition::Success => {
            if range.is_empty() {
                Ok(CompletedRangeSet::empty())
            } else {
                Ok(CompletedRangeSet::new(vec![range])?)
            }
        }
        CompletionDisposition::Short => {
            let length = completed_length.ok_or(MediaError::InvalidFault)?;
            if length == 0 {
                Ok(CompletedRangeSet::empty())
            } else {
                let completed = ByteRange::new(range.offset, length as u64)
                    .map_err(|_| MediaError::InvalidFault)?;
                Ok(CompletedRangeSet::new(vec![completed])?)
            }
        }
        CompletionDisposition::Failed(_)
        | CompletionDisposition::Uncertain
        | CompletionDisposition::Duplicate => Ok(CompletedRangeSet::empty()),
    }
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

    #[test]
    fn media_model_replays_without_protocol_state() {
        let schedule = MediaSchedule::new(vec![
            MediaScheduleStep::SubmitWrite {
                operation_id: child(0),
                offset: 0,
                bytes: vec![1, 2, 3, 4],
                intent: WriteIntent::Ordinary,
            },
            MediaScheduleStep::Deliver { pending_index: 0 },
            MediaScheduleStep::SubmitFlush {
                operation_id: child(1),
                through: StoreWriteWatermark(1),
            },
            MediaScheduleStep::Deliver { pending_index: 0 },
        ]);
        let mut media = MediaSimulator::new(vec![0; 8], MediaConfig::for_size(8)).unwrap();
        let trace = media.run(&schedule).unwrap();

        assert_eq!(trace.final_snapshot.durable_media[..4], [1, 2, 3, 4]);
        assert!(trace.final_snapshot.volatile_acknowledged_writes.is_empty());
        assert_eq!(trace.final_snapshot.pending_operations, 0);
        assert!(
            MediaSimulator::deterministic_replay(vec![0; 8], MediaConfig::for_size(8), &schedule,)
                .unwrap()
        );
    }
}
