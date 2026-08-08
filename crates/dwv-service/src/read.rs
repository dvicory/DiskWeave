use crate::{
    admission::{completion, slot_error},
    evidence::{CompletionEvidence, PersistenceClaim},
    failure::ServiceError,
    range::RangePlan,
};
use dwv_core::ByteRange;
use dwv_store::{CompletionDisposition, OperationSlotToken, PersistenceEvidence, StoreError};
use dwv_store_file::FileStore;

pub(crate) fn read_member(
    store: &mut FileStore,
    admission: &mut crate::OperationAdmission,
    token: OperationSlotToken,
    plan: &RangePlan,
) -> Result<(Vec<u8>, CompletionEvidence), ServiceError> {
    let mut bytes = Vec::new();
    let mut completed_total = 0_u64;
    let children = admission
        .children(token, &plan.ranges)
        .map_err(slot_error)?;
    admission
        .submit_all(token, children.len())
        .map_err(slot_error)?;
    for (range, child) in plan.ranges.iter().zip(children) {
        let progress = store.read_progress(*range);
        let (completion, error) = match progress {
            Ok(progress) => {
                let completed = progress.completed;
                bytes.extend_from_slice(&progress.bytes);
                completed_total += completed;
                let disposition = if completed == range.length {
                    CompletionDisposition::Success
                } else {
                    CompletionDisposition::Short
                };
                (
                    completion(
                        child,
                        *range,
                        completed,
                        disposition,
                        PersistenceEvidence::VolatileOrUnknown,
                    ),
                    None,
                )
            }
            Err(error) => (
                completion(
                    child,
                    *range,
                    0,
                    CompletionDisposition::Failed(StoreError::BackendFailure { code: 5 }),
                    PersistenceEvidence::VolatileOrUnknown,
                ),
                Some(error),
            ),
        };
        let failed =
            error.is_some() || !matches!(completion.disposition, CompletionDisposition::Success);
        admission.complete(token, completion).map_err(slot_error)?;
        if failed {
            let requested = ByteRange::new(
                plan.ranges[0].offset,
                plan.ranges.iter().map(|range| range.length).sum(),
            )
            .expect("range plan is contiguous");
            let disposition = if error.is_some() {
                CompletionDisposition::Failed(StoreError::BackendFailure { code: 5 })
            } else {
                CompletionDisposition::Short
            };
            return Err(ServiceError::incomplete_read(
                bytes,
                CompletionEvidence {
                    requested,
                    completed: completed_total,
                    disposition,
                    persistence: PersistenceClaim::VolatileOrUnknown,
                },
            ));
        }
    }
    Ok((
        bytes,
        CompletionEvidence {
            requested: ByteRange::new(
                plan.ranges[0].offset,
                plan.ranges.iter().map(|range| range.length).sum(),
            )
            .expect("range plan is contiguous"),
            completed: plan.ranges.iter().map(|range| range.length).sum(),
            disposition: CompletionDisposition::Success,
            persistence: PersistenceClaim::VolatileOrUnknown,
        },
    ))
}
