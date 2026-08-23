use dwv_core::{BlockRequest, ByteRange};
use dwv_store::{
    AdmissionError, OperationSlotTable, OperationSlotToken, ReconciliationOutcome, ResourceKind,
    ResourceLimits, ResourceUsage, SlotError, SlotState, StoreCompletionDelivery, StoreError,
    StoreId, StoreIncarnationId, StoreSubmissionIdentity, StoreWriteWatermark,
};
#[cfg(test)]
use dwv_store::{CompletedRangeSet, CompletionDisposition, PersistenceEvidence, StoreCompletion};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmissionConfig {
    pub limits: ResourceLimits,
}

impl Default for AdmissionConfig {
    fn default() -> Self {
        Self {
            limits: ResourceLimits::new(32, 128, 256, 32, 32, 8),
        }
    }
}

/// dwv:req req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe
pub struct OperationAdmission {
    table: OperationSlotTable,
}

impl OperationAdmission {
    pub fn new(config: AdmissionConfig) -> Self {
        Self {
            table: OperationSlotTable::new(config.limits),
        }
    }
    pub fn usage(&self) -> ResourceUsage {
        self.table.usage()
    }
    pub fn reserve(&mut self, request: BlockRequest) -> Result<OperationSlotToken, SlotError> {
        self.table.reserve(request)
    }
    pub fn child(
        &mut self,
        token: OperationSlotToken,
        range: ByteRange,
    ) -> Result<dwv_store::ChildOperationId, SlotError> {
        self.table.register_child(token, range)
    }
    pub fn children(
        &mut self,
        token: OperationSlotToken,
        ranges: &[ByteRange],
    ) -> Result<Vec<dwv_store::ChildOperationId>, SlotError> {
        let snapshot = self.table.snapshot(token)?;
        if snapshot.state == SlotState::Reclaimable {
            return Err(SlotError::InvalidState {
                token,
                state: snapshot.state,
            });
        }
        for range in ranges {
            if range.offset.checked_add(range.length).is_none() {
                return Err(SlotError::Completion(StoreError::RangeOverflow {
                    offset: range.offset,
                    length: range.length,
                }));
            }
        }
        let limits = self.table.limits();
        let usage = self.table.usage();
        let available = limits
            .backend_submissions
            .saturating_sub(usage.backend_submissions);
        let child_index_capacity = (u32::MAX as usize).saturating_sub(snapshot.children.len());
        if ranges.len() > available || ranges.len() > child_index_capacity {
            return Err(SlotError::Admission(AdmissionError::Exhausted {
                resource: ResourceKind::BackendSubmissions,
                limit: limits.backend_submissions,
                in_use: usage.backend_submissions,
            }));
        }
        ranges
            .iter()
            .map(|range| self.table.register_child(token, *range))
            .collect()
    }
    pub fn submitted(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        self.table.mark_submitted(token)
    }
    pub fn submit_all(&mut self, token: OperationSlotToken, count: usize) -> Result<(), SlotError> {
        for _ in 0..count {
            self.table.mark_submitted(token)?;
        }
        Ok(())
    }
    /// Test-only convenience that still exercises checked delivery ingress.
    #[cfg(test)]
    pub fn complete(
        &mut self,
        token: OperationSlotToken,
        completion: StoreCompletion,
    ) -> Result<(), SlotError> {
        let operation_id = completion.operation_id;
        let snapshot = self.table.snapshot(token)?;
        let identity = snapshot
            .children
            .iter()
            .find(|child| child.operation_id == operation_id)
            .and_then(|child| child.submission)
            .unwrap_or_else(|| {
                StoreSubmissionIdentity::new(
                    operation_id,
                    StoreId(0),
                    StoreIncarnationId(0),
                    snapshot.request.topology_epoch,
                )
            });
        if !snapshot
            .children
            .iter()
            .any(|child| child.operation_id == operation_id && child.submission.is_some())
        {
            self.table.accept_submission(token, identity)?;
        }
        self.deliver(StoreCompletionDelivery {
            identity,
            completion,
        })
    }
    /// Register physical acceptance for any read, write, or flush child.
    /// The same identity can be retained by a future semantic continuation.
    pub fn accept(
        &mut self,
        token: OperationSlotToken,
        child: dwv_store::ChildOperationId,
        store_id: StoreId,
        store_incarnation: StoreIncarnationId,
        topology_epoch: dwv_core::TopologyEpoch,
    ) -> Result<StoreSubmissionIdentity, SlotError> {
        let identity =
            StoreSubmissionIdentity::new(child, store_id, store_incarnation, topology_epoch);
        self.table.accept_submission(token, identity)?;
        Ok(identity)
    }
    pub fn record_submitted_watermark(
        &mut self,
        token: OperationSlotToken,
        watermark: StoreWriteWatermark,
    ) -> Result<(), SlotError> {
        self.table.record_submitted_watermark(token, watermark)
    }
    /// The operation driver explicitly abandons only planned children that
    /// never crossed the physical acceptance boundary.
    pub fn refuse_unaccepted(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let children = self
            .table
            .snapshot(token)?
            .children
            .into_iter()
            .filter(|child| !child.terminal && child.submission.is_none())
            .map(|child| child.operation_id)
            .collect::<Vec<_>>();
        for child in children {
            self.table.refuse_submission(token, child)?;
        }
        Ok(())
    }

    /// Deliver a normalized result after any physical delay or reordering.
    pub fn deliver(&mut self, delivery: StoreCompletionDelivery) -> Result<(), SlotError> {
        self.table
            .apply_delivery(delivery.identity.operation_id.slot, delivery)
            .map(|_| ())
    }
    #[cfg(test)]
    pub fn reconcile_children(
        &mut self,
        token: OperationSlotToken,
        children: &[(dwv_store::ChildOperationId, ByteRange)],
    ) -> Result<(), SlotError> {
        for &(operation_id, range) in children {
            self.complete(
                token,
                completion(
                    operation_id,
                    range,
                    0,
                    CompletionDisposition::Uncertain,
                    PersistenceEvidence::VolatileOrUnknown,
                ),
            )?;
        }
        Ok(())
    }
    pub fn abandon(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        self.table.mark_abandoned(token)
    }
    pub fn record_reconciliation(
        &mut self,
        token: OperationSlotToken,
        outcome: ReconciliationOutcome,
    ) -> Result<(), SlotError> {
        self.table.record_reconciliation(token, outcome)
    }
    pub fn release(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        self.table.release(token)
    }
    pub fn reclaim(&mut self, token: OperationSlotToken, uncertain: bool) -> Result<(), SlotError> {
        self.record_reconciliation(
            token,
            if uncertain {
                ReconciliationOutcome::UncertainRetained
            } else {
                ReconciliationOutcome::Durable
            },
        )?;
        self.release(token)
    }
    pub fn snapshot(
        &self,
        token: OperationSlotToken,
    ) -> Result<dwv_store::SlotSnapshot, SlotError> {
        self.table.snapshot(token)
    }
}
#[cfg(test)]
pub(crate) fn completion(
    operation_id: dwv_store::ChildOperationId,
    range: ByteRange,
    completed: u64,
    disposition: CompletionDisposition,
    persistence: PersistenceEvidence,
) -> StoreCompletion {
    let completed_range = ByteRange::new(range.offset, completed).expect("completion is bounded");
    StoreCompletion::new(
        operation_id,
        range,
        CompletedRangeSet::new(if completed == 0 {
            Vec::new()
        } else {
            vec![completed_range]
        })
        .expect("completion range is valid"),
        disposition,
        persistence,
    )
    .expect("adapter completion is valid")
}

pub(crate) fn admission_error(error: impl std::fmt::Display) -> crate::failure::ServiceError {
    crate::failure::ServiceError::io(crate::failure::FailureClass::Admission, error.to_string())
}

pub(crate) fn slot_error(error: SlotError) -> crate::failure::ServiceError {
    admission_error(error)
}
pub(crate) fn _admission_type(_: AdmissionError) {}

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_core::{
        BlockOp, BufferToken, DurabilityIntent, FenceDomain, FrontendId, OrderingIntent, RequestId,
        SlotId, SubmissionSequence, TopologyEpoch,
    };

    fn request(request_id: u64) -> BlockRequest {
        BlockRequest::new(
            RequestId(request_id),
            FrontendId(3),
            SlotId::from_bytes([4; 16]),
            TopologyEpoch(2),
            BlockOp::Read,
            ByteRange::new(0, 4).unwrap(),
            Some(BufferToken::new(0, 1)),
            OrderingIntent {
                submission_sequence: SubmissionSequence(request_id),
                preflush: false,
                fence_domain: FenceDomain(5),
            },
            DurabilityIntent::Ordinary,
        )
    }

    #[test]
    fn bounded_slots_reject_exhaustion_and_reuse_with_new_generation() {
        let mut admission = OperationAdmission::new(AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 1, 1, 1, 1),
        });
        let first = admission.reserve(request(1)).unwrap();
        assert!(matches!(
            admission.reserve(request(2)),
            Err(SlotError::Admission(AdmissionError::Exhausted {
                resource: dwv_store::ResourceKind::OperationSlots,
                ..
            }))
        ));
        admission.reclaim(first, false).unwrap();
        let second = admission.reserve(request(2)).unwrap();
        assert_ne!(first, second);
        assert_eq!(admission.usage().operation_slots, 1);
        admission.reclaim(second, false).unwrap();
        assert_eq!(admission.usage().operation_slots, 0);
    }
    #[test]
    fn child_batch_admission_fails_before_any_child_is_registered() {
        let mut admission = OperationAdmission::new(AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 1, 1, 1, 1),
        });
        let token = admission.reserve(request(3)).unwrap();
        let ranges = [ByteRange::new(0, 4).unwrap(), ByteRange::new(4, 4).unwrap()];

        assert!(matches!(
            admission.children(token, &ranges),
            Err(SlotError::Admission(AdmissionError::Exhausted {
                resource: ResourceKind::BackendSubmissions,
                ..
            }))
        ));
        let snapshot = admission.snapshot(token).unwrap();
        assert_eq!(snapshot.state, SlotState::Reserved);
        assert!(snapshot.children.is_empty());
        assert_eq!(admission.usage().backend_submissions, 0);
        admission.reclaim(token, false).unwrap();
        assert_eq!(admission.usage().operation_slots, 0);
    }

    #[test]
    fn child_completions_can_arrive_out_of_order_and_duplicates_are_safe() {
        let mut admission = OperationAdmission::new(AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 3, 1, 1, 1),
        });
        let token = admission.reserve(request(9)).unwrap();
        let ranges = [
            ByteRange::new(0, 4).unwrap(),
            ByteRange::new(4, 4).unwrap(),
            ByteRange::new(8, 4).unwrap(),
        ];
        let children = admission.children(token, &ranges).unwrap();

        admission.submit_all(token, children.len()).unwrap();
        for (&child, &range) in children.iter().zip(&ranges).rev() {
            admission
                .complete(
                    token,
                    completion(
                        child,
                        range,
                        range.length,
                        CompletionDisposition::Success,
                        PersistenceEvidence::VolatileOrUnknown,
                    ),
                )
                .unwrap();
        }
        admission
            .complete(
                token,
                completion(
                    children[0],
                    ranges[0],
                    ranges[0].length,
                    CompletionDisposition::Success,
                    PersistenceEvidence::VolatileOrUnknown,
                ),
            )
            .unwrap();
        assert_eq!(
            admission.snapshot(token).unwrap().state,
            dwv_store::SlotState::AwaitingReconciliation
        );
        admission.reclaim(token, false).unwrap();
        assert_eq!(admission.usage().backend_submissions, 0);
    }
    #[test]
    fn refusing_planned_children_preserves_accepted_outstanding_resources() {
        let mut admission = OperationAdmission::new(AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 2, 1, 1, 1),
        });
        let token = admission.reserve(request(24)).unwrap();
        let ranges = [ByteRange::new(0, 4).unwrap(), ByteRange::new(4, 4).unwrap()];
        let children = admission.children(token, &ranges).unwrap();
        admission.submit_all(token, children.len()).unwrap();
        let accepted_identity = admission
            .accept(
                token,
                children[0],
                StoreId(7),
                StoreIncarnationId(3),
                TopologyEpoch(2),
            )
            .unwrap();

        admission.refuse_unaccepted(token).unwrap();
        let snapshot = admission.snapshot(token).unwrap();
        assert!(!snapshot.children[0].terminal);
        assert!(snapshot.children[1].terminal);
        assert!(snapshot.children[1].refused_before_acceptance);
        assert_eq!(admission.usage().backend_submissions, 1);

        admission
            .deliver(StoreCompletionDelivery {
                identity: accepted_identity,
                completion: completion(
                    children[0],
                    ranges[0],
                    ranges[0].length,
                    CompletionDisposition::Success,
                    PersistenceEvidence::VolatileOrUnknown,
                ),
            })
            .unwrap();
        admission.reclaim(token, false).unwrap();
        assert_eq!(admission.usage().backend_submissions, 0);
    }

    #[test]
    fn abandonment_before_admission_or_irreversible_work_owns_no_backend_resources() {
        let mut admission = OperationAdmission::new(AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 1, 1, 1, 1),
        });
        let empty = ResourceUsage {
            operation_slots: 0,
            buffers: 0,
            backend_submissions: 0,
            retries: 0,
            range_locks: 0,
            background_work: 0,
        };
        assert_eq!(admission.usage(), empty);

        let token = admission.reserve(request(11)).unwrap();
        admission.abandon(token).unwrap();
        let snapshot = admission.snapshot(token).unwrap();
        assert!(snapshot.abandoned);
        assert_eq!(snapshot.buffers, vec![BufferToken::new(0, 1)]);
        assert!(snapshot.children.is_empty());
        admission.reclaim(token, false).unwrap();
        assert_eq!(admission.usage(), empty);
    }

    #[test]
    fn abandonment_keeps_the_slot_until_uncertain_children_reconcile() {
        let mut admission = OperationAdmission::new(AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 1, 1, 1, 1),
        });
        let token = admission.reserve(request(10)).unwrap();
        let range = ByteRange::new(0, 4).unwrap();
        let child = admission.child(token, range).unwrap();
        admission.submit_all(token, 1).unwrap();
        admission.abandon(token).unwrap();
        assert!(admission.snapshot(token).unwrap().abandoned);
        admission
            .complete(
                token,
                completion(
                    child,
                    range,
                    0,
                    CompletionDisposition::Uncertain,
                    PersistenceEvidence::VolatileOrUnknown,
                ),
            )
            .unwrap();
        admission.reclaim(token, true).unwrap();
        assert_eq!(admission.usage().operation_slots, 0);
        assert_eq!(admission.usage().backend_submissions, 0);
    }

    #[test]
    fn abandonment_matrix_reconciles_every_child_completion_cutpoint() {
        let ranges = [
            ByteRange::new(0, 4).unwrap(),
            ByteRange::new(4, 4).unwrap(),
            ByteRange::new(8, 4).unwrap(),
            ByteRange::new(12, 4).unwrap(),
        ];

        for completed_count in 0..=ranges.len() {
            let mut admission = OperationAdmission::new(AdmissionConfig {
                limits: ResourceLimits::new(1, 1, ranges.len(), 1, 1, 1),
            });
            let token = admission
                .reserve(request(100 + completed_count as u64))
                .unwrap();
            let children = admission.children(token, &ranges).unwrap();
            admission.submit_all(token, children.len()).unwrap();

            for (&child, &range) in children.iter().zip(&ranges).take(completed_count) {
                admission
                    .complete(
                        token,
                        completion(
                            child,
                            range,
                            range.length,
                            CompletionDisposition::Success,
                            PersistenceEvidence::VolatileOrUnknown,
                        ),
                    )
                    .unwrap();
            }

            admission.abandon(token).unwrap();
            assert!(admission.snapshot(token).unwrap().abandoned);
            let remaining = children
                .iter()
                .zip(&ranges)
                .skip(completed_count)
                .map(|(&child, &range)| (child, range))
                .collect::<Vec<_>>();
            admission.reconcile_children(token, &remaining).unwrap();
            admission
                .reclaim(token, completed_count != ranges.len())
                .unwrap();
            assert_eq!(admission.usage().operation_slots, 0);
            assert_eq!(admission.usage().backend_submissions, 0);
        }
    }
}
