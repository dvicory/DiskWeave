use crate::{
    admission::slot_error,
    failure::{FailureClass, ServiceError},
};
use dwv_codec::{Geometry as CodecGeometry, ParityCodec, XorReference};
use dwv_core::ByteRange;
use dwv_store::{
    CompletionDisposition, OperationSlotToken, RandomAccessStore, StoreWriteWatermark, WriteIntent,
};

pub(crate) fn update_parity(
    geometry: &CodecGeometry,
    range: ByteRange,
    slot: usize,
    old_data: &[u8],
    new_data: &[u8],
    old_parity: &[u8],
) -> Result<Vec<u8>, ServiceError> {
    if range.is_empty()
        || old_data.len() as u64 != range.length
        || new_data.len() as u64 != range.length
        || old_parity.len() as u64 != range.length
    {
        return Err(ServiceError::io(
            FailureClass::Range,
            "parity update buffers do not exactly cover the range",
        ));
    }
    let local_geometry =
        CodecGeometry::new(vec![range.length; geometry.data_count()], range.length)
            .map_err(|error| ServiceError::io(FailureClass::Range, error.to_string()))?;
    let mut parity = old_parity.to_vec();
    XorReference
        .update_parity(
            &local_geometry,
            &mut parity,
            slot,
            dwv_codec::ByteRange::new(0, range.length)
                .map_err(|error| ServiceError::io(FailureClass::Range, error.to_string()))?,
            old_data,
            new_data,
        )
        .map_err(|error| ServiceError::io(FailureClass::StoreWrite, error.to_string()))?;
    Ok(parity)
}

pub(crate) fn write_member<S: RandomAccessStore>(
    store: &mut S,
    admission: &mut crate::OperationAdmission,
    token: OperationSlotToken,
    child: dwv_store::ChildOperationId,
    range: ByteRange,
    bytes: &[u8],
    intent: WriteIntent,
) -> Result<StoreWriteWatermark, ServiceError> {
    let result = store.write_at(child, range, bytes, intent);
    let disposition = result.disposition.clone();
    let watermark = result.write_watermark;
    admission.complete(token, result).map_err(slot_error)?;
    if !matches!(disposition, CompletionDisposition::Success) {
        return Err(ServiceError::io(
            FailureClass::StoreWrite,
            "home mutation did not complete",
        ));
    }
    watermark.ok_or_else(|| {
        ServiceError::io(
            FailureClass::Fence,
            "successful home mutation lacks a store write watermark",
        )
    })
}
