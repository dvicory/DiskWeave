use crate::{
    admission::{AdmissionConfig, OperationAdmission, slot_error},
    evidence::{CompletionEvidence, OperationEvidence, PersistenceClaim},
    failure::{FailureClass, ServiceError},
    lifecycle::ServiceState,
    range::split_range,
    read::read_member,
    write::{update_parity, write_member},
};
use dwv_codec::Geometry as CodecGeometry;
use dwv_core::{
    AssignmentGeneration, AssignmentInstanceId, BlockOp, BlockRequest, ByteRange, CodingPosition,
    DurabilityIntent, MemberRole, SlotId, TopologyAssignment, TopologyEpoch, TopologySnapshot,
};
use dwv_recovery::{
    BLAKE3_256_PROFILE, ChecksumAuthority, ChecksumBaselineStatus, ChecksumExtent,
    ChecksumPersistenceEvidence, ChecksumRecord, ChecksumSetGeneration, ChecksumTarget,
    ContentGeneration, DIRTY_REGION_BYTES, FenceCertificate, IntegrityExtentId, IntegrityState,
    InvalidationTarget, RecoveryGeneration, RecoveryMutation, RecoverySnapshot, RecoveryStateStore,
    RecoveryStoreHealth, RecoveryTxn, RegionId, assess_checksum_baseline, dirty_regions_for_range,
};
use dwv_store::{
    CompletedRangeSet, CompletionDisposition, FenceDomain, IdentityAssessment, IdentityComparison,
    IdentityObservationSet, IdentitySourceKind, OperationSlotToken, PersistenceEvidence,
    RandomAccessStore, StoreId, StoreWriteWatermark, WriteIntent,
};
use dwv_transaction_ref::{
    ActionResult, CommittedRecoveryGeneration, ComputationResult, ParityComputationPlan,
    ParityRange, PlannedRead, PlannedWrite, RangeGuardToken, SemanticIoResult, StoreWatermark,
    TransactionLimits, TransactionMachine, TransactionPersistenceEvidence, TransactionPlan,
    WriteRecoveryRecordRequirement,
};
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

pub struct HealthyPortableService<S: RandomAccessStore, R: RecoveryStateStore> {
    topology: TopologySnapshot,
    members: Vec<MemberBinding<S>>,
    recovery: R,
    checksums: ChecksumAuthority,
    admission: OperationAdmission,
    config: ServiceConfig,
    state: ServiceState,
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
            checksums,
            admission: OperationAdmission::new(config.admission),
            config,
            state,
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
                self.finish(token, false)
                    .map_err(|error| error.with_request(request))?;
                let generation = self
                    .recovery_generation()
                    .map_err(|error| error.with_request(request))?;
                let trace = empty_trace(self.topology.topology_epoch(), generation)
                    .map_err(|error| error.with_request(request))?;
                Ok((
                    bytes,
                    OperationEvidence {
                        request,
                        completion,
                        trace,
                    },
                ))
            }
            Err(error) => {
                let _ = self.finish(token, true);
                Err(error.with_request(request))
            }
        }
    }

    /// dwv:req req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity
    pub fn write(
        &mut self,
        request: BlockRequest,
        bytes: &[u8],
    ) -> Result<OperationEvidence, ServiceError> {
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
        let result =
            self.execute_write(member_index, coding_position, request, bytes, &plan, token);
        match result {
            Ok(evidence) => {
                self.finish(token, false)
                    .map_err(|error| error.with_request(request))?;
                Ok(evidence)
            }
            Err(error) => {
                self.state = ServiceState::Recovering;
                let _ = self.finish(token, true);
                Err(error.with_request(request))
            }
        }
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
            self.admission
                .submit_all(token, flush_children.len())
                .map_err(slot_error)?;
            let mut incomplete = None;
            for (member, child) in self.members.iter_mut().zip(&flush_children) {
                let through = member
                    .store
                    .highest_accepted_watermark()
                    .unwrap_or(StoreWriteWatermark(0));
                let completion = member.store.flush(*child, through);
                let reported = completion.clone();
                self.admission
                    .complete(token, completion)
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
            })
        })();
        match result {
            Ok(evidence) => {
                self.finish(token, false)
                    .map_err(|error| error.with_request(request))?;
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
                let _ = self.finish(token, true);
                Err(error.with_request(request))
            }
        }
    }

    pub fn abandon(&mut self, token: OperationSlotToken) -> Result<(), ServiceError> {
        self.admission.abandon(token).map_err(slot_error)
    }

    fn execute_write(
        &mut self,
        member_index: usize,
        coding_position: CodingPosition,
        request: BlockRequest,
        bytes: &[u8],
        plan: &crate::range::RangePlan,
        token: OperationSlotToken,
    ) -> Result<OperationEvidence, ServiceError> {
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
        .with_stores(stores.clone());
        tx_plan.limits = TransactionLimits::default();
        let mut machine = TransactionMachine::new(tx_plan)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
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

        let mut computed = Vec::with_capacity(plan.ranges.len());
        let codec_geometry = CodecGeometry::new(
            vec![
                self.topology.geometry().protected_length();
                usize::from(self.topology.profile().data_slots())
            ],
            self.topology.geometry().parity_length(),
        )
        .map_err(|error| ServiceError::io(FailureClass::Range, error.to_string()))?;
        let mut child_ranges = Vec::new();
        for range in &plan.ranges {
            child_ranges.extend([*range, *range, *range, *range]);
        }
        child_ranges.extend([ByteRange::empty(), ByteRange::empty()]);
        let children = self
            .admission
            .children(token, &child_ranges)
            .map_err(slot_error)?;
        self.admission
            .submit_all(token, children.len())
            .map_err(slot_error)?;
        let mut read_children = Vec::new();
        let mut data_write_children = Vec::new();
        let mut parity_write_children = Vec::new();
        for chunk in children.chunks_exact(4) {
            read_children.push((chunk[0], chunk[1]));
            data_write_children.push(chunk[2]);
            parity_write_children.push(chunk[3]);
        }
        let flush_children = [children[children.len() - 2], children[children.len() - 1]];
        let mut byte_cursor = 0_usize;
        for (index, range) in plan.ranges.iter().enumerate() {
            let old_data = read_child(
                &mut self.members[member_index].store,
                &mut self.admission,
                token,
                read_children[index].0,
                *range,
                FailureClass::StoreRead,
            )?;
            let old_parity = read_child(
                &mut self.members[parity_index].store,
                &mut self.admission,
                token,
                read_children[index].1,
                *range,
                FailureClass::StoreRead,
            )?;
            let length = usize::try_from(range.length)
                .map_err(|_| ServiceError::io(FailureClass::Range, "range does not fit memory"))?;
            let new_data = &bytes[byte_cursor..byte_cursor + length];
            let new_parity = update_parity(
                &codec_geometry,
                *range,
                usize::from(coding_position.0),
                &old_data,
                new_data,
                &old_parity,
            )?;
            computed.push((*range, new_data.to_vec(), new_parity));
            byte_cursor += length;
        }
        machine
            .apply(ActionResult::ReadSetComplete(SemanticIoResult::complete()))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        machine
            .apply(ActionResult::ParityComputed(ComputationResult::complete()))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;

        let write_intent = if request.durability == DurabilityIntent::Ordinary {
            WriteIntent::Ordinary
        } else {
            WriteIntent::Preflush
        };
        let mut data_watermark = None;
        let mut parity_watermark = None;
        for (index, (range, new_data, new_parity)) in computed.iter().enumerate() {
            data_watermark = Some(write_member(
                &mut self.members[member_index].store,
                &mut self.admission,
                token,
                data_write_children[index],
                *range,
                new_data,
                write_intent,
            )?);
            parity_watermark = Some(write_member(
                &mut self.members[parity_index].store,
                &mut self.admission,
                token,
                parity_write_children[index],
                *range,
                new_parity,
                write_intent,
            )?);
        }
        let data_watermark = data_watermark
            .ok_or_else(|| ServiceError::io(FailureClass::Fence, "data write lacks watermark"))?;
        let parity_watermark = parity_watermark
            .ok_or_else(|| ServiceError::io(FailureClass::Fence, "parity write lacks watermark"))?;
        machine
            .set_write_watermarks(vec![
                StoreWatermark::for_incarnation(
                    self.members[member_index].store_id,
                    self.members[member_index].store.incarnation(),
                    data_watermark,
                ),
                StoreWatermark::for_incarnation(
                    self.members[parity_index].store_id,
                    self.members[parity_index].store.incarnation(),
                    parity_watermark,
                ),
            ])
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        machine
            .apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;

        let data_fence = flush_member(
            &mut self.members[member_index].store,
            &mut self.admission,
            token,
            flush_children[0],
            data_watermark,
        )?;
        let parity_fence = flush_member(
            &mut self.members[parity_index].store,
            &mut self.admission,
            token,
            flush_children[1],
            parity_watermark,
        )?;
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
        let certificate = checksum_extents.iter().copied().fold(
            FenceCertificate::new(
                self.topology.topology_epoch(),
                request.ordering.fence_domain,
                store_fences,
                regions
                    .iter()
                    .copied()
                    .map(|region| (region, write_recovery_record.committed_generation))
                    .collect(),
            ),
            |certificate, extent| {
                certificate
                    .with_integrity_extent(extent, write_recovery_record.committed_generation)
            },
        );
        machine
            .apply(ActionResult::FlushSetComplete(
                TransactionPersistenceEvidence::durable(certificate.clone()),
            ))
            .map_err(|error| ServiceError::io(FailureClass::Fence, error.to_string()))?;
        let current = self
            .recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        if current.generation != write_recovery_record.committed_generation {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "recovery generation changed during write",
            ));
        }
        let mut recovery_clean =
            RecoveryTxn::new(current.generation, self.topology.topology_epoch());
        recovery_clean.push(RecoveryMutation::RecordDataParityFence { fence: certificate });
        for region in regions {
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
        machine
            .apply(ActionResult::RecoveryCleanCommitted(
                CommittedRecoveryGeneration::new(committed, self.topology.topology_epoch()),
            ))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        machine
            .apply(ActionResult::RangeReleased)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        Ok(OperationEvidence {
            request,
            completion: CompletionEvidence {
                requested: request.range,
                completed: CompletedRangeSet::new(vec![request.range])
                    .expect("validated write range is a completion range"),
                disposition: CompletionDisposition::Success,
                persistence: PersistenceClaim::HostFenceOnly,
            },
            trace: machine.trace().clone(),
        })
    }

    fn reserve(&mut self, request: BlockRequest) -> Result<OperationSlotToken, ServiceError> {
        self.admission.reserve(request).map_err(slot_error)
    }

    fn finish(&mut self, token: OperationSlotToken, uncertain: bool) -> Result<(), ServiceError> {
        if uncertain {
            let children = self
                .admission
                .snapshot(token)
                .map_err(slot_error)?
                .children
                .into_iter()
                .map(|child| (child.operation_id, child.requested))
                .collect::<Vec<_>>();
            self.admission
                .reconcile_children(token, &children)
                .map_err(slot_error)?;
        }
        self.admission.reclaim(token, uncertain).map_err(slot_error)
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

fn read_child<S: RandomAccessStore>(
    store: &mut S,
    admission: &mut OperationAdmission,
    token: OperationSlotToken,
    child: dwv_store::ChildOperationId,
    range: ByteRange,
    class: FailureClass,
) -> Result<Vec<u8>, ServiceError> {
    let length = usize::try_from(range.length)
        .map_err(|_| ServiceError::io(FailureClass::Range, "range does not fit memory"))?;
    let mut bytes = vec![0; length];
    let result = store.read_at(child, range, &mut bytes);
    let reported = result.clone();
    admission.complete(token, result).map_err(|error| {
        ServiceError::rejected_completion(
            FailureClass::Admission,
            reported.clone(),
            error.to_string(),
        )
    })?;
    if matches!(reported.disposition, CompletionDisposition::Success) {
        Ok(bytes)
    } else {
        Err(ServiceError::store_completion(class, reported))
    }
}

fn flush_member<S: RandomAccessStore>(
    store: &mut S,
    admission: &mut OperationAdmission,
    token: OperationSlotToken,
    child: dwv_store::ChildOperationId,
    through: StoreWriteWatermark,
) -> Result<PersistenceEvidence, ServiceError> {
    let result = store.flush(child, through);
    let reported = result.clone();
    let persistence = result.persistence;
    admission.complete(token, result).map_err(|error| {
        ServiceError::rejected_completion(
            FailureClass::Admission,
            reported.clone(),
            error.to_string(),
        )
    })?;
    if matches!(reported.disposition, CompletionDisposition::Success)
        && reported.persistence.is_durable()
    {
        Ok(persistence)
    } else {
        Err(ServiceError::store_completion(
            FailureClass::Fence,
            reported,
        ))
    }
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
        ResourceLimits, StoreCapabilities, StoreCompletion, StoreError, StoreFenceRef,
        StoreIncarnationId,
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
    }

    impl FakeStore {
        fn new(id: StoreId, epoch: TopologyEpoch, read: FakeRead) -> Self {
            Self {
                id,
                epoch,
                bytes: vec![0; LENGTH as usize],
                read,
                write: FakeEffect::Success,
                flush: FakeEffect::Success,
                watermark: StoreWriteWatermark(0),
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
        assert_eq!(stale.admission_usage().operation_slots, 0);
        assert_eq!(stale.admission_usage().backend_submissions, 0);

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
            assert_eq!(service.admission_usage().operation_slots, 0);
            assert_eq!(service.admission_usage().backend_submissions, 0);
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
        assert_eq!(service.admission_usage().operation_slots, 0);
        assert_eq!(service.admission_usage().backend_submissions, 0);

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
        assert_eq!(service.admission_usage().operation_slots, 0);
        assert_eq!(service.admission_usage().backend_submissions, 0);
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
    fn failed_child_admission_reconciles_registered_children_before_reclaim() {
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
        assert_eq!(service.state(), ServiceState::Recovering);
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
}
