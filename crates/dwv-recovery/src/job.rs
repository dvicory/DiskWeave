//! Deterministic bounded checksum jobs and generation-checked result commits.

use crate::{
    ChecksumExtent, ChecksumPersistenceEvidence, ChecksumProfileId, ChecksumRecord, ChecksumSet,
    ChecksumSetGeneration, ChecksumSetState, ChecksumState, ContentGeneration, Digest,
    DigestProvider, GenerationCapture, InvalidationTarget, RecoveryError, RecoveryGeneration,
    RecoveryStateStore, WriteRecoveryRecordCommit, WriteRecoveryRecordEvidence,
};
use dwv_core::TopologyEpoch;
use std::collections::VecDeque;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChecksumJobKey {
    pub extent: crate::IntegrityExtentId,
    pub profile: ChecksumProfileId,
    pub set_generation: ChecksumSetGeneration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadEvidence {
    pub persistence_evidence: ChecksumPersistenceEvidence,
    pub complete: bool,
}

impl ReadEvidence {
    pub const fn new(persistence_evidence: ChecksumPersistenceEvidence) -> Self {
        Self {
            persistence_evidence,
            complete: true,
        }
    }

    pub const fn volatile(persistence_evidence: ChecksumPersistenceEvidence) -> Self {
        Self {
            persistence_evidence,
            complete: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChecksumJob {
    pub key: ChecksumJobKey,
    pub extent: ChecksumExtent,
    pub capture: GenerationCapture,
    pub content_generation: ContentGeneration,
    pub bytes: Vec<u8>,
    pub evidence: ReadEvidence,
}

impl ChecksumJob {
    pub fn new(
        record: &ChecksumRecord,
        capture: GenerationCapture,
        bytes: Vec<u8>,
        evidence: ReadEvidence,
    ) -> Self {
        Self {
            key: ChecksumJobKey {
                extent: record.extent.id,
                profile: record.profile,
                set_generation: record.set_generation,
            },
            extent: record.extent,
            capture,
            content_generation: record.content_generation,
            bytes,
            evidence,
        }
    }

    pub fn full_overwrite(
        record: &ChecksumRecord,
        capture: GenerationCapture,
        final_bytes: Vec<u8>,
        evidence: ReadEvidence,
    ) -> Option<Self> {
        if evidence.complete
            && final_bytes.len() as u64 == record.extent.range.length
            && record.content_generation.0 <= capture.recovery_generation
        {
            Some(Self::new(record, capture, final_bytes, evidence))
        } else {
            None
        }
    }

    pub fn evaluate<P: DigestProvider>(self, provider: &P) -> Result<ChecksumJobResult, JobError> {
        if provider.profile().id != self.key.profile {
            return Err(JobError::ProviderProfileMismatch {
                expected: self.key.profile,
                actual: provider.profile().id,
            });
        }
        if !self.evidence.complete {
            return Err(JobError::MissingPersistenceEvidence);
        }
        if self.bytes.len() as u64 != self.extent.range.length {
            return Err(JobError::ReadLengthMismatch {
                expected: self.extent.range.length,
                actual: self.bytes.len() as u64,
            });
        }
        if self.evidence.persistence_evidence.recovery_generation != self.content_generation.0 {
            return Err(JobError::PersistenceEvidenceGenerationMismatch);
        }
        let digest = provider.digest(&self.bytes).map_err(JobError::Provider)?;
        Ok(ChecksumJobResult {
            key: self.key,
            extent: self.extent,
            content_generation: self.content_generation,
            capture: self.capture,
            digest,
            evidence: self.evidence,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChecksumJobResult {
    pub key: ChecksumJobKey,
    pub extent: ChecksumExtent,
    pub content_generation: ContentGeneration,
    pub capture: GenerationCapture,
    pub digest: Digest,
    pub evidence: ReadEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommitOutcome {
    Committed(ChecksumRecord),
    Noop,
    Rejected(RejectReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectReason {
    MissingPersistenceEvidence,
    TopologyChanged,
    SetChanged,
    GenerationChanged,
    RecordNotCurrent,
}

#[derive(Debug)]
pub enum JobError {
    QueueFull,
    GenerationExhausted,
    ProviderProfileMismatch {
        expected: ChecksumProfileId,
        actual: ChecksumProfileId,
    },
    MissingPersistenceEvidence,
    ReadLengthMismatch {
        expected: u64,
        actual: u64,
    },
    PersistenceEvidenceGenerationMismatch,
    Provider(crate::ProviderError),
}

impl fmt::Display for JobError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull => formatter.write_str("checksum job queue is full"),
            Self::GenerationExhausted => formatter.write_str("checksum generation exhausted"),
            Self::ProviderProfileMismatch { expected, actual } => {
                write!(
                    formatter,
                    "provider profile {:?} does not match {:?}",
                    actual, expected
                )
            }
            Self::MissingPersistenceEvidence => {
                formatter.write_str("checksum job lacks durable persistence evidence")
            }
            Self::ReadLengthMismatch { expected, actual } => write!(
                formatter,
                "checksum read length {actual} does not match extent length {expected}"
            ),
            Self::PersistenceEvidenceGenerationMismatch => formatter.write_str(
                "checksum persistence evidence does not cover the captured content generation",
            ),
            Self::Provider(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for JobError {}

impl From<crate::ProviderError> for JobError {
    fn from(value: crate::ProviderError) -> Self {
        Self::Provider(value)
    }
}

/// A bounded FIFO queue. Scheduling is intentionally synchronous at this seam;
/// an executor can own the queue without changing validity semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChecksumQueue {
    capacity: usize,
    pending: VecDeque<ChecksumJob>,
}

impl ChecksumQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            pending: VecDeque::new(),
        }
    }

    pub fn submit(&mut self, job: ChecksumJob) -> Result<(), JobError> {
        if self.pending.len() >= self.capacity {
            return Err(JobError::QueueFull);
        }
        self.pending.push_back(job);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<ChecksumJob> {
        self.pending.pop_front()
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChecksumAuthority {
    pub topology_epoch: TopologyEpoch,
    pub recovery_generation: RecoveryGeneration,
    pub active_set: ChecksumSetGeneration,
    records: Vec<ChecksumRecord>,
}

impl ChecksumAuthority {
    pub fn new(topology_epoch: TopologyEpoch, active_set: ChecksumSetGeneration) -> Self {
        Self {
            topology_epoch,
            recovery_generation: RecoveryGeneration::ZERO,
            active_set,
            records: Vec::new(),
        }
    }
    /// Return the single checksum profile represented by this authority.
    ///
    /// Mixed profiles are not a usable capture binding.
    pub fn profile_id(&self) -> Option<ChecksumProfileId> {
        let profile = self.records.first()?.profile;
        self.records
            .iter()
            .all(|record| record.profile == profile)
            .then_some(profile)
    }

    pub fn records(&self) -> &[ChecksumRecord] {
        &self.records
    }

    pub fn register(&mut self, record: ChecksumRecord) {
        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|item| item.extent.id == record.extent.id)
        {
            *existing = record;
        } else {
            self.records.push(record);
        }
    }

    pub fn record(&self, extent: crate::IntegrityExtentId) -> Option<&ChecksumRecord> {
        self.records
            .iter()
            .find(|record| record.extent.id == extent)
    }

    pub fn invalidate(
        &mut self,
        extent: crate::IntegrityExtentId,
    ) -> Result<RecoveryGeneration, JobError> {
        self.recovery_generation = self
            .recovery_generation
            .checked_next()
            .ok_or(JobError::GenerationExhausted)?;
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.extent.id == extent)
        {
            *record = record.stale_from(ContentGeneration(self.recovery_generation));
        }
        Ok(self.recovery_generation)
    }

    /// Persist OS-010 invalidation before advancing the checksum authority.
    pub fn invalidate_with_write_recovery_record<S: RecoveryStateStore + ?Sized>(
        &mut self,
        store: &mut S,
        target: InvalidationTarget,
    ) -> Result<WriteRecoveryRecordEvidence, RecoveryError> {
        self.invalidate_with_write_recovery_record_and_coded_transitions(store, target, [])
            .map(|result| result.evidence)
    }

    pub fn invalidate_with_write_recovery_record_and_coded_transitions<
        S: RecoveryStateStore + ?Sized,
    >(
        &mut self,
        store: &mut S,
        target: InvalidationTarget,
        transitions: impl IntoIterator<Item = crate::CodedCaptureTransition>,
    ) -> Result<crate::WriteRecoveryRecordCommitResult, RecoveryError> {
        let result = WriteRecoveryRecordCommit::new(
            store,
            self.topology_epoch,
            self.recovery_generation,
            target.clone(),
        )
        .with_coded_transitions(transitions)
        .commit_with_receipt()?;
        self.recovery_generation = result.evidence.committed_generation;
        for extent in target.checksum_extents {
            if let Some(record) = self
                .records
                .iter_mut()
                .find(|record| record.extent.id == extent)
            {
                *record = record.stale_from(ContentGeneration(self.recovery_generation));
            }
        }
        Ok(result)
    }

    pub fn capture_job(
        &self,
        extent: crate::IntegrityExtentId,
        bytes: Vec<u8>,
        evidence: ReadEvidence,
    ) -> Option<ChecksumJob> {
        let record = self.record(extent)?;
        Some(ChecksumJob::new(
            record,
            GenerationCapture::new(self.topology_epoch, self.recovery_generation),
            bytes,
            evidence,
        ))
    }

    pub fn commit(&mut self, result: ChecksumJobResult) -> CommitOutcome {
        if !result.evidence.complete {
            return CommitOutcome::Rejected(RejectReason::MissingPersistenceEvidence);
        }
        if result.capture.topology_epoch != self.topology_epoch
            || result.evidence.persistence_evidence.topology_epoch != self.topology_epoch
        {
            return CommitOutcome::Rejected(RejectReason::TopologyChanged);
        }
        if result.key.set_generation != self.active_set {
            return CommitOutcome::Rejected(RejectReason::SetChanged);
        }
        if result.capture.recovery_generation != self.recovery_generation {
            return CommitOutcome::Rejected(RejectReason::GenerationChanged);
        }
        if result.evidence.persistence_evidence.recovery_generation != result.content_generation.0 {
            return CommitOutcome::Rejected(RejectReason::MissingPersistenceEvidence);
        }
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.extent.id == result.key.extent)
        else {
            return CommitOutcome::Rejected(RejectReason::RecordNotCurrent);
        };
        if record.profile != result.key.profile
            || record.set_generation != result.key.set_generation
            || record.content_generation != result.content_generation
            || record.extent != result.extent
        {
            return CommitOutcome::Rejected(RejectReason::RecordNotCurrent);
        }
        if record.state == ChecksumState::Valid && record.digest == Some(result.digest) {
            return CommitOutcome::Noop;
        }
        let next = ChecksumRecord::valid(
            result.extent,
            result.key.profile,
            result.key.set_generation,
            result.content_generation,
            result.digest,
            result.evidence.persistence_evidence,
        );
        *record = next.clone();
        CommitOutcome::Committed(next)
    }

    pub fn install_active_set(
        &mut self,
        set: ChecksumSet,
        records: Vec<ChecksumRecord>,
    ) -> Result<(), crate::MigrationError> {
        if set.state != ChecksumSetState::Active {
            return Err(crate::MigrationError::ActiveSetNotReady);
        }
        if records.iter().any(|record| {
            !record.is_valid()
                || record.profile != set.profile.id
                || record.set_generation != set.generation
        }) {
            return Err(crate::MigrationError::RecordNotForActiveSet);
        }
        self.active_set = set.generation;
        self.records = records;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BLAKE3_256_PROFILE, Blake3Provider, ChecksumPersistenceEvidence, ChecksumTarget,
        InvalidationTarget, MemoryRecoveryStore,
    };
    use dwv_core::{ByteRange, SlotId, TopologyEpoch};
    use dwv_store::{CapabilityEvidenceId, FenceId, StoreFenceRef, StoreId, StoreWriteWatermark};

    fn record() -> ChecksumRecord {
        let extent = ChecksumExtent::new(
            crate::IntegrityExtentId(7),
            ChecksumTarget::data(SlotId([1; 16])),
            ByteRange::new(0, 4).unwrap(),
            4,
        )
        .unwrap();
        ChecksumRecord::absent(
            extent,
            BLAKE3_256_PROFILE.id,
            ChecksumSetGeneration::INITIAL,
        )
    }

    fn evidence(generation: RecoveryGeneration) -> ReadEvidence {
        ReadEvidence::new(ChecksumPersistenceEvidence {
            fence: StoreFenceRef {
                fence_id: FenceId(1),
                store_id: StoreId(1),
                store_incarnation: dwv_store::StoreIncarnationId(0),
                topology_epoch: TopologyEpoch(2),
                through: StoreWriteWatermark(1),
                capability_evidence_id: CapabilityEvidenceId(1),
            },
            topology_epoch: TopologyEpoch(2),
            recovery_generation: generation,
        })
    }

    #[test]
    fn queue_is_bounded_and_fifo() {
        let mut queue = ChecksumQueue::new(1);
        let mut authority =
            ChecksumAuthority::new(TopologyEpoch(2), ChecksumSetGeneration::INITIAL);
        authority.register(record());
        let job = authority
            .capture_job(
                crate::IntegrityExtentId(7),
                vec![1, 2, 3, 4],
                evidence(RecoveryGeneration::ZERO),
            )
            .unwrap();
        queue.submit(job.clone()).unwrap();
        assert!(matches!(queue.submit(job), Err(JobError::QueueFull)));
        assert_eq!(queue.pop().unwrap().key.extent, crate::IntegrityExtentId(7));
    }

    #[test]
    fn changed_generation_rejects_worker_result() {
        let mut authority =
            ChecksumAuthority::new(TopologyEpoch(2), ChecksumSetGeneration::INITIAL);
        authority.register(record());
        let job = authority
            .capture_job(
                crate::IntegrityExtentId(7),
                vec![1, 2, 3, 4],
                evidence(RecoveryGeneration::ZERO),
            )
            .unwrap();
        let result = job.evaluate(&Blake3Provider).unwrap();
        authority.invalidate(crate::IntegrityExtentId(7)).unwrap();
        assert_eq!(
            authority.commit(result),
            CommitOutcome::Rejected(RejectReason::GenerationChanged)
        );
        assert_eq!(
            authority.record(crate::IntegrityExtentId(7)).unwrap().state,
            ChecksumState::Stale
        );
    }

    #[test]
    fn matching_workers_are_idempotent() {
        let mut authority =
            ChecksumAuthority::new(TopologyEpoch(2), ChecksumSetGeneration::INITIAL);
        authority.register(record());
        let job = authority
            .capture_job(
                crate::IntegrityExtentId(7),
                vec![1, 2, 3, 4],
                evidence(RecoveryGeneration::ZERO),
            )
            .unwrap();
        let result = job.clone().evaluate(&Blake3Provider).unwrap();
        assert!(matches!(
            authority.commit(result.clone()),
            CommitOutcome::Committed(_)
        ));
        assert_eq!(authority.commit(result), CommitOutcome::Noop);
    }

    #[test]
    fn full_overwrite_matches_readback() {
        let mut authority =
            ChecksumAuthority::new(TopologyEpoch(2), ChecksumSetGeneration::INITIAL);
        authority.register(record());
        let bytes = vec![9, 8, 7, 6];
        let readback = authority
            .capture_job(
                crate::IntegrityExtentId(7),
                bytes.clone(),
                evidence(RecoveryGeneration::ZERO),
            )
            .unwrap()
            .evaluate(&Blake3Provider)
            .unwrap();
        let overwrite = ChecksumJob::full_overwrite(
            authority.record(crate::IntegrityExtentId(7)).unwrap(),
            GenerationCapture::new(TopologyEpoch(2), RecoveryGeneration::ZERO),
            bytes,
            evidence(RecoveryGeneration::ZERO),
        )
        .unwrap()
        .evaluate(&Blake3Provider)
        .unwrap();
        assert_eq!(readback.digest, overwrite.digest);
    }

    #[test]
    fn short_read_cannot_become_valid_checksum_evidence() {
        let mut authority =
            ChecksumAuthority::new(TopologyEpoch(2), ChecksumSetGeneration::INITIAL);
        authority.register(record());
        let job = authority
            .capture_job(
                crate::IntegrityExtentId(7),
                vec![1, 2, 3],
                evidence(RecoveryGeneration::ZERO),
            )
            .unwrap();
        assert!(matches!(
            job.evaluate(&Blake3Provider),
            Err(JobError::ReadLengthMismatch {
                expected: 4,
                actual: 3
            })
        ));
    }

    #[test]
    fn os010_write_recovery_record_is_durable_before_checksum_stale_state() {
        let mut store = MemoryRecoveryStore::new(TopologyEpoch(2));
        let mut authority =
            ChecksumAuthority::new(TopologyEpoch(2), ChecksumSetGeneration::INITIAL);
        authority.register(record());

        let evidence = authority
            .invalidate_with_write_recovery_record(
                &mut store,
                InvalidationTarget::new(vec![], vec![crate::IntegrityExtentId(7)]),
            )
            .unwrap();

        assert!(evidence.durable);
        assert_eq!(evidence.committed_generation, RecoveryGeneration(1));
        assert_eq!(authority.recovery_generation, RecoveryGeneration(1));
        assert_eq!(
            authority.record(crate::IntegrityExtentId(7)).unwrap().state,
            ChecksumState::Stale
        );
        assert!(matches!(
            store.snapshot().integrity_records[0].state,
            crate::IntegrityState::Stale { .. }
        ));
    }

    #[test]
    fn deterministic_generation_fuzz_never_commits_stale_worker_results() {
        for seed in 0..128_u64 {
            let mut authority =
                ChecksumAuthority::new(TopologyEpoch(2), ChecksumSetGeneration::INITIAL);
            authority.register(record());
            let bytes = vec![seed as u8, (seed >> 8) as u8, 3, 4];
            let job = authority
                .capture_job(
                    crate::IntegrityExtentId(7),
                    bytes,
                    evidence(RecoveryGeneration::ZERO),
                )
                .unwrap();
            let result = job.evaluate(&Blake3Provider).unwrap();
            authority.invalidate(crate::IntegrityExtentId(7)).unwrap();
            assert_eq!(
                authority.commit(result),
                CommitOutcome::Rejected(RejectReason::GenerationChanged)
            );
            assert_eq!(
                authority.record(crate::IntegrityExtentId(7)).unwrap().state,
                ChecksumState::Stale
            );
        }
    }
}
