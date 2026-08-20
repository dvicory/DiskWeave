use serde::{Deserialize, Serialize};
use std::fmt;

pub const TRACE_VERSION: u16 = 2;
pub const LEGACY_TRACE_VERSION: u16 = 0;
pub const PREVIOUS_TRACE_VERSION: u16 = 1;
pub const MAX_EVENTS: usize = 512;
pub const MAX_TRACE_BYTES: usize = 1024 * 1024;
pub const MAX_EVENT_RANGE: u64 = 1024 * 1024;
pub const MAX_FIXTURE_SIZE: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceFixture {
    pub size: u64,
    pub seed: u64,
    pub write_length: u64,
}

impl TraceFixture {
    pub fn validate(&self) -> Result<(), TraceError> {
        if self.size == 0 || self.size > MAX_FIXTURE_SIZE {
            return Err(TraceError::Invalid(format!(
                "fixture size must be between 1 and {MAX_FIXTURE_SIZE}"
            )));
        }
        if self.write_length == 0 || self.write_length > self.size {
            return Err(TraceError::Invalid(
                "fixture write length must be non-empty and within the fixture".to_owned(),
            ));
        }
        if self.write_length > MAX_EVENT_RANGE {
            return Err(TraceError::Invalid(format!(
                "fixture write length exceeds {MAX_EVENT_RANGE}"
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum PayloadPattern {
    Zeroes,
    Counter { seed: u64 },
}

pub fn materialize_pattern(pattern: PayloadPattern, length: u64) -> Result<Vec<u8>, TraceError> {
    if length == 0 || length > MAX_EVENT_RANGE {
        return Err(TraceError::Invalid(format!(
            "pattern length must be between 1 and {MAX_EVENT_RANGE}"
        )));
    }
    let length = usize::try_from(length)
        .map_err(|_| TraceError::Invalid("pattern length does not fit usize".to_owned()))?;
    match pattern {
        PayloadPattern::Zeroes => Ok(vec![0; length]),
        PayloadPattern::Counter { seed } => Ok((0..length)
            .map(|index| seed.wrapping_add(index as u64) as u8)
            .collect()),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ChecksumOutcome {
    CurrentMatch,
    CurrentMismatch,
    Absent,
    Uncertain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RepairDecision {
    Verified,
    Refused,
    Uncertain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TraceOutcome {
    Success,
    Refused,
    Uncertain,
    ReplayMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TraceEventKind {
    Open {
        size: u64,
    },
    Write {
        store: u8,
        offset: u64,
        length: u64,
        pattern: PayloadPattern,
    },
    Flush {
        store: u8,
        through: u64,
    },
    Read {
        store: u8,
        offset: u64,
        length: u64,
    },
    WriteRecoveryRecord {
        generation: u64,
    },
    RecoveryClean {
        generation: u64,
        durable: bool,
    },
    Checksum {
        region: u64,
        outcome: ChecksumOutcome,
    },
    DegradedRead {
        slot: u8,
        offset: u64,
        length: u64,
    },
    RebuildChunk {
        offset: u64,
        length: u64,
        verified: bool,
    },
    RepairDecision {
        target: u8,
        decision: RepairDecision,
    },
    Outcome {
        outcome: TraceOutcome,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceEvent {
    pub sequence: u32,
    pub kind: TraceEventKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trace {
    pub schema: u16,
    pub fixture: TraceFixture,
    pub events: Vec<TraceEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceSummary {
    pub schema: u16,
    pub event_count: usize,
    pub fixture_size: u64,
    pub write_length: u64,
    pub terminal: Option<TraceOutcome>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntegrityState {
    Unknown,
    Clean,
    Dirty,
    Invalid,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TraceModelState {
    pub open: bool,
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub flushes: u64,
    pub recovery_generation: u64,
    pub integrity: IntegrityState,
    pub terminal: Option<TraceOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceError {
    Invalid(String),
    Limit(String),
    Json(String),
    UnsupportedVersion(u16),
}

impl fmt::Display for TraceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid trace: {message}"),
            Self::Limit(message) => write!(formatter, "trace limit exceeded: {message}"),
            Self::Json(message) => write!(formatter, "invalid trace JSON: {message}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported trace schema version {version}")
            }
        }
    }
}

impl std::error::Error for TraceError {}

impl Trace {
    pub fn new(fixture: TraceFixture) -> Result<Self, TraceError> {
        fixture.validate()?;
        Ok(Self {
            schema: TRACE_VERSION,
            fixture,
            events: Vec::new(),
        })
    }

    pub fn push(&mut self, kind: TraceEventKind) -> Result<(), TraceError> {
        if self.events.len() >= MAX_EVENTS {
            return Err(TraceError::Limit(format!("maximum events: {MAX_EVENTS}")));
        }
        let sequence = u32::try_from(self.events.len())
            .map_err(|_| TraceError::Limit("event sequence overflow".to_owned()))?;
        self.events.push(TraceEvent { sequence, kind });
        Ok(())
    }

    pub fn summary(&self) -> TraceSummary {
        TraceSummary {
            schema: self.schema,
            event_count: self.events.len(),
            fixture_size: self.fixture.size,
            write_length: self.fixture.write_length,
            terminal: self.events.iter().rev().find_map(|event| match event.kind {
                TraceEventKind::Outcome { outcome } => Some(outcome),
                _ => None,
            }),
        }
    }

    pub fn model_state(&self) -> Result<TraceModelState, TraceError> {
        self.validate()?;
        let mut state = TraceModelState {
            open: false,
            bytes_written: 0,
            bytes_read: 0,
            flushes: 0,
            recovery_generation: 0,
            integrity: IntegrityState::Unknown,
            terminal: None,
        };
        for event in &self.events {
            match event.kind {
                TraceEventKind::Open { .. } => state.open = true,
                TraceEventKind::Write { length, .. } => state.bytes_written += length,
                TraceEventKind::Read { length, .. }
                | TraceEventKind::DegradedRead { length, .. } => state.bytes_read += length,
                TraceEventKind::Flush { .. } => state.flushes += 1,
                TraceEventKind::WriteRecoveryRecord { generation }
                | TraceEventKind::RecoveryClean { generation, .. } => {
                    state.recovery_generation = state.recovery_generation.max(generation)
                }
                TraceEventKind::Checksum { outcome, .. } => {
                    state.integrity = match outcome {
                        ChecksumOutcome::CurrentMatch => IntegrityState::Clean,
                        ChecksumOutcome::CurrentMismatch => IntegrityState::Invalid,
                        ChecksumOutcome::Absent => IntegrityState::Unknown,
                        ChecksumOutcome::Uncertain => IntegrityState::Uncertain,
                    }
                }
                TraceEventKind::RepairDecision { decision, .. } => {
                    if matches!(decision, RepairDecision::Uncertain) {
                        state.integrity = IntegrityState::Uncertain;
                    }
                }
                TraceEventKind::Outcome { outcome } => state.terminal = Some(outcome),
                TraceEventKind::RebuildChunk { .. } => {}
            }
        }
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), TraceError> {
        if self.schema != TRACE_VERSION {
            return Err(TraceError::UnsupportedVersion(self.schema));
        }
        self.fixture.validate()?;
        if self.events.is_empty() {
            return Err(TraceError::Invalid("trace must contain events".to_owned()));
        }
        if self.events.len() > MAX_EVENTS {
            return Err(TraceError::Limit(format!("maximum events: {MAX_EVENTS}")));
        }
        for (index, event) in self.events.iter().enumerate() {
            if event.sequence != index as u32 {
                return Err(TraceError::Invalid(format!(
                    "event sequence {} does not equal index {index}",
                    event.sequence
                )));
            }
            validate_event(&self.fixture, &event.kind)?;
        }
        if !matches!(
            self.events.first().map(|event| &event.kind),
            Some(TraceEventKind::Open { .. })
        ) {
            return Err(TraceError::Invalid("first event must be Open".to_owned()));
        }
        if !matches!(
            self.events.last().map(|event| &event.kind),
            Some(TraceEventKind::Outcome { .. })
        ) {
            return Err(TraceError::Invalid("last event must be Outcome".to_owned()));
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<Vec<u8>, TraceError> {
        self.validate()?;
        let bytes =
            serde_json::to_vec(self).map_err(|error| TraceError::Json(error.to_string()))?;
        if bytes.len() > MAX_TRACE_BYTES {
            return Err(TraceError::Limit(format!(
                "maximum bytes: {MAX_TRACE_BYTES}"
            )));
        }
        Ok(bytes)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, TraceError> {
        if bytes.len() > MAX_TRACE_BYTES {
            return Err(TraceError::Limit(format!(
                "maximum bytes: {MAX_TRACE_BYTES}"
            )));
        }
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|error| TraceError::Json(error.to_string()))?;
        let schema = value
            .get("schema")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| TraceError::Invalid("schema is required".to_owned()))?;
        let trace = match u16::try_from(schema).unwrap_or(u16::MAX) {
            TRACE_VERSION => serde_json::from_value(value)
                .map_err(|error| TraceError::Json(error.to_string()))?,
            PREVIOUS_TRACE_VERSION => migrate_previous(value)?,
            LEGACY_TRACE_VERSION => migrate_legacy(value)?,
            version => return Err(TraceError::UnsupportedVersion(version)),
        };
        trace.validate()?;
        Ok(trace)
    }

    pub fn minimize_prefix(&self, failing_sequence: usize) -> Result<Self, TraceError> {
        self.validate()?;
        if failing_sequence >= self.events.len() {
            return Err(TraceError::Invalid(
                "failing sequence is outside the trace".to_owned(),
            ));
        }
        let mut minimized = Self::new(self.fixture.clone())?;
        for event in self.events.iter().take(failing_sequence + 1) {
            if matches!(event.kind, TraceEventKind::Outcome { .. }) {
                minimized.push(event.kind.clone())?;
                break;
            }
            minimized.push(event.kind.clone())?;
        }
        if !matches!(
            minimized.events.last().map(|event| &event.kind),
            Some(TraceEventKind::Outcome { .. })
        ) {
            minimized.push(TraceEventKind::Outcome {
                outcome: TraceOutcome::ReplayMismatch,
            })?;
        }
        minimized.validate()?;
        Ok(minimized)
    }
}

fn validate_event(fixture: &TraceFixture, event: &TraceEventKind) -> Result<(), TraceError> {
    let validate_range = |offset: u64, length: u64| {
        if length == 0 || length > MAX_EVENT_RANGE {
            return Err(TraceError::Invalid(format!(
                "event range length must be between 1 and {MAX_EVENT_RANGE}"
            )));
        }
        let end = offset
            .checked_add(length)
            .ok_or_else(|| TraceError::Invalid("event range overflows".to_owned()))?;
        if end > fixture.size {
            return Err(TraceError::Invalid(
                "event range exceeds fixture".to_owned(),
            ));
        }
        Ok(())
    };
    match event {
        TraceEventKind::Open { size } if *size != fixture.size => Err(TraceError::Invalid(
            "Open size differs from fixture".to_owned(),
        )),
        TraceEventKind::Open { .. }
        | TraceEventKind::Flush { .. }
        | TraceEventKind::WriteRecoveryRecord { .. }
        | TraceEventKind::RecoveryClean { .. }
        | TraceEventKind::Checksum { .. }
        | TraceEventKind::RepairDecision { .. }
        | TraceEventKind::Outcome { .. } => Ok(()),
        TraceEventKind::Write { offset, length, .. }
        | TraceEventKind::Read { offset, length, .. }
        | TraceEventKind::DegradedRead { offset, length, .. }
        | TraceEventKind::RebuildChunk { offset, length, .. } => validate_range(*offset, *length),
    }
}

fn rename_prior_event_names(value: &mut serde_json::Value) -> Result<(), TraceError> {
    let events = value
        .get_mut("events")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| TraceError::Invalid("events are required for migration".to_owned()))?;
    for event in events {
        let kind = event
            .get_mut("kind")
            .and_then(serde_json::Value::as_object_mut)
            .ok_or_else(|| TraceError::Invalid("trace event kind is required".to_owned()))?;
        for (old, new) in [
            ("RecoveryIntent", "WriteRecoveryRecord"),
            ("Checkpoint", "RecoveryClean"),
        ] {
            if let Some(payload) = kind.remove(old) {
                if kind.contains_key(new) {
                    return Err(TraceError::Invalid(format!(
                        "trace event contains both {old} and {new}"
                    )));
                }
                kind.insert(new.to_owned(), payload);
            }
        }
    }
    Ok(())
}

fn migrate_previous(mut value: serde_json::Value) -> Result<Trace, TraceError> {
    rename_prior_event_names(&mut value)?;
    value["schema"] = serde_json::json!(TRACE_VERSION);
    serde_json::from_value(value).map_err(|error| TraceError::Json(error.to_string()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTrace {
    schema: u16,
    size: u64,
    seed: u64,
    #[serde(default = "legacy_write_length")]
    write_length: u64,
    events: Vec<TraceEvent>,
}

fn legacy_write_length() -> u64 {
    4096
}

fn migrate_legacy(mut value: serde_json::Value) -> Result<Trace, TraceError> {
    rename_prior_event_names(&mut value)?;
    let legacy: LegacyTrace =
        serde_json::from_value(value).map_err(|error| TraceError::Json(error.to_string()))?;
    if legacy.schema != LEGACY_TRACE_VERSION {
        return Err(TraceError::UnsupportedVersion(legacy.schema));
    }
    let trace = Trace {
        schema: TRACE_VERSION,
        fixture: TraceFixture {
            size: legacy.size,
            seed: legacy.seed,
            write_length: legacy.write_length,
        },
        events: legacy.events,
    };
    trace.validate()?;
    Ok(trace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace() -> Trace {
        let mut trace = Trace::new(TraceFixture {
            size: 16 * 1024,
            seed: 7,
            write_length: 4096,
        })
        .unwrap();
        trace
            .push(TraceEventKind::Open { size: 16 * 1024 })
            .unwrap();
        trace
            .push(TraceEventKind::Write {
                store: 0,
                offset: 0,
                length: 4096,
                pattern: PayloadPattern::Counter { seed: 11 },
            })
            .unwrap();
        trace
            .push(TraceEventKind::Flush {
                store: 0,
                through: 1,
            })
            .unwrap();
        trace
            .push(TraceEventKind::Checksum {
                region: 0,
                outcome: ChecksumOutcome::CurrentMatch,
            })
            .unwrap();
        trace
            .push(TraceEventKind::Outcome {
                outcome: TraceOutcome::Success,
            })
            .unwrap();
        trace
    }

    fn next_seed(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *seed
    }

    fn generated_trace(mut seed: u64) -> Trace {
        let blocks = seed % 32 + 1;
        let size = blocks * 512;
        let write_blocks = (next_seed(&mut seed) % blocks) + 1;
        let start = next_seed(&mut seed) % (blocks - write_blocks + 1) * 512;
        let length = write_blocks * 512;
        let mut trace = Trace::new(TraceFixture {
            size,
            seed,
            write_length: length,
        })
        .unwrap();
        trace.push(TraceEventKind::Open { size }).unwrap();
        trace
            .push(TraceEventKind::Write {
                store: (next_seed(&mut seed) & 1) as u8,
                offset: start,
                length,
                pattern: PayloadPattern::Counter {
                    seed: next_seed(&mut seed),
                },
            })
            .unwrap();
        if seed & 1 != 0 {
            trace
                .push(TraceEventKind::Read {
                    store: 0,
                    offset: start,
                    length,
                })
                .unwrap();
        }
        if seed & 2 != 0 {
            trace
                .push(TraceEventKind::DegradedRead {
                    slot: 1,
                    offset: start,
                    length,
                })
                .unwrap();
        }
        if seed & 4 != 0 {
            trace
                .push(TraceEventKind::RebuildChunk {
                    offset: start,
                    length,
                    verified: seed & 8 != 0,
                })
                .unwrap();
        }
        trace
            .push(TraceEventKind::Checksum {
                region: start,
                outcome: if seed & 16 != 0 {
                    ChecksumOutcome::CurrentMismatch
                } else {
                    ChecksumOutcome::CurrentMatch
                },
            })
            .unwrap();
        trace
            .push(TraceEventKind::Outcome {
                outcome: if seed & 32 != 0 {
                    TraceOutcome::Uncertain
                } else {
                    TraceOutcome::Success
                },
            })
            .unwrap();
        trace
    }

    fn mutate_json(mut bytes: Vec<u8>, mut seed: u64) -> Vec<u8> {
        for _ in 0..=next_seed(&mut seed) % 6 {
            let operation = next_seed(&mut seed) % 4;
            match operation {
                0 if !bytes.is_empty() => {
                    let index = next_seed(&mut seed) as usize % bytes.len();
                    bytes[index] ^= 1 << (next_seed(&mut seed) % 8);
                }
                1 => {
                    let index = next_seed(&mut seed) as usize % (bytes.len() + 1);
                    bytes.insert(index, b'X');
                }
                2 if !bytes.is_empty() => {
                    let index = next_seed(&mut seed) as usize % bytes.len();
                    bytes.remove(index);
                }
                _ => bytes.extend_from_slice(b" "),
            }
        }
        bytes
    }

    #[test]
    fn seeded_trace_corpus_round_trips_and_rejects_or_normalizes_mutations() {
        let seeds: Vec<u64> = serde_json::from_str(include_str!(
            "../../../verification/corpus/trace-seeds.json"
        ))
        .unwrap();
        assert!(!seeds.is_empty());
        for seed in seeds {
            let trace = generated_trace(seed);
            let bytes = trace.to_json().unwrap();
            assert_eq!(Trace::from_json(&bytes).unwrap(), trace);

            let mutated = mutate_json(bytes, seed);
            if let Ok(parsed) = Trace::from_json(&mutated) {
                assert_eq!(
                    Trace::from_json(&parsed.to_json().unwrap()).unwrap(),
                    parsed
                );
            }
        }
    }

    #[test]
    fn canonical_round_trip_contains_no_host_path_or_payload() {
        let trace = trace();
        let bytes = trace.to_json().unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(!text.contains("/tmp"));
        assert!(!text.contains("payload"));
        assert_eq!(Trace::from_json(&bytes).unwrap(), trace);
    }

    #[test]
    fn event_and_byte_limits_fail_before_replay() {
        let mut trace = trace();
        for _ in trace.events.len()..MAX_EVENTS {
            trace
                .push(TraceEventKind::Checksum {
                    region: 0,
                    outcome: ChecksumOutcome::CurrentMatch,
                })
                .unwrap();
        }
        assert!(matches!(
            trace.push(TraceEventKind::Outcome {
                outcome: TraceOutcome::Success,
            }),
            Err(TraceError::Limit(_))
        ));
        let oversized = vec![b' '; MAX_TRACE_BYTES + 1];
        assert!(matches!(
            Trace::from_json(&oversized),
            Err(TraceError::Limit(_))
        ));
    }

    #[test]
    fn malformed_and_future_versions_are_refused() {
        let malformed = br#"{"schema":2,"fixture":{"size":16384,"seed":7,"write_length":4096},"events":[],"extra":true}"#;
        assert!(matches!(
            Trace::from_json(malformed),
            Err(TraceError::Json(_))
        ));
        let future =
            br#"{"schema":9,"fixture":{"size":16384,"seed":7,"write_length":4096},"events":[]}"#;
        assert_eq!(
            Trace::from_json(future),
            Err(TraceError::UnsupportedVersion(9))
        );
    }

    #[test]
    fn schema_one_renames_recovery_events_and_reexports_current_schema() {
        let prior = br#"{
            "schema": 1,
            "fixture": {"size": 16, "seed": 7, "write_length": 4},
            "events": [
                {"sequence": 0, "kind": {"Open": {"size": 16}}},
                {"sequence": 1, "kind": {"RecoveryIntent": {"generation": 1}}},
                {"sequence": 2, "kind": {"Checkpoint": {"generation": 1, "durable": true}}},
                {"sequence": 3, "kind": {"Outcome": {"outcome": "Success"}}}
            ]
        }"#;
        let migrated = Trace::from_json(prior).unwrap();
        assert!(matches!(
            &migrated.events[1].kind,
            TraceEventKind::WriteRecoveryRecord { generation: 1 }
        ));
        assert!(matches!(
            &migrated.events[2].kind,
            TraceEventKind::RecoveryClean {
                generation: 1,
                durable: true
            }
        ));
        let output = String::from_utf8(migrated.to_json().unwrap()).unwrap();
        assert!(output.contains("WriteRecoveryRecord"));
        assert!(output.contains("RecoveryClean"));
        assert!(!output.contains("RecoveryIntent"));
        assert!(!output.contains("Checkpoint"));
    }

    #[test]
    fn schema_zero_migrates_and_reexports_current_schema() {
        let current = trace();
        let mut legacy = serde_json::to_value(&current).unwrap();
        let object = legacy.as_object_mut().unwrap();
        object.insert("schema".to_owned(), serde_json::json!(0));
        object.insert("size".to_owned(), serde_json::json!(16 * 1024));
        object.insert("seed".to_owned(), serde_json::json!(7));
        object.insert("write_length".to_owned(), serde_json::json!(4096));
        object.remove("fixture");
        let migrated = Trace::from_json(&serde_json::to_vec(&legacy).unwrap()).unwrap();
        assert_eq!(migrated, current);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&migrated.to_json().unwrap()).unwrap()["schema"],
            2
        );
    }

    #[test]
    fn minimization_keeps_a_valid_failing_prefix() {
        let mut trace = trace();
        trace.events.insert(
            trace.events.len() - 1,
            TraceEvent {
                sequence: 0,
                kind: TraceEventKind::Read {
                    store: 0,
                    offset: 0,
                    length: 4096,
                },
            },
        );
        for (index, event) in trace.events.iter_mut().enumerate() {
            event.sequence = index as u32;
        }
        let minimized = trace.minimize_prefix(2).unwrap();
        assert_eq!(minimized.events.len(), 4);
        assert_eq!(
            minimized.summary().terminal,
            Some(TraceOutcome::ReplayMismatch)
        );
    }
}
