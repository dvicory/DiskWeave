use crate::{
    admission::{AdmissionConfig, OperationAdmission, completion, slot_error},
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
    BLAKE3_256_PROFILE, ChecksumAuthority, ChecksumExtent, ChecksumRecord, ChecksumSetGeneration,
    ChecksumTarget, DIRTY_REGION_BYTES, FenceCertificate, IntegrityExtentId, InvalidationTarget,
    RecoveryGeneration, RecoveryMutation, RecoveryStateStore, RecoveryStoreHealth, RecoveryTxn,
    RegionId, dirty_regions_for_range,
};
use dwv_store::{
    CompletionDisposition, FenceDomain, OperationSlotToken, PersistenceEvidence, StoreId,
    StoreWriteWatermark, WriteIntent,
};
use dwv_store_file::{FileStore, IdentityComparison};
use dwv_transaction_ref::{
    ActionResult, CommittedRecoveryGeneration, ComputationResult, FenceEvidence, IntentRequirement,
    ParityComputationPlan, ParityRange, PlannedRead, PlannedWrite, RangeGuardToken,
    SemanticIoResult, StoreWatermark, TransactionLimits, TransactionMachine, TransactionPlan,
};
/// dwv:req req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch
pub struct MemberBinding {
    slot_id: SlotId,
    role: MemberRole,
    coding_position: CodingPosition,
    assignment_instance: AssignmentInstanceId,
    assignment_generation: AssignmentGeneration,
    topology_epoch: TopologyEpoch,
    store_id: StoreId,
    store: FileStore,
}

impl MemberBinding {
    pub fn new(
        assignment: &TopologyAssignment,
        topology_epoch: TopologyEpoch,
        store_id: StoreId,
        store: FileStore,
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

pub struct HealthyPortableService<R: RecoveryStateStore> {
    topology: TopologySnapshot,
    members: Vec<MemberBinding>,
    recovery: R,
    checksums: ChecksumAuthority,
    admission: OperationAdmission,
    config: ServiceConfig,
    state: ServiceState,
}

impl<R: RecoveryStateStore> HealthyPortableService<R> {
    /// dwv:req req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe
    pub fn open(
        topology: TopologySnapshot,
        members: Vec<MemberBinding>,
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
        let checksums = checksum_authority(&topology, generation)?;
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
            let mut all_durable = true;
            for (member, child) in self.members.iter_mut().zip(&flush_children) {
                let through = member
                    .store
                    .highest_accepted_watermark()
                    .unwrap_or(StoreWriteWatermark(0));
                let completion = member.store.flush_file(*child, through);
                all_durable &= matches!(completion.disposition, CompletionDisposition::Success)
                    && completion.persistence.is_durable();
                self.admission
                    .complete(token, completion)
                    .map_err(slot_error)?;
            }
            if !all_durable {
                self.state = ServiceState::Recovering;
                return Err(ServiceError::io(
                    FailureClass::Fence,
                    "flush lacks covering host fence evidence",
                ));
            }
            Ok(OperationEvidence {
                request,
                completion: CompletionEvidence {
                    requested: ByteRange::empty(),
                    completed: 0,
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
        .with_intent(IntentRequirement::FirstWrite)
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
        let intent = self
            .checksums
            .invalidate_with_intent(&mut self.recovery, target)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        machine
            .apply(ActionResult::RecoveryIntentCommitted(intent.clone()))
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
                    .map(|region| (region, intent.committed_generation))
                    .collect(),
            ),
            |certificate, extent| {
                certificate.with_integrity_extent(extent, intent.committed_generation)
            },
        );
        machine
            .apply(ActionResult::FlushSetComplete(FenceEvidence::durable(
                certificate.clone(),
            )))
            .map_err(|error| ServiceError::io(FailureClass::Fence, error.to_string()))?;
        let current = self
            .recovery
            .load_assembly_snapshot()
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        if current.generation != intent.committed_generation {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "recovery generation changed during write",
            ));
        }
        let mut checkpoint = RecoveryTxn::new(current.generation, self.topology.topology_epoch());
        checkpoint.push(RecoveryMutation::RecordHomeFence { fence: certificate });
        for region in regions {
            checkpoint.push(RecoveryMutation::MarkRegionClean {
                region,
                through_generation: intent.committed_generation,
            });
        }
        let committed = self
            .recovery
            .commit_durable(checkpoint)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        self.checksums.recovery_generation = committed;
        machine
            .apply(ActionResult::CheckpointCommitted(
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
                completed: request.range.length,
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
            match member
                .store
                .identity_is_current()
                .map_err(|error| ServiceError::io(FailureClass::Identity, error.to_string()))?
            {
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

fn read_child(
    store: &mut FileStore,
    admission: &mut OperationAdmission,
    token: OperationSlotToken,
    child: dwv_store::ChildOperationId,
    range: ByteRange,
    class: FailureClass,
) -> Result<Vec<u8>, ServiceError> {
    match store.read_bytes(range) {
        Ok(bytes) => {
            let result = completion(
                child,
                range,
                range.length,
                CompletionDisposition::Success,
                PersistenceEvidence::VolatileOrUnknown,
            );
            admission.complete(token, result).map_err(slot_error)?;
            Ok(bytes)
        }
        Err(error) => {
            let result = completion(
                child,
                range,
                0,
                CompletionDisposition::Failed(dwv_store::StoreError::BackendFailure { code: 5 }),
                PersistenceEvidence::VolatileOrUnknown,
            );
            admission.complete(token, result).map_err(slot_error)?;
            Err(ServiceError::io(class, error.to_string()))
        }
    }
}

fn flush_member(
    store: &mut FileStore,
    admission: &mut OperationAdmission,
    token: OperationSlotToken,
    child: dwv_store::ChildOperationId,
    through: StoreWriteWatermark,
) -> Result<PersistenceEvidence, ServiceError> {
    let result = store.flush_file(child, through);
    let persistence = result.persistence;
    admission.complete(token, result).map_err(slot_error)?;
    Ok(persistence)
}

/// dwv:req req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments
fn validate_assembly(
    topology: &TopologySnapshot,
    members: &[MemberBinding],
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
        let report = member.store.capabilities_report();
        if report.capabilities.logical_length
            != dwv_store::Evidence::Known(geometry.protected_length())
            || report.capabilities.logical_block_size
                != dwv_store::Evidence::Known(geometry.logical_block_size())
            || !report.capabilities.read.is_supported()
            || !report.capabilities.write.is_supported()
            || !report.capabilities.durable_flush.is_supported()
        {
            return Err(ServiceError::invalid(
                FailureClass::Capability,
                "member capabilities are incomplete for portable healthy I/O",
            ));
        }
        identities.push(report.identity);
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

fn validate_request(
    topology: &TopologySnapshot,
    members: &[MemberBinding],
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

fn member_index_for_assignment(
    members: &[MemberBinding],
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
    use dwv_core::{
        ArrayId, AssignmentGeneration, AssignmentInstanceId, BufferToken, CodingPosition,
        CodingProfile, FrontendId, OrderingIntent, ProtectedGeometry, RequestId, SlotId,
        SubmissionSequence, TopologyAssignment,
    };
    use dwv_recovery::IntentCommit;
    use dwv_recovery::MemoryRecoveryStore;
    use dwv_recovery::{Blake3Provider, DigestProvider};
    use dwv_store_file::{ControlProjection, FileStoreConfig, FileSyncMode};
    use dwv_verify::{
        ChecksumEvidence, DigestEvidence, VerificationIdentity, VerificationStore,
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
                Err(VerificationStoreError::new(format!(
                    "repair write did not complete: {:?}",
                    completion.disposition
                )))
            }
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

    fn bindings(
        topology: &TopologySnapshot,
        mut open: impl FnMut(u64) -> FileStore,
    ) -> Vec<MemberBinding> {
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

    fn reopened_bindings(root: &Path, topology: &TopologySnapshot) -> Vec<MemberBinding> {
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

    fn fixture() -> (PathBuf, HealthyPortableService<MemoryRecoveryStore>) {
        fixture_with_config(ServiceConfig::default())
    }

    fn fixture_with_config(
        config: ServiceConfig,
    ) -> (PathBuf, HealthyPortableService<MemoryRecoveryStore>) {
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
        assert_eq!(evidence.completion.completed, BLOCK as u64);
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
        assert_eq!(read_evidence.completion.completed, BLOCK as u64);
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
                assert_eq!(bytes.len(), (LENGTH - BLOCK as u64) as usize);
                assert_eq!(evidence.requested, range);
                assert_eq!(evidence.completed, LENGTH - BLOCK as u64);
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

    #[test]
    fn stable_slots_ignore_topology_coding_and_member_collection_order() {
        let (root, service) = fixture();
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
        let _ = IntentCommit::new(
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
}
