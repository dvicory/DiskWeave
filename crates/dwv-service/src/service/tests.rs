use super::*;
use crate::{RebuildSource, RebuildStore};
use dwv_core::{
    ArrayId, AssignmentGeneration, AssignmentInstanceId, BufferToken, CodingPosition,
    CodingProfile, FrontendId, OrderingIntent, ProtectedGeometry, RequestId, SlotId,
    SubmissionSequence, TopologyAssignment,
};
use dwv_recovery::MemoryRecoveryStore;
use dwv_recovery::WriteRecoveryRecordCommit;
use dwv_recovery::{Blake3Provider, DigestProvider};
use dwv_recovery::{CodedCaptureId, CodedCaptureMembership, RecoveryGeneration};
use dwv_store::{
    CapabilityEvidenceId, ChildOperationId, CompletedRangeSet, FenceId, IdentityObservation,
    ResourceLimits, StoreCapabilities, StoreCompletion, StoreCompletionDelivery, StoreError,
    StoreFenceRef, StoreIncarnationId,
};
use dwv_store_file::{ControlProjection, FileStore, FileStoreConfig, FileSyncMode};
use dwv_verify::{
    ChecksumEvidence, DigestEvidence, RebuildTarget, VerificationIdentity, VerificationStore,
    VerificationStoreError, apply_repair, plan_repairs, verify_exhaustive,
};
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

const LENGTH: u64 = 4096;
const BLOCK: u32 = 512;
static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

fn member_binding<S: RandomAccessStore>(
    assignment: &TopologyAssignment,
    topology_epoch: TopologyEpoch,
    store_id: StoreId,
    store: S,
) -> MemberBinding<S> {
    MemberBinding {
        slot_id: assignment.slot_id(),
        role: assignment.role(),
        coding_position: assignment.coding_position(),
        assignment_instance: assignment.assignment_instance(),
        assignment_generation: assignment.assignment_generation(),
        topology_epoch,
        store_id,
        store,
    }
}

#[derive(Clone)]
struct LostAckRecovery {
    inner: Rc<RefCell<MemoryRecoveryStore>>,
    commits_until_lost_ack: Rc<Cell<Option<usize>>>,
    commits_until_rejection: Rc<Cell<Option<usize>>>,
    corrupt_next_commit_ack: Rc<Cell<bool>>,
    misreport_next_generation: Rc<Cell<bool>>,
}

impl LostAckRecovery {
    fn new(epoch: TopologyEpoch) -> Self {
        Self::from_store(recovery_for(&topology(epoch)))
    }

    fn from_store(store: MemoryRecoveryStore) -> Self {
        Self {
            inner: Rc::new(RefCell::new(store)),
            commits_until_lost_ack: Rc::new(Cell::new(None)),
            commits_until_rejection: Rc::new(Cell::new(None)),
            corrupt_next_commit_ack: Rc::new(Cell::new(false)),
            misreport_next_generation: Rc::new(Cell::new(false)),
        }
    }

    fn lose_next_commit(&self) {
        self.lose_commit_after(0);
    }

    fn lose_commit_after(&self, successful_commits: usize) {
        self.commits_until_lost_ack.set(Some(successful_commits));
    }
    fn reject_commit_after(&self, successful_commits: usize) {
        self.commits_until_rejection.set(Some(successful_commits));
    }
    fn corrupt_next_commit_ack(&self) {
        self.corrupt_next_commit_ack.set(true);
    }

    fn misreport_next_generation(&self) {
        self.misreport_next_generation.set(true);
    }

    fn durable_store(&self) -> MemoryRecoveryStore {
        self.inner.borrow().clone()
    }
}

impl RecoveryStateStore for LostAckRecovery {
    fn load_assembly_snapshot(
        &self,
    ) -> Result<dwv_recovery::RecoverySnapshot, dwv_recovery::RecoveryError> {
        self.inner.borrow().load_assembly_snapshot()
    }

    fn verify_integrity(&self) -> RecoveryStoreHealth {
        self.inner.borrow().verify_integrity()
    }

    fn begin_protocol_txn(
        &self,
        expected: RecoveryGeneration,
        topology_epoch: TopologyEpoch,
    ) -> RecoveryTxn {
        self.inner
            .borrow()
            .begin_protocol_txn(expected, topology_epoch)
    }

    fn commit_durable(
        &mut self,
        txn: RecoveryTxn,
    ) -> Result<RecoveryGeneration, dwv_recovery::RecoveryError> {
        match self.commits_until_rejection.get() {
            Some(0) => {
                self.commits_until_rejection.set(None);
                return Err(dwv_recovery::RecoveryError::CommitNotDurable(
                    dwv_recovery::RecoveryCommitObservation::Rejected,
                ));
            }
            Some(remaining) => {
                self.commits_until_rejection.set(Some(remaining - 1));
            }
            None => {}
        }
        let committed = self.inner.borrow_mut().commit_durable(txn)?;
        if self.misreport_next_generation.replace(false) {
            return Ok(committed
                .checked_next()
                .expect("test generation has remaining capacity"));
        }
        if self.corrupt_next_commit_ack.replace(false) {
            return Err(dwv_recovery::RecoveryError::CommitNotDurable(
                dwv_recovery::RecoveryCommitObservation::Corrupt,
            ));
        }
        match self.commits_until_lost_ack.get() {
            Some(0) => {
                self.commits_until_lost_ack.set(None);
                Err(dwv_recovery::RecoveryError::CommitNotDurable(
                    dwv_recovery::RecoveryCommitObservation::Lost,
                ))
            }
            Some(remaining) => {
                self.commits_until_lost_ack.set(Some(remaining - 1));
                Ok(committed)
            }
            None => Ok(committed),
        }
    }

    fn export_manifest(
        &self,
        generation: RecoveryGeneration,
    ) -> Result<dwv_recovery::RecoveryManifest, dwv_recovery::RecoveryError> {
        self.inner.borrow().export_manifest(generation)
    }
}

struct FileVerificationStore(FileStore);

impl VerificationStore for FileVerificationStore {
    fn identity(&self) -> VerificationIdentity {
        VerificationIdentity(
            self.0
                .capabilities_report()
                .identity
                .observations
                .first()
                .map(|observation| observation.fingerprint)
                .unwrap_or([0; 16]),
        )
    }

    fn read_exact(&mut self, range: ByteRange) -> Result<Vec<u8>, VerificationStoreError> {
        self.0
            .read_bytes(range)
            .map_err(|error| VerificationStoreError::new(error.to_string()))
    }

    fn write_exact(
        &mut self,
        range: ByteRange,
        bytes: &[u8],
    ) -> Result<(), VerificationStoreError> {
        let completion = self.0.write_bytes(
            dwv_store::ChildOperationId {
                slot: OperationSlotToken::new(0, 1),
                index: 0,
            },
            range,
            bytes,
            dwv_store::WriteIntent::Ordinary,
        );
        if matches!(completion.disposition, CompletionDisposition::Success) {
            Ok(())
        } else {
            Err(VerificationStoreError::from_completion(
                "repair write did not complete",
                completion,
            ))
        }
    }
}

#[derive(Clone, Copy)]
enum FakeRead {
    Exact,
    Short,
    NonPrefix,
    Failed,
    Uncertain,
    StaleToken,
}
#[derive(Clone, Copy, Debug)]
enum FakeEffect {
    Success,
    Failed,
    StaleToken,
    SuccessWithoutWriteWatermark,
    Uncertain,
}

struct FakeStore {
    id: StoreId,
    epoch: TopologyEpoch,
    bytes: Vec<u8>,
    read: FakeRead,
    watermark: StoreWriteWatermark,
    write: FakeEffect,
    flush: FakeEffect,
    physical_reads: usize,
    physical_writes: usize,
    physical_flushes: usize,
}

impl FakeStore {
    fn new(id: StoreId, epoch: TopologyEpoch, read: FakeRead) -> Self {
        Self::new_with_length(id, epoch, read, LENGTH)
    }

    fn new_with_length(id: StoreId, epoch: TopologyEpoch, read: FakeRead, length: u64) -> Self {
        Self {
            id,
            epoch,
            bytes: vec![0; length as usize],
            read,
            physical_reads: 0,
            watermark: StoreWriteWatermark(0),
            write: FakeEffect::Success,
            flush: FakeEffect::Success,
            physical_writes: 0,
            physical_flushes: 0,
        }
    }

    fn with_effects(mut self, write: FakeEffect, flush: FakeEffect) -> Self {
        self.write = write;
        self.flush = flush;
        self
    }

    fn completion(
        operation_id: ChildOperationId,
        requested: ByteRange,
        completed: Option<ByteRange>,
        disposition: CompletionDisposition,
        persistence: PersistenceEvidence,
    ) -> StoreCompletion {
        StoreCompletion::new(
            operation_id,
            requested,
            CompletedRangeSet::new(completed.into_iter().collect()).unwrap(),
            disposition,
            persistence,
        )
        .unwrap()
    }
}

impl RandomAccessStore for FakeStore {
    fn store_id(&self) -> StoreId {
        self.id
    }

    fn topology_epoch(&self) -> TopologyEpoch {
        self.epoch
    }

    fn incarnation(&self) -> StoreIncarnationId {
        StoreIncarnationId(1)
    }

    fn identity_observations(&self) -> IdentityObservationSet {
        IdentityObservationSet::new(
            vec![IdentityObservation {
                source: IdentitySourceKind::StableDeviceId,
                fingerprint: [self.id.0 as u8; 16],
            }],
            IdentityAssessment::Confirmed,
        )
    }

    fn current_identity_observations(&self) -> Result<IdentityObservationSet, StoreError> {
        Ok(self.identity_observations())
    }

    fn capabilities(&self) -> StoreCapabilities {
        let length = self.bytes.len() as u64;
        StoreCapabilities::portable_demo(length, BLOCK, length, CapabilityEvidenceId(self.id.0))
    }

    fn length(&self) -> u64 {
        self.bytes.len() as u64
    }

    fn highest_accepted_watermark(&self) -> Option<StoreWriteWatermark> {
        Some(self.watermark)
    }

    fn read_at(
        &mut self,
        mut operation_id: ChildOperationId,
        range: ByteRange,
        destination: &mut [u8],
    ) -> StoreCompletion {
        self.physical_reads += 1;
        let start = range.offset as usize;
        let half = range.length / 2;
        let (completed, completed_range) = match self.read {
            FakeRead::Exact | FakeRead::StaleToken => (range.length, Some(range)),
            FakeRead::Short => (half, Some(ByteRange::new(range.offset, half).unwrap())),
            FakeRead::NonPrefix => (
                half,
                Some(ByteRange::new(range.offset + half, half).unwrap()),
            ),
            FakeRead::Failed | FakeRead::Uncertain => (0, None),
        };
        if matches!(self.read, FakeRead::NonPrefix) {
            destination[half as usize..completed as usize + half as usize]
                .copy_from_slice(&self.bytes[start + half as usize..start + range.length as usize]);
        } else {
            destination[..completed as usize]
                .copy_from_slice(&self.bytes[start..start + completed as usize]);
        }
        if matches!(self.read, FakeRead::StaleToken) {
            operation_id.slot.generation = operation_id.slot.generation.wrapping_add(1);
        }
        let disposition = match self.read {
            FakeRead::Exact | FakeRead::StaleToken => CompletionDisposition::Success,
            FakeRead::Short | FakeRead::NonPrefix => CompletionDisposition::Short,
            FakeRead::Failed => {
                CompletionDisposition::Failed(StoreError::BackendFailure { code: 5 })
            }
            FakeRead::Uncertain => CompletionDisposition::Uncertain,
        };
        Self::completion(
            operation_id,
            range,
            completed_range,
            disposition,
            PersistenceEvidence::VolatileOrUnknown,
        )
    }

    fn write_at(
        &mut self,
        mut operation_id: ChildOperationId,
        range: ByteRange,
        source: &[u8],
        _intent: WriteIntent,
    ) -> StoreCompletion {
        self.physical_writes += 1;
        let start = range.offset as usize;
        if matches!(self.write, FakeEffect::StaleToken) {
            operation_id.slot.generation = operation_id.slot.generation.wrapping_add(1);
        }
        match self.write {
            FakeEffect::Success
            | FakeEffect::StaleToken
            | FakeEffect::SuccessWithoutWriteWatermark => {
                self.bytes[start..start + source.len()].copy_from_slice(source);
                self.watermark = StoreWriteWatermark(self.watermark.0 + 1);
                let completion = Self::completion(
                    operation_id,
                    range,
                    Some(range),
                    CompletionDisposition::Success,
                    PersistenceEvidence::VolatileOrUnknown,
                );
                if matches!(self.write, FakeEffect::SuccessWithoutWriteWatermark) {
                    completion
                } else {
                    completion.with_write_watermark(self.watermark)
                }
            }
            FakeEffect::Failed => Self::completion(
                operation_id,
                range,
                None,
                CompletionDisposition::Failed(StoreError::BackendFailure { code: 6 }),
                PersistenceEvidence::VolatileOrUnknown,
            ),
            FakeEffect::Uncertain => {
                self.bytes[start..start + source.len()].copy_from_slice(source);
                Self::completion(
                    operation_id,
                    range,
                    None,
                    CompletionDisposition::Uncertain,
                    PersistenceEvidence::VolatileOrUnknown,
                )
            }
        }
    }

    fn flush(
        &mut self,
        mut operation_id: ChildOperationId,
        through: StoreWriteWatermark,
    ) -> StoreCompletion {
        self.physical_flushes += 1;
        if matches!(self.flush, FakeEffect::StaleToken) {
            operation_id.slot.generation = operation_id.slot.generation.wrapping_add(1);
        }
        match self.flush {
            FakeEffect::Success
            | FakeEffect::StaleToken
            | FakeEffect::SuccessWithoutWriteWatermark => Self::completion(
                operation_id,
                ByteRange::empty(),
                None,
                CompletionDisposition::Success,
                PersistenceEvidence::DurableByFence {
                    fence: StoreFenceRef {
                        fence_id: FenceId(through.0),
                        store_id: self.id,
                        topology_epoch: self.epoch,
                        store_incarnation: self.incarnation(),
                        through,
                        capability_evidence_id: CapabilityEvidenceId(self.id.0),
                    },
                },
            ),
            FakeEffect::Failed => Self::completion(
                operation_id,
                ByteRange::empty(),
                None,
                CompletionDisposition::Failed(StoreError::BackendFailure { code: 7 }),
                PersistenceEvidence::VolatileOrUnknown,
            ),
            FakeEffect::Uncertain => Self::completion(
                operation_id,
                ByteRange::empty(),
                None,
                CompletionDisposition::Uncertain,
                PersistenceEvidence::VolatileOrUnknown,
            ),
        }
    }

    fn write_zeroes(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        _intent: WriteIntent,
    ) -> StoreCompletion {
        let start = range.offset as usize;
        self.bytes[start..start + range.length as usize].fill(0);
        self.watermark = StoreWriteWatermark(self.watermark.0 + 1);
        Self::completion(
            operation_id,
            range,
            Some(range),
            CompletionDisposition::Success,
            PersistenceEvidence::VolatileOrUnknown,
        )
        .with_write_watermark(self.watermark)
    }

    fn discard(&mut self, operation_id: ChildOperationId, range: ByteRange) -> StoreCompletion {
        Self::completion(
            operation_id,
            range,
            Some(range),
            CompletionDisposition::Success,
            PersistenceEvidence::VolatileOrUnknown,
        )
    }
}

fn topology(epoch: TopologyEpoch) -> TopologySnapshot {
    topology_with_length(epoch, LENGTH)
}

fn topology_with_length(epoch: TopologyEpoch, length: u64) -> TopologySnapshot {
    let profile = CodingProfile::new(2, 1).unwrap();
    let geometry = ProtectedGeometry::new(length, BLOCK).unwrap();
    let assignments = (0..3)
        .map(|index| {
            let role = if index < 2 {
                MemberRole::Data
            } else {
                MemberRole::Parity
            };
            TopologyAssignment::new(
                SlotId::from_bytes([index as u8 + 1; 16]),
                role,
                CodingPosition(index),
                AssignmentInstanceId::from_bytes([index as u8 + 11; 16]),
                AssignmentGeneration(1),
            )
        })
        .collect();
    TopologySnapshot::new(
        ArrayId::from_bytes([7; 16]),
        epoch,
        profile,
        geometry,
        assignments,
    )
    .unwrap()
}

fn recovery_for(topology: &TopologySnapshot) -> MemoryRecoveryStore {
    let active_topology = dwv_recovery::TopologySnapshot::from_core(
        topology.clone(),
        topology
            .assignments()
            .iter()
            .map(|assignment| StoreId(assignment.coding_position().0 as u64 + 1))
            .collect(),
    )
    .unwrap();
    let mut manifest = dwv_recovery::RecoveryManifest {
        schema: dwv_recovery::CURRENT_RECOVERY_SCHEMA,
        snapshot: MemoryRecoveryStore::new(topology.topology_epoch())
            .snapshot()
            .clone(),
    };
    manifest.snapshot.active_topology = Some(active_topology);
    MemoryRecoveryStore::from_manifest(manifest).unwrap()
}

fn request(
    request_id: RequestId,
    epoch: TopologyEpoch,
    data_slot: usize,
    op: BlockOp,
    range: ByteRange,
    durability: DurabilityIntent,
) -> BlockRequest {
    BlockRequest::new(
        request_id,
        FrontendId(7),
        SlotId::from_bytes([u8::try_from(data_slot).unwrap() + 1; 16]),
        epoch,
        op,
        range,
        matches!(op, BlockOp::Read | BlockOp::Write)
            .then_some(BufferToken::new(u32::try_from(data_slot).unwrap(), 1)),
        OrderingIntent {
            submission_sequence: SubmissionSequence(request_id.0),
            preflush: false,
            fence_domain: FenceDomain(1),
        },
        durability,
    )
}

fn fake_service(
    read: FakeRead,
    config: ServiceConfig,
) -> HealthyPortableService<FakeStore, MemoryRecoveryStore> {
    fake_service_with_effects(read, FakeEffect::Success, FakeEffect::Success, config)
}

fn fake_service_with_recovery<R: RecoveryStateStore>(
    read: FakeRead,
    recovery: R,
    config: ServiceConfig,
) -> HealthyPortableService<FakeStore, R> {
    let epoch = TopologyEpoch(4);
    let topology = topology(epoch);
    let members = topology
        .assignments()
        .iter()
        .enumerate()
        .map(|(index, assignment)| {
            let store_id = StoreId(index as u64 + 1);
            member_binding(
                assignment,
                epoch,
                store_id,
                FakeStore::new(store_id, epoch, read),
            )
        })
        .collect();
    HealthyPortableService::open(topology, members, recovery, config).unwrap()
}

fn fake_service_with_effects(
    read: FakeRead,
    write: FakeEffect,
    flush: FakeEffect,
    config: ServiceConfig,
) -> HealthyPortableService<FakeStore, MemoryRecoveryStore> {
    let epoch = TopologyEpoch(4);
    let topology = topology(epoch);
    let members = topology
        .assignments()
        .iter()
        .enumerate()
        .map(|(index, assignment)| {
            let store_id = StoreId(index as u64 + 1);
            member_binding(
                assignment,
                epoch,
                store_id,
                FakeStore::new(store_id, epoch, read).with_effects(write, flush),
            )
        })
        .collect();
    HealthyPortableService::open(topology.clone(), members, recovery_for(&topology), config)
        .unwrap()
}

fn fake_service_with_length(length: u64) -> HealthyPortableService<FakeStore, MemoryRecoveryStore> {
    let epoch = TopologyEpoch(4);
    let topology = topology_with_length(epoch, length);
    let members = topology
        .assignments()
        .iter()
        .enumerate()
        .map(|(index, assignment)| {
            let store_id = StoreId(index as u64 + 1);
            member_binding(
                assignment,
                epoch,
                store_id,
                FakeStore::new_with_length(store_id, epoch, FakeRead::Exact, length),
            )
        })
        .collect();
    HealthyPortableService::open(
        topology.clone(),
        members,
        recovery_for(&topology),
        ServiceConfig::default(),
    )
    .unwrap()
}
struct DeterministicWriteHarness<S: RandomAccessStore, R: RecoveryStateStore> {
    service: HealthyPortableService<S, R>,
    submission: PortableWriteSubmission,
    accepted: Vec<AcceptedPortableWriteWork>,
    ready: Vec<PortableWriteResult>,
}

impl<S: RandomAccessStore, R: RecoveryStateStore> DeterministicWriteHarness<S, R> {
    fn new(
        mut service: HealthyPortableService<S, R>,
        request: BlockRequest,
        bytes: &[u8],
    ) -> Result<Self, ServiceError> {
        let submission = service.submit_write(request, bytes)?;
        Ok(Self {
            service,
            submission,
            accepted: Vec::new(),
            ready: Vec::new(),
        })
    }

    fn grant_basis(&mut self) -> Result<(), ServiceError> {
        self.service.grant_basis_read_permission(&self.submission)
    }

    fn emit_without_execution(&mut self) -> Result<PortableWriteDrive, ServiceError> {
        let turn = self.service.drive_write(&self.submission)?;
        if let PortableWriteDrive::Work(work) = &turn {
            let accepted = self.service.accept_write_work(work)?;
            self.accepted.push(accepted);
        }
        Ok(turn)
    }

    fn execute_accepted(&mut self, index: usize) -> Result<(), ServiceError> {
        let accepted = self.accepted.swap_remove(index);
        self.ready.push(self.service.execute_write_work(accepted)?);
        Ok(())
    }

    fn emit_one(&mut self) -> Result<PortableWriteDrive, ServiceError> {
        let turn = self.emit_without_execution()?;
        if matches!(&turn, PortableWriteDrive::Work(_)) {
            self.execute_accepted(self.accepted.len() - 1)?;
        }
        Ok(turn)
    }

    fn emit_available(&mut self) -> Result<(), ServiceError> {
        loop {
            match self.emit_one()? {
                PortableWriteDrive::Work(_) => {}
                PortableWriteDrive::Wait(_) => return Ok(()),
                PortableWriteDrive::Complete(_) => return Ok(()),
            }
        }
    }

    fn deliver(&mut self, index: usize) -> Result<Option<OperationEvidence>, ServiceError> {
        let result = self.ready.swap_remove(index);
        self.service.deliver_write_result(result)
    }
}
fn finish_retained_write<S: RandomAccessStore, R: RecoveryStateStore>(
    service: &mut HealthyPortableService<S, R>,
    submission: &PortableWriteSubmission,
) -> OperationEvidence {
    loop {
        match service.drive_write(submission).unwrap() {
            PortableWriteDrive::Wait(PortableWriteWait::BasisReadPermission) => {
                service.grant_basis_read_permission(submission).unwrap();
            }
            PortableWriteDrive::Work(work) => {
                let accepted = service.accept_write_work(&work).unwrap();
                let result = service.execute_write_work(accepted).unwrap();
                if let Some(evidence) = service.deliver_write_result(result).unwrap() {
                    return evidence;
                }
            }
            PortableWriteDrive::Complete(evidence) => return evidence,
            PortableWriteDrive::Wait(wait) => {
                panic!("retained write did not complete: {wait:?}");
            }
        }
    }
}

fn reported_store_completion(
    error: ServiceError,
    request: BlockRequest,
    class: FailureClass,
) -> Box<StoreCompletion> {
    assert_eq!(error.request(), Some(request));
    match error {
        ServiceError::Io {
            class: actual,
            completion: Some(completion),
            ..
        } => {
            assert_eq!(actual, class);
            completion
        }
        other => panic!("expected exact store completion evidence, got {other:?}"),
    }
}
#[test]
fn accepted_read_continuation_survives_submit_return_and_later_delivery() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(40),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let submission = service.submit_read(request).unwrap();
    assert_eq!(service.admission_usage().operation_slots, 1);
    assert_eq!(service.admission_usage().backend_submissions, 1);
    assert!(
        !service
            .admission
            .snapshot(submission.operation)
            .unwrap()
            .children[0]
            .terminal
    );
    let completion = FakeStore::completion(
        submission.child,
        range,
        Some(range),
        CompletionDisposition::Success,
        PersistenceEvidence::VolatileOrUnknown,
    );
    let mut wrong_request = submission;
    wrong_request.request.request_id = RequestId(999);
    assert!(
        service
            .complete_read(
                wrong_request,
                PortableReadPayload::new(
                    request.buffer.expect("read request has buffer"),
                    vec![0; BLOCK as usize],
                ),
                StoreCompletionDelivery::new(
                    StoreId(1),
                    StoreIncarnationId(1),
                    epoch,
                    completion.clone(),
                ),
            )
            .is_err()
    );
    assert_eq!(service.admission_usage().backend_submissions, 1);

    let wrong_delivery =
        StoreCompletionDelivery::new(StoreId(2), StoreIncarnationId(1), epoch, completion.clone());
    assert!(
        service
            .complete_read(
                submission,
                PortableReadPayload::new(
                    dwv_core::BufferToken::new(99, 1),
                    vec![0; BLOCK as usize],
                ),
                StoreCompletionDelivery::new(
                    StoreId(1),
                    StoreIncarnationId(1),
                    epoch,
                    completion.clone(),
                ),
            )
            .is_err()
    );
    assert_eq!(service.admission_usage().backend_submissions, 1);
    assert!(
        service
            .complete_read(
                submission,
                PortableReadPayload::new(
                    request.buffer.expect("read request has buffer"),
                    vec![0; BLOCK as usize],
                ),
                wrong_delivery,
            )
            .is_err()
    );
    assert_eq!(service.admission_usage().backend_submissions, 1);
    let delivery =
        StoreCompletionDelivery::new(StoreId(1), StoreIncarnationId(1), epoch, completion);
    let (bytes, evidence) = service
        .complete_read(
            submission,
            PortableReadPayload::new(
                request.buffer.expect("read request has buffer"),
                vec![0; BLOCK as usize],
            ),
            delivery,
        )
        .unwrap()
        .expect("non-abandoned completion is returned");
    assert_eq!(bytes.len(), BLOCK as usize);
    assert_eq!(evidence.completion.completed.as_slice(), &[range]);
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert_eq!(service.admission_usage().backend_submissions, 0);
}
#[test]
fn deferred_read_rejects_foreign_delivery_without_mutating_other_slot() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let first_request = request(
        RequestId(42),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let second_request = request(
        RequestId(43),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let first = service.submit_read(first_request).unwrap();
    let second = service.submit_read(second_request).unwrap();
    let second_completion = FakeStore::completion(
        second.child,
        range,
        Some(range),
        CompletionDisposition::Success,
        PersistenceEvidence::VolatileOrUnknown,
    );

    assert!(
        service
            .complete_read(
                first,
                PortableReadPayload::new(
                    first_request.buffer.expect("first read has buffer"),
                    vec![0; BLOCK as usize],
                ),
                StoreCompletionDelivery::new(
                    StoreId(1),
                    StoreIncarnationId(1),
                    epoch,
                    second_completion,
                ),
            )
            .is_err()
    );
    assert_eq!(service.admission_usage().operation_slots, 2);
    assert_eq!(service.admission_usage().backend_submissions, 2);
    assert!(
        !service
            .admission
            .snapshot(first.operation)
            .unwrap()
            .children[0]
            .terminal
    );
    assert!(
        !service
            .admission
            .snapshot(second.operation)
            .unwrap()
            .children[0]
            .terminal
    );

    let first_completion = FakeStore::completion(
        first.child,
        range,
        Some(range),
        CompletionDisposition::Success,
        PersistenceEvidence::VolatileOrUnknown,
    );
    service
        .complete_read(
            first,
            PortableReadPayload::new(
                first_request.buffer.expect("first read has buffer"),
                vec![0; BLOCK as usize],
            ),
            StoreCompletionDelivery::new(
                StoreId(1),
                StoreIncarnationId(1),
                epoch,
                first_completion,
            ),
        )
        .unwrap()
        .expect("first completion interest remains required");
    let second_completion = FakeStore::completion(
        second.child,
        range,
        Some(range),
        CompletionDisposition::Success,
        PersistenceEvidence::VolatileOrUnknown,
    );
    service
        .complete_read(
            second,
            PortableReadPayload::new(
                second_request.buffer.expect("second read has buffer"),
                vec![0; BLOCK as usize],
            ),
            StoreCompletionDelivery::new(
                StoreId(1),
                StoreIncarnationId(1),
                epoch,
                second_completion,
            ),
        )
        .unwrap()
        .expect("second completion interest remains required");
    assert_eq!(service.admission_usage().operation_slots, 0);
}

#[test]
fn abandoned_read_completion_is_terminalized_by_slot_owner() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(44),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let submission = service.submit_read(request).unwrap();
    service.abandon(submission.operation).unwrap();
    assert!(
        service
            .admission
            .snapshot(submission.operation)
            .unwrap()
            .abandoned
    );

    let completion = FakeStore::completion(
        submission.child,
        range,
        Some(range),
        CompletionDisposition::Success,
        PersistenceEvidence::VolatileOrUnknown,
    );
    assert!(
        service
            .complete_read(
                submission,
                PortableReadPayload::new(
                    request.buffer.expect("abandoned read has buffer"),
                    vec![0; BLOCK as usize],
                ),
                StoreCompletionDelivery::new(StoreId(1), StoreIncarnationId(1), epoch, completion,),
            )
            .unwrap()
            .is_none(),
        "abandoned completion interest is suppressed after owner terminalization",
    );
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert_eq!(service.admission_usage().backend_submissions, 0);
}
#[test]
fn read_completion_retries_after_terminalization_failure() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(47),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let submission = service.submit_read(request).unwrap();
    let completion = FakeStore::completion(
        submission.child,
        range,
        Some(range),
        CompletionDisposition::Success,
        PersistenceEvidence::VolatileOrUnknown,
    );
    service.inject_terminalization_failure(TerminalizationFault::Reclaim);
    assert!(
        service
            .complete_read(
                submission,
                PortableReadPayload::new(
                    request.buffer.expect("retry read has buffer"),
                    vec![0; BLOCK as usize],
                ),
                StoreCompletionDelivery::new(
                    StoreId(1),
                    StoreIncarnationId(1),
                    epoch,
                    completion.clone(),
                ),
            )
            .is_err()
    );
    assert_eq!(service.admission_usage().operation_slots, 1);
    assert_eq!(service.admission_usage().backend_submissions, 1);

    let (_, evidence) = service
        .complete_read(
            submission,
            PortableReadPayload::new(
                request.buffer.expect("retry read has buffer"),
                vec![0; BLOCK as usize],
            ),
            StoreCompletionDelivery::new(StoreId(1), StoreIncarnationId(1), epoch, completion),
        )
        .unwrap()
        .expect("exact duplicate retries terminalization");
    assert_eq!(evidence.request, request);
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert_eq!(service.admission_usage().backend_submissions, 0);
}

#[test]
fn stale_read_continuation_cannot_address_reused_slot() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let first_request = request(
        RequestId(45),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let second_request = request(
        RequestId(46),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let first = service.submit_read(first_request).unwrap();
    let first_completion = FakeStore::completion(
        first.child,
        range,
        Some(range),
        CompletionDisposition::Success,
        PersistenceEvidence::VolatileOrUnknown,
    );
    service
        .complete_read(
            first,
            PortableReadPayload::new(
                first_request.buffer.expect("first read has buffer"),
                vec![0; BLOCK as usize],
            ),
            StoreCompletionDelivery::new(
                StoreId(1),
                StoreIncarnationId(1),
                epoch,
                first_completion,
            ),
        )
        .unwrap()
        .expect("first completion interest remains required");

    let second = service.submit_read(second_request).unwrap();
    assert_eq!(first.operation.index, second.operation.index);
    assert_ne!(first.operation.generation, second.operation.generation);
    let stale_completion = FakeStore::completion(
        first.child,
        range,
        Some(range),
        CompletionDisposition::Success,
        PersistenceEvidence::VolatileOrUnknown,
    );
    assert!(
        service
            .complete_read(
                first,
                PortableReadPayload::new(
                    first_request.buffer.expect("first read has buffer"),
                    vec![0; BLOCK as usize],
                ),
                StoreCompletionDelivery::new(
                    StoreId(1),
                    StoreIncarnationId(1),
                    epoch,
                    stale_completion,
                ),
            )
            .is_err()
    );
    assert_eq!(service.admission_usage().operation_slots, 1);
    assert!(
        !service
            .admission
            .snapshot(second.operation)
            .unwrap()
            .children[0]
            .terminal
    );

    let second_completion = FakeStore::completion(
        second.child,
        range,
        Some(range),
        CompletionDisposition::Success,
        PersistenceEvidence::VolatileOrUnknown,
    );
    service
        .complete_read(
            second,
            PortableReadPayload::new(
                second_request.buffer.expect("second read has buffer"),
                vec![0; BLOCK as usize],
            ),
            StoreCompletionDelivery::new(
                StoreId(1),
                StoreIncarnationId(1),
                epoch,
                second_completion,
            ),
        )
        .unwrap()
        .expect("second completion interest remains required");
    assert_eq!(service.admission_usage().operation_slots, 0);
}

#[test]
fn deferred_read_child_admission_failure_releases_reserved_operation() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let config = ServiceConfig {
        admission: AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 0, 1, 1, 1),
        },
        ..ServiceConfig::default()
    };
    let mut service = fake_service(FakeRead::Exact, config);
    let request = request(
        RequestId(41),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );

    assert!(matches!(
        service.submit_read(request),
        Err(ServiceError::Io {
            class: FailureClass::Admission,
            ..
        })
    ));
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert_eq!(service.admission_usage().buffers, 0);
    assert_eq!(service.admission_usage().backend_submissions, 0);
    assert_eq!(service.state(), ServiceState::Serving);
}

#[test]
fn coded_claim_mapping_matches_independent_codeword_overlap_oracle() {
    let service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let block = u64::from(BLOCK);
    for start in 0..8 {
        for block_count in 1..=8 - start {
            for chunk_blocks in 1..=block_count {
                let end = start + block_count;
                let mut cursor = start;
                let mut ranges = Vec::new();
                while cursor < end {
                    let chunk_end = (cursor + chunk_blocks).min(end);
                    ranges.push(
                        ByteRange::new(cursor * block, (chunk_end - cursor) * block).unwrap(),
                    );
                    cursor = chunk_end;
                }
                let plan = crate::range::RangePlan { ranges };
                let request = request(
                    RequestId(9_000),
                    TopologyEpoch(4),
                    0,
                    BlockOp::Write,
                    ByteRange::new(start * block, block_count * block).unwrap(),
                    DurabilityIntent::Ordinary,
                );
                let actual = service
                    .coded_claim_for_plan(0, request, &plan, OperationSlotToken::new(0, 1))
                    .unwrap();
                let expected = (0..8)
                    .filter(|unit| {
                        let unit_start = unit * block;
                        let unit_end = unit_start + block;
                        plan.ranges
                            .iter()
                            .any(|range| unit_start < range.end() && range.offset < unit_end)
                    })
                    .map(CodedUnitId)
                    .collect::<BTreeSet<_>>();
                assert_eq!(actual.units(), &expected);
            }
        }
    }
}

#[test]
fn write_driver_start_has_no_physical_io_until_executor_acceptance() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(42),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x3c; BLOCK as usize];
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let submission = service.submit_write(request, &bytes).unwrap();
    assert!(service.members.iter().all(|member| {
        member.store.physical_reads == 0
            && member.store.physical_writes == 0
            && member.store.physical_flushes == 0
    }));
    assert!(matches!(
        service.drive_write(&submission).unwrap(),
        PortableWriteDrive::Wait(PortableWriteWait::BasisReadPermission)
    ));
    service.grant_basis_read_permission(&submission).unwrap();
    let PortableWriteDrive::Work(work) = service.drive_write(&submission).unwrap() else {
        panic!("basis permission should make the first read runnable");
    };
    assert!(service.members.iter().all(|member| {
        member.store.physical_reads == 0
            && member.store.physical_writes == 0
            && member.store.physical_flushes == 0
    }));
    service.accept_write_work(&work).unwrap();
    assert!(service.members.iter().all(|member| {
        member.store.physical_reads == 0
            && member.store.physical_writes == 0
            && member.store.physical_flushes == 0
    }));
}
#[test]
fn protected_write_claim_covers_basis_and_releases_with_authorization() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(424),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x3c; BLOCK as usize];
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let submission = service.submit_write(request, &bytes).unwrap();
    assert!(
        service
            .coded_authority()
            .operation_phase(submission.operation)
            .is_none()
    );

    service.grant_basis_read_permission(&submission).unwrap();
    let PortableWriteDrive::Work(basis_data) = service.drive_write(&submission).unwrap() else {
        panic!("coded admission should precede the first basis read");
    };
    assert_eq!(
        service
            .coded_authority()
            .operation_phase(submission.operation),
        Some(dwv_transaction_ref::CodedOperationPhase::Held)
    );
    let driver = service.write_drivers[usize::try_from(submission.operation.index).unwrap()]
        .as_ref()
        .expect("admitted write remains retained");
    assert!(driver.machine.range_guard().is_some());
    assert_eq!(
        service.release_scope(submission.operation),
        ReleaseScope::InScope
    );
    assert!(driver.write_recovery_record.is_some());
    assert_eq!(service.members[0].store.physical_reads, 0);
    let accepted = service.accept_write_work(&basis_data).unwrap();
    let basis_data_result = service.execute_write_work(accepted).unwrap();
    assert!(
        service
            .deliver_write_result(basis_data_result)
            .unwrap()
            .is_none()
    );

    let PortableWriteDrive::Work(basis_parity) = service.drive_write(&submission).unwrap() else {
        panic!("the second basis read should remain under the held claim");
    };
    assert_eq!(
        service
            .coded_authority()
            .operation_phase(submission.operation),
        Some(dwv_transaction_ref::CodedOperationPhase::Held)
    );
    let accepted = service.accept_write_work(&basis_parity).unwrap();
    let basis_parity_result = service.execute_write_work(accepted).unwrap();
    assert!(
        service
            .deliver_write_result(basis_parity_result)
            .unwrap()
            .is_none()
    );

    let PortableWriteDrive::Work(first_write) = service.drive_write(&submission).unwrap() else {
        panic!("coded effect permission should precede the first protected write");
    };
    assert_eq!(
        service
            .coded_authority()
            .operation_phase(submission.operation),
        Some(dwv_transaction_ref::CodedOperationPhase::EffectPossible)
    );
    assert_eq!(service.members[0].store.physical_writes, 0);
    let accepted = service.accept_write_work(&first_write).unwrap();
    let first_write_result = service.execute_write_work(accepted).unwrap();
    assert!(
        service
            .deliver_write_result(first_write_result)
            .unwrap()
            .is_none()
    );

    let evidence = finish_retained_write(&mut service, &submission);
    assert_eq!(evidence.request, request);
    assert!(evidence.release_authorization.is_some());
    assert!(
        service
            .coded_authority()
            .operation_phase(submission.operation)
            .is_none()
    );
}

#[test]
fn protected_write_contention_parks_until_capture_cut_progresses() {
    let epoch = TopologyEpoch(4);
    let first_range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let holder = service
        .submit_write(
            request(
                RequestId(425),
                epoch,
                0,
                BlockOp::Write,
                first_range,
                DurabilityIntent::Ordinary,
            ),
            &[0x41; BLOCK as usize],
        )
        .unwrap();
    service.grant_basis_read_permission(&holder).unwrap();
    let PortableWriteDrive::Work(holder_basis) = service.drive_write(&holder).unwrap() else {
        panic!("holder should emit its first basis read");
    };
    let accepted_holder_basis = service.accept_write_work(&holder_basis).unwrap();
    assert_eq!(
        service
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .unwrap()
            .membership
            .get(&holder.operation),
        Some(&CodedCaptureMembership::Included)
    );

    let contender = service
        .submit_write(
            request(
                RequestId(426),
                epoch,
                1,
                BlockOp::Write,
                first_range,
                DurabilityIntent::Ordinary,
            ),
            &[0x42; BLOCK as usize],
        )
        .unwrap();
    service.grant_basis_read_permission(&contender).unwrap();
    assert_eq!(
        service.drive_write(&contender).unwrap(),
        PortableWriteDrive::Wait(PortableWriteWait::CodedRangeContention)
    );
    assert_eq!(
        service.release_scope(contender.operation),
        ReleaseScope::Outside
    );
    let contender_driver = service.write_drivers
        [usize::try_from(contender.operation.index).unwrap()]
    .as_ref()
    .expect("contended write remains retained");
    assert!(contender_driver.machine.range_guard().is_none());
    assert_eq!(
        contender_driver
            .machine
            .pending_action()
            .map(|action| action.kind()),
        Some(ActionKind::AcquireRange)
    );
    assert!(
        service
            .coded_authority()
            .operation_phase(contender.operation)
            .is_none()
    );
    assert_eq!(service.members[0].store.physical_reads, 0);
    assert_eq!(service.members[1].store.physical_reads, 0);

    let holder_basis_result = service.execute_write_work(accepted_holder_basis).unwrap();
    assert!(
        service
            .deliver_write_result(holder_basis_result)
            .unwrap()
            .is_none()
    );
    finish_retained_write(&mut service, &holder);

    assert!(
        service
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_none(),
        "a resolved last-write capture needs no synthetic later mutation"
    );
    assert!(
        service
            .recovery
            .load_assembly_snapshot()
            .unwrap()
            .coded_captures
            .is_empty()
    );
    let PortableWriteDrive::Work(later_basis) = service.drive_write(&contender).unwrap() else {
        panic!("contended operation should resume without a new submission");
    };
    assert_eq!(later_basis.submission.operation, contender.operation);
    assert_eq!(
        service
            .coded_captures()
            .capture_snapshot(CodedCaptureId(1))
            .unwrap()
            .membership
            .get(&contender.operation),
        Some(&CodedCaptureMembership::Included)
    );
    let accepted = service.accept_write_work(&later_basis).unwrap();
    let later_basis_result = service.execute_write_work(accepted).unwrap();
    assert!(
        service
            .deliver_write_result(later_basis_result)
            .unwrap()
            .is_none()
    );
    let evidence = finish_retained_write(&mut service, &contender);
    assert_eq!(evidence.request, contender.request);
    assert!(service.coded_captures().snapshots().is_empty());
    assert!(service.recovery.snapshot().coded_captures.is_empty());
}

#[test]
fn coded_capture_capacity_parks_admitted_write_until_retirement_progresses() {
    let epoch = TopologyEpoch(4);
    let separation = DIRTY_REGION_BYTES.max(BLAKE3_256_PROFILE.extent_size);
    let mut service = fake_service_with_length(2 * separation);
    service.config.max_coded_captures = 1;
    let first_range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let second_range = ByteRange::new(separation, u64::from(BLOCK)).unwrap();

    let holder = service
        .submit_write(
            request(
                RequestId(427),
                epoch,
                0,
                BlockOp::Write,
                first_range,
                DurabilityIntent::Ordinary,
            ),
            &[0x41; BLOCK as usize],
        )
        .unwrap();
    service.grant_basis_read_permission(&holder).unwrap();
    let PortableWriteDrive::Work(holder_basis) = service.drive_write(&holder).unwrap() else {
        panic!("holder should establish the only retained capture");
    };

    let waiting = service
        .submit_write(
            request(
                RequestId(428),
                epoch,
                0,
                BlockOp::Write,
                second_range,
                DurabilityIntent::Ordinary,
            ),
            &[0x42; BLOCK as usize],
        )
        .unwrap();
    service.grant_basis_read_permission(&waiting).unwrap();
    assert_eq!(
        service.drive_write(&waiting).unwrap(),
        PortableWriteDrive::Wait(PortableWriteWait::CodedCaptureCapacity)
    );
    assert_eq!(service.state(), ServiceState::Serving);
    let admission_sequence = service
        .coded_authority()
        .admission_sequence(waiting.operation)
        .unwrap();
    let waiting_driver = service.write_drivers[usize::try_from(waiting.operation.index).unwrap()]
        .as_ref()
        .unwrap();
    assert!(waiting_driver.coded_admitted);
    assert!(!waiting_driver.failed);
    assert!(waiting_driver.clean_capture.is_none());

    let accepted = service.accept_write_work(&holder_basis).unwrap();
    let holder_basis_result = service.execute_write_work(accepted).unwrap();
    assert!(
        service
            .deliver_write_result(holder_basis_result)
            .unwrap()
            .is_none()
    );
    finish_retained_write(&mut service, &holder);
    assert!(service.coded_captures().snapshots().is_empty());

    let PortableWriteDrive::Work(waiting_basis) = service.drive_write(&waiting).unwrap() else {
        panic!("capacity-blocked write should resume from its retained admission");
    };
    assert_eq!(waiting_basis.submission.operation, waiting.operation);
    assert_eq!(
        service
            .coded_authority()
            .admission_sequence(waiting.operation)
            .unwrap(),
        admission_sequence
    );
}

#[test]
fn sixty_four_sequential_codewords_leave_no_resolved_capture_history() {
    const CODEWORDS: u64 = 64;
    let epoch = TopologyEpoch(4);
    let mut service = fake_service_with_length(CODEWORDS * u64::from(BLOCK));

    for codeword in 0..CODEWORDS {
        let range = ByteRange::new(codeword * u64::from(BLOCK), u64::from(BLOCK)).unwrap();
        let submission = service
            .submit_write(
                request(
                    RequestId(2_000 + codeword),
                    epoch,
                    0,
                    BlockOp::Write,
                    range,
                    DurabilityIntent::Ordinary,
                ),
                &[codeword as u8; BLOCK as usize],
            )
            .unwrap();
        service.grant_basis_read_permission(&submission).unwrap();
        let PortableWriteDrive::Work(basis) = service.drive_write(&submission).unwrap() else {
            panic!("sequential write should emit its present-basis read");
        };
        let accepted = service.accept_write_work(&basis).unwrap();
        let basis_result = service.execute_write_work(accepted).unwrap();
        assert!(
            service
                .deliver_write_result(basis_result)
                .unwrap()
                .is_none()
        );
        assert!(
            service.coded_captures().snapshots().len() <= 1,
            "retained captures exceeded the one in-flight codeword"
        );
        finish_retained_write(&mut service, &submission);
        assert!(service.coded_captures().snapshots().is_empty());
        assert!(service.recovery.snapshot().coded_captures.is_empty());
    }
}
#[test]
fn protected_write_failure_keeps_claim_until_matching_authorization() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(428),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Uncertain, ServiceConfig::default());
    let submission = service
        .submit_write(request, &[0x44; BLOCK as usize])
        .unwrap();
    service.grant_basis_read_permission(&submission).unwrap();
    let PortableWriteDrive::Work(work) = service.drive_write(&submission).unwrap() else {
        panic!("uncertain write should still acquire coded authority first");
    };
    assert_eq!(
        service
            .coded_authority()
            .operation_phase(submission.operation),
        Some(dwv_transaction_ref::CodedOperationPhase::Held)
    );
    let accepted = service.accept_write_work(&work).unwrap();
    let result = service.execute_write_work(accepted).unwrap();
    assert!(service.deliver_write_result(result).is_err());
    assert!(
        service
            .coded_authority()
            .operation_phase(submission.operation)
            .is_some()
    );

    let mut stale = authoritative_release(submission.operation);
    stale.operation = OperationSlotToken::new(
        submission.operation.index,
        submission.operation.generation.wrapping_add(1),
    );
    assert!(service.reconcile_release_authorization(stale).is_err());
    assert!(
        service
            .coded_authority()
            .operation_phase(submission.operation)
            .is_some()
    );

    let authorization = service
        .reconcile_release_authorization(authoritative_release(submission.operation))
        .unwrap()
        .expect("matching lifecycle evidence should release only the coded claim");
    assert_eq!(authorization.operation(), submission.operation);
    assert!(
        service
            .coded_authority()
            .operation_phase(submission.operation)
            .is_none()
    );
    let capture = service
        .coded_captures()
        .capture_snapshot(CodedCaptureId(0))
        .expect("unresolved capture survives coded claim release");
    assert!(
        capture
            .release_authorized_operations
            .contains(&submission.operation)
    );
    assert_eq!(
        service.recovery.snapshot().coded_captures.as_slice(),
        &[capture]
    );
}
#[test]
fn coded_capture_rejects_incoherent_owner_generation() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    service.checksums.recovery_generation = RecoveryGeneration(1);

    let error = service
        .coded_start_capture(CodedCaptureId(91), [RegionId(0)], [IntegrityExtentId(0)])
        .expect_err("incoherent recovery/checksum generations must refuse capture");
    assert!(
        error
            .to_string()
            .contains("owner topology/recovery/checksum state is incoherent")
    );
    assert!(
        service
            .coded_captures()
            .capture_snapshot(CodedCaptureId(91))
            .is_none()
    );
}

#[test]
fn coded_capture_rejects_incoherent_checksum_topology() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    service.checksums.topology_epoch = TopologyEpoch(5);

    let error = service
        .coded_start_capture(CodedCaptureId(92), [RegionId(0)], [IntegrityExtentId(0)])
        .expect_err("incoherent checksum topology must refuse capture");
    assert!(
        error
            .to_string()
            .contains("owner topology/recovery/checksum state is incoherent")
    );
    assert!(
        service
            .coded_captures()
            .capture_snapshot(CodedCaptureId(92))
            .is_none()
    );
}

#[test]
fn protected_write_persists_owner_later_cut_before_effect() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(429),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    service
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();
    let submission = service
        .submit_write(request, &[0x45; BLOCK as usize])
        .unwrap();
    service.grant_basis_read_permission(&submission).unwrap();
    for _ in 0..2 {
        let PortableWriteDrive::Work(work) = service.drive_write(&submission).unwrap() else {
            panic!("basis reads should remain runnable under the held claim");
        };
        let accepted = service.accept_write_work(&work).unwrap();
        let result = service.execute_write_work(accepted).unwrap();
        assert!(service.deliver_write_result(result).unwrap().is_none());
    }

    let capture = service
        .coded_captures()
        .capture_snapshot(CodedCaptureId(0))
        .expect("earlier capture remains as refused evidence");
    assert_eq!(capture.phase, CodedCapturePhase::Refused);
    assert_eq!(capture.decision, None);
    assert_eq!(
        capture.membership.get(&submission.operation),
        Some(&CodedCaptureMembership::LaterDurableStalesClean)
    );
    let persisted = service.recovery.snapshot();
    assert_eq!(
        persisted
            .coded_captures
            .iter()
            .find(|capture| capture.capture == CodedCaptureId(0)),
        Some(&capture)
    );
    assert!(matches!(
        service.drive_write(&submission).unwrap(),
        PortableWriteDrive::Work(_)
    ));
    assert_eq!(
        service
            .coded_authority()
            .operation_phase(submission.operation),
        Some(dwv_transaction_ref::CodedOperationPhase::EffectPossible)
    );
}

#[test]
fn uncertain_later_cut_revokes_process_local_coded_authority() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(430),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let recovery = LostAckRecovery::new(epoch);
    let recovery_control = recovery.clone();
    let mut service =
        fake_service_with_recovery(FakeRead::Exact, recovery, ServiceConfig::default());
    service
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();
    let submission = service
        .submit_write(request, &[0x46; BLOCK as usize])
        .unwrap();
    service.grant_basis_read_permission(&submission).unwrap();

    recovery_control.lose_commit_after(2);
    assert!(service.drive_write(&submission).is_err());
    assert_eq!(service.state(), ServiceState::Recovering);
    assert!(!service.coded_captures.is_usable());
    assert_eq!(
        service
            .coded_captures
            .capture_snapshot(CodedCaptureId(0))
            .unwrap()
            .membership
            .get(&submission.operation),
        Some(&CodedCaptureMembership::Later)
    );
    assert_eq!(
        recovery_control
            .durable_store()
            .load_assembly_snapshot()
            .unwrap()
            .coded_captures
            .into_iter()
            .find(|capture| capture.capture == CodedCaptureId(0))
            .unwrap()
            .membership
            .get(&submission.operation),
        Some(&CodedCaptureMembership::LaterDurableStalesClean)
    );
}

#[test]
fn blocking_write_uses_the_retained_coded_path() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(430),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let evidence =
        write_with_test_clean_capture(&mut service, request, &[0x46; BLOCK as usize]).unwrap();
    assert_eq!(evidence.request, request);
    let authorization = evidence
        .release_authorization
        .clone()
        .expect("blocking write should expose exact release authorization");
    assert_eq!(
        service
            .coded_authority()
            .operation_phase(authorization.operation()),
        None
    );
}
#[test]
fn blocking_contention_rejects_only_the_unstarted_request() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let holder = service
        .submit_write(
            request(
                RequestId(431),
                epoch,
                0,
                BlockOp::Write,
                range,
                DurabilityIntent::Ordinary,
            ),
            &[0x47; BLOCK as usize],
        )
        .unwrap();
    service.grant_basis_read_permission(&holder).unwrap();
    let PortableWriteDrive::Work(work) = service.drive_write(&holder).unwrap() else {
        panic!("holder should acquire coded authority before the blocking contender");
    };
    service.accept_write_work(&work).unwrap();

    let error = service
        .write(
            request(
                RequestId(432),
                epoch,
                1,
                BlockOp::Write,
                range,
                DurabilityIntent::Ordinary,
            ),
            &[0x48; BLOCK as usize],
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ServiceError::Io {
            class: FailureClass::CodedContention,
            ..
        }
    ));
    assert_eq!(service.state(), ServiceState::Serving);
    assert_eq!(service.admission_usage().operation_slots, 1);
    assert!(
        service
            .coded_authority()
            .operation_phase(holder.operation)
            .is_some()
    );
}

#[test]
fn blocking_cancel_refuses_after_transaction_range_acquired() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(433),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let submission = service
        .submit_write(request, &[0x49; BLOCK as usize])
        .unwrap();
    service.grant_basis_read_permission(&submission).unwrap();
    let PortableWriteDrive::Work(_) = service.drive_write(&submission).unwrap() else {
        panic!("admitted write should emit basis work");
    };

    let error = service
        .cancel_unstarted_write(submission.operation)
        .unwrap_err();
    assert!(matches!(
        error,
        ServiceError::Io {
            class: FailureClass::ReconciliationRequired,
            ..
        }
    ));
    assert_eq!(
        service.release_scope(submission.operation),
        ReleaseScope::InScope
    );
    assert_eq!(
        service
            .coded_authority()
            .operation_phase(submission.operation),
        Some(dwv_transaction_ref::CodedOperationPhase::Held)
    );
}

#[test]
fn write_acceptance_revalidates_current_store_identity_before_io() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(142),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let submission = service
        .submit_write(request, &[0x3d; BLOCK as usize])
        .unwrap();
    service.grant_basis_read_permission(&submission).unwrap();
    let PortableWriteDrive::Work(work) = service.drive_write(&submission).unwrap() else {
        panic!("basis read should be emitted");
    };
    let member_index = service
        .members
        .iter()
        .position(|member| member.store_id == work.identity.store_id)
        .unwrap();
    service.members[member_index].store.id = StoreId(99);

    assert!(matches!(
        service.accept_write_work(&work),
        Err(ServiceError::Io {
            class: FailureClass::Identity,
            ..
        })
    ));
    assert_eq!(service.members[member_index].store.physical_reads, 0);
    let snapshot = service.admission.snapshot(submission.operation).unwrap();
    let child = &snapshot.children[usize::try_from(work.identity.operation_id.index).unwrap()];
    assert!(child.submission.is_none());
    assert!(!child.terminal);
}

#[test]
fn deterministic_harness_delivers_basis_and_siblings_out_of_order() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(43),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x4d; BLOCK as usize];
    let service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    assert!(matches!(
        harness.emit_one().unwrap(),
        PortableWriteDrive::Wait(PortableWriteWait::BasisReadPermission)
    ));
    harness.grant_basis().unwrap();
    assert!(matches!(
        harness.emit_one().unwrap(),
        PortableWriteDrive::Work(_)
    ));
    assert!(matches!(
        harness.emit_one().unwrap(),
        PortableWriteDrive::Work(_)
    ));
    assert_eq!(harness.ready.len(), 2);
    assert_eq!(harness.service.members[0].store.physical_reads, 1);
    assert_eq!(harness.service.members[2].store.physical_reads, 1);
    assert_eq!(harness.service.members[0].store.physical_writes, 0);
    assert!(harness.deliver(1).unwrap().is_none());
    assert!(harness.deliver(0).unwrap().is_none());
    harness.emit_available().unwrap();
    assert_eq!(harness.ready.len(), 2);
    assert_eq!(harness.service.members[0].store.physical_writes, 1);
    assert_eq!(harness.service.members[2].store.physical_writes, 1);
    assert!(harness.deliver(1).unwrap().is_none());
    assert!(harness.deliver(0).unwrap().is_none());
    assert!(
        harness
            .service
            .admission
            .snapshot(harness.submission.operation)
            .unwrap()
            .submitted_watermark
            .is_some()
    );
    harness.emit_available().unwrap();
    assert_eq!(harness.ready.len(), 2);
    let mut evidence = None;
    while !harness.ready.is_empty() {
        evidence = harness.deliver(harness.ready.len() - 1).unwrap();
    }
    let evidence = evidence.expect("flush results complete the retained transaction");
    assert_eq!(evidence.request, request);
    assert_eq!(harness.service.members[0].store.physical_flushes, 1);
    assert_eq!(harness.service.members[2].store.physical_flushes, 1);
    assert_eq!(harness.service.admission_usage().operation_slots, 0);
    assert_eq!(harness.service.admission_usage().backend_submissions, 0);
}

#[test]
fn partial_result_delivery_retains_accepted_siblings_and_delays_writes() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(44),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x5e; BLOCK as usize];
    let service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_one().unwrap();
    harness.emit_one().unwrap();
    assert!(harness.deliver(0).unwrap().is_none());
    assert_eq!(harness.service.members[0].store.physical_writes, 0);
    assert_eq!(harness.service.admission_usage().operation_slots, 1);
    assert!(harness.deliver(0).unwrap().is_none());
    harness.emit_available().unwrap();
    let duplicate = harness.ready[0].clone();
    let duplicate_child = duplicate.work.identity.operation_id;
    assert!(harness.deliver(0).unwrap().is_none());
    assert!(
        harness
            .service
            .deliver_write_result(duplicate)
            .unwrap()
            .is_none()
    );
    let snapshot = harness
        .service
        .admission
        .snapshot(harness.submission.operation)
        .unwrap();
    assert_eq!(
        snapshot.children[usize::try_from(duplicate_child.index).unwrap()].duplicate_deliveries,
        1
    );
    assert_eq!(harness.service.admission_usage().operation_slots, 1);
    assert_eq!(harness.service.admission_usage().backend_submissions, 6);
}

#[test]
fn mismatched_result_identity_rejects_without_mutating_retained_work() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(45),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x6f; BLOCK as usize];
    let service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_one().unwrap();
    let snapshot = harness
        .service
        .admission
        .snapshot(harness.submission.operation)
        .unwrap();
    let mut wrong = harness.ready[0].clone();
    wrong.work.identity.store_id = StoreId(99);
    assert!(harness.service.deliver_write_result(wrong).is_err());
    assert_eq!(harness.service.state(), ServiceState::Recovering);
    assert_eq!(
        harness
            .service
            .admission
            .snapshot(harness.submission.operation)
            .unwrap(),
        snapshot
    );
    assert!(harness.deliver(0).unwrap().is_none());
    let mut stale = harness.submission;
    stale.request.request_id = RequestId(999);
    assert!(harness.service.drive_write(&stale).is_err());
}

#[test]
fn short_failed_and_uncertain_results_fail_closed_with_resources_retained() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(48),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x70; BLOCK as usize];
    let service = fake_service(FakeRead::Short, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_available().unwrap();
    assert!(harness.deliver(0).is_err());
    assert!(
        harness
            .service
            .write_drivers
            .get(usize::try_from(harness.submission.operation.index).unwrap())
            .and_then(Option::as_ref)
            .is_some_and(|driver| driver.machine.is_terminal())
    );
    assert_eq!(harness.service.admission_usage().operation_slots, 1);
    assert!(matches!(
        harness.service.drive_write(&harness.submission).unwrap(),
        PortableWriteDrive::Wait(PortableWriteWait::PhysicalResults)
    ));
    assert!(harness.deliver(0).is_err());
    assert!(matches!(
        harness.service.drive_write(&harness.submission).unwrap(),
        PortableWriteDrive::Wait(PortableWriteWait::OwnerReconciliation)
    ));
    let service = fake_service_with_effects(
        FakeRead::Exact,
        FakeEffect::Failed,
        FakeEffect::Success,
        ServiceConfig::default(),
    );
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_one().unwrap();
    harness.emit_one().unwrap();
    harness.deliver(0).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_one().unwrap();
    let PortableWriteDrive::Work(unaccepted) =
        harness.service.drive_write(&harness.submission).unwrap()
    else {
        panic!("second write should be emitted");
    };
    assert!(harness.deliver(0).is_err());
    assert!(harness.service.accept_write_work(&unaccepted).is_err());
    let snapshot = harness
        .service
        .admission
        .snapshot(harness.submission.operation)
        .unwrap();
    let refused =
        &snapshot.children[usize::try_from(unaccepted.identity.operation_id.index).unwrap()];
    assert!(refused.terminal);
    assert!(refused.refused_before_acceptance);
    assert_eq!(harness.service.admission_usage().backend_submissions, 3);
    let service = fake_service(FakeRead::Uncertain, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_available().unwrap();
    assert!(harness.deliver(0).is_err());
    assert_eq!(harness.service.admission_usage().operation_slots, 1);
}

#[test]
fn abandonment_refuses_unaccepted_children_but_retains_accepted_work() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(49),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x71; BLOCK as usize];
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let submission = service.submit_write(request, &bytes).unwrap();
    service.grant_basis_read_permission(&submission).unwrap();
    let PortableWriteDrive::Work(work) = service.drive_write(&submission).unwrap() else {
        panic!("basis read should be emitted");
    };
    let accepted = service.accept_write_work(&work).unwrap();
    let result = service.execute_write_work(accepted).unwrap();
    service.abandon(submission.operation).unwrap();
    assert_eq!(service.state(), ServiceState::Recovering);
    assert!(service.submit_write(request, &bytes).is_err());
    assert_eq!(service.admission_usage().backend_submissions, 1);
    assert!(service.deliver_write_result(result).unwrap().is_none());
    assert!(matches!(
        service.drive_write(&submission).unwrap(),
        PortableWriteDrive::Wait(PortableWriteWait::OwnerReconciliation)
    ));
    assert_eq!(service.members[0].store.physical_writes, 0);
    assert_eq!(service.members[2].store.physical_writes, 0);
    assert_eq!(service.members[0].store.physical_flushes, 0);
    assert_eq!(service.members[2].store.physical_flushes, 0);
    let snapshot = service.admission.snapshot(submission.operation).unwrap();
    assert!(
        snapshot
            .children
            .iter()
            .any(|child| child.submission.is_some() && child.terminal)
    );
}

#[test]
fn terminal_duplicate_result_is_rejected_after_release() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(50),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x72; BLOCK as usize];
    let service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_one().unwrap();
    harness.emit_one().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    harness.deliver(0).unwrap();
    let duplicate = harness.ready[0].clone();
    let evidence = harness.deliver(0).unwrap().expect("terminal result");
    assert_eq!(evidence.request, request);
    assert!(harness.service.deliver_write_result(duplicate).is_err());
    assert!(harness.service.drive_write(&harness.submission).is_err());
}
#[test]
fn finalization_cleanup_failure_retries_from_drive_without_duplicate_result() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(51),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x73; BLOCK as usize];
    let service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_one().unwrap();
    harness.emit_one().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    harness
        .service
        .inject_terminalization_failure(TerminalizationFault::Reclaim);
    harness.deliver(0).unwrap();
    assert!(harness.deliver(0).is_err());
    assert_eq!(harness.service.state(), ServiceState::Recovering);
    assert!(
        harness
            .service
            .write_drivers
            .get(usize::try_from(harness.submission.operation.index).unwrap())
            .and_then(Option::as_ref)
            .is_some_and(|driver| driver.failed)
    );
    assert!(harness.ready.is_empty());
    let authorization = harness
        .service
        .release_authorization(harness.submission.operation)
        .cloned()
        .expect("release authorization survives cleanup failure");
    let generation = harness
        .service
        .recovery
        .load_assembly_snapshot()
        .unwrap()
        .generation;
    harness.service.checksums.recovery_generation = generation
        .checked_next()
        .expect("fixture generation is bounded");
    assert!(!harness.service.recovery_topology_generation_coherent());
    let PortableWriteDrive::Complete(evidence) =
        harness.service.drive_write(&harness.submission).unwrap()
    else {
        panic!("semantic finalization should resume without another result");
    };
    assert_eq!(evidence.request, request);
    assert_eq!(evidence.release_authorization, Some(authorization));
    assert_eq!(harness.service.admission_usage().operation_slots, 0);
    assert_eq!(harness.service.admission_usage().backend_submissions, 0);
}

#[test]
fn rejected_coded_release_commit_retries_before_slot_reuse() {
    let epoch = TopologyEpoch(4);
    let request = request(
        RequestId(54),
        epoch,
        0,
        BlockOp::Write,
        ByteRange::new(0, u64::from(BLOCK)).unwrap(),
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x75; BLOCK as usize];
    let recovery = LostAckRecovery::new(epoch);
    let recovery_control = recovery.clone();
    let service = fake_service_with_recovery(FakeRead::Exact, recovery, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_one().unwrap();
    harness.emit_one().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    harness.deliver(0).unwrap();

    recovery_control.reject_commit_after(1);
    assert!(harness.deliver(0).is_err());
    assert!(
        harness
            .service
            .coded_captures
            .operation_phase(harness.submission.operation)
            .is_some()
    );
    assert_eq!(harness.service.admission_usage().operation_slots, 1);

    let PortableWriteDrive::Complete(evidence) =
        harness.service.drive_write(&harness.submission).unwrap()
    else {
        panic!("coded release should retry before slot cleanup");
    };
    assert_eq!(evidence.request, request);
    assert!(
        harness
            .service
            .coded_captures
            .operation_phase(harness.submission.operation)
            .is_none()
    );
    assert_eq!(harness.service.admission_usage().operation_slots, 0);
}

#[test]
fn uncertain_clean_commit_revokes_process_local_coded_authority() {
    let epoch = TopologyEpoch(4);
    let request = request(
        RequestId(55),
        epoch,
        0,
        BlockOp::Write,
        ByteRange::new(0, u64::from(BLOCK)).unwrap(),
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x76; BLOCK as usize];
    let recovery = LostAckRecovery::new(epoch);
    let recovery_control = recovery.clone();
    let service = fake_service_with_recovery(FakeRead::Exact, recovery, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_one().unwrap();
    harness.emit_one().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    harness.deliver(0).unwrap();

    recovery_control.lose_next_commit();
    assert!(harness.deliver(0).is_err());
    assert_eq!(harness.service.state(), ServiceState::Recovering);
    assert!(!harness.service.coded_captures.is_usable());
    let local = harness
        .service
        .coded_captures
        .snapshots()
        .into_iter()
        .find(|capture| capture.phase == CodedCapturePhase::Open)
        .expect("lost acknowledgement must retain the local predecessor");
    assert!(
        recovery_control
            .durable_store()
            .load_assembly_snapshot()
            .unwrap()
            .coded_captures
            .into_iter()
            .any(|capture| {
                capture.capture == local.capture && capture.phase == CodedCapturePhase::CleanKnown
            })
    );
}

#[test]
fn volatile_flush_failure_reconciles_retained_machine_once() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(52),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x74; BLOCK as usize];
    let service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_one().unwrap();
    harness.emit_one().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    let mut volatile = harness.ready.swap_remove(0);
    volatile.completion.persistence = PersistenceEvidence::VolatileOrUnknown;
    assert!(
        harness
            .service
            .deliver_write_result(volatile)
            .unwrap()
            .is_none()
    );
    let durable = harness.ready.swap_remove(0);
    assert!(harness.service.deliver_write_result(durable).is_err());
    assert_eq!(harness.service.state(), ServiceState::Recovering);
    assert!(
        harness
            .service
            .write_drivers
            .get(usize::try_from(harness.submission.operation.index).unwrap())
            .and_then(Option::as_ref)
            .is_some_and(|driver| driver.failed)
    );
    assert!(matches!(
        harness.service.drive_write(&harness.submission).unwrap(),
        PortableWriteDrive::Wait(PortableWriteWait::OwnerReconciliation)
    ));
}

#[test]
fn missing_write_watermark_fails_retained_machine_once() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(53),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x75; BLOCK as usize];
    let service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let mut harness = DeterministicWriteHarness::new(service, request, &bytes).unwrap();
    harness.grant_basis().unwrap();
    harness.emit_one().unwrap();
    harness.emit_one().unwrap();
    harness.deliver(1).unwrap();
    harness.deliver(0).unwrap();
    harness.emit_available().unwrap();
    for result in &mut harness.ready {
        result.completion.write_watermark = None;
    }
    while !harness.ready.is_empty() {
        assert!(harness.deliver(0).unwrap().is_none());
    }
    assert!(harness.service.drive_write(&harness.submission).is_err());
    assert_eq!(harness.service.state(), ServiceState::Recovering);
    assert!(
        harness
            .service
            .write_drivers
            .get(usize::try_from(harness.submission.operation.index).unwrap())
            .and_then(Option::as_ref)
            .is_some_and(|driver| driver.failed)
    );
    assert!(matches!(
        harness.service.drive_write(&harness.submission).unwrap(),
        PortableWriteDrive::Wait(PortableWriteWait::OwnerReconciliation)
    ));
}
#[test]
fn blocking_write_facade_uses_retained_split_boundary() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(46),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let bytes = vec![0x71; BLOCK as usize];
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let evidence = write_with_test_clean_capture(&mut service, request, &bytes).unwrap();
    assert_eq!(evidence.request, request);
    assert_eq!(service.members[0].store.physical_writes, 1);
    assert_eq!(service.members[2].store.physical_writes, 1);
    assert_eq!(service.members[0].store.physical_flushes, 1);
    assert_eq!(service.members[2].store.physical_flushes, 1);
    assert_eq!(service.admission_usage().operation_slots, 0);
}

#[test]
fn non_file_store_preserves_exact_and_fail_closed_service_outcomes() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let _ = write_with_test_clean_capture(
        &mut service,
        request(
            RequestId(1),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &[7; BLOCK as usize],
    )
    .unwrap();
    let (bytes, evidence) = service
        .read(request(
            RequestId(2),
            epoch,
            0,
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    assert_eq!(bytes, vec![7; BLOCK as usize]);
    assert_eq!(evidence.completion.completed.as_slice(), &[range]);
    service
        .flush(request(
            RequestId(3),
            epoch,
            0,
            BlockOp::Flush,
            ByteRange::empty(),
            DurabilityIntent::ExplicitFlush,
        ))
        .unwrap();

    for (read, expected) in [
        (FakeRead::Short, CompletionDisposition::Short),
        (
            FakeRead::Failed,
            CompletionDisposition::Failed(StoreError::BackendFailure { code: 5 }),
        ),
        (FakeRead::Uncertain, CompletionDisposition::Uncertain),
    ] {
        let mut service = fake_service(read, ServiceConfig::default());
        let error = service
            .read(request(
                RequestId(4),
                epoch,
                0,
                BlockOp::Read,
                range,
                DurabilityIntent::Ordinary,
            ))
            .unwrap_err();
        assert!(matches!(
            error,
            ServiceError::IncompleteRead { evidence, .. }
                if evidence.disposition == expected
        ));
    }

    let mut stale = fake_service(FakeRead::StaleToken, ServiceConfig::default());
    let admitted = request(
        RequestId(5),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let error = stale.read(admitted).unwrap_err();
    let completion = reported_store_completion(error, admitted, FailureClass::Admission);
    assert_eq!(completion.disposition, CompletionDisposition::Success);
    assert_eq!(completion.completed.as_slice(), &[range]);
    // A rejected stale delivery retains the accepted owner for real
    // reconciliation; it is not converted into synchronous failure cleanup.
    assert_eq!(stale.admission_usage().operation_slots, 1);
    assert_eq!(stale.admission_usage().backend_submissions, 1);

    let exhausted = ServiceConfig {
        admission: AdmissionConfig {
            limits: ResourceLimits::new(0, 128, 256, 32, 32, 8),
        },
        ..ServiceConfig::default()
    };
    let mut service = fake_service(FakeRead::Exact, exhausted);
    assert!(matches!(
        service.read(request(
            RequestId(6),
            epoch,
            0,
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        )),
        Err(ServiceError::Io {
            class: FailureClass::Admission,
            ..
        })
    ));
}

#[test]
fn non_prefix_read_keeps_request_aligned_bytes_and_exact_range() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let half = BLOCK as usize / 2;
    let mut service = fake_service(FakeRead::NonPrefix, ServiceConfig::default());
    service.members[0].store.bytes[half..BLOCK as usize].fill(0x6b);

    let error = service
        .read(request(
            RequestId(7),
            epoch,
            0,
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap_err();

    match error {
        ServiceError::IncompleteRead {
            bytes, evidence, ..
        } => {
            assert_eq!(bytes.len(), BLOCK as usize);
            assert_eq!(&bytes[..half], vec![0; half]);
            assert_eq!(&bytes[half..], vec![0x6b; half]);
            assert_eq!(evidence.requested, range);
            assert_eq!(
                evidence.completed.as_slice(),
                &[ByteRange::new(half as u64, half as u64).unwrap()]
            );
            assert_eq!(evidence.disposition, CompletionDisposition::Short);
        }
        other => panic!("expected non-prefix partial read, got {other:?}"),
    }
}

#[test]
fn non_file_store_preserves_write_flush_and_resource_failure_semantics() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let bytes = [0x3c; BLOCK as usize];

    for effect in [FakeEffect::Failed, FakeEffect::Uncertain] {
        let mut service = fake_service_with_effects(
            FakeRead::Exact,
            effect,
            FakeEffect::Success,
            ServiceConfig::default(),
        );
        let admitted = request(
            RequestId(8),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        );
        let error = service.write(admitted, &bytes).unwrap_err();
        let completion = reported_store_completion(error, admitted, FailureClass::StoreWrite);
        assert_eq!(completion.requested, range);
        assert!(completion.completed.as_slice().is_empty());
        assert_eq!(
            matches!(completion.disposition, CompletionDisposition::Uncertain),
            matches!(effect, FakeEffect::Uncertain)
        );
        assert_eq!(
            &service.members[0].store.bytes[..BLOCK as usize],
            if matches!(effect, FakeEffect::Uncertain) {
                &bytes
            } else {
                &[0; BLOCK as usize]
            }
        );
        assert_eq!(service.state(), ServiceState::Recovering);
        assert_eq!(service.admission_usage().operation_slots, 1);
        assert_eq!(service.admission_usage().backend_submissions, 3);
    }

    for effect in [FakeEffect::Failed, FakeEffect::Uncertain] {
        let mut service = fake_service_with_effects(
            FakeRead::Exact,
            FakeEffect::Success,
            effect,
            ServiceConfig::default(),
        );
        let admitted = request(
            RequestId(9),
            epoch,
            0,
            BlockOp::Flush,
            ByteRange::empty(),
            DurabilityIntent::ExplicitFlush,
        );
        let error = service.flush(admitted).unwrap_err();
        let completion = reported_store_completion(error, admitted, FailureClass::Fence);
        assert!(completion.requested.is_empty());
        assert!(completion.completed.as_slice().is_empty());
        assert_eq!(
            matches!(completion.disposition, CompletionDisposition::Uncertain),
            matches!(effect, FakeEffect::Uncertain)
        );
        assert_eq!(
            completion.persistence,
            PersistenceEvidence::VolatileOrUnknown
        );
        assert_eq!(service.state(), ServiceState::Recovering);
        assert_eq!(service.admission_usage().operation_slots, 0);
        assert_eq!(service.admission_usage().backend_submissions, 0);
    }

    let mut service = fake_service_with_effects(
        FakeRead::Exact,
        FakeEffect::StaleToken,
        FakeEffect::Success,
        ServiceConfig::default(),
    );
    let admitted = request(
        RequestId(10),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let error = service.write(admitted, &bytes).unwrap_err();
    let completion = reported_store_completion(error, admitted, FailureClass::Admission);
    assert_eq!(completion.disposition, CompletionDisposition::Success);
    assert_eq!(completion.completed.as_slice(), &[range]);
    assert_eq!(service.state(), ServiceState::Recovering);
    assert_eq!(service.admission_usage().operation_slots, 1);
    assert_eq!(service.admission_usage().backend_submissions, 3);

    let mut service = fake_service_with_effects(
        FakeRead::Exact,
        FakeEffect::Success,
        FakeEffect::StaleToken,
        ServiceConfig::default(),
    );
    let admitted = request(
        RequestId(11),
        epoch,
        0,
        BlockOp::Flush,
        ByteRange::empty(),
        DurabilityIntent::ExplicitFlush,
    );
    let error = service.flush(admitted).unwrap_err();
    let completion = reported_store_completion(error, admitted, FailureClass::Admission);
    assert_eq!(completion.disposition, CompletionDisposition::Success);
    assert!(completion.persistence.is_durable());
    assert_eq!(service.state(), ServiceState::Recovering);
    // The stale accepted child remains owned; later unaccepted children
    // are refused before physical submission.
    assert_eq!(service.admission_usage().operation_slots, 1);
    assert_eq!(service.admission_usage().backend_submissions, 1);
}

#[test]
fn operation_effect_observation_distinguishes_known_failed_and_uncertain_children() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    for uncertain in [false, true] {
        let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
        let request_id = if uncertain { 13 } else { 12 };
        let token = service
            .reserve(request(
                RequestId(request_id),
                epoch,
                0,
                BlockOp::Write,
                range,
                DurabilityIntent::Ordinary,
            ))
            .unwrap();
        let child = service.admission.child(token, range).unwrap();
        service.admission.submitted(token).unwrap();
        let disposition = if uncertain {
            CompletionDisposition::Uncertain
        } else {
            CompletionDisposition::Failed(StoreError::BackendFailure { code: 74 })
        };
        service
            .admission
            .complete(
                token,
                FakeStore::completion(
                    child,
                    range,
                    None,
                    disposition,
                    PersistenceEvidence::VolatileOrUnknown,
                ),
            )
            .unwrap();
        let observation = service.operation_effect_observation(token).unwrap();
        assert_eq!(
            observation.effect,
            if uncertain {
                OperationEffect::Unresolved
            } else {
                OperationEffect::Terminal
            }
        );
    }
}

#[test]
fn generic_rebuild_adapters_preserve_store_completion_evidence() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let half = u64::from(BLOCK) / 2;

    let mut source = RebuildSource::new(
        AssignmentInstanceId::from_bytes([1; 16]),
        FakeStore::new(StoreId(1), epoch, FakeRead::NonPrefix),
    );
    let error = source.read_exact(range).unwrap_err();
    let completion = error.completion().expect("source completion evidence");
    assert_eq!(completion.disposition, CompletionDisposition::Short);
    assert_eq!(
        completion.completed.as_slice(),
        &[ByteRange::new(half, half).unwrap()]
    );

    let mut replacement = RebuildStore::new(
        FakeStore::new(StoreId(2), epoch, FakeRead::Exact)
            .with_effects(FakeEffect::Uncertain, FakeEffect::Success),
    );
    let error = replacement
        .write_exact(range, &[0x55; BLOCK as usize])
        .unwrap_err();
    let completion = error.completion().expect("write completion evidence");
    assert_eq!(completion.disposition, CompletionDisposition::Uncertain);
    assert!(completion.completed.as_slice().is_empty());

    let mut replacement = RebuildStore::new(
        FakeStore::new(StoreId(4), epoch, FakeRead::Exact).with_effects(
            FakeEffect::SuccessWithoutWriteWatermark,
            FakeEffect::Success,
        ),
    );
    let error = replacement
        .write_exact(range, &[0x5a; BLOCK as usize])
        .unwrap_err();
    let completion = error
        .completion()
        .expect("missing-watermark completion evidence");
    assert_eq!(completion.disposition, CompletionDisposition::Success);
    assert_eq!(completion.completed.as_slice(), &[range]);
    assert_eq!(completion.write_watermark, None);

    let mut replacement = RebuildStore::new(
        FakeStore::new(StoreId(3), epoch, FakeRead::Exact)
            .with_effects(FakeEffect::Success, FakeEffect::Uncertain),
    );
    replacement
        .write_exact(range, &[0x66; BLOCK as usize])
        .unwrap();
    let error = replacement.flush_rebuild().unwrap_err();
    let completion = error.completion().expect("flush completion evidence");
    assert_eq!(completion.disposition, CompletionDisposition::Uncertain);
    assert_eq!(
        completion.persistence,
        PersistenceEvidence::VolatileOrUnknown
    );
}

fn bindings(
    topology: &TopologySnapshot,
    mut open: impl FnMut(u64) -> FileStore,
) -> Vec<MemberBinding<FileStore>> {
    topology
        .assignments()
        .iter()
        .enumerate()
        .map(|(index, assignment)| {
            let store_id = StoreId(index as u64 + 1);
            member_binding(
                assignment,
                topology.topology_epoch(),
                store_id,
                open(store_id.0),
            )
        })
        .collect()
}

fn reopened_bindings(root: &Path, topology: &TopologySnapshot) -> Vec<MemberBinding<FileStore>> {
    let epoch = topology.topology_epoch();
    bindings(topology, |index| {
        FileStore::open(
            FileStoreConfig::new(root.join(format!("member-{index}.raw")), LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .create(true)
                .store_id(StoreId(index))
                .topology_epoch(epoch)
                .sync_mode(FileSyncMode::CallerFlush),
        )
        .unwrap()
    })
}

fn fixture() -> (
    PathBuf,
    HealthyPortableService<FileStore, MemoryRecoveryStore>,
) {
    fixture_with_config(ServiceConfig::default())
}

fn fixture_with_config(
    config: ServiceConfig,
) -> (
    PathBuf,
    HealthyPortableService<FileStore, MemoryRecoveryStore>,
) {
    let root = std::env::temp_dir().join(format!(
        "dwv-service-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let epoch = TopologyEpoch(4);
    let open = |index: u64| {
        FileStore::open(
            FileStoreConfig::new(root.join(format!("member-{index}.raw")), LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .create(true)
                .store_id(StoreId(index))
                .topology_epoch(epoch)
                .sync_mode(FileSyncMode::CallerFlush),
        )
        .unwrap()
    };
    let topology = topology(epoch);
    let members = bindings(&topology, open);
    let service =
        HealthyPortableService::open(topology.clone(), members, recovery_for(&topology), config)
            .unwrap();
    (root, service)
}

fn write_with_test_clean_capture<S, R>(
    service: &mut HealthyPortableService<S, R>,
    request: BlockRequest,
    bytes: &[u8],
) -> Result<OperationEvidence, ServiceError>
where
    S: dwv_store::RandomAccessStore,
    R: dwv_recovery::RecoveryStateStore,
{
    service.write(request, bytes)
}
#[test]
fn mandatory_recovery_baseline_blocks_service_until_exactly_complete() {
    let root = std::env::temp_dir().join(format!(
        "dwv-service-baseline-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let epoch = TopologyEpoch(4);
    let topology = topology(epoch);
    let recovery_topology = dwv_recovery::TopologySnapshot::from_core(
        topology.clone(),
        vec![StoreId(1), StoreId(2), StoreId(3)],
    )
    .unwrap();
    let baseline = dwv_recovery::new_checksum_baseline(
        &recovery_topology,
        RecoveryGeneration::ZERO,
        BLAKE3_256_PROFILE,
        ChecksumSetGeneration::INITIAL,
    )
    .unwrap();
    let mut manifest = dwv_recovery::RecoveryManifest {
        schema: dwv_recovery::CURRENT_RECOVERY_SCHEMA,
        snapshot: RecoverySnapshot {
            generation: RecoveryGeneration::ZERO,
            topology_epoch: epoch,
            active_topology: Some(recovery_topology),
            pending_topology: None,
            writable_session: None,
            dirty_regions: Vec::new(),
            integrity_records: Vec::new(),
            checksum_baseline: Some(baseline),
            fences: Vec::new(),
            roots: Vec::new(),
            next_root_id: dwv_recovery::RecoveryRootId::FIRST,
            legacy_unreconciled: Vec::new(),
            next_fence_occurrence_id: dwv_recovery::FenceOccurrenceId::FIRST,
            maintenance_checkpoints: Vec::new(),
            metadata_loss_audit: Some(dwv_recovery::MetadataLossAudit {
                matrix_version: dwv_recovery::METADATA_LOSS_MATRIX_VERSION,
                lineage_id: topology.array_id(),
                case: dwv_recovery::MetadataLossCase::ChecksumEvidenceUnavailable,
                action: dwv_recovery::MetadataLossAction::RequireDataAuthoritativeRebaseline,
                verification:
                    dwv_recovery::MetadataLossVerification::ExplicitDataAuthoritativeRebaseline,
                baseline: dwv_recovery::BaselineDisposition::NewParityAndChecksumBaselineRequired,
                source_health: RecoveryStoreHealth::Missing,
                topology_epoch: epoch,
            }),
            rebuilds: Vec::new(),
            coded_captures: Vec::new(),
            next_coded_capture_id: 0,
        },
    };
    assert!(matches!(
        assess_checksum_baseline(&manifest.snapshot),
        ChecksumBaselineStatus::Required { total: 3 }
    ));

    let open_members = || {
        bindings(&topology, |index| {
            FileStore::open(
                FileStoreConfig::new(root.join(format!("member-{index}.raw")), LENGTH, BLOCK)
                    .maximum_transfer(LENGTH)
                    .create(true)
                    .store_id(StoreId(index))
                    .topology_epoch(epoch)
                    .sync_mode(FileSyncMode::CallerFlush),
            )
            .unwrap()
        })
    };
    let blocked = HealthyPortableService::open(
        topology.clone(),
        open_members(),
        MemoryRecoveryStore::from_manifest(manifest.clone()).unwrap(),
        ServiceConfig::default(),
    );
    assert!(matches!(
        blocked,
        Err(ServiceError::Io {
            class: FailureClass::Recovery,
            detail,
            ..
        }) if detail.contains("baseline is required")
    ));

    manifest.snapshot.generation = RecoveryGeneration(1);
    let extents = manifest
        .snapshot
        .checksum_baseline
        .as_ref()
        .unwrap()
        .expected_extents
        .clone();
    let mut store_fences = Vec::new();
    for extent in &extents {
        let fence = dwv_store::StoreFenceRef {
            fence_id: dwv_store::FenceId(extent.id.0 + 1),
            store_id: StoreId(extent.id.0 + 1),
            topology_epoch: epoch,
            store_incarnation: dwv_store::StoreIncarnationId(0),
            through: StoreWriteWatermark(0),
            capability_evidence_id: dwv_store::CapabilityEvidenceId(1),
        };
        store_fences.push(fence);
        manifest
            .snapshot
            .integrity_records
            .push(dwv_recovery::IntegrityRecord {
                extent: extent.id,
                state: IntegrityState::Valid {
                    binding: dwv_recovery::ChecksumEvidenceBinding {
                        extent: *extent,
                        profile: manifest
                            .snapshot
                            .checksum_baseline
                            .as_ref()
                            .unwrap()
                            .profile
                            .id,
                        set_generation: manifest
                            .snapshot
                            .checksum_baseline
                            .as_ref()
                            .unwrap()
                            .set_generation,
                        topology_epoch: epoch,
                    },
                    content_generation: RecoveryGeneration::ZERO,
                    durable_fence: fence,
                    fence_occurrence: dwv_recovery::FenceOccurrenceId::FIRST,
                    digest: vec![0; 32],
                    verified_at: RecoveryGeneration(1),
                },
            });
    }
    let mut certificate = FenceCertificate::new(epoch, FenceDomain(1), store_fences, Vec::new())
        .with_occurrence(dwv_recovery::FenceOccurrenceId::FIRST);
    for extent in &extents {
        certificate = certificate.with_integrity_extent(extent.id, RecoveryGeneration::ZERO);
    }
    manifest.snapshot.fences.push(certificate);
    manifest.snapshot.next_fence_occurrence_id = dwv_recovery::FenceOccurrenceId(2);
    for (index, extent) in extents.iter().enumerate() {
        manifest
            .snapshot
            .roots
            .push(dwv_recovery::RecoveryClaimRoot {
                id: dwv_recovery::RecoveryRootId((index as u64) + 1),
                occurrence: dwv_recovery::FenceOccurrenceId::FIRST,
                certificate: manifest.snapshot.fences[0].clone(),
                topology_epoch: epoch,
                generation: RecoveryGeneration(1),
                fact: dwv_recovery::RecoveryRootFact::ValidIntegrity {
                    extent: extent.id,
                    profile: manifest
                        .snapshot
                        .checksum_baseline
                        .as_ref()
                        .unwrap()
                        .profile
                        .id,
                    set_generation: manifest
                        .snapshot
                        .checksum_baseline
                        .as_ref()
                        .unwrap()
                        .set_generation,
                    content_generation: RecoveryGeneration::ZERO,
                    digest: vec![0; 32],
                },
            });
    }
    manifest.snapshot.next_root_id = dwv_recovery::RecoveryRootId((extents.len() as u64) + 1);
    assert_eq!(
        assess_checksum_baseline(&manifest.snapshot),
        ChecksumBaselineStatus::Complete { total: 3 }
    );

    let service = HealthyPortableService::open(
        topology.clone(),
        open_members(),
        MemoryRecoveryStore::from_manifest(manifest).unwrap(),
        ServiceConfig::default(),
    )
    .unwrap();
    assert_eq!(service.checksums().records().len(), 3);
    assert!(
        service
            .checksums()
            .records()
            .iter()
            .all(dwv_recovery::ChecksumRecord::is_valid)
    );
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

/// dwv:req req.healthy-portable-io.healthy-reads-preserve-exact-range-evidence
/// dwv:req req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity
#[test]
fn write_updates_data_and_single_xor_parity_then_reads_exact_bytes() {
    let (root, mut service) = fixture();
    let bytes = vec![0x5a; BLOCK as usize];
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    let write_request = request(
        RequestId(1),
        TopologyEpoch(4),
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let evidence = write_with_test_clean_capture(&mut service, write_request, &bytes).unwrap();
    assert_eq!(evidence.completion.completed.as_slice(), &[range]);
    assert_eq!(evidence.request, write_request);
    assert_eq!(
        evidence.completion.persistence,
        PersistenceClaim::HostFenceOnly
    );
    assert_eq!(service.state(), ServiceState::Serving);
    let read_request = request(
        RequestId(2),
        TopologyEpoch(4),
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let (read, read_evidence) = service.read(read_request).unwrap();
    assert_eq!(read, bytes);
    assert_eq!(read_evidence.completion.completed.as_slice(), &[range]);
    assert_eq!(read_evidence.request, read_request);
    let parity = fs::read(root.join("member-3.raw")).unwrap();
    assert_eq!(&parity[..BLOCK as usize], &[0x5a; BLOCK as usize]);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn partial_write_at_nonzero_offset_updates_only_the_matching_parity_range() {
    let (root, mut service) = fixture();
    let range = ByteRange::new(BLOCK as u64, BLOCK as u64).unwrap();
    write_with_test_clean_capture(
        &mut service,
        request(
            RequestId(4),
            TopologyEpoch(4),
            1,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &vec![0xa5; BLOCK as usize],
    )
    .unwrap();
    let parity = fs::read(root.join("member-3.raw")).unwrap();
    assert_eq!(&parity[..BLOCK as usize], &[0; BLOCK as usize]);
    assert_eq!(
        &parity[BLOCK as usize..(2 * BLOCK) as usize],
        &[0xa5; BLOCK as usize]
    );
    let (read, _) = service
        .read(request(
            RequestId(5),
            TopologyEpoch(4),
            1,
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    assert_eq!(read, vec![0xa5; BLOCK as usize]);
    assert!(
        service
            .checksums()
            .records()
            .iter()
            .any(|record| record.state == dwv_recovery::ChecksumState::Stale)
    );
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn full_overwrite_matches_reference_xor_recomputation() {
    let (root, mut service) = fixture();
    let range = ByteRange::new(0, LENGTH).unwrap();
    write_with_test_clean_capture(
        &mut service,
        request(
            RequestId(10),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &vec![0x33; LENGTH as usize],
    )
    .unwrap();
    write_with_test_clean_capture(
        &mut service,
        request(
            RequestId(11),
            TopologyEpoch(4),
            1,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &vec![0x77; LENGTH as usize],
    )
    .unwrap();
    let parity = fs::read(root.join("member-3.raw")).unwrap();
    assert_eq!(parity, vec![0x44; LENGTH as usize]);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn randomized_single_xor_workload_matches_reference_images() {
    let (root, mut service) = fixture();
    let mut reference = [vec![0_u8; LENGTH as usize], vec![0_u8; LENGTH as usize]];
    let mut seed = 0x5eed_u64;
    for iteration in 0..64_u64 {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let slot = (seed as usize) % reference.len();
        seed = seed.rotate_left(17);
        let block = seed % (LENGTH / BLOCK as u64);
        let range = ByteRange::new(block * BLOCK as u64, BLOCK as u64).unwrap();
        let value = (seed >> 24) as u8;
        let bytes = (0..BLOCK)
            .map(|offset| value.wrapping_add(offset as u8))
            .collect::<Vec<_>>();
        let start = range.offset as usize;
        reference[slot][start..start + bytes.len()].copy_from_slice(&bytes);
        write_with_test_clean_capture(
            &mut service,
            request(
                RequestId(100 + iteration),
                TopologyEpoch(4),
                slot,
                BlockOp::Write,
                range,
                DurabilityIntent::Ordinary,
            ),
            &bytes,
        )
        .unwrap();
        let (read, _) = service
            .read(request(
                RequestId(200 + iteration),
                TopologyEpoch(4),
                slot,
                BlockOp::Read,
                range,
                DurabilityIntent::Ordinary,
            ))
            .unwrap();
        assert_eq!(read, bytes);

        let expected_parity = reference[0]
            .iter()
            .zip(&reference[1])
            .map(|(left, right)| left ^ right)
            .collect::<Vec<_>>();
        assert_eq!(fs::read(root.join("member-1.raw")).unwrap(), reference[0]);
        assert_eq!(fs::read(root.join("member-2.raw")).unwrap(), reference[1]);
        assert_eq!(
            fs::read(root.join("member-3.raw")).unwrap(),
            expected_parity
        );
    }
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn clean_reopen_and_control_rebuild_preserve_ordinary_payloads() {
    let (root, mut service) = fixture();
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    write_with_test_clean_capture(
        &mut service,
        request(
            RequestId(300),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &vec![0x66; BLOCK as usize],
    )
    .unwrap();
    let data_before = fs::read(root.join("member-1.raw")).unwrap();
    let control = ControlProjection::new(root.join("control.sqlite3"));
    assert!(control.available());
    control.add_inventory("epoch", "4").unwrap();
    control.add_history("event", "clean-stop").unwrap();
    let entries = control.export().unwrap().entries;
    control.delete().unwrap();
    assert_eq!(fs::read(root.join("member-1.raw")).unwrap(), data_before);
    control.rebuild(&entries).unwrap();
    assert_eq!(control.export().unwrap().entries, entries);
    drop(service);

    let open = |index: u64| {
        FileStore::open(
            FileStoreConfig::new(root.join(format!("member-{index}.raw")), LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .store_id(StoreId(index))
                .topology_epoch(epoch)
                .sync_mode(FileSyncMode::CallerFlush),
        )
        .unwrap()
    };
    let topology = topology(epoch);
    let members = bindings(&topology, open);
    let mut reopened = HealthyPortableService::open(
        topology.clone(),
        members,
        recovery_for(&topology),
        ServiceConfig::default(),
    )
    .unwrap();
    let (read, _) = reopened
        .read(request(
            RequestId(301),
            epoch,
            0,
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    assert_eq!(read, vec![0x66; BLOCK as usize]);
    drop(reopened);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn active_payload_lease_rejects_competing_store_open() {
    let (root, service) = fixture();
    let result = FileStore::open(
        FileStoreConfig::new(root.join("member-1.raw"), LENGTH, BLOCK)
            .maximum_transfer(LENGTH)
            .store_id(StoreId(1))
            .topology_epoch(TopologyEpoch(4))
            .sync_mode(FileSyncMode::CallerFlush),
    );
    assert!(matches!(
        result,
        Err(dwv_store_file::FileStoreError::Lease(_))
    ));
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn flush_reports_only_host_fence_evidence() {
    let (root, mut service) = fixture();
    let evidence = service
        .flush(request(
            RequestId(12),
            TopologyEpoch(4),
            0,
            BlockOp::Flush,
            ByteRange::empty(),
            DurabilityIntent::ExplicitFlush,
        ))
        .unwrap();
    assert_eq!(
        evidence.completion.persistence,
        PersistenceClaim::HostFenceOnly
    );
    assert_eq!(service.state(), ServiceState::Serving);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn flush_rejects_parity_target() {
    let (root, mut service) = fixture();
    let result = service.flush(request(
        RequestId(13),
        TopologyEpoch(4),
        2,
        BlockOp::Flush,
        ByteRange::empty(),
        DurabilityIntent::ExplicitFlush,
    ));
    assert!(matches!(
        result,
        Err(ServiceError::Invalid {
            class: FailureClass::InvalidRequest,
            ..
        })
    ));
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn flush_admission_failure_releases_partial_children_and_slot() {
    let config = ServiceConfig {
        admission: AdmissionConfig {
            limits: dwv_store::ResourceLimits::new(1, 0, 1, 1, 1, 1),
        },
        ..ServiceConfig::default()
    };
    let (root, mut service) = fixture_with_config(config);
    let admitted = request(
        RequestId(14),
        TopologyEpoch(4),
        0,
        BlockOp::Flush,
        ByteRange::empty(),
        DurabilityIntent::ExplicitFlush,
    );
    let result = service.flush(admitted);
    assert_eq!(result.as_ref().unwrap_err().request(), Some(admitted));
    assert!(matches!(
        result,
        Err(ServiceError::Io {
            class: FailureClass::Admission,
            ..
        })
    ));
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert_eq!(service.admission_usage().backend_submissions, 0);
    assert_eq!(service.state(), ServiceState::Serving);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_flush_intent_is_rejected_on_write_endpoint() {
    let (root, mut service) = fixture();
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    let result = service.write(
        request(
            RequestId(13),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::ExplicitFlush,
        ),
        &[0x11; BLOCK as usize],
    );
    assert!(matches!(
        result,
        Err(ServiceError::Invalid {
            class: FailureClass::InvalidRequest,
            ..
        })
    ));
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert_eq!(
        fs::read(root.join("member-1.raw")).unwrap(),
        vec![0; LENGTH as usize]
    );
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn short_read_returns_partial_evidence_and_reclaims_slot() {
    let (root, mut service) = fixture();
    let path = root.join("member-1.raw");
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(LENGTH - BLOCK as u64).unwrap();
    drop(file);

    let range = ByteRange::new(0, LENGTH).unwrap();
    let admitted = request(
        RequestId(14),
        TopologyEpoch(4),
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let result = service.read(admitted);
    match result {
        Err(ServiceError::IncompleteRead {
            request,
            bytes,
            evidence,
        }) => {
            assert_eq!(*request, admitted);
            assert_eq!(bytes.len(), LENGTH as usize);
            assert_eq!(evidence.requested, range);
            assert_eq!(
                evidence.completed.as_slice(),
                &[ByteRange::new(0, LENGTH - BLOCK as u64).unwrap()]
            );
            assert_eq!(evidence.disposition, CompletionDisposition::Short);
        }
        other => panic!("expected partial read evidence, got {other:?}"),
    }
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert_eq!(service.admission_usage().backend_submissions, 0);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn terminalization_failures_preserve_primary_read_evidence_and_slot() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let config = ServiceConfig {
        admission: AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 8, 1, 1, 1),
        },
        maximum_transfer: Some(u64::from(BLOCK)),
        ..ServiceConfig::default()
    };

    for fault in [
        TerminalizationFault::Snapshot,
        TerminalizationFault::Reconciliation,
    ] {
        let read = match fault {
            TerminalizationFault::Snapshot => FakeRead::Failed,
            TerminalizationFault::Reconciliation => FakeRead::Uncertain,
            TerminalizationFault::Reclaim => unreachable!(),
        };
        let mut service = fake_service(read, config);
        service.inject_terminalization_failure(fault);
        let admitted = request(
            RequestId(50 + fault as u64),
            epoch,
            0,
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        );
        let error = service.read(admitted).unwrap_err();
        match error.primary_error() {
            Some(ServiceError::IncompleteRead {
                request, evidence, ..
            }) => {
                assert_eq!(**request, admitted);
                assert_eq!(evidence.requested, range);
                match fault {
                    TerminalizationFault::Snapshot => assert_eq!(
                        evidence.disposition,
                        CompletionDisposition::Failed(StoreError::BackendFailure { code: 5 })
                    ),
                    TerminalizationFault::Reconciliation => {
                        assert_eq!(evidence.disposition, CompletionDisposition::Uncertain)
                    }
                    TerminalizationFault::Reclaim => unreachable!(),
                }
            }
            other => panic!("expected preserved primary read evidence, got {other:?}"),
        }
        assert!(matches!(
            error.cleanup_error(),
            Some(ServiceError::Io {
                class: FailureClass::ReconciliationRequired,
                ..
            })
        ));
        assert_eq!(service.state(), ServiceState::Recovering);
        assert_eq!(service.admission_usage().operation_slots, 1);
        assert_eq!(service.admission_usage().backend_submissions, 1);
    }
}

#[test]
fn successful_terminalization_reclaims_once_and_surfaces_reclaim_failure() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let config = ServiceConfig {
        admission: AdmissionConfig {
            limits: ResourceLimits::new(1, 1, 8, 1, 1, 1),
        },
        ..ServiceConfig::default()
    };
    let admitted = request(
        RequestId(52),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, config);
    service.inject_terminalization_failure(TerminalizationFault::Reclaim);
    let error = service.read(admitted).unwrap_err();
    assert_eq!(error.request(), Some(admitted));
    assert!(error.primary_error().is_none());
    assert!(matches!(
        error.cleanup_error(),
        Some(ServiceError::Io {
            class: FailureClass::ReconciliationRequired,
            ..
        })
    ));
    assert_eq!(service.state(), ServiceState::Recovering);
    assert_eq!(service.admission_usage().operation_slots, 1);
    assert_eq!(service.admission_usage().backend_submissions, 1);

    let mut service = fake_service(FakeRead::Exact, config);
    service
        .read(admitted)
        .expect("successful read terminalizes and releases once");
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert_eq!(service.admission_usage().backend_submissions, 0);
    let second = request(
        RequestId(53),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    service
        .read(second)
        .expect("released slot can be reused with a new generation");
    assert_eq!(service.admission_usage().operation_slots, 0);
}

fn authoritative_release(token: OperationSlotToken) -> ReleaseReconciliation {
    ReleaseReconciliation {
        operation: token,
        operation_effect: OperationEffect::AuthoritativelyReconciled,
        requirement: ReleaseRequirement::TransactionSatisfied,
        recovery: RecoveryReconciliation::Authoritative,
        basis: BasisConformance::Reconciled,
    }
}
#[test]
fn authoritative_recovery_observation_cannot_terminalize_outstanding_child() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(621),
        epoch,
        0,
        BlockOp::Read,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let submission = service.submit_read(request).unwrap();

    assert!(
        service
            .reconcile_release_authorization(authoritative_release(submission.operation))
            .is_err()
    );
    assert_eq!(service.admission_usage().operation_slots, 1);
    assert_eq!(service.admission_usage().backend_submissions, 1);
    assert!(
        !service
            .admission
            .snapshot(submission.operation)
            .unwrap()
            .children[0]
            .terminal
    );
}

#[test]
fn routine_read_and_flush_do_not_establish_release_authorization() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());

    let (_, read) = service
        .read(request(
            RequestId(60),
            epoch,
            0,
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    assert!(read.release_authorization.is_none());

    let flush = service
        .flush(request(
            RequestId(61),
            epoch,
            0,
            BlockOp::Flush,
            ByteRange::empty(),
            DurabilityIntent::ExplicitFlush,
        ))
        .unwrap();
    assert!(flush.release_authorization.is_none());
    assert_eq!(service.admission_usage().operation_slots, 0);
}

#[test]
fn typed_basis_observation_is_distinct_from_generation_coherence() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let token = service
        .reserve(request(
            RequestId(601),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    assert_eq!(
        service.basis_conformance_observation(token),
        BasisConformance::Unresolved
    );
    service.set_basis_conformance(token, BasisConformance::Consumed);
    assert_eq!(
        service.basis_conformance_observation(token),
        BasisConformance::Consumed
    );
    assert!(BasisConformance::Consumed.is_conformant());
    assert!(BasisConformance::Discarded.is_conformant());
    assert!(BasisConformance::Reconciled.is_conformant());
    assert!(!BasisConformance::Unresolved.is_conformant());

    let generation = service
        .recovery()
        .load_assembly_snapshot()
        .unwrap()
        .generation;
    service.checksums.recovery_generation = generation
        .checked_next()
        .expect("fixture generation is bounded");
    assert!(!service.recovery_topology_generation_coherent());
    assert_eq!(
        service.basis_conformance_observation(token),
        BasisConformance::Consumed
    );
}

#[test]
fn failed_operation_reconciles_to_an_exact_certificate_after_primary_error() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let admitted = request(
        RequestId(62),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service_with_effects(
        FakeRead::Exact,
        FakeEffect::Failed,
        FakeEffect::Success,
        ServiceConfig::default(),
    );
    let error = service
        .write(admitted, &[0x66; BLOCK as usize])
        .unwrap_err();
    assert!(matches!(
        error,
        ServiceError::Io {
            class: FailureClass::StoreWrite,
            ..
        }
    ));
    let token = OperationSlotToken::new(0, 1);
    assert_eq!(service.admission_usage().operation_slots, 1);

    service.inject_terminalization_failure(TerminalizationFault::Reclaim);
    assert!(
        service
            .reconcile_release_authorization(authoritative_release(token))
            .is_err()
    );
    let authorization = service
        .release_authorization(token)
        .cloned()
        .expect("authorization is monotonic across cleanup failure");
    let generation = service
        .recovery()
        .load_assembly_snapshot()
        .unwrap()
        .generation;
    service.checksums.recovery_generation = generation
        .checked_next()
        .expect("fixture generation is bounded");
    let retained = service
        .reconcile_release_authorization(ReleaseReconciliation {
            operation: token,
            operation_effect: OperationEffect::Unresolved,
            requirement: ReleaseRequirement::TransactionUnresolved,
            recovery: RecoveryReconciliation::Unresolved,
            basis: BasisConformance::Unresolved,
        })
        .unwrap()
        .unwrap();
    assert_eq!(retained, authorization);
    assert_eq!(service.release_authorization(token), Some(&authorization));
    assert_eq!(service.admission_usage().operation_slots, 0);
}

#[test]
fn short_and_failed_terminal_evidence_can_keep_certificate_after_cleanup() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    for disposition in [
        CompletionDisposition::Short,
        CompletionDisposition::Failed(StoreError::BackendFailure { code: 73 }),
    ] {
        let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
        let admitted = request(
            RequestId(620 + disposition_code(&disposition)),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        );
        let token = service.reserve(admitted).unwrap();
        // This direct fixture bypasses TransactionMachine; bind its owner-approved
        // acquisition result explicitly instead of inferring scope from the request.
        service.set_release_scope(token, ReleaseScope::InScope);
        let child = service.admission.child(token, range).unwrap();
        service.admission.submitted(token).unwrap();
        service
            .admission
            .complete(
                token,
                FakeStore::completion(
                    child,
                    range,
                    if matches!(&disposition, CompletionDisposition::Short) {
                        Some(ByteRange::new(range.offset, range.length / 2).unwrap())
                    } else {
                        None
                    },
                    disposition,
                    PersistenceEvidence::VolatileOrUnknown,
                ),
            )
            .unwrap();
        let authorization = service
            .reclaim_with_authorization(&ReleaseReconciliation {
                operation: token,
                operation_effect: OperationEffect::Terminal,
                requirement: ReleaseRequirement::TransactionSatisfied,
                recovery: RecoveryReconciliation::Authoritative,
                basis: BasisConformance::Reconciled,
            })
            .unwrap()
            .unwrap();
        assert_eq!(service.release_authorization(token), Some(&authorization));
        assert_eq!(service.admission_usage().operation_slots, 0);
    }
}

fn disposition_code(disposition: &CompletionDisposition) -> u64 {
    match disposition {
        CompletionDisposition::Short => 1,
        CompletionDisposition::Failed(_) => 2,
        _ => 3,
    }
}

#[test]
fn authoritative_older_generation_reconciliation_survives_unrelated_work() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let config = ServiceConfig {
        admission: AdmissionConfig {
            limits: ResourceLimits::new(2, 8, 64, 8, 8, 8),
        },
        ..ServiceConfig::default()
    };
    let admitted = request(
        RequestId(63),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service_with_effects(
        FakeRead::Exact,
        FakeEffect::Failed,
        FakeEffect::Success,
        config,
    );
    service
        .write(admitted, &[0x67; BLOCK as usize])
        .unwrap_err();
    let older = OperationSlotToken::new(0, 1);
    let (_, unrelated) = service
        .read(request(
            RequestId(64),
            epoch,
            0,
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    assert!(unrelated.release_authorization.is_none());
    assert_eq!(service.admission_usage().operation_slots, 1);

    let authorization = service
        .reconcile_release_authorization(authoritative_release(older))
        .unwrap()
        .unwrap();
    assert_eq!(authorization.operation(), older);
    assert_eq!(service.release_authorization(older), Some(&authorization));
}

#[test]
fn cleanup_failure_and_success_do_not_revoke_authorization() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let admitted = request(
        RequestId(65),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    service.inject_terminalization_failure(TerminalizationFault::Reclaim);
    let error =
        write_with_test_clean_capture(&mut service, admitted, &[0x68; BLOCK as usize]).unwrap_err();
    assert!(error.cleanup_error().is_some());
    assert!(
        error.primary_error().is_none(),
        "normal write work completed; only physical cleanup failed"
    );
    let token = OperationSlotToken::new(0, 1);
    let operation_effect = service.operation_effect_observation(token).unwrap();
    assert_eq!(operation_effect.operation, token);
    assert_eq!(
        operation_effect.effect,
        OperationEffect::Terminal,
        "normal write provider must publish exact-generation terminal media effect"
    );
    assert_eq!(
        service.basis_conformance_observation(token),
        BasisConformance::Consumed,
        "normal write provider must publish exact-generation basis consumption before cleanup"
    );
    let stale_submission = PortableWriteSubmission {
        operation: token,
        request: admitted,
    };
    let authorization = service.release_authorization(token).unwrap().clone();
    assert_eq!(service.admission_usage().operation_slots, 1);

    service.release(token).unwrap();
    assert_eq!(service.release_authorization(token), Some(&authorization));
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert!(service.drive_write(&stale_submission).is_err());
    let generation = service
        .recovery()
        .load_assembly_snapshot()
        .unwrap()
        .generation;
    service.checksums.recovery_generation = generation
        .checked_next()
        .expect("fixture generation is bounded");
    assert_eq!(service.release_authorization(token), Some(&authorization));

    let (_, unrelated) = service
        .read(request(
            RequestId(66),
            epoch,
            0,
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    assert!(unrelated.release_authorization.is_none());
    assert_eq!(service.release_authorization(token), Some(&authorization));
}
#[test]
fn lifecycle_owner_withholds_authority_from_every_incomplete_fact_set() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let admitted = request(
        RequestId(6_501),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    service.inject_terminalization_failure(TerminalizationFault::Reclaim);
    write_with_test_clean_capture(&mut service, admitted, &[0x65; BLOCK as usize]).unwrap_err();
    let operation = OperationSlotToken::new(0, 1);

    assert!(
        service
            .lifecycle_authority
            .authorize_release(
                service.admission.lifecycle_slots(),
                operation,
                true,
                true,
                true,
                true,
            )
            .is_some()
    );
    for (effect, transaction, recovery, basis) in [
        (false, true, true, true),
        (true, false, true, true),
        (true, true, false, true),
        (true, true, true, false),
    ] {
        assert!(
            service
                .lifecycle_authority
                .authorize_release(
                    service.admission.lifecycle_slots(),
                    operation,
                    effect,
                    transaction,
                    recovery,
                    basis,
                )
                .is_none()
        );
    }
    assert!(
        service
            .lifecycle_authority
            .authorize_included_live(service.admission.lifecycle_slots(), operation, false)
            .is_none()
    );

    let mut incomplete = fake_service(FakeRead::Exact, ServiceConfig::default());
    let incomplete_operation = incomplete.reserve(admitted).unwrap();
    assert!(
        incomplete
            .lifecycle_authority
            .authorize_release(
                incomplete.admission.lifecycle_slots(),
                incomplete_operation,
                true,
                true,
                true,
                true,
            )
            .is_none(),
        "a token without terminal child, reconciliation, and Reclaimable state is insufficient"
    );
    assert!(
        incomplete
            .lifecycle_authority
            .authorize_included_live(
                incomplete.admission.lifecycle_slots(),
                incomplete_operation,
                true,
            )
            .is_none(),
        "a token without exact live finalization is insufficient"
    );
}

#[test]
fn stale_generation_reconciliation_is_rejected_and_missing_evidence_is_fail_closed() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let admitted = request(
        RequestId(67),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service_with_effects(
        FakeRead::Exact,
        FakeEffect::Failed,
        FakeEffect::Success,
        ServiceConfig::default(),
    );
    service
        .write(admitted, &[0x69; BLOCK as usize])
        .unwrap_err();
    let token = OperationSlotToken::new(0, 1);

    let mut stale = authoritative_release(token);
    stale.operation = OperationSlotToken::new(0, 2);
    assert!(service.reconcile_release_authorization(stale).is_err());
    assert!(service.release_authorization(token).is_none());
    assert_eq!(service.admission_usage().operation_slots, 1);

    let mut missing_transaction = authoritative_release(token);
    missing_transaction.requirement = ReleaseRequirement::TransactionUnresolved;
    assert!(matches!(
        service.reconcile_release_authorization(missing_transaction),
        Err(ServiceError::Io {
            class: FailureClass::ReconciliationRequired,
            ..
        })
    ));
    assert!(service.release_authorization(token).is_none());

    let mut missing_basis = authoritative_release(token);
    missing_basis.basis = BasisConformance::Unresolved;
    assert!(matches!(
        service.reconcile_release_authorization(missing_basis),
        Err(ServiceError::Io {
            class: FailureClass::ReconciliationRequired,
            ..
        })
    ));
    assert!(service.release_authorization(token).is_none());
    assert_eq!(service.admission_usage().operation_slots, 1);
}

#[test]
fn exact_generation_lookup_does_not_use_global_latest_ordering() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let first = write_with_test_clean_capture(
        &mut service,
        request(
            RequestId(68),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &[0x70; BLOCK as usize],
    )
    .unwrap()
    .release_authorization
    .unwrap();
    assert_eq!(
        service.release_authorization(first.operation()),
        Some(&first)
    );

    let second = write_with_test_clean_capture(
        &mut service,
        request(
            RequestId(69),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &[0x71; BLOCK as usize],
    )
    .unwrap()
    .release_authorization
    .unwrap();
    assert_ne!(first.operation(), second.operation());
    assert!(service.release_authorization(first.operation()).is_none());
    assert_eq!(
        service.release_authorization(second.operation()),
        Some(&second)
    );
}
#[test]
fn release_scope_is_bound_to_acquisition_not_write_kind_or_child_count() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, u64::from(BLOCK)).unwrap();
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());

    let outside = service
        .reserve(request(
            RequestId(701),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    assert_eq!(service.release_scope(outside), ReleaseScope::Outside);
    let outside_observation = authoritative_release(outside);
    assert!(
        service
            .reclaim_with_authorization(&outside_observation)
            .unwrap()
            .is_none()
    );
    assert!(service.release_authorization(outside).is_none());

    let in_scope = service
        .reserve(request(
            RequestId(702),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    service.set_release_scope(in_scope, ReleaseScope::InScope);
    assert_eq!(service.release_scope(in_scope), ReleaseScope::InScope);
    let authorization = service
        .reclaim_with_authorization(&authoritative_release(in_scope))
        .unwrap()
        .expect("in-scope zero-child operation can be authorized");
    assert_eq!(authorization.operation(), in_scope);
    assert_eq!(
        service.release_authorization(in_scope),
        Some(&authorization)
    );
    let missing = service
        .reserve(request(
            RequestId(703),
            epoch,
            0,
            BlockOp::Read,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    service.clear_release_scope(missing);
    assert_eq!(service.release_scope(missing), ReleaseScope::Unknown);
    assert!(matches!(
        service.reconcile_release_authorization(authoritative_release(missing)),
        Err(ServiceError::Io {
            class: FailureClass::ReconciliationRequired,
            ..
        })
    ));
    let mismatched = OperationSlotToken::new(missing.index, missing.generation + 1);
    assert_eq!(service.release_scope(mismatched), ReleaseScope::Unknown);
    assert!(
        service
            .reconcile_release_authorization(authoritative_release(mismatched))
            .is_err()
    );
    assert_eq!(service.admission_usage().operation_slots, 1);
}

#[test]
fn failed_child_admission_is_a_pre_transaction_bounded_refusal() {
    let config = ServiceConfig {
        admission: AdmissionConfig {
            limits: dwv_store::ResourceLimits::new(1, 2, 1, 1, 1, 1),
        },
        ..ServiceConfig::default()
    };
    let (root, mut service) = fixture_with_config(config);
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    let admitted = request(
        RequestId(15),
        TopologyEpoch(4),
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let result = service.write(admitted, &vec![0x22; BLOCK as usize]);
    assert_eq!(result.as_ref().unwrap_err().request(), Some(admitted));
    assert!(matches!(
        result,
        Err(ServiceError::Io {
            class: FailureClass::Admission,
            ..
        })
    ));
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert_eq!(service.admission_usage().backend_submissions, 0);
    assert_eq!(service.state(), ServiceState::Serving);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn buffer_admission_failure_releases_the_slot_and_buffers() {
    let config = ServiceConfig {
        admission: AdmissionConfig {
            limits: dwv_store::ResourceLimits::new(1, 0, 8, 1, 1, 1),
        },
        ..ServiceConfig::default()
    };
    let (root, mut service) = fixture_with_config(config);
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    let result = service.write(
        request(
            RequestId(16),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &[0x33; BLOCK as usize],
    );
    assert!(matches!(
        result,
        Err(ServiceError::Io {
            class: FailureClass::Admission,
            ..
        })
    ));
    assert_eq!(service.admission_usage().operation_slots, 0);
    assert_eq!(service.admission_usage().buffers, 0);
    assert_eq!(service.state(), ServiceState::Serving);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn member_claim_rejects_data_parity_identity_alias() {
    let root = std::env::temp_dir().join(format!(
        "dwv-service-alias-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let epoch = TopologyEpoch(4);
    let open = |path: PathBuf, id: u64| {
        FileStore::open(
            FileStoreConfig::new(path, LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .create(true)
                .store_id(StoreId(id))
                .topology_epoch(epoch)
                .sync_mode(FileSyncMode::CallerFlush),
        )
        .unwrap()
    };
    let data0_path = root.join("member-1.raw");
    let data1_path = root.join("member-2.raw");
    let parity_path = root.join("member-3.raw");
    let data0 = open(data0_path.clone(), 1);
    let data1 = open(data1_path, 2);
    fs::hard_link(&data0_path, &parity_path).unwrap();
    assert!(matches!(
        FileStore::open(
            FileStoreConfig::new(parity_path, LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .create(true)
                .store_id(StoreId(3))
                .topology_epoch(epoch)
                .sync_mode(FileSyncMode::CallerFlush),
        ),
        Err(dwv_store_file::FileStoreError::Lease(
            dwv_store_file::FileLeaseError::AlreadyHeld(_)
        ))
    ));
    drop(data0);
    drop(data1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_epoch_is_rejected_before_payload_mutation() {
    let (root, mut service) = fixture();
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    let result = service.write(
        request(
            RequestId(3),
            TopologyEpoch(5),
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &vec![1; BLOCK as usize],
    );
    assert!(matches!(
        result,
        Err(ServiceError::Invalid {
            class: FailureClass::StaleTopology,
            ..
        })
    ));
    assert_eq!(
        fs::read(root.join("member-1.raw")).unwrap(),
        vec![0; LENGTH as usize]
    );
    drop(service);
    fs::remove_dir_all(root).unwrap();
}
/// dwv:req req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch
#[test]
fn stable_slots_select_the_same_members_when_binding_order_changes() {
    let (root, service) = fixture();
    drop(service);
    let epoch = TopologyEpoch(4);
    let topology = topology(epoch);
    let mut members = reopened_bindings(&root, &topology);
    members.reverse();
    let mut service = HealthyPortableService::open(
        topology.clone(),
        members,
        recovery_for(&topology),
        ServiceConfig::default(),
    )
    .unwrap();
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    let bytes = vec![0x8d; BLOCK as usize];
    write_with_test_clean_capture(
        &mut service,
        request(
            RequestId(400),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &bytes,
    )
    .unwrap();
    assert_eq!(
        &fs::read(root.join("member-1.raw")).unwrap()[..BLOCK as usize],
        bytes
    );
    assert_eq!(
        &fs::read(root.join("member-2.raw")).unwrap()[..BLOCK as usize],
        vec![0; BLOCK as usize]
    );
    assert_eq!(
        &fs::read(root.join("member-3.raw")).unwrap()[..BLOCK as usize],
        bytes
    );
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

/// dwv:req req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch
#[test]
fn stable_slots_ignore_topology_coding_and_member_collection_order() {
    let (root, service) = fixture();
    let original_publication = service.publication_identity().unwrap();
    drop(service);
    let epoch = TopologyEpoch(4);
    let base = topology(epoch);
    let mut assignments = base.assignments().to_vec();
    assignments.rotate_left(1);
    let reordered = TopologySnapshot::new(
        base.array_id(),
        epoch,
        base.profile(),
        base.geometry(),
        assignments,
    )
    .unwrap();
    let binding = |slot_id: SlotId, store_id: StoreId| {
        member_binding(
            reordered.assignment_for_slot(slot_id).unwrap(),
            epoch,
            store_id,
            FileStore::open(
                FileStoreConfig::new(
                    root.join(format!("member-{}.raw", store_id.0)),
                    LENGTH,
                    BLOCK,
                )
                .maximum_transfer(LENGTH)
                .store_id(store_id)
                .topology_epoch(epoch)
                .sync_mode(FileSyncMode::CallerFlush),
            )
            .unwrap(),
        )
    };
    let members = vec![
        binding(SlotId::from_bytes([3; 16]), StoreId(3)),
        binding(SlotId::from_bytes([1; 16]), StoreId(1)),
        binding(SlotId::from_bytes([2; 16]), StoreId(2)),
    ];
    let mut service = HealthyPortableService::open(
        reordered.clone(),
        members,
        recovery_for(&reordered),
        ServiceConfig::default(),
    )
    .unwrap();
    assert_eq!(
        service.publication_identity().unwrap(),
        original_publication
    );
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    write_with_test_clean_capture(
        &mut service,
        request(
            RequestId(402),
            epoch,
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ),
        &[0x6c; BLOCK as usize],
    )
    .unwrap();
    assert_eq!(
        &fs::read(root.join("member-1.raw")).unwrap()[..BLOCK as usize],
        &[0x6c; BLOCK as usize]
    );
    assert_eq!(
        &fs::read(root.join("member-2.raw")).unwrap()[..BLOCK as usize],
        &[0; BLOCK as usize]
    );
    assert_eq!(
        &fs::read(root.join("member-3.raw")).unwrap()[..BLOCK as usize],
        &[0x6c; BLOCK as usize]
    );
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn assembly_rejects_stale_assignment_and_store_identity_bindings() {
    let (root, service) = fixture();
    drop(service);
    let epoch = TopologyEpoch(4);

    let stale_topology = topology(epoch);
    let mut members = reopened_bindings(&root, &stale_topology);
    members[0].assignment_generation = AssignmentGeneration(99);
    assert!(matches!(
        HealthyPortableService::open(
            stale_topology,
            members,
            MemoryRecoveryStore::new(epoch),
            ServiceConfig::default(),
        ),
        Err(ServiceError::Invalid {
            class: FailureClass::Identity,
            ..
        })
    ));

    let stale_instance_topology = topology(epoch);
    let mut members = reopened_bindings(&root, &stale_instance_topology);
    members[0].assignment_instance = AssignmentInstanceId([99; 16]);
    assert!(matches!(
        HealthyPortableService::open(
            stale_instance_topology,
            members,
            MemoryRecoveryStore::new(epoch),
            ServiceConfig::default(),
        ),
        Err(ServiceError::Invalid {
            class: FailureClass::Identity,
            ..
        })
    ));

    let mismatched_topology = topology(epoch);
    let mut members = reopened_bindings(&root, &mismatched_topology);
    members[0].store_id = StoreId(99);
    assert!(matches!(
        HealthyPortableService::open(
            mismatched_topology,
            members,
            MemoryRecoveryStore::new(epoch),
            ServiceConfig::default(),
        ),
        Err(ServiceError::Invalid {
            class: FailureClass::Identity,
            ..
        })
    ));

    let stale_store_topology = topology(epoch);
    let members = bindings(&stale_store_topology, |index| {
        FileStore::open(
            FileStoreConfig::new(root.join(format!("member-{index}.raw")), LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .store_id(StoreId(index))
                .topology_epoch(if index == 1 { TopologyEpoch(99) } else { epoch })
                .sync_mode(FileSyncMode::CallerFlush),
        )
        .unwrap()
    });
    assert!(matches!(
        HealthyPortableService::open(
            stale_store_topology,
            members,
            MemoryRecoveryStore::new(epoch),
            ServiceConfig::default(),
        ),
        Err(ServiceError::Invalid {
            class: FailureClass::Identity,
            ..
        })
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn assembly_rejects_stores_swapped_across_stable_topology_assignments() {
    let epoch = TopologyEpoch(4);
    let topology = topology(epoch);
    let owner_topology = dwv_recovery::TopologySnapshot::from_core(
        topology.clone(),
        vec![StoreId(1), StoreId(2), StoreId(3)],
    )
    .unwrap();
    let foreign_topology = dwv_recovery::TopologySnapshot::from_core(
        topology.clone(),
        vec![StoreId(2), StoreId(1), StoreId(3)],
    )
    .unwrap();
    let members = foreign_topology
        .assignments()
        .iter()
        .map(|assignment| {
            MemberBinding::new(
                assignment,
                epoch,
                FakeStore::new(assignment.store_id(), epoch, FakeRead::Exact),
            )
        })
        .collect();
    let mut manifest = dwv_recovery::RecoveryManifest {
        schema: dwv_recovery::CURRENT_RECOVERY_SCHEMA,
        snapshot: MemoryRecoveryStore::new(epoch).snapshot().clone(),
    };
    manifest.snapshot.active_topology = Some(owner_topology);
    let recovery = MemoryRecoveryStore::from_manifest(manifest).unwrap();

    assert!(matches!(
        HealthyPortableService::open(topology, members, recovery, ServiceConfig::default()),
        Err(ServiceError::Invalid {
            class: FailureClass::Identity,
            ..
        })
    ));
}

#[test]
fn parity_slot_cannot_redirect_a_frontend_write() {
    let (root, mut service) = fixture();
    let before = (1..=3)
        .map(|index| fs::read(root.join(format!("member-{index}.raw"))).unwrap())
        .collect::<Vec<_>>();
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    assert!(matches!(
        service.write(
            request(
                RequestId(401),
                TopologyEpoch(4),
                2,
                BlockOp::Write,
                range,
                DurabilityIntent::Ordinary,
            ),
            &vec![0xee; BLOCK as usize],
        ),
        Err(ServiceError::Invalid {
            class: FailureClass::InvalidRequest,
            ..
        })
    ));
    let after = (1..=3)
        .map(|index| fs::read(root.join(format!("member-{index}.raw"))).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(after, before);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_slot_and_payload_mismatch_refuse_before_mutation() {
    let (root, mut service) = fixture();
    let before = (1..=3)
        .map(|index| fs::read(root.join(format!("member-{index}.raw"))).unwrap())
        .collect::<Vec<_>>();
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    let mut missing = request(
        RequestId(403),
        TopologyEpoch(4),
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    missing.slot_id = SlotId::from_bytes([99; 16]);
    assert!(matches!(
        service.write(missing, &[0x4a; BLOCK as usize]),
        Err(ServiceError::Invalid {
            class: FailureClass::InvalidRequest,
            ..
        })
    ));
    assert!(matches!(
        service.write(
            request(
                RequestId(404),
                TopologyEpoch(4),
                0,
                BlockOp::Write,
                range,
                DurabilityIntent::Ordinary,
            ),
            &[0x4a; BLOCK as usize - 1],
        ),
        Err(ServiceError::Invalid {
            class: FailureClass::InvalidRequest,
            ..
        })
    ));
    let after = (1..=3)
        .map(|index| fs::read(root.join(format!("member-{index}.raw"))).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(after, before);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reopening_with_dirty_recovery_state_is_read_only() {
    let (root, service) = fixture();
    drop(service);
    let topology = topology(TopologyEpoch(4));
    let mut recovery = recovery_for(&topology);
    let target = InvalidationTarget::new(vec![RegionId(99)], vec![]);
    let _ = WriteRecoveryRecordCommit::new(
        &mut recovery,
        TopologyEpoch(4),
        RecoveryGeneration::ZERO,
        target,
    )
    .commit()
    .unwrap();
    let open = |index: u64| {
        FileStore::open(
            FileStoreConfig::new(root.join(format!("member-{index}.raw")), LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .store_id(StoreId(index))
                .topology_epoch(TopologyEpoch(4))
                .sync_mode(FileSyncMode::CallerFlush),
        )
        .unwrap()
    };
    let members = bindings(&topology, open);
    let service =
        HealthyPortableService::open(topology, members, recovery, ServiceConfig::default())
            .unwrap();
    assert_eq!(service.state(), ServiceState::Recovering);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrupt_recovery_is_rejected_before_snapshot_load() {
    let (root, service) = fixture();
    drop(service);
    let epoch = TopologyEpoch(4);
    let open = |index: u64| {
        FileStore::open(
            FileStoreConfig::new(root.join(format!("member-{index}.raw")), LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .store_id(StoreId(index))
                .topology_epoch(epoch)
                .sync_mode(FileSyncMode::CallerFlush),
        )
        .unwrap()
    };
    let mut recovery = MemoryRecoveryStore::new(epoch);
    recovery.set_health(dwv_recovery::RecoveryStoreHealth::Corrupt);
    let topology = topology(epoch);
    let members = bindings(&topology, open);
    assert!(matches!(
        HealthyPortableService::open(topology, members, recovery, ServiceConfig::default(),),
        Err(ServiceError::Io {
            class: FailureClass::Recovery,
            ..
        })
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn macos_regular_files_support_separate_target_verification_repair() {
    let root = std::env::temp_dir().join(format!(
        "dwv-verify-file-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let data0 = (0..LENGTH as usize)
        .map(|index| index as u8)
        .collect::<Vec<_>>();
    let data1 = (0..LENGTH as usize)
        .map(|index| 0xa0_u8.wrapping_add(index as u8))
        .collect::<Vec<_>>();
    let parity = data0
        .iter()
        .zip(&data1)
        .map(|(left, right)| left ^ right)
        .collect::<Vec<_>>();
    let data_paths = [root.join("data-0.raw"), root.join("data-1.raw")];
    let parity_path = root.join("parity.raw");
    let target_path = root.join("repair-target.raw");
    fs::write(&data_paths[0], &data0).unwrap();
    fs::write(&data_paths[1], &data1).unwrap();
    let mut corrupt_parity = parity.clone();
    corrupt_parity[1] ^= 0xff;
    fs::write(&parity_path, &corrupt_parity).unwrap();
    fs::write(&target_path, vec![0; LENGTH as usize]).unwrap();
    let epoch = TopologyEpoch(4);
    let open = |path: &PathBuf, id: u64| {
        FileStore::open(
            FileStoreConfig::new(path, LENGTH, BLOCK)
                .maximum_transfer(LENGTH)
                .store_id(StoreId(id))
                .topology_epoch(epoch)
                .sync_mode(FileSyncMode::CallerFlush),
        )
        .unwrap()
    };
    assert!(dwv_store_file::reject_backing_export_alias(&parity_path, &target_path).is_ok());
    let alias_path = root.join("parity-alias.raw");
    fs::hard_link(&parity_path, &alias_path).unwrap();
    assert!(dwv_store_file::reject_backing_export_alias(&parity_path, &alias_path).is_err());

    let ranges = (0..(LENGTH / BLOCK as u64))
        .map(|index| ByteRange::new(index * BLOCK as u64, BLOCK as u64).unwrap())
        .collect::<Vec<_>>();
    let provider = Blake3Provider;
    let digest_records = |bytes: &[u8]| {
        ranges
            .iter()
            .map(|range| {
                DigestEvidence::Current(
                    provider
                        .digest(&bytes[range.offset as usize..range.end() as usize])
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>()
    };
    let evidence = ChecksumEvidence::new(
        vec![digest_records(&data0), digest_records(&data1)],
        digest_records(&parity),
    );
    let geometry = dwv_codec::Geometry::new(vec![LENGTH, LENGTH], LENGTH).unwrap();
    let config = dwv_verify::ScanConfig::new(geometry, BLOCK as u64).unwrap();
    let mut data = vec![
        FileVerificationStore(open(&data_paths[0], 1)),
        FileVerificationStore(open(&data_paths[1], 2)),
    ];
    let mut parity_store = FileVerificationStore(open(&parity_path, 3));
    let mut target = FileVerificationStore(open(&target_path, 4));
    let report = verify_exhaustive(&mut data, &mut parity_store, &config, &evidence).unwrap();
    let plan = plan_repairs(&report);
    assert_eq!(plan.candidates.len(), 1);
    apply_repair(
        &config,
        &mut data,
        &mut parity_store,
        &mut target,
        plan.candidates[0].clone(),
    )
    .unwrap();
    assert_eq!(
        target
            .0
            .read_bytes(ByteRange::new(0, BLOCK as u64).unwrap())
            .unwrap(),
        parity[..BLOCK as usize]
    );
    assert_eq!(fs::read(&parity_path).unwrap(), corrupt_parity);
    drop(target);
    drop(parity_store);
    drop(data);
    fs::remove_dir_all(root).unwrap();
}
/// dwv:req req.healthy-portable-io.publication-identity-is-derived-from-admitted-semantics
#[test]
fn publication_identity_binds_the_admitted_member_observations() {
    let core = topology(TopologyEpoch(1));
    let topology = dwv_recovery::TopologySnapshot::from_core(
        core.clone(),
        vec![StoreId(1), StoreId(2), StoreId(3)],
    )
    .unwrap();
    let identities = |last, source, reverse_observations| {
        [1_u64, 2, 3]
            .into_iter()
            .map(|store_id| {
                let mut observations = vec![
                    dwv_store::IdentityObservation {
                        source: IdentitySourceKind::FileId,
                        fingerprint: [if store_id == 3 { last } else { store_id as u8 }; 16],
                    },
                    dwv_store::IdentityObservation {
                        source,
                        fingerprint: [store_id as u8 + 10; 16],
                    },
                ];
                if reverse_observations {
                    observations.reverse();
                }
                (
                    StoreId(store_id),
                    IdentityObservationSet::new(
                        observations,
                        dwv_store::IdentityAssessment::Confirmed,
                    ),
                )
            })
            .collect::<Vec<_>>()
    };
    let admitted_a =
        publication_identity(&topology, &identities(3, IdentitySourceKind::Path, false)).unwrap();
    assert_eq!(
        admitted_a,
        publication_identity(&topology, &identities(3, IdentitySourceKind::Path, true)).unwrap()
    );
    assert_ne!(
        admitted_a,
        publication_identity(&topology, &identities(4, IdentitySourceKind::Path, false)).unwrap()
    );
    assert_ne!(
        admitted_a,
        publication_identity(
            &topology,
            &identities(3, IdentitySourceKind::Geometry, false),
        )
        .unwrap()
    );
    let mut duplicated = identities(3, IdentitySourceKind::Path, false);
    let identity = &mut duplicated[0].1;
    let mut observations = identity.observations.clone();
    observations.push(observations[0]);
    *identity = IdentityObservationSet::new(observations, identity.assessment);
    assert_ne!(
        admitted_a,
        publication_identity(&topology, &duplicated).unwrap()
    );
    let mut reversed = identities(3, IdentitySourceKind::Path, false);
    reversed.reverse();
    assert_eq!(
        admitted_a,
        publication_identity(&topology, &reversed).unwrap()
    );
    let mut assignments = core.assignments().to_vec();
    assignments.rotate_left(1);
    let reordered_core = TopologySnapshot::new(
        core.array_id(),
        core.topology_epoch(),
        core.profile(),
        core.geometry(),
        assignments,
    )
    .unwrap();
    let reordered_store_ids = reordered_core
        .assignments()
        .iter()
        .map(|assignment| {
            topology
                .assignments()
                .iter()
                .find(|recorded| recorded.slot_id() == assignment.slot_id())
                .unwrap()
                .store_id()
        })
        .collect();
    let reordered =
        dwv_recovery::TopologySnapshot::from_core(reordered_core, reordered_store_ids).unwrap();
    assert_eq!(
        admitted_a,
        publication_identity(&reordered, &identities(3, IdentitySourceKind::Path, false),).unwrap()
    );
    let changed_assessment = topology
        .assignments()
        .iter()
        .map(|assignment| {
            (
                assignment.store_id(),
                IdentityObservationSet::new(
                    identities(3, IdentitySourceKind::Path, false)
                        .into_iter()
                        .find(|(store_id, _)| *store_id == assignment.store_id())
                        .unwrap()
                        .1
                        .observations,
                    IdentityAssessment::Conflicting,
                ),
            )
        })
        .collect::<Vec<_>>();
    assert_ne!(
        admitted_a,
        publication_identity(&topology, &changed_assessment).unwrap()
    );
}
#[path = "tests/coded_range_clean_connect.rs"]
mod coded_range_clean_connect;
#[path = "tests/lifecycle_connect.rs"]
mod lifecycle_connect;
#[path = "tests/persistence_evidence_retirement_connect.rs"]
mod persistence_evidence_retirement_connect;
#[path = "tests/retained_operation_core_connect.rs"]
mod retained_operation_core_connect;
