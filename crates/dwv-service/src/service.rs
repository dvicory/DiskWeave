use crate::{
    admission::{AdmissionConfig, OperationAdmission, slot_error},
    evidence::{
        BasisConformance, CompletionEvidence, IncludedLifecycleAuthorization, OperationEffect,
        OperationEffectObservation, OperationEvidence, PersistenceClaim, RecoveryReconciliation,
        ReleaseAuthorization, ReleaseReconciliation, ReleaseRequirement, ReleaseScope,
    },
    failure::{FailureClass, ServiceError},
    lifecycle::ServiceState,
    range::split_range,
    read::read_member,
    write::update_parity,
};
use basis_observation::BasisObservationLedger;
use dwv_codec::Geometry as CodecGeometry;
#[cfg(test)]
use dwv_core::CodedUnitId;
use dwv_core::{
    AssignmentGeneration, AssignmentInstanceId, BlockOp, BlockRequest, ByteRange, CodingPosition,
    DurabilityIntent, MemberRole, SlotId, TopologyAssignment, TopologyEpoch, TopologySnapshot,
};
use dwv_lifecycle_authority::LifecycleAuthorityOwner;
use dwv_recovery::InvalidationTarget;
use dwv_recovery::{
    BLAKE3_256_PROFILE, ChecksumAuthority, ChecksumBaselineStatus, ChecksumExtent,
    ChecksumPersistenceEvidence, ChecksumRecord, ChecksumSetGeneration, ChecksumTarget,
    CodedCaptureId, CodedCaptureMembership, CodedCaptureOwnerFacts, CodedCapturePhase,
    CodedCaptureReconciliationReceipt, CodedCaptureRetentionOwner, CodedGeometryOwner,
    CodedLifecycleAuthority, CodedReopenResolution, ContentGeneration, DIRTY_REGION_BYTES,
    DurableRecoveryCommit, FenceCertificate, FenceOccurrenceId, IntegrityExtentId, IntegrityState,
    RecoveryCleanDecision, RecoveryCleanRefusalPermit, RecoveryCleanRequest,
    RecoveryCommitObservation, RecoveryError, RecoveryGeneration, RecoveryMutation,
    RecoverySnapshot, RecoveryStateStore, RecoveryStoreHealth, RecoveryTxn, RegionId,
    WriteRecoveryRecordEvidence, assess_checksum_baseline, dirty_regions_for_range,
    evaluate_recovery_clean,
};
use dwv_store::{
    ChildOperationId, CompletedRangeSet, CompletionDisposition, FenceDomain, IdentityAssessment,
    IdentityComparison, IdentityObservationSet, IdentitySourceKind, OperationSlotToken,
    PersistenceEvidence, RandomAccessStore, ReconciliationOutcome, SlotSnapshot, SlotState,
    StoreCompletion, StoreCompletionDelivery, StoreFenceRef, StoreId, StoreSubmissionIdentity,
    StoreWriteWatermark, WriteIntent,
};
use dwv_transaction_ref::{
    ActionKind, ActionResult, CodedAdmissionOutcome, CodedClaimInput, CodedClaimRelease,
    CodedEffectPermit, CommittedRecoveryGeneration, ComputationResult, ErrorClass,
    ParityComputationPlan, ParityRange, PlannedRead, PlannedWrite, RangeGuardToken, ResultKind,
    SemanticIoResult, StoreWatermark, TraceEvent, TransactionLimits, TransactionMachine,
    TransactionPersistenceEvidence, TransactionPlan, WriteRecoveryRecordRequirement,
};
use readiness::OperationReadinessLedger;
use release_authorization::ReleaseAuthorizationLedger;
use release_scope::ReleaseScopeLedger;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
type IncludedLifecycleEvidence = (IncludedLifecycleAuthorization, FenceCertificate);
type CaptureCleanEvaluation = (RecoveryCleanDecision, Vec<IncludedLifecycleEvidence>);

fn coded_capture_has_outstanding_release_consumer(
    coded_captures: &CodedLifecycleAuthority<ReleaseAuthorization, IncludedLifecycleAuthorization>,
    operation: OperationSlotToken,
) -> bool {
    coded_captures.snapshots().into_iter().any(|snapshot| {
        snapshot.membership.contains_key(&operation)
            || snapshot.pending_later_cuts.contains_key(&operation)
            || snapshot.resolved_later_cuts.contains_key(&operation)
            || snapshot.release_authorized_operations.contains(&operation)
            || snapshot.release_frontiers.contains_key(&operation)
            || snapshot.release_certificates.contains_key(&operation)
    })
}

fn has_outstanding_release_consumer(
    admission: &OperationAdmission,
    coded_captures: &CodedLifecycleAuthority<ReleaseAuthorization, IncludedLifecycleAuthorization>,
    write_drivers: &[Option<WriteDriver>],
    operation: OperationSlotToken,
) -> bool {
    admission.snapshot(operation).is_ok()
        || coded_capture_has_outstanding_release_consumer(coded_captures, operation)
        || write_drivers
            .iter()
            .flatten()
            .any(|driver| driver.operation == operation && driver.evidence.is_some())
}
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
    Permitted(CodedEffectPermit),
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
    /// Maximum durable coded captures retained at once.
    pub max_coded_captures: usize,
    /// Maximum exact-generation release authorizations retained per operation slot.
    ///
    /// The default follows the recovery export membership bound. Exhaustion is
    /// explicit backpressure; it never overwrites an older generation.
    pub max_retained_release_authorizations_per_slot: usize,
    pub fence_domain: FenceDomain,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            admission: AdmissionConfig::default(),
            maximum_transfer: None,
            max_coded_captures: 32,
            max_retained_release_authorizations_per_slot:
                dwv_recovery::RecoveryExportLimits::default().max_coded_capture_memberships,
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
    CodedCaptureCapacity,
    BasisReadPermission,
    PhysicalResults,
    OwnerReconciliation,
    CleanCaptureMembers,
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

/// Proof that the service accepted one exact emitted action before backend
/// submission. Only [`HealthyPortableService::accept_write_work`] can create it.
#[derive(Debug, Eq, PartialEq)]
pub struct AcceptedPortableWriteWork {
    work: PortableWriteWork,
}

impl AcceptedPortableWriteWork {
    pub const fn work(&self) -> &PortableWriteWork {
        &self.work
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableWriteResult {
    pub work: PortableWriteWork,
    pub completion: StoreCompletion,
    pub read_payload: Option<Vec<u8>>,
}

impl PortableWriteResult {
    pub fn new(
        accepted: AcceptedPortableWriteWork,
        completion: StoreCompletion,
        read_payload: Option<Vec<u8>>,
    ) -> Self {
        Self {
            work: accepted.work,
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
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum WriteFinalizationPhase {
    Pending,
    WatermarksSet,
    WritesCompleted,
    FlushesCompleted,
    RecoveryCommitted,
    RangeReleased,
    EvidenceReady,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CodedReleaseCommitState {
    NotAttempted,
    KnownNoncommit,
    MayHaveCommitted,
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
    coded_effect_permit: Option<CodedEffectPermit>,
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
    coded_release_commit: CodedReleaseCommitState,
    work: Vec<PendingWriteWork>,
    regions: Vec<RegionId>,
    checksum_extents: Vec<IntegrityExtentId>,
    clean_capture: Option<CodedCaptureId>,
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
    if !snapshot.coded_captures.is_empty() {
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
    write_drivers: Vec<Option<WriteDriver>>,
    operation_readiness: OperationReadinessLedger,
    checksums: ChecksumAuthority,
    admission: OperationAdmission,
    config: ServiceConfig,
    state: ServiceState,
    release_authorizations: ReleaseAuthorizationLedger,
    release_scopes: ReleaseScopeLedger,
    basis_observations: BasisObservationLedger,
    lifecycle_authority: LifecycleAuthorityOwner,
    coded_captures: CodedLifecycleAuthority<ReleaseAuthorization, IncludedLifecycleAuthorization>,
    coded_geometry: CodedGeometryOwner,
    reopened_captures: BTreeSet<CodedCaptureId>,
    #[cfg(test)]
    terminalization_fault: Option<TerminalizationFault>,
}
mod basis_observation;
mod read_ops;
mod readiness;
mod release_authorization;
mod release_scope;
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
        if config.max_retained_release_authorizations_per_slot == 0 {
            return Err(ServiceError::invalid(
                FailureClass::Admission,
                "release authorization retention capacity cannot be zero",
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
        let checksum_profile = checksums.profile_id().ok_or_else(|| {
            ServiceError::io(
                FailureClass::Recovery,
                "coded capture reopen requires one active checksum profile",
            )
        })?;
        if checksum_profile != BLAKE3_256_PROFILE.id {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "coded capture reopen requires the supported checksum profile",
            ));
        }
        let (lifecycle_authority, lifecycle_verifier) = LifecycleAuthorityOwner::new();
        let (coded_lifecycle, coded_geometry) =
            CodedLifecycleAuthority::from_snapshots_with_verifier(
                snapshot.coded_captures.clone(),
                &topology,
                generation,
                BLAKE3_256_PROFILE,
                checksums.active_set,
                DIRTY_REGION_BYTES,
                lifecycle_verifier,
            )
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let reopened_captures = snapshot
            .coded_captures
            .iter()
            .map(|capture| capture.capture)
            .collect::<BTreeSet<_>>();
        let state = if snapshot.topology_epoch != topology.topology_epoch() {
            ServiceState::Blocked(FailureClass::StaleTopology)
        } else if snapshot
            .dirty_regions
            .iter()
            .any(|region| !matches!(region.state, dwv_recovery::RegionState::Clean))
            || !reopened_captures.is_empty()
        {
            ServiceState::Recovering
        } else {
            ServiceState::Serving
        };
        Ok(Self {
            topology,
            members,
            recovery,
            write_drivers: std::iter::repeat_with(|| None)
                .take(config.admission.limits.operation_slots)
                .collect(),
            checksums,
            operation_readiness: OperationReadinessLedger::new(
                config.admission.limits.operation_slots,
            ),
            admission: OperationAdmission::new(config.admission),
            config,
            state,
            release_authorizations: ReleaseAuthorizationLedger::new(
                config.admission.limits.operation_slots,
                config.max_retained_release_authorizations_per_slot,
            ),
            release_scopes: ReleaseScopeLedger::new(config.admission.limits.operation_slots),
            basis_observations: BasisObservationLedger::new(
                config.admission.limits.operation_slots,
            ),
            lifecycle_authority,
            coded_captures: coded_lifecycle,
            coded_geometry,
            reopened_captures,
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
    pub fn checksums(&self) -> &ChecksumAuthority {
        &self.checksums
    }
    pub fn admission_usage(&self) -> dwv_store::ResourceUsage {
        self.admission.usage()
    }

    #[cfg(test)]
    pub(crate) fn coded_authority(
        &self,
    ) -> &CodedLifecycleAuthority<ReleaseAuthorization, IncludedLifecycleAuthorization> {
        &self.coded_captures
    }

    #[cfg(test)]
    pub(crate) fn coded_captures(
        &self,
    ) -> &CodedLifecycleAuthority<ReleaseAuthorization, IncludedLifecycleAuthorization> {
        &self.coded_captures
    }

    #[cfg(test)]
    pub(crate) fn operation_readiness(
        &self,
        operation: OperationSlotToken,
    ) -> Option<OperationReadiness> {
        self.operation_readiness.get(operation)
    }

    fn invalidate_coded_transition_authority(&mut self) {
        self.coded_captures.invalidate_after_uncertain_persistence();
        self.state = ServiceState::Recovering;
    }

    fn finish_coded_persistence<T>(
        &mut self,
        result: Result<T, RecoveryError>,
    ) -> Result<T, ServiceError> {
        match result {
            Ok(value) => Ok(value),
            Err(error @ RecoveryError::CommitNotDurable(RecoveryCommitObservation::Rejected)) => {
                Err(ServiceError::io(FailureClass::Recovery, error.to_string()))
            }
            Err(error) => {
                self.invalidate_coded_transition_authority();
                Err(ServiceError::io(FailureClass::Recovery, error.to_string()))
            }
        }
    }

    fn commit_coded_transition(
        &mut self,
        transaction: RecoveryTxn,
    ) -> Result<DurableRecoveryCommit, ServiceError> {
        let result = self.recovery.commit_durable_receipt(transaction);
        self.finish_coded_persistence(result)
    }

    fn finish_coded_confirmation<T, E>(
        &mut self,
        result: Result<T, E>,
        failure_class: FailureClass,
    ) -> Result<T, ServiceError>
    where
        E: std::fmt::Display,
    {
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                self.invalidate_coded_transition_authority();
                Err(ServiceError::io(failure_class, error.to_string()))
            }
        }
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
        let assignment = self
            .topology
            .assignment_for_slot(snapshot.request.slot_id)
            .ok_or_else(|| {
                ServiceError::invalid(
                    FailureClass::InvalidRequest,
                    "coded admission request slot has no current topology assignment",
                )
            })?;
        if !self.coded_geometry.validates_claim_for_mutation(
            operation,
            &snapshot.request,
            assignment,
            &claim,
        ) {
            return Err(ServiceError::io(
                FailureClass::Admission,
                "coded claim is not bound to the admitted request or the current topology and \
                 geometry owner",
            ));
        }
        let current = self
            .recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let prepared = self
            .coded_captures
            .prepare_admission(operation, snapshot.request, claim, current.generation)
            .map_err(|error| ServiceError::io(FailureClass::Admission, error.to_string()))?;
        if matches!(prepared.outcome(), CodedAdmissionOutcome::Contended) {
            self.operation_readiness.set(
                operation,
                OperationReadiness::Waiting(PendingReason::AdmissionContended),
            );
            return Ok(CodedAdmissionOutcome::Contended);
        }
        let receipt = if let Some(transition) = prepared.transition() {
            let mut txn = RecoveryTxn::new(current.generation, self.topology.topology_epoch());
            txn.push(RecoveryMutation::ApplyCodedTransition { transition });
            let receipt = self.commit_coded_transition(txn)?;
            self.checksums.recovery_generation = receipt.generation();
            Some(receipt)
        } else {
            None
        };
        let confirmation = self
            .coded_captures
            .confirm_admission(prepared, receipt.as_ref());
        let outcome = self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
        self.set_release_scope(operation, ReleaseScope::InScope);
        self.operation_readiness
            .set(operation, OperationReadiness::Runnable);
        Ok(outcome)
    }

    pub(crate) fn coded_start_capture(
        &mut self,
        capture: CodedCaptureId,
        dirty_regions: impl IntoIterator<Item = RegionId>,
        checksum_extents: impl IntoIterator<Item = IntegrityExtentId>,
    ) -> Result<(), ServiceError> {
        if self.coded_captures.capture_count() >= self.config.max_coded_captures {
            return Err(ServiceError::io(
                FailureClass::Admission,
                "coded capture capacity is exhausted",
            ));
        }
        let checksum_profile = self.checksums.profile_id().ok_or_else(|| {
            ServiceError::io(
                FailureClass::Recovery,
                "coded capture requires one active checksum profile",
            )
        })?;
        if checksum_profile != BLAKE3_256_PROFILE.id {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "coded capture requires the supported checksum profile",
            ));
        }
        let snapshot = self
            .recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let baseline_incoherent = snapshot.checksum_baseline.as_ref().is_some_and(|baseline| {
            baseline.topology_epoch != self.topology.topology_epoch()
                || baseline.profile.id != checksum_profile
                || baseline.set_generation != self.checksums.active_set
        });
        if snapshot.topology_epoch != self.topology.topology_epoch()
            || self.checksums.topology_epoch != self.topology.topology_epoch()
            || snapshot.generation != self.checksums.recovery_generation
            || baseline_incoherent
        {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "coded capture owner topology/recovery/checksum state is incoherent",
            ));
        }
        if capture.0 != snapshot.next_coded_capture_id {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "coded capture ID does not match the durable next capture ID",
            ));
        }
        let topology_epoch = self.topology.topology_epoch();
        let target = InvalidationTarget::new(
            dirty_regions.into_iter().collect(),
            checksum_extents.into_iter().collect(),
        );
        let recovery = &mut self.recovery;
        let result = self.coded_captures.start_capture(
            capture,
            snapshot.generation,
            self.checksums.active_set,
            target,
            topology_epoch,
            recovery,
        );
        if !self.coded_captures.is_usable() {
            self.state = ServiceState::Recovering;
        }
        let receipt = result
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        self.checksums.recovery_generation = receipt.generation();
        Ok(())
    }

    /// Resolve and durably retire one capture inherited from a prior process.
    ///
    /// The current recovery owner either proves exact closure or atomically
    /// invalidates the selected state while abandoning unreconstructible
    /// prior-process authority.
    pub fn resolve_reopened_coded_capture(
        &mut self,
        capture: CodedCaptureId,
        refusal: Option<RecoveryCleanRefusalPermit>,
    ) -> Result<(), ServiceError> {
        if self.state != ServiceState::Recovering
            || !self.reopened_captures.contains(&capture)
            || self.admission_usage().operation_slots != 0
        {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "coded reopen resolution requires an inherited capture and no live operations",
            ));
        }
        let current = self
            .recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let resolution = self
            .coded_captures
            .resolve_capture(&current, capture, refusal)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        match resolution {
            CodedReopenResolution::Refused(authorization) => {
                let prepared = self
                    .coded_captures
                    .prepare_refused_cleanup(authorization)
                    .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
                let mut txn = RecoveryTxn::new(current.generation, self.topology.topology_epoch());
                txn.push(RecoveryMutation::ApplyCodedTransition {
                    transition: prepared.transition(),
                });
                let receipt = self.commit_coded_transition(txn)?;
                let confirmation = self
                    .coded_captures
                    .confirm_refused_cleanup(prepared, &receipt);
                self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
                self.checksums.recovery_generation = receipt.generation();
            }
            CodedReopenResolution::CleanKnown(authorization) => {
                let prepared = self
                    .coded_captures
                    .prepare_clean_known_cleanup(authorization)
                    .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
                let mut txn = RecoveryTxn::new(current.generation, self.topology.topology_epoch());
                txn.push(RecoveryMutation::ApplyCodedTransition {
                    transition: prepared.transition(),
                });
                let receipt = self.commit_coded_transition(txn)?;
                let confirmation = self
                    .coded_captures
                    .confirm_clean_known_cleanup(prepared, &receipt);
                self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
                self.checksums.recovery_generation = receipt.generation();
            }
            CodedReopenResolution::Abandon(authorization) => {
                let prepared = self
                    .coded_captures
                    .prepare_inherited_capture_abandonment(authorization)
                    .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
                let mut txn = RecoveryTxn::new(current.generation, self.topology.topology_epoch());
                txn.push(RecoveryMutation::ApplyCodedTransition {
                    transition: prepared.transition(),
                });
                let receipt = self.commit_coded_transition(txn)?;
                let confirmation = self
                    .coded_captures
                    .confirm_inherited_capture_abandonment(prepared, &receipt);
                self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
                self.checksums.recovery_generation = receipt.generation();
            }
        }
        self.reopened_captures.remove(&capture);
        Ok(())
    }

    /// Persist one recovery-inspection-issued resolution for an uncertain CLEAN commit.
    pub fn reconcile_coded_clean_commit(
        &mut self,
        receipt: CodedCaptureReconciliationReceipt,
    ) -> Result<(), ServiceError> {
        let capture = receipt.capture();
        self.apply_coded_reconciliation(capture, receipt)
    }

    /// Persist one recovery-inspection-issued resolution for an uncertain later cut.
    pub fn reconcile_coded_later_cut(
        &mut self,
        receipt: CodedCaptureReconciliationReceipt,
    ) -> Result<(), ServiceError> {
        let capture = receipt.capture();
        self.apply_coded_reconciliation(capture, receipt)
    }

    fn apply_coded_reconciliation(
        &mut self,
        capture: CodedCaptureId,
        reconciliation: CodedCaptureReconciliationReceipt,
    ) -> Result<(), ServiceError> {
        if reconciliation.capture() != capture {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "coded reconciliation capture does not match",
            ));
        }
        let current = self
            .recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let prepared = self
            .coded_captures
            .prepare_reconciliation(reconciliation, current.generation)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let mut txn = RecoveryTxn::new(current.generation, self.topology.topology_epoch());
        txn.push(RecoveryMutation::ApplyCodedTransition {
            transition: prepared.transition(),
        });
        let receipt = self.commit_coded_transition(txn)?;
        let confirmation = self
            .coded_captures
            .confirm_prepared_updates(prepared, &receipt);
        self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
        self.checksums.recovery_generation = receipt.generation();
        self.compact_resolved_capture_membership()
    }

    pub(crate) fn coded_permit_effect(
        &mut self,
        operation: OperationSlotToken,
    ) -> Result<CodedEffectOutcome, ServiceError> {
        self.admission.snapshot(operation).map_err(slot_error)?;
        let permit = self
            .coded_captures
            .permit_effect(operation)
            .map_err(|error| ServiceError::io(FailureClass::Admission, error.to_string()))?;
        let Some(permit) = permit else {
            self.operation_readiness.set(
                operation,
                OperationReadiness::Waiting(PendingReason::CaptureBlocked),
            );
            return Ok(CodedEffectOutcome::BlockedByCapture);
        };
        self.operation_readiness
            .set(operation, OperationReadiness::Runnable);
        Ok(CodedEffectOutcome::Permitted(permit))
    }

    fn coded_release_claim_with_certificate(
        &mut self,
        operation: OperationSlotToken,
        authorization: &ReleaseAuthorization,
        supplied_certificate: Option<&FenceCertificate>,
    ) -> Result<CodedClaimRelease, ServiceError> {
        if authorization.operation() != operation {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "release authorization does not match the coded operation",
            ));
        }
        let mut certificate = supplied_certificate.cloned().or_else(|| {
            usize::try_from(operation.index)
                .ok()
                .and_then(|index| self.write_drivers.get(index))
                .and_then(Option::as_ref)
                .filter(|driver| driver.operation == operation)
                .and_then(|driver| driver.certificate.clone())
        });
        let permit = self
            .admission
            .release_permit(operation)
            .map_err(slot_error)?;
        let mut current = self
            .recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        if let Some(candidate) = certificate.as_ref() {
            if candidate.occurrence == FenceOccurrenceId::UNASSIGNED {
                let mut fence_transaction =
                    RecoveryTxn::new(current.generation, self.topology.topology_epoch());
                fence_transaction.push(RecoveryMutation::RecordDataParityFence {
                    fence: candidate.clone(),
                });
                let receipt = self.recovery.commit_durable_receipt(fence_transaction);
                let receipt = self.finish_coded_persistence(receipt)?;
                certificate =
                    Some(receipt.persisted_fences().last().cloned().ok_or_else(|| {
                        ServiceError::io(
                            FailureClass::Recovery,
                            "durable release fence commit did not publish a fence occurrence",
                        )
                    })?);
                current = self
                    .recovery
                    .load_assembly_snapshot()
                    .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
                self.checksums.recovery_generation = current.generation;
            } else if !current
                .fence(candidate.occurrence_id())
                .is_some_and(|stored| stored.same_certificate_facts(candidate))
            {
                return Err(ServiceError::io(
                    FailureClass::ReconciliationRequired,
                    "release certificate does not match the persisted fence occurrence",
                ));
            }
        }
        let prepared = self
            .coded_captures
            .prepare_release(
                authorization,
                permit,
                certificate.as_ref(),
                current.generation,
            )
            .map_err(|error| {
                ServiceError::io(FailureClass::ReconciliationRequired, error.to_string())
            })?;
        let receipt = if let Some(transition) = prepared.transition() {
            let mut txn = RecoveryTxn::new(current.generation, self.topology.topology_epoch());
            txn.push(RecoveryMutation::ApplyCodedTransition { transition });
            let result = self.recovery.commit_durable_receipt(txn);
            if let Err(RecoveryError::CommitNotDurable(observation)) = &result {
                self.record_coded_release_commit_failure(operation, *observation);
            }
            let receipt = self.finish_coded_persistence(result)?;
            self.checksums.recovery_generation = receipt.generation();
            Some(receipt)
        } else {
            None
        };
        let confirmation = self
            .coded_captures
            .confirm_release(prepared, receipt.as_ref());
        self.finish_coded_confirmation(confirmation, FailureClass::ReconciliationRequired)
    }
    fn record_coded_release_commit_failure(
        &mut self,
        operation: OperationSlotToken,
        observation: RecoveryCommitObservation,
    ) {
        let state = if observation == RecoveryCommitObservation::Rejected {
            CodedReleaseCommitState::KnownNoncommit
        } else {
            CodedReleaseCommitState::MayHaveCommitted
        };
        if let Ok(index) = usize::try_from(operation.index)
            && let Some(Some(driver)) = self.write_drivers.get_mut(index)
            && driver.operation == operation
        {
            driver.coded_release_commit = state;
        }
    }
    fn release_coded_claim_if_authorized(
        &mut self,
        authorization: &ReleaseAuthorization,
        certificate: Option<&FenceCertificate>,
    ) -> Result<(), ServiceError> {
        let operation = authorization.operation();
        if self.coded_captures.operation_phase(operation).is_some() {
            self.coded_release_claim_with_certificate(operation, authorization, certificate)?;
        }
        Ok(())
    }
    /// Observe authorization only for the exact generation that produced it.
    pub fn release_authorization(
        &self,
        operation: OperationSlotToken,
    ) -> Option<&ReleaseAuthorization> {
        self.release_authorizations.observe(operation)
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
            self.release_coded_claim_if_authorized(&authorization, None)?;
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
        let token = {
            let release_authorizations = &self.release_authorizations;
            let coded_captures = &self.coded_captures;
            self.admission
                .reserve_if(request, |candidate| {
                    !release_authorizations.is_full_and_consumed(candidate, |existing| {
                        coded_capture_has_outstanding_release_consumer(coded_captures, existing)
                    })
                })
                .map_err(slot_error)?
        };
        self.set_release_scope(token, ReleaseScope::Outside);
        self.clear_basis_observation(token);
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

    fn allocate_clean_capture(&self) -> Result<CodedCaptureId, ServiceError> {
        let snapshot = self
            .recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        snapshot
            .next_coded_capture_id
            .checked_add(1)
            .ok_or_else(|| {
                ServiceError::io(
                    FailureClass::Recovery,
                    "coded CLEAN capture identity space is exhausted",
                )
            })?;
        Ok(CodedCaptureId(snapshot.next_coded_capture_id))
    }

    /// Persist a complete closed-set boundary before the first basis read.
    fn establish_clean_capture(
        &mut self,
        driver: &mut WriteDriver,
        capture: CodedCaptureId,
    ) -> Result<(), ServiceError> {
        if !driver.coded_admitted {
            return Err(ServiceError::io(
                FailureClass::Admission,
                "coded CLEAN capture requires completed coded admission",
            ));
        }
        if driver.clean_capture.is_some() {
            return Err(ServiceError::io(
                FailureClass::Admission,
                "write already has a coded CLEAN capture",
            ));
        }
        self.coded_start_capture(
            capture,
            driver.regions.iter().copied(),
            driver.checksum_extents.iter().copied(),
        )?;
        driver.clean_capture = Some(capture);
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
                    Err(ServiceError::Blocked(FailureClass::ReconciliationRequired)) => Ok(
                        PortableWriteDrive::Wait(PortableWriteWait::OwnerReconciliation),
                    ),
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
            Err(ServiceError::Blocked(FailureClass::ReconciliationRequired)) => {
                self.write_drivers[index] = Some(driver);
                Ok(PortableWriteDrive::Wait(
                    PortableWriteWait::OwnerReconciliation,
                ))
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
    pub fn accept_write_work(
        &mut self,
        work: &PortableWriteWork,
    ) -> Result<AcceptedPortableWriteWork, ServiceError> {
        let index = self.write_index(&work.submission)?;
        self.state.require_writes()?;
        if !self.coded_captures.is_usable() {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "coded lifecycle authority requires durable reopen reconciliation",
            ));
        }
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
        Ok(AcceptedPortableWriteWork { work: work.clone() })
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
                Err(ServiceError::Blocked(FailureClass::ReconciliationRequired)) => Ok(None),
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
            Ok(Some(evidence)) => evidence,
            Ok(None) => {
                self.write_drivers[index] = Some(driver);
                return Ok(None);
            }
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
            Err(ServiceError::Blocked(FailureClass::ReconciliationRequired)) => Ok(None),
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
                        let accepted = self.accept_write_work(&work)?;
                        let result = self.execute_write_work(accepted)?;
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
        if driver.coded_release_commit == CodedReleaseCommitState::MayHaveCommitted {
            return Ok(PortableWriteDrive::Wait(
                PortableWriteWait::OwnerReconciliation,
            ));
        }
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
        if !self.state.accepts_writes() || !self.coded_captures.is_usable() {
            return Ok(PortableWriteDrive::Wait(
                PortableWriteWait::OwnerReconciliation,
            ));
        }
        if !driver.coded_admitted {
            match self.coded_admit(driver.operation, driver.coded_claim.clone())? {
                CodedAdmissionOutcome::Contended => {
                    return Ok(PortableWriteDrive::Wait(
                        PortableWriteWait::CodedRangeContention,
                    ));
                }
                CodedAdmissionOutcome::Admitted(_) => {
                    driver.coded_admitted = true;
                    driver
                        .machine
                        .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
                        .map_err(|error| {
                            ServiceError::io(FailureClass::Admission, error.to_string())
                        })?;
                }
            }
        }
        if driver.basis_permission != BasisReadPermission::Granted {
            return Ok(PortableWriteDrive::Wait(
                PortableWriteWait::BasisReadPermission,
            ));
        }
        if driver.clean_capture.is_none() {
            if self.coded_captures.capture_count() >= self.config.max_coded_captures {
                self.compact_resolved_capture_membership()?;
                self.retire_resolved_captures()?;
                if self.coded_captures.capture_count() >= self.config.max_coded_captures {
                    return Ok(PortableWriteDrive::Wait(
                        PortableWriteWait::CodedCaptureCapacity,
                    ));
                }
            }
            let capture = self.allocate_clean_capture()?;
            self.establish_clean_capture(driver, capture)?;
        }
        self.ensure_write_recovery_record(driver)?;
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
                if driver.coded_effect_permit.is_none() {
                    match self.coded_permit_effect(driver.operation)? {
                        CodedEffectOutcome::BlockedByCapture => {
                            return Ok(PortableWriteDrive::Wait(PortableWriteWait::CaptureBlocked));
                        }
                        CodedEffectOutcome::Permitted(permit) => {
                            driver.coded_effect_permit = Some(permit);
                        }
                    }
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
                match self.finish_write_driver(driver)? {
                    Some(evidence) => Ok(PortableWriteDrive::Complete(evidence)),
                    None => Ok(PortableWriteDrive::Wait(
                        PortableWriteWait::CleanCaptureMembers,
                    )),
                }
            }
        }
    }
    fn release_write_driver(
        &mut self,
        index: usize,
        driver: WriteDriver,
        mut evidence: OperationEvidence,
    ) -> Result<OperationEvidence, ServiceError> {
        // Every retry re-enters the exact lifecycle/coded-release protocol.
        // Cached lifecycle authorization alone does not prove that the coded
        // release receipt committed durably.
        self.set_basis_conformance(driver.operation, BasisConformance::Consumed);
        let request = driver.request;
        self.write_drivers[index] = Some(driver);
        match self.finish(
            self.write_drivers[index]
                .as_ref()
                .expect("write driver was retained for release")
                .operation,
            false,
            transaction_requirement(Some(&evidence.trace)),
        ) {
            Ok(release_authorization) => {
                evidence.release_authorization = release_authorization;
                self.write_drivers[index] = None;
                Ok(evidence)
            }
            Err(ServiceError::Blocked(FailureClass::ReconciliationRequired)) => {
                Err(ServiceError::Blocked(FailureClass::ReconciliationRequired))
            }
            Err(cleanup) => {
                let mut driver = self.write_drivers[index]
                    .take()
                    .expect("failed release retains its write driver");
                let cleanup = self.cleanup_error(request, cleanup);
                let class = Self::write_failure_class(&driver);
                let error = self.fail_write_driver(&mut driver, class, cleanup);
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
        accepted: AcceptedPortableWriteWork,
    ) -> Result<PortableWriteResult, ServiceError> {
        let work = accepted.work();
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
        Ok(PortableWriteResult::new(accepted, completion, read_payload))
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
    fn coded_claim_for_plan(
        &self,
        member_index: usize,
        request: BlockRequest,
        plan: &crate::range::RangePlan,
        operation: OperationSlotToken,
    ) -> Result<CodedClaimInput, ServiceError> {
        let assignment = self
            .topology
            .assignment_for_slot(request.slot_id)
            .ok_or_else(|| {
                ServiceError::invalid(
                    FailureClass::InvalidRequest,
                    "coded claim member is not in the admitted topology",
                )
            })?;
        if member_index_for_assignment(&self.members, assignment)? != member_index {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "coded claim member does not match the admitted request",
            ));
        }
        self.coded_geometry
            .claim_for_mutation(operation, request, assignment, plan.ranges.iter().copied())
            .map_err(|error| ServiceError::invalid(FailureClass::InvalidRequest, error.to_string()))
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
        let coded_claim = self.coded_claim_for_plan(member_index, request, plan, token)?;
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
            coded_effect_permit: None,
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
            coded_release_commit: CodedReleaseCommitState::NotAttempted,
            work,
            regions,
            checksum_extents,
            clean_capture: None,
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
        let mut prepared_cuts = Vec::new();
        let mut capture_mutations = Vec::new();
        let target = self
            .coded_captures
            .invalidation_target(driver.operation)
            .ok_or_else(|| {
                ServiceError::io(
                    FailureClass::Admission,
                    "coded admission witness is unavailable",
                )
            })?;
        for snapshot in self.coded_captures.snapshots() {
            if snapshot.membership.get(&driver.operation) != Some(&CodedCaptureMembership::Later) {
                continue;
            }
            let prepared = self
                .coded_captures
                .prepare_later_cut(
                    snapshot.capture,
                    driver.operation,
                    self.topology.topology_epoch(),
                    generation,
                )
                .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
            capture_mutations.push(prepared.transition());
            prepared_cuts.push(prepared);
        }
        let result = self
            .checksums
            .invalidate_with_write_recovery_record_and_coded_transitions(
                &mut self.recovery,
                target,
                capture_mutations,
            );
        let result = if prepared_cuts.is_empty() {
            result.map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?
        } else {
            self.finish_coded_persistence(result)?
        };
        if !prepared_cuts.is_empty() {
            let Some(receipt) = result.receipt.as_ref() else {
                self.invalidate_coded_transition_authority();
                return Err(ServiceError::io(
                    FailureClass::Recovery,
                    "write-recovery commit omitted its coded-cut receipt",
                ));
            };
            for prepared in prepared_cuts {
                let confirmation = self.coded_captures.confirm_later_cut(prepared, receipt);
                self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
            }
        }
        let record = result.evidence;
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

    fn released_coded_lifecycle_evidence(
        &self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    ) -> Option<(IncludedLifecycleAuthorization, FenceCertificate)> {
        let snapshot = self.coded_captures.capture_snapshot(capture)?;
        let certificate = snapshot.release_certificates.get(&operation)?;
        if snapshot.membership.get(&operation) != Some(&CodedCaptureMembership::Included)
            || !snapshot.release_authorized_operations.contains(&operation)
        {
            return None;
        }
        let release = self.release_authorizations.observe(operation)?;
        let authorization = self
            .lifecycle_authority
            .authorize_included_released(release.lifecycle())
            .map(IncludedLifecycleAuthorization::from_lifecycle)?;
        Some((authorization, certificate.clone()))
    }

    fn evaluate_capture_clean(
        &self,
        driver: &WriteDriver,
        current: &RecoverySnapshot,
        write_recovery_record: &WriteRecoveryRecordEvidence,
        certificate: &FenceCertificate,
    ) -> Result<Option<CaptureCleanEvaluation>, ServiceError> {
        let capture = driver.clean_capture.ok_or_else(|| {
            ServiceError::io(
                FailureClass::ReconciliationRequired,
                "recovery CLEAN requires an externally established coded capture",
            )
        })?;
        let capture_snapshot = self
            .coded_captures
            .capture_snapshot(capture)
            .ok_or_else(|| {
                ServiceError::io(
                    FailureClass::ReconciliationRequired,
                    format!("coded CLEAN capture {capture:?} is no longer available"),
                )
            })?;
        if capture_snapshot.phase != CodedCapturePhase::Open
            || capture_snapshot.decision.is_some()
            || !capture_snapshot.scope_complete
            || !capture_snapshot.scope_validated
            || !capture_snapshot.lower_frontier_covered
            || !self.coded_captures.capture_owner_revalidated(capture)
        {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                format!("coded CLEAN capture {capture:?} is not an undecided owner-issued capture"),
            ));
        }
        if capture_snapshot.topology != self.topology
            || capture_snapshot.checksum_profile
                != self.checksums.profile_id().ok_or_else(|| {
                    ServiceError::io(FailureClass::Recovery, "checksum profile is unavailable")
                })?
            || capture_snapshot.checksum_set_generation != self.checksums.active_set
        {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                format!("coded CLEAN capture {capture:?} is stale for the current owner state"),
            ));
        }
        let selected_regions = driver.regions.iter().copied().collect::<BTreeSet<_>>();
        let selected_extents = driver
            .checksum_extents
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let expected_scope = CodedCaptureOwnerFacts::selected_invalidation_scope(
            &self.topology,
            selected_regions.iter().copied(),
            selected_extents.iter().copied(),
            DIRTY_REGION_BYTES,
            BLAKE3_256_PROFILE,
        )
        .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        if capture_snapshot.dirty_regions != selected_regions
            || capture_snapshot.checksum_extents != selected_extents
            || capture_snapshot.scope != *expected_scope.units()
            || capture_snapshot.membership.get(&driver.operation)
                != Some(&CodedCaptureMembership::Included)
        {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                format!("coded CLEAN capture {capture:?} does not cover the exact selected state"),
            ));
        }
        if capture_snapshot.membership.values().any(|membership| {
            matches!(
                membership,
                CodedCaptureMembership::Later | CodedCaptureMembership::LaterUnknown
            )
        }) {
            return Ok(None);
        }

        let mut evidence = Vec::new();
        let mut certificates = Vec::new();
        for (operation, membership) in &capture_snapshot.membership {
            if *membership != CodedCaptureMembership::Included {
                continue;
            }
            let lifecycle_evidence = if *operation == driver.operation {
                let Some(authorization) = self
                    .lifecycle_authority
                    .authorize_included_live(
                        self.admission.lifecycle_slots(),
                        *operation,
                        driver.finalization >= WriteFinalizationPhase::FlushesCompleted,
                    )
                    .map(IncludedLifecycleAuthorization::from_lifecycle)
                else {
                    return Ok(None);
                };
                (authorization, certificate.clone())
            } else if let Some(evidence) =
                self.released_coded_lifecycle_evidence(capture, *operation)
            {
                evidence
            } else if let Some(certificate) = self
                .write_drivers
                .iter()
                .flatten()
                .find(|other| {
                    other.operation == *operation
                        && other.finalization >= WriteFinalizationPhase::FlushesCompleted
                })
                .and_then(|other| other.certificate.as_ref())
            {
                let Some(authorization) = self
                    .lifecycle_authority
                    .authorize_included_live(self.admission.lifecycle_slots(), *operation, true)
                    .map(IncludedLifecycleAuthorization::from_lifecycle)
                else {
                    return Ok(None);
                };
                (authorization, certificate.clone())
            } else {
                return Ok(None);
            };
            certificates.push(lifecycle_evidence.1.clone());
            evidence.push(lifecycle_evidence);
        }

        let mut store_fences = BTreeMap::<StoreId, StoreFenceRef>::new();
        let mut captured_regions = BTreeMap::<RegionId, RecoveryGeneration>::new();
        let mut captured_extents = BTreeMap::<IntegrityExtentId, RecoveryGeneration>::new();
        for certificate in certificates {
            if certificate.topology_epoch != self.topology.topology_epoch()
                || certificate.fence_domain != driver.request.ordering.fence_domain
            {
                return Err(ServiceError::io(
                    FailureClass::Fence,
                    "included operation persistence certificate is stale or from another domain",
                ));
            }
            for fence in &certificate.stores {
                match store_fences.entry(fence.store_id) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(*fence);
                    }
                    std::collections::btree_map::Entry::Occupied(mut entry) => {
                        let prior = entry.get();
                        if prior.topology_epoch != fence.topology_epoch
                            || prior.store_incarnation != fence.store_incarnation
                            || prior.capability_evidence_id != fence.capability_evidence_id
                        {
                            return Err(ServiceError::io(
                                FailureClass::Fence,
                                "included operation certificates disagree on store identity",
                            ));
                        }
                        if fence.through > prior.through {
                            entry.insert(*fence);
                        }
                    }
                }
            }
            for (region, generation) in &certificate.captured_region_generations {
                captured_regions
                    .entry(*region)
                    .and_modify(|current| *current = (*current).max(*generation))
                    .or_insert(*generation);
            }
            for (extent, generation) in &certificate.captured_integrity_generations {
                captured_extents
                    .entry(*extent)
                    .and_modify(|current| *current = (*current).max(*generation))
                    .or_insert(*generation);
            }
        }
        let aggregate_certificate = FenceCertificate {
            occurrence: FenceOccurrenceId::UNASSIGNED,
            topology_epoch: self.topology.topology_epoch(),
            fence_domain: driver.request.ordering.fence_domain,
            stores: store_fences.values().copied().collect(),
            captured_region_generations: captured_regions.into_iter().collect(),
            captured_integrity_generations: captured_extents.into_iter().collect(),
        };
        let mut request = RecoveryCleanRequest::new(
            self.topology.topology_epoch(),
            driver.request.ordering.fence_domain,
            current.generation,
        );
        for fence in &aggregate_certificate.stores {
            request = request.with_store(fence.store_id, fence.through);
        }
        for region in &capture_snapshot.dirty_regions {
            request = request.with_region(*region);
        }
        for extent in &capture_snapshot.checksum_extents {
            request = request.with_checksum_extent(*extent);
        }
        if write_recovery_record.committed_generation > current.generation {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "write-recovery generation is newer than the current recovery snapshot",
            ));
        }
        let decision = evaluate_recovery_clean(current, &request, &aggregate_certificate)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        Ok(Some((decision, evidence)))
    }

    fn finish_write_driver(
        &mut self,
        driver: &mut WriteDriver,
    ) -> Result<Option<OperationEvidence>, ServiceError> {
        if let Some(evidence) = driver.evidence.clone() {
            return Ok(Some(evidence));
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
                store_fences.sort_unstable_by_key(|fence| {
                    (
                        fence.store_id,
                        fence.fence_id,
                        fence.topology_epoch,
                        fence.store_incarnation,
                        fence.capability_evidence_id,
                        fence.through,
                    )
                });
                let mut regions = driver.regions.clone();
                regions.sort_unstable();
                let mut extents = driver.checksum_extents.clone();
                extents.sort_unstable();
                let certificate = extents.into_iter().fold(
                    FenceCertificate::new(
                        self.topology.topology_epoch(),
                        driver.request.ordering.fence_domain,
                        store_fences,
                        regions
                            .into_iter()
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
                let certificate = driver
                    .certificate
                    .clone()
                    .expect("flush finalization retains its certificate");
                let capture = driver.clean_capture.expect("validated capture is bound");
                let Some((decision, included_evidence)) = self.evaluate_capture_clean(
                    driver,
                    &current,
                    &write_recovery_record,
                    &certificate,
                )?
                else {
                    return Ok(None);
                };
                let prepared = match decision {
                    RecoveryCleanDecision::Refused {
                        reason, receipt, ..
                    } => {
                        let prepared = self
                            .coded_captures
                            .prepare_clean_refusal(capture, &receipt)
                            .map_err(|error| {
                                ServiceError::io(FailureClass::Recovery, error.to_string())
                            })?;
                        let mut refused =
                            RecoveryTxn::new(current.generation, self.topology.topology_epoch());
                        refused.push(RecoveryMutation::ApplyCodedTransition {
                            transition: prepared.transition(),
                        });
                        let durable = self.commit_coded_transition(refused)?;
                        let confirmation = self
                            .coded_captures
                            .confirm_clean_refusal(prepared, &durable);
                        self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
                        self.checksums.recovery_generation = durable.generation();
                        return Err(ServiceError::io(
                            FailureClass::Recovery,
                            format!("recovery CLEAN owner refused the selected state: {reason:?}"),
                        ));
                    }
                    RecoveryCleanDecision::Clear(permit) => {
                        let authorization = self
                            .coded_captures
                            .authorize_clean(&current, capture, permit, included_evidence)
                            .map_err(|error| {
                                ServiceError::io(FailureClass::Recovery, error.to_string())
                            })?;
                        self.coded_captures
                            .prepare_clean_commit(authorization)
                            .map_err(|error| {
                                ServiceError::io(FailureClass::Recovery, error.to_string())
                            })?
                    }
                };
                let mut recovery_clean =
                    RecoveryTxn::new(current.generation, self.topology.topology_epoch());
                recovery_clean.push(RecoveryMutation::ApplyCodedTransition {
                    transition: prepared.transition(),
                });
                let receipt = self.commit_coded_transition(recovery_clean)?;
                let persisted_fence =
                    receipt.persisted_fences().last().cloned().ok_or_else(|| {
                        ServiceError::io(
                            FailureClass::Recovery,
                            "durable CLEAN commit did not publish a fence occurrence",
                        )
                    })?;
                driver.certificate = Some(persisted_fence.clone());
                driver
                    .machine
                    .bind_persisted_fence(persisted_fence)
                    .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
                let committed = receipt.generation();
                let confirmation = self.coded_captures.confirm_clean_commit(prepared, &receipt);
                self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
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
            .map(Some)
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
        self.establish_release_authorization_with_certificate(observation, None)
    }

    #[cfg(test)]
    pub(crate) fn establish_release_authorization_for_test(
        &mut self,
        observation: &ReleaseReconciliation,
        certificate: &FenceCertificate,
    ) -> Result<Option<ReleaseAuthorization>, ServiceError> {
        self.establish_release_authorization_with_certificate(observation, Some(certificate))
    }

    fn establish_release_authorization_with_certificate(
        &mut self,
        observation: &ReleaseReconciliation,
        certificate: Option<&FenceCertificate>,
    ) -> Result<Option<ReleaseAuthorization>, ServiceError> {
        let authorization = self.release_authorization_candidate(observation)?;
        if let Some(authorization) = authorization.as_ref() {
            // LifecycleRelease owns this monotonic exact-generation fact.
            // Retain it before invoking downstream coded-claim removal so a
            // coded cleanup failure cannot retroactively erase authorization.
            self.remember_release_authorization(authorization.clone())?;
            self.release_coded_claim_if_authorized(authorization, certificate)?;
        }
        Ok(authorization)
    }

    #[cfg(test)]
    fn included_lifecycle_authorization_for_test(
        &mut self,
        observation: &ReleaseReconciliation,
    ) -> Result<Option<IncludedLifecycleAuthorization>, ServiceError> {
        self.release_authorization_candidate(observation)
            .map(|authorization| {
                authorization.and_then(|authorization| {
                    self.lifecycle_authority
                        .authorize_included_live_from_release(authorization.lifecycle())
                        .map(IncludedLifecycleAuthorization::from_lifecycle)
                })
            })
    }

    fn release_authorization_candidate(
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
        self.ensure_release_authorization_available(token)?;
        Ok(self.authorization_candidate(token, &snapshot, observation))
    }

    fn release(&mut self, token: OperationSlotToken) -> Result<(), ServiceError> {
        let result = self.admission.release(token).map_err(slot_error);
        if result.is_ok() {
            self.clear_release_scope(token);
            self.clear_basis_observation(token);
            self.operation_readiness.clear(token);
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
    fn compact_resolved_capture_membership(&mut self) -> Result<(), ServiceError> {
        let current = self
            .recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let captures = self
            .coded_captures
            .snapshots()
            .into_iter()
            .filter(|snapshot| !snapshot.release_frontiers.is_empty())
            .map(|snapshot| snapshot.capture)
            .collect::<Vec<_>>();
        let mut prepared = Vec::new();
        let mut txn = RecoveryTxn::new(current.generation, self.topology.topology_epoch());
        for capture in captures {
            let Some(authorization) =
                CodedCaptureRetentionOwner::authorize_membership_compaction(&current, capture)
                    .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?
            else {
                continue;
            };
            let compaction = self
                .coded_captures
                .prepare_membership_compaction(authorization)
                .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
            txn.push(RecoveryMutation::ApplyCodedTransition {
                transition: compaction.transition(),
            });
            prepared.push(compaction);
        }
        if prepared.is_empty() {
            return Ok(());
        }
        let receipt = self.commit_coded_transition(txn)?;
        for compaction in prepared {
            let confirmation = self
                .coded_captures
                .confirm_membership_compaction(compaction, &receipt);
            self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
        }
        self.checksums.recovery_generation = receipt.generation();
        Ok(())
    }

    fn retire_resolved_captures(&mut self) -> Result<(), ServiceError> {
        let current = self
            .recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let mut refused = Vec::new();
        let mut clean_known = Vec::new();
        let mut txn = RecoveryTxn::new(current.generation, self.topology.topology_epoch());
        for snapshot in self.coded_captures.snapshots() {
            if snapshot.phase == CodedCapturePhase::Open {
                continue;
            }
            let resolution = self
                .coded_captures
                .resolve_capture(&current, snapshot.capture, None)
                .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
            match resolution {
                CodedReopenResolution::Refused(authorization) => {
                    let cleanup = self
                        .coded_captures
                        .prepare_refused_cleanup(authorization)
                        .map_err(|error| {
                            ServiceError::io(FailureClass::Recovery, error.to_string())
                        })?;
                    txn.push(RecoveryMutation::ApplyCodedTransition {
                        transition: cleanup.transition(),
                    });
                    refused.push(cleanup);
                }
                CodedReopenResolution::CleanKnown(authorization) => {
                    let cleanup = self
                        .coded_captures
                        .prepare_clean_known_cleanup(authorization)
                        .map_err(|error| {
                            ServiceError::io(FailureClass::Recovery, error.to_string())
                        })?;
                    txn.push(RecoveryMutation::ApplyCodedTransition {
                        transition: cleanup.transition(),
                    });
                    clean_known.push(cleanup);
                }
                CodedReopenResolution::Abandon(_) => {}
            }
        }
        if refused.is_empty() && clean_known.is_empty() {
            return Ok(());
        }
        let receipt = self.commit_coded_transition(txn)?;
        for cleanup in refused {
            let confirmation = self
                .coded_captures
                .confirm_refused_cleanup(cleanup, &receipt);
            self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
        }
        for cleanup in clean_known {
            let confirmation = self
                .coded_captures
                .confirm_clean_known_cleanup(cleanup, &receipt);
            self.finish_coded_confirmation(confirmation, FailureClass::Recovery)?;
        }
        self.checksums.recovery_generation = receipt.generation();
        Ok(())
    }

    fn release_after_reclaim(&mut self, token: OperationSlotToken) -> Result<(), ServiceError> {
        if self.coded_captures.operation_phase(token).is_some() {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "operation slot cannot be released while its coded claim remains active",
            ));
        }
        #[cfg(test)]
        if self.take_terminalization_fault(TerminalizationFault::Reclaim) {
            return Err(ServiceError::io(
                FailureClass::ReconciliationRequired,
                "injected terminalization reclaim failure",
            ));
        }
        self.compact_resolved_capture_membership()?;
        self.retire_resolved_captures()?;
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

    fn remember_release_authorization(
        &mut self,
        authorization: ReleaseAuthorization,
    ) -> Result<(), ServiceError> {
        let operation = authorization.operation();
        let discharged = {
            let admission = &self.admission;
            let coded_captures = &self.coded_captures;
            let write_drivers = &self.write_drivers;
            self.release_authorizations
                .operations_at_index(operation)
                .find(|existing| {
                    !has_outstanding_release_consumer(
                        admission,
                        coded_captures,
                        write_drivers,
                        *existing,
                    )
                })
        };
        if let Some(discharged) = discharged {
            self.release_authorizations.retire(discharged);
        }
        let admission = &self.admission;
        let coded_captures = &self.coded_captures;
        let write_drivers = &self.write_drivers;
        self.release_authorizations
            .retain_with_cleanup(authorization, |operation| {
                !has_outstanding_release_consumer(
                    admission,
                    coded_captures,
                    write_drivers,
                    operation,
                )
            })
            .map_err(|_| {
                ServiceError::io(
                    FailureClass::ReconciliationRequired,
                    "release authorization retention is exhausted",
                )
            })
    }
    fn release_authorization_blocked(&self, operation: OperationSlotToken) -> bool {
        let admission = &self.admission;
        let coded_captures = &self.coded_captures;
        let write_drivers = &self.write_drivers;
        self.release_authorizations
            .is_full_and_consumed(operation, |existing| {
                has_outstanding_release_consumer(admission, coded_captures, write_drivers, existing)
            })
    }

    fn ensure_release_authorization_available(
        &self,
        operation: OperationSlotToken,
    ) -> Result<(), ServiceError> {
        if self.release_authorization_blocked(operation) {
            return Err(ServiceError::Blocked(FailureClass::ReconciliationRequired));
        }
        Ok(())
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
        self.lifecycle_authority
            .authorize_release(
                self.admission.lifecycle_slots(),
                token,
                matches!(
                    observation.operation_effect,
                    OperationEffect::Terminal | OperationEffect::AuthoritativelyReconciled
                ),
                observation.requirement.is_satisfied(),
                matches!(observation.recovery, RecoveryReconciliation::Authoritative),
                observation.basis.is_conformant(),
            )
            .map(ReleaseAuthorization::from_lifecycle)
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
        self.release_scopes.observe(operation)
    }

    fn set_release_scope(&mut self, operation: OperationSlotToken, scope: ReleaseScope) {
        self.release_scopes.set(operation, scope);
    }

    fn clear_release_scope(&mut self, operation: OperationSlotToken) {
        self.release_scopes.clear(operation);
    }

    fn basis_conformance_observation(&self, operation: OperationSlotToken) -> BasisConformance {
        self.basis_observations.observe(operation)
    }

    fn set_basis_conformance(
        &mut self,
        operation: OperationSlotToken,
        observation: BasisConformance,
    ) {
        self.basis_observations.record(operation, observation);
    }

    fn clear_basis_observation(&mut self, operation: OperationSlotToken) {
        self.basis_observations.clear(operation);
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
