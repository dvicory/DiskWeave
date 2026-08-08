use crate::{
    admission::{AdmissionConfig, OperationAdmission, completion, slot_error},
    evidence::{CompletionEvidence, OperationEvidence, PersistenceClaim},
    failure::{FailureClass, ServiceError},
    lifecycle::ServiceState,
    range::split_range,
    read::read_member,
    request::{PortableRequest, RequestOperation},
    write::{update_parity, write_member},
};
use dwv_codec::Geometry as CodecGeometry;
use dwv_core::{ByteRange, DurabilityIntent, MemberRole, TopologyEpoch, TopologySnapshot};
use dwv_recovery::{
    BLAKE3_256_PROFILE, ChecksumAuthority, ChecksumExtent, ChecksumRecord, ChecksumSetGeneration,
    ChecksumTarget, FenceCertificate, IntegrityExtentId, InvalidationTarget, RecoveryGeneration,
    RecoveryMutation, RecoveryStateStore, RecoveryStoreHealth, RecoveryTxn, RegionId,
};
use dwv_store::{
    CompletionDisposition, FenceDomain, OperationId, OperationSlotToken, PersistenceEvidence,
    StoreId, StoreWriteWatermark, WriteIntent,
};
use dwv_store_file::{FileStore, IdentityComparison};
use dwv_transaction_ref::{
    ActionResult, CommittedRecoveryGeneration, ComputationResult, FenceEvidence, IntentRequirement,
    ParityComputationPlan, ParityRange, PlannedRead, PlannedWrite, RangeGuardToken,
    SemanticIoResult, StoreWatermark, TransactionLimits, TransactionMachine, TransactionPlan,
};

pub struct MemberStore {
    pub id: StoreId,
    pub store: FileStore,
}

impl MemberStore {
    pub fn new(id: StoreId, store: FileStore) -> Self {
        Self { id, store }
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
    data: Vec<MemberStore>,
    parity: MemberStore,
    recovery: R,
    checksums: ChecksumAuthority,
    admission: OperationAdmission,
    config: ServiceConfig,
    state: ServiceState,
}

impl<R: RecoveryStateStore> HealthyPortableService<R> {
    pub fn open(
        topology: TopologySnapshot,
        data: Vec<MemberStore>,
        parity: MemberStore,
        recovery: R,
        config: ServiceConfig,
    ) -> Result<Self, ServiceError> {
        validate_assembly(&topology, &data, &parity)?;
        if config.maximum_transfer == Some(0) {
            return Err(ServiceError::invalid(
                FailureClass::Capability,
                "maximum transfer cannot be zero",
            ));
        }
        let health = recovery.verify_integrity();
        let (state, generation) = if health != RecoveryStoreHealth::Healthy {
            (
                ServiceState::Blocked(FailureClass::Recovery),
                RecoveryGeneration::ZERO,
            )
        } else {
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
            (state, snapshot.generation)
        };
        let checksums = checksum_authority(&topology, generation)?;
        Ok(Self {
            topology,
            data,
            parity,
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

    pub fn read(
        &mut self,
        request: PortableRequest,
    ) -> Result<(Vec<u8>, OperationEvidence), ServiceError> {
        self.state.require_reads()?;
        let data_slot = validate_request(&self.topology, &request)?;
        if !matches!(request.operation, RequestOperation::Read { .. }) {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "read endpoint requires a read request",
            ));
        }
        self.ensure_identities()?;
        let plan = split_range(
            request.range,
            self.topology.geometry(),
            self.maximum_transfer(),
        )?;
        let token = self.reserve_with_buffers(&request, 1)?;
        let result = read_member(
            &mut self.data[data_slot].store,
            &mut self.admission,
            token,
            &plan,
        );
        match result {
            Ok((bytes, completion)) => {
                self.finish(token, false)?;
                Ok((
                    bytes,
                    OperationEvidence {
                        completion,
                        trace: empty_trace(
                            self.topology.topology_epoch(),
                            self.recovery_generation(),
                        )?,
                    },
                ))
            }
            Err(error) => {
                let _ = self.finish(token, true);
                Err(error)
            }
        }
    }

    pub fn write(&mut self, request: PortableRequest) -> Result<OperationEvidence, ServiceError> {
        self.state.require_writes()?;
        let data_slot = validate_request(&self.topology, &request)?;
        let RequestOperation::Write { bytes, .. } = &request.operation else {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "write endpoint requires a write request",
            ));
        };
        if bytes.len() as u64 != request.range.length {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "write buffer does not exactly cover the request",
            ));
        }
        if matches!(
            request.durability,
            DurabilityIntent::Fua | DurabilityIntent::ExplicitFlush
        ) {
            return Err(ServiceError::invalid(
                FailureClass::Capability,
                "portable file stores do not expose FUA or write-embedded flush",
            ));
        }
        self.ensure_identities()?;
        let plan = split_range(
            request.range,
            self.topology.geometry(),
            self.maximum_transfer(),
        )?;
        let token = self.reserve_with_buffers(&request, 2)?;
        let result = self.execute_write(data_slot, &request, bytes, &plan, token);
        match result {
            Ok(evidence) => {
                self.finish(token, false)?;
                Ok(evidence)
            }
            Err(error) => {
                self.state = ServiceState::Recovering;
                let _ = self.finish(token, true);
                Err(error)
            }
        }
    }

    pub fn flush(&mut self, request: PortableRequest) -> Result<OperationEvidence, ServiceError> {
        self.state.require_reads()?;
        if !matches!(request.operation, RequestOperation::Flush) || !request.range.is_empty() {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "flush request must have an empty range",
            ));
        }
        if request.topology_epoch != self.topology.topology_epoch() {
            return Err(ServiceError::invalid(
                FailureClass::StaleTopology,
                "flush captured a stale topology epoch",
            ));
        }
        self.ensure_identities()?;
        let token = self.reserve(&request)?;
        let flush_ranges = vec![ByteRange::empty(); self.data.len() + 1];
        let flush_children = self
            .admission
            .children(token, &flush_ranges)
            .map_err(slot_error)?;
        self.admission
            .submit_all(token, flush_children.len())
            .map_err(slot_error)?;
        let mut all_durable = true;
        for (member, child) in self.data.iter_mut().zip(&flush_children) {
            let completion = member
                .store
                .flush_file(*child, StoreWriteWatermark(u64::MAX));
            all_durable &= matches!(completion.disposition, CompletionDisposition::Success)
                && completion.persistence.is_durable();
            self.admission
                .complete(token, completion)
                .map_err(slot_error)?;
        }
        let parity_completion = self.parity.store.flush_file(
            *flush_children.last().expect("parity child exists"),
            StoreWriteWatermark(u64::MAX),
        );
        all_durable &= matches!(
            parity_completion.disposition,
            CompletionDisposition::Success
        ) && parity_completion.persistence.is_durable();
        self.admission
            .complete(token, parity_completion)
            .map_err(slot_error)?;
        if !all_durable {
            self.state = ServiceState::Recovering;
            let _ = self.finish(token, true);
            return Err(ServiceError::io(
                FailureClass::Fence,
                "flush lacks covering host fence evidence",
            ));
        }
        self.finish(token, false)?;
        Ok(OperationEvidence {
            completion: CompletionEvidence {
                requested: ByteRange::empty(),
                completed: 0,
                disposition: CompletionDisposition::Success,
                persistence: PersistenceClaim::HostFenceOnly,
            },
            trace: empty_trace(self.topology.topology_epoch(), self.recovery_generation())?,
        })
    }

    pub fn abandon(&mut self, token: OperationSlotToken) -> Result<(), ServiceError> {
        self.admission.abandon(token).map_err(slot_error)
    }

    fn execute_write(
        &mut self,
        data_slot: usize,
        request: &PortableRequest,
        bytes: &[u8],
        plan: &crate::range::RangePlan,
        token: OperationSlotToken,
    ) -> Result<OperationEvidence, ServiceError> {
        let generation = self.recovery_generation();
        if generation != self.checksums.recovery_generation {
            return Err(ServiceError::io(
                FailureClass::Recovery,
                "checksum authority generation is stale",
            ));
        }
        let region = region_for(data_slot, request.range);
        let checksum_extents = self.checksum_extents_for(data_slot, request.range)?;
        let stores = vec![self.data[data_slot].id, self.parity.id];
        let mut tx_plan = TransactionPlan::new(
            self.topology.topology_epoch(),
            generation,
            self.config.fence_domain,
        )
        .with_intent(IntentRequirement::FirstWrite)
        .with_ranges(
            plan.ranges
                .iter()
                .flat_map(|range| {
                    [
                        ParityRange::new(region, self.data[data_slot].id, *range),
                        ParityRange::new(region, self.parity.id, *range),
                    ]
                })
                .collect(),
        )
        .with_dirty_regions(vec![region])
        .with_checksum_extents(checksum_extents.clone())
        .with_reads(
            plan.ranges
                .iter()
                .flat_map(|range| {
                    [
                        PlannedRead::new(self.data[data_slot].id, *range),
                        PlannedRead::new(self.parity.id, *range),
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
                        PlannedWrite::new(self.data[data_slot].id, *range),
                        PlannedWrite::new(self.parity.id, *range),
                    ]
                })
                .collect(),
        )
        .with_stores(stores.clone())
        .with_watermarks(
            stores
                .iter()
                .map(|store| StoreWatermark::new(*store, StoreWriteWatermark(u64::MAX)))
                .collect(),
        );
        tx_plan.limits = TransactionLimits::default();
        let mut machine = TransactionMachine::new(tx_plan)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        let target = InvalidationTarget::new(vec![region], checksum_extents.clone());
        let intent = self
            .checksums
            .invalidate_with_intent(&mut self.recovery, target)
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;
        machine
            .apply(ActionResult::RecoveryIntentCommitted(intent.clone()))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;

        let mut computed = Vec::with_capacity(plan.ranges.len());
        let codec_geometry = CodecGeometry::new(
            vec![self.topology.geometry().protected_length(); self.data.len()],
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
                &mut self.data[data_slot].store,
                &mut self.admission,
                token,
                read_children[index].0,
                *range,
                FailureClass::StoreRead,
            )?;
            let old_parity = read_child(
                &mut self.parity.store,
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
                data_slot,
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
        let mut fences = Vec::new();
        for (index, (range, new_data, new_parity)) in computed.iter().enumerate() {
            fences.push(write_member(
                &mut self.data[data_slot].store,
                &mut self.admission,
                token,
                data_write_children[index],
                *range,
                new_data,
                write_intent,
            )?);
            fences.push(write_member(
                &mut self.parity.store,
                &mut self.admission,
                token,
                parity_write_children[index],
                *range,
                new_parity,
                write_intent,
            )?);
        }
        machine
            .apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))
            .map_err(|error| ServiceError::io(FailureClass::Recovery, error.to_string()))?;

        let data_id = self.data[data_slot].id;
        let parity_id = self.parity.id;
        let data_fence = flush_member(
            &mut self.data[data_slot].store,
            &mut self.admission,
            token,
            flush_children[0],
            data_id,
        )?;
        let parity_fence = flush_member(
            &mut self.parity.store,
            &mut self.admission,
            token,
            flush_children[1],
            parity_id,
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
                self.config.fence_domain,
                store_fences,
                vec![(region, intent.committed_generation)],
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
        checkpoint.push(RecoveryMutation::MarkRegionClean {
            region,
            through_generation: intent.committed_generation,
        });
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
            completion: CompletionEvidence {
                requested: request.range,
                completed: request.range.length,
                disposition: CompletionDisposition::Success,
                persistence: PersistenceClaim::HostFenceOnly,
            },
            trace: machine.trace().clone(),
        })
    }

    fn reserve(&mut self, request: &PortableRequest) -> Result<OperationSlotToken, ServiceError> {
        self.admission
            .reserve(OperationId(request.request_id.0), request.topology_epoch)
            .map_err(slot_error)
    }

    fn reserve_with_buffers(
        &mut self,
        request: &PortableRequest,
        count: usize,
    ) -> Result<OperationSlotToken, ServiceError> {
        let token = self.reserve(request)?;
        for index in 0..count {
            let buffer_index = match u32::try_from(index) {
                Ok(index) => index,
                Err(error) => {
                    let service_error = ServiceError::io(
                        FailureClass::Admission,
                        format!("buffer index does not fit: {error}"),
                    );
                    let _ = self.finish(token, true);
                    return Err(service_error);
                }
            };
            if let Err(error) = self.admission.attach_buffer(token, buffer_index) {
                let service_error = slot_error(error);
                let _ = self.finish(token, true);
                return Err(service_error);
            }
        }
        Ok(token)
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

    fn recovery_generation(&self) -> RecoveryGeneration {
        self.recovery
            .load_assembly_snapshot()
            .map(|snapshot| snapshot.generation)
            .unwrap_or(RecoveryGeneration::ZERO)
    }

    fn checksum_extents_for(
        &self,
        data_slot: usize,
        range: ByteRange,
    ) -> Result<Vec<IntegrityExtentId>, ServiceError> {
        let per_member = checksum_extent_count(self.topology.geometry().protected_length())?;
        let first_extent = range.offset / BLAKE3_256_PROFILE.extent_size;
        let last_extent = (range.end() - 1) / BLAKE3_256_PROFILE.extent_size;
        let member_indices = [data_slot, self.data.len()];
        let mut extents = Vec::new();
        for member_index in member_indices {
            let member_base = (member_index as u64)
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
        for member in self.data.iter().chain(std::iter::once(&self.parity)) {
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
    for (member_index, assignment) in topology.assignments().iter().enumerate() {
        let target = match assignment.role() {
            MemberRole::Data => ChecksumTarget::data(assignment.slot_id()),
            MemberRole::Parity => ChecksumTarget::parity(assignment.coding_position()),
        };
        let first_id = IntegrityExtentId(
            (member_index as u64)
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
    _id: StoreId,
) -> Result<PersistenceEvidence, ServiceError> {
    let result = store.flush_file(child, StoreWriteWatermark(u64::MAX));
    let persistence = result.persistence;
    admission.complete(token, result).map_err(slot_error)?;
    Ok(persistence)
}

fn validate_assembly(
    topology: &TopologySnapshot,
    data: &[MemberStore],
    parity: &MemberStore,
) -> Result<(), ServiceError> {
    topology
        .validate()
        .map_err(|error| ServiceError::invalid(FailureClass::InvalidRequest, error.to_string()))?;
    if topology.profile().parity_slots() != 1
        || data.len() != usize::from(topology.profile().data_slots())
    {
        return Err(ServiceError::invalid(
            FailureClass::Capability,
            "healthy portable service requires exactly one parity member and the recorded data count",
        ));
    }
    let geometry = topology.geometry();
    let mut identities = Vec::new();
    for member in data.iter().chain(std::iter::once(parity)) {
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
    for (index, identity) in identities.iter().enumerate() {
        for other in identities.iter().skip(index + 1) {
            if identity.compare(other) != dwv_store::IdentityComparison::Changed {
                return Err(ServiceError::invalid(
                    FailureClass::Alias,
                    "data and parity members alias or have ambiguous identity",
                ));
            }
        }
    }
    for (index, assignment) in topology.assignments().iter().enumerate() {
        let expected_role = if index < data.len() {
            MemberRole::Data
        } else {
            MemberRole::Parity
        };
        if assignment.role() != expected_role {
            return Err(ServiceError::invalid(
                FailureClass::InvalidRequest,
                "topology assignment order does not match portable member order",
            ));
        }
    }
    Ok(())
}

fn validate_request(
    topology: &TopologySnapshot,
    request: &PortableRequest,
) -> Result<usize, ServiceError> {
    if request.topology_epoch != topology.topology_epoch() {
        return Err(ServiceError::invalid(
            FailureClass::StaleTopology,
            "request topology epoch is stale",
        ));
    }
    match request.operation {
        RequestOperation::Read { data_slot } | RequestOperation::Write { data_slot, .. }
            if data_slot < usize::from(topology.profile().data_slots()) =>
        {
            Ok(data_slot)
        }
        RequestOperation::Flush if request.range.is_empty() => Ok(0),
        _ => Err(ServiceError::invalid(
            FailureClass::InvalidRequest,
            "request addresses an unavailable data slot or has an invalid range",
        )),
    }
}

fn region_for(data_slot: usize, range: ByteRange) -> RegionId {
    RegionId((u64::try_from(data_slot).unwrap_or(u64::MAX) << 32) ^ (range.offset / 4096))
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
        ArrayId, AssignmentGeneration, AssignmentInstanceId, CodingPosition, CodingProfile,
        ProtectedGeometry, RequestId, SlotId, TopologyAssignment,
    };
    use dwv_recovery::IntentCommit;
    use dwv_recovery::MemoryRecoveryStore;
    use dwv_store_file::{ControlProjection, FileStoreConfig, FileSyncMode};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const LENGTH: u64 = 4096;
    const BLOCK: u32 = 512;
    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

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
        let service = HealthyPortableService::open(
            topology(epoch),
            vec![
                MemberStore::new(StoreId(1), open(1)),
                MemberStore::new(StoreId(2), open(2)),
            ],
            MemberStore::new(StoreId(3), open(3)),
            MemoryRecoveryStore::new(epoch),
            config,
        )
        .unwrap();
        (root, service)
    }

    #[test]
    fn write_updates_data_and_single_xor_parity_then_reads_exact_bytes() {
        let (root, mut service) = fixture();
        let bytes = vec![0x5a; BLOCK as usize];
        let range = ByteRange::new(0, BLOCK as u64).unwrap();
        let evidence = service
            .write(PortableRequest::write(
                RequestId(1),
                TopologyEpoch(4),
                0,
                range,
                bytes.clone(),
            ))
            .unwrap();
        assert_eq!(evidence.completion.completed, BLOCK as u64);
        assert_eq!(
            evidence.completion.persistence,
            PersistenceClaim::HostFenceOnly
        );
        assert_eq!(service.state(), ServiceState::Serving);
        let (read, read_evidence) = service
            .read(PortableRequest::read(
                RequestId(2),
                TopologyEpoch(4),
                0,
                range,
            ))
            .unwrap();
        assert_eq!(read, bytes);
        assert_eq!(read_evidence.completion.completed, BLOCK as u64);
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
            .write(PortableRequest::write(
                RequestId(4),
                TopologyEpoch(4),
                1,
                range,
                vec![0xa5; BLOCK as usize],
            ))
            .unwrap();
        let parity = fs::read(root.join("member-3.raw")).unwrap();
        assert_eq!(&parity[..BLOCK as usize], &[0; BLOCK as usize]);
        assert_eq!(
            &parity[BLOCK as usize..(2 * BLOCK) as usize],
            &[0xa5; BLOCK as usize]
        );
        let (read, _) = service
            .read(PortableRequest::read(
                RequestId(5),
                TopologyEpoch(4),
                1,
                range,
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
            .write(PortableRequest::write(
                RequestId(10),
                TopologyEpoch(4),
                0,
                range,
                vec![0x33; LENGTH as usize],
            ))
            .unwrap();
        service
            .write(PortableRequest::write(
                RequestId(11),
                TopologyEpoch(4),
                1,
                range,
                vec![0x77; LENGTH as usize],
            ))
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
                .write(PortableRequest::write(
                    RequestId(100 + iteration),
                    TopologyEpoch(4),
                    slot,
                    range,
                    bytes.clone(),
                ))
                .unwrap();
            let (read, _) = service
                .read(PortableRequest::read(
                    RequestId(200 + iteration),
                    TopologyEpoch(4),
                    slot,
                    range,
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
            .write(PortableRequest::write(
                RequestId(300),
                epoch,
                0,
                range,
                vec![0x66; BLOCK as usize],
            ))
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
        let mut reopened = HealthyPortableService::open(
            topology(epoch),
            vec![
                MemberStore::new(StoreId(1), open(1)),
                MemberStore::new(StoreId(2), open(2)),
            ],
            MemberStore::new(StoreId(3), open(3)),
            MemoryRecoveryStore::new(epoch),
            ServiceConfig::default(),
        )
        .unwrap();
        let (read, _) = reopened
            .read(PortableRequest::read(RequestId(301), epoch, 0, range))
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
            .flush(PortableRequest::flush(RequestId(12), TopologyEpoch(4)))
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
    fn explicit_flush_intent_is_rejected_on_write_endpoint() {
        let (root, mut service) = fixture();
        let range = ByteRange::new(0, BLOCK as u64).unwrap();
        let result = service.write(
            PortableRequest::write(
                RequestId(13),
                TopologyEpoch(4),
                0,
                range,
                vec![0x11; BLOCK as usize],
            )
            .with_durability(DurabilityIntent::ExplicitFlush),
        );
        assert!(matches!(
            result,
            Err(ServiceError::Invalid {
                class: FailureClass::Capability,
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
        let result = service.read(PortableRequest::read(
            RequestId(14),
            TopologyEpoch(4),
            0,
            range,
        ));
        match result {
            Err(ServiceError::IncompleteRead { bytes, evidence }) => {
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
        let result = service.write(PortableRequest::write(
            RequestId(15),
            TopologyEpoch(4),
            0,
            range,
            vec![0x22; BLOCK as usize],
        ));
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
                limits: dwv_store::ResourceLimits::new(1, 1, 8, 1, 1, 1),
            },
            ..ServiceConfig::default()
        };
        let (root, mut service) = fixture_with_config(config);
        let range = ByteRange::new(0, BLOCK as u64).unwrap();
        let result = service.write(PortableRequest::write(
            RequestId(16),
            TopologyEpoch(4),
            0,
            range,
            vec![0x33; BLOCK as usize],
        ));
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
    fn assembly_rejects_data_parity_identity_alias() {
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
        let parity = open(parity_path, 3);
        let result = HealthyPortableService::open(
            topology(epoch),
            vec![
                MemberStore::new(StoreId(1), data0),
                MemberStore::new(StoreId(2), data1),
            ],
            MemberStore::new(StoreId(3), parity),
            MemoryRecoveryStore::new(epoch),
            ServiceConfig::default(),
        );
        assert!(matches!(
            result,
            Err(ServiceError::Invalid {
                class: FailureClass::Alias,
                ..
            })
        ));
        drop(result);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_epoch_is_rejected_before_payload_mutation() {
        let (root, mut service) = fixture();
        let range = ByteRange::new(0, BLOCK as u64).unwrap();
        let result = service.write(PortableRequest::write(
            RequestId(3),
            TopologyEpoch(5),
            0,
            range,
            vec![1; BLOCK as usize],
        ));
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
        let service = HealthyPortableService::open(
            topology(TopologyEpoch(4)),
            vec![
                MemberStore::new(StoreId(1), open(1)),
                MemberStore::new(StoreId(2), open(2)),
            ],
            MemberStore::new(StoreId(3), open(3)),
            recovery,
            ServiceConfig::default(),
        )
        .unwrap();
        assert_eq!(service.state(), ServiceState::Recovering);
        drop(service);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_recovery_opens_blocked_without_attempting_snapshot_load() {
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
        let mut service = HealthyPortableService::open(
            topology(epoch),
            vec![
                MemberStore::new(StoreId(1), open(1)),
                MemberStore::new(StoreId(2), open(2)),
            ],
            MemberStore::new(StoreId(3), open(3)),
            recovery,
            ServiceConfig::default(),
        )
        .unwrap();
        assert_eq!(
            service.state(),
            ServiceState::Blocked(FailureClass::Recovery)
        );
        let range = ByteRange::new(0, BLOCK as u64).unwrap();
        assert!(matches!(
            service.write(PortableRequest::write(
                RequestId(17),
                epoch,
                0,
                range,
                vec![0x44; BLOCK as usize],
            )),
            Err(ServiceError::Blocked(FailureClass::Recovery))
        ));
        drop(service);
        fs::remove_dir_all(root).unwrap();
    }
}
