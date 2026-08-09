use dwv_core::{
    BlockOp, BlockRequest, BufferToken, ByteRange, DurabilityIntent, FenceDomain, FrontendId,
    OrderingIntent, RequestId, SlotId, SubmissionSequence, TopologyEpoch,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequestOperation {
    Read { data_slot: usize },
    Write { data_slot: usize, bytes: Vec<u8> },
    Flush,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableRequest {
    pub request_id: RequestId,
    pub frontend_id: FrontendId,
    pub target_slot: SlotId,
    pub topology_epoch: TopologyEpoch,
    pub range: ByteRange,
    pub buffer: Option<BufferToken>,
    pub ordering: OrderingIntent,
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
        let data_slot_id = u8::try_from(data_slot).unwrap_or(u8::MAX);
        Self {
            request_id,
            frontend_id: FrontendId(0),
            target_slot: SlotId([data_slot_id; 16]),
            topology_epoch: epoch,
            range,
            buffer: None,
            ordering: OrderingIntent {
                submission_sequence: SubmissionSequence(request_id.0),
                preflush: false,
                fence_domain: FenceDomain(1),
            },
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
        let data_slot_id = u8::try_from(data_slot).unwrap_or(u8::MAX);
        Self {
            request_id,
            frontend_id: FrontendId(0),
            target_slot: SlotId([data_slot_id; 16]),
            topology_epoch: epoch,
            range,
            buffer: None,
            ordering: OrderingIntent {
                submission_sequence: SubmissionSequence(request_id.0),
                preflush: false,
                fence_domain: FenceDomain(1),
            },
            operation: RequestOperation::Write { data_slot, bytes },
            durability: DurabilityIntent::Ordinary,
        }
    }

    pub fn flush(request_id: RequestId, epoch: TopologyEpoch) -> Self {
        Self {
            request_id,
            frontend_id: FrontendId(0),
            target_slot: SlotId([0; 16]),
            topology_epoch: epoch,
            range: ByteRange::empty(),
            buffer: None,
            ordering: OrderingIntent {
                submission_sequence: SubmissionSequence(request_id.0),
                preflush: false,
                fence_domain: FenceDomain(1),
            },
            operation: RequestOperation::Flush,
            durability: DurabilityIntent::ExplicitFlush,
        }
    }

    pub fn with_durability(mut self, durability: DurabilityIntent) -> Self {
        self.durability = durability;
        self
    }

    pub fn from_block_request(
        request: BlockRequest,
        data_slot: usize,
        write_bytes: Option<Vec<u8>>,
    ) -> Option<Self> {
        let operation = match request.op {
            BlockOp::Read => RequestOperation::Read { data_slot },
            BlockOp::Write => RequestOperation::Write {
                data_slot,
                bytes: write_bytes?,
            },
            BlockOp::Flush => RequestOperation::Flush,
            BlockOp::WriteZeroes | BlockOp::Discard | BlockOp::Zoned => return None,
        };
        Some(Self {
            request_id: request.request_id,
            frontend_id: request.frontend_id,
            target_slot: request.slot_id,
            topology_epoch: request.topology_epoch,
            range: request.range,
            buffer: request.buffer,
            ordering: request.ordering,
            operation,
            durability: request.durability,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_request_fields_survive_service_translation() {
        let normalized = BlockRequest::new(
            RequestId(7),
            FrontendId(8),
            SlotId([9; 16]),
            TopologyEpoch(10),
            BlockOp::Write,
            ByteRange::new(512, 1024).unwrap(),
            Some(BufferToken::new(11, 12)),
            OrderingIntent {
                submission_sequence: SubmissionSequence(13),
                preflush: true,
                fence_domain: FenceDomain(14),
            },
            DurabilityIntent::ExplicitFlush,
        );
        let portable =
            PortableRequest::from_block_request(normalized, 0, Some(vec![1; 1024])).unwrap();
        assert_eq!(portable.request_id, normalized.request_id);
        assert_eq!(portable.frontend_id, normalized.frontend_id);
        assert_eq!(portable.target_slot, normalized.slot_id);
        assert_eq!(portable.topology_epoch, normalized.topology_epoch);
        assert_eq!(portable.range, normalized.range);
        assert_eq!(portable.buffer, normalized.buffer);
        assert_eq!(portable.ordering, normalized.ordering);
        assert_eq!(portable.durability, normalized.durability);
    }
}
