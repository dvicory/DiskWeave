use crate::failure::{FailureClass, ServiceError};
use dwv_codec::{Geometry as CodecGeometry, ParityCodec, XorReference};
use dwv_core::ByteRange;

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
