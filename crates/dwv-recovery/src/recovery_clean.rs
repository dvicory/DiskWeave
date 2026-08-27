//! Generation-checked fence and recovery CLEAN decisions.

use crate::{
    FenceCertificate, IntegrityExtentId, RecoveryError, RecoveryGeneration, RecoverySnapshot,
    RegionId, TransitionError,
};
use dwv_core::{FenceDomain, TopologyEpoch};
use dwv_store::{StoreFenceRef, StoreId, StoreWriteWatermark};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequiredFence {
    pub store: StoreId,
    pub through: StoreWriteWatermark,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryCleanRequest {
    pub topology_epoch: TopologyEpoch,
    pub fence_domain: FenceDomain,
    pub generation: RecoveryGeneration,
    pub stores: Vec<RequiredFence>,
    pub regions: Vec<RegionId>,
    pub checksum_extents: Vec<IntegrityExtentId>,
}

impl RecoveryCleanRequest {
    pub fn new(
        topology_epoch: TopologyEpoch,
        fence_domain: FenceDomain,
        generation: RecoveryGeneration,
    ) -> Self {
        Self {
            topology_epoch,
            fence_domain,
            generation,
            stores: Vec::new(),
            regions: Vec::new(),
            checksum_extents: Vec::new(),
        }
    }

    pub fn with_store(mut self, store: StoreId, through: StoreWriteWatermark) -> Self {
        self.stores.push(RequiredFence { store, through });
        self
    }

    pub fn with_region(mut self, region: RegionId) -> Self {
        self.regions.push(region);
        self
    }

    pub fn with_checksum_extent(mut self, extent: IntegrityExtentId) -> Self {
        self.checksum_extents.push(extent);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryCleanRefusal {
    GenerationChanged,
    TopologyChanged,
    MissingStoreFence,
    MissingRegionCoverage,
    MissingIntegrityCoverage,
    VolatileFence,
    SessionStillDirty,
}

/// Recovery-owner proof that one exact CLEAN request was refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryCleanRefusalPermit {
    topology_epoch: TopologyEpoch,
    generation: RecoveryGeneration,
    regions: Vec<RegionId>,
    checksum_extents: Vec<IntegrityExtentId>,
}

impl RecoveryCleanRefusalPermit {
    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch
    }

    pub const fn generation(&self) -> RecoveryGeneration {
        self.generation
    }

    pub fn regions(&self) -> &[RegionId] {
        &self.regions
    }

    pub fn checksum_extents(&self) -> &[IntegrityExtentId] {
        &self.checksum_extents
    }
}

/// Recovery-owner authorization for one exact CLEAN mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryCleanPermit {
    topology_epoch: TopologyEpoch,
    generation: RecoveryGeneration,
    regions: Vec<RegionId>,
    checksum_extents: Vec<IntegrityExtentId>,
    certificate: FenceCertificate,
}

impl RecoveryCleanPermit {
    pub const fn generation(&self) -> RecoveryGeneration {
        self.generation
    }

    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch
    }

    pub fn regions(&self) -> &[RegionId] {
        &self.regions
    }

    pub fn checksum_extents(&self) -> &[IntegrityExtentId] {
        &self.checksum_extents
    }

    pub fn certificate(&self) -> &FenceCertificate {
        &self.certificate
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryCleanDecision {
    Clear(RecoveryCleanPermit),
    Refused {
        reason: RecoveryCleanRefusal,
        receipt: RecoveryCleanRefusalPermit,
        missing_stores: Vec<StoreId>,
        missing_regions: Vec<RegionId>,
        missing_extents: Vec<IntegrityExtentId>,
    },
}

impl RecoveryCleanDecision {
    pub const fn permits_clear(&self) -> bool {
        matches!(self, Self::Clear { .. })
    }
}

fn refused(
    snapshot: &RecoverySnapshot,
    request: &RecoveryCleanRequest,
    reason: RecoveryCleanRefusal,
    missing_stores: Vec<StoreId>,
    missing_regions: Vec<RegionId>,
    missing_extents: Vec<IntegrityExtentId>,
) -> RecoveryCleanDecision {
    RecoveryCleanDecision::Refused {
        reason,
        receipt: RecoveryCleanRefusalPermit {
            topology_epoch: snapshot.topology_epoch,
            generation: snapshot.generation,
            regions: request.regions.clone(),
            checksum_extents: request.checksum_extents.clone(),
        },
        missing_stores,
        missing_regions,
        missing_extents,
    }
}

pub fn evaluate_recovery_clean(
    snapshot: &RecoverySnapshot,
    request: &RecoveryCleanRequest,
    certificate: &FenceCertificate,
) -> Result<RecoveryCleanDecision, RecoveryError> {
    if snapshot.generation != request.generation {
        return Ok(refused(
            snapshot,
            request,
            RecoveryCleanRefusal::GenerationChanged,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ));
    }
    if certificate.topology_epoch != request.topology_epoch
        || certificate.fence_domain != request.fence_domain
    {
        return Ok(refused(
            snapshot,
            request,
            RecoveryCleanRefusal::TopologyChanged,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ));
    }
    if certificate.stores.is_empty() {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceEmpty,
        ));
    }

    let missing_stores: Vec<_> = request
        .stores
        .iter()
        .filter(|required| {
            !certificate.stores.iter().any(|observed| {
                observed.store_id == required.store && observed.through.0 >= required.through.0
            })
        })
        .map(|required| required.store)
        .collect();
    let missing_regions: Vec<_> = request
        .regions
        .iter()
        .copied()
        .filter(|region| !certificate.covers_region(*region, request.generation))
        .collect();
    let missing_extents: Vec<_> = request
        .checksum_extents
        .iter()
        .copied()
        .filter(|extent| !certificate.covers_integrity_extent(*extent, request.generation))
        .collect();

    if !missing_stores.is_empty() {
        return Ok(refused(
            snapshot,
            request,
            RecoveryCleanRefusal::MissingStoreFence,
            missing_stores,
            missing_regions,
            missing_extents,
        ));
    }
    if !missing_regions.is_empty() {
        return Ok(refused(
            snapshot,
            request,
            RecoveryCleanRefusal::MissingRegionCoverage,
            missing_stores,
            missing_regions,
            missing_extents,
        ));
    }
    if !missing_extents.is_empty() {
        return Ok(refused(
            snapshot,
            request,
            RecoveryCleanRefusal::MissingIntegrityCoverage,
            missing_stores,
            missing_regions,
            missing_extents,
        ));
    }

    let session_dirty = snapshot
        .writable_session
        .as_ref()
        .is_some_and(|session| !session.closed || session.global_fence.is_none());
    if session_dirty {
        return Ok(refused(
            snapshot,
            request,
            RecoveryCleanRefusal::SessionStillDirty,
            missing_stores,
            missing_regions,
            missing_extents,
        ));
    }

    Ok(RecoveryCleanDecision::Clear(RecoveryCleanPermit {
        topology_epoch: request.topology_epoch,
        generation: request.generation,
        regions: request.regions.clone(),
        checksum_extents: request.checksum_extents.clone(),
        certificate: certificate.clone(),
    }))
}

pub fn fence_ref(
    store: StoreId,
    topology_epoch: TopologyEpoch,
    through: StoreWriteWatermark,
    capability_evidence_id: dwv_store::CapabilityEvidenceId,
    fence_id: dwv_store::FenceId,
) -> StoreFenceRef {
    StoreFenceRef {
        fence_id,
        store_id: store,
        store_incarnation: dwv_store::StoreIncarnationId(0),
        topology_epoch,
        through,
        capability_evidence_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InvalidationTarget, MemoryRecoveryStore, WriteRecoveryRecordCommit};

    #[test]
    fn recovery_clean_requires_covering_fence_and_matching_generation() {
        let mut store = MemoryRecoveryStore::new(TopologyEpoch(5));
        WriteRecoveryRecordCommit::new(
            &mut store,
            TopologyEpoch(5),
            RecoveryGeneration(0),
            InvalidationTarget::new(vec![RegionId(1)], vec![IntegrityExtentId(2)]),
        )
        .commit()
        .unwrap();
        let request =
            RecoveryCleanRequest::new(TopologyEpoch(5), FenceDomain(9), RecoveryGeneration(1))
                .with_store(StoreId(7), StoreWriteWatermark(4))
                .with_region(RegionId(1))
                .with_checksum_extent(IntegrityExtentId(2));
        let certificate = FenceCertificate::new(
            TopologyEpoch(5),
            FenceDomain(9),
            vec![fence_ref(
                StoreId(7),
                TopologyEpoch(5),
                StoreWriteWatermark(4),
                dwv_store::CapabilityEvidenceId(1),
                dwv_store::FenceId(1),
            )],
            vec![(RegionId(1), RecoveryGeneration(1))],
        )
        .with_integrity_extent(IntegrityExtentId(2), RecoveryGeneration(1));
        let decision = evaluate_recovery_clean(store.snapshot(), &request, &certificate).unwrap();
        assert!(decision.permits_clear());

        let stale =
            RecoveryCleanRequest::new(TopologyEpoch(5), FenceDomain(9), RecoveryGeneration(0));
        assert!(matches!(
            evaluate_recovery_clean(store.snapshot(), &stale, &certificate).unwrap(),
            RecoveryCleanDecision::Refused {
                reason: RecoveryCleanRefusal::GenerationChanged,
                ..
            }
        ));
    }
}
