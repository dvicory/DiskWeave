use crate::coded_authority::CodedGeometryAuthorityId;
use crate::{
    ChecksumProfile, CodedCaptureError, CodedClaimInput, IntegrityExtentId, InvalidationTarget,
    RegionId, ValidatedCodedCaptureScope, dirty_regions_for_range,
};
use dwv_core::{
    BlockOp, BlockRequest, ByteRange, CodedUnitId, MemberRole, TopologyAssignment, TopologySnapshot,
};
use dwv_store::OperationSlotToken;
use std::collections::BTreeSet;
/// Canonical mapper for per-write claims and future-inclusive capture scope.
#[derive(Clone)]
pub struct CodedGeometryOwner {
    authority: CodedGeometryAuthorityId,
    topology: TopologySnapshot,
    dirty_region_bytes: u64,
    checksum_profile: ChecksumProfile,
}

impl CodedGeometryOwner {
    pub fn new(
        topology: TopologySnapshot,
        dirty_region_bytes: u64,
        checksum_profile: ChecksumProfile,
    ) -> Result<Self, CodedCaptureError> {
        if dirty_region_bytes == 0
            || checksum_profile.extent_size == 0
            || topology.geometry().logical_block_size() == 0
        {
            return Err(CodedCaptureError::InvalidScopeGeometry);
        }
        Ok(Self {
            authority: CodedGeometryAuthorityId::new(),
            topology,
            dirty_region_bytes,
            checksum_profile,
        })
    }

    pub(crate) const fn authority(&self) -> CodedGeometryAuthorityId {
        self.authority
    }

    /// Issue one complete claim for one exact admitted mutation.
    pub fn claim_for_mutation(
        &self,
        operation: OperationSlotToken,
        request: BlockRequest,
        assignment: &TopologyAssignment,
        ranges: impl IntoIterator<Item = ByteRange>,
    ) -> Result<CodedClaimInput, CodedCaptureError> {
        let ranges = ranges.into_iter().collect::<Vec<_>>();
        if request.op != BlockOp::Write
            || request.topology_epoch != self.topology.topology_epoch()
            || request.slot_id != assignment.slot_id()
            || !self
                .topology
                .assignments()
                .iter()
                .any(|current| current == assignment)
        {
            return Err(CodedCaptureError::ClaimBindingInvalid);
        }
        let mut cursor = request.range.offset;
        for range in &ranges {
            if range.offset != cursor {
                return Err(CodedCaptureError::ClaimBindingInvalid);
            }
            cursor = range.end();
        }
        if cursor != request.range.end() {
            return Err(CodedCaptureError::ClaimBindingInvalid);
        }
        let units = self.map_units(ranges.iter().copied())?;
        let invalidation_target =
            self.invalidation_target(assignment.coding_position(), ranges.iter().copied())?;
        Ok(CodedClaimInput::issued(
            self.authority,
            operation,
            request,
            assignment.assignment_instance(),
            assignment.assignment_generation(),
            assignment.coding_position(),
            self.topology.profile(),
            self.checksum_profile.id,
            ranges,
            invalidation_target,
            units,
        ))
    }

    /// Validate that an opaque claim still belongs to this exact geometry owner.
    pub fn validates_claim_for_mutation(
        &self,
        operation: OperationSlotToken,
        request: &BlockRequest,
        assignment: &TopologyAssignment,
        claim: &CodedClaimInput,
    ) -> bool {
        if request.op != BlockOp::Write
            || request.topology_epoch != self.topology.topology_epoch()
            || request.slot_id != assignment.slot_id()
            || !self
                .topology
                .assignments()
                .iter()
                .any(|current| current == assignment)
        {
            return false;
        }
        let Ok(units) = self.map_units(claim.normalized_ranges().iter().copied()) else {
            return false;
        };
        let Ok(invalidation_target) = self.invalidation_target(
            assignment.coding_position(),
            claim.normalized_ranges().iter().copied(),
        ) else {
            return false;
        };
        claim.matches_owner_context(
            self.authority,
            operation,
            request,
            assignment.assignment_instance(),
            assignment.assignment_generation(),
            assignment.coding_position(),
            self.topology.profile(),
            self.checksum_profile.id,
            claim.normalized_ranges(),
            &invalidation_target,
            &units,
        )
    }

    fn invalidation_target(
        &self,
        data_position: dwv_core::CodingPosition,
        ranges: impl IntoIterator<Item = ByteRange>,
    ) -> Result<InvalidationTarget, CodedCaptureError> {
        let parity_position = self
            .topology
            .assignments()
            .iter()
            .find(|assignment| assignment.role() == MemberRole::Parity)
            .map(TopologyAssignment::coding_position)
            .ok_or(CodedCaptureError::ClaimBindingInvalid)?;
        let per_member = self
            .topology
            .geometry()
            .protected_length()
            .div_ceil(self.checksum_profile.extent_size);
        let mut regions = BTreeSet::new();
        let mut checksum_extents = BTreeSet::new();
        for range in ranges {
            regions.extend(
                dirty_regions_for_range(u32::from(data_position.0), range, self.dirty_region_bytes)
                    .map_err(|_| CodedCaptureError::ClaimBindingInvalid)?,
            );
            let first_extent = range.offset / self.checksum_profile.extent_size;
            let last_extent = (range.end() - 1) / self.checksum_profile.extent_size;
            for position in [data_position, parity_position] {
                let member_base = u64::from(position.0)
                    .checked_mul(per_member)
                    .ok_or(CodedCaptureError::ScopeArithmeticOverflow)?;
                for extent in first_extent..=last_extent {
                    checksum_extents.insert(IntegrityExtentId(
                        member_base
                            .checked_add(extent)
                            .ok_or(CodedCaptureError::ScopeArithmeticOverflow)?,
                    ));
                }
            }
        }
        Ok(InvalidationTarget::new(
            regions.into_iter().collect(),
            checksum_extents.into_iter().collect(),
        ))
    }

    fn map_units(
        &self,
        ranges: impl IntoIterator<Item = ByteRange>,
    ) -> Result<BTreeSet<CodedUnitId>, CodedCaptureError> {
        let logical_block = u64::from(self.topology.geometry().logical_block_size());
        let protected_length = self.topology.geometry().protected_length();
        let mut units = BTreeSet::new();
        for range in ranges {
            let end = range
                .offset
                .checked_add(range.length)
                .ok_or(CodedCaptureError::ScopeArithmeticOverflow)?;
            if range.is_empty()
                || !range.offset.is_multiple_of(logical_block)
                || !range.length.is_multiple_of(logical_block)
                || end > protected_length
            {
                return Err(CodedCaptureError::ScopeIdentityOutOfRange);
            }
            let first = range.offset / logical_block;
            let last = (end - 1) / logical_block;
            units.extend((first..=last).map(CodedUnitId));
        }
        if units.is_empty() {
            return Err(CodedCaptureError::EmptyScope);
        }
        Ok(units)
    }

    /// Map selected dirty/checksum identities through the same coded geometry.
    pub fn capture_scope(
        &self,
        dirty_regions: impl IntoIterator<Item = RegionId>,
        checksum_extents: impl IntoIterator<Item = IntegrityExtentId>,
    ) -> Result<ValidatedCodedCaptureScope, CodedCaptureError> {
        let dirty_regions = dirty_regions.into_iter().collect::<BTreeSet<_>>();
        let checksum_extents = checksum_extents.into_iter().collect::<BTreeSet<_>>();
        let protected_length = self.topology.geometry().protected_length();
        let logical_block = u64::from(self.topology.geometry().logical_block_size());
        let checksum_extents_per_member =
            protected_length.div_ceil(self.checksum_profile.extent_size);
        let coding_positions = self
            .topology
            .assignments()
            .iter()
            .map(|assignment| u32::from(assignment.coding_position().0))
            .collect::<BTreeSet<_>>();
        let mut units = BTreeSet::new();
        let mut add_range = |offset: u64, length: u64| -> Result<(), CodedCaptureError> {
            let end = offset
                .checked_add(length)
                .map(|end| end.min(protected_length))
                .ok_or(CodedCaptureError::ScopeArithmeticOverflow)?;
            if offset >= end {
                return Err(CodedCaptureError::ScopeIdentityOutOfRange);
            }
            let first = offset / logical_block;
            let last = (end - 1) / logical_block;
            units.extend((first..=last).map(CodedUnitId));
            Ok(())
        };
        for region in &dirty_regions {
            let coding_position = u32::try_from(region.0 >> 32)
                .map_err(|_| CodedCaptureError::ScopeIdentityOutOfRange)?;
            if !coding_positions.contains(&coding_position) {
                return Err(CodedCaptureError::ScopeIdentityOutOfRange);
            }
            let region_index = region.0 & u64::from(u32::MAX);
            let offset = region_index
                .checked_mul(self.dirty_region_bytes)
                .ok_or(CodedCaptureError::ScopeArithmeticOverflow)?;
            add_range(offset, self.dirty_region_bytes)?;
        }
        for extent in &checksum_extents {
            if checksum_extents_per_member == 0 {
                return Err(CodedCaptureError::InvalidScopeGeometry);
            }
            let coding_position = extent.0 / checksum_extents_per_member;
            let coding_position = u32::try_from(coding_position)
                .map_err(|_| CodedCaptureError::ScopeIdentityOutOfRange)?;
            if !coding_positions.contains(&coding_position) {
                return Err(CodedCaptureError::ScopeIdentityOutOfRange);
            }
            let extent_index = extent.0 % checksum_extents_per_member;
            let offset = extent_index
                .checked_mul(self.checksum_profile.extent_size)
                .ok_or(CodedCaptureError::ScopeArithmeticOverflow)?;
            add_range(offset, self.checksum_profile.extent_size)?;
        }
        if units.is_empty() {
            return Err(CodedCaptureError::EmptyScope);
        }
        Ok(ValidatedCodedCaptureScope::validated(
            units,
            self.topology.clone(),
            dirty_regions,
            checksum_extents,
            self.dirty_region_bytes,
            self.checksum_profile,
        ))
    }
}
