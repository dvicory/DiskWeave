//! Bounded Linux ublk acceptance adapter behind DiskWeave's portable service.

mod fixture;
mod trace;

#[cfg(target_os = "linux")]
mod linux;

use dwv_core::{
    BlockOp, BlockRequest, BufferToken, ByteRange, DurabilityIntent, FenceDomain,
    FrontendCapabilities, FrontendId, OrderingIntent, RequestId, SlotId, SubmissionSequence,
    TopologyEpoch,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

pub use fixture::{Fixture, FixtureInspection, FixtureManifest, OpenFixture};
pub use trace::{
    KernelCompletion, MAX_TRACE_BYTES, NormalizedTraceRequest, TRACE_SCHEMA, TraceBounds,
    TraceDocument, TraceDurability, TraceLog, TraceRecord, TraceReplaySummary, TraceReservation,
    map_kernel_completion, replay_trace_file,
};

pub const LOGICAL_BLOCK_SIZE: u32 = 512;
pub const PHYSICAL_BLOCK_SIZE: u32 = 4096;
pub const MAX_TRANSFER: u64 = 128 * 1024;
pub const QUEUE_DEPTH: usize = 8;
pub const MAX_TRACE_RECORDS: usize = 4096;
pub const DATA_SLOT: SlotId = SlotId([1; 16]);
pub const FRONTEND_ID: FrontendId = FrontendId(1);
#[cfg(target_os = "linux")]
const SEMANTICALLY_NEUTRAL_FLAGS: u32 = libublk::sys::UBLK_IO_F_FAILFAST_DEV
    | libublk::sys::UBLK_IO_F_FAILFAST_TRANSPORT
    | libublk::sys::UBLK_IO_F_FAILFAST_DRIVER
    | libublk::sys::UBLK_IO_F_META;
#[cfg(not(target_os = "linux"))]
const UBLK_IO_F_FAILFAST_DEV: u32 = 1 << 8;
#[cfg(not(target_os = "linux"))]
const UBLK_IO_F_FAILFAST_TRANSPORT: u32 = 1 << 9;
#[cfg(not(target_os = "linux"))]
const UBLK_IO_F_FAILFAST_DRIVER: u32 = 1 << 10;
#[cfg(not(target_os = "linux"))]
const UBLK_IO_F_META: u32 = 1 << 11;
#[cfg(not(target_os = "linux"))]
const SEMANTICALLY_NEUTRAL_FLAGS: u32 = UBLK_IO_F_FAILFAST_DEV
    | UBLK_IO_F_FAILFAST_TRANSPORT
    | UBLK_IO_F_FAILFAST_DRIVER
    | UBLK_IO_F_META;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KernelOperation {
    Read,
    Write,
    Flush,
    WriteZeroes,
    Discard,
    Zoned,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelRequest {
    pub operation: KernelOperation,
    pub flags: u32,
    pub start_sector: u64,
    pub sectors: u32,
    pub tag: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranslatedRequest {
    pub normalized: BlockRequest,
    pub data_length: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalResult {
    Success,
    Invalid,
    Unsupported,
    ResourceExhausted,
    Io,
    ReconciliationRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdapterError {
    Invalid(&'static str),
    Unsupported(&'static str),
    Exhausted(&'static str),
    Conflict(String),
    Io(String),
    ReconciliationRequired(String),
}

impl AdapterError {
    pub const fn terminal(&self) -> TerminalResult {
        match self {
            Self::Invalid(_) => TerminalResult::Invalid,
            Self::Unsupported(_) => TerminalResult::Unsupported,
            Self::Exhausted(_) => TerminalResult::ResourceExhausted,
            Self::Conflict(_) | Self::Io(_) => TerminalResult::Io,
            Self::ReconciliationRequired(_) => TerminalResult::ReconciliationRequired,
        }
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid request: {message}"),
            Self::Unsupported(message) => write!(formatter, "unsupported: {message}"),
            Self::Exhausted(message) => write!(formatter, "resource exhausted: {message}"),
            Self::Conflict(message) => write!(formatter, "conflict: {message}"),
            Self::Io(message) => write!(formatter, "I/O failure: {message}"),
            Self::ReconciliationRequired(message) => {
                write!(formatter, "reconciliation required: {message}")
            }
        }
    }
}

impl std::error::Error for AdapterError {}

pub fn translate_request(
    raw: KernelRequest,
    capacity: u64,
    epoch: TopologyEpoch,
    sequence: SubmissionSequence,
    generation: u32,
) -> Result<TranslatedRequest, AdapterError> {
    if raw.flags & !SEMANTICALLY_NEUTRAL_FLAGS != 0 {
        return Err(AdapterError::Unsupported("request flags"));
    }
    let (op, durability) = match raw.operation {
        KernelOperation::Read => (BlockOp::Read, DurabilityIntent::Ordinary),
        KernelOperation::Write => (BlockOp::Write, DurabilityIntent::Ordinary),
        KernelOperation::Flush => (BlockOp::Flush, DurabilityIntent::ExplicitFlush),
        KernelOperation::WriteZeroes => return Err(AdapterError::Unsupported("write-zeroes")),
        KernelOperation::Discard => return Err(AdapterError::Unsupported("discard")),
        KernelOperation::Zoned => return Err(AdapterError::Unsupported("zoned operation")),
        KernelOperation::Unknown => return Err(AdapterError::Unsupported("unknown operation")),
    };
    let range = if op == BlockOp::Flush {
        if raw.sectors != 0 {
            return Err(AdapterError::Invalid("flush carries a length"));
        }
        ByteRange::empty()
    } else {
        let offset = raw
            .start_sector
            .checked_mul(u64::from(LOGICAL_BLOCK_SIZE))
            .ok_or(AdapterError::Invalid("sector offset overflows"))?;
        let length = u64::from(raw.sectors)
            .checked_mul(u64::from(LOGICAL_BLOCK_SIZE))
            .ok_or(AdapterError::Invalid("sector count overflows"))?;
        if length == 0 {
            return Err(AdapterError::Invalid("empty data request"));
        }
        let range = ByteRange::new(offset, length)
            .map_err(|_| AdapterError::Invalid("byte range overflows"))?;
        if range.end() > capacity {
            return Err(AdapterError::Invalid("request exceeds published capacity"));
        }
        range
    };
    let buffer = matches!(op, BlockOp::Read | BlockOp::Write)
        .then_some(BufferToken::new(u32::from(raw.tag), generation));
    let request = BlockRequest::new(
        RequestId(sequence.0),
        FRONTEND_ID,
        DATA_SLOT,
        epoch,
        op,
        range,
        buffer,
        OrderingIntent {
            submission_sequence: sequence,
            preflush: false,
            fence_domain: FenceDomain(1),
        },
        durability,
    );
    request
        .validate(&FrontendCapabilities {
            max_transfer: Some(MAX_TRANSFER),
            supports_preflush: false,
            supports_fua: false,
            supports_write_zeroes: false,
            supports_discard: false,
        })
        .map_err(|_| AdapterError::Invalid("normalized request validation failed"))?;
    Ok(TranslatedRequest {
        normalized: request,
        data_length: usize::try_from(range.length)
            .map_err(|_| AdapterError::Invalid("transfer does not fit memory size"))?,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TagToken {
    pub tag: u16,
    pub generation: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TagState {
    generation: u32,
    active: bool,
}

pub struct TagTable {
    tags: [TagState; QUEUE_DEPTH],
}

impl Default for TagTable {
    fn default() -> Self {
        Self {
            tags: [TagState {
                generation: 0,
                active: false,
            }; QUEUE_DEPTH],
        }
    }
}

impl TagTable {
    pub fn reserve(&mut self, tag: u16) -> Result<TagToken, AdapterError> {
        let state = self
            .tags
            .get_mut(usize::from(tag))
            .ok_or(AdapterError::Invalid("tag exceeds queue depth"))?;
        if state.active {
            return Err(AdapterError::Exhausted("tag is already active"));
        }
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or(AdapterError::Exhausted("tag generation exhausted"))?;
        state.active = true;
        Ok(TagToken {
            tag,
            generation: state.generation,
        })
    }

    pub fn complete(&mut self, token: TagToken) -> Result<(), AdapterError> {
        let state = self
            .tags
            .get_mut(usize::from(token.tag))
            .ok_or(AdapterError::Invalid("tag exceeds queue depth"))?;
        if !state.active || state.generation != token.generation {
            return Err(AdapterError::Conflict(
                "stale or duplicate tag completion".into(),
            ));
        }
        state.active = false;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.tags.iter().all(|state| !state.active)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleState {
    Stopped,
    Assembling,
    Published,
    AdmissionClosed,
    Draining,
    ReconciliationRequired,
}

pub struct Lifecycle {
    state: LifecycleState,
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self {
            state: LifecycleState::Stopped,
        }
    }
}

impl Lifecycle {
    pub const fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn transition(&mut self, next: LifecycleState) -> Result<(), AdapterError> {
        let valid = matches!(
            (self.state, next),
            (LifecycleState::Stopped, LifecycleState::Assembling)
                | (LifecycleState::Assembling, LifecycleState::Published)
                | (LifecycleState::Assembling, LifecycleState::Stopped)
                | (LifecycleState::Published, LifecycleState::AdmissionClosed)
                | (LifecycleState::AdmissionClosed, LifecycleState::Draining)
                | (LifecycleState::Draining, LifecycleState::Stopped)
                | (_, LifecycleState::ReconciliationRequired)
        );
        if !valid {
            return Err(AdapterError::Conflict(format!(
                "invalid lifecycle transition {:?} -> {:?}",
                self.state, next
            )));
        }
        self.state = next;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProbeReport {
    pub platform: String,
    pub architecture: String,
    pub kernel_release: Option<String>,
    pub control: ProbeDisposition,
    pub ext4_tools: ProbeDisposition,
    pub mount_tools: ProbeDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason", rename_all = "kebab-case")]
pub enum ProbeDisposition {
    Present,
    Unsupported(String),
    Blocked(String),
}
pub fn classify_control_access(
    access: std::io::Result<()>,
    module_installed: bool,
) -> ProbeDisposition {
    match access {
        Ok(()) => ProbeDisposition::Present,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            ProbeDisposition::Blocked(format!("/dev/ublk-control: {error}"))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let reason = if module_installed {
                "ublk module is installed but /dev/ublk-control is absent"
            } else {
                "ublk control and an installed module were not found"
            };
            ProbeDisposition::Unsupported(reason.into())
        }
        Err(error) => ProbeDisposition::Blocked(format!("/dev/ublk-control: {error}")),
    }
}

pub fn validate_shutdown_evidence(
    drained: bool,
    checkpointed: bool,
    endpoint_removed: bool,
) -> Result<(), AdapterError> {
    if drained && checkpointed && endpoint_removed {
        Ok(())
    } else {
        Err(AdapterError::ReconciliationRequired(
            "shutdown lacks drain, checkpoint, or endpoint-removal evidence".into(),
        ))
    }
}
#[cfg(any(target_os = "linux", test))]
pub(crate) const fn completion_was_delivered(
    submit_succeeded: bool,
    queue_stopped: bool,
    shutdown_requested: bool,
    queue_failed: bool,
) -> bool {
    submit_succeeded || (queue_stopped && shutdown_requested && !queue_failed)
}

pub fn probe() -> ProbeReport {
    #[cfg(target_os = "linux")]
    {
        linux::probe()
    }
    #[cfg(not(target_os = "linux"))]
    {
        ProbeReport {
            platform: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
            kernel_release: None,
            control: ProbeDisposition::Unsupported("ublk requires Linux".into()),
            ext4_tools: ProbeDisposition::Unsupported("ext4 acceptance requires Linux".into()),
            mount_tools: ProbeDisposition::Unsupported("mount acceptance requires Linux".into()),
        }
    }
}

pub fn initialize(root: &Path, size: u64) -> Result<FixtureManifest, AdapterError> {
    Fixture::initialize(root, size)
}

pub fn inspect(root: &Path) -> Result<FixtureInspection, AdapterError> {
    Fixture::load(root)?.inspect()
}

#[cfg(target_os = "linux")]
pub fn serve(root: &Path, device_id: i32) -> Result<serde_json::Value, AdapterError> {
    linux::serve(root, device_id)
}

#[cfg(not(target_os = "linux"))]
pub fn serve(_root: &Path, _device_id: i32) -> Result<serde_json::Value, AdapterError> {
    Err(AdapterError::Unsupported("ublk serving requires Linux"))
}

#[cfg(target_os = "linux")]
pub fn cleanup(root: &Path, device_id: u32) -> Result<serde_json::Value, AdapterError> {
    linux::cleanup(root, device_id)
}

#[cfg(not(target_os = "linux"))]
pub fn cleanup(_root: &Path, _device_id: u32) -> Result<serde_json::Value, AdapterError> {
    Err(AdapterError::Unsupported("ublk cleanup requires Linux"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(operation: KernelOperation) -> KernelRequest {
        KernelRequest {
            operation,
            flags: 0,
            start_sector: 1,
            sectors: 8,
            tag: 0,
        }
    }

    #[test]
    fn translation_is_checked_and_narrow() {
        let translated = translate_request(
            raw(KernelOperation::Write),
            8192,
            TopologyEpoch(3),
            SubmissionSequence(9),
            1,
        )
        .unwrap();
        assert_eq!(
            translated.normalized.range,
            ByteRange::new(512, 4096).unwrap()
        );
        assert_eq!(translated.data_length, 4096);
        assert!(
            translate_request(
                KernelRequest {
                    flags: SEMANTICALLY_NEUTRAL_FLAGS,
                    ..raw(KernelOperation::Read)
                },
                8192,
                TopologyEpoch(3),
                SubmissionSequence(10),
                1,
            )
            .is_ok()
        );
        assert!(matches!(
            translate_request(
                KernelRequest {
                    flags: !SEMANTICALLY_NEUTRAL_FLAGS,
                    ..raw(KernelOperation::Write)
                },
                8192,
                TopologyEpoch(3),
                SubmissionSequence(10),
                1,
            ),
            Err(AdapterError::Unsupported(_))
        ));
        assert!(
            translate_request(
                KernelRequest {
                    operation: KernelOperation::Flush,
                    flags: 0,
                    start_sector: u64::MAX,
                    sectors: 0,
                    tag: 0,
                },
                8192,
                TopologyEpoch(3),
                SubmissionSequence(10),
                1,
            )
            .is_ok()
        );
        assert!(matches!(
            translate_request(
                KernelRequest {
                    start_sector: u64::MAX,
                    ..raw(KernelOperation::Read)
                },
                8192,
                TopologyEpoch(3),
                SubmissionSequence(10),
                1
            ),
            Err(AdapterError::Invalid(_))
        ));
        assert!(matches!(
            translate_request(
                raw(KernelOperation::Discard),
                8192,
                TopologyEpoch(3),
                SubmissionSequence(11),
                1
            ),
            Err(AdapterError::Unsupported(_))
        ));
    }

    #[test]
    fn tags_reject_stale_and_duplicate_completion() {
        let mut tags = TagTable::default();
        let first = tags.reserve(0).unwrap();
        assert!(matches!(tags.reserve(0), Err(AdapterError::Exhausted(_))));
        tags.complete(first).unwrap();
        let second = tags.reserve(0).unwrap();
        assert_ne!(first.generation, second.generation);
        assert!(tags.complete(first).is_err());
        tags.complete(second).unwrap();
        assert!(tags.complete(second).is_err());
        assert!(tags.is_empty());
    }

    #[test]
    fn lifecycle_refuses_unsafe_shortcuts() {
        let mut lifecycle = Lifecycle::default();
        assert!(lifecycle.transition(LifecycleState::Published).is_err());
        lifecycle.transition(LifecycleState::Assembling).unwrap();
        lifecycle.transition(LifecycleState::Published).unwrap();
        lifecycle
            .transition(LifecycleState::AdmissionClosed)
            .unwrap();
        lifecycle.transition(LifecycleState::Draining).unwrap();
        lifecycle.transition(LifecycleState::Stopped).unwrap();
    }

    #[test]
    fn trace_is_bounded_replayable_and_detects_divergence() {
        let kernel = raw(KernelOperation::Write);
        let translated =
            translate_request(kernel, 8192, TopologyEpoch(3), SubmissionSequence(1), 1).unwrap();
        let normalized = NormalizedTraceRequest::from_translated(translated).unwrap();
        let mut log = TraceLog::default();
        let reservation = log.reserve().unwrap();
        log.complete(
            reservation,
            TraceRecord {
                sequence: 1,
                tag: 0,
                generation: 1,
                kernel_submission: kernel,
                normalized_request: Some(normalized),
                semantic_result: TerminalResult::Success,
                kernel_completion: KernelCompletion::Success { bytes: 4096 },
            },
        )
        .unwrap();
        let document = log.document("fixture".into(), 8192, 3).unwrap();
        let bytes = document.to_json().unwrap();
        let replay = TraceDocument::from_json(&bytes).unwrap().replay().unwrap();
        assert!(replay.clean);
        assert_eq!(replay.record_count, 1);

        let mut divergent = document.clone();
        divergent.records[0]
            .normalized_request
            .as_mut()
            .unwrap()
            .offset += 512;
        assert!(matches!(divergent.replay(), Err(AdapterError::Conflict(_))));
        let mut mismatched_tag = document;
        mismatched_tag.records[0].tag = 1;
        assert!(matches!(
            mismatched_tag.replay(),
            Err(AdapterError::Invalid(_))
        ));
        assert!(matches!(
            TraceDocument::from_json(b"{"),
            Err(AdapterError::Io(_))
        ));
        assert!(matches!(
            TraceDocument::from_json(&vec![b' '; MAX_TRACE_BYTES + 1]),
            Err(AdapterError::Exhausted(_))
        ));
    }

    #[test]
    fn trace_reservation_precedes_work_and_abandonment_is_not_clean() {
        let mut log = TraceLog::default();
        for _ in 0..MAX_TRACE_RECORDS {
            log.reserve().unwrap();
        }
        assert!(matches!(log.reserve(), Err(AdapterError::Exhausted(_))));
        assert!(matches!(
            log.document("fixture".into(), 8192, 3),
            Err(AdapterError::ReconciliationRequired(_))
        ));

        let kernel = raw(KernelOperation::Read);
        let translated =
            translate_request(kernel, 8192, TopologyEpoch(3), SubmissionSequence(1), 1).unwrap();
        let mut abandoned = TraceLog::default();
        let reservation = abandoned.reserve().unwrap();
        abandoned
            .complete(
                reservation,
                TraceRecord {
                    sequence: 1,
                    tag: 0,
                    generation: 1,
                    kernel_submission: kernel,
                    normalized_request: Some(
                        NormalizedTraceRequest::from_translated(translated).unwrap(),
                    ),
                    semantic_result: TerminalResult::Success,
                    kernel_completion: KernelCompletion::Abandoned,
                },
            )
            .unwrap();
        assert!(
            !abandoned
                .document("fixture".into(), 8192, 3)
                .unwrap()
                .replay()
                .unwrap()
                .clean
        );
    }

    #[test]
    fn completion_probe_and_shutdown_fail_closed() {
        assert!(matches!(
            map_kernel_completion(TerminalResult::Success, Some(i32::MAX as u64 + 1)),
            Err(AdapterError::Invalid(_))
        ));
        assert!(matches!(
            classify_control_access(
                Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
                false
            ),
            ProbeDisposition::Unsupported(_)
        ));
        assert!(matches!(
            classify_control_access(
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
                true
            ),
            ProbeDisposition::Blocked(_)
        ));
        assert!(validate_shutdown_evidence(true, true, true).is_ok());
        for evidence in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            assert!(matches!(
                validate_shutdown_evidence(evidence.0, evidence.1, evidence.2),
                Err(AdapterError::ReconciliationRequired(_))
            ));
        }
        assert!(completion_was_delivered(true, false, false, false));
        assert!(completion_was_delivered(false, true, true, false));
        assert!(!completion_was_delivered(false, true, false, false));
        assert!(!completion_was_delivered(false, true, true, true));
        assert!(!completion_was_delivered(false, false, true, false));
    }
}
