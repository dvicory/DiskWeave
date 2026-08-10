//! Portable random-access store and operation-lifetime contracts.
//!
//! This crate deliberately contains no filesystem, kernel, database, or
//! executor implementation.  Adapters provide exact-range completions while
//! callers retain policy over retries, uncertainty, and reconciliation.

use std::fmt;

mod identity;

pub use identity::*;

pub use dwv_core::{
    BlockRequest, BufferToken, ByteRange, FenceDomain, SubmissionSequence, TopologyEpoch,
};

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct OperationId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct StoreId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct CapabilityEvidenceId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct FenceId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct StoreWriteWatermark(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct StoreIncarnationId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FrontendTag(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationSlotToken {
    pub index: u32,
    pub generation: u32,
}

impl OperationSlotToken {
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChildOperationId {
    pub slot: OperationSlotToken,
    pub index: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreOperation {
    Read,
    Write,
    Flush,
    WriteZeroes,
    Discard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteIntent {
    Ordinary,
    Fua,
    Preflush,
    FuaAndPreflush,
}

impl WriteIntent {
    const fn uses_fua(self) -> bool {
        matches!(self, Self::Fua | Self::FuaAndPreflush)
    }

    const fn uses_preflush(self) -> bool {
        matches!(self, Self::Preflush | Self::FuaAndPreflush)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityField {
    LogicalLength,
    LogicalBlockSize,
    PhysicalBlockSize,
    MinimumIoSize,
    MaximumTransfer,
    RequiredAlignment,
    Read,
    Write,
    DurableFlush,
    Fua,
    StableOrdering,
    TornWriteModel,
    VolatileCache,
    WriteZeroes,
    Discard,
    SparseAllocation,
    Cancellation,
    StableIdentity,
    SimulationCertification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreError {
    RangeOverflow {
        offset: u64,
        length: u64,
    },
    EmptyRange {
        operation: StoreOperation,
    },
    MissingBuffer {
        operation: StoreOperation,
    },
    UnexpectedBuffer {
        operation: StoreOperation,
    },
    TransferTooLarge {
        length: u64,
        maximum: u64,
    },
    RangeOutsideStore {
        end: u64,
        length: u64,
    },
    AlignmentViolation {
        offset: u64,
        length: u64,
        alignment: u32,
    },
    BackendFailure {
        code: u16,
    },
    UnsupportedOperation(StoreOperation),
    CapabilityUnavailable {
        field: CapabilityField,
        support: CapabilitySupport,
    },
    CapabilityUnknown(CapabilityField),
    InvalidCompletion(CompletionError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionError {
    CompletedRangeOutsideRequest,
    SuccessfulCompletionIsIncomplete,
    ShortCompletionCoversFullRequest,
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RangeOverflow { offset, length } => {
                write!(
                    formatter,
                    "range overflows: offset={offset} length={length}"
                )
            }
            Self::EmptyRange { operation } => write!(formatter, "{operation:?} requires a range"),
            Self::MissingBuffer { operation } => {
                write!(formatter, "{operation:?} requires a buffer")
            }
            Self::UnexpectedBuffer { operation } => {
                write!(formatter, "{operation:?} cannot carry a buffer")
            }
            Self::TransferTooLarge { length, maximum } => {
                write!(
                    formatter,
                    "transfer length {length} exceeds maximum {maximum}"
                )
            }
            Self::RangeOutsideStore { end, length } => {
                write!(formatter, "range end {end} exceeds store length {length}")
            }
            Self::AlignmentViolation {
                offset,
                length,
                alignment,
            } => write!(
                formatter,
                "range offset={offset} length={length} violates alignment {alignment}"
            ),
            Self::BackendFailure { code } => write!(formatter, "backend failure code {code}"),
            Self::UnsupportedOperation(operation) => {
                write!(formatter, "unsupported store operation {operation:?}")
            }
            Self::CapabilityUnavailable { field, support } => {
                write!(formatter, "capability {field:?} is {support:?}")
            }
            Self::CapabilityUnknown(field) => write!(formatter, "capability {field:?} is unknown"),
            Self::InvalidCompletion(error) => write!(formatter, "invalid completion: {error:?}"),
        }
    }
}

impl std::error::Error for StoreError {}

fn checked_end(range: ByteRange) -> Result<u64, StoreError> {
    range
        .offset
        .checked_add(range.length)
        .ok_or(StoreError::RangeOverflow {
            offset: range.offset,
            length: range.length,
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreRequestKind {
    Read {
        range: ByteRange,
    },
    Write {
        range: ByteRange,
        intent: WriteIntent,
    },
    Flush {
        through: StoreWriteWatermark,
    },
    WriteZeroes {
        range: ByteRange,
        intent: WriteIntent,
    },
    Discard {
        range: ByteRange,
    },
}

impl StoreRequestKind {
    pub const fn operation(self) -> StoreOperation {
        match self {
            Self::Read { .. } => StoreOperation::Read,
            Self::Write { .. } => StoreOperation::Write,
            Self::Flush { .. } => StoreOperation::Flush,
            Self::WriteZeroes { .. } => StoreOperation::WriteZeroes,
            Self::Discard { .. } => StoreOperation::Discard,
        }
    }

    pub const fn range(self) -> ByteRange {
        match self {
            Self::Read { range }
            | Self::Write { range, .. }
            | Self::WriteZeroes { range, .. }
            | Self::Discard { range } => range,
            Self::Flush { .. } => ByteRange::empty(),
        }
    }

    const fn buffer_required(self) -> bool {
        matches!(self, Self::Read { .. } | Self::Write { .. })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoreRequest {
    pub operation_id: OperationId,
    pub store_id: StoreId,
    pub topology_epoch: TopologyEpoch,
    pub kind: StoreRequestKind,
    pub buffer: Option<BufferToken>,
}

impl StoreRequest {
    pub fn validate(&self, capabilities: &StoreCapabilities) -> Result<(), StoreError> {
        let operation = self.kind.operation();
        let range = self.kind.range();
        let end = checked_end(range)?;

        if operation != StoreOperation::Flush && range.length == 0 {
            return Err(StoreError::EmptyRange { operation });
        }
        if self.kind.buffer_required() && self.buffer.is_none() {
            return Err(StoreError::MissingBuffer { operation });
        }
        if !self.kind.buffer_required() && self.buffer.is_some() {
            return Err(StoreError::UnexpectedBuffer { operation });
        }

        if operation != StoreOperation::Flush {
            capabilities.validate_range(range, end)?;
        }

        let support = match operation {
            StoreOperation::Read => capabilities.read,
            StoreOperation::Write => capabilities.write,
            StoreOperation::Flush => capabilities.durable_flush,
            StoreOperation::WriteZeroes => capabilities.write_zeroes,
            StoreOperation::Discard => capabilities.discard,
        };
        require_support(support, operation.capability_field())?;

        let intent = match self.kind {
            StoreRequestKind::Write { intent, .. }
            | StoreRequestKind::WriteZeroes { intent, .. } => Some(intent),
            _ => None,
        };
        if let Some(intent) = intent {
            if intent.uses_fua() {
                require_support(capabilities.fua, CapabilityField::Fua)?;
            }
            if intent.uses_preflush() {
                require_support(capabilities.durable_flush, CapabilityField::DurableFlush)?;
            }
        }
        Ok(())
    }
}

impl StoreOperation {
    const fn capability_field(self) -> CapabilityField {
        match self {
            Self::Read => CapabilityField::Read,
            Self::Write => CapabilityField::Write,
            Self::Flush => CapabilityField::DurableFlush,
            Self::WriteZeroes => CapabilityField::WriteZeroes,
            Self::Discard => CapabilityField::Discard,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedRangeSet {
    ranges: Vec<ByteRange>,
}

impl CompletedRangeSet {
    pub fn new(mut ranges: Vec<ByteRange>) -> Result<Self, StoreError> {
        for range in &ranges {
            checked_end(*range)?;
        }
        ranges.sort_by_key(|range| (range.offset, range.length));
        Ok(Self { ranges })
    }

    pub const fn empty() -> Self {
        Self { ranges: Vec::new() }
    }

    pub fn as_slice(&self) -> &[ByteRange] {
        &self.ranges
    }

    pub fn covers(&self, requested: ByteRange) -> Result<bool, StoreError> {
        let requested_end = checked_end(requested)?;
        if requested.length == 0 {
            return Ok(true);
        }

        let mut cursor = requested.offset;
        for completed in &self.ranges {
            let completed_end = checked_end(*completed)?;
            if completed_end <= cursor {
                continue;
            }
            if completed.offset > cursor {
                return Ok(false);
            }
            cursor = cursor.max(completed_end);
            if cursor >= requested_end {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompletionDisposition {
    Success,
    Short,
    Failed(StoreError),
    Uncertain,
    Duplicate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistenceEvidence {
    VolatileOrUnknown,
    DurableByFua {
        store_id: StoreId,
        topology_epoch: TopologyEpoch,
        store_incarnation: StoreIncarnationId,
        through: StoreWriteWatermark,
    },
    DurableByFence {
        fence: StoreFenceRef,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StoreFenceRef {
    pub fence_id: FenceId,
    pub store_id: StoreId,
    pub topology_epoch: TopologyEpoch,
    pub store_incarnation: StoreIncarnationId,
    pub through: StoreWriteWatermark,
    pub capability_evidence_id: CapabilityEvidenceId,
}

impl PersistenceEvidence {
    pub fn is_durable(self) -> bool {
        !matches!(self, Self::VolatileOrUnknown)
    }

    pub fn covers(
        self,
        store_id: StoreId,
        topology_epoch: TopologyEpoch,
        watermark: StoreWriteWatermark,
        store_incarnation: StoreIncarnationId,
    ) -> bool {
        match self {
            Self::VolatileOrUnknown => false,
            Self::DurableByFua {
                store_id: observed_store,
                store_incarnation: observed_incarnation,
                topology_epoch: observed_epoch,
                through,
            } => {
                observed_store == store_id
                    && observed_incarnation == store_incarnation
                    && observed_epoch == topology_epoch
                    && through.0 >= watermark.0
            }
            Self::DurableByFence { fence } => {
                fence.store_id == store_id
                    && fence.store_incarnation == store_incarnation
                    && fence.topology_epoch == topology_epoch
                    && fence.through.0 >= watermark.0
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreCompletion {
    pub operation_id: ChildOperationId,
    pub requested: ByteRange,
    pub completed: CompletedRangeSet,
    pub disposition: CompletionDisposition,
    pub persistence: PersistenceEvidence,
    pub write_watermark: Option<StoreWriteWatermark>,
}

impl StoreCompletion {
    pub fn new(
        operation_id: ChildOperationId,
        requested: ByteRange,
        completed: CompletedRangeSet,
        disposition: CompletionDisposition,
        persistence: PersistenceEvidence,
    ) -> Result<Self, StoreError> {
        let completion = Self {
            operation_id,
            requested,
            completed,
            disposition,
            persistence,
            write_watermark: None,
        };
        completion.validate()?;
        Ok(completion)
    }

    pub fn with_write_watermark(mut self, watermark: StoreWriteWatermark) -> Self {
        self.write_watermark = Some(watermark);
        self
    }

    pub fn validate(&self) -> Result<(), StoreError> {
        let requested_end = checked_end(self.requested)?;
        for completed in self.completed.as_slice() {
            let completed_end = checked_end(*completed)?;
            let outside = completed.offset < self.requested.offset || completed_end > requested_end;
            if outside {
                return Err(StoreError::InvalidCompletion(
                    CompletionError::CompletedRangeOutsideRequest,
                ));
            }
        }

        match self.disposition {
            CompletionDisposition::Success if !self.completed.covers(self.requested)? => Err(
                StoreError::InvalidCompletion(CompletionError::SuccessfulCompletionIsIncomplete),
            ),
            CompletionDisposition::Short if self.completed.covers(self.requested)? => Err(
                StoreError::InvalidCompletion(CompletionError::ShortCompletionCoversFullRequest),
            ),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceStrength {
    Declared,
    Probed,
    Certified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilitySupport {
    Supported { strength: EvidenceStrength },
    Unsupported,
    Unknown,
}

impl CapabilitySupport {
    pub const fn declared() -> Self {
        Self::Supported {
            strength: EvidenceStrength::Declared,
        }
    }

    pub const fn probed() -> Self {
        Self::Supported {
            strength: EvidenceStrength::Probed,
        }
    }

    pub const fn certified() -> Self {
        Self::Supported {
            strength: EvidenceStrength::Certified,
        }
    }

    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Supported { .. })
    }

    pub const fn is_certified(self) -> bool {
        matches!(
            self,
            Self::Supported {
                strength: EvidenceStrength::Certified
            }
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Evidence<T> {
    Known(T),
    Unknown,
}

impl<T> Evidence<T> {
    fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TornWriteModel {
    Atomic { granularity: u32 },
    MayTear,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VolatileCacheModel {
    None,
    VolatileUntilFlush,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafetyProfile {
    SimulationCertified,
    PortableDemo,
    ProductionReadOnly,
    ProductionWriteSafe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileAuthorization {
    pub profile: SafetyProfile,
    pub capability_evidence_id: CapabilityEvidenceId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileError {
    pub profile: SafetyProfile,
    pub field: CapabilityField,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "profile {:?} lacks required {:?} evidence",
            self.profile, self.field
        )
    }
}

impl std::error::Error for ProfileError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreCapabilities {
    pub logical_length: Evidence<u64>,
    pub logical_block_size: Evidence<u32>,
    pub physical_block_size: Evidence<u32>,
    pub minimum_io_size: Evidence<u32>,
    pub optimal_io_size: Evidence<Option<u32>>,
    pub maximum_transfer: Evidence<u64>,
    pub required_alignment: Evidence<u32>,
    pub read: CapabilitySupport,
    pub write: CapabilitySupport,
    pub durable_flush: CapabilitySupport,
    pub fua: CapabilitySupport,
    pub stable_ordering: CapabilitySupport,
    pub atomic_write_granularity: Evidence<Option<u32>>,
    pub torn_write_model: TornWriteModel,
    pub volatile_cache: VolatileCacheModel,
    pub write_zeroes: CapabilitySupport,
    pub discard: CapabilitySupport,
    pub sparse_allocation: CapabilitySupport,
    pub cancellation: CapabilitySupport,
    pub stable_identity_sources: IdentitySourceSet,
    pub simulation_certification: CapabilitySupport,
    pub evidence_id: CapabilityEvidenceId,
}

impl StoreCapabilities {
    pub fn unknown() -> Self {
        Self {
            logical_length: Evidence::Unknown,
            logical_block_size: Evidence::Unknown,
            physical_block_size: Evidence::Unknown,
            minimum_io_size: Evidence::Unknown,
            optimal_io_size: Evidence::Unknown,
            maximum_transfer: Evidence::Unknown,
            required_alignment: Evidence::Unknown,
            read: CapabilitySupport::Unknown,
            write: CapabilitySupport::Unknown,
            durable_flush: CapabilitySupport::Unknown,
            fua: CapabilitySupport::Unknown,
            stable_ordering: CapabilitySupport::Unknown,
            atomic_write_granularity: Evidence::Unknown,
            torn_write_model: TornWriteModel::Unknown,
            volatile_cache: VolatileCacheModel::Unknown,
            write_zeroes: CapabilitySupport::Unknown,
            discard: CapabilitySupport::Unknown,
            sparse_allocation: CapabilitySupport::Unknown,
            cancellation: CapabilitySupport::Unknown,
            stable_identity_sources: IdentitySourceSet::none(),
            simulation_certification: CapabilitySupport::Unknown,
            evidence_id: CapabilityEvidenceId(0),
        }
    }

    pub fn portable_demo(
        logical_length: u64,
        logical_block_size: u32,
        maximum_transfer: u64,
        evidence_id: CapabilityEvidenceId,
    ) -> Self {
        Self {
            logical_length: Evidence::Known(logical_length),
            logical_block_size: Evidence::Known(logical_block_size),
            physical_block_size: Evidence::Unknown,
            minimum_io_size: Evidence::Known(logical_block_size),
            optimal_io_size: Evidence::Unknown,
            maximum_transfer: Evidence::Known(maximum_transfer),
            required_alignment: Evidence::Known(logical_block_size),
            read: CapabilitySupport::declared(),
            write: CapabilitySupport::declared(),
            durable_flush: CapabilitySupport::declared(),
            fua: CapabilitySupport::Unsupported,
            stable_ordering: CapabilitySupport::declared(),
            atomic_write_granularity: Evidence::Unknown,
            torn_write_model: TornWriteModel::Unknown,
            volatile_cache: VolatileCacheModel::Unknown,
            write_zeroes: CapabilitySupport::Unsupported,
            discard: CapabilitySupport::Unsupported,
            sparse_allocation: CapabilitySupport::Unknown,
            cancellation: CapabilitySupport::Unknown,
            stable_identity_sources: IdentitySourceSet::none(),
            simulation_certification: CapabilitySupport::Unsupported,
            evidence_id,
        }
    }

    fn validate_range(
        self: &StoreCapabilities,
        range: ByteRange,
        end: u64,
    ) -> Result<(), StoreError> {
        let length = match &self.logical_length {
            Evidence::Known(length) => *length,
            Evidence::Unknown => {
                return Err(StoreError::CapabilityUnknown(
                    CapabilityField::LogicalLength,
                ));
            }
        };
        if end > length {
            return Err(StoreError::RangeOutsideStore { end, length });
        }

        let maximum = match &self.maximum_transfer {
            Evidence::Known(maximum) => *maximum,
            Evidence::Unknown => {
                return Err(StoreError::CapabilityUnknown(
                    CapabilityField::MaximumTransfer,
                ));
            }
        };
        if range.length > maximum {
            return Err(StoreError::TransferTooLarge {
                length: range.length,
                maximum,
            });
        }

        let alignment = match &self.required_alignment {
            Evidence::Known(alignment) if *alignment != 0 => *alignment,
            Evidence::Known(_) => {
                return Err(StoreError::CapabilityUnknown(
                    CapabilityField::RequiredAlignment,
                ));
            }
            Evidence::Unknown => {
                return Err(StoreError::CapabilityUnknown(
                    CapabilityField::RequiredAlignment,
                ));
            }
        };
        if !range.offset.is_multiple_of(u64::from(alignment))
            || !range.length.is_multiple_of(u64::from(alignment))
        {
            return Err(StoreError::AlignmentViolation {
                offset: range.offset,
                length: range.length,
                alignment,
            });
        }
        Ok(())
    }

    pub fn authorize(&self, profile: SafetyProfile) -> Result<ProfileAuthorization, ProfileError> {
        let require_known = |condition: bool, field| {
            if condition {
                Ok(())
            } else {
                Err(ProfileError { profile, field })
            }
        };

        match profile {
            SafetyProfile::SimulationCertified => {
                require_support_for_profile(
                    self.simulation_certification,
                    profile,
                    CapabilityField::SimulationCertification,
                )?;
            }
            SafetyProfile::PortableDemo => {
                require_known(
                    self.logical_length.is_known(),
                    CapabilityField::LogicalLength,
                )?;
                require_known(
                    self.logical_block_size.is_known(),
                    CapabilityField::LogicalBlockSize,
                )?;
                require_known(
                    self.maximum_transfer.is_known(),
                    CapabilityField::MaximumTransfer,
                )?;
                require_known(
                    self.required_alignment.is_known(),
                    CapabilityField::RequiredAlignment,
                )?;
                require_support_for_profile(self.read, profile, CapabilityField::Read)?;
                require_support_for_profile(self.write, profile, CapabilityField::Write)?;
            }
            SafetyProfile::ProductionReadOnly => {
                self.authorize(SafetyProfile::PortableDemo)?;
                require_known(
                    self.physical_block_size.is_known(),
                    CapabilityField::PhysicalBlockSize,
                )?;
                require_known(
                    self.minimum_io_size.is_known(),
                    CapabilityField::MinimumIoSize,
                )?;
                require_known(
                    self.stable_identity_sources.has_stable_source(),
                    CapabilityField::StableIdentity,
                )?;
            }
            SafetyProfile::ProductionWriteSafe => {
                self.authorize(SafetyProfile::ProductionReadOnly)?;
                require_support_for_profile(self.write, profile, CapabilityField::Write)?;
                require_certified(self.durable_flush, profile, CapabilityField::DurableFlush)?;
                require_certified(
                    self.stable_ordering,
                    profile,
                    CapabilityField::StableOrdering,
                )?;
                require_known(
                    !matches!(self.torn_write_model, TornWriteModel::Unknown),
                    CapabilityField::TornWriteModel,
                )?;
                require_known(
                    !matches!(self.volatile_cache, VolatileCacheModel::Unknown),
                    CapabilityField::VolatileCache,
                )?;
            }
        }

        Ok(ProfileAuthorization {
            profile,
            capability_evidence_id: self.evidence_id,
        })
    }
}

fn require_support(support: CapabilitySupport, field: CapabilityField) -> Result<(), StoreError> {
    match support {
        CapabilitySupport::Supported { .. } => Ok(()),
        CapabilitySupport::Unsupported => Err(StoreError::CapabilityUnavailable { field, support }),
        CapabilitySupport::Unknown => Err(StoreError::CapabilityUnknown(field)),
    }
}

fn require_support_for_profile(
    support: CapabilitySupport,
    profile: SafetyProfile,
    field: CapabilityField,
) -> Result<(), ProfileError> {
    if support.is_supported() {
        Ok(())
    } else {
        Err(ProfileError { profile, field })
    }
}

fn require_certified(
    support: CapabilitySupport,
    profile: SafetyProfile,
    field: CapabilityField,
) -> Result<(), ProfileError> {
    if support.is_certified() {
        Ok(())
    } else {
        Err(ProfileError { profile, field })
    }
}

pub trait RandomAccessStore {
    fn identity_observations(&self) -> IdentityObservationSet;
    fn capabilities(&self) -> StoreCapabilities;
    fn length(&self) -> u64;

    fn read_at(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        destination: BufferToken,
    ) -> StoreCompletion;

    fn write_at(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        source: BufferToken,
        intent: WriteIntent,
    ) -> StoreCompletion;

    fn flush(
        &mut self,
        operation_id: ChildOperationId,
        through: StoreWriteWatermark,
    ) -> StoreCompletion;

    fn write_zeroes(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        intent: WriteIntent,
    ) -> StoreCompletion;

    fn discard(&mut self, operation_id: ChildOperationId, range: ByteRange) -> StoreCompletion;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    OperationSlots,
    Buffers,
    BackendSubmissions,
    Retries,
    RangeLocks,
    BackgroundWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    pub operation_slots: usize,
    pub buffers: usize,
    pub backend_submissions: usize,
    pub retries: usize,
    pub range_locks: usize,
    pub background_work: usize,
}

impl ResourceLimits {
    pub const fn new(
        operation_slots: usize,
        buffers: usize,
        backend_submissions: usize,
        retries: usize,
        range_locks: usize,
        background_work: usize,
    ) -> Self {
        Self {
            operation_slots,
            buffers,
            backend_submissions,
            retries,
            range_locks,
            background_work,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceUsage {
    pub operation_slots: usize,
    pub buffers: usize,
    pub backend_submissions: usize,
    pub retries: usize,
    pub range_locks: usize,
    pub background_work: usize,
}

impl ResourceUsage {
    const fn empty() -> Self {
        Self {
            operation_slots: 0,
            buffers: 0,
            backend_submissions: 0,
            retries: 0,
            range_locks: 0,
            background_work: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionError {
    Exhausted {
        resource: ResourceKind,
        limit: usize,
        in_use: usize,
    },
    ReleaseUnderflow {
        resource: ResourceKind,
        in_use: usize,
        requested: usize,
    },
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted {
                resource,
                limit,
                in_use,
            } => write!(
                formatter,
                "{resource:?} bound {limit} reached with {in_use} in use"
            ),
            Self::ReleaseUnderflow {
                resource,
                in_use,
                requested,
            } => write!(
                formatter,
                "cannot release {requested} {resource:?} with {in_use} in use"
            ),
        }
    }
}

impl std::error::Error for AdmissionError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmissionController {
    limits: ResourceLimits,
    usage: ResourceUsage,
}

impl AdmissionController {
    pub const fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            usage: ResourceUsage::empty(),
        }
    }

    pub const fn limits(self) -> ResourceLimits {
        self.limits
    }

    pub const fn usage(self) -> ResourceUsage {
        self.usage
    }

    pub fn try_acquire(&mut self, resource: ResourceKind) -> Result<(), AdmissionError> {
        let (usage, limit) = self.usage_limit(resource);
        if *usage >= limit {
            return Err(AdmissionError::Exhausted {
                resource,
                limit,
                in_use: *usage,
            });
        }
        *usage += 1;
        Ok(())
    }

    pub fn release(&mut self, resource: ResourceKind) -> Result<(), AdmissionError> {
        let (usage, _) = self.usage_limit(resource);
        if *usage == 0 {
            return Err(AdmissionError::ReleaseUnderflow {
                resource,
                in_use: 0,
                requested: 1,
            });
        }
        *usage -= 1;
        Ok(())
    }

    fn usage_limit(&mut self, resource: ResourceKind) -> (&mut usize, usize) {
        match resource {
            ResourceKind::OperationSlots => {
                (&mut self.usage.operation_slots, self.limits.operation_slots)
            }
            ResourceKind::Buffers => (&mut self.usage.buffers, self.limits.buffers),
            ResourceKind::BackendSubmissions => (
                &mut self.usage.backend_submissions,
                self.limits.backend_submissions,
            ),
            ResourceKind::Retries => (&mut self.usage.retries, self.limits.retries),
            ResourceKind::RangeLocks => (&mut self.usage.range_locks, self.limits.range_locks),
            ResourceKind::BackgroundWork => {
                (&mut self.usage.background_work, self.limits.background_work)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotState {
    Reserved,
    Submitted,
    PartiallyCompleted,
    Draining,
    CompletionUncertain,
    ReconciliationRequired,
    Reclaimable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DrainState {
    NotRequired,
    Required,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconciliationOutcome {
    Durable,
    UncertainRetained,
    Invalidated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreInvalidationReason {
    Disappeared,
    IdentityChanged,
    GeometryChanged,
    TopologyEpochChanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryEligibility {
    Allowed,
    ReconcileFirst,
    Forbidden,
    CallerDecision,
}

pub const fn retry_eligibility(
    idempotent: bool,
    effect_uncertain: bool,
    duplicate_delivery_possible: bool,
) -> RetryEligibility {
    if effect_uncertain {
        if !idempotent {
            RetryEligibility::Forbidden
        } else if duplicate_delivery_possible {
            RetryEligibility::ReconcileFirst
        } else {
            RetryEligibility::Allowed
        }
    } else if idempotent {
        RetryEligibility::Allowed
    } else {
        RetryEligibility::CallerDecision
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SlotError {
    StaleGeneration {
        token: OperationSlotToken,
    },
    GenerationExhausted {
        index: u32,
    },
    InvalidState {
        token: OperationSlotToken,
        state: SlotState,
    },
    ChildUnknown {
        operation_id: ChildOperationId,
    },
    UnexpectedDuplicate {
        operation_id: ChildOperationId,
    },
    Completion(StoreError),
    Admission(AdmissionError),
    ChildrenNotTerminal,
    DrainIncomplete,
}

impl fmt::Display for SlotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleGeneration { token } => write!(formatter, "stale operation slot {token:?}"),
            Self::GenerationExhausted { index } => {
                write!(formatter, "operation slot {index} generation exhausted")
            }
            Self::InvalidState { token, state } => {
                write!(formatter, "operation slot {token:?} is in state {state:?}")
            }
            Self::ChildUnknown { operation_id } => {
                write!(formatter, "unknown child operation {operation_id:?}")
            }
            Self::UnexpectedDuplicate { operation_id } => {
                write!(
                    formatter,
                    "duplicate completion arrived before terminal completion for {operation_id:?}"
                )
            }
            Self::Completion(error) => write!(formatter, "completion rejected: {error}"),
            Self::Admission(error) => write!(formatter, "admission rejected: {error}"),
            Self::ChildrenNotTerminal => write!(formatter, "child operations are not terminal"),
            Self::DrainIncomplete => write!(formatter, "required backend drain is incomplete"),
        }
    }
}

impl std::error::Error for SlotError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionHandling {
    Accepted,
    DuplicateIgnored,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildOperationSnapshot {
    pub operation_id: ChildOperationId,
    pub requested: ByteRange,
    pub terminal: bool,
    pub duplicate_deliveries: u32,
    pub completion: Option<StoreCompletion>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotSnapshot {
    pub token: OperationSlotToken,
    pub request: BlockRequest,
    pub state: SlotState,
    pub buffers: Vec<BufferToken>,
    pub frontend_tags: Vec<FrontendTag>,
    pub children: Vec<ChildOperationSnapshot>,
    pub submitted_watermark: Option<StoreWriteWatermark>,
    pub drain_state: DrainState,
    pub reconciliation: Option<ReconciliationOutcome>,
    pub terminal_evidence: Vec<StoreCompletion>,
    pub invalidated_by: Option<StoreInvalidationReason>,
    pub abandoned: bool,
    pub retry_count: usize,
}

#[derive(Clone, Debug)]
struct ChildRecord {
    operation_id: ChildOperationId,
    requested: ByteRange,
    terminal: bool,
    duplicate_deliveries: u32,
    completion: Option<StoreCompletion>,
}

#[derive(Clone, Debug)]
struct SlotRecord {
    token: OperationSlotToken,
    request: BlockRequest,
    state: SlotState,
    buffers: Vec<BufferToken>,
    frontend_tags: Vec<FrontendTag>,
    children: Vec<ChildRecord>,
    submitted_watermark: Option<StoreWriteWatermark>,
    drain_state: DrainState,
    reconciliation: Option<ReconciliationOutcome>,
    terminal_evidence: Vec<StoreCompletion>,
    invalidated_by: Option<StoreInvalidationReason>,
    abandoned: bool,
    retry_count: usize,
}

/// dwv:req req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
pub struct OperationSlotTable {
    slots: Vec<Option<SlotRecord>>,
    next_generations: Vec<u32>,
    admission: AdmissionController,
}

impl OperationSlotTable {
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            slots: (0..limits.operation_slots).map(|_| None).collect(),
            next_generations: vec![1; limits.operation_slots],
            admission: AdmissionController::new(limits),
        }
    }

    pub const fn limits(&self) -> ResourceLimits {
        self.admission.limits()
    }

    pub const fn usage(&self) -> ResourceUsage {
        self.admission.usage()
    }

    pub fn reserve(&mut self, request: BlockRequest) -> Result<OperationSlotToken, SlotError> {
        let index = self.slots.iter().position(Option::is_none).ok_or_else(|| {
            SlotError::Admission(AdmissionError::Exhausted {
                resource: ResourceKind::OperationSlots,
                limit: self.limits().operation_slots,
                in_use: self.usage().operation_slots,
            })
        })?;
        let generation = self.next_generations[index];
        let next = generation
            .checked_add(1)
            .ok_or(SlotError::GenerationExhausted {
                index: index as u32,
            })?;
        self.admission
            .try_acquire(ResourceKind::OperationSlots)
            .map_err(SlotError::Admission)?;
        if request.buffer.is_some()
            && let Err(error) = self.admission.try_acquire(ResourceKind::Buffers)
        {
            self.admission
                .release(ResourceKind::OperationSlots)
                .expect("operation slot acquisition must be balanced");
            return Err(SlotError::Admission(error));
        }
        self.next_generations[index] = next;
        let token = OperationSlotToken::new(index as u32, generation);
        self.slots[index] = Some(SlotRecord {
            token,
            request,
            state: SlotState::Reserved,
            buffers: request.buffer.into_iter().collect(),
            frontend_tags: Vec::new(),
            children: Vec::new(),
            submitted_watermark: None,
            drain_state: DrainState::NotRequired,
            reconciliation: None,
            terminal_evidence: Vec::new(),
            invalidated_by: None,
            abandoned: false,
            retry_count: 0,
        });
        Ok(token)
    }

    pub fn attach_frontend_tag(
        &mut self,
        token: OperationSlotToken,
        tag: FrontendTag,
    ) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        if slot.state == SlotState::Reclaimable {
            return Err(SlotError::InvalidState {
                token,
                state: slot.state,
            });
        }
        slot.frontend_tags.push(tag);
        Ok(())
    }

    pub fn register_child(
        &mut self,
        token: OperationSlotToken,
        requested: ByteRange,
    ) -> Result<ChildOperationId, SlotError> {
        checked_end(requested).map_err(SlotError::Completion)?;
        let index = self.active_index(token)?;
        let child_index = {
            let slot = self.slots[index].as_ref().expect("active index has a slot");
            if slot.state == SlotState::Reclaimable {
                return Err(SlotError::InvalidState {
                    token,
                    state: slot.state,
                });
            }
            u32::try_from(slot.children.len()).map_err(|_| {
                SlotError::Admission(AdmissionError::Exhausted {
                    resource: ResourceKind::BackendSubmissions,
                    limit: self.limits().backend_submissions,
                    in_use: self.usage().backend_submissions,
                })
            })?
        };
        self.admission
            .try_acquire(ResourceKind::BackendSubmissions)
            .map_err(SlotError::Admission)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        let operation_id = ChildOperationId {
            slot: token,
            index: child_index,
        };
        slot.children.push(ChildRecord {
            operation_id,
            requested,
            terminal: false,
            duplicate_deliveries: 0,
            completion: None,
        });
        Ok(operation_id)
    }

    pub fn mark_submitted(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        match slot.state {
            SlotState::Reserved => slot.state = SlotState::Submitted,
            SlotState::Submitted => {}
            state => {
                return Err(SlotError::InvalidState { token, state });
            }
        }
        Ok(())
    }

    pub fn record_submitted_watermark(
        &mut self,
        token: OperationSlotToken,
        watermark: StoreWriteWatermark,
    ) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        slot.submitted_watermark = Some(
            slot.submitted_watermark
                .map_or(watermark, |current| current.max(watermark)),
        );
        Ok(())
    }

    pub fn mark_abandoned(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        self.slots[index]
            .as_mut()
            .expect("active index has a slot")
            .abandoned = true;
        Ok(())
    }

    pub fn mark_drain_required(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        if slot.state == SlotState::Reclaimable {
            return Err(SlotError::InvalidState {
                token,
                state: slot.state,
            });
        }
        slot.drain_state = DrainState::Required;
        if slot.children.iter().all(|child| child.terminal) {
            slot.state = if slot.children.iter().any(|child| {
                child.completion.as_ref().is_some_and(|completion| {
                    completion.disposition == CompletionDisposition::Uncertain
                })
            }) {
                SlotState::CompletionUncertain
            } else {
                SlotState::Draining
            };
        }
        Ok(())
    }

    pub fn complete_drain(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        if slot.drain_state != DrainState::Required {
            return Err(SlotError::InvalidState {
                token,
                state: slot.state,
            });
        }
        slot.drain_state = DrainState::Complete;
        if slot.children.iter().all(|child| child.terminal) {
            slot.state = if slot.children.iter().any(|child| {
                child.completion.as_ref().is_some_and(|completion| {
                    completion.disposition == CompletionDisposition::Uncertain
                })
            }) {
                SlotState::CompletionUncertain
            } else {
                SlotState::ReconciliationRequired
            };
        }
        Ok(())
    }

    pub fn apply_completion(
        &mut self,
        token: OperationSlotToken,
        completion: StoreCompletion,
    ) -> Result<CompletionHandling, SlotError> {
        let index = self.active_index(token)?;
        if completion.operation_id.slot != token {
            return Err(SlotError::StaleGeneration { token });
        }
        let child_index = usize::try_from(completion.operation_id.index).map_err(|_| {
            SlotError::ChildUnknown {
                operation_id: completion.operation_id,
            }
        })?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        let child = slot
            .children
            .get_mut(child_index)
            .ok_or(SlotError::ChildUnknown {
                operation_id: completion.operation_id,
            })?;
        if child.operation_id != completion.operation_id {
            return Err(SlotError::ChildUnknown {
                operation_id: completion.operation_id,
            });
        }
        completion.validate().map_err(SlotError::Completion)?;
        if completion.requested != child.requested {
            return Err(SlotError::Completion(StoreError::InvalidCompletion(
                CompletionError::CompletedRangeOutsideRequest,
            )));
        }
        if child.terminal {
            child.duplicate_deliveries = child.duplicate_deliveries.saturating_add(1);
            return Ok(CompletionHandling::DuplicateIgnored);
        }
        if completion.disposition == CompletionDisposition::Duplicate {
            return Err(SlotError::UnexpectedDuplicate {
                operation_id: completion.operation_id,
            });
        }

        child.terminal = true;
        child.completion = Some(completion.clone());
        slot.terminal_evidence.push(completion);
        let all_terminal = slot.children.iter().all(|child| child.terminal);
        if all_terminal {
            let uncertain = slot.children.iter().any(|child| {
                child.completion.as_ref().is_some_and(|completion| {
                    completion.disposition == CompletionDisposition::Uncertain
                })
            });
            slot.state = if uncertain {
                SlotState::CompletionUncertain
            } else if slot.drain_state == DrainState::Required {
                SlotState::Draining
            } else {
                SlotState::ReconciliationRequired
            };
        } else {
            slot.state = SlotState::PartiallyCompleted;
        }
        Ok(CompletionHandling::Accepted)
    }

    pub fn record_reconciliation(
        &mut self,
        token: OperationSlotToken,
        outcome: ReconciliationOutcome,
    ) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        if !slot.children.iter().all(|child| child.terminal) {
            return Err(SlotError::ChildrenNotTerminal);
        }
        if slot.drain_state == DrainState::Required {
            return Err(SlotError::DrainIncomplete);
        }
        slot.reconciliation = Some(outcome);
        slot.state = SlotState::Reclaimable;
        Ok(())
    }

    pub fn invalidate(
        &mut self,
        token: OperationSlotToken,
        reason: StoreInvalidationReason,
    ) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        slot.invalidated_by = Some(reason);
        slot.reconciliation = None;
        slot.state = SlotState::CompletionUncertain;
        Ok(())
    }

    pub fn record_retry(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        self.admission
            .try_acquire(ResourceKind::Retries)
            .map_err(SlotError::Admission)?;
        self.slots[index]
            .as_mut()
            .expect("active index has a slot")
            .retry_count += 1;
        Ok(())
    }

    pub fn snapshot(&self, token: OperationSlotToken) -> Result<SlotSnapshot, SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_ref().expect("active index has a slot");
        Ok(SlotSnapshot {
            token: slot.token,
            request: slot.request,
            state: slot.state,
            buffers: slot.buffers.clone(),
            frontend_tags: slot.frontend_tags.clone(),
            children: slot
                .children
                .iter()
                .map(|child| ChildOperationSnapshot {
                    operation_id: child.operation_id,
                    requested: child.requested,
                    terminal: child.terminal,
                    duplicate_deliveries: child.duplicate_deliveries,
                    completion: child.completion.clone(),
                })
                .collect(),
            submitted_watermark: slot.submitted_watermark,
            drain_state: slot.drain_state,
            reconciliation: slot.reconciliation,
            terminal_evidence: slot.terminal_evidence.clone(),
            invalidated_by: slot.invalidated_by,
            abandoned: slot.abandoned,
            retry_count: slot.retry_count,
        })
    }

    pub fn release(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_ref().expect("active index has a slot");
        if slot.state != SlotState::Reclaimable {
            return Err(SlotError::InvalidState {
                token,
                state: slot.state,
            });
        }
        let slot = self.slots[index].take().expect("active index has a slot");
        self.admission
            .release(ResourceKind::OperationSlots)
            .map_err(SlotError::Admission)?;
        for _ in slot.buffers {
            self.admission
                .release(ResourceKind::Buffers)
                .map_err(SlotError::Admission)?;
        }
        for _ in slot.children {
            self.admission
                .release(ResourceKind::BackendSubmissions)
                .map_err(SlotError::Admission)?;
        }
        for _ in 0..slot.retry_count {
            self.admission
                .release(ResourceKind::Retries)
                .map_err(SlotError::Admission)?;
        }
        Ok(())
    }

    fn active_index(&self, token: OperationSlotToken) -> Result<usize, SlotError> {
        let index = usize::try_from(token.index).ok();
        let Some(index) = index else {
            return Err(SlotError::StaleGeneration { token });
        };
        let Some(slot) = self.slots.get(index).and_then(Option::as_ref) else {
            return Err(SlotError::StaleGeneration { token });
        };
        if slot.token != token {
            return Err(SlotError::StaleGeneration { token });
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_core::{BlockOp, DurabilityIntent, FrontendId, OrderingIntent, RequestId, SlotId};

    fn request(request_id: u64, epoch: u64) -> BlockRequest {
        BlockRequest::new(
            RequestId(request_id),
            FrontendId(7),
            SlotId::from_bytes([9; 16]),
            TopologyEpoch(epoch),
            BlockOp::Read,
            RANGE,
            Some(BufferToken::new(3, 4)),
            OrderingIntent {
                submission_sequence: SubmissionSequence(11),
                preflush: false,
                fence_domain: FenceDomain(12),
            },
            DurabilityIntent::Ordinary,
        )
    }

    const RANGE: ByteRange = ByteRange {
        offset: 0,
        length: 4,
    };

    fn child(index: u32) -> ChildOperationId {
        ChildOperationId {
            slot: OperationSlotToken::new(0, 1),
            index,
        }
    }

    fn full_success(operation_id: ChildOperationId) -> StoreCompletion {
        StoreCompletion::new(
            operation_id,
            RANGE,
            CompletedRangeSet::new(vec![RANGE]).unwrap(),
            CompletionDisposition::Success,
            PersistenceEvidence::VolatileOrUnknown,
        )
        .unwrap()
    }

    fn limits() -> ResourceLimits {
        ResourceLimits::new(1, 1, 1, 1, 1, 1)
    }

    #[test]
    fn exact_requests_and_capability_limits_are_enforced() {
        let capabilities = StoreCapabilities::portable_demo(64, 4, 32, CapabilityEvidenceId(7));
        let request = StoreRequest {
            operation_id: OperationId(1),
            store_id: StoreId(2),
            topology_epoch: TopologyEpoch(3),
            kind: StoreRequestKind::Write {
                range: RANGE,
                intent: WriteIntent::Ordinary,
            },
            buffer: Some(BufferToken::new(0, 1)),
        };
        assert!(request.validate(&capabilities).is_ok());

        let unaligned = StoreRequest {
            kind: StoreRequestKind::Read {
                range: ByteRange::new(1, 4).unwrap(),
            },
            buffer: Some(BufferToken::new(0, 1)),
            ..request
        };
        assert!(matches!(
            unaligned.validate(&capabilities),
            Err(StoreError::AlignmentViolation { .. })
        ));

        let discard = StoreRequest {
            kind: StoreRequestKind::Discard { range: RANGE },
            buffer: None,
            ..request
        };
        assert!(matches!(
            discard.validate(&capabilities),
            Err(StoreError::CapabilityUnavailable {
                field: CapabilityField::Discard,
                ..
            })
        ));
    }

    #[test]
    fn profiles_refuse_unknown_or_uncertified_durability() {
        let mut capabilities = StoreCapabilities::portable_demo(64, 4, 32, CapabilityEvidenceId(9));
        assert!(capabilities.authorize(SafetyProfile::PortableDemo).is_ok());
        assert!(
            capabilities
                .authorize(SafetyProfile::ProductionReadOnly)
                .is_err()
        );

        capabilities.physical_block_size = Evidence::Known(4096);
        capabilities.stable_identity_sources.stable_device_id = true;
        assert!(
            capabilities
                .authorize(SafetyProfile::ProductionReadOnly)
                .is_ok()
        );
        capabilities.durable_flush = CapabilitySupport::probed();
        capabilities.stable_ordering = CapabilitySupport::certified();
        capabilities.write = CapabilitySupport::certified();
        capabilities.torn_write_model = TornWriteModel::MayTear;
        capabilities.volatile_cache = VolatileCacheModel::VolatileUntilFlush;
        assert!(
            capabilities
                .authorize(SafetyProfile::ProductionWriteSafe)
                .is_err()
        );
        capabilities.durable_flush = CapabilitySupport::certified();
        assert!(
            capabilities
                .authorize(SafetyProfile::ProductionWriteSafe)
                .is_ok()
        );
    }

    #[test]
    fn completion_preserves_short_and_uncertain_evidence() {
        let short_range = ByteRange::new(0, 2).unwrap();
        let short = StoreCompletion::new(
            child(0),
            RANGE,
            CompletedRangeSet::new(vec![short_range]).unwrap(),
            CompletionDisposition::Short,
            PersistenceEvidence::VolatileOrUnknown,
        )
        .unwrap();
        assert_eq!(short.completed.as_slice(), &[short_range]);
        assert_eq!(short.disposition, CompletionDisposition::Short);

        let uncertain = StoreCompletion::new(
            child(1),
            RANGE,
            CompletedRangeSet::empty(),
            CompletionDisposition::Uncertain,
            PersistenceEvidence::VolatileOrUnknown,
        )
        .unwrap();
        assert!(!uncertain.persistence.is_durable());
    }

    #[test]
    fn fake_adapter_contract_keeps_exact_completion_identity() {
        struct FakeAdapter {
            capabilities: StoreCapabilities,
        }

        impl RandomAccessStore for FakeAdapter {
            fn identity_observations(&self) -> IdentityObservationSet {
                IdentityObservationSet::new(Vec::new(), IdentityAssessment::Unknown)
            }

            fn capabilities(&self) -> StoreCapabilities {
                self.capabilities.clone()
            }

            fn length(&self) -> u64 {
                64
            }

            fn read_at(
                &mut self,
                operation_id: ChildOperationId,
                range: ByteRange,
                _destination: BufferToken,
            ) -> StoreCompletion {
                full_success_for(operation_id, range)
            }

            fn write_at(
                &mut self,
                operation_id: ChildOperationId,
                range: ByteRange,
                _source: BufferToken,
                _intent: WriteIntent,
            ) -> StoreCompletion {
                full_success_for(operation_id, range)
            }

            fn flush(
                &mut self,
                operation_id: ChildOperationId,
                _through: StoreWriteWatermark,
            ) -> StoreCompletion {
                full_success_for(operation_id, ByteRange::empty())
            }

            fn write_zeroes(
                &mut self,
                operation_id: ChildOperationId,
                range: ByteRange,
                _intent: WriteIntent,
            ) -> StoreCompletion {
                full_success_for(operation_id, range)
            }

            fn discard(
                &mut self,
                operation_id: ChildOperationId,
                range: ByteRange,
            ) -> StoreCompletion {
                full_success_for(operation_id, range)
            }
        }

        let mut adapter = FakeAdapter {
            capabilities: StoreCapabilities::portable_demo(64, 4, 32, CapabilityEvidenceId(1)),
        };
        fn_adapter(&mut adapter);

        fn fn_adapter(adapter: &mut FakeAdapter) {
            let completion = adapter.read_at(child(3), RANGE, BufferToken::new(0, 1));
            assert_eq!(completion.operation_id, child(3));
            assert_eq!(completion.completed.as_slice(), &[RANGE]);
        }
    }

    #[test]
    fn stale_generations_and_duplicate_delivery_are_safe() {
        let mut table = OperationSlotTable::new(limits());
        let token = table.reserve(request(1, 1)).unwrap();
        let operation_id = table.register_child(token, RANGE).unwrap();
        assert_eq!(table.snapshot(token).unwrap().request, request(1, 1));
        table.mark_submitted(token).unwrap();
        let completion = full_success(operation_id);
        assert_eq!(
            table.apply_completion(token, completion.clone()).unwrap(),
            CompletionHandling::Accepted
        );
        assert_eq!(
            table.apply_completion(token, completion).unwrap(),
            CompletionHandling::DuplicateIgnored
        );
        assert_eq!(
            table.snapshot(token).unwrap().children[0].duplicate_deliveries,
            1
        );
        table
            .record_reconciliation(token, ReconciliationOutcome::Durable)
            .unwrap();
        table.release(token).unwrap();

        let replacement = table.reserve(request(2, 2)).unwrap();
        assert_ne!(token.generation, replacement.generation);
        assert!(matches!(
            table.apply_completion(
                token,
                full_success(ChildOperationId {
                    slot: token,
                    index: 0
                })
            ),
            Err(SlotError::StaleGeneration { .. })
        ));
    }

    #[test]
    fn equal_numeric_request_ids_from_distinct_frontends_do_not_collide() {
        let mut table = OperationSlotTable::new(ResourceLimits::new(2, 2, 1, 1, 1, 1));
        let first_request = request(42, 1);
        let second_request = BlockRequest {
            frontend_id: FrontendId(8),
            buffer: Some(BufferToken::new(4, 4)),
            ..first_request
        };
        let first = table.reserve(first_request).unwrap();
        let second = table.reserve(second_request).unwrap();
        assert_ne!(first, second);
        assert_eq!(table.snapshot(first).unwrap().request, first_request);
        assert_eq!(table.snapshot(second).unwrap().request, second_request);
    }

    #[test]
    fn uncertain_completion_requires_drain_and_reconciliation() {
        let mut table = OperationSlotTable::new(limits());
        let token = table.reserve(request(1, 1)).unwrap();
        let operation_id = table.register_child(token, RANGE).unwrap();
        let uncertain = StoreCompletion::new(
            operation_id,
            RANGE,
            CompletedRangeSet::empty(),
            CompletionDisposition::Uncertain,
            PersistenceEvidence::VolatileOrUnknown,
        )
        .unwrap();
        table.apply_completion(token, uncertain).unwrap();
        table.mark_drain_required(token).unwrap();
        assert_eq!(
            table.snapshot(token).unwrap().state,
            SlotState::CompletionUncertain
        );
        table.complete_drain(token).unwrap();
        table
            .record_reconciliation(token, ReconciliationOutcome::UncertainRetained)
            .unwrap();
        assert_eq!(table.snapshot(token).unwrap().state, SlotState::Reclaimable);
    }

    #[test]
    fn admission_bounds_slots_buffers_children_and_retries() {
        let mut table = OperationSlotTable::new(limits());
        let token = table.reserve(request(1, 1)).unwrap();
        assert!(matches!(
            table.reserve(request(2, 1)),
            Err(SlotError::Admission(AdmissionError::Exhausted {
                resource: ResourceKind::OperationSlots,
                ..
            }))
        ));
        let mut buffer_table = OperationSlotTable::new(ResourceLimits::new(2, 1, 1, 1, 1, 1));
        buffer_table.reserve(request(3, 1)).unwrap();
        assert!(matches!(
            buffer_table.reserve(request(4, 1)),
            Err(SlotError::Admission(AdmissionError::Exhausted {
                resource: ResourceKind::Buffers,
                ..
            }))
        ));
        table.register_child(token, RANGE).unwrap();
        assert!(matches!(
            table.register_child(token, RANGE),
            Err(SlotError::Admission(AdmissionError::Exhausted {
                resource: ResourceKind::BackendSubmissions,
                ..
            }))
        ));
        table.record_retry(token).unwrap();
        assert_eq!(
            retry_eligibility(false, true, false),
            RetryEligibility::Forbidden
        );
        assert!(matches!(
            table.record_retry(token),
            Err(SlotError::Admission(AdmissionError::Exhausted {
                resource: ResourceKind::Retries,
                ..
            }))
        ));
    }

    #[test]
    fn capability_matrix_rejects_only_unsupported_operations() {
        let supports = [
            CapabilitySupport::Unknown,
            CapabilitySupport::declared(),
            CapabilitySupport::probed(),
            CapabilitySupport::certified(),
        ];
        for read in supports {
            for write in supports {
                for flush in supports {
                    for write_zeroes in supports {
                        for discard in supports {
                            let mut capabilities = StoreCapabilities::portable_demo(
                                64,
                                4,
                                32,
                                CapabilityEvidenceId(11),
                            );
                            capabilities.read = read;
                            capabilities.write = write;
                            capabilities.durable_flush = flush;
                            capabilities.write_zeroes = write_zeroes;
                            capabilities.discard = discard;

                            let base = StoreRequest {
                                operation_id: OperationId(1),
                                store_id: StoreId(2),
                                topology_epoch: TopologyEpoch(3),
                                kind: StoreRequestKind::Read { range: RANGE },
                                buffer: Some(BufferToken::new(0, 1)),
                            };
                            assert_eq!(
                                base.validate(&capabilities).is_ok(),
                                read.is_supported(),
                                "read support={read:?}"
                            );

                            let write_request = StoreRequest {
                                kind: StoreRequestKind::Write {
                                    range: RANGE,
                                    intent: WriteIntent::Ordinary,
                                },
                                ..base
                            };
                            assert_eq!(
                                write_request.validate(&capabilities).is_ok(),
                                write.is_supported(),
                                "write support={write:?}"
                            );

                            let flush_request = StoreRequest {
                                kind: StoreRequestKind::Flush {
                                    through: StoreWriteWatermark(1),
                                },
                                buffer: None,
                                ..base
                            };
                            assert_eq!(
                                flush_request.validate(&capabilities).is_ok(),
                                flush.is_supported(),
                                "flush support={flush:?}"
                            );

                            let zeroes_request = StoreRequest {
                                kind: StoreRequestKind::WriteZeroes {
                                    range: RANGE,
                                    intent: WriteIntent::Ordinary,
                                },
                                buffer: None,
                                ..base
                            };
                            assert_eq!(
                                zeroes_request.validate(&capabilities).is_ok(),
                                write_zeroes.is_supported(),
                                "write-zeroes support={write_zeroes:?}"
                            );

                            let discard_request = StoreRequest {
                                kind: StoreRequestKind::Discard { range: RANGE },
                                buffer: None,
                                ..base
                            };
                            assert_eq!(
                                discard_request.validate(&capabilities).is_ok(),
                                discard.is_supported(),
                                "discard support={discard:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn fake_adapter_keeps_backend_failure_and_timeout_evidence_explicit() {
        let failed = StoreCompletion::new(
            child(4),
            RANGE,
            CompletedRangeSet::empty(),
            CompletionDisposition::Failed(StoreError::BackendFailure { code: 5 }),
            PersistenceEvidence::VolatileOrUnknown,
        )
        .unwrap();
        assert_eq!(
            failed.disposition,
            CompletionDisposition::Failed(StoreError::BackendFailure { code: 5 })
        );
        assert!(!failed.persistence.is_durable());

        let timeout = StoreCompletion::new(
            child(5),
            RANGE,
            CompletedRangeSet::empty(),
            CompletionDisposition::Uncertain,
            PersistenceEvidence::VolatileOrUnknown,
        )
        .unwrap();
        assert_eq!(timeout.disposition, CompletionDisposition::Uncertain);
        assert!(!timeout.persistence.is_durable());
    }

    #[test]
    fn identity_and_store_disappearance_invalidate_without_rewriting_observations() {
        let before = IdentityObservationSet::new(
            vec![IdentityObservation {
                source: IdentitySourceKind::Serial,
                fingerprint: [1; 16],
            }],
            IdentityAssessment::Confirmed,
        );
        let after = IdentityObservationSet::new(
            vec![IdentityObservation {
                source: IdentitySourceKind::Serial,
                fingerprint: [2; 16],
            }],
            IdentityAssessment::Confirmed,
        );
        assert_eq!(before.compare(&after), IdentityComparison::Changed);

        let mut table = OperationSlotTable::new(limits());
        let token = table.reserve(request(1, 1)).unwrap();
        table
            .invalidate(token, StoreInvalidationReason::IdentityChanged)
            .unwrap();
        let snapshot = table.snapshot(token).unwrap();
        assert_eq!(
            snapshot.invalidated_by,
            Some(StoreInvalidationReason::IdentityChanged)
        );
        assert_eq!(snapshot.request.topology_epoch, TopologyEpoch(1));
    }

    fn full_success_for(operation_id: ChildOperationId, range: ByteRange) -> StoreCompletion {
        StoreCompletion::new(
            operation_id,
            range,
            CompletedRangeSet::new(if range.is_empty() {
                Vec::new()
            } else {
                vec![range]
            })
            .unwrap(),
            CompletionDisposition::Success,
            PersistenceEvidence::VolatileOrUnknown,
        )
        .unwrap()
    }
}
