use crate::{
    admission::slot_error,
    evidence::{CompletionEvidence, PersistenceClaim},
    failure::{FailureClass, ServiceError},
    range::RangePlan,
};
use dwv_core::{BlockRequest, ByteRange};
use dwv_store::{CompletionDisposition, OperationSlotToken, RandomAccessStore};

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
    let mut completed_total = 0_u64;
    let mut byte_cursor = 0_usize;
    let children = admission
        .children(token, &plan.ranges)
        .map_err(slot_error)?;
    admission
        .submit_all(token, children.len())
        .map_err(slot_error)?;
    for (range, child) in plan.ranges.iter().zip(children) {
        let length = usize::try_from(range.length).map_err(|_| {
            ServiceError::io(FailureClass::Range, "child range does not fit memory")
        })?;
        let completion =
            store.read_at(child, *range, &mut bytes[byte_cursor..byte_cursor + length]);
        completed_total += completion
            .completed
            .as_slice()
            .iter()
            .map(|range| range.length)
            .sum::<u64>();
        let disposition = completion.disposition.clone();
        admission.complete(token, completion).map_err(slot_error)?;
        if !matches!(disposition, CompletionDisposition::Success) {
            bytes.truncate(completed_total as usize);
            return Err(ServiceError::incomplete_read(
                request,
                bytes,
                CompletionEvidence {
                    requested,
                    completed: completed_total,
                    disposition,
                    persistence: PersistenceClaim::VolatileOrUnknown,
                },
            ));
        }
        byte_cursor += length;
    }
    Ok((
        bytes,
        CompletionEvidence {
            requested,
            completed: requested.length,
            disposition: CompletionDisposition::Success,
            persistence: PersistenceClaim::VolatileOrUnknown,
        },
    ))
}
