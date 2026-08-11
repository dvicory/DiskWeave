use crate::capabilities::require_support;
use crate::*;

pub(crate) fn checked_end(range: ByteRange) -> Result<u64, StoreError> {
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
