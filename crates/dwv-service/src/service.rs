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
    AssignmentGeneration, AssignmentInstanceId, BlockOp, BlockRequest, ByteRange, CodedUnitId,
    CodingPosition, DurabilityIntent, MemberRole, SlotId, TopologyAssignment, TopologyEpoch,
    TopologySnapshot,
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
/// The retained operation is parked without changing its transaction or slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortableWriteWait {
    CodedRangeContention,
    CaptureBlocked,
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
    transaction_plan: TransactionPlan,
    member_index: usize,
    parity_index: usize,
    coding_position: CodingPosition,
    coded_claim: CodedClaimInput,
    coded_admitted: bool,
    coded_effect_permitted: bool,
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
    write_recovery_record: Option<WriteRecoveryRecordEvidence>,
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
    fn release_coded_claim_if_authorized(
        &mut self,
        authorization: &ReleaseAuthorization,
    ) -> Result<(), ServiceError> {
        if self
            .coded_authority
            .operation_phase(authorization.operation)
            .is_some()
        {
            self.coded_release_claim(authorization.operation, authorization)?;
        }
        Ok(())
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
            self.release_coded_claim_if_authorized(&authorization)?;
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
                let bounded_admission_refusal = matches!(
                    &error,
                    ServiceError::Io {
                        class: FailureClass::Admission,
                        ..
                    } | ServiceError::Invalid {
                        class: FailureClass::Admission,
                        ..
                    }
                );
                if !bounded_admission_refusal {
                    self.state = ServiceState::Recovering;
                }
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
    /// The synchronous facade returns explicit retryable contention for a
    /// pre-admission coded wait without degrading the service. Callers that
    /// need to park and resume across an external wait use the retained
    /// submit/drive boundary; waits after admission remain conservative
    /// owner-reconciliation outcomes.
    pub fn write(
        &mut self,
        request: BlockRequest,
        bytes: &[u8],
    ) -> Result<OperationEvidence, ServiceError> {
        let submission = self.submit_write(request, bytes)?;
        let mut cancelled_contention = false;
        let result: Result<OperationEvidence, ServiceError> = (|| {
            self.grant_basis_read_permission(&submission)?;
            loop {
                match self.drive_write(&submission)? {
                    PortableWriteDrive::Wait(PortableWriteWait::CodedRangeContention) => {
                        self.cancel_unstarted_write(submission.operation)?;
                        cancelled_contention = true;
                        return Err(ServiceError::io(
                            FailureClass::CodedContention,
                            "blocking write encountered coded contention; use the retained write driver to resume",
                        ));
                    }
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
            Err(error) if cancelled_contention => Err(error.with_request(request)),
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
    fn cancel_unstarted_write(
        &mut self,
        operation: OperationSlotToken,
    ) -> Result<(), ServiceError> {
        let index = usize::try_from(operation.index)
            .map_err(|_| ServiceError::io(FailureClass::Admission, "stale write submission"))?;
        let Some(driver) = self.write_drivers.get_mut(index).and_then(Option::take) else {
            return Err(ServiceError::io(
                FailureClass::Admission,
                "stale or unknown write continuation",
            ));
        };
        let transaction_unstarted = driver.machine.range_guard().is_none()
            && driver
                .machine
                .pending_action()
                .is_some_and(|action| action.kind() == ActionKind::AcquireRange)
            && self.release_scope(operation) == ReleaseScope::Outside;
        let unstarted = driver.operation == operation
            && !driver.coded_admitted
            && transaction_unstarted
            && driver.write_recovery_record.is_none()
            && driver
                .work
                .iter()
                .all(|pending| pending.state == WriteWorkState::Planned);
        if !unstarted {
            self.write_drivers[index] = Some(driver);
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "blocking write wait crossed its cancellation boundary",
            ));
        }
        self.admission
            .refuse_unaccepted(operation)
            .map_err(slot_error)?;
        self.admission
            .record_reconciliation(operation, ReconciliationOutcome::Durable)
            .map_err(slot_error)?;
        self.release(operation)
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
        if !driver.coded_admitted {
            match self.coded_admit(driver.operation, driver.coded_claim.clone())? {
                CodedAdmissionOutcome::Contended => {
                    return Ok(PortableWriteDrive::Wait(
                        PortableWriteWait::CodedRangeContention,
                    ));
                }
                CodedAdmissionOutcome::Admitted => {
                    driver.coded_admitted = true;
                    driver
                        .machine
                        .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
                        .map_err(|error| {
                            ServiceError::io(FailureClass::Recovery, error.to_string())
                        })?;
                    self.set_release_scope(driver.operation, ReleaseScope::InScope);
                }
            }
        }
        self.ensure_write_recovery_record(driver)?;
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
                if !driver.coded_effect_permitted
                    && matches!(
                        self.coded_permit_effect(driver.operation)?,
                        CodedEffectOutcome::BlockedByCapture
                    )
                {
                    return Ok(PortableWriteDrive::Wait(PortableWriteWait::CaptureBlocked));
                }
                driver.coded_effect_permitted = true;
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
            // The recovery record is persisted at the first admitted driver
            // step. Abandonment before that boundary has no irreversible
            // effect and therefore has no record to reconcile.
            self.state = ServiceState::Recovering;
        }
        self.admission
            .refuse_unaccepted(token)
            .map_err(slot_error)?;
        self.admission.abandon(token).map_err(slot_error)
    }
    /// Map the validated captured topology's logical codeword blocks to the
    /// opaque coded-unit keys used by the shared authority. Member identity is
    /// intentionally absent: data members sharing a codeword contend.
    fn coded_claim_for_plan(
        &self,
        plan: &crate::range::RangePlan,
    ) -> Result<CodedClaimInput, ServiceError> {
        let block = u64::from(self.topology.geometry().logical_block_size());
        let mut units = Vec::new();
        for range in &plan.ranges {
            let mut offset = range.offset;
            let end = range.end();
            while offset < end {
                units.push(CodedUnitId(offset / block));
                offset += block;
            }
        }
        Ok(CodedClaimInput::complete(units))
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
        let coded_claim = self.coded_claim_for_plan(plan)?;
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
        let transaction_plan = tx_plan.clone();
        let machine = TransactionMachine::new(tx_plan)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        // The transaction's abstract range acquisition is fulfilled only after
        // complete coded admission succeeds. A contended operation therefore
        // remains pre-transaction and can be safely discarded by a blocking
        // caller without inventing a release observation.
        // The recovery record is committed only after the complete coded claim
        // is admitted. A contended operation must not advance recovery state
        // before it can ever issue dependent basis I/O.

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
            transaction_plan,
            member_index,
            parity_index,
            coding_position,
            coded_claim,
            coded_admitted: false,
            coded_effect_permitted: false,
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
            write_recovery_record: None,
            failed: false,
            evidence: None,
        })
    }
    fn ensure_write_recovery_record(
        &mut self,
        driver: &mut WriteDriver,
    ) -> Result<(), ServiceError> {
        if driver.write_recovery_record.is_some() {
            return Ok(());
        }
        let generation = self.recovery_generation()?;
        if driver.transaction_plan.recovery_generation != generation {
            let mut transaction_plan = driver.transaction_plan.clone();
            transaction_plan.recovery_generation = generation;
            let mut machine = TransactionMachine::new(transaction_plan.clone())
                .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
            machine
                .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
                .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
            driver.transaction_plan = transaction_plan;
            driver.machine = machine;
        }
        let target =
            InvalidationTarget::new(driver.regions.clone(), driver.checksum_extents.clone());
        let record = self
            .checksums
            .invalidate_with_write_recovery_record(&mut self.recovery, target)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        driver.write_recovery_record = Some(record.clone());
        driver
            .machine
            .apply(ActionResult::WriteRecoveryRecordDurableWithEvidence(record))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))
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
        let write_recovery_record = driver.write_recovery_record.clone().ok_or_else(|| {
            ServiceError::io(FailureClass::Recovery, "write-recovery record is missing")
        })?;
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
                            .map(|region| (region, write_recovery_record.committed_generation))
                            .collect(),
                    ),
                    |certificate, extent| {
                        certificate.with_integrity_extent(
                            extent,
                            write_recovery_record.committed_generation,
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
                if current.generation < write_recovery_record.committed_generation {
                    return Err(ServiceError::io(
                        FailureClass::Recovery,
                        "recovery generation moved backwards during write",
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
                        through_generation: write_recovery_record.committed_generation,
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
            // LifecycleRelease owns this monotonic exact-generation fact.
            // Retain it before invoking downstream coded-claim removal so a
            // coded cleanup failure cannot retroactively erase authorization.
            self.remember_release_authorization(authorization.clone());
            self.release_coded_claim_if_authorized(authorization)?;
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
mod tests;
