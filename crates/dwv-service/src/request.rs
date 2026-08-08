use dwv_core::{ByteRange, DurabilityIntent, RequestId, TopologyEpoch};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequestOperation {
    Read { data_slot: usize },
    Write { data_slot: usize, bytes: Vec<u8> },
    Flush,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableRequest {
    pub request_id: RequestId,
    pub topology_epoch: TopologyEpoch,
    pub range: ByteRange,
    pub operation: RequestOperation,
    pub durability: DurabilityIntent,
}

impl PortableRequest {
    pub fn read(
        request_id: RequestId,
        epoch: TopologyEpoch,
        data_slot: usize,
        range: ByteRange,
    ) -> Self {
        Self {
            request_id,
            topology_epoch: epoch,
            range,
            operation: RequestOperation::Read { data_slot },
            durability: DurabilityIntent::Ordinary,
        }
    }

    pub fn write(
        request_id: RequestId,
        epoch: TopologyEpoch,
        data_slot: usize,
        range: ByteRange,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            request_id,
            topology_epoch: epoch,
            range,
            operation: RequestOperation::Write { data_slot, bytes },
            durability: DurabilityIntent::Ordinary,
        }
    }

    pub fn flush(request_id: RequestId, epoch: TopologyEpoch) -> Self {
        Self {
            request_id,
            topology_epoch: epoch,
            range: ByteRange::empty(),
            operation: RequestOperation::Flush,
            durability: DurabilityIntent::ExplicitFlush,
        }
    }

    pub fn with_durability(mut self, durability: DurabilityIntent) -> Self {
        self.durability = durability;
        self
    }
}
