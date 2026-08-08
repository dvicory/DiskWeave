use dwv_core::{ByteRange, TopologyEpoch};
use dwv_store::{
    AdmissionError, CompletedRangeSet, CompletionDisposition, OperationId, OperationSlotTable,
    OperationSlotToken, PersistenceEvidence, ResourceLimits, ResourceUsage, SlotError,
    StoreCompletion,
};

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
    pub fn reserve(
        &mut self,
        request_id: OperationId,
        epoch: TopologyEpoch,
    ) -> Result<OperationSlotToken, SlotError> {
        self.table.reserve(request_id, epoch)
    }
    pub fn attach_buffer(
        &mut self,
        token: OperationSlotToken,
        index: u32,
    ) -> Result<(), SlotError> {
        self.table
            .attach_buffer(token, dwv_store::BufferToken::new(index, token.generation))
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
    pub fn complete(
        &mut self,
        token: OperationSlotToken,
        completion: StoreCompletion,
    ) -> Result<(), SlotError> {
        self.table.apply_completion(token, completion).map(|_| ())
    }
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
    pub fn reclaim(&mut self, token: OperationSlotToken, uncertain: bool) -> Result<(), SlotError> {
        self.table.record_reconciliation(
            token,
            if uncertain {
                dwv_store::ReconciliationOutcome::UncertainRetained
            } else {
                dwv_store::ReconciliationOutcome::Durable
            },
        )?;
        self.table.release(token)
    }
    pub fn snapshot(
        &self,
        token: OperationSlotToken,
    ) -> Result<dwv_store::SlotSnapshot, SlotError> {
        self.table.snapshot(token)
    }
}

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

    #[test]
    fn bounded_slots_reject_exhaustion_and_reuse_with_new_generation() {
        let mut admission = OperationAdmission::new(AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 1, 1, 1, 1),
        });
        let first = admission.reserve(OperationId(1), TopologyEpoch(2)).unwrap();
        assert!(matches!(
            admission.reserve(OperationId(2), TopologyEpoch(2)),
            Err(SlotError::Admission(AdmissionError::Exhausted {
                resource: dwv_store::ResourceKind::OperationSlots,
                ..
            }))
        ));
        admission.reclaim(first, false).unwrap();
        let second = admission.reserve(OperationId(2), TopologyEpoch(2)).unwrap();
        assert_ne!(first, second);
        assert_eq!(admission.usage().operation_slots, 1);
        admission.reclaim(second, false).unwrap();
        assert_eq!(admission.usage().operation_slots, 0);
    }

    #[test]
    fn child_completions_can_arrive_out_of_order_and_duplicates_are_safe() {
        let mut admission = OperationAdmission::new(AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 3, 1, 1, 1),
        });
        let token = admission.reserve(OperationId(9), TopologyEpoch(2)).unwrap();
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
            dwv_store::SlotState::ReconciliationRequired
        );
        admission.reclaim(token, false).unwrap();
        assert_eq!(admission.usage().backend_submissions, 0);
    }

    #[test]
    fn abandonment_keeps_the_slot_until_uncertain_children_reconcile() {
        let mut admission = OperationAdmission::new(AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 1, 1, 1, 1),
        });
        let token = admission
            .reserve(OperationId(10), TopologyEpoch(2))
            .unwrap();
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
}
