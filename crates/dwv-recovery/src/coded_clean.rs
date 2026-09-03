use crate::coded_authority::{CodedAdmissionAuthorityId, CodedCaptureEstablishment};
use crate::{
    ChecksumProfile, ChecksumProfileId, ChecksumSetGeneration, CodedAdmission,
    CodedAuthorityFrontier, CodedClaimRelease, CodedGeometryOwner, DirtyRegionRecord,
    DurableRecoveryCommit, FenceCertificate, FenceOccurrenceId, IntegrityExtentId, IntegrityRecord,
    IntegrityState, InvalidationTarget, RecoveryCleanPermit, RecoveryCleanRefusalPermit,
    RecoveryError, RecoveryGeneration, RecoveryMutation, RecoverySnapshot, RegionId, RegionState,
    TransitionError,
};
use dwv_core::{CodedUnitId, TopologyEpoch, TopologySnapshot};
use dwv_lifecycle_authority::{
    IncludedLifecycleAuthorization, LifecycleAuthorityOwner, LifecycleAuthorityVerifier,
};
use dwv_store::OperationSlotToken;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

mod transition;

pub(crate) use transition::CodedCaptureRemoval;
pub use transition::CodedCaptureTransition;
#[cfg(test)]
use transition::CodedCaptureTransitionKind;

mod operation_map_serde {
    use dwv_store::OperationSlotToken;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
    use std::collections::BTreeMap;

    pub fn serialize<S, V>(
        map: &BTreeMap<OperationSlotToken, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(
        deserializer: D,
    ) -> Result<BTreeMap<OperationSlotToken, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        let entries = Vec::<(OperationSlotToken, V)>::deserialize(deserializer)?;
        let mut map = BTreeMap::new();
        for (operation, value) in entries {
            if map.insert(operation, value).is_some() {
                return Err(D::Error::custom(
                    "duplicate coded capture operation identity",
                ));
            }
        }
        Ok(map)
    }
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct CodedCaptureId(pub u64);

/// Owner-supplied bounded position used to close a capture without retaining
/// an unbounded operation ledger.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct CodedCaptureFrontier {
    recovery_generation: RecoveryGeneration,
    admission_sequence: u64,
}

impl CodedCaptureFrontier {
    fn new(recovery_generation: RecoveryGeneration, admission_sequence: u64) -> Self {
        Self {
            recovery_generation,
            admission_sequence,
        }
    }

    pub const fn recovery_generation(self) -> RecoveryGeneration {
        self.recovery_generation
    }

    pub const fn admission_sequence(self) -> u64 {
        self.admission_sequence
    }
}

/// Owner-approved durable boundary for a mutation admitted after capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CodedCaptureCut {
    topology_epoch: TopologyEpoch,
    recovery_generation: RecoveryGeneration,
    frontier: CodedCaptureFrontier,
}

impl CodedCaptureCut {
    fn new(
        topology_epoch: TopologyEpoch,
        recovery_generation: RecoveryGeneration,
        frontier: CodedCaptureFrontier,
    ) -> Self {
        Self {
            topology_epoch,
            recovery_generation,
            frontier,
        }
    }

    pub const fn topology_epoch(self) -> TopologyEpoch {
        self.topology_epoch
    }

    pub const fn recovery_generation(self) -> RecoveryGeneration {
        self.recovery_generation
    }

    pub const fn frontier(self) -> CodedCaptureFrontier {
        self.frontier
    }
}
/// Owner-approved evidence that exact resolved membership may be forgotten
/// through a bounded frontier. Every current operation identity must be listed
/// in exactly one of `releasable_operations` or `retained_operations`, and
/// every releasable identity must carry an external
/// `ReleaseAllowed(operation-generation)` fact from the lifecycle owner. The
/// coordinator does not infer historicality from capture-local membership.
#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedCaptureRetentionSummary {
    pub capture: CodedCaptureId,
    pub topology_epoch: TopologyEpoch,
    pub frontier: CodedCaptureFrontier,
    pub releasable_operations: BTreeSet<OperationSlotToken>,
    pub retained_operations: BTreeSet<OperationSlotToken>,
    pub release_authorized_operations: BTreeSet<OperationSlotToken>,
}

#[cfg(test)]
impl CodedCaptureRetentionSummary {
    pub fn new(
        capture: CodedCaptureId,
        topology_epoch: TopologyEpoch,
        frontier: CodedCaptureFrontier,
        releasable_operations: impl IntoIterator<Item = OperationSlotToken>,
    ) -> Self {
        Self {
            capture,
            topology_epoch,
            frontier,
            releasable_operations: releasable_operations.into_iter().collect(),
            retained_operations: BTreeSet::new(),
            release_authorized_operations: BTreeSet::new(),
        }
    }

    pub fn with_retained_operations(
        mut self,
        retained_operations: impl IntoIterator<Item = OperationSlotToken>,
    ) -> Self {
        self.retained_operations = retained_operations.into_iter().collect();
        self
    }

    pub fn with_release_authorized_operations(
        mut self,
        operations: impl IntoIterator<Item = OperationSlotToken>,
    ) -> Self {
        self.release_authorized_operations = operations.into_iter().collect();
        self
    }
}

/// Owner-approved evidence that a resolved capture no longer needs exclusion.
/// For `CleanKnown`, `frontier` must be strictly newer than both the captured
/// and retained frontiers; that superseding dirty/recovery boundary ends the
/// capture's future-exclusion responsibility. A `Refused` capture has no
/// future-exclusion obligation.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodedCaptureRetirement {
    pub capture: CodedCaptureId,
    pub topology_epoch: TopologyEpoch,
    pub frontier: CodedCaptureFrontier,
}

#[cfg(test)]
impl CodedCaptureRetirement {
    pub const fn new(
        capture: CodedCaptureId,
        topology_epoch: TopologyEpoch,
        frontier: CodedCaptureFrontier,
    ) -> Self {
        Self {
            capture,
            topology_epoch,
            frontier,
        }
    }
}

/// Exact owner facts captured with a recovery-CLEAN closed-set boundary.
///
/// The coordinator records these facts; it does not derive topology,
/// persistence, dirty-state, checksum, or frontier meaning from private
/// operation state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodedCaptureOwnerFacts {
    pub topology: TopologySnapshot,
    pub recovery_generation: RecoveryGeneration,
    pub checksum_profile: ChecksumProfileId,
    pub checksum_set_generation: ChecksumSetGeneration,
    pub scope: ValidatedCodedCaptureScope,
    pub dirty_regions: BTreeSet<RegionId>,
    pub checksum_extents: BTreeSet<IntegrityExtentId>,
    pub lower_frontier: CodedCaptureFrontier,
    pub capture_frontier: CodedCaptureFrontier,
}

impl CodedCaptureOwnerFacts {
    #[allow(clippy::too_many_arguments)]
    fn new(
        topology: TopologySnapshot,
        recovery_generation: RecoveryGeneration,
        checksum_profile: ChecksumProfileId,
        checksum_set_generation: ChecksumSetGeneration,
        scope: ValidatedCodedCaptureScope,
        dirty_regions: impl IntoIterator<Item = RegionId>,
        checksum_extents: impl IntoIterator<Item = IntegrityExtentId>,
        lower_frontier: CodedCaptureFrontier,
        capture_frontier: CodedCaptureFrontier,
    ) -> Self {
        Self {
            topology,
            recovery_generation,
            checksum_profile,
            checksum_set_generation,
            scope,
            dirty_regions: dirty_regions.into_iter().collect(),
            checksum_extents: checksum_extents.into_iter().collect(),
            lower_frontier,
            capture_frontier,
        }
    }

    /// Build the complete coded scope that can invalidate the selected dirty
    /// regions or checksum extents under the captured topology and profiles.
    pub fn selected_invalidation_scope(
        topology: &TopologySnapshot,
        dirty_regions: impl IntoIterator<Item = RegionId>,
        checksum_extents: impl IntoIterator<Item = IntegrityExtentId>,
        dirty_region_bytes: u64,
        checksum_profile: ChecksumProfile,
    ) -> Result<ValidatedCodedCaptureScope, CodedCaptureError> {
        CodedGeometryOwner::new(topology.clone(), dirty_region_bytes, checksum_profile)?
            .capture_scope(dirty_regions, checksum_extents)
    }
}

/// Exact future-inclusive coded scope computed from owner geometry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedCodedCaptureScope {
    units: BTreeSet<CodedUnitId>,
    binding: Option<CaptureScopeBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CaptureScopeBinding {
    topology: TopologySnapshot,
    dirty_regions: BTreeSet<RegionId>,
    checksum_extents: BTreeSet<IntegrityExtentId>,
    dirty_region_bytes: u64,
    checksum_profile: ChecksumProfile,
}

impl ValidatedCodedCaptureScope {
    pub(crate) fn validated(
        units: BTreeSet<CodedUnitId>,
        topology: TopologySnapshot,
        dirty_regions: BTreeSet<RegionId>,
        checksum_extents: BTreeSet<IntegrityExtentId>,
        dirty_region_bytes: u64,
        checksum_profile: ChecksumProfile,
    ) -> Self {
        Self {
            units,
            binding: Some(CaptureScopeBinding {
                topology,
                dirty_regions,
                checksum_extents,
                dirty_region_bytes,
                checksum_profile,
            }),
        }
    }

    fn from_snapshot_units(units: impl IntoIterator<Item = CodedUnitId>) -> Self {
        Self {
            units: units.into_iter().collect(),
            binding: None,
        }
    }

    #[cfg(test)]
    fn complete(units: impl IntoIterator<Item = CodedUnitId>) -> Self {
        Self::from_snapshot_units(units)
    }

    pub fn units(&self) -> &BTreeSet<CodedUnitId> {
        &self.units
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CodedCapturePhase {
    Open,
    CleanCommitPending,
    CleanCommitUnknown,
    CleanKnown,
    Refused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CodedCaptureMembership {
    Included,
    Later,
    LaterDurableAfterClean,
    LaterDurableStalesClean,
    LaterRejected,
    LaterUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CodedCaptureDecision {
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedCleanCommitObservation {
    Durable,
    Rejected,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedCleanReconciliation {
    Durable,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedLaterCutObservation {
    DurableAfterClean,
    DurableStalesClean,
    Rejected,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedLaterCutReconciliation {
    DurableAfterClean,
    StalesClean,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CodedCaptureSnapshot {
    pub phase: CodedCapturePhase,
    pub capture: CodedCaptureId,
    pub topology: TopologySnapshot,
    pub recovery_generation: RecoveryGeneration,
    pub checksum_profile: ChecksumProfileId,
    pub checksum_set_generation: ChecksumSetGeneration,
    pub scope: BTreeSet<CodedUnitId>,
    pub scope_complete: bool,
    pub scope_validated: bool,
    pub lower_frontier_covered: bool,
    pub dirty_regions: BTreeSet<RegionId>,
    pub checksum_extents: BTreeSet<IntegrityExtentId>,
    pub lower_frontier: CodedCaptureFrontier,
    pub capture_frontier: CodedCaptureFrontier,
    pub retained_frontier: CodedCaptureFrontier,
    #[serde(with = "operation_map_serde")]
    pub membership: BTreeMap<OperationSlotToken, CodedCaptureMembership>,
    /// Exact post-capture cut retained for each unresolved `LaterUnknown`.
    #[serde(with = "operation_map_serde")]
    pub pending_later_cuts: BTreeMap<OperationSlotToken, CodedCaptureCut>,
    /// Exact owner-confirmed cuts retained until their operations are compacted.
    #[serde(default, with = "operation_map_serde")]
    pub resolved_later_cuts: BTreeMap<OperationSlotToken, CodedCaptureCut>,
    /// Newer durable dirty/recovery boundary that can supersede future exclusion.
    #[serde(default)]
    pub retirement_frontier: Option<CodedCaptureFrontier>,
    pub release_authorized_operations: BTreeSet<OperationSlotToken>,
    #[serde(default, with = "operation_map_serde")]
    pub release_frontiers: BTreeMap<OperationSlotToken, CodedAuthorityFrontier>,
    #[serde(default, with = "operation_map_serde")]
    pub release_certificates: BTreeMap<OperationSlotToken, FenceCertificate>,
    #[serde(default)]
    pub clean_closure_fence: Option<FenceCertificate>,
    pub decision: Option<CodedCaptureDecision>,
}
impl CodedCaptureSnapshot {
    fn clean_authorization_applies_to(&self, current: &Self) -> bool {
        self.phase == CodedCapturePhase::Open
            && current.phase == CodedCapturePhase::Open
            && self.decision.is_none()
            && current.decision.is_none()
            && self.capture == current.capture
            && self.topology == current.topology
            && self.recovery_generation == current.recovery_generation
            && self.checksum_profile == current.checksum_profile
            && self.checksum_set_generation == current.checksum_set_generation
            && self.scope == current.scope
            && self.scope_complete == current.scope_complete
            && self.scope_validated == current.scope_validated
            && self.lower_frontier_covered == current.lower_frontier_covered
            && self.dirty_regions == current.dirty_regions
            && self.checksum_extents == current.checksum_extents
            && self.lower_frontier == current.lower_frontier
            && self.capture_frontier == current.capture_frontier
            && self.retained_frontier == current.retained_frontier
            && self.retirement_frontier == current.retirement_frontier
            && self.membership.iter().all(|(operation, membership)| {
                current.membership.get(operation) == Some(membership)
            })
            && current.membership.iter().all(|(operation, membership)| {
                self.membership.contains_key(operation)
                    || *membership != CodedCaptureMembership::Included
            })
    }
}

impl CodedCaptureSnapshot {
    pub(crate) fn validates_internal_state(&self) -> bool {
        let decision_valid = match self.phase {
            CodedCapturePhase::Open => self.decision.is_none(),
            CodedCapturePhase::CleanCommitPending
            | CodedCapturePhase::CleanCommitUnknown
            | CodedCapturePhase::CleanKnown => {
                self.decision == Some(CodedCaptureDecision::Accepted)
            }
            CodedCapturePhase::Refused => self.decision != Some(CodedCaptureDecision::Accepted),
        };
        let closure_valid = self
            .clean_closure_fence
            .as_ref()
            .is_none_or(|fence| fence.occurrence != FenceOccurrenceId::UNASSIGNED)
            && (self.phase != CodedCapturePhase::CleanKnown || self.clean_closure_fence.is_some());
        let pending = self
            .membership
            .iter()
            .filter_map(|(operation, membership)| {
                (*membership == CodedCaptureMembership::LaterUnknown).then_some(*operation)
            })
            .collect::<BTreeSet<_>>();
        let resolved = self
            .membership
            .iter()
            .filter_map(|(operation, membership)| {
                matches!(
                    membership,
                    CodedCaptureMembership::LaterDurableAfterClean
                        | CodedCaptureMembership::LaterDurableStalesClean
                )
                .then_some(*operation)
            })
            .collect::<BTreeSet<_>>();
        let cut_valid = |cut: &CodedCaptureCut| {
            cut.topology_epoch == self.topology.topology_epoch()
                && cut.recovery_generation == cut.frontier.recovery_generation
                && cut.frontier > self.capture_frontier
        };
        !self.scope.is_empty()
            && decision_valid
            && closure_valid
            && self.lower_frontier.recovery_generation == self.recovery_generation
            && self.retained_frontier >= self.lower_frontier
            && self
                .retirement_frontier
                .is_none_or(|frontier| frontier > self.capture_frontier)
            && pending == self.pending_later_cuts.keys().copied().collect()
            && resolved == self.resolved_later_cuts.keys().copied().collect()
            && self.pending_later_cuts.values().all(cut_valid)
            && self.resolved_later_cuts.values().all(cut_valid)
            && self
                .release_authorized_operations
                .is_subset(&self.membership.keys().copied().collect())
            && self
                .release_frontiers
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
                == self.release_authorized_operations
            && self
                .release_certificates
                .keys()
                .all(|operation| self.release_authorized_operations.contains(operation))
            && !(self.phase == CodedCapturePhase::CleanKnown
                && self.membership.values().any(|membership| {
                    *membership == CodedCaptureMembership::LaterDurableStalesClean
                }))
            && !(self.phase == CodedCapturePhase::Open
                && self.membership.values().any(|membership| {
                    *membership == CodedCaptureMembership::LaterDurableAfterClean
                }))
    }
}

/// Capture-wide CLEAN authority issued after every Included lifecycle and
/// persistence proof has been evaluated.
pub struct CodedCaptureCleanAuthorization {
    authority_id: u64,
    accepted: CodedCaptureSnapshot,
    durable_prior: CodedCaptureSnapshot,
    permit: RecoveryCleanPermit,
    included_certificates: BTreeMap<OperationSlotToken, FenceCertificate>,
}

/// Exact dirty-integrity-owner authorization for one membership compaction.
pub struct CodedMembershipCompactionAuthorization {
    expected_generation: RecoveryGeneration,
    prior: CodedCaptureSnapshot,
    proposed: CodedCaptureSnapshot,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CodedCleanClosureEvidence {
    regions: Vec<DirtyRegionRecord>,
    fences: Vec<FenceCertificate>,
    closure_fence: FenceCertificate,
}

impl CodedCleanClosureEvidence {
    pub(crate) fn validates(
        &self,
        recovery: &RecoverySnapshot,
        capture: &CodedCaptureSnapshot,
    ) -> bool {
        self.regions
            .iter()
            .map(|record| record.region)
            .collect::<BTreeSet<_>>()
            == capture.dirty_regions
            && self
                .regions
                .iter()
                .all(|record| recovery.dirty_regions.contains(record))
            && self.closure_fence.occurrence != FenceOccurrenceId::UNASSIGNED
            && recovery
                .fence(self.closure_fence.occurrence_id())
                .is_some_and(|stored| stored.same_certificate_facts(&self.closure_fence))
            && capture.dirty_regions.iter().all(|region| {
                self.closure_fence
                    .covers_region(*region, capture.recovery_generation)
            })
            && capture.checksum_extents.iter().all(|extent| {
                self.closure_fence
                    .covers_integrity_extent(*extent, capture.recovery_generation)
            })
            && self.fences.iter().all(|fence| {
                recovery
                    .fence(fence.occurrence_id())
                    .is_some_and(|stored| stored.same_certificate_facts(fence))
            })
            && self.regions.iter().all(|record| {
                matches!(record.state, RegionState::Clean)
                    && record.last_clean_fence.as_ref().is_some_and(|fence| {
                        self.fences.iter().any(|included| {
                            included.occurrence_id() == fence.occurrence_id()
                                && included.same_certificate_facts(fence)
                        }) && recovery
                            .fence(fence.occurrence_id())
                            .is_some_and(|stored| stored.same_certificate_facts(fence))
                            && fence.covers_region(record.region, capture.recovery_generation)
                    })
            })
            && capture.checksum_extents.iter().all(|extent| {
                self.fences.iter().any(|fence| {
                    fence.covers_integrity_extent(*extent, capture.recovery_generation)
                })
            })
    }
}

/// Exact current-owner proof that all selected state is already conservative.
#[derive(Clone, Debug, Eq, PartialEq)]
struct CodedConservativeStateProof {
    expected_generation: RecoveryGeneration,
    predecessor: CodedCaptureSnapshot,
    dirty_regions: Vec<DirtyRegionRecord>,
    integrity_records: Vec<IntegrityRecord>,
}

impl CodedConservativeStateProof {
    fn from_recovery(
        recovery: &RecoverySnapshot,
        predecessor: &CodedCaptureSnapshot,
    ) -> Option<Self> {
        let dirty_regions = predecessor
            .dirty_regions
            .iter()
            .map(|region| {
                recovery
                    .dirty_regions
                    .iter()
                    .find(|record| {
                        record.region == *region
                            && matches!(
                                record.state,
                                RegionState::Dirty { .. } | RegionState::Indeterminate
                            )
                    })
                    .cloned()
            })
            .collect::<Option<Vec<_>>>()?;
        let integrity_records = predecessor
            .checksum_extents
            .iter()
            .map(|extent| {
                recovery
                    .integrity_records
                    .iter()
                    .find(|record| {
                        record.extent == *extent
                            && matches!(record.state, IntegrityState::Stale { .. })
                    })
                    .cloned()
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            expected_generation: recovery.generation,
            predecessor: predecessor.clone(),
            dirty_regions,
            integrity_records,
        })
    }

    fn validates(&self, recovery: &RecoverySnapshot, predecessor: &CodedCaptureSnapshot) -> bool {
        self.expected_generation == recovery.generation
            && &self.predecessor == predecessor
            && predecessor.topology.topology_epoch() == recovery.topology_epoch
            && recovery
                .coded_captures
                .iter()
                .any(|capture| capture == predecessor)
            && self.dirty_regions.iter().all(|expected| {
                predecessor.dirty_regions.contains(&expected.region)
                    && recovery
                        .dirty_regions
                        .iter()
                        .any(|record| record == expected)
                    && matches!(
                        expected.state,
                        RegionState::Dirty { .. } | RegionState::Indeterminate
                    )
            })
            && self
                .dirty_regions
                .iter()
                .map(|record| record.region)
                .collect::<BTreeSet<_>>()
                == predecessor.dirty_regions
            && self.integrity_records.iter().all(|expected| {
                predecessor.checksum_extents.contains(&expected.extent)
                    && recovery
                        .integrity_records
                        .iter()
                        .any(|record| record == expected)
                    && matches!(expected.state, IntegrityState::Stale { .. })
            })
            && self
                .integrity_records
                .iter()
                .map(|record| record.extent)
                .collect::<BTreeSet<_>>()
                == predecessor.checksum_extents
    }
}

/// Current-owner proof that one persisted refused capture remains
/// conservatively dirty and may be removed.
pub struct CodedRefusedCleanupAuthorization {
    expected_generation: RecoveryGeneration,
    prior: CodedCaptureSnapshot,
    refusal: Option<RecoveryCleanRefusalPermit>,
    conservative_state: Option<CodedConservativeStateProof>,
}

/// Current-owner proof that one empty `CleanKnown` capture has complete
/// independently durable closure and may be removed.
pub struct CodedCleanKnownCleanupAuthorization {
    expected_generation: RecoveryGeneration,
    prior: CodedCaptureSnapshot,
    clean_closure: CodedCleanClosureEvidence,
}
/// Current-owner proof that an inherited capture cannot close safely and
/// must be invalidated while it is removed.
pub struct CodedInheritedCaptureAbandonmentAuthorization {
    expected_generation: RecoveryGeneration,
    prior: CodedCaptureSnapshot,
    conservative_state: Option<CodedConservativeStateProof>,
}

pub enum CodedReopenResolution {
    Refused(CodedRefusedCleanupAuthorization),
    CleanKnown(CodedCleanKnownCleanupAuthorization),
    Abandon(CodedInheritedCaptureAbandonmentAuthorization),
}

/// Dirty-integrity owner for current durable capture evidence.
///
/// Persisted flags and frontiers are inputs. These methods issue live
/// capabilities only after validating the exact durable capture and the
/// independent region/fence evidence retained by recovery state.
struct CodedCaptureLifecycleOwner;
pub struct CodedCaptureRetentionOwner;

impl CodedCaptureLifecycleOwner {
    fn durable_capture(
        recovery: &RecoverySnapshot,
        capture: CodedCaptureId,
    ) -> Result<&CodedCaptureSnapshot, CodedCaptureError> {
        recovery
            .coded_captures
            .iter()
            .find(|snapshot| snapshot.capture == capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))
    }
    fn selected_clean_closure(
        recovery: &RecoverySnapshot,
        capture: &CodedCaptureSnapshot,
    ) -> Option<CodedCleanClosureEvidence> {
        let regions = capture
            .dirty_regions
            .iter()
            .map(|region| {
                recovery
                    .dirty_regions
                    .iter()
                    .find(|record| record.region == *region)
                    .cloned()
            })
            .collect::<Option<Vec<_>>>()?;
        let fences = regions
            .iter()
            .map(|record| record.last_clean_fence.clone())
            .collect::<Option<Vec<_>>>()?;
        let closure_fence = capture.clean_closure_fence.clone()?;
        let evidence = CodedCleanClosureEvidence {
            regions,
            fences,
            closure_fence,
        };
        evidence.validates(recovery, capture).then_some(evidence)
    }

    fn selected_conservative_state(
        recovery: &RecoverySnapshot,
        capture: &CodedCaptureSnapshot,
    ) -> Option<CodedConservativeStateProof> {
        CodedConservativeStateProof::from_recovery(recovery, capture)
    }

    fn clean_permit_applies(
        recovery: &RecoverySnapshot,
        capture: &CodedCaptureSnapshot,
        permit: &RecoveryCleanPermit,
    ) -> bool {
        recovery.generation == permit.generation()
            && permit.topology_epoch() == capture.topology.topology_epoch()
            && permit.regions().iter().copied().collect::<BTreeSet<_>>() == capture.dirty_regions
            && permit
                .checksum_extents()
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                == capture.checksum_extents
    }

    fn permit_covers_certificate(
        permit: &RecoveryCleanPermit,
        required: &FenceCertificate,
    ) -> bool {
        required.stores.iter().all(|required| {
            permit.certificate().stores.iter().any(|observed| {
                observed.store_id == required.store_id
                    && observed.topology_epoch == required.topology_epoch
                    && observed.store_incarnation == required.store_incarnation
                    && observed.capability_evidence_id == required.capability_evidence_id
                    && observed.through.0 >= required.through.0
            })
        })
    }

    fn authorize_clean(
        authority_id: u64,
        lifecycle_authority: &LifecycleAuthorityVerifier,
        recovery: &RecoverySnapshot,
        capture: CodedCaptureId,
        permit: RecoveryCleanPermit,
        included: impl IntoIterator<Item = (IncludedLifecycleAuthorization, FenceCertificate)>,
    ) -> Result<CodedCaptureCleanAuthorization, CodedCaptureError> {
        let prior = Self::durable_capture(recovery, capture)?.clone();
        let included = included
            .into_iter()
            .map(|(authorization, certificate)| {
                (authorization.operation(), (authorization, certificate))
            })
            .collect::<BTreeMap<_, _>>();
        let expected = prior
            .membership
            .iter()
            .filter_map(|(operation, membership)| {
                (*membership == CodedCaptureMembership::Included).then_some(*operation)
            })
            .collect::<BTreeSet<_>>();
        let observed = included.keys().copied().collect::<BTreeSet<_>>();
        let included_certificates = included
            .iter()
            .map(|(operation, (_, certificate))| (*operation, certificate.clone()))
            .collect::<BTreeMap<_, _>>();
        let dispositions_valid =
            included
                .iter()
                .all(|(&operation, (authorization, certificate))| {
                    lifecycle_authority.accepts_included(authorization)
                        && certificate.fence_domain == permit.certificate().fence_domain
                        && Self::permit_covers_certificate(&permit, certificate)
                        && if authorization.is_released() {
                            prior
                                .release_certificates
                                .get(&operation)
                                .is_some_and(|recorded| recorded == certificate)
                                && prior.release_authorized_operations.contains(&operation)
                        } else {
                            !prior.release_authorized_operations.contains(&operation)
                                && !prior.release_certificates.contains_key(&operation)
                        }
                });
        if !Self::clean_permit_applies(recovery, &prior, &permit)
            || prior.phase != CodedCapturePhase::Open
            || prior.decision.is_some()
            || !prior.validates_internal_state()
            || expected != observed
            || !dispositions_valid
        {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        Ok(CodedCaptureCleanAuthorization {
            authority_id,
            accepted: prior.clone(),
            durable_prior: prior,
            permit,
            included_certificates,
        })
    }

    fn refresh_clean_authorization(
        authority_id: u64,
        recovery: &RecoverySnapshot,
        current: &CodedCaptureSnapshot,
        authorization: CodedCaptureCleanAuthorization,
        permit: RecoveryCleanPermit,
    ) -> Result<CodedCaptureCleanAuthorization, CodedCaptureError> {
        let capture = authorization.accepted.capture;
        let durable_prior = Self::durable_capture(recovery, capture)?.clone();
        if authorization.authority_id != authority_id
            || !authorization.accepted.validates_internal_state()
            || !durable_prior.validates_internal_state()
            || !current.validates_internal_state()
            || !authorization
                .accepted
                .clean_authorization_applies_to(&durable_prior)
            || !authorization
                .accepted
                .clean_authorization_applies_to(current)
            || !durable_prior.clean_authorization_applies_to(current)
            || !Self::clean_permit_applies(recovery, &durable_prior, &permit)
            || !authorization
                .included_certificates
                .values()
                .all(|certificate| Self::permit_covers_certificate(&permit, certificate))
        {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        Ok(CodedCaptureCleanAuthorization {
            authority_id,
            accepted: authorization.accepted,
            durable_prior,
            permit,
            included_certificates: authorization.included_certificates,
        })
    }

    fn resolve(
        recovery: &RecoverySnapshot,
        prior: CodedCaptureSnapshot,
        refusal: Option<RecoveryCleanRefusalPermit>,
        retained_history_revalidated: bool,
    ) -> Result<CodedReopenResolution, CodedCaptureError> {
        let capture = prior.capture;
        if !prior.validates_internal_state() {
            return Err(CodedCaptureError::SnapshotInvalid(capture));
        }
        let clean_closure = Self::selected_clean_closure(recovery, &prior);
        match prior.phase {
            CodedCapturePhase::Refused => Ok(CodedReopenResolution::Refused(
                CodedRefusedCleanupAuthorization {
                    expected_generation: recovery.generation,
                    refusal: None,
                    conservative_state: Self::selected_conservative_state(recovery, &prior),
                    prior,
                },
            )),
            CodedCapturePhase::CleanKnown
                if retained_history_revalidated
                    && prior.membership.is_empty()
                    && prior.pending_later_cuts.is_empty()
                    && prior.resolved_later_cuts.is_empty()
                    && prior.release_authorized_operations.is_empty()
                    && prior.release_frontiers.is_empty()
                    && prior.release_certificates.is_empty()
                    && clean_closure.is_some() =>
            {
                Ok(CodedReopenResolution::CleanKnown(
                    CodedCleanKnownCleanupAuthorization {
                        expected_generation: recovery.generation,
                        prior,
                        clean_closure: clean_closure
                            .expect("guard established exact clean closure evidence"),
                    },
                ))
            }
            CodedCapturePhase::Open => {
                let refusal = refusal.ok_or(CodedCaptureError::CleanNotEligible(capture))?;
                Ok(CodedReopenResolution::Refused(
                    CodedRefusedCleanupAuthorization {
                        expected_generation: recovery.generation,
                        refusal: Some(refusal),
                        conservative_state: Self::selected_conservative_state(recovery, &prior),
                        prior,
                    },
                ))
            }
            CodedCapturePhase::CleanKnown => Ok(CodedReopenResolution::Abandon(
                CodedInheritedCaptureAbandonmentAuthorization {
                    expected_generation: recovery.generation,
                    conservative_state: Self::selected_conservative_state(recovery, &prior),
                    prior,
                },
            )),
            _ => Err(CodedCaptureError::CaptureNotRetirable(capture)),
        }
    }
}

impl CodedCaptureRetentionOwner {
    pub fn authorize_membership_compaction(
        recovery: &RecoverySnapshot,
        capture: CodedCaptureId,
    ) -> Result<Option<CodedMembershipCompactionAuthorization>, CodedCaptureError> {
        let prior = CodedCaptureLifecycleOwner::durable_capture(recovery, capture)?.clone();
        if !matches!(
            prior.phase,
            CodedCapturePhase::Refused | CodedCapturePhase::CleanKnown
        ) {
            return Ok(None);
        }
        let operations = prior.release_frontiers.keys().copied().collect::<Vec<_>>();
        if operations.is_empty() {
            return Ok(None);
        }
        let committed_generation = recovery
            .generation
            .checked_next()
            .ok_or(CodedCaptureError::RetentionFrontierStale(capture))?;
        let mut proposed = prior.clone();
        for operation in &operations {
            let membership = prior
                .membership
                .get(operation)
                .ok_or(CodedCaptureError::RetentionSummaryInvalid(capture))?;
            if !matches!(
                membership,
                CodedCaptureMembership::Included
                    | CodedCaptureMembership::LaterDurableAfterClean
                    | CodedCaptureMembership::LaterDurableStalesClean
                    | CodedCaptureMembership::LaterRejected
            ) || !prior.release_authorized_operations.contains(operation)
                || (prior.phase == CodedCapturePhase::CleanKnown
                    && *membership != CodedCaptureMembership::LaterRejected
                    && !prior.release_certificates.contains_key(operation))
            {
                return Err(CodedCaptureError::RetentionSummaryInvalid(capture));
            }
            let frontier = CodedCaptureFrontier::new(
                committed_generation,
                prior.release_frontiers[operation].admission_sequence(),
            );
            proposed.membership.remove(operation);
            proposed.pending_later_cuts.remove(operation);
            proposed.resolved_later_cuts.remove(operation);
            proposed.release_authorized_operations.remove(operation);
            proposed.release_frontiers.remove(operation);
            proposed.release_certificates.remove(operation);
            proposed.retained_frontier = proposed.retained_frontier.max(frontier);
        }
        if !proposed.validates_internal_state() {
            return Err(CodedCaptureError::RetentionSummaryInvalid(capture));
        }
        Ok(Some(CodedMembershipCompactionAuthorization {
            expected_generation: recovery.generation,
            prior,
            proposed,
        }))
    }
}

/// Coordinator-issued exact current-to-proposed durable capture transition.
///
/// This capability is intentionally not deserializable: persisted snapshots
/// are data, while a transition must be issued by the live coordinator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CodedCaptureUpdate {
    expected: Option<CodedCaptureSnapshot>,
    proposed: CodedCaptureSnapshot,
}

impl CodedCaptureUpdate {
    pub(crate) fn expected(&self) -> Option<&CodedCaptureSnapshot> {
        self.expected.as_ref()
    }

    pub(crate) fn proposed(&self) -> &CodedCaptureSnapshot {
        &self.proposed
    }
}

/// Recovery-inspection-issued exact uncertain-transition resolution.
pub struct CodedCaptureReconciliationReceipt {
    expected_unknown: CodedCaptureSnapshot,
    resolved: CodedCaptureSnapshot,
}

impl CodedCaptureReconciliationReceipt {
    pub(crate) fn new(
        expected_unknown: CodedCaptureSnapshot,
        resolved: CodedCaptureSnapshot,
    ) -> Self {
        Self {
            expected_unknown,
            resolved,
        }
    }

    pub const fn capture(&self) -> CodedCaptureId {
        self.resolved.capture
    }
}

/// One exact owner-evaluated CLEAN transition awaiting its recovery commit.
#[must_use = "prepared CLEAN transitions must be committed or discarded"]
pub struct PreparedCodedCleanCommit {
    capture: CodedCaptureId,
    expected_generation: RecoveryGeneration,
    durable_prior: CodedCaptureSnapshot,
    live_prior: CodedCaptureSnapshot,
    fence: FenceCertificate,
    proposed: CodedCaptureSnapshot,
    proposed_record: CaptureRecord,
}

/// One exact owner-refused transition awaiting durable persistence.
#[must_use = "prepared refusal must be committed or discarded"]
pub struct PreparedCodedRefusal {
    capture: CodedCaptureId,
    expected_generation: RecoveryGeneration,
    prior: CodedCaptureSnapshot,
    proposed: CodedCaptureSnapshot,
    proposed_record: CaptureRecord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedCleanAttemptPhase {
    Pending,
    Unknown,
    Refused,
}

/// One exact process-local CLEAN transaction and its observed commit state.
pub struct CodedCleanAttempt {
    prepared: PreparedCodedCleanCommit,
    phase: CodedCleanAttemptPhase,
}

impl CodedCleanAttempt {
    pub fn new(prepared: PreparedCodedCleanCommit) -> Self {
        Self {
            prepared,
            phase: CodedCleanAttemptPhase::Pending,
        }
    }

    pub const fn phase(&self) -> CodedCleanAttemptPhase {
        self.phase
    }

    pub fn observe(
        &mut self,
        observation: CodedCleanCommitObservation,
    ) -> Result<(), CodedCaptureError> {
        if self.phase != CodedCleanAttemptPhase::Pending {
            return Err(CodedCaptureError::CleanCommitNotPending(
                self.prepared.capture,
            ));
        }
        self.phase = match observation {
            CodedCleanCommitObservation::Unknown => CodedCleanAttemptPhase::Unknown,
            CodedCleanCommitObservation::Rejected => CodedCleanAttemptPhase::Refused,
            CodedCleanCommitObservation::Durable => {
                return Err(CodedCaptureError::CleanCommitNotPending(
                    self.prepared.capture,
                ));
            }
        };
        Ok(())
    }

    pub fn reject_unknown(&mut self) -> Result<(), CodedCaptureError> {
        if self.phase != CodedCleanAttemptPhase::Unknown {
            return Err(CodedCaptureError::CleanCommitUnknownNotPending(
                self.prepared.capture,
            ));
        }
        self.phase = CodedCleanAttemptPhase::Refused;
        Ok(())
    }

    pub fn into_pending_prepared(self) -> Result<PreparedCodedCleanCommit, CodedCaptureError> {
        if self.phase == CodedCleanAttemptPhase::Pending {
            Ok(self.prepared)
        } else {
            Err(CodedCaptureError::CleanCommitNotPending(
                self.prepared.capture,
            ))
        }
    }

    pub fn into_unknown_prepared(self) -> Result<PreparedCodedCleanCommit, CodedCaptureError> {
        if self.phase == CodedCleanAttemptPhase::Unknown {
            Ok(self.prepared)
        } else {
            Err(CodedCaptureError::CleanCommitUnknownNotPending(
                self.prepared.capture,
            ))
        }
    }
}

/// One exact later-cut transition awaiting its write-recovery commit.
#[must_use = "prepared later cuts must be committed or discarded"]
pub struct PreparedCodedLaterCut {
    capture: CodedCaptureId,
    expected_generation: RecoveryGeneration,
    operation: OperationSlotToken,
    target: InvalidationTarget,
    prior: CodedCaptureSnapshot,
    proposed: CodedCaptureSnapshot,
    proposed_record: CaptureRecord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodedLaterCutAttemptPhase {
    Unknown,
    Rejected,
}

/// One exact non-durable later-cut observation awaiting reconciliation.
pub struct CodedLaterCutAttempt {
    prepared: PreparedCodedLaterCut,
    phase: CodedLaterCutAttemptPhase,
}

impl CodedLaterCutAttempt {
    pub fn new(
        prepared: PreparedCodedLaterCut,
        observation: CodedLaterCutObservation,
    ) -> Result<Self, CodedCaptureError> {
        let phase = match observation {
            CodedLaterCutObservation::Unknown => CodedLaterCutAttemptPhase::Unknown,
            CodedLaterCutObservation::Rejected => CodedLaterCutAttemptPhase::Rejected,
            CodedLaterCutObservation::DurableAfterClean
            | CodedLaterCutObservation::DurableStalesClean => {
                return Err(CodedCaptureError::LaterCutOrder(prepared.capture));
            }
        };
        Ok(Self { prepared, phase })
    }

    pub const fn phase(&self) -> CodedLaterCutAttemptPhase {
        self.phase
    }

    pub const fn operation(&self) -> OperationSlotToken {
        self.prepared.operation
    }

    pub fn reject_unknown(&mut self) -> Result<(), CodedCaptureError> {
        if self.phase != CodedLaterCutAttemptPhase::Unknown {
            return Err(CodedCaptureError::LaterCutNotPending {
                capture: self.prepared.capture,
                operation: self.prepared.operation,
            });
        }
        self.phase = CodedLaterCutAttemptPhase::Rejected;
        Ok(())
    }

    pub fn into_unknown_prepared(self) -> Result<PreparedCodedLaterCut, CodedCaptureError> {
        if self.phase == CodedLaterCutAttemptPhase::Unknown {
            Ok(self.prepared)
        } else {
            Err(CodedCaptureError::LaterCutNotPending {
                capture: self.prepared.capture,
                operation: self.prepared.operation,
            })
        }
    }
}

/// Exact owner-issued membership compaction awaiting its recovery commit.
#[must_use = "prepared membership compaction must be committed or discarded"]
pub struct PreparedCodedMembershipCompaction {
    capture: CodedCaptureId,
    expected_generation: RecoveryGeneration,
    prior: CodedCaptureSnapshot,
    proposed: CodedCaptureSnapshot,
    proposed_record: CaptureRecord,
}

/// Exact refused-capture cleanup awaiting its recovery commit.
#[must_use = "prepared refused cleanup must be committed or discarded"]
pub struct PreparedCodedRefusedCleanup(PreparedCodedCaptureRemoval);

/// Exact clean-known capture cleanup awaiting its recovery commit.
#[must_use = "prepared clean-known cleanup must be committed or discarded"]
pub struct PreparedCodedCleanKnownCleanup(PreparedCodedCaptureRemoval);

/// Exact inherited capture invalidation and cleanup awaiting one recovery
/// commit.
#[must_use = "prepared inherited capture abandonment must be committed or discarded"]
pub struct PreparedCodedInheritedCaptureAbandonment(PreparedCodedCaptureRemoval);

struct PreparedCodedCaptureRemoval {
    clean_closure: Option<CodedCleanClosureEvidence>,
    conservative_state: Option<CodedConservativeStateProof>,
    capture: CodedCaptureId,
    expected_generation: RecoveryGeneration,
    prior: CodedCaptureSnapshot,
}

/// Exact coordinator-issued capture updates awaiting one recovery commit.
#[must_use = "prepared capture updates must be committed or discarded"]
pub struct PreparedCodedCaptureUpdates {
    expected_generation: RecoveryGeneration,
    prior: Vec<CodedCaptureSnapshot>,
    proposed: CodedCaptureCoordinator,
    updates: Vec<CodedCaptureUpdate>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodedCaptureError {
    EmptyScope,
    IncompleteScope,
    UnvalidatedScope,
    CaptureSequenceExhausted,
    LowerFrontierUncovered,
    InvalidScopeGeometry,
    ScopeIdentityOutOfRange,
    ScopeArithmeticOverflow,
    PreparedTransitionStale(CodedCaptureId),
    DurableReceiptMismatch(CodedCaptureId),
    SnapshotInvalid(CodedCaptureId),
    CaptureFrontierGenerationMismatch(CodedCaptureId),
    RetentionSummaryInvalid(CodedCaptureId),
    ClaimBindingInvalid,
    ForeignAdmissionAuthority,
    ForeignCaptureAuthority,
    AuthorityInvalidated,
    RetentionFrontierStale(CodedCaptureId),
    CaptureOperationUnresolved {
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    },
    CaptureNotRetirable(CodedCaptureId),
    RetirementEvidenceInvalid(CodedCaptureId),
    FrontierOrderInvalid(CodedCaptureId),
    CaptureAlreadyExists(CodedCaptureId),
    CaptureNotFound(CodedCaptureId),
    CaptureNotOpen(CodedCaptureId),
    DecisionAlreadyObserved(CodedCaptureId),
    CleanNotEligible(CodedCaptureId),
    CleanCommitNotPending(CodedCaptureId),
    CleanCommitUnknownNotPending(CodedCaptureId),
    LaterOperationNotMember {
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    },
    LaterCutNotPending {
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    },
    LaterCutMismatch {
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    },
    LaterCutOrder(CodedCaptureId),
    LaterCutEvidenceInvalid(CodedCaptureId),
}

impl fmt::Display for CodedCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyScope => formatter.write_str("coded capture scope is empty"),
            Self::IncompleteScope => formatter.write_str("coded capture scope is incomplete"),
            Self::UnvalidatedScope => formatter.write_str("coded capture scope is not validated"),
            Self::CaptureSequenceExhausted => {
                formatter.write_str("coded capture sequence is exhausted")
            }
            Self::LowerFrontierUncovered => {
                formatter.write_str("coded capture lower frontier is not covered")
            }
            Self::InvalidScopeGeometry => {
                formatter.write_str("coded capture scope geometry is invalid")
            }
            Self::ScopeIdentityOutOfRange => {
                formatter.write_str("coded capture scope identity is outside the topology")
            }
            Self::ScopeArithmeticOverflow => {
                formatter.write_str("coded capture scope arithmetic overflowed")
            }
            Self::ClaimBindingInvalid => {
                formatter.write_str("coded claim binding is not the exact admitted mutation")
            }
            Self::ForeignAdmissionAuthority => {
                formatter.write_str("coded admission came from another authority graph")
            }
            Self::ForeignCaptureAuthority => {
                formatter.write_str("coded transition came from another live capture owner")
            }
            Self::AuthorityInvalidated => formatter.write_str(
                "coded capture authority was invalidated by an uncertain durable transition",
            ),
            Self::PreparedTransitionStale(capture) => {
                write!(
                    formatter,
                    "prepared transition for capture {capture:?} is stale"
                )
            }
            Self::DurableReceiptMismatch(capture) => {
                write!(
                    formatter,
                    "durable receipt does not cover capture {capture:?}"
                )
            }
            Self::SnapshotInvalid(capture) => {
                write!(formatter, "durable coded capture {capture:?} is invalid")
            }
            Self::CaptureFrontierGenerationMismatch(capture) => write!(
                formatter,
                "capture frontier generation does not match capture {capture:?} recovery generation"
            ),
            Self::FrontierOrderInvalid(capture) => write!(
                formatter,
                "capture {capture:?} lower frontier is after its capture frontier"
            ),
            Self::RetentionSummaryInvalid(capture) => {
                write!(
                    formatter,
                    "retention summary for capture {capture:?} is invalid"
                )
            }
            Self::RetentionFrontierStale(capture) => {
                write!(
                    formatter,
                    "retention frontier for capture {capture:?} is stale"
                )
            }
            Self::CaptureOperationUnresolved { capture, operation } => write!(
                formatter,
                "operation {operation:?} remains unresolved in capture {capture:?}"
            ),
            Self::CaptureNotRetirable(capture) => {
                write!(
                    formatter,
                    "capture {capture:?} still has live or unresolved obligations"
                )
            }
            Self::RetirementEvidenceInvalid(capture) => {
                write!(
                    formatter,
                    "retirement evidence for capture {capture:?} is invalid"
                )
            }
            Self::CaptureAlreadyExists(capture) => {
                write!(formatter, "capture {capture:?} already exists")
            }
            Self::CaptureNotFound(capture) => {
                write!(formatter, "capture {capture:?} was not found")
            }
            Self::CaptureNotOpen(capture) => write!(formatter, "capture {capture:?} is not open"),
            Self::DecisionAlreadyObserved(capture) => {
                write!(
                    formatter,
                    "capture decision for {capture:?} was already observed"
                )
            }
            Self::CleanNotEligible(capture) => {
                write!(formatter, "capture {capture:?} is not eligible for CLEAN")
            }
            Self::CleanCommitNotPending(capture) => {
                write!(formatter, "capture {capture:?} has no pending CLEAN commit")
            }
            Self::CleanCommitUnknownNotPending(capture) => write!(
                formatter,
                "capture {capture:?} has no unresolved CLEAN commit"
            ),
            Self::LaterOperationNotMember { capture, operation } => write!(
                formatter,
                "operation {operation:?} is not a later member of capture {capture:?}"
            ),
            Self::LaterCutNotPending { capture, operation } => write!(
                formatter,
                "later cut for operation {operation:?} in capture {capture:?} is not unknown"
            ),
            Self::LaterCutMismatch { capture, operation } => write!(
                formatter,
                "later cut for operation {operation:?} does not match the exact unknown observation in capture {capture:?}"
            ),
            Self::LaterCutOrder(capture) => {
                write!(
                    formatter,
                    "later cut ordering is invalid for capture {capture:?}"
                )
            }
            Self::LaterCutEvidenceInvalid(capture) => {
                write!(
                    formatter,
                    "later cut evidence for capture {capture:?} is invalid"
                )
            }
        }
    }
}

impl std::error::Error for CodedCaptureError {}

#[derive(Clone, Debug)]
struct CaptureRecord {
    owner_facts: CodedCaptureOwnerFacts,
    phase: CodedCapturePhase,
    scope: BTreeSet<CodedUnitId>,
    scope_complete_data: bool,
    scope_validated_data: bool,
    lower_frontier_covered_data: bool,
    owner_revalidated: bool,
    membership: BTreeMap<OperationSlotToken, CodedCaptureMembership>,
    pending_later_cuts: BTreeMap<OperationSlotToken, CodedCaptureCut>,
    resolved_later_cuts: BTreeMap<OperationSlotToken, CodedCaptureCut>,
    release_authorized_operations: BTreeSet<OperationSlotToken>,
    release_frontiers: BTreeMap<OperationSlotToken, CodedAuthorityFrontier>,
    release_certificates: BTreeMap<OperationSlotToken, FenceCertificate>,
    clean_closure_fence: Option<FenceCertificate>,
    retained_frontier: CodedCaptureFrontier,
    retained_history_revalidated: bool,
    retirement_frontier: Option<CodedCaptureFrontier>,
    decision: Option<CodedCaptureDecision>,
}

impl CaptureRecord {
    fn snapshot(&self, capture: CodedCaptureId) -> CodedCaptureSnapshot {
        CodedCaptureSnapshot {
            phase: self.phase,
            capture,
            topology: self.owner_facts.topology.clone(),
            recovery_generation: self.owner_facts.recovery_generation,
            checksum_profile: self.owner_facts.checksum_profile,
            checksum_set_generation: self.owner_facts.checksum_set_generation,
            scope: self.scope.clone(),
            scope_complete: self.scope_complete_data,
            scope_validated: self.scope_validated_data,
            lower_frontier_covered: self.lower_frontier_covered_data,
            dirty_regions: self.owner_facts.dirty_regions.clone(),
            checksum_extents: self.owner_facts.checksum_extents.clone(),
            lower_frontier: self.owner_facts.lower_frontier,
            capture_frontier: self.owner_facts.capture_frontier,
            retained_frontier: self.retained_frontier,
            membership: self.membership.clone(),
            pending_later_cuts: self.pending_later_cuts.clone(),
            resolved_later_cuts: self.resolved_later_cuts.clone(),
            release_authorized_operations: self.release_authorized_operations.clone(),
            release_frontiers: self.release_frontiers.clone(),
            release_certificates: self.release_certificates.clone(),
            clean_closure_fence: self.clean_closure_fence.clone(),
            retirement_frontier: self.retirement_frontier,
            decision: self.decision,
        }
    }

    fn intersects(&self, units: &BTreeSet<CodedUnitId>) -> bool {
        self.scope.iter().any(|unit| units.contains(unit))
    }

    fn tracks_future(&self) -> bool {
        !matches!(self.phase, CodedCapturePhase::Refused)
    }

    fn recorded_membership_permits_effect(&self, operation: OperationSlotToken) -> bool {
        matches!(
            self.membership.get(&operation),
            Some(CodedCaptureMembership::Included)
                | Some(CodedCaptureMembership::LaterDurableAfterClean)
                | Some(CodedCaptureMembership::LaterDurableStalesClean)
        )
    }

    fn mutation_set_closed(&self) -> bool {
        !self.scope.is_empty()
            && !self.membership.values().any(|membership| {
                matches!(
                    membership,
                    CodedCaptureMembership::LaterDurableStalesClean
                        | CodedCaptureMembership::LaterUnknown
                )
            })
    }

    #[cfg(test)]
    fn clean_eligible(&self) -> bool {
        self.decision == Some(CodedCaptureDecision::Accepted) && self.mutation_set_closed()
    }

    fn accepts_later_cut(&self, cut: CodedCaptureCut) -> bool {
        let prior_frontier = self
            .owner_facts
            .capture_frontier
            .max(self.retained_frontier);
        cut.topology_epoch == self.owner_facts.topology.topology_epoch()
            && cut.frontier.recovery_generation == cut.recovery_generation
            && cut.frontier > prior_frontier
    }

    #[cfg(test)]
    fn can_forget_operation(
        &self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
    ) -> Result<(), CodedCaptureError> {
        let Some(membership) = self.membership.get(&operation) else {
            return Ok(());
        };
        if !matches!(
            self.phase,
            CodedCapturePhase::Refused | CodedCapturePhase::CleanKnown
        ) {
            return Err(CodedCaptureError::CaptureOperationUnresolved { capture, operation });
        }
        matches!(
            membership,
            CodedCaptureMembership::Included
                | CodedCaptureMembership::LaterDurableAfterClean
                | CodedCaptureMembership::LaterDurableStalesClean
                | CodedCaptureMembership::LaterRejected
        )
        .then_some(())
        .ok_or(CodedCaptureError::CaptureOperationUnresolved { capture, operation })
    }
}
static NEXT_CODED_CAPTURE_AUTHORITY_ID: AtomicU64 = AtomicU64::new(1);

fn next_coded_capture_authority_id() -> u64 {
    NEXT_CODED_CAPTURE_AUTHORITY_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .expect("coded capture authority identity exhausted")
}

pub struct CodedCaptureCoordinator {
    authority_id: u64,
    admission_authority: CodedAdmissionAuthorityId,
    lifecycle_authority: LifecycleAuthorityVerifier,
    captures: BTreeMap<CodedCaptureId, CaptureRecord>,
}

impl Default for CodedCaptureCoordinator {
    fn default() -> Self {
        let (_, lifecycle_authority) = LifecycleAuthorityOwner::new();
        Self::new_with_authorities(lifecycle_authority, CodedAdmissionAuthorityId::new())
    }
}

/// dwv:req req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set
impl CodedCaptureCoordinator {
    fn fork_candidate(&self) -> Self {
        Self {
            authority_id: self.authority_id,
            admission_authority: self.admission_authority,
            lifecycle_authority: self.lifecycle_authority.clone(),
            captures: self.captures.clone(),
        }
    }
    pub(crate) const fn authority_id(&self) -> u64 {
        self.authority_id
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_with_authorities(
        lifecycle_authority: LifecycleAuthorityVerifier,
        admission_authority: CodedAdmissionAuthorityId,
    ) -> Self {
        Self {
            authority_id: next_coded_capture_authority_id(),
            admission_authority,
            lifecycle_authority,
            captures: BTreeMap::new(),
        }
    }

    /// Issue capture-wide CLEAN authority only from observations produced by
    /// this exact live coordinator owner.
    pub(crate) fn authorize_clean(
        &self,
        recovery: &RecoverySnapshot,
        capture: CodedCaptureId,
        permit: RecoveryCleanPermit,
        included: impl IntoIterator<Item = (IncludedLifecycleAuthorization, FenceCertificate)>,
    ) -> Result<CodedCaptureCleanAuthorization, CodedCaptureError> {
        CodedCaptureLifecycleOwner::authorize_clean(
            self.authority_id,
            &self.lifecycle_authority,
            recovery,
            capture,
            permit,
            included,
        )
    }
    /// Refresh recovery-state evidence while consuming the existing
    /// capture-owner acceptance witness.
    pub(crate) fn refresh_clean_authorization(
        &self,
        recovery: &RecoverySnapshot,
        authorization: CodedCaptureCleanAuthorization,
        permit: RecoveryCleanPermit,
    ) -> Result<CodedCaptureCleanAuthorization, CodedCaptureError> {
        let capture = authorization.accepted.capture;
        let current = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        CodedCaptureLifecycleOwner::refresh_clean_authorization(
            self.authority_id,
            recovery,
            &current,
            authorization,
            permit,
        )
    }

    /// Resolve a capture using only retained history established by this live
    /// owner. A compacted summary loaded after restart remains untrusted and
    /// can only take the conservative inherited-abandonment path.
    pub(crate) fn resolve_capture(
        &self,
        recovery: &RecoverySnapshot,
        capture: CodedCaptureId,
        refusal: Option<RecoveryCleanRefusalPermit>,
    ) -> Result<CodedReopenResolution, CodedCaptureError> {
        let retained_history_revalidated = self
            .captures
            .get(&capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?
            .retained_history_revalidated;
        let prior = CodedCaptureLifecycleOwner::durable_capture(recovery, capture)?.clone();
        CodedCaptureLifecycleOwner::resolve(recovery, prior, refusal, retained_history_revalidated)
    }

    pub(crate) fn from_snapshots(
        snapshots: impl IntoIterator<Item = CodedCaptureSnapshot>,
    ) -> Result<Self, CodedCaptureError> {
        let mut coordinator = Self::new();
        for snapshot in snapshots {
            let capture = snapshot.capture;
            let owner_facts = CodedCaptureOwnerFacts::new(
                snapshot.topology.clone(),
                snapshot.recovery_generation,
                snapshot.checksum_profile,
                snapshot.checksum_set_generation,
                ValidatedCodedCaptureScope::from_snapshot_units(snapshot.scope.iter().copied()),
                snapshot.dirty_regions.iter().copied(),
                snapshot.checksum_extents.iter().copied(),
                snapshot.lower_frontier,
                snapshot.capture_frontier,
            );
            coordinator.insert_capture(capture, owner_facts, [])?;
            coordinator.restore_snapshot_state(snapshot)?;
        }
        Ok(coordinator)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_snapshots_for_owner(
        snapshots: impl IntoIterator<Item = CodedCaptureSnapshot>,
        topology: &TopologySnapshot,
        recovery_generation: RecoveryGeneration,
        checksum_profile: ChecksumProfile,
        checksum_set_generation: ChecksumSetGeneration,
        dirty_region_bytes: u64,
        lifecycle_authority: LifecycleAuthorityVerifier,
        admission_authority: CodedAdmissionAuthorityId,
    ) -> Result<Self, CodedCaptureError> {
        let mut coordinator = Self::new_with_authorities(lifecycle_authority, admission_authority);
        for snapshot in snapshots {
            let capture = snapshot.capture;
            let scope = CodedCaptureOwnerFacts::selected_invalidation_scope(
                topology,
                snapshot.dirty_regions.iter().copied(),
                snapshot.checksum_extents.iter().copied(),
                dirty_region_bytes,
                checksum_profile,
            )
            .map_err(|_| CodedCaptureError::SnapshotInvalid(capture))?;
            if snapshot.topology != *topology
                || snapshot.recovery_generation > recovery_generation
                || snapshot.checksum_profile != checksum_profile.id
                || snapshot.checksum_set_generation != checksum_set_generation
                || snapshot.scope != *scope.units()
                || snapshot.lower_frontier.recovery_generation != snapshot.recovery_generation
                || snapshot.capture_frontier.recovery_generation != snapshot.recovery_generation
                || snapshot.retained_frontier < snapshot.lower_frontier
                || snapshot.retained_frontier.recovery_generation > recovery_generation
            {
                return Err(CodedCaptureError::SnapshotInvalid(capture));
            }
            let owner_facts = CodedCaptureOwnerFacts::new(
                snapshot.topology.clone(),
                snapshot.recovery_generation,
                snapshot.checksum_profile,
                snapshot.checksum_set_generation,
                scope,
                snapshot.dirty_regions.iter().copied(),
                snapshot.checksum_extents.iter().copied(),
                snapshot.lower_frontier,
                snapshot.capture_frontier,
            );
            coordinator.insert_capture(capture, owner_facts, [])?;
            coordinator.restore_snapshot_state(snapshot)?;
        }
        Ok(coordinator)
    }
    fn restore_snapshot_state(
        &mut self,
        snapshot: CodedCaptureSnapshot,
    ) -> Result<(), CodedCaptureError> {
        let capture = snapshot.capture;
        let record = self.record_mut(capture)?;
        if !snapshot.validates_internal_state() {
            return Err(CodedCaptureError::SnapshotInvalid(capture));
        }
        record.phase = snapshot.phase;
        record.scope_complete_data = snapshot.scope_complete;
        record.scope_validated_data = snapshot.scope_validated;
        record.lower_frontier_covered_data = snapshot.lower_frontier_covered;
        record.membership = snapshot.membership;
        record.pending_later_cuts = snapshot.pending_later_cuts;
        record.resolved_later_cuts = snapshot.resolved_later_cuts;
        record.release_authorized_operations = snapshot.release_authorized_operations;
        record.retained_frontier = snapshot.retained_frontier;
        record.retained_history_revalidated = false;
        record.release_frontiers = snapshot.release_frontiers;
        record.release_certificates = snapshot.release_certificates;
        record.clean_closure_fence = snapshot.clean_closure_fence;
        record.retirement_frontier = snapshot.retirement_frontier;
        record.decision = snapshot.decision;
        Ok(())
    }

    pub fn snapshots(&self) -> Vec<CodedCaptureSnapshot> {
        self.captures
            .iter()
            .map(|(capture, record)| record.snapshot(*capture))
            .collect()
    }

    pub fn capture_count(&self) -> usize {
        self.captures.len()
    }

    fn prepare_updates(
        &self,
        proposed: Self,
        expected_generation: RecoveryGeneration,
    ) -> Result<Option<PreparedCodedCaptureUpdates>, CodedCaptureError> {
        let prior = self.snapshots();
        let prior_by_capture = prior
            .iter()
            .map(|snapshot| (snapshot.capture, snapshot))
            .collect::<BTreeMap<_, _>>();
        let proposed_snapshots = proposed.snapshots();
        let proposed_by_capture = proposed_snapshots
            .iter()
            .map(|snapshot| (snapshot.capture, snapshot))
            .collect::<BTreeMap<_, _>>();
        if let Some(capture) = prior_by_capture
            .keys()
            .find(|capture| !proposed_by_capture.contains_key(capture))
        {
            return Err(CodedCaptureError::SnapshotInvalid(*capture));
        }
        let updates = proposed_snapshots
            .into_iter()
            .filter(|snapshot| prior_by_capture.get(&snapshot.capture).copied() != Some(snapshot))
            .map(|proposed| CodedCaptureUpdate {
                expected: prior_by_capture
                    .get(&proposed.capture)
                    .map(|snapshot| (*snapshot).clone()),
                proposed,
            })
            .collect::<Vec<_>>();
        Ok(
            (!updates.is_empty()).then_some(PreparedCodedCaptureUpdates {
                expected_generation,
                prior,
                proposed,
                updates,
            }),
        )
    }

    pub(crate) fn confirm_prepared_updates(
        &mut self,
        prepared: PreparedCodedCaptureUpdates,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        let capture = prepared
            .updates
            .first()
            .expect("prepared capture updates are non-empty")
            .proposed
            .capture;
        if self.snapshots() != prepared.prior {
            return Err(CodedCaptureError::PreparedTransitionStale(capture));
        }
        if receipt.expected_generation() != prepared.expected_generation
            || prepared.expected_generation.checked_next() != Some(receipt.generation())
            || prepared.updates.iter().any(|update| {
                receipt.topology_epoch() != update.proposed.topology.topology_epoch()
                    || !receipt.committed_coded_capture(&update.proposed)
            })
        {
            return Err(CodedCaptureError::DurableReceiptMismatch(capture));
        }
        *self = prepared.proposed;
        Ok(())
    }

    pub(crate) fn prepare_admission_observation(
        &self,
        admission: &CodedAdmission,
        expected_generation: RecoveryGeneration,
    ) -> Result<Option<PreparedCodedCaptureUpdates>, CodedCaptureError> {
        if !admission.belongs_to(self.admission_authority) {
            return Err(CodedCaptureError::ForeignAdmissionAuthority);
        }
        let mut proposed = self.fork_candidate();
        proposed.observe_admitted_claim(admission);
        self.prepare_updates(proposed, expected_generation)
    }

    pub(crate) fn prepare_release_observation(
        &self,
        release: &CodedClaimRelease,
        certificate: Option<&FenceCertificate>,
        expected_generation: RecoveryGeneration,
    ) -> Result<Option<PreparedCodedCaptureUpdates>, CodedCaptureError> {
        let mut proposed = self.fork_candidate();
        proposed.observe_release_allowed(release, certificate)?;
        self.prepare_updates(proposed, expected_generation)
    }

    pub(crate) fn prepare_reconciliation(
        &self,
        receipt: CodedCaptureReconciliationReceipt,
        expected_generation: RecoveryGeneration,
    ) -> Result<PreparedCodedCaptureUpdates, CodedCaptureError> {
        let capture = receipt.capture();
        let mut proposed = self.fork_candidate();
        proposed.apply_reconciliation(receipt)?;
        self.prepare_updates(proposed, expected_generation)?
            .ok_or(CodedCaptureError::PreparedTransitionStale(capture))
    }

    pub fn has_unresolved_reopen_state(&self) -> bool {
        self.captures.values().any(|record| {
            matches!(
                record.phase,
                CodedCapturePhase::Open
                    | CodedCapturePhase::CleanCommitPending
                    | CodedCapturePhase::CleanCommitUnknown
            ) || !record.pending_later_cuts.is_empty()
                || record
                    .membership
                    .keys()
                    .any(|operation| !record.release_authorized_operations.contains(operation))
        })
    }

    pub(crate) fn prepare_capture_start(
        &self,
        capture: CodedCaptureId,
        expected_generation: RecoveryGeneration,
        checksum_set_generation: ChecksumSetGeneration,
        scope: ValidatedCodedCaptureScope,
        establishment: CodedCaptureEstablishment,
    ) -> Result<PreparedCodedCaptureUpdates, CodedCaptureError> {
        let recovery_generation = expected_generation.checked_next().ok_or(
            CodedCaptureError::CaptureFrontierGenerationMismatch(capture),
        )?;
        let mut proposed = self.fork_candidate();
        proposed.start_capture(
            capture,
            recovery_generation,
            checksum_set_generation,
            scope,
            establishment,
        )?;
        self.prepare_updates(proposed, expected_generation)?
            .ok_or(CodedCaptureError::CaptureNotFound(capture))
    }

    /// Retain one exact lifecycle release and persistence receipt for every
    /// affected capture.
    fn observe_release_allowed(
        &mut self,
        release: &CodedClaimRelease,
        certificate: Option<&FenceCertificate>,
    ) -> Result<bool, CodedCaptureError> {
        let operation = release.operation();
        let mut observed = false;
        for (capture, record) in &mut self.captures {
            if !record.membership.contains_key(&operation) {
                continue;
            }
            if let Some(certificate) = certificate {
                if certificate.topology_epoch != record.owner_facts.topology.topology_epoch()
                    || certificate.stores.is_empty()
                {
                    return Err(CodedCaptureError::SnapshotInvalid(*capture));
                }
                record
                    .release_certificates
                    .insert(operation, certificate.clone());
            }
            observed |= record.release_authorized_operations.insert(operation);
            record
                .release_frontiers
                .insert(operation, release.frontier());
        }
        Ok(observed)
    }

    /// Start a bounded capture from capabilities issued by the scope and
    /// coded-admission owners.
    fn start_capture(
        &mut self,
        capture: CodedCaptureId,
        recovery_generation: RecoveryGeneration,
        checksum_set_generation: ChecksumSetGeneration,
        scope: ValidatedCodedCaptureScope,
        establishment: CodedCaptureEstablishment,
    ) -> Result<(), CodedCaptureError> {
        let (history, boundary, active_claims) = establishment.into_parts();
        if scope.units.is_empty() {
            return Err(CodedCaptureError::EmptyScope);
        }
        let binding = scope
            .binding
            .as_ref()
            .ok_or(CodedCaptureError::UnvalidatedScope)?;
        let lower_frontier =
            CodedCaptureFrontier::new(recovery_generation, history.through_sequence());
        let capture_frontier =
            CodedCaptureFrontier::new(recovery_generation, boundary.capture_sequence());
        let owner_facts = CodedCaptureOwnerFacts::new(
            binding.topology.clone(),
            recovery_generation,
            binding.checksum_profile.id,
            checksum_set_generation,
            scope.clone(),
            binding.dirty_regions.iter().copied(),
            binding.checksum_extents.iter().copied(),
            lower_frontier,
            capture_frontier,
        );
        self.insert_capture(
            capture,
            owner_facts,
            active_claims
                .into_iter()
                .map(|admission| (admission.operation(), admission.units().clone())),
        )
    }

    fn insert_capture(
        &mut self,
        capture: CodedCaptureId,
        owner_facts: CodedCaptureOwnerFacts,
        active_claims: impl IntoIterator<Item = (OperationSlotToken, BTreeSet<CodedUnitId>)>,
    ) -> Result<(), CodedCaptureError> {
        if self.captures.contains_key(&capture) {
            return Err(CodedCaptureError::CaptureAlreadyExists(capture));
        }
        let scope = owner_facts.scope.clone();
        if scope.units.is_empty() {
            return Err(CodedCaptureError::EmptyScope);
        }
        if owner_facts.capture_frontier.recovery_generation != owner_facts.recovery_generation {
            return Err(CodedCaptureError::CaptureFrontierGenerationMismatch(
                capture,
            ));
        }
        if owner_facts.lower_frontier > owner_facts.capture_frontier {
            return Err(CodedCaptureError::FrontierOrderInvalid(capture));
        }
        let membership = active_claims
            .into_iter()
            .filter_map(|(operation, units)| {
                scope
                    .units
                    .iter()
                    .any(|unit| units.contains(unit))
                    .then_some((operation, CodedCaptureMembership::Included))
            })
            .collect();
        let retained_frontier = owner_facts.lower_frontier;
        self.captures.insert(
            capture,
            CaptureRecord {
                owner_facts,
                phase: CodedCapturePhase::Open,
                scope: scope.units,
                scope_complete_data: true,
                scope_validated_data: true,
                lower_frontier_covered_data: true,
                owner_revalidated: true,
                membership,
                pending_later_cuts: BTreeMap::new(),
                resolved_later_cuts: BTreeMap::new(),
                release_authorized_operations: BTreeSet::new(),
                release_frontiers: BTreeMap::new(),
                release_certificates: BTreeMap::new(),
                clean_closure_fence: None,
                retained_frontier,
                retained_history_revalidated: true,
                retirement_frontier: None,
                decision: None,
            },
        );
        Ok(())
    }

    /// Observe a later admitted claim against every active capture.
    fn observe_admitted_claim(&mut self, admission: &CodedAdmission) {
        for capture in self.captures.values_mut() {
            if capture.tracks_future() && capture.intersects(admission.units()) {
                capture
                    .membership
                    .insert(admission.operation(), CodedCaptureMembership::Later);
            }
        }
    }

    #[cfg(test)]
    /// Return whether every active capture's recorded membership permits the effect.
    pub(crate) fn effect_allowed(&self, operation: OperationSlotToken) -> bool {
        // Rejected and uncertain cuts remain blocked; a refused capture stops tracking.
        self.captures.values().all(|capture| {
            !capture.tracks_future()
                || !capture.membership.contains_key(&operation)
                || capture.recorded_membership_permits_effect(operation)
        })
    }

    pub(crate) fn effect_allowed_for(
        &self,
        operation: OperationSlotToken,
        units: &BTreeSet<CodedUnitId>,
    ) -> bool {
        self.captures.values().all(|capture| {
            !capture.tracks_future()
                || !capture.intersects(units)
                || capture.recorded_membership_permits_effect(operation)
        })
    }

    /// Prepare exact resolved-capture membership compaction.
    pub(crate) fn prepare_membership_compaction(
        &self,
        authorization: CodedMembershipCompactionAuthorization,
    ) -> Result<PreparedCodedMembershipCompaction, CodedCaptureError> {
        let capture = authorization.prior.capture;
        if self.capture_snapshot(capture).as_ref() != Some(&authorization.prior) {
            return Err(CodedCaptureError::PreparedTransitionStale(capture));
        }
        let current_record = self
            .captures
            .get(&capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        if !current_record.retained_history_revalidated {
            return Err(CodedCaptureError::RetentionSummaryInvalid(capture));
        }
        let mut proposed_record = current_record.clone();
        proposed_record.membership = authorization.proposed.membership.clone();
        proposed_record.pending_later_cuts = authorization.proposed.pending_later_cuts.clone();
        proposed_record.resolved_later_cuts = authorization.proposed.resolved_later_cuts.clone();
        proposed_record.release_authorized_operations =
            authorization.proposed.release_authorized_operations.clone();
        proposed_record.release_frontiers = authorization.proposed.release_frontiers.clone();
        proposed_record.release_certificates = authorization.proposed.release_certificates.clone();
        proposed_record.retained_frontier = authorization.proposed.retained_frontier;
        proposed_record.retained_history_revalidated = true;
        proposed_record.owner_revalidated = true;
        if proposed_record.snapshot(capture) != authorization.proposed {
            return Err(CodedCaptureError::RetentionSummaryInvalid(capture));
        }
        Ok(PreparedCodedMembershipCompaction {
            capture,
            expected_generation: authorization.expected_generation,
            prior: authorization.prior,
            proposed: authorization.proposed,
            proposed_record,
        })
    }

    /// Install exact compaction only after its proposed snapshot is durable.
    pub(crate) fn confirm_membership_compaction(
        &mut self,
        prepared: PreparedCodedMembershipCompaction,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        if self.capture_snapshot(prepared.capture).as_ref() != Some(&prepared.prior) {
            return Err(CodedCaptureError::PreparedTransitionStale(prepared.capture));
        }
        if receipt.expected_generation() != prepared.expected_generation
            || receipt.topology_epoch() != prepared.proposed.topology.topology_epoch()
            || prepared.expected_generation.checked_next() != Some(receipt.generation())
            || !receipt.committed_coded_capture(&prepared.proposed)
        {
            return Err(CodedCaptureError::DurableReceiptMismatch(prepared.capture));
        }
        self.captures
            .insert(prepared.capture, prepared.proposed_record);
        Ok(())
    }

    /// Prepare exact cleanup for one currently revalidated refused capture or
    /// one inherited Open capture with a current exact refusal.
    pub(crate) fn prepare_refused_cleanup(
        &self,
        authorization: CodedRefusedCleanupAuthorization,
    ) -> Result<PreparedCodedRefusedCleanup, CodedCaptureError> {
        let capture = authorization.prior.capture;
        let refusal_valid = match (authorization.prior.phase, authorization.refusal.as_ref()) {
            (CodedCapturePhase::Refused, None) => true,
            (CodedCapturePhase::Open, Some(refusal)) => {
                refusal.topology_epoch() == authorization.prior.topology.topology_epoch()
                    && refusal.generation() == authorization.expected_generation
                    && refusal.regions().iter().copied().collect::<BTreeSet<_>>()
                        == authorization.prior.dirty_regions
                    && refusal
                        .checksum_extents()
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>()
                        == authorization.prior.checksum_extents
            }
            _ => false,
        };
        if !refusal_valid {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        let expected_phase = authorization.prior.phase;
        self.prepare_capture_removal(
            authorization.expected_generation,
            authorization.prior,
            expected_phase,
            None,
            authorization.conservative_state,
        )
        .map(PreparedCodedRefusedCleanup)
    }

    /// Prepare exact cleanup for one owner-certified settled clean capture.
    pub(crate) fn prepare_clean_known_cleanup(
        &self,
        authorization: CodedCleanKnownCleanupAuthorization,
    ) -> Result<PreparedCodedCleanKnownCleanup, CodedCaptureError> {
        self.prepare_capture_removal(
            authorization.expected_generation,
            authorization.prior,
            CodedCapturePhase::CleanKnown,
            Some(authorization.clean_closure),
            None,
        )
        .map(PreparedCodedCleanKnownCleanup)
    }

    /// Prepare exact invalidation/removal for an inherited capture whose
    /// prior-process closure authority cannot be reconstructed.
    pub(crate) fn prepare_inherited_capture_abandonment(
        &self,
        authorization: CodedInheritedCaptureAbandonmentAuthorization,
    ) -> Result<PreparedCodedInheritedCaptureAbandonment, CodedCaptureError> {
        self.prepare_capture_removal(
            authorization.expected_generation,
            authorization.prior,
            CodedCapturePhase::CleanKnown,
            None,
            authorization.conservative_state,
        )
        .map(PreparedCodedInheritedCaptureAbandonment)
    }

    fn prepare_capture_removal(
        &self,
        expected_generation: RecoveryGeneration,
        prior: CodedCaptureSnapshot,
        expected_phase: CodedCapturePhase,
        clean_closure: Option<CodedCleanClosureEvidence>,
        conservative_state: Option<CodedConservativeStateProof>,
    ) -> Result<PreparedCodedCaptureRemoval, CodedCaptureError> {
        let capture = prior.capture;
        if self.capture_snapshot(capture).as_ref() != Some(&prior) || prior.phase != expected_phase
        {
            return Err(CodedCaptureError::PreparedTransitionStale(capture));
        }
        Ok(PreparedCodedCaptureRemoval {
            capture,
            expected_generation,
            prior,
            clean_closure,
            conservative_state,
        })
    }

    /// Install refused cleanup only after exact durable invalidation/removal.
    pub(crate) fn confirm_refused_cleanup(
        &mut self,
        prepared: PreparedCodedRefusedCleanup,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        let expected_phase = prepared.0.prior.phase;
        self.confirm_capture_removal(prepared.0, expected_phase, true, receipt)
    }

    /// Install clean-known cleanup only after exact durable removal.
    pub(crate) fn confirm_clean_known_cleanup(
        &mut self,
        prepared: PreparedCodedCleanKnownCleanup,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        self.confirm_capture_removal(prepared.0, CodedCapturePhase::CleanKnown, false, receipt)
    }

    /// Install inherited capture abandonment only after the exact atomic
    /// dirty invalidation and capture removal are durable.
    pub(crate) fn confirm_inherited_capture_abandonment(
        &mut self,
        prepared: PreparedCodedInheritedCaptureAbandonment,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        let expected_phase = prepared.0.prior.phase;
        self.confirm_capture_removal(prepared.0, expected_phase, true, receipt)
    }

    fn confirm_capture_removal(
        &mut self,
        prepared: PreparedCodedCaptureRemoval,
        expected_phase: CodedCapturePhase,
        requires_invalidation: bool,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        if self.capture_snapshot(prepared.capture).as_ref() != Some(&prepared.prior)
            || prepared.prior.phase != expected_phase
        {
            return Err(CodedCaptureError::PreparedTransitionStale(prepared.capture));
        }
        if receipt.expected_generation() != prepared.expected_generation
            || receipt.topology_epoch() != prepared.prior.topology.topology_epoch()
            || prepared.expected_generation.checked_next() != Some(receipt.generation())
            || !receipt.committed_coded_capture_removal(&prepared.removal())
            || (requires_invalidation
                && prepared.conservative_state.is_none()
                && (!prepared.prior.dirty_regions.iter().all(|region| {
                    receipt.committed_region_dirty(*region, prepared.expected_generation)
                }) || !prepared.prior.checksum_extents.iter().all(|extent| {
                    receipt.committed_integrity_stale(*extent, prepared.expected_generation)
                })))
        {
            return Err(CodedCaptureError::DurableReceiptMismatch(prepared.capture));
        }
        self.captures.remove(&prepared.capture);
        Ok(())
    }

    #[cfg(test)]
    fn apply_compaction(
        &mut self,
        summary: CodedCaptureRetentionSummary,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(summary.capture)?;
        if summary.topology_epoch != record.owner_facts.topology.topology_epoch()
            || summary.release_authorized_operations != summary.releasable_operations
            || !summary
                .releasable_operations
                .is_subset(&record.release_authorized_operations)
        {
            return Err(CodedCaptureError::RetentionSummaryInvalid(summary.capture));
        }
        if summary.frontier < record.retained_frontier {
            return Err(CodedCaptureError::RetentionFrontierStale(summary.capture));
        }
        if record.membership.keys().any(|operation| {
            !summary.releasable_operations.contains(operation)
                && !summary.retained_operations.contains(operation)
        }) || summary
            .retained_operations
            .iter()
            .any(|operation| !record.membership.contains_key(operation))
        {
            return Err(CodedCaptureError::RetentionSummaryInvalid(summary.capture));
        }
        for operation in &summary.releasable_operations {
            record.can_forget_operation(summary.capture, *operation)?;
        }
        for operation in summary.releasable_operations {
            record.membership.remove(&operation);
            record.release_authorized_operations.remove(&operation);
            record.release_frontiers.remove(&operation);
            record.release_certificates.remove(&operation);
        }
        record.retained_frontier = summary.frontier;
        record.retained_history_revalidated = true;
        Ok(())
    }

    #[cfg(test)]
    fn apply_retirement(
        &mut self,
        retirement: CodedCaptureRetirement,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(retirement.capture)?;
        if retirement.topology_epoch != record.owner_facts.topology.topology_epoch()
            || retirement.frontier < record.retained_frontier
            || (record.phase == CodedCapturePhase::CleanKnown
                && retirement.frontier
                    <= record
                        .owner_facts
                        .capture_frontier
                        .max(record.retained_frontier))
        {
            return Err(CodedCaptureError::RetirementEvidenceInvalid(
                retirement.capture,
            ));
        }
        let retirable = match record.phase {
            CodedCapturePhase::Refused => true,
            CodedCapturePhase::CleanKnown => {
                record.membership.is_empty() && record.pending_later_cuts.is_empty()
            }
            _ => false,
        };
        if !retirable {
            return Err(CodedCaptureError::CaptureNotRetirable(retirement.capture));
        }
        self.captures.remove(&retirement.capture);
        Ok(())
    }

    pub fn capture_owner_revalidated(&self, capture: CodedCaptureId) -> bool {
        self.captures
            .get(&capture)
            .is_some_and(|record| record.owner_revalidated)
    }
    pub fn capture_snapshot(&self, capture: CodedCaptureId) -> Option<CodedCaptureSnapshot> {
        self.captures
            .get(&capture)
            .map(|record| record.snapshot(capture))
    }

    /// Prepare one exact recovery-owner refusal without installing it.
    pub(crate) fn prepare_clean_refusal(
        &self,
        capture: CodedCaptureId,
        refusal: &RecoveryCleanRefusalPermit,
    ) -> Result<PreparedCodedRefusal, CodedCaptureError> {
        let prior = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        let mut candidate = self.fork_candidate();
        let record = candidate.record_mut(capture)?;
        if record.phase != CodedCapturePhase::Open
            || record.decision.is_some()
            || refusal.topology_epoch() != record.owner_facts.topology.topology_epoch()
            || refusal.generation() < record.owner_facts.recovery_generation
            || refusal.regions().iter().copied().collect::<BTreeSet<_>>()
                != record.owner_facts.dirty_regions
            || refusal
                .checksum_extents()
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != record.owner_facts.checksum_extents
        {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        record.decision = Some(CodedCaptureDecision::Rejected);
        record.phase = CodedCapturePhase::Refused;
        let proposed_record = record.clone();
        let proposed = proposed_record.snapshot(capture);
        Ok(PreparedCodedRefusal {
            capture,
            expected_generation: refusal.generation(),
            prior,
            proposed,
            proposed_record,
        })
    }

    /// Install a refused capture only after exact durable persistence.
    pub(crate) fn confirm_clean_refusal(
        &mut self,
        prepared: PreparedCodedRefusal,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        if self.capture_snapshot(prepared.capture).as_ref() != Some(&prepared.prior) {
            return Err(CodedCaptureError::PreparedTransitionStale(prepared.capture));
        }
        if receipt.expected_generation() != prepared.expected_generation
            || receipt.topology_epoch() != prepared.proposed.topology.topology_epoch()
            || prepared.expected_generation.checked_next() != Some(receipt.generation())
            || !receipt.committed_coded_capture(&prepared.proposed)
        {
            return Err(CodedCaptureError::DurableReceiptMismatch(prepared.capture));
        }
        self.captures
            .insert(prepared.capture, prepared.proposed_record);
        Ok(())
    }

    /// Prepare one accepted CLEAN result without installing it as durable state.
    pub(crate) fn prepare_clean_commit(
        &self,
        authorization: CodedCaptureCleanAuthorization,
    ) -> Result<PreparedCodedCleanCommit, CodedCaptureError> {
        let capture = authorization.accepted.capture;
        let prior = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        if authorization.authority_id != self.authority_id
            || !authorization
                .accepted
                .clean_authorization_applies_to(&prior)
            || !authorization
                .durable_prior
                .clean_authorization_applies_to(&prior)
        {
            return Err(CodedCaptureError::PreparedTransitionStale(capture));
        }
        let mut candidate = self.fork_candidate();
        let record = candidate.record_mut(capture)?;
        if !record.owner_revalidated || !record.mutation_set_closed() {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        record.decision = Some(CodedCaptureDecision::Accepted);
        record.phase = CodedCapturePhase::CleanKnown;
        let proposed_record = record.clone();
        let proposed = proposed_record.snapshot(capture);
        Ok(PreparedCodedCleanCommit {
            capture,
            expected_generation: authorization.permit.generation(),
            fence: authorization.permit.certificate().clone(),
            durable_prior: authorization.durable_prior,
            live_prior: prior,
            proposed,
            proposed_record,
        })
    }

    /// Install a prepared CLEAN only under the exact recovery-owner receipt.
    pub(crate) fn confirm_clean_commit(
        &mut self,
        prepared: PreparedCodedCleanCommit,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        if self.capture_snapshot(prepared.capture).as_ref() != Some(&prepared.live_prior) {
            return Err(CodedCaptureError::PreparedTransitionStale(prepared.capture));
        }
        let expected_transition = prepared.transition();
        if receipt.expected_generation() != prepared.expected_generation
            || receipt.topology_epoch() != prepared.proposed.topology.topology_epoch()
            || prepared.expected_generation.checked_next() != Some(receipt.generation())
            || !receipt.committed_coded_transition(&expected_transition)
        {
            return Err(CodedCaptureError::DurableReceiptMismatch(prepared.capture));
        }
        let closure_fence = receipt
            .persisted_fences()
            .last()
            .cloned()
            .ok_or(CodedCaptureError::DurableReceiptMismatch(prepared.capture))?;
        let mut proposed_record = prepared.proposed_record;
        proposed_record.clean_closure_fence = Some(closure_fence);
        self.captures.insert(prepared.capture, proposed_record);
        Ok(())
    }

    /// Prepare an owner-derived later cut without installing it as durable state.
    pub(crate) fn prepare_later_cut(
        &self,
        capture: CodedCaptureId,
        admission: &CodedAdmission,
        topology_epoch: TopologyEpoch,
        expected_generation: RecoveryGeneration,
    ) -> Result<PreparedCodedLaterCut, CodedCaptureError> {
        if !admission.belongs_to(self.admission_authority) {
            return Err(CodedCaptureError::ForeignAdmissionAuthority);
        }
        let committed_generation = expected_generation
            .checked_next()
            .ok_or(CodedCaptureError::LaterCutEvidenceInvalid(capture))?;
        let prior = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        let target = admission.invalidation_target().clone();
        let mut candidate = self.fork_candidate();
        let cut = CodedCaptureCut::new(
            topology_epoch,
            committed_generation,
            CodedCaptureFrontier::new(committed_generation, admission.sequence()),
        );
        let observation = match candidate.record_mut(capture)?.phase {
            CodedCapturePhase::Open => CodedLaterCutObservation::DurableStalesClean,
            CodedCapturePhase::CleanKnown => CodedLaterCutObservation::DurableAfterClean,
            CodedCapturePhase::Refused => {
                return Err(CodedCaptureError::CaptureNotOpen(capture));
            }
            CodedCapturePhase::CleanCommitPending | CodedCapturePhase::CleanCommitUnknown => {
                return Err(CodedCaptureError::LaterCutOrder(capture));
            }
        };
        candidate.apply_later_cut(capture, admission.operation(), cut, observation)?;
        let proposed_record = candidate.record_mut(capture)?.clone();
        let proposed = proposed_record.snapshot(capture);
        Ok(PreparedCodedLaterCut {
            capture,
            operation: admission.operation(),
            target,
            expected_generation,
            prior,
            proposed,
            proposed_record,
        })
    }

    pub(crate) fn confirm_later_cut(
        &mut self,
        prepared: PreparedCodedLaterCut,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        if self.capture_snapshot(prepared.capture).as_ref() != Some(&prepared.prior) {
            return Err(CodedCaptureError::PreparedTransitionStale(prepared.capture));
        }
        if receipt.expected_generation() != prepared.expected_generation
            || receipt.topology_epoch() != prepared.proposed.topology.topology_epoch()
            || prepared.expected_generation.checked_next() != Some(receipt.generation())
            || !receipt.committed_coded_transition(&prepared.transition())
        {
            return Err(CodedCaptureError::DurableReceiptMismatch(prepared.capture));
        }
        self.captures
            .insert(prepared.capture, prepared.proposed_record);
        Ok(())
    }

    /// Apply one exact resolution issued by recovery inspection.
    fn apply_reconciliation(
        &mut self,
        receipt: CodedCaptureReconciliationReceipt,
    ) -> Result<(), CodedCaptureError> {
        let capture = receipt.capture();
        if self.capture_snapshot(capture).as_ref() != Some(&receipt.expected_unknown)
            || !receipt.resolved.validates_internal_state()
        {
            return Err(CodedCaptureError::DurableReceiptMismatch(capture));
        }
        self.restore_snapshot_state(receipt.resolved)
    }

    #[cfg(test)]
    fn observe_decision(
        &mut self,
        capture: CodedCaptureId,
        decision: CodedCaptureDecision,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.phase != CodedCapturePhase::Open {
            return Err(CodedCaptureError::CaptureNotOpen(capture));
        }
        if record.decision.is_some() {
            return Err(CodedCaptureError::DecisionAlreadyObserved(capture));
        }
        record.decision = Some(decision);
        if decision == CodedCaptureDecision::Rejected {
            record.phase = CodedCapturePhase::Refused;
        }
        Ok(())
    }

    #[cfg(test)]
    fn request_clean(&mut self, capture: CodedCaptureId) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.phase != CodedCapturePhase::Open {
            return Err(CodedCaptureError::CaptureNotOpen(capture));
        }
        if !record.clean_eligible() {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        record.phase = CodedCapturePhase::CleanCommitPending;
        Ok(())
    }

    #[cfg(test)]
    fn observe_clean_commit(
        &mut self,
        capture: CodedCaptureId,
        observation: CodedCleanCommitObservation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.phase != CodedCapturePhase::CleanCommitPending {
            return Err(CodedCaptureError::CleanCommitNotPending(capture));
        }
        if observation == CodedCleanCommitObservation::Durable && !record.clean_eligible() {
            return Err(CodedCaptureError::CleanNotEligible(capture));
        }
        if observation == CodedCleanCommitObservation::Rejected {
            record.decision = Some(CodedCaptureDecision::Rejected);
        }
        record.phase = match observation {
            CodedCleanCommitObservation::Durable => CodedCapturePhase::CleanKnown,
            CodedCleanCommitObservation::Rejected => CodedCapturePhase::Refused,
            CodedCleanCommitObservation::Unknown => CodedCapturePhase::CleanCommitUnknown,
        };
        Ok(())
    }

    fn apply_later_cut(
        &mut self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        cut: CodedCaptureCut,
        observation: CodedLaterCutObservation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if !record.tracks_future() {
            return Err(CodedCaptureError::CaptureNotOpen(capture));
        }
        if record.membership.get(&operation) != Some(&CodedCaptureMembership::Later) {
            return Err(CodedCaptureError::LaterOperationNotMember { capture, operation });
        }
        if matches!(
            observation,
            CodedLaterCutObservation::DurableAfterClean
                | CodedLaterCutObservation::DurableStalesClean
                | CodedLaterCutObservation::Unknown
        ) && !record.accepts_later_cut(cut)
        {
            return Err(CodedCaptureError::LaterCutEvidenceInvalid(capture));
        }
        if observation == CodedLaterCutObservation::DurableAfterClean
            && record.phase != CodedCapturePhase::CleanKnown
        {
            return Err(CodedCaptureError::LaterCutOrder(capture));
        }
        if observation == CodedLaterCutObservation::DurableStalesClean
            && record.phase == CodedCapturePhase::CleanKnown
        {
            return Err(CodedCaptureError::LaterCutOrder(capture));
        }
        let membership = match observation {
            CodedLaterCutObservation::DurableAfterClean => {
                record.resolved_later_cuts.insert(operation, cut);
                record.retirement_frontier = Some(
                    record
                        .retirement_frontier
                        .map_or(cut.frontier, |frontier| frontier.max(cut.frontier)),
                );
                CodedCaptureMembership::LaterDurableAfterClean
            }
            CodedLaterCutObservation::DurableStalesClean => {
                record.resolved_later_cuts.insert(operation, cut);
                record.retirement_frontier = Some(
                    record
                        .retirement_frontier
                        .map_or(cut.frontier, |frontier| frontier.max(cut.frontier)),
                );
                record.phase = CodedCapturePhase::Refused;
                CodedCaptureMembership::LaterDurableStalesClean
            }
            CodedLaterCutObservation::Rejected => CodedCaptureMembership::LaterRejected,
            CodedLaterCutObservation::Unknown => {
                record.pending_later_cuts.insert(operation, cut);
                CodedCaptureMembership::LaterUnknown
            }
        };
        record.membership.insert(operation, membership);
        Ok(())
    }

    #[cfg(test)]
    fn observe_later_cut(
        &mut self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        cut: CodedCaptureCut,
        observation: CodedLaterCutObservation,
    ) -> Result<(), CodedCaptureError> {
        self.apply_later_cut(capture, operation, cut, observation)
    }

    #[cfg(test)]
    fn reconcile_later_cut(
        &mut self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        observation: CodedLaterCutReconciliation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.membership.get(&operation) != Some(&CodedCaptureMembership::LaterUnknown)
            || !record.pending_later_cuts.contains_key(&operation)
        {
            return Err(CodedCaptureError::LaterCutNotPending { capture, operation });
        }
        if observation == CodedLaterCutReconciliation::DurableAfterClean
            && record.phase != CodedCapturePhase::CleanKnown
        {
            return Err(CodedCaptureError::LaterCutOrder(capture));
        }
        if observation == CodedLaterCutReconciliation::StalesClean
            && record.phase == CodedCapturePhase::CleanKnown
        {
            return Err(CodedCaptureError::LaterCutOrder(capture));
        }
        let cut = record
            .pending_later_cuts
            .remove(&operation)
            .ok_or(CodedCaptureError::LaterCutNotPending { capture, operation })?;
        let membership = match observation {
            CodedLaterCutReconciliation::DurableAfterClean => {
                record.resolved_later_cuts.insert(operation, cut);
                record.retirement_frontier = Some(
                    record
                        .retirement_frontier
                        .map_or(cut.frontier, |frontier| frontier.max(cut.frontier)),
                );
                CodedCaptureMembership::LaterDurableAfterClean
            }
            CodedLaterCutReconciliation::StalesClean => {
                record.resolved_later_cuts.insert(operation, cut);
                record.retirement_frontier = Some(
                    record
                        .retirement_frontier
                        .map_or(cut.frontier, |frontier| frontier.max(cut.frontier)),
                );
                record.phase = CodedCapturePhase::Refused;
                CodedCaptureMembership::LaterDurableStalesClean
            }
            CodedLaterCutReconciliation::Rejected => CodedCaptureMembership::LaterRejected,
        };
        record.membership.insert(operation, membership);
        Ok(())
    }

    #[cfg(test)]
    fn start_capture_unchecked(
        &mut self,
        capture: CodedCaptureId,
        owner_facts: CodedCaptureOwnerFacts,
        active_claims: impl IntoIterator<Item = (OperationSlotToken, BTreeSet<CodedUnitId>)>,
    ) -> Result<(), CodedCaptureError> {
        self.insert_capture(capture, owner_facts, active_claims)
    }

    #[cfg(test)]
    fn observe_admitted_claim_unchecked(
        &mut self,
        operation: OperationSlotToken,
        units: &BTreeSet<CodedUnitId>,
    ) {
        for capture in self.captures.values_mut() {
            if capture.tracks_future() && capture.intersects(units) {
                capture
                    .membership
                    .insert(operation, CodedCaptureMembership::Later);
            }
        }
    }

    #[cfg(test)]
    fn compact_capture_unchecked(
        &mut self,
        summary: CodedCaptureRetentionSummary,
    ) -> Result<(), CodedCaptureError> {
        self.apply_compaction(summary)
    }

    #[cfg(test)]
    fn retire_capture_unchecked(
        &mut self,
        retirement: CodedCaptureRetirement,
    ) -> Result<(), CodedCaptureError> {
        self.apply_retirement(retirement)
    }

    #[cfg(test)]
    fn prepare_membership_compaction_unchecked(
        &self,
        capture: CodedCaptureId,
        expected_generation: RecoveryGeneration,
    ) -> Result<Option<PreparedCodedMembershipCompaction>, CodedCaptureError> {
        let prior = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        assert!(prior.validates_internal_state(), "{prior:#?}");
        let mut recovery = crate::MemoryRecoveryStore::new(TopologyEpoch(1))
            .snapshot()
            .clone();
        recovery.generation = expected_generation;
        recovery.coded_captures = self.snapshots();
        let Some(authorization) =
            CodedCaptureRetentionOwner::authorize_membership_compaction(&recovery, capture)?
        else {
            return Ok(None);
        };
        self.prepare_membership_compaction(authorization).map(Some)
    }

    #[cfg(test)]
    fn prepare_refused_cleanup_unchecked(
        &self,
        capture: CodedCaptureId,
        expected_generation: RecoveryGeneration,
    ) -> Result<PreparedCodedRefusedCleanup, CodedCaptureError> {
        let prior = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        self.prepare_refused_cleanup(CodedRefusedCleanupAuthorization {
            expected_generation,
            prior,
            refusal: None,
            conservative_state: None,
        })
    }

    #[cfg(test)]
    fn prepare_clean_known_cleanup_unchecked(
        &self,
        capture: CodedCaptureId,
        expected_generation: RecoveryGeneration,
    ) -> Result<PreparedCodedCleanKnownCleanup, CodedCaptureError> {
        let prior = self
            .capture_snapshot(capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))?;
        self.prepare_clean_known_cleanup(CodedCleanKnownCleanupAuthorization {
            expected_generation,
            prior,
            clean_closure: CodedCleanClosureEvidence {
                regions: Vec::new(),
                fences: Vec::new(),
                closure_fence: FenceCertificate::new(
                    TopologyEpoch(0),
                    dwv_store::FenceDomain(0),
                    Vec::new(),
                    Vec::new(),
                ),
            },
        })
    }

    #[cfg(test)]
    fn reconcile_later_cut_unchecked(
        &mut self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        cut: CodedCaptureCut,
        observation: CodedLaterCutReconciliation,
    ) -> Result<(), CodedCaptureError> {
        let record = self.record_mut(capture)?;
        if record.pending_later_cuts.get(&operation).copied() != Some(cut) {
            return Err(CodedCaptureError::LaterCutMismatch { capture, operation });
        }
        self.reconcile_later_cut(capture, operation, observation)
    }

    fn record_mut(
        &mut self,
        capture: CodedCaptureId,
    ) -> Result<&mut CaptureRecord, CodedCaptureError> {
        self.captures
            .get_mut(&capture)
            .ok_or(CodedCaptureError::CaptureNotFound(capture))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BLAKE3_256_PROFILE, DIRTY_REGION_BYTES, MemoryRecoveryStore, RecoveryCleanDecision,
        RecoveryCleanRequest, RecoveryError, RecoveryMutation, RecoveryStateStore, RecoveryTxn,
        TransitionError, evaluate_recovery_clean,
    };

    fn token(index: u32) -> OperationSlotToken {
        OperationSlotToken::new(index, 1)
    }
    fn live_lifecycle_authorization(
        owner: &LifecycleAuthorityOwner,
        operation: OperationSlotToken,
    ) -> IncludedLifecycleAuthorization {
        use dwv_core::{
            BlockOp, BlockRequest, BufferToken, ByteRange, DurabilityIntent, FrontendId,
            OrderingIntent, RequestId, SlotId, SubmissionSequence,
        };
        use dwv_store::{OperationSlotTable, ResourceLimits};

        let range = ByteRange::new(0, 1).unwrap();
        let mut operations = OperationSlotTable::new(ResourceLimits::new(1, 1, 1, 1, 1, 1));
        let reserved = operations
            .reserve(BlockRequest::new(
                RequestId(1),
                FrontendId(1),
                SlotId::from_bytes([1; 16]),
                TopologyEpoch(1),
                BlockOp::Write,
                range,
                Some(BufferToken::new(0, 1)),
                OrderingIntent {
                    submission_sequence: SubmissionSequence(1),
                    preflush: false,
                    fence_domain: dwv_store::FenceDomain(1),
                },
                DurabilityIntent::Ordinary,
            ))
            .unwrap();
        assert_eq!(reserved, operation);
        let child = operations.register_child(operation, range).unwrap();
        operations.refuse_submission(operation, child).unwrap();
        owner
            .authorize_included_live(&operations, operation, true)
            .expect("terminal operation produces live lifecycle evidence")
    }

    fn release(operation: OperationSlotToken) -> CodedClaimRelease {
        CodedClaimRelease::for_test(operation, u64::from(operation.index) + 1)
    }

    fn fence_certificate() -> FenceCertificate {
        FenceCertificate::new(
            TopologyEpoch(1),
            dwv_store::FenceDomain(1),
            vec![dwv_store::StoreFenceRef {
                fence_id: dwv_store::FenceId(1),
                store_id: dwv_store::StoreId(1),
                topology_epoch: TopologyEpoch(1),
                store_incarnation: dwv_store::StoreIncarnationId(1),
                through: dwv_store::StoreWriteWatermark(1),
                capability_evidence_id: dwv_store::CapabilityEvidenceId(1),
            }],
            vec![(RegionId(0), RecoveryGeneration::ZERO)],
        )
        .with_integrity_extent(IntegrityExtentId(0), RecoveryGeneration::ZERO)
    }

    fn units(units: impl IntoIterator<Item = u32>) -> BTreeSet<CodedUnitId> {
        units
            .into_iter()
            .map(|unit| CodedUnitId(u64::from(unit)))
            .collect()
    }
    fn owner_facts(scope: ValidatedCodedCaptureScope) -> CodedCaptureOwnerFacts {
        use dwv_core::{
            ArrayId, AssignmentGeneration, AssignmentInstanceId, CodingPosition, CodingProfile,
            MemberRole, ProtectedGeometry, SlotId, TopologyAssignment, TopologyEpoch,
        };

        let topology = TopologySnapshot::new(
            ArrayId([1; 16]),
            TopologyEpoch(1),
            CodingProfile::new(1, 1).unwrap(),
            ProtectedGeometry::new(4096, 512).unwrap(),
            vec![
                TopologyAssignment::new(
                    SlotId::from_bytes([1; 16]),
                    MemberRole::Data,
                    CodingPosition(0),
                    AssignmentInstanceId::from_bytes([2; 16]),
                    AssignmentGeneration(1),
                ),
                TopologyAssignment::new(
                    SlotId::from_bytes([2; 16]),
                    MemberRole::Parity,
                    CodingPosition(1),
                    AssignmentInstanceId::from_bytes([3; 16]),
                    AssignmentGeneration(1),
                ),
            ],
        )
        .unwrap();
        CodedCaptureOwnerFacts::new(
            topology,
            RecoveryGeneration::ZERO,
            ChecksumProfileId(1),
            ChecksumSetGeneration::INITIAL,
            scope,
            [RegionId(0)],
            [IntegrityExtentId(0)],
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 0),
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 1),
        )
    }

    fn later_cut() -> CodedCaptureCut {
        CodedCaptureCut::new(
            dwv_core::TopologyEpoch(1),
            RecoveryGeneration(1),
            CodedCaptureFrontier::new(RecoveryGeneration(1), 2),
        )
    }

    #[test]
    fn selected_scope_unions_block_dirty_region_and_checksum_extent_geometry() {
        use dwv_core::{
            ArrayId, AssignmentGeneration, AssignmentInstanceId, CodingPosition, CodingProfile,
            MemberRole, ProtectedGeometry, SlotId, TopologyAssignment, TopologyEpoch,
        };

        let topology = TopologySnapshot::new(
            ArrayId([4; 16]),
            TopologyEpoch(1),
            CodingProfile::new(1, 1).unwrap(),
            ProtectedGeometry::new(8 * 1024 * 1024, 512).unwrap(),
            vec![
                TopologyAssignment::new(
                    SlotId::from_bytes([1; 16]),
                    MemberRole::Data,
                    CodingPosition(0),
                    AssignmentInstanceId::from_bytes([2; 16]),
                    AssignmentGeneration(1),
                ),
                TopologyAssignment::new(
                    SlotId::from_bytes([2; 16]),
                    MemberRole::Parity,
                    CodingPosition(1),
                    AssignmentInstanceId::from_bytes([3; 16]),
                    AssignmentGeneration(1),
                ),
            ],
        )
        .unwrap();
        let scope = CodedCaptureOwnerFacts::selected_invalidation_scope(
            &topology,
            [RegionId(1025)],
            [IntegrityExtentId(0)],
            DIRTY_REGION_BYTES,
            BLAKE3_256_PROFILE,
        )
        .unwrap();

        assert_eq!(scope.units().len(), 8_200);
        assert!(scope.units().contains(&CodedUnitId(0)));
        assert!(scope.units().contains(&CodedUnitId(8_191)));
        assert!(!scope.units().contains(&CodedUnitId(8_192)));
        assert!(!scope.units().contains(&CodedUnitId(8_199)));
        assert!(scope.units().contains(&CodedUnitId(8_200)));
        assert!(scope.units().contains(&CodedUnitId(8_207)));
    }
    #[test]
    fn capture_scope_matches_independent_block_overlap_oracle() {
        use dwv_core::{
            ArrayId, AssignmentGeneration, AssignmentInstanceId, CodingPosition, CodingProfile,
            MemberRole, ProtectedGeometry, SlotId, TopologyAssignment, TopologyEpoch,
        };

        const PROTECTED_LENGTH: u64 = 8 * 1024 * 1024;
        const LOGICAL_BLOCK: u64 = 512;
        let topology = TopologySnapshot::new(
            ArrayId([5; 16]),
            TopologyEpoch(1),
            CodingProfile::new(1, 1).unwrap(),
            ProtectedGeometry::new(PROTECTED_LENGTH, LOGICAL_BLOCK as u32).unwrap(),
            vec![
                TopologyAssignment::new(
                    SlotId::from_bytes([1; 16]),
                    MemberRole::Data,
                    CodingPosition(0),
                    AssignmentInstanceId::from_bytes([2; 16]),
                    AssignmentGeneration(1),
                ),
                TopologyAssignment::new(
                    SlotId::from_bytes([2; 16]),
                    MemberRole::Parity,
                    CodingPosition(1),
                    AssignmentInstanceId::from_bytes([3; 16]),
                    AssignmentGeneration(1),
                ),
            ],
        )
        .unwrap();
        let geometry =
            CodedGeometryOwner::new(topology, DIRTY_REGION_BYTES, BLAKE3_256_PROFILE).unwrap();
        let extents_per_member = PROTECTED_LENGTH.div_ceil(BLAKE3_256_PROFILE.extent_size);
        let cases = [
            (vec![RegionId(0)], vec![]),
            (vec![RegionId(1023)], vec![]),
            (vec![RegionId(1024)], vec![]),
            (vec![], vec![IntegrityExtentId(0)]),
            (vec![], vec![IntegrityExtentId(1)]),
            (vec![RegionId(0)], vec![IntegrityExtentId(0)]),
            (vec![RegionId(0)], vec![IntegrityExtentId(1)]),
            (
                vec![RegionId(1_u64 << 32)],
                vec![IntegrityExtentId(extents_per_member)],
            ),
        ];

        for (dirty_regions, checksum_extents) in cases {
            let actual = geometry
                .capture_scope(
                    dirty_regions.iter().copied(),
                    checksum_extents.iter().copied(),
                )
                .unwrap();
            let mut selected_ranges = dirty_regions
                .iter()
                .map(|region| {
                    let index = region.0 & u64::from(u32::MAX);
                    let start = index * DIRTY_REGION_BYTES;
                    (start, (start + DIRTY_REGION_BYTES).min(PROTECTED_LENGTH))
                })
                .collect::<Vec<_>>();
            selected_ranges.extend(checksum_extents.iter().map(|extent| {
                let index = extent.0 % extents_per_member;
                let start = index * BLAKE3_256_PROFILE.extent_size;
                (
                    start,
                    (start + BLAKE3_256_PROFILE.extent_size).min(PROTECTED_LENGTH),
                )
            }));
            let expected = (0..PROTECTED_LENGTH.div_ceil(LOGICAL_BLOCK))
                .filter(|unit| {
                    let start = unit * LOGICAL_BLOCK;
                    let end = (start + LOGICAL_BLOCK).min(PROTECTED_LENGTH);
                    selected_ranges
                        .iter()
                        .any(|(selected_start, selected_end)| {
                            start < *selected_end && *selected_start < end
                        })
                })
                .map(CodedUnitId)
                .collect::<BTreeSet<_>>();

            assert_eq!(actual.units(), &expected);
        }
    }

    #[test]
    fn capture_rejects_incoherent_frontier_owner_facts() {
        let capture = CodedCaptureId(8);
        let scope = ValidatedCodedCaptureScope::complete([CodedUnitId(0)]);

        let mut mismatched_generation = owner_facts(scope.clone());
        mismatched_generation.capture_frontier =
            CodedCaptureFrontier::new(RecoveryGeneration(1), 1);
        let mut captures = CodedCaptureCoordinator::new();
        assert_eq!(
            captures.start_capture_unchecked(capture, mismatched_generation, []),
            Err(CodedCaptureError::CaptureFrontierGenerationMismatch(
                capture
            ))
        );

        let mut reversed = owner_facts(scope);
        reversed.lower_frontier = CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 2);
        let mut captures = CodedCaptureCoordinator::new();
        assert_eq!(
            captures.start_capture_unchecked(capture, reversed, []),
            Err(CodedCaptureError::FrontierOrderInvalid(capture))
        );
    }

    #[test]
    fn older_recovery_generation_is_not_a_newer_later_cut_even_with_a_larger_sequence() {
        let capture = CodedCaptureId(9);
        let operation = token(4);
        let mut facts = owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)]));
        facts.recovery_generation = RecoveryGeneration(1);
        facts.lower_frontier = CodedCaptureFrontier::new(RecoveryGeneration(1), 0);
        facts.capture_frontier = CodedCaptureFrontier::new(RecoveryGeneration(1), 1);

        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(capture, facts, [])
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
            .unwrap();
        captures.observe_admitted_claim_unchecked(operation, &units([0]));

        let older_generation = CodedCaptureCut::new(
            dwv_core::TopologyEpoch(1),
            RecoveryGeneration::ZERO,
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, u64::MAX),
        );
        assert_eq!(
            captures.observe_later_cut(
                capture,
                operation,
                older_generation,
                CodedLaterCutObservation::DurableAfterClean,
            ),
            Err(CodedCaptureError::LaterCutEvidenceInvalid(capture))
        );
        assert!(!captures.effect_allowed(operation));
    }

    #[test]
    fn retention_summary_bounds_slot_reuse_and_preserves_unknown_until_reconciled() {
        let capture = CodedCaptureId(10);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
            .unwrap();

        for generation in 1..=32 {
            let operation = OperationSlotToken::new(0, generation);
            let frontier =
                CodedCaptureFrontier::new(RecoveryGeneration::ZERO, u64::from(generation) + 1);
            captures.observe_admitted_claim_unchecked(operation, &units([0]));
            captures
                .observe_later_cut(
                    capture,
                    operation,
                    CodedCaptureCut::new(
                        dwv_core::TopologyEpoch(1),
                        RecoveryGeneration::ZERO,
                        frontier,
                    ),
                    CodedLaterCutObservation::DurableAfterClean,
                )
                .unwrap();
            assert!(
                captures
                    .observe_release_allowed(&release(operation), Some(&fence_certificate()))
                    .unwrap()
            );
            captures
                .compact_capture_unchecked(
                    CodedCaptureRetentionSummary::new(
                        capture,
                        dwv_core::TopologyEpoch(1),
                        frontier,
                        [operation],
                    )
                    .with_release_authorized_operations([operation]),
                )
                .unwrap();
            let snapshot = captures.capture_snapshot(capture).unwrap();
            assert!(snapshot.membership.is_empty());
            assert_eq!(snapshot.retained_frontier, frontier);
        }

        let live = OperationSlotToken::new(2, 1);
        captures.observe_admitted_claim_unchecked(live, &units([0]));
        assert_eq!(
            captures.compact_capture_unchecked(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                later_cut().frontier,
                [],
            )),
            Err(CodedCaptureError::RetentionSummaryInvalid(capture))
        );
        captures
            .compact_capture_unchecked(
                CodedCaptureRetentionSummary::new(
                    capture,
                    dwv_core::TopologyEpoch(1),
                    later_cut().frontier,
                    [],
                )
                .with_retained_operations([live]),
            )
            .unwrap();
        assert_eq!(
            captures
                .capture_snapshot(capture)
                .unwrap()
                .membership
                .get(&live),
            Some(&CodedCaptureMembership::Later)
        );
        captures
            .observe_later_cut(
                capture,
                live,
                later_cut(),
                CodedLaterCutObservation::Rejected,
            )
            .unwrap();
        assert!(!captures.effect_allowed(live));
        assert_eq!(
            captures.compact_capture_unchecked(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                later_cut().frontier,
                [live],
            )),
            Err(CodedCaptureError::RetentionSummaryInvalid(capture))
        );
        assert!(!captures.effect_allowed(live));
        assert!(
            captures
                .observe_release_allowed(&release(live), Some(&fence_certificate()))
                .unwrap()
        );
        captures
            .compact_capture_unchecked(
                CodedCaptureRetentionSummary::new(
                    capture,
                    dwv_core::TopologyEpoch(1),
                    later_cut().frontier,
                    [live],
                )
                .with_release_authorized_operations([live]),
            )
            .unwrap();
        assert!(captures.effect_allowed(live));

        let unknown_cut = CodedCaptureCut::new(
            dwv_core::TopologyEpoch(1),
            RecoveryGeneration(1),
            CodedCaptureFrontier::new(RecoveryGeneration(1), 3),
        );
        let unresolved = OperationSlotToken::new(1, 1);
        captures.observe_admitted_claim_unchecked(unresolved, &units([0]));
        captures
            .observe_later_cut(
                capture,
                unresolved,
                unknown_cut,
                CodedLaterCutObservation::Unknown,
            )
            .unwrap();
        assert_eq!(
            captures.compact_capture_unchecked(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                unknown_cut.frontier,
                [],
            )),
            Err(CodedCaptureError::RetentionSummaryInvalid(capture))
        );
        let retirement_frontier = CodedCaptureFrontier::new(RecoveryGeneration(1), 5);
        let newer_cut = CodedCaptureCut::new(
            dwv_core::TopologyEpoch(1),
            RecoveryGeneration(1),
            CodedCaptureFrontier::new(RecoveryGeneration(1), 4),
        );
        captures
            .compact_capture_unchecked(
                CodedCaptureRetentionSummary::new(
                    capture,
                    dwv_core::TopologyEpoch(1),
                    newer_cut.frontier,
                    [],
                )
                .with_retained_operations([unresolved]),
            )
            .unwrap();
        assert_eq!(
            captures
                .capture_snapshot(capture)
                .unwrap()
                .pending_later_cuts
                .get(&unresolved),
            Some(&unknown_cut)
        );
        assert_eq!(
            captures.reconcile_later_cut_unchecked(
                capture,
                unresolved,
                newer_cut,
                CodedLaterCutReconciliation::Rejected,
            ),
            Err(CodedCaptureError::LaterCutMismatch {
                capture,
                operation: unresolved,
            })
        );
        assert_eq!(
            captures.reconcile_later_cut_unchecked(
                capture,
                unresolved,
                unknown_cut,
                CodedLaterCutReconciliation::Rejected,
            ),
            Ok(())
        );

        assert_eq!(
            captures.compact_capture_unchecked(CodedCaptureRetentionSummary::new(
                capture,
                dwv_core::TopologyEpoch(1),
                CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 2),
                [],
            )),
            Err(CodedCaptureError::RetentionFrontierStale(capture))
        );
        assert_eq!(
            captures.retire_capture_unchecked(CodedCaptureRetirement::new(
                capture,
                dwv_core::TopologyEpoch(1),
                retirement_frontier,
            )),
            Err(CodedCaptureError::CaptureNotRetirable(capture))
        );
        assert!(
            captures
                .observe_release_allowed(&release(unresolved), Some(&fence_certificate()))
                .unwrap()
        );

        captures
            .compact_capture_unchecked(
                CodedCaptureRetentionSummary::new(
                    capture,
                    dwv_core::TopologyEpoch(1),
                    newer_cut.frontier,
                    [unresolved],
                )
                .with_release_authorized_operations([unresolved]),
            )
            .unwrap();
        captures
            .retire_capture_unchecked(CodedCaptureRetirement::new(
                capture,
                dwv_core::TopologyEpoch(1),
                retirement_frontier,
            ))
            .unwrap();
        assert!(captures.capture_snapshot(capture).is_none());
    }

    #[test]
    fn capture_rejects_an_empty_internal_scope() {
        let mut captures = CodedCaptureCoordinator::new();
        assert_eq!(
            captures.start_capture_unchecked(
                CodedCaptureId(0),
                owner_facts(ValidatedCodedCaptureScope::complete([])),
                [],
            ),
            Err(CodedCaptureError::EmptyScope)
        );
        assert!(captures.capture_snapshot(CodedCaptureId(0)).is_none());
    }

    #[test]
    fn capture_records_included_and_later_membership() {
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                CodedCaptureId(0),
                owner_facts(ValidatedCodedCaptureScope::complete([
                    CodedUnitId(0),
                    CodedUnitId(1),
                ])),
                [(token(0), units([0]))],
            )
            .unwrap();
        captures.observe_admitted_claim_unchecked(token(1), &units([1]));
        let snapshot = captures.capture_snapshot(CodedCaptureId(0)).unwrap();
        assert_eq!(
            snapshot.membership.get(&token(0)),
            Some(&CodedCaptureMembership::Included)
        );
        assert_eq!(
            snapshot.membership.get(&token(1)),
            Some(&CodedCaptureMembership::Later)
        );
    }

    #[test]
    fn capture_binds_owner_facts_and_requires_a_newer_durable_cut() {
        let capture = CodedCaptureId(7);
        let operation = token(3);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [],
            )
            .unwrap();

        let snapshot = captures.capture_snapshot(capture).unwrap();
        assert_eq!(snapshot.capture, capture);
        assert_eq!(
            snapshot.topology.topology_epoch(),
            dwv_core::TopologyEpoch(1)
        );
        assert_eq!(snapshot.recovery_generation, RecoveryGeneration::ZERO);
        assert_eq!(snapshot.checksum_profile, ChecksumProfileId(1));
        assert_eq!(
            snapshot.checksum_set_generation,
            ChecksumSetGeneration::INITIAL
        );
        assert!(snapshot.dirty_regions.contains(&RegionId(0)));
        assert!(snapshot.checksum_extents.contains(&IntegrityExtentId(0)));
        assert_eq!(
            snapshot.lower_frontier,
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 0)
        );
        assert_eq!(
            snapshot.capture_frontier,
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 1)
        );

        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
            .unwrap();
        captures.observe_admitted_claim_unchecked(operation, &units([0]));

        let stale_cut = CodedCaptureCut::new(
            dwv_core::TopologyEpoch(1),
            RecoveryGeneration::ZERO,
            CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 1),
        );
        assert_eq!(
            captures.observe_later_cut(
                capture,
                operation,
                stale_cut,
                CodedLaterCutObservation::DurableAfterClean,
            ),
            Err(CodedCaptureError::LaterCutEvidenceInvalid(capture))
        );
        assert!(!captures.effect_allowed(operation));

        captures
            .observe_later_cut(
                capture,
                operation,
                later_cut(),
                CodedLaterCutObservation::DurableAfterClean,
            )
            .unwrap();
        assert!(captures.effect_allowed(operation));
    }

    #[test]
    fn rejected_capture_does_not_classify_future_admissions() {
        let mut captures = CodedCaptureCoordinator::new();
        let capture = CodedCaptureId(0);
        let inherited = token(0);
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [(inherited, units([0]))],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Rejected)
            .unwrap();
        captures.observe_admitted_claim_unchecked(token(1), &units([0]));
        assert!(
            !captures
                .capture_snapshot(capture)
                .unwrap()
                .membership
                .contains_key(&token(1))
        );
        assert!(captures.effect_allowed(token(1)));
        assert_eq!(
            captures
                .capture_snapshot(capture)
                .unwrap()
                .membership
                .get(&inherited),
            Some(&CodedCaptureMembership::Included)
        );
        captures
            .retire_capture_unchecked(CodedCaptureRetirement::new(
                capture,
                dwv_core::TopologyEpoch(1),
                CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 0),
            ))
            .unwrap();
        assert!(captures.capture_snapshot(capture).is_none());
    }
    #[test]
    fn every_applicable_capture_must_satisfy_its_later_cut_before_effect() {
        let mut captures = CodedCaptureCoordinator::new();
        for capture in [CodedCaptureId(0), CodedCaptureId(1)] {
            captures
                .start_capture_unchecked(
                    capture,
                    owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                    [],
                )
                .unwrap();
            captures
                .observe_decision(capture, CodedCaptureDecision::Accepted)
                .unwrap();
            captures.request_clean(capture).unwrap();
            captures
                .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
                .unwrap();
        }

        let later = token(1);
        captures.observe_admitted_claim_unchecked(later, &units([0]));
        assert!(!captures.effect_allowed(later));

        captures
            .observe_later_cut(
                CodedCaptureId(0),
                later,
                later_cut(),
                CodedLaterCutObservation::DurableAfterClean,
            )
            .unwrap();
        assert!(
            !captures.effect_allowed(later),
            "satisfying one of two overlapping applicable capture cuts must not permit the effect"
        );

        captures
            .observe_later_cut(
                CodedCaptureId(1),
                later,
                later_cut(),
                CodedLaterCutObservation::DurableAfterClean,
            )
            .unwrap();
        assert!(captures.effect_allowed(later));
    }

    #[test]
    fn disjoint_active_capture_does_not_gate_later_effect() {
        let mut captures = CodedCaptureCoordinator::new();
        for (capture, unit) in [(CodedCaptureId(0), 0), (CodedCaptureId(1), 1)] {
            captures
                .start_capture_unchecked(
                    capture,
                    owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(unit)])),
                    [],
                )
                .unwrap();
            captures
                .observe_decision(capture, CodedCaptureDecision::Accepted)
                .unwrap();
            captures.request_clean(capture).unwrap();
            captures
                .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
                .unwrap();
        }

        let later = token(1);
        captures.observe_admitted_claim_unchecked(later, &units([0]));
        assert!(!captures.effect_allowed(later));
        assert_eq!(
            captures
                .capture_snapshot(CodedCaptureId(1))
                .unwrap()
                .membership
                .get(&later),
            None,
            "disjoint active capture must not classify or gate the operation"
        );

        captures
            .observe_later_cut(
                CodedCaptureId(0),
                later,
                later_cut(),
                CodedLaterCutObservation::DurableAfterClean,
            )
            .unwrap();
        assert!(
            captures.effect_allowed(later),
            "only applicable capture obligations should gate the effect"
        );
    }

    #[test]
    fn unknown_clean_and_later_cut_remain_distinct() {
        let mut captures = CodedCaptureCoordinator::new();
        let capture = CodedCaptureId(0);
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [(token(0), units([0]))],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Unknown)
            .unwrap();
        captures.observe_admitted_claim_unchecked(token(1), &units([0]));
        assert!(!captures.effect_allowed(token(1)));
        captures
            .observe_later_cut(
                capture,
                token(1),
                later_cut(),
                CodedLaterCutObservation::Unknown,
            )
            .unwrap();
        captures
            .reconcile_later_cut_unchecked(
                capture,
                token(1),
                later_cut(),
                CodedLaterCutReconciliation::Rejected,
            )
            .unwrap();
        assert!(!captures.effect_allowed(token(1)));
        assert_eq!(
            captures.capture_snapshot(capture).unwrap().phase,
            CodedCapturePhase::CleanCommitUnknown
        );
    }

    #[test]
    fn unresolved_capture_never_prepares_membership_compaction() {
        let capture = CodedCaptureId(20);
        let operation = token(0);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [(operation, units([0]))],
            )
            .unwrap();
        assert!(
            captures
                .observe_release_allowed(&release(operation), Some(&fence_certificate()))
                .unwrap()
        );

        assert!(
            captures
                .prepare_membership_compaction_unchecked(capture, RecoveryGeneration::ZERO)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            captures
                .capture_snapshot(capture)
                .unwrap()
                .membership
                .get(&operation),
            Some(&CodedCaptureMembership::Included)
        );
    }

    #[test]
    fn clean_known_empty_capture_prepares_cleanup_without_a_later_cut() {
        let capture = CodedCaptureId(21);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
            .unwrap();

        assert!(
            captures
                .prepare_clean_known_cleanup_unchecked(capture, RecoveryGeneration::ZERO)
                .is_ok()
        );
        assert!(matches!(
            captures.prepare_refused_cleanup_unchecked(capture, RecoveryGeneration::ZERO),
            Err(CodedCaptureError::CleanNotEligible(found)) if found == capture
        ));
    }

    #[test]
    fn prepared_compaction_and_cleanup_reject_intervening_owner_state() {
        let capture = CodedCaptureId(23);
        let first = OperationSlotToken::new(0, 1);
        let second = OperationSlotToken::new(1, 1);
        let topology_epoch = dwv_core::TopologyEpoch(1);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [(first, units([0])), (second, units([0]))],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Rejected)
            .unwrap();
        assert!(
            captures
                .observe_release_allowed(&release(first), Some(&fence_certificate()))
                .unwrap()
        );

        let stale_compaction = captures
            .prepare_membership_compaction_unchecked(capture, RecoveryGeneration::ZERO)
            .unwrap()
            .unwrap();
        let stale_receipt = crate::DurableRecoveryCommit {
            expected_generation: RecoveryGeneration::ZERO,
            topology_epoch,
            committed_generation: RecoveryGeneration(1),
            mutations: vec![crate::RecoveryMutation::ApplyCodedTransition {
                transition: stale_compaction.transition(),
            }],
            persisted_fences: Vec::new(),
        };
        assert!(
            captures
                .observe_release_allowed(&release(second), Some(&fence_certificate()))
                .unwrap()
        );
        assert_eq!(
            captures.confirm_membership_compaction(stale_compaction, &stale_receipt),
            Err(CodedCaptureError::PreparedTransitionStale(capture))
        );

        let compaction = captures
            .prepare_membership_compaction_unchecked(capture, RecoveryGeneration::ZERO)
            .unwrap()
            .unwrap();
        let compaction_receipt = crate::DurableRecoveryCommit {
            expected_generation: RecoveryGeneration::ZERO,
            topology_epoch,
            committed_generation: RecoveryGeneration(1),
            mutations: vec![crate::RecoveryMutation::ApplyCodedTransition {
                transition: compaction.transition(),
            }],
            persisted_fences: Vec::new(),
        };
        captures
            .confirm_membership_compaction(compaction, &compaction_receipt)
            .unwrap();
        assert!(
            captures
                .capture_snapshot(capture)
                .unwrap()
                .membership
                .is_empty()
        );

        assert!(matches!(
            captures.prepare_clean_known_cleanup_unchecked(capture, RecoveryGeneration(1)),
            Err(CodedCaptureError::PreparedTransitionStale(found)) if found == capture
        ));
        let cleanup = captures
            .prepare_refused_cleanup_unchecked(capture, RecoveryGeneration(1))
            .unwrap();
        let concurrent_cleanup = captures
            .prepare_refused_cleanup_unchecked(capture, RecoveryGeneration(1))
            .unwrap();
        let cleanup_receipt = crate::DurableRecoveryCommit {
            expected_generation: RecoveryGeneration(1),
            topology_epoch,
            committed_generation: RecoveryGeneration(2),
            mutations: vec![crate::RecoveryMutation::ApplyCodedTransition {
                transition: cleanup.transition(),
            }],
            persisted_fences: Vec::new(),
        };
        captures
            .confirm_refused_cleanup(cleanup, &cleanup_receipt)
            .unwrap();
        assert_eq!(
            captures.confirm_refused_cleanup(concurrent_cleanup, &cleanup_receipt),
            Err(CodedCaptureError::PreparedTransitionStale(capture))
        );
    }
    #[test]
    fn reopen_recomputes_scope_instead_of_trusting_serialized_proof_flags() {
        let capture = CodedCaptureId(22);
        let topology = owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)]))
            .topology
            .clone();
        let scope = CodedCaptureOwnerFacts::selected_invalidation_scope(
            &topology,
            [RegionId(0)],
            [IntegrityExtentId(0)],
            DIRTY_REGION_BYTES,
            BLAKE3_256_PROFILE,
        )
        .unwrap();
        let (_, lifecycle_authority) = LifecycleAuthorityOwner::new();
        let admission_authority = CodedAdmissionAuthorityId::new();
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(capture, owner_facts(scope), [])
            .unwrap();
        let snapshot = captures.capture_snapshot(capture).unwrap();
        let reopened_exact = CodedCaptureCoordinator::from_snapshots_for_owner(
            [snapshot.clone()],
            &topology,
            RecoveryGeneration::ZERO,
            BLAKE3_256_PROFILE,
            ChecksumSetGeneration::INITIAL,
            DIRTY_REGION_BYTES,
            lifecycle_authority.clone(),
            admission_authority,
        )
        .unwrap();
        assert!(
            !reopened_exact.captures[&capture].retained_history_revalidated,
            "restart must not mint retained-history authority from snapshot equality"
        );

        let mut forged_frontier = snapshot.clone();
        forged_frontier.retained_frontier = CodedCaptureFrontier::new(RecoveryGeneration::ZERO, 1);
        let reopened = CodedCaptureCoordinator::from_snapshots_for_owner(
            [forged_frontier],
            &topology,
            RecoveryGeneration::ZERO,
            BLAKE3_256_PROFILE,
            ChecksumSetGeneration::INITIAL,
            DIRTY_REGION_BYTES,
            lifecycle_authority.clone(),
            admission_authority,
        )
        .unwrap();
        assert!(
            !reopened.captures[&capture].retained_history_revalidated,
            "an inherited compacted frontier must remain quarantined"
        );

        let mut forged = snapshot;
        forged.scope.insert(CodedUnitId(u64::MAX));
        assert!(forged.scope_complete && forged.scope_validated && forged.lower_frontier_covered);
        assert!(matches!(
            CodedCaptureCoordinator::from_snapshots_for_owner(
                [forged],
                &topology,
                RecoveryGeneration::ZERO,
                BLAKE3_256_PROFILE,
                ChecksumSetGeneration::INITIAL,
                DIRTY_REGION_BYTES,
                lifecycle_authority,
                admission_authority,
            ),
            Err(CodedCaptureError::SnapshotInvalid(found)) if found == capture
        ));
    }
    #[test]
    fn later_cut_binds_admission_target_and_requires_exact_transition_receipt() {
        let capture = CodedCaptureId(26);
        let included = token(0);
        let later = token(1);
        let target = InvalidationTarget::new(vec![RegionId(0)], vec![IntegrityExtentId(0)]);
        let mut owner = CodedCaptureCoordinator::new();
        let admission = CodedAdmission::for_test(
            owner.admission_authority,
            later,
            [CodedUnitId(0)],
            target.clone(),
            2,
        );
        owner
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [(included, units([0]))],
            )
            .unwrap();
        owner.observe_admitted_claim_unchecked(later, admission.units());

        let prepared = owner
            .prepare_later_cut(
                capture,
                &admission,
                TopologyEpoch(1),
                RecoveryGeneration::ZERO,
            )
            .unwrap();
        assert_eq!(prepared.invalidation_target(), &target);
        let mut substituted_snapshot = prepared.transition();
        match &mut substituted_snapshot.kind {
            CodedCaptureTransitionKind::LaterCut { update, .. } => {
                update
                    .proposed
                    .membership
                    .insert(later, CodedCaptureMembership::LaterRejected);
            }
            _ => unreachable!("prepared later cut emitted a non-later-cut transition"),
        }
        let substituted_receipt = crate::DurableRecoveryCommit {
            expected_generation: RecoveryGeneration::ZERO,
            topology_epoch: TopologyEpoch(1),
            committed_generation: RecoveryGeneration(1),
            mutations: vec![crate::RecoveryMutation::ApplyCodedTransition {
                transition: substituted_snapshot,
            }],
            persisted_fences: Vec::new(),
        };
        assert_eq!(
            owner.confirm_later_cut(prepared, &substituted_receipt),
            Err(CodedCaptureError::DurableReceiptMismatch(capture))
        );

        let prepared = owner
            .prepare_later_cut(
                capture,
                &admission,
                TopologyEpoch(1),
                RecoveryGeneration::ZERO,
            )
            .unwrap();
        let mut wrong_transition = prepared.transition();
        match &mut wrong_transition.kind {
            CodedCaptureTransitionKind::LaterCut { target, .. } => {
                target.regions.push(RegionId(99));
            }
            _ => unreachable!("prepared later cut emitted a non-later-cut transition"),
        }
        let wrong_transition_receipt = crate::DurableRecoveryCommit {
            expected_generation: RecoveryGeneration::ZERO,
            topology_epoch: TopologyEpoch(1),
            committed_generation: RecoveryGeneration(1),
            mutations: vec![crate::RecoveryMutation::ApplyCodedTransition {
                transition: wrong_transition,
            }],
            persisted_fences: Vec::new(),
        };
        assert_eq!(
            owner.confirm_later_cut(prepared, &wrong_transition_receipt),
            Err(CodedCaptureError::DurableReceiptMismatch(capture))
        );
    }

    #[test]
    fn owner_issued_lifecycle_evidence_is_accepted() {
        let capture = CodedCaptureId(23);
        let operation = token(0);
        let (lifecycle_owner, lifecycle_authority) = LifecycleAuthorityOwner::new();
        let mut owner = CodedCaptureCoordinator::new_with_authorities(
            lifecycle_authority,
            CodedAdmissionAuthorityId::new(),
        );
        owner
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [(operation, units([0]))],
            )
            .unwrap();
        let mut recovery = MemoryRecoveryStore::new(TopologyEpoch(1))
            .snapshot()
            .clone();
        recovery.coded_captures = owner.snapshots();
        let certificate = fence_certificate();
        let request = RecoveryCleanRequest::new(
            TopologyEpoch(1),
            dwv_store::FenceDomain(1),
            RecoveryGeneration::ZERO,
        )
        .with_region(RegionId(0))
        .with_checksum_extent(IntegrityExtentId(0));
        let permit = match evaluate_recovery_clean(&recovery, &request, &certificate).unwrap() {
            RecoveryCleanDecision::Clear(permit) => permit,
            RecoveryCleanDecision::Refused { .. } => panic!("complete CLEAN evidence was refused"),
        };
        let (foreign_owner, _) = LifecycleAuthorityOwner::new();
        assert!(matches!(
            owner.authorize_clean(
                &recovery,
                capture,
                permit.clone(),
                [(
                    live_lifecycle_authorization(&foreign_owner, operation),
                    certificate.clone(),
                )],
            ),
            Err(CodedCaptureError::CleanNotEligible(found)) if found == capture
        ));
        let evidence = (
            live_lifecycle_authorization(&lifecycle_owner, operation),
            certificate.clone(),
        );
        let authorization = owner
            .authorize_clean(&recovery, capture, permit, [evidence])
            .unwrap();
        let prepared = owner.prepare_clean_commit(authorization).unwrap();
        let transition = prepared.transition();
        assert!(matches!(
            &transition.kind,
            CodedCaptureTransitionKind::Clean { fence, .. } if fence == &certificate
        ));

        let mut substituted = transition;
        match &mut substituted.kind {
            CodedCaptureTransitionKind::Clean { fence, .. } => {
                fence.stores[0].capability_evidence_id = dwv_store::CapabilityEvidenceId(u64::MAX);
            }
            _ => unreachable!("prepared CLEAN emitted a non-CLEAN transition"),
        }
        let substituted_receipt = crate::DurableRecoveryCommit {
            expected_generation: RecoveryGeneration::ZERO,
            topology_epoch: TopologyEpoch(1),
            committed_generation: RecoveryGeneration(1),
            mutations: vec![crate::RecoveryMutation::ApplyCodedTransition {
                transition: substituted,
            }],
            persisted_fences: Vec::new(),
        };
        assert_eq!(
            owner.confirm_clean_commit(prepared, &substituted_receipt),
            Err(CodedCaptureError::DurableReceiptMismatch(capture))
        );
    }

    #[test]
    fn refreshed_clean_permit_cannot_weaken_included_persistence_bounds() {
        let capture = CodedCaptureId(24);
        let operation = token(0);
        let (lifecycle_owner, lifecycle_authority) = LifecycleAuthorityOwner::new();
        let mut owner = CodedCaptureCoordinator::new_with_authorities(
            lifecycle_authority,
            CodedAdmissionAuthorityId::new(),
        );
        owner
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [(operation, units([0]))],
            )
            .unwrap();
        let mut recovery = MemoryRecoveryStore::new(TopologyEpoch(1))
            .snapshot()
            .clone();
        recovery.coded_captures = owner.snapshots();
        let strong_certificate = fence_certificate();
        let request = RecoveryCleanRequest::new(
            TopologyEpoch(1),
            dwv_store::FenceDomain(1),
            RecoveryGeneration::ZERO,
        )
        .with_region(RegionId(0))
        .with_checksum_extent(IntegrityExtentId(0));
        let strong_permit =
            match evaluate_recovery_clean(&recovery, &request, &strong_certificate).unwrap() {
                RecoveryCleanDecision::Clear(permit) => permit,
                RecoveryCleanDecision::Refused { .. } => {
                    panic!("complete CLEAN evidence was refused")
                }
            };
        let authorization = owner
            .authorize_clean(
                &recovery,
                capture,
                strong_permit,
                [(
                    live_lifecycle_authorization(&lifecycle_owner, operation),
                    strong_certificate,
                )],
            )
            .unwrap();
        let mut weaker_certificate = fence_certificate();
        weaker_certificate.stores[0].through = dwv_store::StoreWriteWatermark(0);
        let weaker_permit = match evaluate_recovery_clean(&recovery, &request, &weaker_certificate)
            .unwrap()
        {
            RecoveryCleanDecision::Clear(permit) => permit,
            RecoveryCleanDecision::Refused { .. } => {
                panic!("scope-only CLEAN request unexpectedly required the stronger store bound")
            }
        };

        assert!(matches!(
            owner.refresh_clean_authorization(&recovery, authorization, weaker_permit),
            Err(CodedCaptureError::CleanNotEligible(found)) if found == capture
        ));
    }

    #[test]
    fn recovery_store_rejects_clean_cleanup_after_same_commit_dirties_scope() {
        let capture = CodedCaptureId(24);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Durable)
            .unwrap();

        let fence = fence_certificate().with_occurrence(FenceOccurrenceId::FIRST);
        captures.record_mut(capture).unwrap().clean_closure_fence = Some(fence.clone());
        let mut store = MemoryRecoveryStore::new(TopologyEpoch(1));
        store.snapshot.coded_captures = captures.snapshots();
        store.snapshot.dirty_regions = vec![DirtyRegionRecord {
            region: RegionId(0),
            state: RegionState::Clean,
            last_clean_fence: Some(fence.clone()),
            clean_generation: Some(RecoveryGeneration::ZERO),
        }];
        store.snapshot.fences = vec![fence];
        let authorization = match captures
            .resolve_capture(&store.snapshot, capture, None)
            .unwrap()
        {
            CodedReopenResolution::CleanKnown(authorization) => authorization,
            _ => panic!("live owner history did not authorize cleanup"),
        };
        let cleanup = captures.prepare_clean_known_cleanup(authorization).unwrap();
        let transition = cleanup.transition();
        for raw_first in [true, false] {
            let mut candidate = store.clone();
            let raw = RecoveryMutation::MarkRegionDirty {
                region: RegionId(0),
                mutation_generation: RecoveryGeneration::ZERO,
            };
            let coded = RecoveryMutation::ApplyCodedTransition {
                transition: transition.clone(),
            };
            let mut txn = RecoveryTxn::new(RecoveryGeneration::ZERO, TopologyEpoch(1));
            if raw_first {
                txn.push(raw);
                txn.push(coded);
            } else {
                txn.push(coded);
                txn.push(raw);
            }
            assert_eq!(
                candidate.commit_durable(txn),
                Err(RecoveryError::InvalidTransition(
                    TransitionError::CodedSemanticTransactionConflict
                ))
            );
            assert_eq!(candidate.snapshot, store.snapshot);
        }
    }

    #[test]
    fn every_coded_transition_kind_rejects_ordinary_co_mutations_in_both_orders() {
        let capture = CodedCaptureId(30);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [],
            )
            .unwrap();
        let snapshot = captures.capture_snapshot(capture).unwrap();
        let update = || CodedCaptureUpdate {
            expected: Some(snapshot.clone()),
            proposed: snapshot.clone(),
        };
        let removal = || CodedCaptureRemoval {
            expected: snapshot.clone(),
            clean_closure: None,
        };
        let transitions = [
            CodedCaptureTransition::updates(vec![update()]),
            CodedCaptureTransition {
                kind: CodedCaptureTransitionKind::Clean {
                    update: update(),
                    fence: fence_certificate(),
                    dirty_regions: snapshot.dirty_regions.clone(),
                    checksum_extents: snapshot.checksum_extents.clone(),
                },
            },
            CodedCaptureTransition {
                kind: CodedCaptureTransitionKind::LaterCut {
                    update: update(),
                    target: InvalidationTarget::new(
                        snapshot.dirty_regions.iter().copied().collect(),
                        snapshot.checksum_extents.iter().copied().collect(),
                    ),
                    invalidation_generation: RecoveryGeneration::ZERO,
                },
            },
            CodedCaptureTransition {
                kind: CodedCaptureTransitionKind::InvalidatingRemoval {
                    removal: removal(),
                    dirty_regions: snapshot.dirty_regions.clone(),
                    checksum_extents: snapshot.checksum_extents.clone(),
                    invalidation_generation: RecoveryGeneration::ZERO,
                    conservative_state: None,
                },
            },
            CodedCaptureTransition {
                kind: CodedCaptureTransitionKind::CleanRemoval(removal()),
            },
        ];
        for transition in transitions {
            for raw_first in [true, false] {
                let raw = RecoveryMutation::MarkRegionDirty {
                    region: RegionId(99),
                    mutation_generation: RecoveryGeneration::ZERO,
                };
                let coded = RecoveryMutation::ApplyCodedTransition {
                    transition: transition.clone(),
                };
                let mut txn = RecoveryTxn::new(RecoveryGeneration::ZERO, TopologyEpoch(1));
                if raw_first {
                    txn.push(raw);
                    txn.push(coded);
                } else {
                    txn.push(coded);
                    txn.push(raw);
                }
                assert_eq!(
                    txn.validate_coded_composition(),
                    Err(RecoveryError::InvalidTransition(
                        TransitionError::CodedSemanticTransactionConflict
                    ))
                );
            }
        }
    }

    #[test]
    fn coded_group_composition_rejects_conflicts_and_accepts_compatible_captures() {
        let mut captures = CodedCaptureCoordinator::new();
        for capture in [CodedCaptureId(31), CodedCaptureId(32)] {
            captures
                .start_capture_unchecked(
                    capture,
                    owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                    [],
                )
                .unwrap();
        }
        let first = captures.capture_snapshot(CodedCaptureId(31)).unwrap();
        let second = captures.capture_snapshot(CodedCaptureId(32)).unwrap();
        let update = |snapshot: &CodedCaptureSnapshot| CodedCaptureUpdate {
            expected: Some(snapshot.clone()),
            proposed: snapshot.clone(),
        };
        let first_update = CodedCaptureTransition::updates(vec![update(&first)]);
        let second_update = CodedCaptureTransition::updates(vec![update(&second)]);

        let mut same_capture = RecoveryTxn::new(RecoveryGeneration::ZERO, TopologyEpoch(1));
        for transition in [first_update.clone(), first_update.clone()] {
            same_capture.push(RecoveryMutation::ApplyCodedTransition { transition });
        }
        assert_eq!(
            same_capture.validate_coded_composition(),
            Err(RecoveryError::InvalidTransition(
                TransitionError::CodedSemanticTransactionConflict
            ))
        );

        let clean = CodedCaptureTransition {
            kind: CodedCaptureTransitionKind::Clean {
                update: update(&first),
                fence: fence_certificate(),
                dirty_regions: first.dirty_regions.clone(),
                checksum_extents: first.checksum_extents.clone(),
            },
        };
        let dirty = CodedCaptureTransition {
            kind: CodedCaptureTransitionKind::LaterCut {
                update: update(&second),
                target: InvalidationTarget::new(
                    second.dirty_regions.iter().copied().collect(),
                    second.checksum_extents.iter().copied().collect(),
                ),
                invalidation_generation: RecoveryGeneration::ZERO,
            },
        };
        let mut incompatible = RecoveryTxn::new(RecoveryGeneration::ZERO, TopologyEpoch(1));
        for transition in [clean, dirty] {
            incompatible.push(RecoveryMutation::ApplyCodedTransition { transition });
        }
        assert_eq!(
            incompatible.validate_coded_composition(),
            Err(RecoveryError::InvalidTransition(
                TransitionError::CodedSemanticTransactionConflict
            ))
        );

        let mut store = MemoryRecoveryStore::new(TopologyEpoch(1));
        store.snapshot.coded_captures = vec![first.clone(), second.clone()];
        let mut compatible = RecoveryTxn::new(RecoveryGeneration::ZERO, TopologyEpoch(1));
        for transition in [first_update, second_update] {
            compatible.push(RecoveryMutation::ApplyCodedTransition { transition });
        }
        assert_eq!(store.commit_durable(compatible), Ok(RecoveryGeneration(1)));
        assert_eq!(store.snapshot.coded_captures, vec![first, second]);
    }
    #[test]
    fn conservative_state_proof_allows_omitted_effects_but_rejects_stale_replay() {
        let capture = CodedCaptureId(25);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [],
            )
            .unwrap();
        captures
            .observe_decision(capture, CodedCaptureDecision::Accepted)
            .unwrap();
        captures.request_clean(capture).unwrap();
        captures
            .observe_clean_commit(capture, CodedCleanCommitObservation::Rejected)
            .unwrap();

        let mut store = MemoryRecoveryStore::new(TopologyEpoch(1));
        store.snapshot.coded_captures = captures.snapshots();
        store.snapshot.dirty_regions = vec![DirtyRegionRecord {
            region: RegionId(0),
            state: RegionState::Dirty {
                dirty_since: RecoveryGeneration::ZERO,
            },
            last_clean_fence: None,
            clean_generation: None,
        }];
        store.snapshot.integrity_records = vec![IntegrityRecord {
            extent: IntegrityExtentId(0),
            state: IntegrityState::Stale {
                stale_generation: RecoveryGeneration::ZERO,
            },
        }];

        let authorization = match captures
            .resolve_capture(&store.snapshot, capture, None)
            .unwrap()
        {
            CodedReopenResolution::Refused(authorization) => authorization,
            _ => panic!("refused capture did not produce cleanup authority"),
        };
        let cleanup = captures.prepare_refused_cleanup(authorization).unwrap();
        let stale_authorization = match captures
            .resolve_capture(&store.snapshot, capture, None)
            .unwrap()
        {
            CodedReopenResolution::Refused(authorization) => authorization,
            _ => panic!("refused capture did not reproduce cleanup authority"),
        };
        let stale_cleanup = captures
            .prepare_refused_cleanup(stale_authorization)
            .unwrap();

        let transition = cleanup.transition();
        assert!(!transition.contains_region_dirty(RegionId(0), RecoveryGeneration::ZERO));
        assert!(
            !transition.contains_integrity_stale(IntegrityExtentId(0), RecoveryGeneration::ZERO)
        );
        let mut txn = RecoveryTxn::new(RecoveryGeneration::ZERO, TopologyEpoch(1));
        txn.push(RecoveryMutation::ApplyCodedTransition { transition });
        let receipt = store.commit_durable_receipt(txn).unwrap();
        captures.confirm_refused_cleanup(cleanup, &receipt).unwrap();
        assert!(store.snapshot.coded_captures.is_empty());
        assert!(matches!(
            store.snapshot.dirty_regions[0].state,
            RegionState::Dirty { .. }
        ));
        assert!(matches!(
            store.snapshot.integrity_records[0].state,
            IntegrityState::Stale { .. }
        ));

        let mut replay_store = MemoryRecoveryStore::new(TopologyEpoch(1));
        replay_store.snapshot.coded_captures = vec![stale_cleanup.0.prior.clone()];
        replay_store.snapshot.dirty_regions = vec![DirtyRegionRecord {
            region: RegionId(0),
            state: RegionState::Clean,
            last_clean_fence: None,
            clean_generation: None,
        }];
        replay_store.snapshot.integrity_records = vec![IntegrityRecord {
            extent: IntegrityExtentId(0),
            state: IntegrityState::Stale {
                stale_generation: RecoveryGeneration::ZERO,
            },
        }];
        let prior = replay_store.snapshot.clone();
        let mut replay = RecoveryTxn::new(RecoveryGeneration::ZERO, TopologyEpoch(1));
        replay.push(RecoveryMutation::ApplyCodedTransition {
            transition: stale_cleanup.transition(),
        });
        assert_eq!(
            replay_store.commit_durable(replay),
            Err(RecoveryError::InvalidTransition(
                TransitionError::CodedSemanticTransitionIncomplete
            ))
        );
        assert_eq!(replay_store.snapshot, prior);
    }

    #[test]
    fn recovery_store_rejects_malformed_coded_semantic_transition_groups() {
        let capture = CodedCaptureId(26);
        let mut captures = CodedCaptureCoordinator::new();
        captures
            .start_capture_unchecked(
                capture,
                owner_facts(ValidatedCodedCaptureScope::complete([CodedUnitId(0)])),
                [],
            )
            .unwrap();
        let snapshot = captures.capture_snapshot(capture).unwrap();
        let mut store = MemoryRecoveryStore::new(TopologyEpoch(1));
        store.snapshot.coded_captures = vec![snapshot.clone()];
        let durable = store.snapshot.clone();

        let malformed = [
            CodedCaptureTransition::updates(Vec::new()),
            CodedCaptureTransition::updates(vec![
                CodedCaptureUpdate {
                    expected: Some(snapshot.clone()),
                    proposed: snapshot.clone(),
                },
                CodedCaptureUpdate {
                    expected: Some(snapshot.clone()),
                    proposed: snapshot.clone(),
                },
            ]),
            CodedCaptureTransition {
                kind: CodedCaptureTransitionKind::LaterCut {
                    update: CodedCaptureUpdate {
                        expected: Some(snapshot.clone()),
                        proposed: snapshot.clone(),
                    },
                    target: InvalidationTarget::new(Vec::new(), Vec::new()),
                    invalidation_generation: RecoveryGeneration::ZERO,
                },
            },
            CodedCaptureTransition {
                kind: CodedCaptureTransitionKind::InvalidatingRemoval {
                    removal: CodedCaptureRemoval {
                        expected: snapshot.clone(),
                        clean_closure: None,
                    },
                    dirty_regions: BTreeSet::new(),
                    checksum_extents: [IntegrityExtentId(0)].into_iter().collect(),
                    invalidation_generation: RecoveryGeneration::ZERO,
                    conservative_state: None,
                },
            },
            CodedCaptureTransition {
                kind: CodedCaptureTransitionKind::InvalidatingRemoval {
                    removal: CodedCaptureRemoval {
                        expected: snapshot.clone(),
                        clean_closure: None,
                    },
                    dirty_regions: [RegionId(0)].into_iter().collect(),
                    checksum_extents: BTreeSet::new(),
                    invalidation_generation: RecoveryGeneration::ZERO,
                    conservative_state: None,
                },
            },
            CodedCaptureTransition {
                kind: CodedCaptureTransitionKind::InvalidatingRemoval {
                    removal: CodedCaptureRemoval {
                        expected: snapshot.clone(),
                        clean_closure: None,
                    },
                    dirty_regions: [RegionId(0), RegionId(999)].into_iter().collect(),
                    checksum_extents: [IntegrityExtentId(0)].into_iter().collect(),
                    invalidation_generation: RecoveryGeneration::ZERO,
                    conservative_state: None,
                },
            },
        ];
        for transition in malformed {
            let mut candidate = store.clone();
            let mut txn = RecoveryTxn::new(RecoveryGeneration::ZERO, TopologyEpoch(1));
            txn.push(RecoveryMutation::ApplyCodedTransition { transition });
            assert_eq!(
                candidate.commit_durable(txn),
                Err(RecoveryError::InvalidTransition(
                    TransitionError::CodedSemanticTransitionIncomplete
                ))
            );
            assert_eq!(candidate.snapshot, durable);
        }

        let mut wrong_predecessor = snapshot.clone();
        wrong_predecessor.recovery_generation = RecoveryGeneration(99);
        let transition = CodedCaptureTransition::updates(vec![CodedCaptureUpdate {
            expected: Some(wrong_predecessor),
            proposed: snapshot,
        }]);
        let mut txn = RecoveryTxn::new(RecoveryGeneration::ZERO, TopologyEpoch(1));
        txn.push(RecoveryMutation::ApplyCodedTransition { transition });
        assert_eq!(
            store.commit_durable(txn),
            Err(RecoveryError::InvalidTransition(
                TransitionError::CodedCaptureIdentityMismatch
            ))
        );
        assert_eq!(store.snapshot, durable);
    }
}
