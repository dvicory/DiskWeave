use crate::{
    AdapterError, KernelOperation, KernelRequest, MAX_TRACE_RECORDS, MAX_TRANSFER, QUEUE_DEPTH,
    TerminalResult, TranslatedRequest, translate_request,
};
use dwv_core::{BlockOp, DurabilityIntent, SubmissionSequence, TopologyEpoch};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const TRACE_SCHEMA: &str = "dwv.ublk.trace.v3";
pub const MAX_TRACE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceDurability {
    Ordinary,
    ExplicitFlush,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedTraceRequest {
    pub request_id: u64,
    pub frontend_id: u64,
    pub slot_id: [u8; 16],
    pub topology_epoch: u64,
    pub operation: KernelOperation,
    pub offset: u64,
    pub length: u64,
    pub buffer_index: Option<u32>,
    pub buffer_generation: Option<u32>,
    pub submission_sequence: u64,
    pub preflush: bool,
    pub fence_domain: u64,
    pub durability: TraceDurability,
}

impl NormalizedTraceRequest {
    pub fn from_translated(translated: TranslatedRequest) -> Result<Self, AdapterError> {
        let request = translated.normalized;
        let operation = match request.op {
            BlockOp::Read => KernelOperation::Read,
            BlockOp::Write => KernelOperation::Write,
            BlockOp::Flush => KernelOperation::Flush,
            _ => {
                return Err(AdapterError::Invalid(
                    "normalized operation is outside the ublk profile",
                ));
            }
        };
        let durability = match request.durability {
            DurabilityIntent::Ordinary => TraceDurability::Ordinary,
            DurabilityIntent::ExplicitFlush => TraceDurability::ExplicitFlush,
            DurabilityIntent::Fua => {
                return Err(AdapterError::Invalid(
                    "FUA cannot enter the ublk normalized trace",
                ));
            }
        };
        Ok(Self {
            request_id: request.request_id.0,
            frontend_id: request.frontend_id.0,
            slot_id: request.slot_id.0,
            topology_epoch: request.topology_epoch.0,
            operation,
            offset: request.range.offset,
            length: request.range.length,
            buffer_index: request.buffer.map(|buffer| buffer.index),
            buffer_generation: request.buffer.map(|buffer| buffer.generation),
            submission_sequence: request.ordering.submission_sequence.0,
            preflush: request.ordering.preflush,
            fence_domain: request.ordering.fence_domain.0,
            durability,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum KernelCompletion {
    Success { bytes: u32 },
    Error { terminal: TerminalResult },
    Abandoned,
    MappingRefused,
}

pub fn map_kernel_completion(
    terminal: TerminalResult,
    completed_length: Option<u64>,
) -> Result<KernelCompletion, AdapterError> {
    if terminal != TerminalResult::Success {
        return Ok(KernelCompletion::Error { terminal });
    }
    let length = completed_length.ok_or(AdapterError::Invalid(
        "successful semantic result lacks a completion length",
    ))?;
    let bytes = u32::try_from(length)
        .ok()
        .filter(|bytes| i32::try_from(*bytes).is_ok())
        .ok_or(AdapterError::Invalid(
            "successful semantic result exceeds the Linux completion range",
        ))?;
    Ok(KernelCompletion::Success { bytes })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceRecord {
    pub sequence: u64,
    pub tag: u16,
    pub generation: u32,
    pub kernel_submission: KernelRequest,
    pub normalized_request: Option<NormalizedTraceRequest>,
    pub semantic_result: TerminalResult,
    pub kernel_completion: KernelCompletion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceBounds {
    pub queue_depth: usize,
    pub maximum_transfer: u64,
    pub maximum_records: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceDocument {
    pub schema: String,
    pub fixture_digest: String,
    pub capacity: u64,
    pub topology_epoch: u64,
    pub bounds: TraceBounds,
    /// Number of reservations refused because no slot was safely available.
    pub exhausted_records: u64,
    /// Number of completed records retired from the retained window.
    pub retired_records: u64,
    /// True once the retired-record count reached its representational bound.
    pub retired_records_saturated: bool,
    pub records: Vec<TraceRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TraceReplaySummary {
    pub schema: String,
    pub record_count: usize,
    pub clean: bool,
    pub partial_session: bool,
}

impl TraceDocument {
    pub fn from_json(bytes: &[u8]) -> Result<Self, AdapterError> {
        if bytes.len() > MAX_TRACE_BYTES {
            return Err(AdapterError::Exhausted("trace byte bound exceeded"));
        }
        let trace: Self = serde_json::from_slice(bytes)
            .map_err(|error| AdapterError::Io(format!("invalid frontend trace: {error}")))?;
        trace.replay()?;
        Ok(trace)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, AdapterError> {
        let bytes =
            serde_json::to_vec_pretty(self).map_err(|error| AdapterError::Io(error.to_string()))?;
        if bytes.len() > MAX_TRACE_BYTES {
            return Err(AdapterError::Exhausted("trace byte bound exceeded"));
        }
        Ok(bytes)
    }

    pub fn replay(&self) -> Result<TraceReplaySummary, AdapterError> {
        if self.schema != TRACE_SCHEMA {
            return Err(AdapterError::Unsupported("frontend trace schema"));
        }
        if self.retired_records_saturated && self.retired_records != u64::MAX {
            return Err(AdapterError::Invalid(
                "frontend trace retired-record accounting is invalid",
            ));
        }
        if self.fixture_digest.is_empty()
            || self.records.len() > self.bounds.maximum_records
            || self.bounds.maximum_records > MAX_TRACE_RECORDS
            || self.bounds.queue_depth == 0
            || self.bounds.queue_depth > QUEUE_DEPTH
            || self.bounds.maximum_transfer == 0
            || self.bounds.maximum_transfer > MAX_TRANSFER
        {
            return Err(AdapterError::Invalid("frontend trace bounds or identity"));
        }
        let partial_session = self.retired_records > 0 || self.retired_records_saturated;
        let mut previous = 0;
        let mut clean = self.exhausted_records == 0 && !partial_session;
        for record in &self.records {
            if record.sequence <= previous
                || usize::from(record.tag) >= self.bounds.queue_depth
                || record.kernel_submission.tag != record.tag
            {
                return Err(AdapterError::Invalid(
                    "frontend trace sequence or tag is invalid",
                ));
            }
            previous = record.sequence;
            if record.generation == 0 {
                if record.normalized_request.is_some()
                    || record.semantic_result != TerminalResult::ResourceExhausted
                {
                    return Err(AdapterError::Conflict(format!(
                        "trace admission refusal diverged at sequence {}",
                        record.sequence
                    )));
                }
            } else {
                match translate_request(
                    record.kernel_submission,
                    self.capacity,
                    TopologyEpoch(self.topology_epoch),
                    SubmissionSequence(record.sequence),
                    record.generation,
                ) {
                    Ok(translated) => {
                        let expected = NormalizedTraceRequest::from_translated(translated)?;
                        if record.normalized_request != Some(expected) {
                            return Err(AdapterError::Conflict(format!(
                                "trace normalized request diverged at sequence {}",
                                record.sequence
                            )));
                        }
                    }
                    Err(error) => {
                        if record.normalized_request.is_some()
                            || record.semantic_result != error.terminal()
                        {
                            return Err(AdapterError::Conflict(format!(
                                "trace rejection diverged at sequence {}",
                                record.sequence
                            )));
                        }
                    }
                }
            }
            let completed_length = record.normalized_request.map(|request| request.length);
            let expected = map_kernel_completion(record.semantic_result, completed_length);
            match record.kernel_completion {
                KernelCompletion::Abandoned => clean = false,
                KernelCompletion::MappingRefused if expected.is_err() => clean = false,
                completion if expected.as_ref() == Ok(&completion) => {}
                _ => {
                    return Err(AdapterError::Conflict(format!(
                        "trace kernel completion diverged at sequence {}",
                        record.sequence
                    )));
                }
            }
        }
        Ok(TraceReplaySummary {
            schema: self.schema.clone(),
            record_count: self.records.len(),
            clean,
            partial_session,
        })
    }
}

pub fn replay_trace_file(path: &Path) -> Result<TraceReplaySummary, AdapterError> {
    let length = fs::metadata(path)
        .map_err(|error| AdapterError::Io(error.to_string()))?
        .len();
    if length > MAX_TRACE_BYTES as u64 {
        return Err(AdapterError::Exhausted("trace byte bound exceeded"));
    }
    let bytes = fs::read(path).map_err(|error| AdapterError::Io(error.to_string()))?;
    TraceDocument::from_json(&bytes)?.replay()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceReservation {
    index: usize,
    generation: u64,
}

#[derive(Clone, Debug)]
enum TraceSlot {
    Available,
    Reserved {
        generation: u64,
    },
    Terminal {
        generation: u64,
        record: TraceRecord,
        reclaimable: bool,
    },
}

pub struct TraceLog {
    records: Vec<TraceSlot>,
    exhausted_records: u64,
    retired_records: u64,
    retired_records_saturated: bool,
}

impl Default for TraceLog {
    fn default() -> Self {
        Self {
            records: vec![TraceSlot::Available; MAX_TRACE_RECORDS],
            exhausted_records: 0,
            retired_records: 0,
            retired_records_saturated: false,
        }
    }
}
#[cfg(test)]
impl TraceLog {
    pub(crate) fn set_retired_records_for_test(&mut self, count: u64) {
        self.retired_records = count;
        self.retired_records_saturated = count == u64::MAX;
    }
}

impl TraceLog {
    pub fn reserve(&mut self) -> Result<TraceReservation, AdapterError> {
        let index = self
            .records
            .iter()
            .position(|slot| matches!(slot, TraceSlot::Available))
            .or_else(|| {
                self.records
                    .iter()
                    .enumerate()
                    .filter_map(|(index, slot)| match slot {
                        TraceSlot::Terminal {
                            record,
                            reclaimable: true,
                            ..
                        } => Some((index, record.sequence)),
                        _ => None,
                    })
                    .min_by_key(|(_, sequence)| *sequence)
                    .map(|(index, _)| index)
            });
        let Some(index) = index else {
            self.exhausted_records = self.exhausted_records.saturating_add(1);
            return Err(AdapterError::Exhausted(
                "no safely reclaimable trace record",
            ));
        };
        let generation = match &self.records[index] {
            TraceSlot::Available => 1,
            TraceSlot::Terminal { generation, .. } => generation.checked_add(1).ok_or(
                AdapterError::Exhausted("trace reservation generation exhausted"),
            )?,
            TraceSlot::Reserved { .. } => {
                return Err(AdapterError::Conflict(
                    "trace reservation selected an active slot".into(),
                ));
            }
        };
        if matches!(self.records[index], TraceSlot::Terminal { .. }) {
            self.retired_records = self.retired_records.saturating_add(1);
            self.retired_records_saturated = self.retired_records == u64::MAX;
        }
        self.records[index] = TraceSlot::Reserved { generation };
        Ok(TraceReservation { index, generation })
    }

    pub fn complete(
        &mut self,
        reservation: TraceReservation,
        record: TraceRecord,
    ) -> Result<(), AdapterError> {
        let slot = self
            .records
            .get_mut(reservation.index)
            .ok_or(AdapterError::Invalid("unknown trace reservation"))?;
        match slot {
            TraceSlot::Reserved { generation } if *generation == reservation.generation => {
                *slot = TraceSlot::Terminal {
                    generation: reservation.generation,
                    record,
                    reclaimable: false,
                };
                Ok(())
            }
            TraceSlot::Reserved { .. } => Err(AdapterError::Conflict(
                "stale trace reservation completion".into(),
            )),
            TraceSlot::Available | TraceSlot::Terminal { .. } => Err(AdapterError::Conflict(
                "trace reservation is not incomplete".into(),
            )),
        }
    }

    /// Marks a terminal record reclaimable after its operation owner releases resources.
    ///
    /// Abandoned records stay non-reclaimable because their terminal ownership has not
    /// been reconciled.
    pub fn mark_reclaimable(&mut self, reservation: TraceReservation) -> Result<(), AdapterError> {
        let slot = self
            .records
            .get_mut(reservation.index)
            .ok_or(AdapterError::Invalid("unknown trace reservation"))?;
        match slot {
            TraceSlot::Terminal {
                generation,
                record,
                reclaimable,
            } if *generation == reservation.generation => {
                if matches!(record.kernel_completion, KernelCompletion::Abandoned) {
                    return Err(AdapterError::Conflict(
                        "abandoned trace reservation is not reclaimable".into(),
                    ));
                }
                if *reclaimable {
                    return Err(AdapterError::Conflict(
                        "trace reservation is already reclaimable".into(),
                    ));
                }
                *reclaimable = true;
                Ok(())
            }
            TraceSlot::Terminal { .. } => {
                Err(AdapterError::Conflict("stale trace reclamation".into()))
            }
            TraceSlot::Reserved { .. } => Err(AdapterError::Conflict(
                "incomplete trace reservation is not reclaimable".into(),
            )),
            TraceSlot::Available => Err(AdapterError::Conflict("unknown trace reservation".into())),
        }
    }

    pub fn document(
        &self,
        fixture_digest: String,
        capacity: u64,
        topology_epoch: u64,
    ) -> Result<TraceDocument, AdapterError> {
        let mut records = Vec::new();
        for slot in &self.records {
            match slot {
                TraceSlot::Available => {}
                TraceSlot::Reserved { .. } => {
                    return Err(AdapterError::ReconciliationRequired(
                        "frontend trace contains an incomplete reservation".into(),
                    ));
                }
                TraceSlot::Terminal { record, .. } => records.push(record.clone()),
            }
        }
        records.sort_unstable_by_key(|record| record.sequence);
        Ok(TraceDocument {
            schema: TRACE_SCHEMA.into(),
            fixture_digest,
            capacity,
            topology_epoch,
            bounds: TraceBounds {
                queue_depth: QUEUE_DEPTH,
                maximum_transfer: MAX_TRANSFER,
                maximum_records: MAX_TRACE_RECORDS,
            },
            exhausted_records: self.exhausted_records,
            retired_records: self.retired_records,
            retired_records_saturated: self.retired_records_saturated,
            records,
        })
    }
}
