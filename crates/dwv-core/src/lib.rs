//! Portable normalized block semantics for DiskWeave.
//!
//! This crate deliberately contains no operating-system, database, executor,
//! or async-runtime types. Frontends translate into these semantic values and
//! retain their own resources until the backend lifetime contract permits
//! reclamation.

use std::fmt;

mod topology;

pub use topology::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RequestId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FrontendId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SubmissionSequence(pub u64);

/// Opaque identifier for one protected parity/codeword unit within an externally
/// validated captured mapping.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct CodedUnitId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct FenceDomain(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BufferToken {
    pub index: u32,
    pub generation: u32,
}

impl BufferToken {
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ByteRange {
    pub offset: u64,
    pub length: u64,
}

impl ByteRange {
    pub const fn new(offset: u64, length: u64) -> Result<Self, RangeError> {
        if offset.checked_add(length).is_none() {
            return Err(RangeError::Overflow { offset, length });
        }
        Ok(Self { offset, length })
    }

    pub const fn empty() -> Self {
        Self {
            offset: 0,
            length: 0,
        }
    }

    pub const fn end(self) -> u64 {
        self.offset + self.length
    }

    pub const fn is_empty(self) -> bool {
        self.length == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RangeError {
    Overflow { offset: u64, length: u64 },
}

impl fmt::Display for RangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow { offset, length } => {
                write!(
                    formatter,
                    "byte range overflows: offset={offset} length={length}"
                )
            }
        }
    }
}

impl std::error::Error for RangeError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BlockOp {
    Read,
    Write,
    Flush,
    WriteZeroes,
    Discard,
    Zoned,
}

impl BlockOp {
    const fn requires_data_range(self) -> bool {
        !matches!(self, Self::Flush)
    }

    const fn accepts_buffer(self) -> bool {
        matches!(self, Self::Read | Self::Write)
    }

    const fn is_mutation(self) -> bool {
        matches!(self, Self::Write | Self::WriteZeroes | Self::Discard)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OrderingIntent {
    pub submission_sequence: SubmissionSequence,
    pub preflush: bool,
    pub fence_domain: FenceDomain,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DurabilityIntent {
    Ordinary,
    Fua,
    ExplicitFlush,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrontendCapabilities {
    pub max_transfer: Option<u64>,
    pub supports_preflush: bool,
    pub supports_fua: bool,
    pub supports_write_zeroes: bool,
    pub supports_discard: bool,
}

impl Default for FrontendCapabilities {
    fn default() -> Self {
        Self {
            max_transfer: None,
            supports_preflush: false,
            supports_fua: false,
            supports_write_zeroes: true,
            supports_discard: false,
        }
    }
}

/// dwv:req req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlockRequest {
    pub request_id: RequestId,
    pub frontend_id: FrontendId,
    pub slot_id: SlotId,
    pub topology_epoch: TopologyEpoch,
    pub op: BlockOp,
    pub range: ByteRange,
    pub buffer: Option<BufferToken>,
    pub ordering: OrderingIntent,
    pub durability: DurabilityIntent,
}

impl BlockRequest {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        request_id: RequestId,
        frontend_id: FrontendId,
        slot_id: SlotId,
        topology_epoch: TopologyEpoch,
        op: BlockOp,
        range: ByteRange,
        buffer: Option<BufferToken>,
        ordering: OrderingIntent,
        durability: DurabilityIntent,
    ) -> Self {
        Self {
            request_id,
            frontend_id,
            slot_id,
            topology_epoch,
            op,
            range,
            buffer,
            ordering,
            durability,
        }
    }

    pub fn validate(&self, capabilities: &FrontendCapabilities) -> Result<(), RequestError> {
        if self.op.requires_data_range() && self.range.is_empty() {
            return Err(RequestError::EmptyRange { op: self.op });
        }
        if matches!(self.op, BlockOp::Flush) && !self.range.is_empty() {
            return Err(RequestError::FlushHasRange { range: self.range });
        }

        if self.op.accepts_buffer() && self.buffer.is_none() {
            return Err(RequestError::MissingBuffer { op: self.op });
        }
        if !self.op.accepts_buffer() && self.buffer.is_some() {
            return Err(RequestError::UnexpectedBuffer { op: self.op });
        }

        if capabilities
            .max_transfer
            .is_some_and(|maximum| self.range.length > maximum)
        {
            return Err(RequestError::TransferTooLarge {
                length: self.range.length,
                maximum: capabilities.max_transfer.unwrap_or_default(),
            });
        }

        match self.op {
            BlockOp::Discard if !capabilities.supports_discard => {
                return Err(RequestError::UnsupportedOperation(BlockOp::Discard));
            }
            BlockOp::WriteZeroes if !capabilities.supports_write_zeroes => {
                return Err(RequestError::UnsupportedOperation(BlockOp::WriteZeroes));
            }
            BlockOp::Zoned => return Err(RequestError::UnsupportedOperation(BlockOp::Zoned)),
            _ => {}
        }

        if self.ordering.preflush && !capabilities.supports_preflush {
            return Err(RequestError::UnsupportedPreflush);
        }

        match self.durability {
            DurabilityIntent::Ordinary => {}
            DurabilityIntent::Fua => {
                if !matches!(self.op, BlockOp::Write | BlockOp::WriteZeroes) {
                    return Err(RequestError::InvalidDurabilityIntent {
                        op: self.op,
                        intent: self.durability,
                    });
                }
                if !capabilities.supports_fua {
                    return Err(RequestError::UnsupportedFua);
                }
            }
            DurabilityIntent::ExplicitFlush => {
                if self.op != BlockOp::Flush {
                    return Err(RequestError::InvalidDurabilityIntent {
                        op: self.op,
                        intent: self.durability,
                    });
                }
            }
        }

        if self.op.is_mutation() && self.durability == DurabilityIntent::ExplicitFlush {
            return Err(RequestError::InvalidDurabilityIntent {
                op: self.op,
                intent: self.durability,
            });
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RequestError {
    EmptyRange {
        op: BlockOp,
    },
    FlushHasRange {
        range: ByteRange,
    },
    MissingBuffer {
        op: BlockOp,
    },
    UnexpectedBuffer {
        op: BlockOp,
    },
    TransferTooLarge {
        length: u64,
        maximum: u64,
    },
    UnsupportedOperation(BlockOp),
    UnsupportedPreflush,
    UnsupportedFua,
    InvalidDurabilityIntent {
        op: BlockOp,
        intent: DurabilityIntent,
    },
}

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRange { op } => write!(formatter, "{op:?} requires a non-empty range"),
            Self::FlushHasRange { range } => {
                write!(formatter, "flush cannot carry range {range:?}")
            }
            Self::MissingBuffer { op } => write!(formatter, "{op:?} requires a buffer"),
            Self::UnexpectedBuffer { op } => write!(formatter, "{op:?} cannot carry a buffer"),
            Self::TransferTooLarge { length, maximum } => {
                write!(
                    formatter,
                    "transfer length {length} exceeds maximum {maximum}"
                )
            }
            Self::UnsupportedOperation(op) => write!(formatter, "unsupported operation {op:?}"),
            Self::UnsupportedPreflush => write!(formatter, "preflush capability is unavailable"),
            Self::UnsupportedFua => write!(formatter, "FUA capability is unavailable"),
            Self::InvalidDurabilityIntent { op, intent } => {
                write!(
                    formatter,
                    "durability intent {intent:?} is invalid for {op:?}"
                )
            }
        }
    }
}

impl std::error::Error for RequestError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrontendEvent {
    Abandon {
        frontend_id: FrontendId,
        request_id: RequestId,
    },
    Quiesced {
        frontend_id: FrontendId,
        through_sequence: SubmissionSequence,
    },
    Lost {
        frontend_id: FrontendId,
        duplicate_delivery_possible: bool,
    },
    Recovered {
        frontend_id: FrontendId,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompletionInterest {
    Required,
    Suppressed,
}

/// dwv:req req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RequestLifecycle {
    pub frontend_id: FrontendId,
    pub request_id: RequestId,
    pub completion_interest: CompletionInterest,
    pub irreversible: bool,
}

impl RequestLifecycle {
    pub const fn new(frontend_id: FrontendId, request_id: RequestId) -> Self {
        Self {
            frontend_id,
            request_id,
            completion_interest: CompletionInterest::Required,
            irreversible: false,
        }
    }

    pub const fn mark_irreversible(mut self) -> Self {
        self.irreversible = true;
        self
    }

    pub fn apply(&mut self, event: FrontendEvent) -> LifecycleEffect {
        match event {
            FrontendEvent::Abandon {
                frontend_id,
                request_id,
            } if (frontend_id, request_id) == (self.frontend_id, self.request_id) => {
                self.completion_interest = CompletionInterest::Suppressed;
                LifecycleEffect::CompletionSuppressed
            }
            FrontendEvent::Abandon { .. } => LifecycleEffect::Ignored,
            FrontendEvent::Quiesced {
                frontend_id,
                through_sequence,
            } => LifecycleEffect::Quiesced {
                frontend_id,
                through_sequence,
            },
            FrontendEvent::Lost {
                frontend_id,
                duplicate_delivery_possible,
            } => LifecycleEffect::Lost {
                frontend_id,
                duplicate_delivery_possible,
            },
            FrontendEvent::Recovered { frontend_id } => LifecycleEffect::Recovered { frontend_id },
        }
    }

    pub const fn may_deliver_completion(self) -> bool {
        matches!(self.completion_interest, CompletionInterest::Required)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LifecycleEffect {
    CompletionSuppressed,
    Quiesced {
        frontend_id: FrontendId,
        through_sequence: SubmissionSequence,
    },
    Lost {
        frontend_id: FrontendId,
        duplicate_delivery_possible: bool,
    },
    Recovered {
        frontend_id: FrontendId,
    },
    Ignored,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOT: SlotId = SlotId([7; 16]);
    const ORDERING: OrderingIntent = OrderingIntent {
        submission_sequence: SubmissionSequence(3),
        preflush: false,
        fence_domain: FenceDomain(9),
    };

    fn range(length: u64) -> ByteRange {
        ByteRange::new(8, length).expect("test range should fit")
    }

    fn write_request(buffer: Option<BufferToken>, durability: DurabilityIntent) -> BlockRequest {
        BlockRequest::new(
            RequestId(1),
            FrontendId(2),
            SLOT,
            TopologyEpoch(4),
            BlockOp::Write,
            range(16),
            buffer,
            ORDERING,
            durability,
        )
    }

    #[test]
    fn byte_range_overflow_is_rejected() {
        assert_eq!(
            ByteRange::new(u64::MAX, 1),
            Err(RangeError::Overflow {
                offset: u64::MAX,
                length: 1,
            })
        );
    }

    #[test]
    fn valid_write_preserves_semantic_fields() {
        let request = write_request(Some(BufferToken::new(4, 8)), DurabilityIntent::Ordinary);
        assert_eq!(request.request_id, RequestId(1));
        assert_eq!(request.topology_epoch, TopologyEpoch(4));
        assert_eq!(request.range.end(), 24);
        assert!(request.validate(&FrontendCapabilities::default()).is_ok());
    }

    #[test]
    fn writes_require_buffers() {
        assert_eq!(
            write_request(None, DurabilityIntent::Ordinary)
                .validate(&FrontendCapabilities::default()),
            Err(RequestError::MissingBuffer { op: BlockOp::Write })
        );
    }

    #[test]
    fn flush_requires_empty_range_and_no_buffer() {
        let request = BlockRequest::new(
            RequestId(1),
            FrontendId(2),
            SLOT,
            TopologyEpoch(4),
            BlockOp::Flush,
            range(1),
            None,
            ORDERING,
            DurabilityIntent::ExplicitFlush,
        );
        assert!(matches!(
            request.validate(&FrontendCapabilities::default()),
            Err(RequestError::FlushHasRange { .. })
        ));

        let empty_flush = BlockRequest::new(
            RequestId(1),
            FrontendId(2),
            SLOT,
            TopologyEpoch(4),
            BlockOp::Flush,
            ByteRange::empty(),
            Some(BufferToken::new(4, 8)),
            ORDERING,
            DurabilityIntent::ExplicitFlush,
        );
        assert_eq!(
            empty_flush.validate(&FrontendCapabilities::default()),
            Err(RequestError::UnexpectedBuffer { op: BlockOp::Flush })
        );
    }

    #[test]
    fn unsupported_discard_and_zoned_are_explicit() {
        let discard = BlockRequest::new(
            RequestId(1),
            FrontendId(2),
            SLOT,
            TopologyEpoch(4),
            BlockOp::Discard,
            range(16),
            None,
            ORDERING,
            DurabilityIntent::Ordinary,
        );
        assert_eq!(
            discard.validate(&FrontendCapabilities::default()),
            Err(RequestError::UnsupportedOperation(BlockOp::Discard))
        );

        let zoned = BlockRequest::new(
            RequestId(1),
            FrontendId(2),
            SLOT,
            TopologyEpoch(4),
            BlockOp::Zoned,
            range(16),
            None,
            ORDERING,
            DurabilityIntent::Ordinary,
        );
        assert_eq!(
            zoned.validate(&FrontendCapabilities::default()),
            Err(RequestError::UnsupportedOperation(BlockOp::Zoned))
        );
    }

    #[test]
    fn fua_and_preflush_need_capability_evidence() {
        let fua = write_request(Some(BufferToken::new(4, 8)), DurabilityIntent::Fua);
        assert_eq!(
            fua.validate(&FrontendCapabilities::default()),
            Err(RequestError::UnsupportedFua)
        );

        let mut ordering = ORDERING;
        ordering.preflush = true;
        let mut request = write_request(Some(BufferToken::new(4, 8)), DurabilityIntent::Ordinary);
        request.ordering = ordering;
        assert_eq!(
            request.validate(&FrontendCapabilities::default()),
            Err(RequestError::UnsupportedPreflush)
        );
    }

    #[test]
    fn transfer_limits_are_enforced() {
        let request = write_request(Some(BufferToken::new(4, 8)), DurabilityIntent::Ordinary);
        let capabilities = FrontendCapabilities {
            max_transfer: Some(8),
            ..FrontendCapabilities::default()
        };
        assert_eq!(
            request.validate(&capabilities),
            Err(RequestError::TransferTooLarge {
                length: 16,
                maximum: 8,
            })
        );
    }

    #[test]
    fn abandonment_suppresses_completion_without_cancellation() {
        let mut lifecycle = RequestLifecycle::new(FrontendId(2), RequestId(1)).mark_irreversible();
        assert_eq!(
            lifecycle.apply(FrontendEvent::Abandon {
                frontend_id: FrontendId(3),
                request_id: RequestId(1),
            }),
            LifecycleEffect::Ignored
        );
        assert_eq!(
            lifecycle.apply(FrontendEvent::Abandon {
                frontend_id: FrontendId(2),
                request_id: RequestId(1),
            }),
            LifecycleEffect::CompletionSuppressed
        );
        assert!(!lifecycle.may_deliver_completion());
        assert!(lifecycle.irreversible);
    }

    #[test]
    fn lifecycle_events_preserve_quiescence_and_loss_evidence() {
        let mut lifecycle = RequestLifecycle::new(FrontendId(2), RequestId(1));
        assert_eq!(
            lifecycle.apply(FrontendEvent::Quiesced {
                frontend_id: FrontendId(2),
                through_sequence: SubmissionSequence(11),
            }),
            LifecycleEffect::Quiesced {
                frontend_id: FrontendId(2),
                through_sequence: SubmissionSequence(11),
            }
        );
        assert_eq!(
            lifecycle.apply(FrontendEvent::Lost {
                frontend_id: FrontendId(2),
                duplicate_delivery_possible: true,
            }),
            LifecycleEffect::Lost {
                frontend_id: FrontendId(2),
                duplicate_delivery_possible: true,
            }
        );
    }

    fn next_sequence(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state
    }

    fn generated_request(index: u64, state: &mut u64) -> BlockRequest {
        let selector = next_sequence(state) % 5;
        let op = match selector {
            0 => BlockOp::Read,
            1 => BlockOp::Write,
            2 => BlockOp::Flush,
            3 => BlockOp::WriteZeroes,
            _ => BlockOp::Discard,
        };
        let range = if op == BlockOp::Flush {
            ByteRange::empty()
        } else {
            ByteRange::new(8 * (index + 1), 8 * ((next_sequence(state) % 8) + 1))
                .expect("generated range should fit")
        };
        let buffer = if matches!(op, BlockOp::Read | BlockOp::Write) {
            Some(BufferToken::new(
                index as u32,
                (index as u32).saturating_add(1),
            ))
        } else {
            None
        };
        let durability = match op {
            BlockOp::Flush => DurabilityIntent::ExplicitFlush,
            BlockOp::Write | BlockOp::WriteZeroes if next_sequence(state) & 1 == 0 => {
                DurabilityIntent::Fua
            }
            _ => DurabilityIntent::Ordinary,
        };
        BlockRequest::new(
            RequestId(index),
            FrontendId(2),
            SLOT,
            TopologyEpoch(4),
            op,
            range,
            buffer,
            OrderingIntent {
                submission_sequence: SubmissionSequence(index),
                preflush: next_sequence(state) & 1 == 0,
                fence_domain: FenceDomain(9),
            },
            durability,
        )
    }

    fn adapter_round_trip(request: BlockRequest) -> BlockRequest {
        BlockRequest::new(
            request.request_id,
            request.frontend_id,
            request.slot_id,
            request.topology_epoch,
            request.op,
            request.range,
            request.buffer,
            request.ordering,
            request.durability,
        )
    }

    #[test]
    fn generated_request_sequences_round_trip_through_adapter_oracle() {
        let capabilities = FrontendCapabilities {
            supports_preflush: true,
            supports_fua: true,
            supports_write_zeroes: true,
            supports_discard: true,
            ..FrontendCapabilities::default()
        };
        let mut state = 0xD15C_A11A_u64;
        let requests: Vec<_> = (1..=256)
            .map(|index| generated_request(index, &mut state))
            .collect();

        for request in requests {
            assert_eq!(request, adapter_round_trip(request));
            assert!(
                request.validate(&capabilities).is_ok(),
                "request: {request:?}"
            );
        }
    }
}
