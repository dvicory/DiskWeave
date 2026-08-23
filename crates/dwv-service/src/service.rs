use crate::{
    admission::{AdmissionConfig, OperationAdmission, slot_error},
    evidence::{
        BasisConformance, CompletionEvidence, OperationEffect, OperationEffectObservation,
        OperationEvidence, PersistenceClaim, RecoveryReconciliation, ReleaseAuthorization,
        ReleaseReconciliation, ReleaseRequirement, ReleaseScope,
    },
    failure::{FailureClass, ServiceError},
    lifecycle::ServiceState,
    range::split_range,
    read::read_member,
    write::update_parity,
};
use dwv_codec::Geometry as CodecGeometry;
use dwv_core::{
    AssignmentGeneration, AssignmentInstanceId, BlockOp, BlockRequest, ByteRange, CodingPosition,
    DurabilityIntent, MemberRole, SlotId, TopologyAssignment, TopologyEpoch, TopologySnapshot,
};
use dwv_recovery::{
    BLAKE3_256_PROFILE, ChecksumAuthority, ChecksumBaselineStatus, ChecksumExtent,
    ChecksumPersistenceEvidence, ChecksumRecord, ChecksumSetGeneration, ChecksumTarget,
    CodedCaptureCoordinator, CodedCaptureId, CodedCaptureScopeInput, ContentGeneration,
    DIRTY_REGION_BYTES, FenceCertificate, IntegrityExtentId, IntegrityState, InvalidationTarget,
    RecoveryGeneration, RecoveryMutation, RecoverySnapshot, RecoveryStateStore,
    RecoveryStoreHealth, RecoveryTxn, RegionId, WriteRecoveryRecordEvidence,
    assess_checksum_baseline, dirty_regions_for_range,
};
use dwv_store::{
    ChildOperationId, CompletedRangeSet, CompletionDisposition, FenceDomain, IdentityAssessment,
    IdentityComparison, IdentityObservationSet, IdentitySourceKind, OperationSlotToken,
    PersistenceEvidence, RandomAccessStore, ReconciliationOutcome, SlotSnapshot, SlotState,
    StoreCompletion, StoreCompletionDelivery, StoreId, StoreSubmissionIdentity,
    StoreWriteWatermark, WriteIntent,
};
use dwv_transaction_ref::{
    ActionKind, ActionResult, CodedAdmissionOutcome, CodedClaimInput, CodedClaimRelease,
    CodedRangeAuthority, CommittedRecoveryGeneration, ComputationResult, ErrorClass,
    ParityComputationPlan, ParityRange, PlannedRead, PlannedWrite, RangeGuardToken, ResultKind,
    SemanticIoResult, StoreWatermark, TraceEvent, TransactionLimits, TransactionMachine,
    TransactionPersistenceEvidence, TransactionPlan, WriteRecoveryRecordRequirement,
};
use std::sync::Arc;
/// Runnable/waiting bookkeeping stays in the service driver. It records why
/// an operation is waiting without assigning wakeup, retry, or fairness policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PendingReason {
    AdmissionContended,
    CaptureBlocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OperationReadiness {
    Runnable,
    Waiting(PendingReason),
}

#[cfg_attr(not(test), allow(dead_code))]
/// Normalized coordination result; scheduler and wakeup policy remain external.
#[must_use = "coded effect permission may remain blocked and must be handled explicitly"]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CodedEffectOutcome {
    Permitted,
    BlockedByCapture,
}
/// dwv:req req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch
pub struct MemberBinding<S: RandomAccessStore> {
    slot_id: SlotId,
    role: MemberRole,
    coding_position: CodingPosition,
    assignment_instance: AssignmentInstanceId,
    assignment_generation: AssignmentGeneration,
    topology_epoch: TopologyEpoch,
    store_id: StoreId,
    store: S,
}

impl<S: RandomAccessStore> MemberBinding<S> {
    pub fn new(
        assignment: &TopologyAssignment,
        topology_epoch: TopologyEpoch,
        store_id: StoreId,
        store: S,
    ) -> Self {
        Self {
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
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicationIdentity([u8; 32]);

impl PublicationIdentity {
    pub fn hex(self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationIdentityError {
    MemberCount,
    InvalidTopology,
    MissingMember(StoreId),
}

impl std::fmt::Display for PublicationIdentityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MemberCount => formatter.write_str("publication member count mismatch"),
            Self::InvalidTopology => formatter.write_str("publication topology is invalid"),
            Self::MissingMember(store_id) => {
                write!(
                    formatter,
                    "publication identity is missing store {store_id:?}"
                )
            }
        }
    }
}

impl std::error::Error for PublicationIdentityError {}
/// dwv:req req.healthy-portable-io.publication-identity-is-derived-from-admitted-semantics
pub fn publication_identity(
    topology: &dwv_recovery::TopologySnapshot,
    identities: &[(StoreId, IdentityObservationSet)],
) -> Result<PublicationIdentity, PublicationIdentityError> {
    if topology.assignments().len() != identities.len() {
        return Err(PublicationIdentityError::MemberCount);
    }
    let mut hash = blake3::Hasher::new();
    hash.update(b"dwv.service.publication.v1");
    hash.update(&topology.array_id().as_bytes());
    hash.update(&topology.topology_epoch().0.to_le_bytes());
    hash.update(&topology.profile().data_slots().to_le_bytes());
    hash.update(&topology.profile().parity_slots().to_le_bytes());
    hash.update(&topology.geometry().protected_length().to_le_bytes());
    hash.update(&topology.geometry().logical_block_size().to_le_bytes());
    for assignment in topology.assignments() {
        hash.update(&assignment.slot_id().as_bytes());
        hash.update(&[assignment.role().id().0]);
        hash.update(&assignment.coding_position().0.to_le_bytes());
        hash.update(&assignment.assignment_instance().as_bytes());
        hash.update(&assignment.assignment_generation().0.to_le_bytes());
        hash.update(&assignment.store_id().0.to_le_bytes());
        let evidence = assignment.evidence();
        hash.update(&[
            evidence.observed_sources(),
            evidence.stable_sources(),
            evidence.conflict_count(),
            confidence_id(evidence.confidence()),
        ]);
        let identity = identities
            .iter()
            .find_map(|(store_id, identity)| {
                (*store_id == assignment.store_id()).then_some(identity)
            })
            .ok_or(PublicationIdentityError::MissingMember(
                assignment.store_id(),
            ))?;
        hash.update(&[identity_assessment_id(identity.assessment)]);
        hash.update(&(identity.observations.len() as u64).to_le_bytes());
        for observation in &identity.observations {
            hash.update(&[identity_source_id(observation.source)]);
            hash.update(&observation.fingerprint);
        }
    }
    Ok(PublicationIdentity(*hash.finalize().as_bytes()))
}

const fn confidence_id(confidence: dwv_core::EvidenceConfidence) -> u8 {
    match confidence {
        dwv_core::EvidenceConfidence::None => 0,
        dwv_core::EvidenceConfidence::Low => 1,
        dwv_core::EvidenceConfidence::Medium => 2,
        dwv_core::EvidenceConfidence::High => 3,
        dwv_core::EvidenceConfidence::Attested => 4,
    }
}

const fn identity_assessment_id(assessment: IdentityAssessment) -> u8 {
    match assessment {
        IdentityAssessment::Confirmed => 0,
        IdentityAssessment::Match => 1,
        IdentityAssessment::Changed => 2,
        IdentityAssessment::Clone => 3,
        IdentityAssessment::Ambiguous => 4,
        IdentityAssessment::Conflicting => 5,
        IdentityAssessment::InsufficientEvidence => 6,
        IdentityAssessment::NewDevice => 7,
        IdentityAssessment::Unknown => 8,
    }
}

const fn identity_source_id(source: IdentitySourceKind) -> u8 {
    match source {
        IdentitySourceKind::StableDeviceId => 0,
        IdentitySourceKind::Serial => 1,
        IdentitySourceKind::FilesystemId => 2,
        IdentitySourceKind::WorldWideName => 3,
        IdentitySourceKind::FileId => 4,
        IdentitySourceKind::Capacity => 5,
        IdentitySourceKind::Geometry => 6,
        IdentitySourceKind::OperatorAttestation => 7,
        IdentitySourceKind::Path => 8,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceConfig {
    pub admission: AdmissionConfig,
    pub maximum_transfer: Option<u64>,
    pub fence_domain: FenceDomain,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            admission: AdmissionConfig::default(),
            maximum_transfer: None,
            fence_domain: FenceDomain(1),
        }
    }
}

/// Accepted physical work retained across the submitting call.
///
/// The normalized request carries the exact logical operation, range,
/// ordering, durability, and buffer identity. `identity` binds that request
/// to one child, store incarnation, and topology epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortableOperationSubmission {
    pub operation: OperationSlotToken,
    pub child: ChildOperationId,
    pub identity: StoreSubmissionIdentity,
    pub request: BlockRequest,
}

/// Handle for one retained protected-write transaction. The driver owns the
/// input bytes; the request carries the buffer identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortableWriteSubmission {
    pub operation: OperationSlotToken,
    pub request: BlockRequest,
}

/// Permission supplied by the caller before basis reads may be emitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasisReadPermission {
    Pending,
    Granted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortableWriteWait {
    BasisReadPermission,
    PhysicalResults,
    OwnerReconciliation,
}

/// One normalized physical action. It is only a description: no store method
/// is called until the executor accepts this exact work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortableWriteAction {
    BasisRead {
        destination: ChildOperationId,
    },
    Write {
        payload: Arc<[u8]>,
        intent: WriteIntent,
    },
    Flush {
        through: StoreWriteWatermark,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableWriteWork {
    pub submission: PortableWriteSubmission,
    pub identity: StoreSubmissionIdentity,
    pub range: ByteRange,
    pub action: PortableWriteAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableWriteResult {
    pub work: PortableWriteWork,
    pub completion: StoreCompletion,
    pub read_payload: Option<Vec<u8>>,
}

impl PortableWriteResult {
    pub fn new(
        work: PortableWriteWork,
        completion: StoreCompletion,
        read_payload: Option<Vec<u8>>,
    ) -> Self {
        Self {
            work,
            completion,
            read_payload,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortableWriteDrive {
    Wait(PortableWriteWait),
    Work(PortableWriteWork),
    Complete(OperationEvidence),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WriteWorkState {
    Planned,
    Emitted,
    Accepted,
    Completed,
}

struct PendingWriteWork {
    work: PortableWriteWork,
    state: WriteWorkState,
    result: Option<PortableWriteResult>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WritePhase {
    BasisReads,
    Writes,
    Flushes,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WriteFinalizationPhase {
    Pending,
    WatermarksSet,
    WritesCompleted,
    FlushesCompleted,
    RecoveryCommitted,
    RangeReleased,
    EvidenceReady,
}

struct WriteDriver {
    operation: OperationSlotToken,
    request: BlockRequest,
    payload: Vec<u8>,
    machine: TransactionMachine,
    member_index: usize,
    parity_index: usize,
    coding_position: CodingPosition,
    ranges: Vec<ByteRange>,
    basis_reads: Vec<[usize; 2]>,
    data_write_children: Vec<ChildOperationId>,
    parity_write_children: Vec<ChildOperationId>,
    flush_children: [ChildOperationId; 2],
    write_intent: WriteIntent,
    basis_permission: BasisReadPermission,
    phase: WritePhase,
    finalization: WriteFinalizationPhase,
    write_watermarks: Option<[(StoreSubmissionIdentity, StoreWriteWatermark); 2]>,
    certificate: Option<FenceCertificate>,
    recovery_committed: Option<RecoveryGeneration>,
    work: Vec<PendingWriteWork>,
    regions: Vec<RegionId>,
    checksum_extents: Vec<IntegrityExtentId>,
    write_recovery_record: WriteRecoveryRecordEvidence,
    failed: bool,
    evidence: Option<OperationEvidence>,
}

/// Read bytes correlated with the accepted request's owned buffer.
#[derive(Debug, Eq, PartialEq)]
pub struct PortableReadPayload {
    buffer: dwv_core::BufferToken,
    bytes: Vec<u8>,
}

impl PortableReadPayload {
    pub fn new(buffer: dwv_core::BufferToken, bytes: Vec<u8>) -> Self {
        Self { buffer, bytes }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingOperation {
    request: BlockRequest,
    child: ChildOperationId,
    identity: StoreSubmissionIdentity,
}

impl PendingOperation {
    const fn operation(self) -> OperationSlotToken {
        self.child.slot
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WritableStartAssessment {
    Available,
    RecoveryUnavailable,
    RecoveryRequired,
    MembersUnavailable,
    BaselineRequired,
    BaselineInvalid,
}

/// Read-only preflight owned by the service admission boundary. Actual service
/// open still revalidates topology, member claims, recovery health, and capabilities.
pub fn assess_writable_start(
    snapshot: Option<&RecoverySnapshot>,
    members_current: bool,
) -> WritableStartAssessment {
    let Some(snapshot) = snapshot else {
        return WritableStartAssessment::RecoveryUnavailable;
    };
    if !members_current {
        return WritableStartAssessment::MembersUnavailable;
    }
    if snapshot
        .dirty_regions
        .iter()
        .any(|region| !matches!(region.state, dwv_recovery::RegionState::Clean))
    {
        return WritableStartAssessment::RecoveryRequired;
    }
    match assess_checksum_baseline(snapshot) {
        ChecksumBaselineStatus::NotRequired | ChecksumBaselineStatus::Complete { .. } => {
            WritableStartAssessment::Available
        }
        ChecksumBaselineStatus::Required { .. } | ChecksumBaselineStatus::Partial { .. } => {
            WritableStartAssessment::BaselineRequired
        }
        ChecksumBaselineStatus::Invalid(_) => WritableStartAssessment::BaselineInvalid,
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalizationFault {
    Snapshot,
    Reconciliation,
    Reclaim,
}

enum FinishCleanupError {
    Outstanding,
    Failed(ServiceError),
}

impl FinishCleanupError {
    fn into_service_error(self) -> ServiceError {
        match self {
            Self::Outstanding => ServiceError::io(
                FailureClass::ReconciliationRequired,
                "accepted child work remains outstanding",
            ),
            Self::Failed(error) => error,
        }
    }
}

impl From<ServiceError> for FinishCleanupError {
    fn from(error: ServiceError) -> Self {
        Self::Failed(error)
    }
}

pub struct HealthyPortableService<S: RandomAccessStore, R: RecoveryStateStore> {
    topology: TopologySnapshot,
    members: Vec<MemberBinding<S>>,
    recovery: R,
    pending_operations: Vec<Option<PendingOperation>>,
    write_drivers: Vec<Option<WriteDriver>>,
    operation_readiness: Vec<Option<(OperationSlotToken, OperationReadiness)>>,
    checksums: ChecksumAuthority,
    admission: OperationAdmission,
    config: ServiceConfig,
    state: ServiceState,
    release_authorizations: Vec<Option<ReleaseAuthorization>>,
    release_scopes: Vec<Option<(OperationSlotToken, ReleaseScope)>>,
    basis_observations: Vec<Option<(OperationSlotToken, BasisConformance)>>,
    coded_authority: CodedRangeAuthority,
    coded_captures: CodedCaptureCoordinator,
    #[cfg(test)]
    terminalization_fault: Option<TerminalizationFault>,
}
impl<S: RandomAccessStore, R: RecoveryStateStore> HealthyPortableService<S, R> {
    /// dwv:req req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe
    /// dwv:req req.checksum-plane.current-baseline-completion-is-persisted-and-exact
    pub fn open(
        topology: TopologySnapshot,
        members: Vec<MemberBinding<S>>,
        recovery: R,
        config: ServiceConfig,
    ) -> Result<Self, ServiceError> {
        validate_assembly(&topology, &members)?;
        if config.maximum_transfer == Some(0) {
            return Err(ServiceError::invalid(
                FailureClass::Capability,
                "maximum transfer cannot be zero",
            ));
        }
        let health = recovery.verify_integrity();
        if health != RecoveryStoreHealth::Healthy {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                format!("recovery authority is not healthy: {health:?}"),
            ));
        }
        let snapshot = recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let state = if snapshot.topology_epoch != topology.topology_epoch() {
            ServiceState::Blocked(FailureClass::StaleTopology)
        } else if snapshot
            .dirty_regions
            .iter()
            .any(|region| !matches!(region.state, dwv_recovery::RegionState::Clean))
        {
            ServiceState::Recovering
        } else {
            ServiceState::Serving
        };
        let generation = snapshot.generation;
        let checksums = match assess_checksum_baseline(&snapshot) {
            ChecksumBaselineStatus::NotRequired => checksum_authority(&topology, generation)?,
            ChecksumBaselineStatus::Complete { .. } => persisted_checksum_authority(&snapshot)?,
            ChecksumBaselineStatus::Required { total } => {
                return Err(ServiceError::io(
                    FailureClass::Recovery,
                    format!("mandatory checksum baseline is required for {total} extents"),
                ));
            }
            ChecksumBaselineStatus::Partial { valid, total } => {
                return Err(ServiceError::io(
                    FailureClass::Recovery,
                    format!(
                        "mandatory checksum baseline is partial: {valid} of {total} extents are current"
                    ),
                ));
            }
            ChecksumBaselineStatus::Invalid(reason) => {
                return Err(ServiceError::io(
                    FailureClass::Recovery,
                    format!("mandatory checksum baseline is invalid: {reason:?}"),
                ));
            }
        };
        Ok(Self {
            topology,
            members,
            recovery,
            pending_operations: vec![None; config.admission.limits.operation_slots],
            write_drivers: std::iter::repeat_with(|| None)
                .take(config.admission.limits.operation_slots)
                .collect(),
            checksums,
            admission: OperationAdmission::new(config.admission),
            config,
            state,
            release_authorizations: vec![None; config.admission.limits.operation_slots],
            release_scopes: vec![None; config.admission.limits.operation_slots],
            operation_readiness: vec![None; config.admission.limits.operation_slots],
            basis_observations: vec![None; config.admission.limits.operation_slots],
            coded_authority: CodedRangeAuthority::new(),
            coded_captures: CodedCaptureCoordinator::new(),
            #[cfg(test)]
            terminalization_fault: None,
        })
    }

    pub fn state(&self) -> ServiceState {
        self.state
    }
    pub fn topology(&self) -> &TopologySnapshot {
        &self.topology
    }
    /// dwv:req req.healthy-portable-io.publication-identity-is-derived-from-admitted-semantics
    pub fn publication_identity(&self) -> Result<PublicationIdentity, PublicationIdentityError> {
        let store_ids = self
            .topology
            .assignments()
            .iter()
            .map(|assignment| {
                self.members
                    .iter()
                    .find(|member| {
                        member.slot_id == assignment.slot_id()
                            && member.role == assignment.role()
                            && member.coding_position == assignment.coding_position()
                            && member.assignment_instance == assignment.assignment_instance()
                            && member.assignment_generation == assignment.assignment_generation()
                    })
                    .map(|member| member.store_id)
                    .ok_or(PublicationIdentityError::InvalidTopology)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let topology = dwv_recovery::TopologySnapshot::from_core(self.topology.clone(), store_ids)
            .map_err(|_| PublicationIdentityError::InvalidTopology)?;
        let identities = self
            .members
            .iter()
            .map(|member| (member.store_id, member.store.identity_observations()))
            .collect::<Vec<_>>();
        publication_identity(&topology, &identities)
    }
    pub fn recovery(&self) -> &R {
        &self.recovery
    }
    pub fn recovery_mut(&mut self) -> &mut R {
        &mut self.recovery
    }
    pub fn checksums(&self) -> &ChecksumAuthority {
        &self.checksums
    }
    pub fn admission_usage(&self) -> dwv_store::ResourceUsage {
        self.admission.usage()
    }

    #[cfg(test)]
    pub(crate) fn coded_authority(&self) -> &CodedRangeAuthority {
        &self.coded_authority
    }

    #[cfg(test)]
    pub(crate) fn coded_authority_mut(&mut self) -> &mut CodedRangeAuthority {
        &mut self.coded_authority
    }

    #[cfg(test)]
    pub(crate) fn coded_captures(&self) -> &CodedCaptureCoordinator {
        &self.coded_captures
    }

    #[cfg(test)]
    pub(crate) fn coded_captures_mut(&mut self) -> &mut CodedCaptureCoordinator {
        &mut self.coded_captures
    }

    fn set_readiness(&mut self, operation: OperationSlotToken, readiness: OperationReadiness) {
        if let Ok(index) = usize::try_from(operation.index)
            && let Some(entry) = self.operation_readiness.get_mut(index)
        {
            *entry = Some((operation, readiness));
        }
    }

    #[cfg(test)]
    pub(crate) fn operation_readiness(
        &self,
        operation: OperationSlotToken,
    ) -> Option<OperationReadiness> {
        usize::try_from(operation.index)
            .ok()
            .and_then(|index| self.operation_readiness.get(index).copied())
            .and_then(|entry| {
                let (token, readiness) = entry?;
                (token == operation).then_some(readiness)
            })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn coded_admit(
        &mut self,
        operation: OperationSlotToken,
        claim: CodedClaimInput,
    ) -> Result<CodedAdmissionOutcome, ServiceError> {
        let snapshot = self.admission.snapshot(operation).map_err(slot_error)?;
        if snapshot.state != SlotState::Reserved {
            return Err(ServiceError::io(
                FailureClass::Admission,
                format!(
                    "coded admission requires a reserved operation slot, found {:?}",
                    snapshot.state
                ),
            ));
        }
        let outcome = self
            .coded_authority
            .admit(operation, claim)
            .map_err(|error| ServiceError::io(FailureClass::Admission, error.to_string()))?;
        match outcome {
            CodedAdmissionOutcome::Contended => {
                self.set_readiness(
                    operation,
                    OperationReadiness::Waiting(PendingReason::AdmissionContended),
                );
            }
            CodedAdmissionOutcome::Admitted => {
                let units = self
                    .coded_authority
                    .active_claim(operation)
                    .map(|claim| claim.units().clone())
                    .ok_or_else(|| {
                        ServiceError::io(
                            FailureClass::Admission,
                            "coded authority lost an admitted claim",
                        )
                    })?;
                self.coded_captures
                    .observe_admitted_claim(operation, &units);
                self.set_readiness(operation, OperationReadiness::Runnable);
            }
        }
        Ok(outcome)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn coded_start_capture(
        &mut self,
        capture: CodedCaptureId,
        scope: CodedCaptureScopeInput,
    ) -> Result<(), ServiceError> {
        let active_claims = self
            .coded_authority
            .active_claims()
            .map(|(operation, claim)| (operation, claim.units().clone()))
            .collect::<Vec<_>>();
        self.coded_captures
            .start_capture(capture, scope, active_claims)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn coded_permit_effect(
        &mut self,
        operation: OperationSlotToken,
    ) -> Result<CodedEffectOutcome, ServiceError> {
        self.admission.snapshot(operation).map_err(slot_error)?;
        if !self.coded_captures.effect_allowed(operation) {
            self.set_readiness(
                operation,
                OperationReadiness::Waiting(PendingReason::CaptureBlocked),
            );
            return Ok(CodedEffectOutcome::BlockedByCapture);
        }
        self.coded_authority
            .permit_effect(operation)
            .map_err(|error| ServiceError::io(FailureClass::Admission, error.to_string()))?;
        self.set_readiness(operation, OperationReadiness::Runnable);
        Ok(CodedEffectOutcome::Permitted)
    }

    /// Consume one exact lifecycle-owned release certificate.
    ///
    /// Coded claim removal does not require the operation slot to remain live.
    /// The lifecycle owner supplies the exact generation-qualified certificate.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn coded_release_claim(
        &mut self,
        operation: OperationSlotToken,
        authorization: &ReleaseAuthorization,
    ) -> Result<CodedClaimRelease, ServiceError> {
        self.coded_authority
            .release(operation, Some(authorization.operation))
            .map_err(|error| {
                ServiceError::io(FailureClass::ReconciliationRequired, error.to_string())
            })
    }
    /// Observe authorization only for the exact generation that produced it.
    pub fn release_authorization(
        &self,
        operation: OperationSlotToken,
    ) -> Option<&ReleaseAuthorization> {
        self.release_authorizations
            .iter()
            .filter_map(Option::as_ref)
            .find(|authorization| authorization.operation == operation)
    }

    /// Reconcile one exact retained generation from owner-approved observations.
    ///
    /// A missing observation leaves the slot retained and fails closed. A valid
    /// applicable observation establishes the authorization before cleanup.
    pub(crate) fn reconcile_release_authorization(
        &mut self,
        observation: ReleaseReconciliation,
    ) -> Result<Option<ReleaseAuthorization>, ServiceError> {
        let snapshot = match self.admission.snapshot(observation.operation) {
            Ok(snapshot) => snapshot,
            Err(dwv_store::SlotError::StaleGeneration { .. }) => {
                return Err(ServiceError::io(
                    FailureClass::StaleSlot,
                    "release reconciliation generation is stale",
                ));
            }
            Err(error) => return Err(slot_error(error)),
        };
        if observation.operation != snapshot.token {
            return Err(ServiceError::io(
                FailureClass::StaleSlot,
                "release reconciliation generation does not match the retained slot",
            ));
        }
        match self.release_scope(observation.operation) {
            ReleaseScope::Outside => {
                self.release(observation.operation)?;
                return Ok(None);
            }
            ReleaseScope::Unknown => {
                return Err(ServiceError::io(
                    FailureClass::ReconciliationRequired,
                    "release reconciliation lacks an applicability observation",
                ));
            }
            ReleaseScope::InScope => {}
        }
        if let Some(authorization) = self.release_authorization(observation.operation).cloned() {
            self.release_after_reclaim(observation.operation)?;
            return Ok(Some(authorization));
        }
        if matches!(observation.operation_effect, OperationEffect::Unresolved)
            || matches!(observation.recovery, RecoveryReconciliation::Unresolved)
        {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "release reconciliation lacks authoritative owner evidence",
            ));
        }
        self.set_basis_conformance(observation.operation, observation.basis);
        let authorization = self.reclaim_with_authorization(&observation)?;
        if self.release_scope(observation.operation).is_in_scope() && authorization.is_none() {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "release reconciliation lacks a satisfied owner fact",
            ));
        }
        Ok(authorization)
    }
    #[cfg(test)]
    fn inject_terminalization_failure(&mut self, fault: TerminalizationFault) {
        self.terminalization_fault = Some(fault);
    }

    /// Accept one physical read child and retain its continuation for later delivery.
    ///
    /// The blocking/file path calls this method and then immediately delivers the
    /// store result. Other adapters may retain the returned identity and call
    /// `complete_read` after this method returns.
    pub fn submit_read(
        &mut self,
        request: BlockRequest,
    ) -> Result<PortableOperationSubmission, ServiceError> {
        self.state.require_reads()?;
        let (member_index, role, _) = validate_request(
            &self.topology,
            &self.members,
            request,
            self.maximum_transfer(),
        )?;
        if request.op != BlockOp::Read || role != MemberRole::Data {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "read endpoint requires a data-member read request",
            ));
        }
        self.ensure_identities()?;
        let plan = split_range(
            request.range,
            self.topology.geometry(),
            self.maximum_transfer(),
        )?;
        let [range] = plan.ranges.as_slice() else {
            return Err(ServiceError::invalid(
                FailureClass::Range,
                "deferred read requires one physical child",
            ));
        };
        let token = self.reserve(request)?;
        let result = (|| {
            let child = self.admission.child(token, *range).map_err(slot_error)?;
            let store = &self.members[member_index].store;
            let identity = self
                .admission
                .accept(
                    token,
                    child,
                    store.store_id(),
                    store.incarnation(),
                    store.topology_epoch(),
                )
                .map_err(slot_error)?;
            let pending = PendingOperation {
                request,
                child,
                identity,
            };
            let index = usize::try_from(token.index)
                .map_err(|_| slot_error(dwv_store::SlotError::StaleGeneration { token }))?;
            self.pending_operations[index] = Some(pending);
            Ok(PortableOperationSubmission {
                operation: token,
                child,
                identity,
                request,
            })
        })();
        match result {
            Ok(submission) => Ok(submission),
            Err(error) => {
                Err(self.finish_error(token, request, error, ReleaseRequirement::NotApplicable))
            }
        }
    }

    /// Deliver a previously accepted read and resume normal service completion.
    pub fn complete_read(
        &mut self,
        submission: PortableOperationSubmission,
        payload: PortableReadPayload,
        delivery: StoreCompletionDelivery,
    ) -> Result<(Vec<u8>, OperationEvidence), ServiceError> {
        let index = usize::try_from(submission.operation.index)
            .map_err(|_| ServiceError::io(FailureClass::Admission, "stale read submission"))?;
        let pending = self
            .pending_operations
            .get(index)
            .and_then(|entry| *entry)
            .filter(|pending| {
                pending.child.slot == submission.operation
                    && pending.child == submission.child
                    && pending.identity == submission.identity
                    && pending.request == submission.request
            })
            .ok_or_else(|| {
                ServiceError::io(
                    FailureClass::Admission,
                    "stale or unknown read continuation",
                )
            })?;
        let expected_length = usize::try_from(pending.request.range.length)
            .map_err(|_| ServiceError::io(FailureClass::Range, "read range does not fit memory"))?;
        if Some(payload.buffer) != pending.request.buffer
            || payload.bytes.len() != expected_length
            || delivery.identity != pending.identity
        {
            return Err(ServiceError::rejected_completion(
                FailureClass::Admission,
                delivery.completion,
                "read result does not match its accepted physical work",
            )
            .with_request(pending.request));
        }
        let bytes = payload.bytes;
        let reported = delivery.completion.clone();
        self.admission.deliver(delivery).map_err(|error| {
            ServiceError::rejected_completion(
                FailureClass::Admission,
                reported.clone(),
                error.to_string(),
            )
            .with_request(pending.request)
        })?;
        let completion = CompletionEvidence {
            requested: pending.request.range,
            completed: reported.completed.clone(),
            disposition: reported.disposition.clone(),
            persistence: if reported.persistence.is_durable() {
                PersistenceClaim::HostFenceOnly
            } else {
                PersistenceClaim::VolatileOrUnknown
            },
        };
        let result = if matches!(reported.disposition, CompletionDisposition::Success) {
            let generation = self
                .recovery_generation()
                .map_err(|error| error.with_request(pending.request))?;
            let trace = empty_trace(self.topology.topology_epoch(), generation)
                .map_err(|error| error.with_request(pending.request))?;
            let release_authorization = self
                .finish(
                    pending.operation(),
                    false,
                    ReleaseRequirement::NotApplicable,
                )
                .map_err(|cleanup| self.cleanup_error(pending.request, cleanup))?;
            Ok((
                bytes,
                OperationEvidence {
                    request: pending.request,
                    completion,
                    trace,
                    release_authorization,
                },
            ))
        } else {
            let primary = ServiceError::incomplete_read(pending.request, bytes, completion);
            Err(self.finish_error(
                pending.operation(),
                pending.request,
                primary,
                ReleaseRequirement::NotApplicable,
            ))
        };
        self.clear_pending_operation(submission.operation);
        result
    }

    fn clear_pending_operation(&mut self, operation: OperationSlotToken) {
        if let Ok(index) = usize::try_from(operation.index)
            && let Some(entry) = self.pending_operations.get_mut(index)
            && entry.is_some_and(|pending| pending.operation() == operation)
        {
            *entry = None;
        }
    }

    /// dwv:req req.healthy-portable-io.healthy-reads-preserve-exact-range-evidence
    pub fn read(
        &mut self,
        request: BlockRequest,
    ) -> Result<(Vec<u8>, OperationEvidence), ServiceError> {
        self.state.require_reads()?;
        let (member_index, role, _) = validate_request(
            &self.topology,
            &self.members,
            request,
            self.maximum_transfer(),
        )?;
        if request.op != BlockOp::Read || role != MemberRole::Data {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "read endpoint requires a data-member read request",
            ));
        }
        self.ensure_identities()?;
        let plan = split_range(
            request.range,
            self.topology.geometry(),
            self.maximum_transfer(),
        )?;
        if plan.ranges.len() == 1 {
            let submission = self.submit_read(request)?;
            let length = usize::try_from(request.range.length).map_err(|_| {
                ServiceError::io(FailureClass::Range, "read range does not fit memory")
            })?;
            let mut bytes = vec![0; length];
            let completion = self.members[member_index].store.read_at(
                submission.child,
                request.range,
                &mut bytes,
            );
            return self.complete_read(
                submission,
                PortableReadPayload::new(
                    request.buffer.expect("validated read request has a buffer"),
                    bytes,
                ),
                StoreCompletionDelivery {
                    identity: submission.identity,
                    completion,
                },
            );
        }
        let token = self.reserve(request)?;
        let result = read_member(
            &mut self.members[member_index].store,
            &mut self.admission,
            request,
            token,
            &plan,
        );
        match result {
            Ok((bytes, completion)) => {
                let generation = self
                    .recovery_generation()
                    .map_err(|error| error.with_request(request))?;
                let trace = empty_trace(self.topology.topology_epoch(), generation)
                    .map_err(|error| error.with_request(request))?;
                let release_authorization = self
                    .finish(token, false, ReleaseRequirement::NotApplicable)
                    .map_err(|cleanup| self.cleanup_error(request, cleanup))?;
                Ok((
                    bytes,
                    OperationEvidence {
                        request,
                        completion,
                        trace,
                        release_authorization,
                    },
                ))
            }
            Err(error) => {
                Err(self.finish_error(token, request, error, ReleaseRequirement::NotApplicable))
            }
        }
    }

    /// dwv:req req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity
    pub fn submit_write(
        &mut self,
        request: BlockRequest,
        bytes: &[u8],
    ) -> Result<PortableWriteSubmission, ServiceError> {
        self.state.require_writes()?;
        let (member_index, role, coding_position) = validate_request(
            &self.topology,
            &self.members,
            request,
            self.maximum_transfer(),
        )?;
        if request.op != BlockOp::Write || role != MemberRole::Data {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "write endpoint requires a data-member write request",
            ));
        }
        if bytes.len() as u64 != request.range.length {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "write buffer does not exactly cover the request",
            ));
        }
        self.ensure_identities()?;
        let plan = split_range(
            request.range,
            self.topology.geometry(),
            self.maximum_transfer(),
        )?;
        let token = self.reserve(request)?;
        let index = usize::try_from(token.index)
            .map_err(|_| slot_error(dwv_store::SlotError::StaleGeneration { token }))?;
        match self.prepare_write(member_index, coding_position, request, bytes, &plan, token) {
            Ok(driver) => {
                self.write_drivers[index] = Some(driver);
                Ok(PortableWriteSubmission {
                    operation: token,
                    request,
                })
            }
            Err(error) => {
                self.state = ServiceState::Recovering;
                Err(self.finish_error(token, request, error, transaction_requirement(None)))
            }
        }
    }

    /// Grant the generic pre-basis permission. No store call occurs here.
    pub fn grant_basis_read_permission(
        &mut self,
        submission: &PortableWriteSubmission,
    ) -> Result<(), ServiceError> {
        let index = self.write_index(submission)?;
        let mut driver = self.write_drivers[index]
            .take()
            .expect("validated write driver remains retained");
        driver.basis_permission = BasisReadPermission::Granted;
        self.write_drivers[index] = Some(driver);
        Ok(())
    }

    /// Advance the retained transaction without executing physical work.
    pub fn drive_write(
        &mut self,
        submission: &PortableWriteSubmission,
    ) -> Result<PortableWriteDrive, ServiceError> {
        let index = self.write_index(submission)?;
        let mut driver = self.write_drivers[index]
            .take()
            .expect("validated write driver remains retained");
        let request = driver.request;
        let result = self.drive_write_driver(&mut driver);
        match result {
            Ok(PortableWriteDrive::Complete(evidence)) => {
                match self.release_write_driver(index, driver, evidence) {
                    Ok(evidence) => Ok(PortableWriteDrive::Complete(evidence)),
                    Err(error) => {
                        self.state = ServiceState::Recovering;
                        Err(error.with_request(request))
                    }
                }
            }
            Ok(result) => {
                self.write_drivers[index] = Some(driver);
                Ok(result)
            }
            Err(error) => {
                self.state = ServiceState::Recovering;
                let class = Self::write_failure_class(&driver);
                let error = self.fail_write_driver(&mut driver, class, error);
                self.write_drivers[index] = Some(driver);
                Err(error.with_request(request))
            }
        }
    }

    /// Tell the slot table that the executor accepted exactly this emitted
    /// work. Store execution must happen only after this method succeeds.
    pub fn accept_write_work(&mut self, work: &PortableWriteWork) -> Result<(), ServiceError> {
        let index = self.write_index(&work.submission)?;
        let mut driver = self.write_drivers[index]
            .take()
            .expect("validated write driver remains retained");
        if driver.failed {
            self.write_drivers[index] = Some(driver);
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "failed write cannot accept additional physical work",
            ));
        }
        if self.write_work_member_index(work).is_none() {
            self.write_drivers[index] = Some(driver);
            return Err(ServiceError::io(
                FailureClass::Identity,
                "physical work store identity is no longer current",
            ));
        }
        let Some(pending) = driver.work.iter_mut().find(|pending| pending.work == *work) else {
            self.write_drivers[index] = Some(driver);
            return Err(ServiceError::io(
                FailureClass::Admission,
                "physical work is stale or mismatched",
            ));
        };
        if pending.state != WriteWorkState::Emitted {
            self.write_drivers[index] = Some(driver);
            return Err(ServiceError::io(
                FailureClass::Admission,
                "physical work was not emitted exactly once",
            ));
        }
        if let Err(error) = self.admission.accept(
            work.identity.operation_id.slot,
            work.identity.operation_id,
            work.identity.store_id,
            work.identity.store_incarnation,
            work.identity.topology_epoch,
        ) {
            self.write_drivers[index] = Some(driver);
            return Err(ServiceError::io(FailureClass::Admission, error.to_string()));
        }
        pending.state = WriteWorkState::Accepted;
        self.write_drivers[index] = Some(driver);
        Ok(())
    }

    /// Deliver a checked result for previously accepted work and resume the
    /// same retained transaction machine.
    pub fn deliver_write_result(
        &mut self,
        result: PortableWriteResult,
    ) -> Result<Option<OperationEvidence>, ServiceError> {
        let index = self.write_index(&result.work.submission)?;
        let mut driver = self.write_drivers[index]
            .take()
            .expect("validated write driver remains retained");
        let request = driver.request;
        let Some(pending_index) = driver
            .work
            .iter()
            .position(|pending| pending.work == result.work)
        else {
            return self.reject_write_result(
                index,
                driver,
                result.completion,
                request,
                "physical result is stale or mismatched",
            );
        };
        let pending_state = driver.work[pending_index].state;
        if pending_state == WriteWorkState::Completed
            && driver.work[pending_index].result.as_ref() == Some(&result)
        {
            let completion = result.completion.clone();
            if let Err(error) = self.admission.deliver(StoreCompletionDelivery {
                identity: result.work.identity,
                completion: result.completion,
            }) {
                return self.reject_write_result(
                    index,
                    driver,
                    completion,
                    request,
                    error.to_string(),
                );
            }
            self.write_drivers[index] = Some(driver);
            return Ok(None);
        }
        if let Some(evidence) = driver.evidence.clone() {
            if pending_state != WriteWorkState::Completed
                || driver.work[pending_index].result.as_ref() != Some(&result)
            {
                return self.reject_write_result(
                    index,
                    driver,
                    result.completion,
                    request,
                    "physical result is stale, duplicate, or mismatched",
                );
            }
            if driver.failed {
                return self.reject_write_result(
                    index,
                    driver,
                    result.completion,
                    request,
                    "abandoned or failed write cannot report normal completion",
                );
            }
            return match self.release_write_driver(index, driver, evidence) {
                Ok(evidence) => Ok(Some(evidence)),
                Err(error) => {
                    self.state = ServiceState::Recovering;
                    Err(error.with_request(request))
                }
            };
        }
        if pending_state != WriteWorkState::Accepted {
            return self.reject_write_result(
                index,
                driver,
                result.completion,
                request,
                "physical result was not accepted or was already delivered",
            );
        }
        if result.completion.operation_id != result.work.identity.operation_id
            || result.completion.requested != result.work.range
        {
            return self.reject_write_result(
                index,
                driver,
                result.completion,
                request,
                "physical result identity does not match emitted work",
            );
        }
        if let Err(error) = result.completion.validate() {
            return self.reject_write_result(
                index,
                driver,
                result.completion,
                request,
                error.to_string(),
            );
        }
        let is_read = matches!(result.work.action, PortableWriteAction::BasisRead { .. });
        if matches!(
            result.completion.disposition,
            CompletionDisposition::Success
        ) && is_read
        {
            let Some(payload) = result.read_payload.as_ref() else {
                return self.reject_write_result(
                    index,
                    driver,
                    result.completion,
                    request,
                    "successful basis read omitted its destination payload",
                );
            };
            if payload.len() != usize::try_from(result.work.range.length).unwrap_or(usize::MAX)
                || !result
                    .completion
                    .completed
                    .covers(result.work.range)
                    .unwrap_or(false)
            {
                return self.reject_write_result(
                    index,
                    driver,
                    result.completion,
                    request,
                    "successful basis read payload does not cover its destination",
                );
            }
        } else if !is_read && result.read_payload.is_some() {
            return self.reject_write_result(
                index,
                driver,
                result.completion,
                request,
                "non-read physical result carried a read payload",
            );
        }
        if matches!(result.work.action, PortableWriteAction::Write { .. })
            && matches!(
                result.completion.disposition,
                CompletionDisposition::Success
            )
            && let Some(watermark) = result.completion.write_watermark
            && let Err(error) = self
                .admission
                .record_submitted_watermark(result.work.identity.operation_id.slot, watermark)
        {
            self.state = ServiceState::Recovering;
            return self.reject_write_result(
                index,
                driver,
                result.completion,
                request,
                error.to_string(),
            );
        }
        if let Err(error) = self.admission.deliver(StoreCompletionDelivery {
            identity: result.work.identity,
            completion: result.completion.clone(),
        }) {
            self.state = ServiceState::Recovering;
            return self.reject_write_result(
                index,
                driver,
                result.completion,
                request,
                error.to_string(),
            );
        }
        driver.work[pending_index].state = WriteWorkState::Completed;
        driver.work[pending_index].result = Some(result.clone());
        if !matches!(
            result.completion.disposition,
            CompletionDisposition::Success
        ) {
            let failure_class = if is_read {
                if matches!(
                    result.completion.disposition,
                    CompletionDisposition::Uncertain
                ) {
                    ErrorClass::ReadUncertain
                } else {
                    ErrorClass::ReadFailed
                }
            } else if matches!(result.work.action, PortableWriteAction::Flush { .. }) {
                if matches!(
                    result.completion.disposition,
                    CompletionDisposition::Uncertain
                ) {
                    ErrorClass::FenceUncertain
                } else {
                    ErrorClass::FenceIncomplete
                }
            } else if matches!(
                result.completion.disposition,
                CompletionDisposition::Uncertain
            ) {
                ErrorClass::WriteUncertain
            } else {
                ErrorClass::WriteFailed
            };
            let error = ServiceError::store_completion(
                if is_read {
                    FailureClass::StoreRead
                } else if matches!(result.work.action, PortableWriteAction::Flush { .. }) {
                    FailureClass::Fence
                } else {
                    FailureClass::StoreWrite
                },
                result.completion,
            );
            self.state = ServiceState::Recovering;
            let error = self.fail_write_driver(&mut driver, failure_class, error);
            self.write_drivers[index] = Some(driver);
            return Err(error.with_request(request));
        }
        if driver.failed {
            self.write_drivers[index] = Some(driver);
            return Ok(None);
        }
        let final_flush = driver.phase == WritePhase::Flushes
            && driver
                .work
                .iter()
                .filter(|pending| matches!(pending.work.action, PortableWriteAction::Flush { .. }))
                .all(|pending| pending.state == WriteWorkState::Completed);
        if !final_flush {
            self.write_drivers[index] = Some(driver);
            return Ok(None);
        }
        let evidence = match self.finish_write_driver(&mut driver) {
            Ok(evidence) => evidence,
            Err(error) => {
                self.state = ServiceState::Recovering;
                let class = Self::write_failure_class(&driver);
                let error = self.fail_write_driver(&mut driver, class, error);
                self.write_drivers[index] = Some(driver);
                return Err(error.with_request(request));
            }
        };
        match self.release_write_driver(index, driver, evidence) {
            Ok(evidence) => Ok(Some(evidence)),
            Err(error) => {
                self.state = ServiceState::Recovering;
                Err(error.with_request(request))
            }
        }
    }

    /// Execute the same normalized work port used by external executors.
    pub fn write(
        &mut self,
        request: BlockRequest,
        bytes: &[u8],
    ) -> Result<OperationEvidence, ServiceError> {
        let submission = self.submit_write(request, bytes)?;
        let result: Result<OperationEvidence, ServiceError> = (|| {
            self.grant_basis_read_permission(&submission)?;
            loop {
                match self.drive_write(&submission)? {
                    PortableWriteDrive::Wait(wait) => {
                        return Err(ServiceError::io(
                            FailureClass::ReconciliationRequired,
                            format!("blocking write stopped at {wait:?}"),
                        ));
                    }
                    PortableWriteDrive::Complete(evidence) => return Ok(evidence),
                    PortableWriteDrive::Work(work) => {
                        self.accept_write_work(&work)?;
                        let result = self.execute_write_work(&work)?;
                        if let Some(evidence) = self.deliver_write_result(result)? {
                            return Ok(evidence);
                        }
                    }
                }
            }
        })();
        match result {
            Ok(evidence) => Ok(evidence),
            Err(error) => {
                self.state = ServiceState::Recovering;
                let error = error.with_request(request);
                match self.abandon(submission.operation) {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(ServiceError::terminalization(
                        request,
                        Some(error),
                        cleanup.with_request(request),
                    )),
                }
            }
        }
    }
    fn write_failure_class(driver: &WriteDriver) -> ErrorClass {
        match driver.machine.pending_action().map(|action| action.kind()) {
            Some(ActionKind::AcquireRange) => ErrorClass::RangeAcquisitionFailed,
            Some(ActionKind::PersistDirtyAndInvalidateIntegrity) => {
                ErrorClass::WriteRecoveryRecordLost
            }
            Some(ActionKind::ReadSet) => ErrorClass::ReadUncertain,
            Some(ActionKind::ComputeParity) => ErrorClass::ParityComputationFailed,
            Some(ActionKind::WriteSet) => ErrorClass::WriteUncertain,
            Some(ActionKind::FlushSet) => ErrorClass::FenceUncertain,
            Some(ActionKind::CommitRecoveryClean) => ErrorClass::RecoveryCleanFailed,
            Some(ActionKind::ReleaseRange) => ErrorClass::ReleaseFailed,
            None => ErrorClass::ReconciliationRequired,
        }
    }
    fn fail_write_driver(
        &mut self,
        driver: &mut WriteDriver,
        class: ErrorClass,
        error: ServiceError,
    ) -> ServiceError {
        if !driver.failed {
            let _ = driver.machine.apply(ActionResult::failure(class));
            driver.failed = true;
        }
        match self.admission.refuse_unaccepted(driver.operation) {
            Ok(()) => error,
            Err(cleanup) => ServiceError::terminalization(
                driver.request,
                Some(error.with_request(driver.request)),
                slot_error(cleanup).with_request(driver.request),
            ),
        }
    }
    fn reject_write_result(
        &mut self,
        index: usize,
        driver: WriteDriver,
        completion: StoreCompletion,
        request: BlockRequest,
        message: impl Into<String>,
    ) -> Result<Option<OperationEvidence>, ServiceError> {
        self.state = ServiceState::Recovering;
        self.write_drivers[index] = Some(driver);
        Err(
            ServiceError::rejected_completion(FailureClass::Admission, completion, message)
                .with_request(request),
        )
    }
    fn write_index(&self, submission: &PortableWriteSubmission) -> Result<usize, ServiceError> {
        let index = usize::try_from(submission.operation.index)
            .map_err(|_| ServiceError::io(FailureClass::Admission, "stale write submission"))?;
        let Some(driver) = self.write_drivers.get(index).and_then(Option::as_ref) else {
            return Err(ServiceError::io(
                FailureClass::Admission,
                "stale or unknown write continuation",
            ));
        };
        if driver.operation != submission.operation || driver.request != submission.request {
            return Err(ServiceError::io(
                FailureClass::Admission,
                "stale or mismatched write submission",
            ));
        }
        Ok(index)
    }

    fn drive_write_driver(
        &mut self,
        driver: &mut WriteDriver,
    ) -> Result<PortableWriteDrive, ServiceError> {
        if let Some(evidence) = driver.evidence.clone() {
            return Ok(PortableWriteDrive::Complete(evidence));
        }
        if driver.failed {
            let snapshot = self
                .admission
                .snapshot(driver.operation)
                .map_err(slot_error)?;
            let has_outstanding_physical_work = snapshot
                .children
                .iter()
                .any(|child| child.submission.is_some() && !child.terminal);
            return Ok(PortableWriteDrive::Wait(if has_outstanding_physical_work {
                PortableWriteWait::PhysicalResults
            } else {
                PortableWriteWait::OwnerReconciliation
            }));
        }
        if driver.basis_permission != BasisReadPermission::Granted {
            return Ok(PortableWriteDrive::Wait(
                PortableWriteWait::BasisReadPermission,
            ));
        }
        if driver
            .work
            .iter()
            .any(|pending| pending.state == WriteWorkState::Emitted)
        {
            return Ok(PortableWriteDrive::Wait(PortableWriteWait::PhysicalResults));
        }
        match driver.phase {
            WritePhase::BasisReads => {
                if let Some(pending) = driver.work.iter_mut().find(|pending| {
                    matches!(pending.work.action, PortableWriteAction::BasisRead { .. })
                        && pending.state == WriteWorkState::Planned
                }) {
                    pending.state = WriteWorkState::Emitted;
                    return Ok(PortableWriteDrive::Work(pending.work.clone()));
                }
                if driver.work.iter().any(|pending| {
                    matches!(pending.work.action, PortableWriteAction::BasisRead { .. })
                        && pending.state != WriteWorkState::Completed
                }) {
                    return Ok(PortableWriteDrive::Wait(PortableWriteWait::PhysicalResults));
                }
                self.prepare_write_actions(driver)?;
                self.drive_write_driver(driver)
            }
            WritePhase::Writes => {
                if let Some(pending) = driver.work.iter_mut().find(|pending| {
                    matches!(pending.work.action, PortableWriteAction::Write { .. })
                        && pending.state == WriteWorkState::Planned
                }) {
                    pending.state = WriteWorkState::Emitted;
                    return Ok(PortableWriteDrive::Work(pending.work.clone()));
                }
                if driver.work.iter().any(|pending| {
                    matches!(pending.work.action, PortableWriteAction::Write { .. })
                        && pending.state != WriteWorkState::Completed
                }) {
                    return Ok(PortableWriteDrive::Wait(PortableWriteWait::PhysicalResults));
                }
                self.prepare_flush_actions(driver)?;
                self.drive_write_driver(driver)
            }
            WritePhase::Flushes => {
                if let Some(pending) = driver.work.iter_mut().find(|pending| {
                    matches!(pending.work.action, PortableWriteAction::Flush { .. })
                        && pending.state == WriteWorkState::Planned
                }) {
                    pending.state = WriteWorkState::Emitted;
                    return Ok(PortableWriteDrive::Work(pending.work.clone()));
                }
                if driver.work.iter().any(|pending| {
                    matches!(pending.work.action, PortableWriteAction::Flush { .. })
                        && pending.state != WriteWorkState::Completed
                }) {
                    return Ok(PortableWriteDrive::Wait(PortableWriteWait::PhysicalResults));
                }
                let evidence = self.finish_write_driver(driver)?;
                Ok(PortableWriteDrive::Complete(evidence))
            }
        }
    }
    fn release_write_driver(
        &mut self,
        index: usize,
        mut driver: WriteDriver,
        mut evidence: OperationEvidence,
    ) -> Result<OperationEvidence, ServiceError> {
        if driver.evidence.is_some()
            && let Some(authorization) = self.release_authorization(driver.operation).cloned()
        {
            evidence.release_authorization = Some(authorization);
            return match self.release_after_reclaim(driver.operation) {
                Ok(()) => {
                    self.write_drivers[index] = None;
                    Ok(evidence)
                }
                Err(cleanup) => {
                    let error = self.cleanup_error(driver.request, cleanup);
                    driver.evidence = Some(evidence);
                    self.write_drivers[index] = Some(driver);
                    Err(error)
                }
            };
        }
        self.set_basis_conformance(driver.operation, BasisConformance::Consumed);
        match self
            .finish(
                driver.operation,
                false,
                transaction_requirement(Some(&evidence.trace)),
            )
            .map_err(|cleanup| self.cleanup_error(driver.request, cleanup))
        {
            Ok(release_authorization) => {
                evidence.release_authorization = release_authorization;
                self.write_drivers[index] = None;
                Ok(evidence)
            }
            Err(error) => {
                let class = Self::write_failure_class(&driver);
                let error = self.fail_write_driver(&mut driver, class, error);
                driver.evidence = Some(evidence);
                self.write_drivers[index] = Some(driver);
                Err(error)
            }
        }
    }
    fn write_work_member_index(&self, work: &PortableWriteWork) -> Option<usize> {
        self.members.iter().position(|member| {
            member.store_id == work.identity.store_id
                && member.store.store_id() == work.identity.store_id
                && member.store.incarnation() == work.identity.store_incarnation
                && member.store.topology_epoch() == work.identity.topology_epoch
        })
    }
    fn execute_write_work(
        &mut self,
        work: &PortableWriteWork,
    ) -> Result<PortableWriteResult, ServiceError> {
        let Some(member_index) = self.write_work_member_index(work) else {
            return Err(ServiceError::io(
                FailureClass::Identity,
                "physical work store identity is no longer current",
            ));
        };
        let member = &mut self.members[member_index];
        let mut read_payload = None;
        let completion = match &work.action {
            PortableWriteAction::BasisRead { .. } => {
                let length = usize::try_from(work.range.length).map_err(|_| {
                    ServiceError::io(FailureClass::Range, "read range is too large")
                })?;
                let mut destination = vec![0; length];
                let completion =
                    member
                        .store
                        .read_at(work.identity.operation_id, work.range, &mut destination);
                read_payload = Some(destination);
                completion
            }
            PortableWriteAction::Write { payload, intent } => member.store.write_at(
                work.identity.operation_id,
                work.range,
                payload.as_ref(),
                *intent,
            ),
            PortableWriteAction::Flush { through } => {
                member.store.flush(work.identity.operation_id, *through)
            }
        };
        Ok(PortableWriteResult::new(
            work.clone(),
            completion,
            read_payload,
        ))
    }
    pub fn flush(&mut self, request: BlockRequest) -> Result<OperationEvidence, ServiceError> {
        self.state.require_reads()?;
        let (_, role, _) = validate_request(
            &self.topology,
            &self.members,
            request,
            self.maximum_transfer(),
        )?;
        if request.op != BlockOp::Flush || role != MemberRole::Data {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "flush endpoint requires a data-member flush request",
            ));
        }
        self.ensure_identities()?;
        let token = self.reserve(request)?;
        let result = (|| {
            let flush_ranges = vec![ByteRange::empty(); self.members.len()];
            let flush_children = self
                .admission
                .children(token, &flush_ranges)
                .map_err(slot_error)?;
            let mut incomplete = None;
            for (member, child) in self.members.iter_mut().zip(&flush_children) {
                let through = member
                    .store
                    .highest_accepted_watermark()
                    .unwrap_or(StoreWriteWatermark(0));
                let identity = self
                    .admission
                    .accept(
                        token,
                        *child,
                        member.store.store_id(),
                        member.store.incarnation(),
                        member.store.topology_epoch(),
                    )
                    .map_err(slot_error)?;
                let completion = member.store.flush(*child, through);
                let reported = completion.clone();
                self.admission
                    .deliver(StoreCompletionDelivery {
                        identity,
                        completion,
                    })
                    .map_err(|error| {
                        ServiceError::rejected_completion(
                            FailureClass::Admission,
                            reported.clone(),
                            error.to_string(),
                        )
                    })?;
                if incomplete.is_none()
                    && (!matches!(reported.disposition, CompletionDisposition::Success)
                        || !reported.persistence.is_durable())
                {
                    incomplete = Some(reported);
                }
            }
            if let Some(completion) = incomplete {
                return Err(ServiceError::store_completion(
                    FailureClass::Fence,
                    completion,
                ));
            }
            Ok(OperationEvidence {
                request,
                completion: CompletionEvidence {
                    requested: ByteRange::empty(),
                    completed: CompletedRangeSet::empty(),
                    disposition: CompletionDisposition::Success,
                    persistence: PersistenceClaim::HostFenceOnly,
                },
                trace: empty_trace(self.topology.topology_epoch(), self.recovery_generation()?)?,
                release_authorization: None,
            })
        })();
        match result {
            Ok(mut evidence) => {
                let release_authorization = self
                    .finish(token, false, ReleaseRequirement::NotApplicable)
                    .map_err(|cleanup| self.cleanup_error(request, cleanup))?;
                evidence.release_authorization = release_authorization;
                Ok(evidence)
            }
            Err(error) => {
                if matches!(
                    &error,
                    ServiceError::Io {
                        completion: Some(_),
                        ..
                    }
                ) {
                    self.state = ServiceState::Recovering;
                }
                Err(self.finish_error(token, request, error, ReleaseRequirement::NotApplicable))
            }
        }
    }

    pub fn abandon(&mut self, token: OperationSlotToken) -> Result<(), ServiceError> {
        if let Ok(index) = usize::try_from(token.index)
            && let Some(existing) = self.write_drivers.get(index).and_then(Option::as_ref)
            && existing.operation == token
        {
            let mut driver = self.write_drivers[index]
                .take()
                .expect("matching write driver remains retained");
            if let Err(error) = driver.machine.abandon()
                && !driver.machine.is_terminal()
            {
                self.write_drivers[index] = Some(driver);
                return Err(ServiceError::io(
                    FailureClass::ReconciliationRequired,
                    error.to_string(),
                ));
            }
            driver.failed = true;
            self.write_drivers[index] = Some(driver);
            // Every protected write has already persisted its write-recovery
            // record before the driver is retained. Abandonment therefore
            // requires owner reconciliation before normal admission resumes.
            self.state = ServiceState::Recovering;
        }
        self.admission
            .refuse_unaccepted(token)
            .map_err(slot_error)?;
        self.admission.abandon(token).map_err(slot_error)
    }

    fn prepare_write(
        &mut self,
        member_index: usize,
        coding_position: CodingPosition,
        request: BlockRequest,
        bytes: &[u8],
        plan: &crate::range::RangePlan,
        token: OperationSlotToken,
    ) -> Result<WriteDriver, ServiceError> {
        let generation = self.recovery_generation()?;
        if generation != self.checksums.recovery_generation {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "checksum authority generation is stale",
            ));
        }
        let parity_assignment = self
            .topology
            .assignments()
            .iter()
            .find(|assignment| assignment.role() == MemberRole::Parity)
            .ok_or_else(|| {
                ServiceError::invalid(FailureClass::Identity, "parity assignment is missing")
            })?;
        let parity_index = member_index_for_assignment(&self.members, parity_assignment)?;
        let region_member_index = u32::from(coding_position.0);
        let regions =
            dirty_regions_for_range(region_member_index, request.range, DIRTY_REGION_BYTES)
                .map_err(|error| ServiceError::io(FailureClass::Range, error.to_string()))?;
        let checksum_extents = self.checksum_extents_for(coding_position, request.range)?;
        let stores = vec![
            self.members[member_index].store_id,
            self.members[parity_index].store_id,
        ];
        let mut parity_ranges = Vec::with_capacity(plan.ranges.len() * 2);
        for range in &plan.ranges {
            let region = dirty_regions_for_range(region_member_index, *range, DIRTY_REGION_BYTES)
                .map_err(|error| ServiceError::io(FailureClass::Range, error.to_string()))?
                .into_iter()
                .next()
                .ok_or_else(|| ServiceError::io(FailureClass::Range, "missing dirty region"))?;
            parity_ranges.extend([
                ParityRange::new(region, self.members[member_index].store_id, *range),
                ParityRange::new(region, self.members[parity_index].store_id, *range),
            ]);
        }
        let mut tx_plan = TransactionPlan::new(
            self.topology.topology_epoch(),
            generation,
            request.ordering.fence_domain,
        )
        .with_write_recovery_record(WriteRecoveryRecordRequirement::CommitRequired)
        .with_ranges(parity_ranges)
        .with_dirty_regions(regions.clone())
        .with_checksum_extents(checksum_extents.clone())
        .with_reads(
            plan.ranges
                .iter()
                .flat_map(|range| {
                    [
                        PlannedRead::new(self.members[member_index].store_id, *range),
                        PlannedRead::new(self.members[parity_index].store_id, *range),
                    ]
                })
                .collect(),
        )
        .with_parity(ParityComputationPlan::new(
            self.topology.topology_epoch(),
            request.range,
            1,
        ))
        .with_writes(
            plan.ranges
                .iter()
                .flat_map(|range| {
                    [
                        PlannedWrite::new(self.members[member_index].store_id, *range),
                        PlannedWrite::new(self.members[parity_index].store_id, *range),
                    ]
                })
                .collect(),
        )
        .with_stores(stores);
        tx_plan.limits = TransactionLimits::default();
        let mut machine = TransactionMachine::new(tx_plan)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        self.set_release_scope(token, ReleaseScope::InScope);
        let target = InvalidationTarget::new(regions.clone(), checksum_extents.clone());
        let write_recovery_record = self
            .checksums
            .invalidate_with_write_recovery_record(&mut self.recovery, target)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        machine
            .apply(ActionResult::WriteRecoveryRecordDurableWithEvidence(
                write_recovery_record.clone(),
            ))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;

        let child_ranges = plan
            .ranges
            .iter()
            .flat_map(|range| [*range, *range, *range, *range])
            .chain([ByteRange::empty(), ByteRange::empty()])
            .collect::<Vec<_>>();
        let children = self
            .admission
            .children(token, &child_ranges)
            .map_err(slot_error)?;
        let mut basis_reads = Vec::with_capacity(plan.ranges.len());
        let mut work = Vec::with_capacity(plan.ranges.len() * 2);
        let mut data_write_children = Vec::with_capacity(plan.ranges.len());
        let mut parity_write_children = Vec::with_capacity(plan.ranges.len());
        let submission = PortableWriteSubmission {
            operation: token,
            request,
        };
        for (index, range) in plan.ranges.iter().enumerate() {
            let chunk = &children[index * 4..index * 4 + 4];
            let read_data = self.make_write_work(
                &submission,
                chunk[0],
                member_index,
                *range,
                PortableWriteAction::BasisRead {
                    destination: chunk[0],
                },
            );
            let read_parity = self.make_write_work(
                &submission,
                chunk[1],
                parity_index,
                *range,
                PortableWriteAction::BasisRead {
                    destination: chunk[1],
                },
            );
            basis_reads.push([work.len(), work.len() + 1]);
            work.extend([
                PendingWriteWork {
                    work: read_data,
                    state: WriteWorkState::Planned,
                    result: None,
                },
                PendingWriteWork {
                    work: read_parity,
                    state: WriteWorkState::Planned,
                    result: None,
                },
            ]);
            data_write_children.push(chunk[2]);
            parity_write_children.push(chunk[3]);
        }
        let flush_children = [children[children.len() - 2], children[children.len() - 1]];
        let write_intent = if request.durability == DurabilityIntent::Ordinary {
            WriteIntent::Ordinary
        } else {
            WriteIntent::Preflush
        };
        Ok(WriteDriver {
            operation: token,
            request,
            payload: bytes.to_vec(),
            machine,
            member_index,
            parity_index,
            coding_position,
            ranges: plan.ranges.clone(),
            basis_reads,
            data_write_children,
            parity_write_children,
            flush_children,
            write_intent,
            basis_permission: BasisReadPermission::Pending,
            phase: WritePhase::BasisReads,
            finalization: WriteFinalizationPhase::Pending,
            write_watermarks: None,
            certificate: None,
            recovery_committed: None,
            work,
            regions,
            checksum_extents,
            write_recovery_record,
            failed: false,
            evidence: None,
        })
    }
    fn make_write_work(
        &self,
        submission: &PortableWriteSubmission,
        child: ChildOperationId,
        member_index: usize,
        range: ByteRange,
        action: PortableWriteAction,
    ) -> PortableWriteWork {
        PortableWriteWork {
            submission: *submission,
            identity: StoreSubmissionIdentity::new(
                child,
                self.members[member_index].store.store_id(),
                self.members[member_index].store.incarnation(),
                self.members[member_index].store.topology_epoch(),
            ),
            range,
            action,
        }
    }

    fn prepare_write_actions(&self, driver: &mut WriteDriver) -> Result<(), ServiceError> {
        let codec_geometry = CodecGeometry::new(
            vec![
                self.topology.geometry().protected_length();
                usize::from(self.topology.profile().data_slots())
            ],
            self.topology.geometry().parity_length(),
        )
        .map_err(|error| ServiceError::io(FailureClass::Range, error.to_string()))?;
        let mut computed = Vec::with_capacity(driver.ranges.len());
        let mut byte_cursor = 0_usize;
        for (index, range) in driver.ranges.iter().enumerate() {
            let old_data = driver.work[driver.basis_reads[index][0]]
                .result
                .as_ref()
                .and_then(|result| result.read_payload.clone())
                .ok_or_else(|| {
                    ServiceError::io(FailureClass::StoreRead, "basis data is missing")
                })?;
            let old_parity = driver.work[driver.basis_reads[index][1]]
                .result
                .as_ref()
                .and_then(|result| result.read_payload.clone())
                .ok_or_else(|| {
                    ServiceError::io(FailureClass::StoreRead, "basis parity is missing")
                })?;
            let length = usize::try_from(range.length)
                .map_err(|_| ServiceError::io(FailureClass::Range, "range does not fit memory"))?;
            let new_data = driver
                .payload
                .get(byte_cursor..byte_cursor + length)
                .ok_or_else(|| {
                    ServiceError::io(FailureClass::Range, "write payload is too short")
                })?;
            let new_parity = update_parity(
                &codec_geometry,
                *range,
                usize::from(driver.coding_position.0),
                &old_data,
                new_data,
                &old_parity,
            )?;
            computed.push((*range, new_data.to_vec(), new_parity));
            byte_cursor += length;
        }
        driver
            .machine
            .apply(ActionResult::ReadSetComplete(SemanticIoResult::complete()))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        driver
            .machine
            .apply(ActionResult::ParityComputed(ComputationResult::complete()))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let submission = PortableWriteSubmission {
            operation: driver.operation,
            request: driver.request,
        };
        for (index, (range, data, parity)) in computed.into_iter().enumerate() {
            driver.work.push(PendingWriteWork {
                work: self.make_write_work(
                    &submission,
                    driver.data_write_children[index],
                    driver.member_index,
                    range,
                    PortableWriteAction::Write {
                        payload: Arc::from(data.into_boxed_slice()),
                        intent: driver.write_intent,
                    },
                ),
                state: WriteWorkState::Planned,
                result: None,
            });
            driver.work.push(PendingWriteWork {
                work: self.make_write_work(
                    &submission,
                    driver.parity_write_children[index],
                    driver.parity_index,
                    range,
                    PortableWriteAction::Write {
                        payload: Arc::from(parity.into_boxed_slice()),
                        intent: driver.write_intent,
                    },
                ),
                state: WriteWorkState::Planned,
                result: None,
            });
        }
        driver.phase = WritePhase::Writes;
        Ok(())
    }

    fn prepare_flush_actions(&self, driver: &mut WriteDriver) -> Result<(), ServiceError> {
        let data = driver
            .work
            .iter()
            .filter(|pending| {
                driver
                    .data_write_children
                    .contains(&pending.work.identity.operation_id)
            })
            .filter_map(|pending| {
                pending
                    .result
                    .as_ref()
                    .and_then(|result| result.completion.write_watermark)
                    .map(|watermark| (pending.work.identity, watermark))
            })
            .max_by_key(|(_, watermark)| watermark.0)
            .ok_or_else(|| ServiceError::io(FailureClass::Fence, "data write lacks watermark"))?;
        let parity = driver
            .work
            .iter()
            .filter(|pending| {
                driver
                    .parity_write_children
                    .contains(&pending.work.identity.operation_id)
            })
            .filter_map(|pending| {
                pending
                    .result
                    .as_ref()
                    .and_then(|result| result.completion.write_watermark)
                    .map(|watermark| (pending.work.identity, watermark))
            })
            .max_by_key(|(_, watermark)| watermark.0)
            .ok_or_else(|| ServiceError::io(FailureClass::Fence, "parity write lacks watermark"))?;
        driver.write_watermarks = Some([data, parity]);
        let submission = PortableWriteSubmission {
            operation: driver.operation,
            request: driver.request,
        };
        driver.work.extend([
            PendingWriteWork {
                work: self.make_write_work(
                    &submission,
                    driver.flush_children[0],
                    driver.member_index,
                    ByteRange::empty(),
                    PortableWriteAction::Flush { through: data.1 },
                ),
                state: WriteWorkState::Planned,
                result: None,
            },
            PendingWriteWork {
                work: self.make_write_work(
                    &submission,
                    driver.flush_children[1],
                    driver.parity_index,
                    ByteRange::empty(),
                    PortableWriteAction::Flush { through: parity.1 },
                ),
                state: WriteWorkState::Planned,
                result: None,
            },
        ]);
        driver.phase = WritePhase::Flushes;
        Ok(())
    }

    fn finish_write_driver(
        &mut self,
        driver: &mut WriteDriver,
    ) -> Result<OperationEvidence, ServiceError> {
        if let Some(evidence) = driver.evidence.clone() {
            return Ok(evidence);
        }
        if driver.finalization == WriteFinalizationPhase::Pending {
            let [
                (data_identity, data_watermark),
                (parity_identity, parity_watermark),
            ] = driver.write_watermarks.ok_or_else(|| {
                ServiceError::io(FailureClass::Fence, "write watermarks are missing")
            })?;
            driver
                .machine
                .set_write_watermarks(vec![
                    StoreWatermark::for_incarnation(
                        data_identity.store_id,
                        data_identity.store_incarnation,
                        data_watermark,
                    ),
                    StoreWatermark::for_incarnation(
                        parity_identity.store_id,
                        parity_identity.store_incarnation,
                        parity_watermark,
                    ),
                ])
                .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
            driver.finalization = WriteFinalizationPhase::WatermarksSet;
        }
        if driver.finalization == WriteFinalizationPhase::WatermarksSet {
            driver
                .machine
                .apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))
                .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
            driver.finalization = WriteFinalizationPhase::WritesCompleted;
        }
        if driver.finalization == WriteFinalizationPhase::WritesCompleted {
            let certificate = if let Some(certificate) = driver.certificate.clone() {
                certificate
            } else {
                let data_fence = driver
                    .work
                    .iter()
                    .find(|pending| pending.work.identity.operation_id == driver.flush_children[0])
                    .and_then(|pending| pending.result.as_ref())
                    .map(|result| result.completion.persistence)
                    .ok_or_else(|| {
                        ServiceError::io(FailureClass::Fence, "data flush is missing")
                    })?;
                let parity_fence = driver
                    .work
                    .iter()
                    .find(|pending| pending.work.identity.operation_id == driver.flush_children[1])
                    .and_then(|pending| pending.result.as_ref())
                    .map(|result| result.completion.persistence)
                    .ok_or_else(|| {
                        ServiceError::io(FailureClass::Fence, "parity flush is missing")
                    })?;
                let mut store_fences = Vec::new();
                for evidence in [data_fence, parity_fence] {
                    let PersistenceEvidence::DurableByFence { fence } = evidence else {
                        return Err(ServiceError::io(
                            FailureClass::Fence,
                            "flush returned volatile or unknown evidence",
                        ));
                    };
                    store_fences.push(fence);
                }
                let certificate = driver.checksum_extents.iter().copied().fold(
                    FenceCertificate::new(
                        self.topology.topology_epoch(),
                        driver.request.ordering.fence_domain,
                        store_fences,
                        driver
                            .regions
                            .iter()
                            .copied()
                            .map(|region| {
                                (region, driver.write_recovery_record.committed_generation)
                            })
                            .collect(),
                    ),
                    |certificate, extent| {
                        certificate.with_integrity_extent(
                            extent,
                            driver.write_recovery_record.committed_generation,
                        )
                    },
                );
                driver.certificate = Some(certificate.clone());
                certificate
            };
            driver
                .machine
                .apply(ActionResult::FlushSetComplete(
                    TransactionPersistenceEvidence::durable(certificate),
                ))
                .map_err(|error| ServiceError::io(FailureClass::Fence, error.to_string()))?;
            if driver.machine.is_terminal() {
                driver.failed = true;
                return Err(ServiceError::io(
                    FailureClass::ReconciliationRequired,
                    "transaction machine entered reconciliation during flush finalization",
                ));
            }
            driver.finalization = WriteFinalizationPhase::FlushesCompleted;
        }
        if driver.finalization == WriteFinalizationPhase::FlushesCompleted {
            let committed = if let Some(committed) = driver.recovery_committed {
                committed
            } else {
                let current = self
                    .recovery
                    .load_assembly_snapshot()
                    .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
                if current.generation != driver.write_recovery_record.committed_generation {
                    return Err(ServiceError::io(
                        FailureClass::Recovery,
                        "recovery generation changed during write",
                    ));
                }
                let mut recovery_clean =
                    RecoveryTxn::new(current.generation, self.topology.topology_epoch());
                recovery_clean.push(RecoveryMutation::RecordDataParityFence {
                    fence: driver
                        .certificate
                        .clone()
                        .expect("flush finalization retains its certificate"),
                });
                for region in driver.regions.iter().copied() {
                    recovery_clean.push(RecoveryMutation::MarkRegionClean {
                        region,
                        through_generation: driver.write_recovery_record.committed_generation,
                    });
                }
                let committed = self
                    .recovery
                    .commit_durable(recovery_clean)
                    .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
                self.checksums.recovery_generation = committed;
                driver.recovery_committed = Some(committed);
                committed
            };
            driver
                .machine
                .apply(ActionResult::RecoveryCleanCommitted(
                    CommittedRecoveryGeneration::new(committed, self.topology.topology_epoch()),
                ))
                .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
            driver.finalization = WriteFinalizationPhase::RecoveryCommitted;
        }
        if driver.finalization == WriteFinalizationPhase::RecoveryCommitted {
            driver
                .machine
                .apply(ActionResult::RangeReleased)
                .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
            driver.finalization = WriteFinalizationPhase::RangeReleased;
        }
        if driver.finalization == WriteFinalizationPhase::RangeReleased {
            let evidence = OperationEvidence {
                request: driver.request,
                completion: CompletionEvidence {
                    requested: driver.request.range,
                    completed: CompletedRangeSet::new(vec![driver.request.range])
                        .expect("validated write range is a completion range"),
                    disposition: CompletionDisposition::Success,
                    persistence: PersistenceClaim::HostFenceOnly,
                },
                trace: driver.machine.trace().clone(),
                release_authorization: None,
            };
            driver.evidence = Some(evidence);
            driver.finalization = WriteFinalizationPhase::EvidenceReady;
        }
        driver
            .evidence
            .clone()
            .ok_or_else(|| ServiceError::io(FailureClass::Recovery, "write finalization stopped"))
    }

    fn reserve(&mut self, request: BlockRequest) -> Result<OperationSlotToken, ServiceError> {
        let token = self.admission.reserve(request).map_err(slot_error)?;
        self.set_release_scope(token, ReleaseScope::Outside);
        self.clear_basis_observation(token);
        Ok(token)
    }

    fn finish(
        &mut self,
        token: OperationSlotToken,
        uncertain: bool,
        requirement: ReleaseRequirement,
    ) -> Result<Option<ReleaseAuthorization>, ServiceError> {
        if uncertain {
            #[cfg(test)]
            if self.take_terminalization_fault(TerminalizationFault::Snapshot) {
                return Err(ServiceError::io(
                    FailureClass::ReconciliationRequired,
                    "injected terminalization snapshot failure",
                ));
            }
            let snapshot = self.admission.snapshot(token).map_err(slot_error)?;
            self.finish_uncertain(token, snapshot)
                .map_err(FinishCleanupError::into_service_error)?;
            match self.release_scope(token) {
                ReleaseScope::Outside => self.release(token)?,
                ReleaseScope::InScope => {}
                ReleaseScope::Unknown => {
                    return Err(ServiceError::io(
                        FailureClass::ReconciliationRequired,
                        "operation lacks a release applicability observation",
                    ));
                }
            }
            Ok(None)
        } else {
            let operation_effect = self.operation_effect_observation(token)?;
            let observation =
                self.current_release_observation(token, operation_effect.effect, requirement);
            match self.release_scope(token) {
                ReleaseScope::Outside => self.reclaim_with_authorization(&observation),
                ReleaseScope::InScope => self.reconcile_release_authorization(observation),
                ReleaseScope::Unknown => Err(ServiceError::io(
                    FailureClass::ReconciliationRequired,
                    "operation lacks a release applicability observation",
                )),
            }
        }
    }

    fn finish_after_error(
        &mut self,
        token: OperationSlotToken,
        requirement: ReleaseRequirement,
    ) -> Result<Option<ReleaseAuthorization>, FinishCleanupError> {
        if matches!(self.release_scope(token), ReleaseScope::InScope)
            && matches!(requirement, ReleaseRequirement::TransactionUnresolved)
        {
            // The transaction owner has not supplied its release fact. Keep
            // the exact slot and primary error; this is not cleanup failure.
            return Err(FinishCleanupError::Outstanding);
        }
        #[cfg(test)]
        if self.take_terminalization_fault(TerminalizationFault::Snapshot) {
            return Err(FinishCleanupError::Failed(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "injected terminalization snapshot failure",
            )));
        }
        let snapshot = self.admission.snapshot(token).map_err(slot_error)?;
        let operation_effect = operation_effect_from_snapshot(token, &snapshot);
        if operation_effect.effect == OperationEffect::Unresolved {
            self.finish_uncertain(token, snapshot)?;
            match self.release_scope(token) {
                ReleaseScope::Outside => self.release(token)?,
                ReleaseScope::InScope => {}
                ReleaseScope::Unknown => {
                    return Err(ServiceError::io(
                        FailureClass::ReconciliationRequired,
                        "operation lacks a release applicability observation",
                    )
                    .into());
                }
            }
            Ok(None)
        } else {
            let observation =
                self.current_release_observation(token, operation_effect.effect, requirement);
            match self.release_scope(token) {
                ReleaseScope::Outside => Ok(self.reclaim_with_authorization(&observation)?),
                ReleaseScope::InScope => Ok(self.reconcile_release_authorization(observation)?),
                ReleaseScope::Unknown => Err(ServiceError::io(
                    FailureClass::ReconciliationRequired,
                    "operation lacks a release applicability observation",
                )
                .into()),
            }
        }
    }

    fn finish_uncertain(
        &mut self,
        token: OperationSlotToken,
        snapshot: SlotSnapshot,
    ) -> Result<(), FinishCleanupError> {
        #[cfg(test)]
        if self.take_terminalization_fault(TerminalizationFault::Reconciliation) {
            return Err(FinishCleanupError::Failed(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "injected terminalization reconciliation failure",
            )));
        }
        if snapshot.children.iter().any(|child| !child.terminal) {
            return Err(FinishCleanupError::Outstanding);
        }
        self.mark_reclaimable(token, ReconciliationOutcome::UncertainRetained)
            .map_err(FinishCleanupError::Failed)
    }

    fn mark_reclaimable(
        &mut self,
        token: OperationSlotToken,
        reconciliation: ReconciliationOutcome,
    ) -> Result<(), ServiceError> {
        self.admission
            .record_reconciliation(token, reconciliation)
            .map_err(slot_error)
    }

    /// Establish the exact lifecycle authorization before physical cleanup.
    ///
    /// Coded claim removal consumes this fact independently. The caller owns
    /// the later physical slot cleanup and must retain the exact token.
    pub(crate) fn establish_release_authorization(
        &mut self,
        observation: &ReleaseReconciliation,
    ) -> Result<Option<ReleaseAuthorization>, ServiceError> {
        let token = observation.operation;
        if !self.release_scope(token).is_in_scope() {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "coded release requires an in-scope lifecycle operation",
            ));
        }
        if matches!(observation.operation_effect, OperationEffect::Unresolved)
            || matches!(observation.recovery, RecoveryReconciliation::Unresolved)
        {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "release authorization lacks authoritative owner evidence",
            ));
        }
        if !observation.requirement.is_satisfied() {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "release authorization lacks transaction release evidence",
            ));
        }
        if !observation.basis.is_conformant() {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "release authorization lacks basis conformance",
            ));
        }
        self.set_basis_conformance(token, observation.basis);
        // Recovery observations do not certify missing physical children.
        // The slot owner must observe every child terminal before reclaiming.
        self.mark_reclaimable(token, ReconciliationOutcome::Durable)?;
        let snapshot = self.admission.snapshot(token).map_err(slot_error)?;
        let authorization = self.authorization_candidate(token, &snapshot, observation);
        if let Some(authorization) = authorization.as_ref() {
            self.remember_release_authorization(authorization.clone());
        }
        Ok(authorization)
    }

    fn release(&mut self, token: OperationSlotToken) -> Result<(), ServiceError> {
        let result = self.admission.release(token).map_err(slot_error);
        if result.is_ok() {
            self.clear_release_scope(token);
            self.clear_basis_observation(token);
            if let Ok(index) = usize::try_from(token.index)
                && let Some(entry) = self.operation_readiness.get_mut(index)
                && entry.is_some_and(|(operation, _)| operation == token)
            {
                *entry = None;
            }
            if let Ok(index) = usize::try_from(token.index)
                && let Some(driver) = self.write_drivers.get_mut(index)
                && driver
                    .as_ref()
                    .is_some_and(|driver| driver.operation == token)
            {
                *driver = None;
            }
        }
        result
    }

    fn release_after_reclaim(&mut self, token: OperationSlotToken) -> Result<(), ServiceError> {
        #[cfg(test)]
        if self.take_terminalization_fault(TerminalizationFault::Reclaim) {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "injected terminalization reclaim failure",
            ));
        }
        self.release(token)
    }
    fn reclaim_with_authorization(
        &mut self,
        observation: &ReleaseReconciliation,
    ) -> Result<Option<ReleaseAuthorization>, ServiceError> {
        let token = observation.operation;
        let scope = self.release_scope(token);
        if matches!(scope, ReleaseScope::Unknown) {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "operation lacks a release applicability observation",
            ));
        }
        if matches!(scope, ReleaseScope::Outside) {
            self.mark_reclaimable(token, ReconciliationOutcome::Durable)?;
            self.release_after_reclaim(token)?;
            return Ok(None);
        }
        let authorization = self.establish_release_authorization(observation)?;
        if authorization.is_none() {
            return Ok(None);
        }
        self.release_after_reclaim(token).map(|()| authorization)
    }

    fn remember_release_authorization(&mut self, authorization: ReleaseAuthorization) {
        let Ok(index) = usize::try_from(authorization.operation.index) else {
            return;
        };
        if let Some(retained) = self.release_authorizations.get_mut(index) {
            *retained = Some(authorization);
        } else {
            debug_assert!(false, "authorization index exceeds bounded slot table");
        }
    }

    fn authorization_candidate(
        &self,
        token: OperationSlotToken,
        snapshot: &SlotSnapshot,
        observation: &ReleaseReconciliation,
    ) -> Option<ReleaseAuthorization> {
        let reconciliation = snapshot.reconciliation?;
        if snapshot.token != token
            || observation.operation != token
            || !self.release_scope(token).is_in_scope()
            || snapshot.state != SlotState::Reclaimable
            || snapshot.drain_state == dwv_store::DrainState::Required
            || (needs_uncertain_reconciliation(snapshot)
                && !matches!(
                    observation.operation_effect,
                    OperationEffect::AuthoritativelyReconciled
                ))
            || reconciliation != ReconciliationOutcome::Durable
            || !observation.requirement.is_satisfied()
            || !matches!(
                observation.operation_effect,
                OperationEffect::Terminal | OperationEffect::AuthoritativelyReconciled
            )
            || !matches!(observation.recovery, RecoveryReconciliation::Authoritative)
            || !observation.basis.is_conformant()
        {
            return None;
        }
        Some(ReleaseAuthorization { operation: token })
    }

    fn operation_effect_observation(
        &self,
        operation: OperationSlotToken,
    ) -> Result<OperationEffectObservation, ServiceError> {
        let snapshot = self.admission.snapshot(operation).map_err(slot_error)?;
        Ok(operation_effect_from_snapshot(operation, &snapshot))
    }

    fn current_release_observation(
        &self,
        operation: OperationSlotToken,
        operation_effect: OperationEffect,
        requirement: ReleaseRequirement,
    ) -> ReleaseReconciliation {
        ReleaseReconciliation {
            operation,
            operation_effect,
            requirement,
            recovery: self.recovery_reconciliation_observation(),
            basis: self.basis_conformance_observation(operation),
        }
    }

    /// The recovery/topology/checksum-generation fact is separate from the
    /// typed basis-conformance observation consumed by lifecycle release.
    fn recovery_topology_generation_coherent(&self) -> bool {
        self.recovery
            .load_assembly_snapshot()
            .is_ok_and(|snapshot| {
                snapshot.topology_epoch == self.topology.topology_epoch()
                    && snapshot.generation == self.checksums.recovery_generation
            })
    }

    fn recovery_reconciliation_observation(&self) -> RecoveryReconciliation {
        if self.recovery.verify_integrity() == RecoveryStoreHealth::Healthy
            && self.recovery_topology_generation_coherent()
        {
            RecoveryReconciliation::Authoritative
        } else {
            RecoveryReconciliation::Unresolved
        }
    }

    fn release_scope(&self, operation: OperationSlotToken) -> ReleaseScope {
        let Ok(index) = usize::try_from(operation.index) else {
            return ReleaseScope::Unknown;
        };
        self.release_scopes
            .get(index)
            .and_then(|scope| *scope)
            .filter(|(token, _)| *token == operation)
            .map_or(ReleaseScope::Unknown, |(_, scope)| scope)
    }

    fn set_release_scope(&mut self, operation: OperationSlotToken, scope: ReleaseScope) {
        let Ok(index) = usize::try_from(operation.index) else {
            return;
        };
        if let Some(slot) = self.release_scopes.get_mut(index) {
            *slot = Some((operation, scope));
        }
    }

    fn clear_release_scope(&mut self, operation: OperationSlotToken) {
        let Ok(index) = usize::try_from(operation.index) else {
            return;
        };
        if let Some(slot) = self.release_scopes.get_mut(index)
            && slot.is_some_and(|(token, _)| token == operation)
        {
            *slot = None;
        }
    }

    fn basis_conformance_observation(&self, operation: OperationSlotToken) -> BasisConformance {
        let Ok(index) = usize::try_from(operation.index) else {
            return BasisConformance::Unresolved;
        };
        self.basis_observations
            .get(index)
            .and_then(|observation| *observation)
            .filter(|(token, _)| *token == operation)
            .map_or(BasisConformance::Unresolved, |(_, observation)| observation)
    }

    fn set_basis_conformance(
        &mut self,
        operation: OperationSlotToken,
        observation: BasisConformance,
    ) {
        if !observation.is_conformant() {
            return;
        }
        let Ok(index) = usize::try_from(operation.index) else {
            return;
        };
        if let Some(slot) = self.basis_observations.get_mut(index) {
            *slot = Some((operation, observation));
        }
    }

    fn clear_basis_observation(&mut self, operation: OperationSlotToken) {
        let Ok(index) = usize::try_from(operation.index) else {
            return;
        };
        if let Some(slot) = self.basis_observations.get_mut(index)
            && slot.is_some_and(|(token, _)| token == operation)
        {
            *slot = None;
        }
    }

    fn finish_error(
        &mut self,
        token: OperationSlotToken,
        request: BlockRequest,
        primary: ServiceError,
        requirement: ReleaseRequirement,
    ) -> ServiceError {
        let primary = primary.with_request(request);
        if let Err(error) = self.admission.refuse_unaccepted(token) {
            self.state = ServiceState::Recovering;
            return ServiceError::terminalization(request, Some(primary), slot_error(error));
        }
        match self.finish_after_error(token, requirement) {
            Ok(_) => primary,
            Err(FinishCleanupError::Outstanding) => {
                self.state = ServiceState::Recovering;
                primary
            }
            Err(FinishCleanupError::Failed(cleanup)) => {
                self.state = ServiceState::Recovering;
                ServiceError::terminalization(request, Some(primary), cleanup.with_request(request))
            }
        }
    }

    fn cleanup_error(&mut self, request: BlockRequest, cleanup: ServiceError) -> ServiceError {
        self.state = ServiceState::Recovering;
        ServiceError::terminalization(request, None, cleanup.with_request(request))
    }

    fn maximum_transfer(&self) -> u64 {
        self.config
            .maximum_transfer
            .unwrap_or(self.topology.geometry().protected_length())
    }

    fn recovery_generation(&self) -> Result<RecoveryGeneration, ServiceError> {
        self.recovery
            .load_assembly_snapshot()
            .map(|snapshot| snapshot.generation)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))
    }

    fn checksum_extents_for(
        &self,
        data_position: CodingPosition,
        range: ByteRange,
    ) -> Result<Vec<IntegrityExtentId>, ServiceError> {
        let parity_position = self
            .topology
            .assignments()
            .iter()
            .find(|assignment| assignment.role() == MemberRole::Parity)
            .map(TopologyAssignment::coding_position)
            .ok_or_else(|| {
                ServiceError::invalid(FailureClass::Identity, "parity assignment is missing")
            })?;
        let per_member = checksum_extent_count(self.topology.geometry().protected_length())?;
        let first_extent = range.offset / BLAKE3_256_PROFILE.extent_size;
        let last_extent = (range.end() - 1) / BLAKE3_256_PROFILE.extent_size;
        let mut extents = Vec::new();
        for position in [data_position, parity_position] {
            let member_base = u64::from(position.0)
                .checked_mul(per_member)
                .ok_or_else(|| {
                    ServiceError::io(FailureClass::Range, "checksum extent ID overflowed")
                })?;
            for extent_index in first_extent..=last_extent {
                extents.push(IntegrityExtentId(
                    member_base.checked_add(extent_index).ok_or_else(|| {
                        ServiceError::io(FailureClass::Range, "checksum extent ID overflowed")
                    })?,
                ));
            }
        }
        Ok(extents)
    }

    fn ensure_identities(&self) -> Result<(), ServiceError> {
        for member in &self.members {
            let current = member
                .store
                .current_identity_observations()
                .map_err(|error| ServiceError::io(FailureClass::Identity, error.to_string()))?;
            match member.store.identity_observations().compare(&current) {
                IdentityComparison::Unchanged => {}
                IdentityComparison::Changed | IdentityComparison::Ambiguous => {
                    return Err(ServiceError::invalid(
                        FailureClass::Identity,
                        "member identity changed or became ambiguous",
                    ));
                }
            }
        }
        Ok(())
    }
}
fn transaction_release_complete(trace: Option<&dwv_transaction_ref::Trace>) -> bool {
    let Some(trace) = trace else {
        return false;
    };
    !trace
        .events()
        .iter()
        .any(|event| matches!(event, TraceEvent::ReconciliationRequired { .. }))
        && trace.events().iter().any(|event| {
            matches!(
                event,
                TraceEvent::ResultApplied {
                    action: ActionKind::ReleaseRange,
                    result: ResultKind::RangeReleased,

                    ..
                }
            )
        })
}

fn transaction_requirement(trace: Option<&dwv_transaction_ref::Trace>) -> ReleaseRequirement {
    if transaction_release_complete(trace) {
        ReleaseRequirement::TransactionSatisfied
    } else {
        ReleaseRequirement::TransactionUnresolved
    }
}

fn operation_effect_from_snapshot(
    operation: OperationSlotToken,
    snapshot: &SlotSnapshot,
) -> OperationEffectObservation {
    debug_assert_eq!(snapshot.token, operation);
    let effect = if needs_uncertain_reconciliation(snapshot) {
        OperationEffect::Unresolved
    } else {
        OperationEffect::Terminal
    };
    OperationEffectObservation { operation, effect }
}

fn needs_uncertain_reconciliation(snapshot: &SlotSnapshot) -> bool {
    snapshot.children.iter().any(|child| {
        !child.terminal
            || child.completion.as_ref().is_some_and(|completion| {
                completion.disposition == CompletionDisposition::Uncertain
            })
    })
}

#[cfg(test)]
impl<S: RandomAccessStore, R: RecoveryStateStore> HealthyPortableService<S, R> {
    fn take_terminalization_fault(&mut self, fault: TerminalizationFault) -> bool {
        if self.terminalization_fault == Some(fault) {
            self.terminalization_fault = None;
            true
        } else {
            false
        }
    }
}

fn checksum_extent_count(protected_length: u64) -> Result<u64, ServiceError> {
    let count = protected_length.div_ceil(BLAKE3_256_PROFILE.extent_size);
    if count == 0 || count > 1_048_576 {
        return Err(ServiceError::io(
            FailureClass::Range,
            "checksum extent count exceeds the bounded profile",
        ));
    }
    Ok(count)
}

fn checksum_authority(
    topology: &TopologySnapshot,
    generation: RecoveryGeneration,
) -> Result<ChecksumAuthority, ServiceError> {
    let per_member = checksum_extent_count(topology.geometry().protected_length())?;
    let mut authority =
        ChecksumAuthority::new(topology.topology_epoch(), ChecksumSetGeneration::INITIAL);
    authority.recovery_generation = generation;
    for assignment in topology.assignments() {
        let target = match assignment.role() {
            MemberRole::Data => ChecksumTarget::data(assignment.slot_id()),
            MemberRole::Parity => ChecksumTarget::parity(assignment.coding_position()),
        };
        let first_id = IntegrityExtentId(
            u64::from(assignment.coding_position().0)
                .checked_mul(per_member)
                .ok_or_else(|| {
                    ServiceError::io(FailureClass::Range, "checksum extent ID overflowed")
                })?,
        );
        let extents = ChecksumExtent::partition(
            target,
            topology.geometry().protected_length(),
            BLAKE3_256_PROFILE.extent_size,
            first_id,
        )
        .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        for extent in extents {
            authority.register(ChecksumRecord::absent(
                extent,
                BLAKE3_256_PROFILE.id,
                ChecksumSetGeneration::INITIAL,
            ));
        }
    }
    Ok(authority)
}

fn persisted_checksum_authority(
    snapshot: &RecoverySnapshot,
) -> Result<ChecksumAuthority, ServiceError> {
    let baseline = snapshot.checksum_baseline.as_ref().ok_or_else(|| {
        ServiceError::io(
            FailureClass::Recovery,
            "complete checksum baseline has no descriptor",
        )
    })?;
    let mut authority = ChecksumAuthority::new(baseline.topology_epoch, baseline.set_generation);
    authority.recovery_generation = snapshot.generation;
    for extent in baseline.expected_extents.iter().copied() {
        let persisted = snapshot
            .integrity_records
            .iter()
            .find(|record| record.extent == extent.id)
            .ok_or_else(|| {
                ServiceError::io(
                    FailureClass::Recovery,
                    "complete checksum baseline is missing an extent record",
                )
            })?;
        let IntegrityState::Valid {
            content_generation,
            durable_fence,
            digest,
            ..
        } = &persisted.state
        else {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "complete checksum baseline contains non-valid evidence",
            ));
        };
        let digest: [u8; 32] = digest.as_slice().try_into().map_err(|_| {
            ServiceError::io(
                FailureClass::Recovery,
                "complete checksum baseline contains a malformed digest",
            )
        })?;
        authority.register(ChecksumRecord::valid(
            extent,
            baseline.profile.id,
            baseline.set_generation,
            ContentGeneration(*content_generation),
            digest,
            ChecksumPersistenceEvidence {
                fence: *durable_fence,
                topology_epoch: baseline.topology_epoch,
                recovery_generation: *content_generation,
            },
        ));
    }
    Ok(authority)
}

/// dwv:req req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments
fn validate_assembly<S: RandomAccessStore>(
    topology: &TopologySnapshot,
    members: &[MemberBinding<S>],
) -> Result<(), ServiceError> {
    topology
        .validate()
        .map_err(|error| ServiceError::invalid(FailureClass::InvalidRequest, error.to_string()))?;
    if topology.profile().parity_slots() != 1 || members.len() != topology.assignments().len() {
        return Err(ServiceError::invalid(
            FailureClass::Capability,
            "healthy portable service requires one binding for every single-parity assignment",
        ));
    }
    let geometry = topology.geometry();
    let mut identities = Vec::with_capacity(members.len());
    for (index, member) in members.iter().enumerate() {
        let assignment = topology
            .assignment_for_slot(member.slot_id)
            .ok_or_else(|| {
                ServiceError::invalid(
                    FailureClass::Identity,
                    "member binding has no topology assignment",
                )
            })?;
        if member.role != assignment.role()
            || member.coding_position != assignment.coding_position()
            || member.assignment_instance != assignment.assignment_instance()
            || member.assignment_generation != assignment.assignment_generation()
            || member.topology_epoch != topology.topology_epoch()
        {
            return Err(ServiceError::invalid(
                FailureClass::Identity,
                "member binding does not match its captured topology assignment",
            ));
        }
        if member.store.store_id() != member.store_id
            || member.store.topology_epoch() != member.topology_epoch
        {
            return Err(ServiceError::invalid(
                FailureClass::Identity,
                "member binding store identity does not match the opened store",
            ));
        }
        if members[..index]
            .iter()
            .any(|prior| prior.slot_id == member.slot_id || prior.store_id == member.store_id)
        {
            return Err(ServiceError::invalid(
                FailureClass::Identity,
                "member binding duplicates a slot or store identity",
            ));
        }
        let capabilities = member.store.capabilities();
        if capabilities.logical_length != dwv_store::Evidence::Known(geometry.protected_length())
            || capabilities.logical_block_size
                != dwv_store::Evidence::Known(geometry.logical_block_size())
            || !capabilities.read.is_supported()
            || !capabilities.write.is_supported()
            || !capabilities.durable_flush.is_supported()
        {
            return Err(ServiceError::invalid(
                FailureClass::Capability,
                "member capabilities are incomplete for portable healthy I/O",
            ));
        }
        identities.push(member.store.identity_observations());
    }
    for assignment in topology.assignments() {
        if members
            .iter()
            .filter(|member| member.slot_id == assignment.slot_id())
            .count()
            != 1
        {
            return Err(ServiceError::invalid(
                FailureClass::Identity,
                "topology assignment does not have exactly one member binding",
            ));
        }
    }
    for (index, identity) in identities.iter().enumerate() {
        for other in identities.iter().skip(index + 1) {
            if identity.compare(other) != dwv_store::IdentityComparison::Changed {
                return Err(ServiceError::invalid(
                    FailureClass::Alias,
                    "member bindings alias or have ambiguous identity",
                ));
            }
        }
    }
    Ok(())
}

fn validate_request<S: RandomAccessStore>(
    topology: &TopologySnapshot,
    members: &[MemberBinding<S>],
    request: BlockRequest,
    maximum_transfer: u64,
) -> Result<(usize, MemberRole, CodingPosition), ServiceError> {
    let capabilities = dwv_core::FrontendCapabilities {
        max_transfer: Some(maximum_transfer),
        supports_preflush: false,
        supports_fua: false,
        supports_write_zeroes: false,
        supports_discard: false,
    };
    request.validate(&capabilities).map_err(|error| {
        let class = match error {
            dwv_core::RequestError::UnsupportedOperation(_)
            | dwv_core::RequestError::UnsupportedPreflush
            | dwv_core::RequestError::UnsupportedFua => FailureClass::Capability,
            _ => FailureClass::InvalidRequest,
        };
        ServiceError::invalid(class, error.to_string())
    })?;
    if request.topology_epoch != topology.topology_epoch() {
        return Err(ServiceError::invalid(
            FailureClass::StaleTopology,
            "request topology epoch is stale",
        ));
    }
    let assignment = topology
        .assignment_for_slot(request.slot_id)
        .ok_or_else(|| {
            ServiceError::invalid(
                FailureClass::InvalidRequest,
                "request target slot is absent from the captured topology",
            )
        })?;
    let member_index = member_index_for_assignment(members, assignment)?;
    Ok((
        member_index,
        assignment.role(),
        assignment.coding_position(),
    ))
}

fn member_index_for_assignment<S: RandomAccessStore>(
    members: &[MemberBinding<S>],
    assignment: &TopologyAssignment,
) -> Result<usize, ServiceError> {
    members
        .iter()
        .position(|member| {
            member.slot_id == assignment.slot_id()
                && member.role == assignment.role()
                && member.coding_position == assignment.coding_position()
                && member.assignment_instance == assignment.assignment_instance()
                && member.assignment_generation == assignment.assignment_generation()
        })
        .ok_or_else(|| {
            ServiceError::invalid(
                FailureClass::Identity,
                "captured topology assignment has no exact member binding",
            )
        })
}

fn empty_trace(
    epoch: TopologyEpoch,
    generation: RecoveryGeneration,
) -> Result<dwv_transaction_ref::Trace, ServiceError> {
    let plan = TransactionPlan::new(epoch, generation, FenceDomain(0))
        .with_ranges(vec![ParityRange::new(
            RegionId(0),
            StoreId(1),
            ByteRange::new(0, 1).expect("constant range"),
        )])
        .with_reads(vec![PlannedRead::new(
            StoreId(1),
            ByteRange::new(0, 1).expect("constant range"),
        )])
        .with_writes(vec![PlannedWrite::new(
            StoreId(1),
            ByteRange::new(0, 1).expect("constant range"),
        )])
        .with_stores(vec![StoreId(1)])
        .with_watermarks(vec![StoreWatermark::new(
            StoreId(1),
            StoreWriteWatermark(1),
        )]);
    TransactionMachine::new(plan)
        .map(|machine| machine.trace().clone())
        .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))
}

#[cfg(test)]
mod tests {
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
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const LENGTH: u64 = 4096;
    const BLOCK: u32 = 512;
    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

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
            Self {
                id,
                epoch,
                bytes: vec![0; LENGTH as usize],
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
            StoreCapabilities::portable_demo(LENGTH, BLOCK, LENGTH, CapabilityEvidenceId(self.id.0))
        }

        fn length(&self) -> u64 {
            LENGTH
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
                destination[half as usize..completed as usize + half as usize].copy_from_slice(
                    &self.bytes[start + half as usize..start + range.length as usize],
                );
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
        let profile = CodingProfile::new(2, 1).unwrap();
        let geometry = ProtectedGeometry::new(LENGTH, BLOCK).unwrap();
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
                MemberBinding::new(
                    assignment,
                    epoch,
                    store_id,
                    FakeStore::new(store_id, epoch, read).with_effects(write, flush),
                )
            })
            .collect();
        HealthyPortableService::open(topology, members, MemoryRecoveryStore::new(epoch), config)
            .unwrap()
    }

    struct DeterministicWriteHarness<S: RandomAccessStore, R: RecoveryStateStore> {
        service: HealthyPortableService<S, R>,
        submission: PortableWriteSubmission,
        accepted: Vec<PortableWriteWork>,
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
                self.service.accept_write_work(work)?;
                self.accepted.push(work.clone());
            }
            Ok(turn)
        }

        fn execute_accepted(&mut self, index: usize) -> Result<(), ServiceError> {
            let work = self.accepted.swap_remove(index);
            self.ready.push(self.service.execute_write_work(&work)?);
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
        let wrong_delivery = StoreCompletionDelivery::new(
            StoreId(2),
            StoreIncarnationId(1),
            epoch,
            completion.clone(),
        );
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
            .unwrap();
        assert_eq!(bytes.len(), BLOCK as usize);
        assert_eq!(evidence.completion.completed.as_slice(), &[range]);
        assert_eq!(service.admission_usage().operation_slots, 0);
        assert_eq!(service.admission_usage().backend_submissions, 0);
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
        service.accept_write_work(&work).unwrap();
        let result = service.execute_write_work(&work).unwrap();
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
        let evidence = service.write(request, &bytes).unwrap();
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
        service
            .write(
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
                MemberBinding::new(
                    assignment,
                    topology.topology_epoch(),
                    store_id,
                    open(store_id.0),
                )
            })
            .collect()
    }

    fn reopened_bindings(
        root: &Path,
        topology: &TopologySnapshot,
    ) -> Vec<MemberBinding<FileStore>> {
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
        let service = HealthyPortableService::open(
            topology,
            members,
            MemoryRecoveryStore::new(epoch),
            config,
        )
        .unwrap();
        (root, service)
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
                maintenance_checkpoints: Vec::new(),
                metadata_loss_audit: Some(dwv_recovery::MetadataLossAudit {
                    matrix_version: dwv_recovery::METADATA_LOSS_MATRIX_VERSION,
                    lineage_id: topology.array_id(),
                    case: dwv_recovery::MetadataLossCase::ChecksumEvidenceUnavailable,
                    action: dwv_recovery::MetadataLossAction::RequireDataAuthoritativeRebaseline,
                    verification:
                        dwv_recovery::MetadataLossVerification::ExplicitDataAuthoritativeRebaseline,
                    baseline:
                        dwv_recovery::BaselineDisposition::NewParityAndChecksumBaselineRequired,
                    source_health: RecoveryStoreHealth::Missing,
                    topology_epoch: epoch,
                }),
                rebuilds: Vec::new(),
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
                        digest: vec![0; 32],
                        verified_at: RecoveryGeneration(1),
                    },
                });
        }
        let mut certificate =
            FenceCertificate::new(epoch, FenceDomain(1), store_fences, Vec::new());
        for extent in extents {
            certificate = certificate.with_integrity_extent(extent.id, RecoveryGeneration::ZERO);
        }
        manifest.snapshot.fences.push(certificate);
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
        let evidence = service.write(write_request, &bytes).unwrap();
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
        service
            .write(
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
        service
            .write(
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
        service
            .write(
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
            service
                .write(
                    request(
                        RequestId(100 + iteration),
                        TopologyEpoch(4),
                        slot,
                        BlockOp::Write,
                        range,
                        DurabilityIntent::Ordinary,
                    ),
                    &bytes.clone(),
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
        service
            .write(
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
            topology,
            members,
            MemoryRecoveryStore::new(epoch),
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
        assert_eq!(authorization.operation, older);
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
        let error = service
            .write(admitted, &[0x68; BLOCK as usize])
            .unwrap_err();
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
        let first = service
            .write(
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
        assert_eq!(service.release_authorization(first.operation), Some(&first));

        let second = service
            .write(
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
        assert_ne!(first.operation, second.operation);
        assert!(service.release_authorization(first.operation).is_none());
        assert_eq!(
            service.release_authorization(second.operation),
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
        assert_eq!(authorization.operation, in_scope);
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
    fn failed_child_admission_retains_primary_error_until_owner_release() {
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
        assert_eq!(service.admission_usage().operation_slots, 1);
        assert_eq!(service.admission_usage().backend_submissions, 0);
        assert_eq!(service.state(), ServiceState::Recovering);
        let token = OperationSlotToken::new(0, 1);
        assert_eq!(service.release_scope(token), ReleaseScope::InScope);
        assert!(
            service
                .admission
                .snapshot(token)
                .unwrap()
                .children
                .is_empty()
        );
        let authorization = service
            .reconcile_release_authorization(authoritative_release(token))
            .unwrap()
            .expect("an acquired release scope does not require physical children");
        assert_eq!(authorization.operation, token);
        assert_eq!(service.release_authorization(token), Some(&authorization));
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
            topology,
            members,
            MemoryRecoveryStore::new(epoch),
            ServiceConfig::default(),
        )
        .unwrap();
        let range = ByteRange::new(0, BLOCK as u64).unwrap();
        let bytes = vec![0x8d; BLOCK as usize];
        service
            .write(
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
            MemberBinding::new(
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
            reordered,
            members,
            MemoryRecoveryStore::new(epoch),
            ServiceConfig::default(),
        )
        .unwrap();
        assert_eq!(
            service.publication_identity().unwrap(),
            original_publication
        );
        let range = ByteRange::new(0, BLOCK as u64).unwrap();
        service
            .write(
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
        let mut recovery = MemoryRecoveryStore::new(TopologyEpoch(4));
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
        let topology = topology(TopologyEpoch(4));
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
            publication_identity(&topology, &identities(3, IdentitySourceKind::Path, false))
                .unwrap();
        assert_eq!(
            admitted_a,
            publication_identity(&topology, &identities(3, IdentitySourceKind::Path, true))
                .unwrap()
        );
        assert_ne!(
            admitted_a,
            publication_identity(&topology, &identities(4, IdentitySourceKind::Path, false))
                .unwrap()
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
            publication_identity(&reordered, &identities(3, IdentitySourceKind::Path, false),)
                .unwrap()
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
    #[path = "coded_range_clean_connect.rs"]
    mod coded_range_clean_connect;
    #[path = "lifecycle_connect.rs"]
    mod lifecycle_connect;
}
