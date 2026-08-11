//! Portable random-access store and operation-lifetime contracts.
//!
//! This crate deliberately contains no filesystem, kernel, database, or
//! executor implementation.  Adapters provide exact-range completions while
//! callers retain policy over retries, uncertainty, and reconciliation.

use std::fmt;

mod admission;
mod capabilities;
mod completion;
mod identity;
mod range;
mod reconciliation;

pub use admission::*;
pub use capabilities::*;
pub use completion::*;
pub use identity::*;
pub use range::*;
pub use reconciliation::*;

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

pub trait RandomAccessStore {
    fn store_id(&self) -> StoreId;
    fn topology_epoch(&self) -> TopologyEpoch;
    fn incarnation(&self) -> StoreIncarnationId;
    fn identity_observations(&self) -> IdentityObservationSet;
    fn current_identity_observations(&self) -> Result<IdentityObservationSet, StoreError>;
    fn capabilities(&self) -> StoreCapabilities;
    fn length(&self) -> u64;
    fn highest_accepted_watermark(&self) -> Option<StoreWriteWatermark>;

    fn read_at(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        destination: &mut [u8],
    ) -> StoreCompletion;

    fn write_at(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        source: &[u8],
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
            fn store_id(&self) -> StoreId {
                StoreId(1)
            }

            fn topology_epoch(&self) -> TopologyEpoch {
                TopologyEpoch(1)
            }

            fn incarnation(&self) -> StoreIncarnationId {
                StoreIncarnationId(1)
            }

            fn identity_observations(&self) -> IdentityObservationSet {
                IdentityObservationSet::new(Vec::new(), IdentityAssessment::Unknown)
            }
            fn current_identity_observations(&self) -> Result<IdentityObservationSet, StoreError> {
                Ok(self.identity_observations())
            }

            fn capabilities(&self) -> StoreCapabilities {
                self.capabilities.clone()
            }

            fn length(&self) -> u64 {
                64
            }
            fn highest_accepted_watermark(&self) -> Option<StoreWriteWatermark> {
                None
            }

            fn read_at(
                &mut self,
                operation_id: ChildOperationId,
                range: ByteRange,
                _destination: &mut [u8],
            ) -> StoreCompletion {
                full_success_for(operation_id, range)
            }

            fn write_at(
                &mut self,
                operation_id: ChildOperationId,
                range: ByteRange,
                _source: &[u8],
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
            let mut bytes = [0; 4];
            let completion = adapter.read_at(child(3), RANGE, &mut bytes);
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
