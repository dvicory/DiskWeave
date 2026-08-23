use crate::{
    admission::slot_error,
    evidence::{CompletionEvidence, PersistenceClaim},
    failure::{FailureClass, ServiceError},
    range::RangePlan,
};
use dwv_core::{BlockRequest, ByteRange};
use dwv_store::{
    CompletedRangeSet, CompletionDisposition, OperationSlotToken, RandomAccessStore,
    StoreCompletionDelivery,
};

pub(crate) fn read_member<S: RandomAccessStore>(
    store: &mut S,
    admission: &mut crate::OperationAdmission,
    request: BlockRequest,
    token: OperationSlotToken,
    plan: &RangePlan,
) -> Result<(Vec<u8>, CompletionEvidence), ServiceError> {
    let requested = ByteRange::new(
        plan.ranges[0].offset,
        plan.ranges.iter().map(|range| range.length).sum(),
    )
    .expect("range plan is contiguous");
    let requested_length = usize::try_from(requested.length)
        .map_err(|_| ServiceError::io(FailureClass::Range, "read range does not fit memory"))?;
    let mut bytes = vec![0; requested_length];
    let mut completed_ranges = Vec::new();
    let mut all_durable = true;
    let mut byte_cursor = 0_usize;
    let children = admission
        .children(token, &plan.ranges)
        .map_err(slot_error)?;
    for (range, child) in plan.ranges.iter().zip(children) {
        let length = usize::try_from(range.length).map_err(|_| {
            ServiceError::io(FailureClass::Range, "child range does not fit memory")
        })?;
        let identity = admission
            .accept(
                token,
                child,
                store.store_id(),
                store.incarnation(),
                store.topology_epoch(),
            )
            .map_err(slot_error)?;
        let completion =
            store.read_at(child, *range, &mut bytes[byte_cursor..byte_cursor + length]);
        let reported = completion.clone();
        let child_completed = completion.completed.as_slice().to_vec();
        let disposition = completion.disposition.clone();
        all_durable &= completion.persistence.is_durable();
        admission
            .deliver(StoreCompletionDelivery {
                identity,
                completion,
            })
            .map_err(|error| {
                ServiceError::rejected_completion(
                    FailureClass::Admission,
                    reported.clone(),
                    error.to_string(),
                )
            })?;
        completed_ranges.extend(child_completed);
        if !matches!(disposition, CompletionDisposition::Success) {
            let completed = CompletedRangeSet::new(completed_ranges)
                .map_err(|error| ServiceError::io(FailureClass::StoreRead, error.to_string()))?;
            return Err(ServiceError::incomplete_read(
                request,
                bytes,
                CompletionEvidence {
                    requested,
                    completed,
                    disposition,
                    persistence: if all_durable {
                        PersistenceClaim::HostFenceOnly
                    } else {
                        PersistenceClaim::VolatileOrUnknown
                    },
                },
            ));
        }
        byte_cursor += length;
    }
    let completed = CompletedRangeSet::new(completed_ranges)
        .map_err(|error| ServiceError::io(FailureClass::StoreRead, error.to_string()))?;
    Ok((
        bytes,
        CompletionEvidence {
            requested,
            completed,
            disposition: CompletionDisposition::Success,
            persistence: if all_durable {
                PersistenceClaim::HostFenceOnly
            } else {
                PersistenceClaim::VolatileOrUnknown
            },
        },
    ))
}
