use crate::range::checked_end;
use crate::*;

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
