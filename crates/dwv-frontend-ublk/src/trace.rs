use crate::{
    AdapterError, KernelOperation, KernelRequest, MAX_TRACE_RECORDS, MAX_TRANSFER, QUEUE_DEPTH,
    TerminalResult, TranslatedRequest, translate_request,
};
use dwv_core::{BlockOp, DurabilityIntent, SubmissionSequence, TopologyEpoch};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const TRACE_SCHEMA: &str = "dwv.ublk.trace.v2";
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
    pub exhausted_records: u64,
    pub records: Vec<TraceRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TraceReplaySummary {
    pub schema: String,
    pub record_count: usize,
    pub clean: bool,
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
        let mut previous = 0;
        let mut clean = self.exhausted_records == 0;
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
pub struct TraceReservation(usize);

#[derive(Default)]
pub struct TraceLog {
    records: Vec<Option<TraceRecord>>,
    exhausted_records: u64,
}

impl TraceLog {
    pub fn reserve(&mut self) -> Result<TraceReservation, AdapterError> {
        if self.records.len() == MAX_TRACE_RECORDS {
            self.exhausted_records = self.exhausted_records.saturating_add(1);
            return Err(AdapterError::Exhausted("trace record bound reached"));
        }
        let reservation = TraceReservation(self.records.len());
        self.records.push(None);
        Ok(reservation)
    }

    pub fn complete(
        &mut self,
        reservation: TraceReservation,
        record: TraceRecord,
    ) -> Result<(), AdapterError> {
        let slot = self
            .records
            .get_mut(reservation.0)
            .ok_or(AdapterError::Invalid("unknown trace reservation"))?;
        if slot.is_some() {
            return Err(AdapterError::Conflict(
                "trace reservation already completed".into(),
            ));
        }
        *slot = Some(record);
        Ok(())
    }

    pub fn document(
        &self,
        fixture_digest: String,
        capacity: u64,
        topology_epoch: u64,
    ) -> Result<TraceDocument, AdapterError> {
        let mut records = self
            .records
            .iter()
            .cloned()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                AdapterError::ReconciliationRequired(
                    "frontend trace contains an incomplete reservation".into(),
                )
            })?;
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
            records,
        })
    }
}
