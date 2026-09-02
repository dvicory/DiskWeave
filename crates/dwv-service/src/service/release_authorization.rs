use super::{OperationSlotToken, ReleaseAuthorization};
use std::collections::{BTreeMap, btree_map::Entry};

pub(super) struct ReleaseAuthorizationLedger {
    entries: Vec<BTreeMap<OperationSlotToken, ReleaseAuthorization>>,
    capacity_per_slot: usize,
}

impl ReleaseAuthorizationLedger {
    pub(super) fn new(slot_count: usize, capacity_per_slot: usize) -> Self {
        Self {
            entries: std::iter::repeat_with(BTreeMap::new)
                .take(slot_count)
                .collect(),
            capacity_per_slot,
        }
    }

    pub(super) fn observe(&self, operation: OperationSlotToken) -> Option<&ReleaseAuthorization> {
        let index = usize::try_from(operation.index).ok()?;
        self.entries.get(index)?.get(&operation)
    }

    pub(super) fn operations_at_index(
        &self,
        operation: OperationSlotToken,
    ) -> impl Iterator<Item = OperationSlotToken> + '_ {
        usize::try_from(operation.index)
            .ok()
            .and_then(|index| self.entries.get(index))
            .into_iter()
            .flat_map(|entries| entries.keys().copied())
    }

    pub(super) fn is_full_and_consumed(
        &self,
        operation: OperationSlotToken,
        mut has_consumer: impl FnMut(OperationSlotToken) -> bool,
    ) -> bool {
        if self.observe(operation).is_some() {
            return false;
        }
        let Some(entries) = usize::try_from(operation.index)
            .ok()
            .and_then(|index| self.entries.get(index))
        else {
            return false;
        };
        !entries.is_empty()
            && entries.len() >= self.capacity_per_slot
            && entries
                .values()
                .all(|authorization| has_consumer(authorization.operation()))
    }

    pub(super) fn retain(
        &mut self,
        authorization: ReleaseAuthorization,
    ) -> Result<(), ReleaseAuthorization> {
        let operation = authorization.operation();
        let Ok(index) = usize::try_from(operation.index) else {
            return Err(authorization);
        };
        let Some(entries) = self.entries.get_mut(index) else {
            return Err(authorization);
        };
        let has_capacity = entries.len() < self.capacity_per_slot;
        match entries.entry(operation) {
            Entry::Occupied(mut entry) => {
                entry.insert(authorization);
                Ok(())
            }
            Entry::Vacant(entry) if has_capacity => {
                entry.insert(authorization);
                Ok(())
            }
            Entry::Vacant(_) => Err(authorization),
        }
    }

    pub(super) fn retain_with_cleanup(
        &mut self,
        authorization: ReleaseAuthorization,
        mut can_retire: impl FnMut(OperationSlotToken) -> bool,
    ) -> Result<(), ReleaseAuthorization> {
        let operation = authorization.operation();
        let Ok(index) = usize::try_from(operation.index) else {
            return Err(authorization);
        };
        let Some(entries) = self.entries.get_mut(index) else {
            return Err(authorization);
        };
        if let Entry::Occupied(mut entry) = entries.entry(operation) {
            entry.insert(authorization);
            return Ok(());
        }
        entries.retain(|operation, _| !can_retire(*operation));
        self.retain(authorization)
    }

    pub(super) fn retire(&mut self, operation: OperationSlotToken) -> Option<ReleaseAuthorization> {
        let index = usize::try_from(operation.index).ok()?;
        self.entries.get_mut(index)?.remove(&operation)
    }

    #[cfg(test)]
    pub(super) fn set_capacity_per_slot_for_test(&mut self, capacity: usize) {
        self.capacity_per_slot = capacity;
    }
}

#[cfg(test)]
mod tests {
    use super::ReleaseAuthorizationLedger;
    use crate::evidence::ReleaseAuthorization;
    use dwv_core::{
        BlockOp, BlockRequest, ByteRange, DurabilityIntent, FenceDomain, FrontendId,
        OrderingIntent, RequestId, SlotId, SubmissionSequence, TopologyEpoch,
    };
    use dwv_lifecycle_authority::LifecycleAuthorityOwner;
    use dwv_store::{
        OperationSlotTable, OperationSlotToken, ReconciliationOutcome, ResourceLimits,
    };

    fn authorization(
        operations: &mut OperationSlotTable,
        owner: &LifecycleAuthorityOwner,
        request_id: u64,
    ) -> (OperationSlotToken, ReleaseAuthorization) {
        let request = BlockRequest::new(
            RequestId(request_id),
            FrontendId(1),
            SlotId([0; 16]),
            TopologyEpoch(1),
            BlockOp::Write,
            ByteRange::new(0, 1).expect("test range is valid"),
            None,
            OrderingIntent {
                submission_sequence: SubmissionSequence(request_id),
                preflush: false,
                fence_domain: FenceDomain(1),
            },
            DurabilityIntent::Ordinary,
        );
        let operation = operations.reserve(request).expect("test slot is available");
        operations
            .record_reconciliation(operation, ReconciliationOutcome::Durable)
            .expect("empty operation is reclaimable");
        let lifecycle = owner
            .authorize_release(operations, operation, true, true, true, true)
            .expect("test operation has complete release facts");
        (operation, ReleaseAuthorization::from_lifecycle(lifecycle))
    }

    #[test]
    fn retention_is_exact_and_capacity_does_not_overwrite() {
        let mut operations = OperationSlotTable::new(ResourceLimits::new(1, 0, 0, 0, 0, 0));
        let (owner, _verifier) = LifecycleAuthorityOwner::new();
        let (first_operation, first) = authorization(&mut operations, &owner, 1);
        let mut ledger = ReleaseAuthorizationLedger::new(1, 2);

        ledger
            .retain(first.clone())
            .expect("first generation is retained");
        operations
            .release(first_operation)
            .expect("reclaimable test slot releases");
        let (second_operation, second) = authorization(&mut operations, &owner, 2);
        ledger
            .retain(second.clone())
            .expect("second generation is retained beside the first");

        assert_eq!(ledger.observe(first_operation), Some(&first));
        assert_eq!(ledger.observe(second_operation), Some(&second));
        assert_eq!(
            ledger
                .operations_at_index(second_operation)
                .collect::<Vec<_>>(),
            vec![first_operation, second_operation]
        );
        let next_generation =
            OperationSlotToken::new(second_operation.index, second_operation.generation + 1);
        assert!(ledger.is_full_and_consumed(next_generation, |_| true));
        assert!(!ledger.is_full_and_consumed(next_generation, |operation| {
            operation == first_operation
        }));

        operations
            .release(second_operation)
            .expect("second reclaimable test slot releases");
        let (third_operation, third) = authorization(&mut operations, &owner, 3);
        assert!(ledger.retain(third.clone()).is_err());
        assert_eq!(ledger.observe(first_operation), Some(&first));
        assert_eq!(ledger.observe(second_operation), Some(&second));
        assert_eq!(ledger.observe(third_operation), None);

        assert!(ledger.retire(first_operation).is_some());
        ledger
            .retain(third.clone())
            .expect("capacity is available after exact retirement");
        assert_eq!(ledger.observe(first_operation), None);
        assert_eq!(ledger.observe(second_operation), Some(&second));
        assert_eq!(ledger.observe(third_operation), Some(&third));
    }
}
