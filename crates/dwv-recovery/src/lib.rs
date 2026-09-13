//! Portable recovery-state semantics.
//!
//! This crate is the authority boundary for write-recovery records, integrity
//! evidence, topology epochs, and recovery CLEAN decisions. The reference implementation
//! is in memory so its transaction rules can be exercised without selecting a
//! SQLite layout or durability configuration.

use dwv_core::{FenceDomain, TopologyEpoch};
#[doc = r#"
An exact-generation Included-operation lifecycle capability.

Ordinary recovery consumers cannot synthesize one:

```compile_fail
use dwv_recovery::IncludedLifecycleAuthorization;
use dwv_store::OperationSlotToken;

fn mint(operation: OperationSlotToken) -> IncludedLifecycleAuthorization {
    IncludedLifecycleAuthorization {
        operation,
        released: false,
    }
}
```

```compile_fail
use dwv_lifecycle_authority::LifecycleAuthorityOwner;
use dwv_recovery::CodedRangeAuthority;

let (_, verifier) = LifecycleAuthorityOwner::new();
let _ = CodedRangeAuthority::new_with_lifecycle_authority(verifier);
```

```compile_fail
use dwv_lifecycle_authority::LifecycleAuthorityOwner;
use dwv_recovery::CodedCaptureCoordinator;

let (_, verifier) = LifecycleAuthorityOwner::new();
let _ = CodedCaptureCoordinator::new_with_lifecycle_authority(verifier);
```

Capture cuts cannot be issued from a stale cloned range authority:

```compile_fail
use dwv_recovery::CodedRangeAuthority;

fn stale_cut(range: &mut CodedRangeAuthority) {
    let _ = range.capture_boundary();
}
```

Raw range authority cannot issue media-effect permission without the paired
capture owner:

```compile_fail
use dwv_recovery::CodedRangeAuthority;
use dwv_store::OperationSlotToken;

fn bypass_capture(range: &mut CodedRangeAuthority, operation: OperationSlotToken) {
    let _ = range.permit_effect(operation);
}
```

Capture establishment cannot bypass the lifecycle owner with a caller-built
geometry scope:

```compile_fail
use dwv_recovery::CodedCaptureCoordinator;

fn substitute_scope(coordinator: &CodedCaptureCoordinator) {
    let _ = CodedCaptureCoordinator::prepare_capture_start;
}
```
"#]
pub use dwv_lifecycle_authority::IncludedLifecycleAuthorization;
use dwv_store::{StoreFenceRef, StoreId};
use std::{collections::BTreeSet, fmt};

mod baseline;
mod coded_authority;
mod coded_clean;
mod coded_geometry;
mod coded_lifecycle;
mod extent;
mod generation;
mod inspection;
mod invalidation;
mod job;
mod metadata_loss;
mod migration;
mod profile;
mod provider;
mod rebuild;
mod record;
mod recovery_clean;
mod transition;
mod write_recovery_record;

pub use baseline::{
    ChecksumBaseline, ChecksumBaselineInvalidReason, ChecksumBaselineProvenance,
    ChecksumBaselineStatus, assess_checksum_baseline, expected_checksum_extents,
    new_checksum_baseline, pending_checksum_baseline_extents,
};
pub use coded_authority::{
    CodedAdmission, CodedAdmissionOutcome, CodedAuthorityError, CodedAuthorityFrontier, CodedClaim,
    CodedClaimInput, CodedClaimRelease, CodedEffectPermit, CodedOperationPhase,
    CodedRangeAuthority,
};
pub use coded_clean::{
    CodedCaptureCleanAuthorization, CodedCaptureCoordinator, CodedCaptureCut, CodedCaptureDecision,
    CodedCaptureError, CodedCaptureFrontier, CodedCaptureId, CodedCaptureMembership,
    CodedCaptureOwnerFacts, CodedCapturePhase, CodedCaptureReconciliationReceipt,
    CodedCaptureRetentionOwner, CodedCaptureSnapshot, CodedCaptureTransition, CodedCleanAttempt,
    CodedCleanAttemptPhase, CodedCleanCommitObservation, CodedCleanKnownCleanupAuthorization,
    CodedCleanReconciliation, CodedInheritedCaptureAbandonmentAuthorization, CodedLaterCutAttempt,
    CodedLaterCutAttemptPhase, CodedLaterCutObservation, CodedLaterCutReconciliation,
    CodedMembershipCompactionAuthorization, CodedRefusedCleanupAuthorization,
    CodedReopenResolution, PreparedCodedCaptureUpdates, PreparedCodedCleanCommit,
    PreparedCodedCleanKnownCleanup, PreparedCodedInheritedCaptureAbandonment,
    PreparedCodedLaterCut, PreparedCodedMembershipCompaction, PreparedCodedRefusal,
    PreparedCodedRefusedCleanup, ValidatedCodedCaptureScope,
};
pub use coded_geometry::CodedGeometryOwner;
pub use coded_lifecycle::{
    CodedIncludedAuthority, CodedLifecycleAuthority, CodedLifecycleError, CodedOwnerCleanAttempt,
    CodedOwnerLaterCutAttempt, CodedReleaseAuthority, PreparedCodedAdmission,
    PreparedCodedOwnerTransition, PreparedCodedRelease,
};
pub use extent::{ChecksumExtent, ChecksumTarget, ExtentError};
pub use generation::{GenerationCapture, RecoveryGeneration};
pub use inspection::{
    RecoveryFormatLayer, RecoveryInspection, RecoveryReconciliation, reconcile_coded_clean_attempt,
    reconcile_coded_later_cut_attempt, reconcile_uncertain_commit,
};
pub use invalidation::{
    InvalidationTarget, WriteRecoveryRecordBoundary, WriteRecoveryRecordCoverage,
    WriteRecoveryRecordDecision, WriteRecoveryRecordEvidence, assess_write_recovery_record,
};
pub use job::{
    ChecksumAuthority, ChecksumJob, ChecksumJobKey, ChecksumJobResult, ChecksumQueue,
    CommitOutcome, JobError, ReadEvidence,
};
pub use metadata_loss::{
    BaselineDisposition, EvidenceRequirement, METADATA_LOSS_MATRIX_VERSION, MetadataLossAction,
    MetadataLossAudit, MetadataLossAuthorization, MetadataLossCase, MetadataLossDisposition,
    MetadataLossError, MetadataLossPlan, MetadataLossVerification, PayloadWritePolicy,
    create_fresh_manifest, render_metadata_loss_matrix,
};
pub use migration::{MigrationError, MigrationOutcome, ProfileMigration};
pub use profile::{
    BLAKE3_256_PROFILE, ChecksumProfile, ChecksumProfileId, ChecksumSet, ChecksumSetGeneration,
    ChecksumSetState, ProfileError,
};
pub use provider::{Blake3Provider, DigestProvider, ProviderError};
pub use rebuild::{
    REBUILD_ID_BYTES, REBUILD_TARGET_IDENTITY_BYTES, RebuildChunkReceipt, RebuildCompletionReceipt,
    RebuildCursor, RebuildError, RebuildId, RebuildLifecycle, RebuildState, RebuildTargetIdentity,
};
pub use record::{
    ChecksumPersistenceEvidence, ChecksumRecord, ChecksumState, ContentGeneration, Digest,
};
pub use recovery_clean::{
    RecoveryCleanDecision, RecoveryCleanPermit, RecoveryCleanRefusal, RecoveryCleanRefusalPermit,
    RecoveryCleanRequest, RequiredFence, evaluate_recovery_clean, fence_ref,
};
pub use transition::{
    RecoveryTransitionId, TransitionEvidence, TransitionKind, TransitionOutcome, TransitionTrace,
};
pub use write_recovery_record::{WriteRecoveryRecordCommit, WriteRecoveryRecordCommitResult};

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct SessionId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RegionId(pub u64);

pub const DIRTY_REGION_BYTES: u64 = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionMappingError {
    EmptyRange,
    InvalidRegionBytes,
    RegionIdOverflow,
}

impl fmt::Display for RegionMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRange => {
                write!(formatter, "dirty-region mapping requires a non-empty range")
            }
            Self::InvalidRegionBytes => write!(formatter, "dirty-region size must be non-zero"),
            Self::RegionIdOverflow => {
                write!(formatter, "dirty-region identity is not representable")
            }
        }
    }
}

impl std::error::Error for RegionMappingError {}

pub fn dirty_regions_for_range(
    member_index: u32,
    range: dwv_core::ByteRange,
    region_bytes: u64,
) -> Result<Vec<RegionId>, RegionMappingError> {
    if range.is_empty() {
        return Err(RegionMappingError::EmptyRange);
    }
    if region_bytes == 0 {
        return Err(RegionMappingError::InvalidRegionBytes);
    }

    let first = range.offset / region_bytes;
    let last = range
        .end()
        .checked_sub(1)
        .ok_or(RegionMappingError::RegionIdOverflow)?
        / region_bytes;
    let first = u32::try_from(first).map_err(|_| RegionMappingError::RegionIdOverflow)?;
    let last = u32::try_from(last).map_err(|_| RegionMappingError::RegionIdOverflow)?;
    Ok((first..=last)
        .map(|region| RegionId((u64::from(member_index) << 32) | u64::from(region)))
        .collect())
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct IntegrityExtentId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct JobId(pub u64);

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RecoveryCursor(pub u64);

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct FenceOccurrenceId(pub u64);

impl FenceOccurrenceId {
    pub const FIRST: Self = Self(1);
    pub const UNASSIGNED: Self = Self(0);
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RecoverySchemaVersion(pub u16);

pub const CURRENT_RECOVERY_SCHEMA: RecoverySchemaVersion = RecoverySchemaVersion(7);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryRecordKind {
    SnapshotHeader,
    Topology,
    DirtyRegion,
    IntegrityEvidence,
    ChecksumBaseline,
    StoreFence,
    WritableSession,
    MaintenanceCheckpoint,
    MigrationState,
    MetadataLossAudit,
    OfflineRebuild,
    CodedCleanCapture,
    ClaimRoot,
    LegacyUnreconciled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverySchemaDescriptor {
    pub version: RecoverySchemaVersion,
    pub records: Vec<RecoveryRecordKind>,
}

pub fn current_recovery_schema() -> RecoverySchemaDescriptor {
    RecoverySchemaDescriptor {
        version: CURRENT_RECOVERY_SCHEMA,
        records: vec![
            RecoveryRecordKind::SnapshotHeader,
            RecoveryRecordKind::Topology,
            RecoveryRecordKind::DirtyRegion,
            RecoveryRecordKind::IntegrityEvidence,
            RecoveryRecordKind::StoreFence,
            RecoveryRecordKind::WritableSession,
            RecoveryRecordKind::MaintenanceCheckpoint,
            RecoveryRecordKind::MigrationState,
            RecoveryRecordKind::MetadataLossAudit,
            RecoveryRecordKind::OfflineRebuild,
            RecoveryRecordKind::ChecksumBaseline,
            RecoveryRecordKind::CodedCleanCapture,
            RecoveryRecordKind::ClaimRoot,
            RecoveryRecordKind::LegacyUnreconciled,
        ],
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryMigrationStep {
    InitializeSemanticSchemaV1,
    AddMetadataLossAuditV2,
    AddOfflineRebuildV3,
    AddChecksumBaselineV4,
    AddCodedCleanCaptureV5,
    AddCodedCaptureReleaseEvidenceV6,
    AddFenceOccurrenceIdentityV7,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryMigrationPlan {
    pub from: RecoverySchemaVersion,
    pub to: RecoverySchemaVersion,
    pub steps: Vec<RecoveryMigrationStep>,
}

impl RecoveryMigrationPlan {
    pub fn plan(
        from: RecoverySchemaVersion,
        to: RecoverySchemaVersion,
    ) -> Result<Self, RecoveryError> {
        if from == to && from.0 <= CURRENT_RECOVERY_SCHEMA.0 {
            return Ok(Self {
                from,
                to,
                steps: Vec::new(),
            });
        }
        if from.0 >= to.0 || to.0 > CURRENT_RECOVERY_SCHEMA.0 {
            return Err(RecoveryError::UnsupportedSchemaMigration { from, to });
        }
        let mut steps = Vec::new();
        for version in (from.0 + 1)..=to.0 {
            steps.push(match version {
                1 => RecoveryMigrationStep::InitializeSemanticSchemaV1,
                2 => RecoveryMigrationStep::AddMetadataLossAuditV2,
                3 => RecoveryMigrationStep::AddOfflineRebuildV3,
                4 => RecoveryMigrationStep::AddChecksumBaselineV4,
                5 => RecoveryMigrationStep::AddCodedCleanCaptureV5,
                6 => RecoveryMigrationStep::AddCodedCaptureReleaseEvidenceV6,
                7 => RecoveryMigrationStep::AddFenceOccurrenceIdentityV7,
                _ => return Err(RecoveryError::UnsupportedSchemaMigration { from, to }),
            });
        }
        Ok(Self { from, to, steps })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RecoveryStoreHealth {
    Healthy,
    Missing,
    Corrupt,
    Stale,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryWritePolicy {
    Allowed,
    Blocked(RecoveryStoreHealth),
}

impl RecoveryStoreHealth {
    pub fn write_policy(self) -> RecoveryWritePolicy {
        match self {
            Self::Healthy => RecoveryWritePolicy::Allowed,
            health => RecoveryWritePolicy::Blocked(health),
        }
    }

    pub fn disposition(self) -> RecoveryDisposition {
        match self {
            Self::Healthy => RecoveryDisposition::Proceed,
            Self::Missing | Self::Corrupt => RecoveryDisposition::RebuildFromData,
            Self::Stale => RecoveryDisposition::ReconcileReadOnly,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryDisposition {
    Proceed,
    ReconcileReadOnly,
    RebuildFromData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryCommitObservation {
    Durable,
    Rejected,
    Lost,
    Corrupt,
}

impl RecoveryCommitObservation {
    pub fn is_durable(self) -> bool {
        matches!(self, Self::Durable)
    }

    pub fn disposition(self) -> RecoveryDisposition {
        match self {
            Self::Durable => RecoveryDisposition::Proceed,
            Self::Rejected | Self::Lost => RecoveryDisposition::ReconcileReadOnly,
            Self::Corrupt => RecoveryDisposition::RebuildFromData,
        }
    }
}

/// Owner-issued proof that one exact recovery transaction committed durably.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DurableRecoveryCommit {
    expected_generation: RecoveryGeneration,
    topology_epoch: TopologyEpoch,
    committed_generation: RecoveryGeneration,
    mutations: Vec<RecoveryMutation>,
    persisted_fences: Vec<FenceCertificate>,
}

impl DurableRecoveryCommit {
    pub const fn generation(&self) -> RecoveryGeneration {
        self.committed_generation
    }

    pub const fn expected_generation(&self) -> RecoveryGeneration {
        self.expected_generation
    }

    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch
    }
    /// Returns fence certificates assigned and published by this commit.
    pub fn persisted_fences(&self) -> &[FenceCertificate] {
        &self.persisted_fences
    }

    pub(crate) fn committed_coded_capture(&self, snapshot: &CodedCaptureSnapshot) -> bool {
        self.mutations.iter().any(|mutation| {
            matches!(
                mutation,
                RecoveryMutation::ApplyCodedTransition { transition }
                    if transition.contains_capture(snapshot)
            )
        })
    }

    pub(crate) fn committed_coded_transition(&self, expected: &CodedCaptureTransition) -> bool {
        self.mutations.iter().any(|mutation| {
            matches!(
                mutation,
                RecoveryMutation::ApplyCodedTransition { transition }
                    if transition == expected
            )
        })
    }

    pub(crate) fn committed_coded_capture_removal(
        &self,
        expected: &coded_clean::CodedCaptureRemoval,
    ) -> bool {
        self.mutations.iter().any(|mutation| {
            matches!(
                mutation,
                RecoveryMutation::ApplyCodedTransition { transition }
                    if transition.contains_removal(expected)
            )
        })
    }

    pub(crate) fn committed_region_dirty(
        &self,
        region: RegionId,
        mutation_generation: RecoveryGeneration,
    ) -> bool {
        self.mutations.iter().any(|mutation| match mutation {
            RecoveryMutation::MarkRegionDirty {
                region: observed_region,
                mutation_generation: observed_generation,
            } => *observed_region == region && *observed_generation == mutation_generation,
            RecoveryMutation::ApplyCodedTransition { transition } => {
                transition.contains_region_dirty(region, mutation_generation)
            }
            _ => false,
        })
    }

    pub(crate) fn committed_integrity_stale(
        &self,
        extent: IntegrityExtentId,
        stale_generation: RecoveryGeneration,
    ) -> bool {
        self.mutations.iter().any(|mutation| match mutation {
            RecoveryMutation::MarkIntegrityStale {
                extent: observed_extent,
                stale_generation: observed_generation,
            } => *observed_extent == extent && *observed_generation == stale_generation,
            RecoveryMutation::ApplyCodedTransition { transition } => {
                transition.contains_integrity_stale(extent, stale_generation)
            }
            _ => false,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RegionState {
    Clean,
    Dirty { dirty_since: RecoveryGeneration },
    Indeterminate,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DirtyRegionRecord {
    pub region: RegionId,
    pub state: RegionState,
    pub last_clean_fence: Option<FenceCertificate>,
    /// Exact generation owned by the clean fence, when present.
    pub clean_generation: Option<RecoveryGeneration>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ChecksumEvidenceBinding {
    pub extent: ChecksumExtent,
    pub profile: ChecksumProfileId,
    pub set_generation: ChecksumSetGeneration,
    pub topology_epoch: TopologyEpoch,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum IntegrityState {
    Absent,
    Stale {
        stale_generation: RecoveryGeneration,
    },
    Valid {
        binding: ChecksumEvidenceBinding,
        content_generation: RecoveryGeneration,
        durable_fence: StoreFenceRef,
        /// Exact composite fence occurrence used by this integrity claim.
        fence_occurrence: FenceOccurrenceId,
        digest: Vec<u8>,
        verified_at: RecoveryGeneration,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct IntegrityRecord {
    pub extent: IntegrityExtentId,
    pub state: IntegrityState,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// dwv:req req.recovery-state-semantics.fence-occurrences-have-stable-exact-identities-and-coverage
pub struct FenceCertificate {
    /// Zero is used only while a producer is preparing a new certificate.
    ///
    /// The recovery store assigns the next lineage identity before publishing
    /// the certificate. A serialized current manifest must never contain zero.
    pub occurrence: FenceOccurrenceId,
    pub topology_epoch: TopologyEpoch,
    pub fence_domain: FenceDomain,
    pub stores: Vec<StoreFenceRef>,
    pub captured_region_generations: Vec<(RegionId, RecoveryGeneration)>,
    pub captured_integrity_generations: Vec<(IntegrityExtentId, RecoveryGeneration)>,
}

impl FenceCertificate {
    pub fn new(
        topology_epoch: TopologyEpoch,
        fence_domain: FenceDomain,
        stores: Vec<StoreFenceRef>,
        captured_region_generations: Vec<(RegionId, RecoveryGeneration)>,
    ) -> Self {
        Self {
            occurrence: FenceOccurrenceId::UNASSIGNED,
            topology_epoch,
            fence_domain,
            stores,
            captured_region_generations,
            captured_integrity_generations: Vec::new(),
        }
    }

    pub const fn with_occurrence(mut self, occurrence: FenceOccurrenceId) -> Self {
        self.occurrence = occurrence;
        self
    }

    pub const fn occurrence_id(&self) -> FenceOccurrenceId {
        self.occurrence
    }
    /// Compares only immutable certificate facts; occurrence identity is excluded.
    pub fn same_certificate_facts(&self, other: &Self) -> bool {
        self.topology_epoch == other.topology_epoch
            && self.fence_domain == other.fence_domain
            && self.stores == other.stores
            && self.captured_region_generations == other.captured_region_generations
            && self.captured_integrity_generations == other.captured_integrity_generations
    }
    /// Returns whether this occurrence covers all exact facts required by another certificate.
    pub fn covers_certificate(&self, required: &Self) -> bool {
        self.topology_epoch == required.topology_epoch
            && self.fence_domain == required.fence_domain
            && self.stores.len() == required.stores.len()
            && required.stores.iter().all(|required_fence| {
                self.stores.iter().any(|observed| {
                    observed.store_id == required_fence.store_id
                        && observed.topology_epoch == required_fence.topology_epoch
                        && observed.store_incarnation == required_fence.store_incarnation
                        && observed.capability_evidence_id == required_fence.capability_evidence_id
                        && observed.through >= required_fence.through
                })
            })
            && required.captured_region_generations.iter().all(
                |(required_region, required_generation)| {
                    self.captured_region_generations.iter().any(
                        |(observed_region, observed_generation)| {
                            observed_region == required_region
                                && observed_generation >= required_generation
                        },
                    )
                },
            )
            && required.captured_integrity_generations.iter().all(
                |(required_extent, required_generation)| {
                    self.captured_integrity_generations.iter().any(
                        |(observed_extent, observed_generation)| {
                            observed_extent == required_extent
                                && observed_generation >= required_generation
                        },
                    )
                },
            )
    }
    fn covers_store_bindings(&self, required: &Self) -> bool {
        self.stores.len() == required.stores.len()
            && required.stores.iter().all(|required_fence| {
                self.stores.iter().any(|observed| {
                    observed.store_id == required_fence.store_id
                        && observed.topology_epoch == required_fence.topology_epoch
                        && observed.store_incarnation == required_fence.store_incarnation
                        && observed.capability_evidence_id == required_fence.capability_evidence_id
                        && observed.through >= required_fence.through
                })
            })
    }

    pub fn with_integrity_extent(
        mut self,
        extent: IntegrityExtentId,
        generation: RecoveryGeneration,
    ) -> Self {
        self.captured_integrity_generations
            .push((extent, generation));
        self
    }

    pub(crate) fn covers_region(&self, region: RegionId, generation: RecoveryGeneration) -> bool {
        self.captured_region_generations
            .iter()
            .any(|(captured_region, captured_generation)| {
                *captured_region == region && *captured_generation >= generation
            })
    }

    pub(crate) fn contains_store_fence(&self, fence: StoreFenceRef) -> bool {
        self.stores.contains(&fence)
    }

    pub(crate) fn covers_integrity_extent(
        &self,
        extent: IntegrityExtentId,
        generation: RecoveryGeneration,
    ) -> bool {
        self.captured_integrity_generations
            .iter()
            .any(|(captured_extent, captured_generation)| {
                *captured_extent == extent && *captured_generation >= generation
            })
    }
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LegacyUnreconciledRoot {
    pub schema: RecoverySchemaVersion,
    pub occurrence: FenceOccurrenceId,
    pub candidates: Vec<FenceOccurrenceId>,
    pub certificate: FenceCertificate,
}
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct RecoveryRootId(pub u64);

impl RecoveryRootId {
    pub const FIRST: Self = Self(1);
    pub const UNASSIGNED: Self = Self(0);
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RecoveryCommitIntentId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryRootKind {
    CleanRegion,
    ValidIntegrity,
    WritableSession,
    CodedCapture,
    LegacyUnreconciled,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RecoveryRootFact {
    CleanRegion {
        region: RegionId,
        clean_generation: RecoveryGeneration,
    },
    ValidIntegrity {
        extent: IntegrityExtentId,
        profile: ChecksumProfileId,
        set_generation: ChecksumSetGeneration,
        content_generation: RecoveryGeneration,
        digest: Vec<u8>,
    },
    WritableSession {
        session_id: SessionId,
        close_generation: RecoveryGeneration,
    },
    CodedCapture {
        capture: CodedCaptureId,
        operation: dwv_store::OperationSlotToken,
        phase: CodedCapturePhase,
    },
    CodedCleanClosure {
        capture: CodedCaptureId,
        phase: CodedCapturePhase,
    },
    LegacyUnreconciled {
        schema: RecoverySchemaVersion,
        candidates: Vec<FenceOccurrenceId>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RecoveryClaimRoot {
    pub id: RecoveryRootId,
    pub occurrence: FenceOccurrenceId,
    pub certificate: FenceCertificate,
    pub topology_epoch: TopologyEpoch,
    pub generation: RecoveryGeneration,
    pub fact: RecoveryRootFact,
}

impl RecoveryClaimRoot {
    pub const fn kind(&self) -> RecoveryRootKind {
        match self.fact {
            RecoveryRootFact::CleanRegion { .. } => RecoveryRootKind::CleanRegion,
            RecoveryRootFact::ValidIntegrity { .. } => RecoveryRootKind::ValidIntegrity,
            RecoveryRootFact::WritableSession { .. } => RecoveryRootKind::WritableSession,
            RecoveryRootFact::CodedCapture { .. } | RecoveryRootFact::CodedCleanClosure { .. } => {
                RecoveryRootKind::CodedCapture
            }
            RecoveryRootFact::LegacyUnreconciled { .. } => RecoveryRootKind::LegacyUnreconciled,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRootRebind {
    pub root: RecoveryRootId,
    pub predecessor: FenceOccurrenceId,
    pub successor: FenceOccurrenceId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRootDischarge {
    pub root: RecoveryRootId,
    pub occurrence: FenceOccurrenceId,
}

// Root-proof issuers remain private until their owner-specific serving
// cutovers are wired; test builds exercise the full validation surface.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug, Eq, PartialEq)]
enum RecoveryOwnerRetirementProof {
    CleanRegionRebind {
        root: RecoveryRootId,
        region: RegionId,
        successor: FenceOccurrenceId,
    },
    CleanRegionDischarge {
        root: RecoveryRootId,
        region: RegionId,
    },
    IntegrityRebind {
        root: RecoveryRootId,
        extent: IntegrityExtentId,
        successor: FenceOccurrenceId,
    },
    IntegrityDischarge {
        root: RecoveryRootId,
        extent: IntegrityExtentId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRetirementPlan {
    pub predecessor: RecoverySnapshot,
    pub successor: RecoverySnapshot,
    retired_occurrences: Vec<FenceOccurrenceId>,
    rebinds: Vec<RecoveryRootRebind>,
    discharges: Vec<RecoveryRootDischarge>,
    owner_proofs: Vec<RecoveryOwnerRetirementProof>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl RecoveryRetirementPlan {
    pub fn new(predecessor: RecoverySnapshot, successor: RecoverySnapshot) -> Self {
        Self {
            predecessor,
            successor,
            retired_occurrences: Vec::new(),
            rebinds: Vec::new(),
            discharges: Vec::new(),
            owner_proofs: Vec::new(),
        }
    }

    pub fn retire(&mut self, occurrence: FenceOccurrenceId) -> &mut Self {
        self.retired_occurrences.push(occurrence);
        self
    }

    pub(crate) fn rebind(
        &mut self,
        root: RecoveryRootId,
        predecessor: FenceOccurrenceId,
        successor: FenceOccurrenceId,
    ) -> &mut Self {
        self.rebinds.push(RecoveryRootRebind {
            root,
            predecessor,
            successor,
        });
        self
    }

    pub(crate) fn discharge(
        &mut self,
        root: RecoveryRootId,
        occurrence: FenceOccurrenceId,
    ) -> &mut Self {
        self.discharges
            .push(RecoveryRootDischarge { root, occurrence });
        self
    }

    pub(crate) fn authorize_clean_region_rebind(
        &mut self,
        root: RecoveryRootId,
        region: RegionId,
        successor: FenceOccurrenceId,
    ) -> &mut Self {
        self.owner_proofs
            .push(RecoveryOwnerRetirementProof::CleanRegionRebind {
                root,
                region,
                successor,
            });
        self
    }

    pub(crate) fn authorize_clean_region_discharge(
        &mut self,
        root: RecoveryRootId,
        region: RegionId,
    ) -> &mut Self {
        self.owner_proofs
            .push(RecoveryOwnerRetirementProof::CleanRegionDischarge { root, region });
        self
    }

    pub(crate) fn authorize_integrity_rebind(
        &mut self,
        root: RecoveryRootId,
        extent: IntegrityExtentId,
        successor: FenceOccurrenceId,
    ) -> &mut Self {
        self.owner_proofs
            .push(RecoveryOwnerRetirementProof::IntegrityRebind {
                root,
                extent,
                successor,
            });
        self
    }

    pub(crate) fn authorize_integrity_discharge(
        &mut self,
        root: RecoveryRootId,
        extent: IntegrityExtentId,
    ) -> &mut Self {
        self.owner_proofs
            .push(RecoveryOwnerRetirementProof::IntegrityDischarge { root, extent });
        self
    }
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WritableSession {
    pub session_id: SessionId,
    pub topology_epoch: TopologyEpoch,
    pub dirty_envelope_generation: u64,
    pub closed: bool,
    /// Exact generation owned by the close fence, when present.
    pub close_generation: Option<RecoveryGeneration>,
    pub global_fence: Option<FenceCertificate>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StoreAssignment {
    assignment: dwv_core::TopologyAssignment,
    store_id: StoreId,
}

impl StoreAssignment {
    pub const fn slot_id(&self) -> dwv_core::SlotId {
        self.assignment.slot_id()
    }

    pub const fn role(&self) -> dwv_core::MemberRole {
        self.assignment.role()
    }

    pub const fn coding_position(&self) -> dwv_core::CodingPosition {
        self.assignment.coding_position()
    }

    pub const fn assignment_instance(&self) -> dwv_core::AssignmentInstanceId {
        self.assignment.assignment_instance()
    }

    pub const fn assignment_generation(&self) -> dwv_core::AssignmentGeneration {
        self.assignment.assignment_generation()
    }

    pub const fn evidence(&self) -> dwv_core::AssignmentEvidence {
        self.assignment.evidence()
    }

    pub const fn store_id(&self) -> StoreId {
        self.store_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TopologySnapshot {
    array_id: dwv_core::ArrayId,
    topology_epoch: TopologyEpoch,
    profile: dwv_core::CodingProfile,
    geometry: dwv_core::ProtectedGeometry,
    assignments: Vec<StoreAssignment>,
}

impl TopologySnapshot {
    /// dwv:req req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch
    pub fn from_core(
        topology: dwv_core::TopologySnapshot,
        store_ids: Vec<StoreId>,
    ) -> Result<Self, RecoveryTopologyError> {
        topology
            .validate()
            .map_err(RecoveryTopologyError::InvalidCoreTopology)?;
        if topology.assignments().len() != store_ids.len() {
            return Err(RecoveryTopologyError::StoreCount {
                expected: topology.assignments().len(),
                actual: store_ids.len(),
            });
        }
        for (index, store_id) in store_ids.iter().enumerate() {
            if store_ids[..index].contains(store_id) {
                return Err(RecoveryTopologyError::DuplicateStore(*store_id));
            }
        }
        let mut assignments = topology
            .assignments()
            .iter()
            .cloned()
            .zip(store_ids)
            .map(|(assignment, store_id)| StoreAssignment {
                assignment,
                store_id,
            })
            .collect::<Vec<_>>();
        assignments.sort_unstable_by(|left, right| {
            left.slot_id().as_bytes().cmp(&right.slot_id().as_bytes())
        });
        Ok(Self {
            array_id: topology.array_id(),
            topology_epoch: topology.topology_epoch(),
            profile: topology.profile(),
            geometry: topology.geometry(),
            assignments,
        })
    }

    pub const fn array_id(&self) -> dwv_core::ArrayId {
        self.array_id
    }

    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch
    }

    pub const fn profile(&self) -> dwv_core::CodingProfile {
        self.profile
    }

    pub const fn geometry(&self) -> dwv_core::ProtectedGeometry {
        self.geometry
    }

    pub fn assignments(&self) -> &[StoreAssignment] {
        &self.assignments
    }

    pub fn assignment_for_slot(&self, slot_id: dwv_core::SlotId) -> Option<&StoreAssignment> {
        self.assignments
            .iter()
            .find(|assignment| assignment.slot_id() == slot_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryTopologyError {
    InvalidCoreTopology(dwv_core::TopologyValidationError),
    StoreCount { expected: usize, actual: usize },
    DuplicateStore(StoreId),
}

impl fmt::Display for RecoveryTopologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCoreTopology(error) => write!(formatter, "invalid core topology: {error}"),
            Self::StoreCount { expected, actual } => {
                write!(
                    formatter,
                    "topology has {expected} assignments but {actual} stores"
                )
            }
            Self::DuplicateStore(store_id) => {
                write!(formatter, "store {store_id:?} appears more than once")
            }
        }
    }
}

impl std::error::Error for RecoveryTopologyError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MaintenanceCheckpoint {
    pub job: JobId,
    pub cursor: RecoveryCursor,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RecoverySnapshot {
    pub generation: RecoveryGeneration,
    pub topology_epoch: TopologyEpoch,
    pub active_topology: Option<TopologySnapshot>,
    pub pending_topology: Option<TopologySnapshot>,
    pub writable_session: Option<WritableSession>,
    pub dirty_regions: Vec<DirtyRegionRecord>,
    pub integrity_records: Vec<IntegrityRecord>,
    pub checksum_baseline: Option<ChecksumBaseline>,
    pub fences: Vec<FenceCertificate>,
    /// Owner-qualified roots are the only current liveness references.
    pub roots: Vec<RecoveryClaimRoot>,
    /// Retained after root discharge so root identities are not reused.
    pub next_root_id: RecoveryRootId,
    /// Legacy references remain conservative until an owner-qualified rebind.
    pub legacy_unreconciled: Vec<LegacyUnreconciledRoot>,
    /// The next identity is retained after retirement so identities are never reused.
    pub next_fence_occurrence_id: FenceOccurrenceId,
    pub maintenance_checkpoints: Vec<MaintenanceCheckpoint>,
    pub metadata_loss_audit: Option<MetadataLossAudit>,
    pub rebuilds: Vec<RebuildState>,
    #[serde(default)]
    pub coded_captures: Vec<CodedCaptureSnapshot>,
    #[serde(default)]
    pub next_coded_capture_id: u64,
}
impl RecoverySnapshot {
    /// Looks up a persisted certificate by its exact composite occurrence identity.
    pub fn fence(&self, occurrence: FenceOccurrenceId) -> Option<&FenceCertificate> {
        self.fences
            .iter()
            .find(|fence| fence.occurrence == occurrence)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RecoveryManifest {
    pub schema: RecoverySchemaVersion,
    pub snapshot: RecoverySnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryExportLimits {
    pub max_dirty_regions: usize,
    pub max_integrity_records: usize,
    pub max_fences: usize,
    pub max_roots: usize,
    pub max_legacy_unreconciled: usize,
    pub max_maintenance_checkpoints: usize,
    pub max_topology_assignments: usize,
    pub max_stores_per_fence: usize,
    pub max_region_captures_per_fence: usize,
    pub max_integrity_captures_per_fence: usize,
    pub max_digest_bytes: usize,
    pub max_rebuilds: usize,
    pub max_coded_captures: usize,
    pub max_coded_capture_units: usize,
    pub max_coded_capture_memberships: usize,
}

impl Default for RecoveryExportLimits {
    fn default() -> Self {
        Self {
            max_dirty_regions: 16 * 1024,
            max_integrity_records: 16 * 1024,
            max_fences: 16 * 1024,
            max_roots: 16 * 1024,
            max_legacy_unreconciled: 16 * 1024,
            max_maintenance_checkpoints: 1024,
            max_topology_assignments: 1024,
            max_stores_per_fence: 1024,
            max_region_captures_per_fence: 16 * 1024,
            max_integrity_captures_per_fence: 16 * 1024,
            max_digest_bytes: 1024,
            max_rebuilds: 64,
            max_coded_captures: 1024,
            max_coded_capture_units: 64 * 1024,
            max_coded_capture_memberships: 16 * 1024,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryError {
    Unhealthy(RecoveryStoreHealth),
    GenerationExhausted,
    FenceOccurrenceExhausted,
    GenerationMismatch {
        expected: RecoveryGeneration,
        actual: RecoveryGeneration,
    },
    TopologyMismatch {
        expected: TopologyEpoch,
        actual: TopologyEpoch,
    },
    InvalidTransition(TransitionError),
    CommitNotDurable(RecoveryCommitObservation),
    ExportGenerationMismatch {
        requested: RecoveryGeneration,
        actual: RecoveryGeneration,
    },
    UnsupportedSchemaMigration {
        from: RecoverySchemaVersion,
        to: RecoverySchemaVersion,
    },
    ExportLimitExceeded(RecoveryRecordKind),
    ChecksumBaseline(ExtentError),
    Rebuild(RebuildError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionError {
    SessionAlreadyOpen,
    SessionMissing,
    SessionIdMismatch,
    RegionNotDirty,
    RegionAlreadyUnknown,
    FenceEmpty,
    FenceTopologyMismatch,
    FenceOccurrenceMissing,
    FenceOccurrenceAmbiguous,
    FenceOccurrenceReused,
    FenceOccurrenceBindingMismatch,
    FenceNonCanonical,
    FenceDuplicateStore,
    FenceDuplicateRegion,
    FenceDuplicateIntegrity,
    FenceCoverageMissing,
    IntegrityFenceMissing,
    IntegrityGenerationStale,
    PendingTopologyMissing,
    SessionOpen,
    RecoveryCleanGenerationBehind,
    IntegrityCoverageMissing,
    IntegrityContentGenerationFuture,
    IntegrityBindingMismatch,
    CodedCaptureIdentityMismatch,
    CodedCaptureClosureMismatch,
    CodedCaptureMissing,
    CodedCaptureTopologyMismatch,
    CodedSemanticTransitionIncomplete,
    CodedSemanticTransactionConflict,
}

impl fmt::Display for RecoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unhealthy(health) => write!(formatter, "recovery state is {health:?}"),
            Self::GenerationExhausted => write!(formatter, "recovery generation exhausted"),
            Self::FenceOccurrenceExhausted => {
                write!(formatter, "recovery fence occurrence identity exhausted")
            }
            Self::GenerationMismatch { expected, actual } => {
                write!(
                    formatter,
                    "recovery generation mismatch: expected {expected:?}, got {actual:?}"
                )
            }
            Self::TopologyMismatch { expected, actual } => {
                write!(
                    formatter,
                    "topology epoch mismatch: expected {expected:?}, got {actual:?}"
                )
            }
            Self::InvalidTransition(error) => {
                write!(formatter, "invalid recovery transition: {error:?}")
            }
            Self::CommitNotDurable(observation) => {
                write!(
                    formatter,
                    "recovery commit was not durable: {observation:?}"
                )
            }
            Self::ExportGenerationMismatch { requested, actual } => {
                write!(
                    formatter,
                    "manifest generation {requested:?} is not current {actual:?}"
                )
            }
            Self::UnsupportedSchemaMigration { from, to } => {
                write!(
                    formatter,
                    "unsupported recovery schema migration {from:?} -> {to:?}"
                )
            }
            Self::ExportLimitExceeded(kind) => {
                write!(formatter, "recovery export limit exceeded for {kind:?}")
            }
            Self::Rebuild(error) => error.fmt(formatter),
            Self::ChecksumBaseline(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RecoveryError {}

// Exact predecessor-bound mutations stay inline; boxing would allocate on every
// coded recovery-state transition to save a small transaction enum stack slot.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryMutation {
    BeginWritableSession {
        session_id: SessionId,
        topology_epoch: TopologyEpoch,
        dirty_envelope_generation: u64,
    },
    MarkRegionDirty {
        region: RegionId,
        mutation_generation: RecoveryGeneration,
    },
    MarkIntegrityStale {
        extent: IntegrityExtentId,
        stale_generation: RecoveryGeneration,
    },
    RecordDataParityFence {
        fence: FenceCertificate,
    },
    CloseWritableSession {
        session_id: SessionId,
        global_fence: FenceCertificate,
    },
    MarkRegionClean {
        region: RegionId,
        through_generation: RecoveryGeneration,
        fence_occurrence: FenceOccurrenceId,
    },
    InstallIntegrityDigest {
        record: IntegrityRecord,
    },
    PrepareTopology {
        topology: TopologySnapshot,
    },
    CommitTopology {
        topology_epoch: TopologyEpoch,
    },
    RecordMaintenanceCheckpoint {
        job: JobId,
        cursor: RecoveryCursor,
    },
    BeginOfflineRebuild {
        rebuild: RebuildState,
    },
    AdvanceOfflineRebuild {
        receipt: RebuildChunkReceipt,
    },
    CompleteOfflineRebuild {
        receipt: RebuildCompletionReceipt,
    },
    ApplyCodedTransition {
        transition: CodedCaptureTransition,
    },
    RetireFences {
        plan: RecoveryRetirementPlan,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryTxn {
    pub expected_generation: RecoveryGeneration,
    pub expected_topology_epoch: TopologyEpoch,
    pub mutations: Vec<RecoveryMutation>,
}

impl RecoveryTxn {
    pub fn new(
        expected_generation: RecoveryGeneration,
        expected_topology_epoch: TopologyEpoch,
    ) -> Self {
        Self {
            expected_generation,
            expected_topology_epoch,
            mutations: Vec::new(),
        }
    }

    pub fn push(&mut self, mutation: RecoveryMutation) -> &mut Self {
        self.mutations.push(mutation);
        self
    }

    fn validate_coded_composition(&self) -> Result<(), RecoveryError> {
        let has_coded_transition = self
            .mutations
            .iter()
            .any(|mutation| matches!(mutation, RecoveryMutation::ApplyCodedTransition { .. }));
        if !has_coded_transition {
            return Ok(());
        }
        if self
            .mutations
            .iter()
            .any(|mutation| !matches!(mutation, RecoveryMutation::ApplyCodedTransition { .. }))
        {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::CodedSemanticTransactionConflict,
            ));
        }
        for (index, mutation) in self.mutations.iter().enumerate() {
            let RecoveryMutation::ApplyCodedTransition { transition } = mutation else {
                unreachable!("mixed coded transaction rejected above");
            };
            for other in &self.mutations[index + 1..] {
                let RecoveryMutation::ApplyCodedTransition { transition: other } = other else {
                    unreachable!("mixed coded transaction rejected above");
                };
                if transition.conflicts_with(other) {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::CodedSemanticTransactionConflict,
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn mark_region_dirty(
        &mut self,
        region: RegionId,
        mutation_generation: RecoveryGeneration,
    ) -> &mut Self {
        self.push(RecoveryMutation::MarkRegionDirty {
            region,
            mutation_generation,
        })
    }

    pub fn mark_integrity_stale(
        &mut self,
        extent: IntegrityExtentId,
        stale_generation: RecoveryGeneration,
    ) -> &mut Self {
        self.push(RecoveryMutation::MarkIntegrityStale {
            extent,
            stale_generation,
        })
    }

    pub fn begin_offline_rebuild(&mut self, rebuild: RebuildState) -> &mut Self {
        self.push(RecoveryMutation::BeginOfflineRebuild { rebuild })
    }

    pub fn advance_offline_rebuild(&mut self, receipt: RebuildChunkReceipt) -> &mut Self {
        self.push(RecoveryMutation::AdvanceOfflineRebuild { receipt })
    }

    pub fn complete_offline_rebuild(&mut self, receipt: RebuildCompletionReceipt) -> &mut Self {
        self.push(RecoveryMutation::CompleteOfflineRebuild { receipt })
    }
}

pub trait RecoveryStateStore {
    fn load_assembly_snapshot(&self) -> Result<RecoverySnapshot, RecoveryError>;
    fn verify_integrity(&self) -> RecoveryStoreHealth;
    fn begin_protocol_txn(
        &self,
        expected: RecoveryGeneration,
        topology_epoch: TopologyEpoch,
    ) -> RecoveryTxn;
    fn commit_durable(&mut self, txn: RecoveryTxn) -> Result<RecoveryGeneration, RecoveryError>;
    fn commit_observed(
        &mut self,
        txn: RecoveryTxn,
        observation: RecoveryCommitObservation,
    ) -> Result<RecoveryGeneration, RecoveryError> {
        if !observation.is_durable() {
            return Err(RecoveryError::CommitNotDurable(observation));
        }
        self.commit_durable(txn)
    }
    fn commit_durable_receipt(
        &mut self,
        txn: RecoveryTxn,
    ) -> Result<DurableRecoveryCommit, RecoveryError> {
        let expected_generation = txn.expected_generation;
        let topology_epoch = txn.expected_topology_epoch;
        let mutations = txn.mutations.clone();
        let prior_next_occurrence = self.load_assembly_snapshot()?.next_fence_occurrence_id;
        let committed_generation = self.commit_durable(txn)?;
        let persisted_fences = self
            .load_assembly_snapshot()?
            .fences
            .into_iter()
            .filter(|fence| fence.occurrence >= prior_next_occurrence)
            .collect();
        Ok(DurableRecoveryCommit {
            expected_generation,
            topology_epoch,
            committed_generation,
            mutations,
            persisted_fences,
        })
    }
    fn export_manifest(
        &self,
        generation: RecoveryGeneration,
    ) -> Result<RecoveryManifest, RecoveryError>;
}

/// A durable recovery cut that can be reopened without retaining process state.
///
/// The cut contains only the bounded exported owner manifest. Volatile media
/// behavior remains the responsibility of `dwv-sim`; this helper is the
/// recovery-owner side of a crash/reopen test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryReopenCut {
    manifest: RecoveryManifest,
}

impl RecoveryReopenCut {
    /// Capture the durable owner manifest at the requested generation.
    pub fn capture<S: RecoveryStateStore + ?Sized>(
        store: &S,
        generation: RecoveryGeneration,
    ) -> Result<Self, RecoveryError> {
        Ok(Self {
            manifest: store.export_manifest(generation)?,
        })
    }

    /// Return the exact manifest captured by this cut.
    pub fn manifest(&self) -> &RecoveryManifest {
        &self.manifest
    }

    /// Reopen a memory owner from the captured durable manifest.
    pub fn reopen(&self) -> Result<MemoryRecoveryStore, RecoveryError> {
        MemoryRecoveryStore::from_manifest(self.manifest.clone())
    }
}

#[derive(Clone)]
pub struct MemoryRecoveryStore {
    snapshot: RecoverySnapshot,
    health: RecoveryStoreHealth,
}

impl MemoryRecoveryStore {
    pub fn new(topology_epoch: TopologyEpoch) -> Self {
        Self {
            snapshot: RecoverySnapshot {
                generation: RecoveryGeneration(0),
                topology_epoch,
                active_topology: None,
                pending_topology: None,
                writable_session: None,
                dirty_regions: Vec::new(),
                integrity_records: Vec::new(),
                checksum_baseline: None,
                fences: Vec::new(),
                roots: Vec::new(),
                next_root_id: RecoveryRootId::FIRST,
                legacy_unreconciled: Vec::new(),
                next_fence_occurrence_id: FenceOccurrenceId::FIRST,
                maintenance_checkpoints: Vec::new(),
                metadata_loss_audit: None,
                rebuilds: Vec::new(),
                coded_captures: Vec::new(),
                next_coded_capture_id: 0,
            },
            health: RecoveryStoreHealth::Healthy,
        }
    }

    pub fn snapshot(&self) -> &RecoverySnapshot {
        &self.snapshot
    }

    pub(crate) fn with_active_topology(topology: TopologySnapshot) -> Self {
        let mut store = Self::new(topology.topology_epoch());
        store.snapshot.active_topology = Some(topology);
        store
    }

    pub(crate) fn set_checksum_baseline(&mut self, baseline: ChecksumBaseline) {
        self.snapshot.checksum_baseline = Some(baseline);
    }
    pub(crate) fn set_metadata_loss_audit(&mut self, audit: MetadataLossAudit) {
        self.snapshot.metadata_loss_audit = Some(audit);
    }

    pub fn set_health(&mut self, health: RecoveryStoreHealth) {
        self.health = health;
    }

    pub fn health(&self) -> RecoveryStoreHealth {
        self.health
    }

    pub fn write_policy(&self) -> RecoveryWritePolicy {
        self.health.write_policy()
    }

    pub fn disposition(&self) -> RecoveryDisposition {
        self.health.disposition()
    }

    fn apply_mutation(
        snapshot: &mut RecoverySnapshot,
        mutation: RecoveryMutation,
        expected_topology_epoch: TopologyEpoch,
    ) -> Result<(), RecoveryError> {
        match mutation {
            RecoveryMutation::BeginWritableSession {
                session_id,
                topology_epoch,
                dirty_envelope_generation,
            } => {
                if topology_epoch != expected_topology_epoch {
                    return Err(RecoveryError::TopologyMismatch {
                        expected: expected_topology_epoch,
                        actual: topology_epoch,
                    });
                }
                if snapshot
                    .writable_session
                    .as_ref()
                    .is_some_and(|session| !session.closed)
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::SessionAlreadyOpen,
                    ));
                }
                snapshot.writable_session = Some(WritableSession {
                    session_id,
                    topology_epoch,
                    dirty_envelope_generation,
                    closed: false,
                    close_generation: None,
                    global_fence: None,
                });
            }
            RecoveryMutation::MarkRegionDirty {
                region,
                mutation_generation,
            } => {
                upsert_region(
                    snapshot,
                    region,
                    RegionState::Dirty {
                        dirty_since: mutation_generation,
                    },
                );
                discharge_clean_root(snapshot, region);
            }
            RecoveryMutation::MarkIntegrityStale {
                extent,
                stale_generation,
            } => {
                let record = snapshot
                    .integrity_records
                    .iter_mut()
                    .find(|record| record.extent == extent);
                match record {
                    Some(record) => {
                        record.state = match record.state {
                            IntegrityState::Stale {
                                stale_generation: current,
                            } => IntegrityState::Stale {
                                stale_generation: current.max(stale_generation),
                            },
                            _ => IntegrityState::Stale { stale_generation },
                        };
                    }
                    None => snapshot.integrity_records.push(IntegrityRecord {
                        extent,
                        state: IntegrityState::Stale { stale_generation },
                    }),
                }
                discharge_integrity_root(snapshot, extent);
            }
            RecoveryMutation::RecordDataParityFence { fence } => {
                append_fence(snapshot, fence, expected_topology_epoch)?;
            }
            RecoveryMutation::CloseWritableSession {
                session_id,
                global_fence,
            } => {
                let session =
                    snapshot
                        .writable_session
                        .as_ref()
                        .ok_or(RecoveryError::InvalidTransition(
                            TransitionError::SessionMissing,
                        ))?;
                if session.session_id != session_id {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::SessionIdMismatch,
                    ));
                }
                let global_fence = append_fence(snapshot, global_fence, expected_topology_epoch)?;
                let close_generation = snapshot
                    .generation
                    .checked_next()
                    .ok_or(RecoveryError::GenerationExhausted)?;
                {
                    let session = snapshot.writable_session.as_mut().ok_or(
                        RecoveryError::InvalidTransition(TransitionError::SessionMissing),
                    )?;
                    session.closed = true;
                    session.close_generation = Some(close_generation);
                    session.global_fence = Some(global_fence.clone());
                }
                append_root(
                    snapshot,
                    global_fence,
                    RecoveryRootFact::WritableSession {
                        session_id,
                        close_generation,
                    },
                )?;
            }
            RecoveryMutation::MarkRegionClean {
                region,
                through_generation,
                fence_occurrence,
            } => {
                let record = snapshot
                    .dirty_regions
                    .iter_mut()
                    .find(|record| record.region == region)
                    .ok_or(RecoveryError::InvalidTransition(
                        TransitionError::RegionNotDirty,
                    ))?;
                let dirty_since = match record.state {
                    RegionState::Dirty { dirty_since } => dirty_since,
                    RegionState::Clean | RegionState::Indeterminate => {
                        return Err(RecoveryError::InvalidTransition(
                            TransitionError::RegionNotDirty,
                        ));
                    }
                };
                let fence = snapshot
                    .fences
                    .iter()
                    .find(|fence| fence.occurrence == fence_occurrence)
                    .filter(|fence| {
                        fence.topology_epoch == expected_topology_epoch
                            && fence.covers_region(region, dirty_since)
                    })
                    .cloned()
                    .ok_or(RecoveryError::InvalidTransition(
                        TransitionError::FenceCoverageMissing,
                    ))?;
                if through_generation < dirty_since {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::RecoveryCleanGenerationBehind,
                    ));
                }
                let fence_for_root = fence.clone();
                record.state = RegionState::Clean;
                record.clean_generation = Some(through_generation);
                record.last_clean_fence = Some(fence);
                discharge_clean_root(snapshot, region);
                append_root(
                    snapshot,
                    fence_for_root,
                    RecoveryRootFact::CleanRegion {
                        region,
                        clean_generation: through_generation,
                    },
                )?;
            }
            RecoveryMutation::InstallIntegrityDigest { record } => {
                let (
                    binding,
                    content_generation,
                    durable_fence,
                    requested_occurrence,
                    digest,
                    verified_at,
                ) = match record.state {
                    IntegrityState::Valid {
                        binding,
                        content_generation,
                        durable_fence,
                        fence_occurrence,
                        digest,
                        verified_at,
                    } => (
                        binding,
                        content_generation,
                        durable_fence,
                        fence_occurrence,
                        digest,
                        verified_at,
                    ),
                    _ => {
                        return Err(RecoveryError::InvalidTransition(
                            TransitionError::IntegrityFenceMissing,
                        ));
                    }
                };
                if binding.extent.id != record.extent
                    || binding.topology_epoch != expected_topology_epoch
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::IntegrityBindingMismatch,
                    ));
                }
                if requested_occurrence == FenceOccurrenceId::UNASSIGNED {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::FenceOccurrenceMissing,
                    ));
                }
                let integrity_certificate = snapshot.fence(requested_occurrence).ok_or(
                    RecoveryError::InvalidTransition(TransitionError::FenceOccurrenceMissing),
                )?;
                if !integrity_certificate.contains_store_fence(durable_fence) {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::IntegrityBindingMismatch,
                    ));
                }
                if !integrity_certificate.covers_integrity_extent(record.extent, content_generation)
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::IntegrityCoverageMissing,
                    ));
                }
                let integrity_certificate = integrity_certificate.clone();
                let next_generation = snapshot
                    .generation
                    .0
                    .checked_add(1)
                    .map(RecoveryGeneration)
                    .ok_or(RecoveryError::GenerationExhausted)?;
                if content_generation > next_generation || verified_at > next_generation {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::IntegrityContentGenerationFuture,
                    ));
                }
                if let Some(existing) = snapshot
                    .integrity_records
                    .iter()
                    .find(|existing| existing.extent == record.extent)
                    && let IntegrityState::Stale { stale_generation } = existing.state
                    && content_generation < stale_generation
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::IntegrityGenerationStale,
                    ));
                }
                let target = snapshot
                    .integrity_records
                    .iter_mut()
                    .find(|existing| existing.extent == record.extent);
                let root_digest = digest.clone();
                let value = IntegrityRecord {
                    extent: record.extent,
                    state: IntegrityState::Valid {
                        binding,
                        content_generation,
                        durable_fence,
                        fence_occurrence: integrity_certificate.occurrence,
                        digest,
                        verified_at,
                    },
                };
                match target {
                    Some(target) => *target = value,
                    None => snapshot.integrity_records.push(value),
                }
                discharge_integrity_root(snapshot, record.extent);
                append_root(
                    snapshot,
                    integrity_certificate,
                    RecoveryRootFact::ValidIntegrity {
                        extent: record.extent,
                        profile: binding.profile,
                        set_generation: binding.set_generation,
                        content_generation,
                        digest: root_digest,
                    },
                )?;
            }
            RecoveryMutation::PrepareTopology { topology } => {
                if topology.topology_epoch() <= snapshot.topology_epoch {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::FenceTopologyMismatch,
                    ));
                }
                snapshot.pending_topology = Some(topology);
            }
            RecoveryMutation::CommitTopology { topology_epoch } => {
                let pending =
                    snapshot
                        .pending_topology
                        .as_ref()
                        .ok_or(RecoveryError::InvalidTransition(
                            TransitionError::PendingTopologyMissing,
                        ))?;
                if pending.topology_epoch() != topology_epoch {
                    return Err(RecoveryError::TopologyMismatch {
                        expected: topology_epoch,
                        actual: pending.topology_epoch(),
                    });
                }
                if snapshot
                    .writable_session
                    .as_ref()
                    .is_some_and(|session| !session.closed)
                {
                    return Err(RecoveryError::InvalidTransition(
                        TransitionError::SessionOpen,
                    ));
                }
                snapshot.active_topology = snapshot.pending_topology.take();
                snapshot.topology_epoch = topology_epoch;
            }
            RecoveryMutation::RecordMaintenanceCheckpoint { job, cursor } => {
                if let Some(checkpoint) = snapshot
                    .maintenance_checkpoints
                    .iter_mut()
                    .find(|checkpoint| checkpoint.job == job)
                {
                    checkpoint.cursor = cursor;
                } else {
                    snapshot
                        .maintenance_checkpoints
                        .push(MaintenanceCheckpoint { job, cursor });
                }
            }
            RecoveryMutation::BeginOfflineRebuild { rebuild } => {
                if snapshot
                    .rebuilds
                    .iter()
                    .any(|existing| existing.id() == rebuild.id())
                {
                    return Err(RecoveryError::Rebuild(RebuildError::DuplicateRebuildId));
                }
                rebuild
                    .validate_source(snapshot.active_topology.as_ref(), snapshot.generation)
                    .map_err(RecoveryError::Rebuild)?;
                snapshot.rebuilds.push(rebuild);
            }

            RecoveryMutation::AdvanceOfflineRebuild { receipt } => {
                let rebuild = snapshot
                    .rebuilds
                    .iter_mut()
                    .find(|rebuild| rebuild.id() == receipt.rebuild_id())
                    .ok_or(RecoveryError::Rebuild(RebuildError::RebuildNotFound))?;
                rebuild
                    .apply_chunk_receipt(receipt, snapshot.generation)
                    .map_err(RecoveryError::Rebuild)?;
            }
            RecoveryMutation::CompleteOfflineRebuild { receipt } => {
                let rebuild = snapshot
                    .rebuilds
                    .iter_mut()
                    .find(|rebuild| rebuild.id() == receipt.rebuild_id())
                    .ok_or(RecoveryError::Rebuild(RebuildError::RebuildNotFound))?;
                rebuild
                    .apply_completion_receipt(receipt, snapshot.generation)
                    .map_err(RecoveryError::Rebuild)?;
            }
            RecoveryMutation::ApplyCodedTransition { transition } => {
                transition.apply(snapshot, expected_topology_epoch)?;
                sync_coded_capture_roots(snapshot)?;
            }
            RecoveryMutation::RetireFences { plan } => {
                apply_retirement(snapshot, plan, expected_topology_epoch)?;
            }
        }
        Ok(())
    }
}

impl RecoveryStateStore for MemoryRecoveryStore {
    fn load_assembly_snapshot(&self) -> Result<RecoverySnapshot, RecoveryError> {
        if self.health != RecoveryStoreHealth::Healthy {
            return Err(RecoveryError::Unhealthy(self.health));
        }
        Ok(self.snapshot.clone())
    }

    fn verify_integrity(&self) -> RecoveryStoreHealth {
        self.health
    }

    fn begin_protocol_txn(
        &self,
        expected: RecoveryGeneration,
        topology_epoch: TopologyEpoch,
    ) -> RecoveryTxn {
        RecoveryTxn::new(expected, topology_epoch)
    }

    /// dwv:req req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic
    fn commit_durable(&mut self, txn: RecoveryTxn) -> Result<RecoveryGeneration, RecoveryError> {
        if self.health != RecoveryStoreHealth::Healthy {
            return Err(RecoveryError::Unhealthy(self.health));
        }
        if txn.expected_generation != self.snapshot.generation {
            return Err(RecoveryError::GenerationMismatch {
                expected: txn.expected_generation,
                actual: self.snapshot.generation,
            });
        }
        if txn.expected_topology_epoch != self.snapshot.topology_epoch {
            return Err(RecoveryError::TopologyMismatch {
                expected: txn.expected_topology_epoch,
                actual: self.snapshot.topology_epoch,
            });
        }

        txn.validate_coded_composition()?;

        let mut candidate = self.snapshot.clone();
        for mutation in txn.mutations {
            Self::apply_mutation(&mut candidate, mutation, txn.expected_topology_epoch)?;
        }
        candidate.generation = candidate
            .generation
            .checked_next()
            .ok_or(RecoveryError::GenerationExhausted)?;
        validate_root_registry(&candidate)?;
        validate_export_limits(&candidate, RecoveryExportLimits::default())?;
        self.snapshot = candidate;
        Ok(self.snapshot.generation)
    }

    fn export_manifest(
        &self,
        generation: RecoveryGeneration,
    ) -> Result<RecoveryManifest, RecoveryError> {
        self.export_manifest_with_limits(generation, RecoveryExportLimits::default())
    }
}

impl MemoryRecoveryStore {
    pub fn from_manifest(manifest: RecoveryManifest) -> Result<Self, RecoveryError> {
        if manifest.schema != CURRENT_RECOVERY_SCHEMA {
            return Err(RecoveryError::UnsupportedSchemaMigration {
                from: manifest.schema,
                to: CURRENT_RECOVERY_SCHEMA,
            });
        }
        if manifest.snapshot.legacy_unreconciled.is_empty() {
            validate_snapshot_fences(&manifest.snapshot)?;
        } else {
            validate_migrated_snapshot_fences(&manifest.snapshot)?;
        }
        CodedCaptureCoordinator::from_snapshots(manifest.snapshot.coded_captures.clone()).map_err(
            |_| RecoveryError::InvalidTransition(TransitionError::CodedCaptureIdentityMismatch),
        )?;
        let expected_next_capture = manifest
            .snapshot
            .coded_captures
            .iter()
            .map(|capture| capture.capture.0)
            .max()
            .map_or(0, |capture| capture.saturating_add(1));
        if manifest.snapshot.next_coded_capture_id < expected_next_capture {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::CodedCaptureIdentityMismatch,
            ));
        }
        if manifest.snapshot.topology_epoch
            != manifest.snapshot.active_topology.as_ref().map_or(
                manifest.snapshot.topology_epoch,
                TopologySnapshot::topology_epoch,
            )
        {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceTopologyMismatch,
            ));
        }
        for (index, rebuild) in manifest.snapshot.rebuilds.iter().enumerate() {
            if manifest.snapshot.rebuilds[..index]
                .iter()
                .any(|prior| prior.id() == rebuild.id())
            {
                return Err(RecoveryError::Rebuild(RebuildError::DuplicateRebuildId));
            }
            rebuild
                .validate_restored(
                    manifest.snapshot.active_topology.as_ref(),
                    manifest.snapshot.generation,
                )
                .map_err(RecoveryError::Rebuild)?;
        }
        validate_derived_root_bindings(&manifest.snapshot)?;
        if manifest.snapshot.legacy_unreconciled.is_empty() {
            validate_snapshot_fences(&manifest.snapshot)?;
        } else {
            validate_migrated_snapshot_fences(&manifest.snapshot)?;
        }
        validate_export_limits(&manifest.snapshot, RecoveryExportLimits::default())?;
        let health = if manifest.snapshot.legacy_unreconciled.is_empty() {
            RecoveryStoreHealth::Healthy
        } else {
            RecoveryStoreHealth::Stale
        };
        Ok(Self {
            snapshot: manifest.snapshot,
            health,
        })
    }

    /// dwv:req req.recovery-state-semantics.legacy-fence-retention-migrates-explicitly
    /// Assigns current occurrence identities to a legacy semantic manifest.
    ///
    /// The schema gate is explicit: ordinary restoration rejects the legacy
    /// manifest, while this operation is the only path that may reinterpret
    /// its zero-valued compatibility fields.
    pub fn migrate_legacy_manifest(
        mut manifest: RecoveryManifest,
    ) -> Result<RecoveryManifest, RecoveryError> {
        let legacy_schema = manifest.schema;
        if legacy_schema == CURRENT_RECOVERY_SCHEMA {
            return Ok(manifest);
        }
        RecoveryMigrationPlan::plan(legacy_schema, CURRENT_RECOVERY_SCHEMA)?;
        for fence in &manifest.snapshot.fences {
            validate_legacy_fence(fence, fence.topology_epoch)?;
        }
        let mut next = FenceOccurrenceId::FIRST;
        for fence in &mut manifest.snapshot.fences {
            fence.occurrence = next;
            next = next
                .0
                .checked_add(1)
                .map(FenceOccurrenceId)
                .ok_or(RecoveryError::FenceOccurrenceExhausted)?;
        }
        let mut candidates = manifest.snapshot.fences.clone();
        let mut legacy_roots = candidates
            .iter()
            .map(|certificate| LegacyUnreconciledRoot {
                schema: legacy_schema,
                occurrence: certificate.occurrence,
                candidates: vec![certificate.occurrence],
                certificate: certificate.clone(),
            })
            .collect::<Vec<_>>();
        if let Some(session) = manifest.snapshot.writable_session.as_mut()
            && let Some(fence) = session.global_fence.as_mut()
        {
            migrate_legacy_fence_reference(
                fence,
                &mut candidates,
                &mut next,
                legacy_schema,
                &mut legacy_roots,
            )?;
        }
        for region in &mut manifest.snapshot.dirty_regions {
            if let Some(fence) = region.last_clean_fence.as_mut() {
                migrate_legacy_fence_reference(
                    fence,
                    &mut candidates,
                    &mut next,
                    legacy_schema,
                    &mut legacy_roots,
                )?;
            }
        }
        for capture in &mut manifest.snapshot.coded_captures {
            if let Some(fence) = capture.clean_closure_fence.as_mut() {
                migrate_legacy_fence_reference(
                    fence,
                    &mut candidates,
                    &mut next,
                    legacy_schema,
                    &mut legacy_roots,
                )?;
            }
            for fence in capture.release_certificates.values_mut() {
                migrate_legacy_fence_reference(
                    fence,
                    &mut candidates,
                    &mut next,
                    legacy_schema,
                    &mut legacy_roots,
                )?;
            }
        }
        for record in &mut manifest.snapshot.integrity_records {
            migrate_legacy_integrity_reference(
                record,
                &candidates,
                legacy_schema,
                &mut legacy_roots,
            )?;
        }
        manifest.snapshot.fences = candidates;
        manifest.snapshot.next_fence_occurrence_id = next;
        manifest.snapshot.next_root_id = RecoveryRootId::FIRST;
        manifest.snapshot.legacy_unreconciled = legacy_roots;
        manifest.schema = CURRENT_RECOVERY_SCHEMA;
        validate_migrated_snapshot_fences(&manifest.snapshot)?;
        validate_export_limits(&manifest.snapshot, RecoveryExportLimits::default())?;
        Ok(manifest)
    }

    pub fn export_manifest_with_limits(
        &self,
        generation: RecoveryGeneration,
        limits: RecoveryExportLimits,
    ) -> Result<RecoveryManifest, RecoveryError> {
        if self.health != RecoveryStoreHealth::Healthy {
            return Err(RecoveryError::Unhealthy(self.health));
        }
        if generation != self.snapshot.generation {
            return Err(RecoveryError::ExportGenerationMismatch {
                requested: generation,
                actual: self.snapshot.generation,
            });
        }
        validate_snapshot_fences(&self.snapshot)?;
        validate_export_limits(&self.snapshot, limits)?;
        Ok(RecoveryManifest {
            schema: CURRENT_RECOVERY_SCHEMA,
            snapshot: self.snapshot.clone(),
        })
    }
}

fn migrate_legacy_fence_reference(
    reference: &mut FenceCertificate,
    candidates: &mut Vec<FenceCertificate>,
    next: &mut FenceOccurrenceId,
    schema: RecoverySchemaVersion,
    roots: &mut Vec<LegacyUnreconciledRoot>,
) -> Result<(), RecoveryError> {
    let matches = candidates
        .iter()
        .filter(|candidate| reference.same_certificate_facts(candidate))
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => {
            validate_legacy_fence(reference, reference.topology_epoch)?;
            let occurrence = *next;
            *next = next
                .0
                .checked_add(1)
                .map(FenceOccurrenceId)
                .ok_or(RecoveryError::FenceOccurrenceExhausted)?;
            let promoted = reference.clone().with_occurrence(occurrence);
            candidates.push(promoted.clone());
            roots.push(LegacyUnreconciledRoot {
                schema,
                occurrence,
                candidates: vec![occurrence],
                certificate: promoted.clone(),
            });
            *reference = promoted;
            Ok(())
        }
        [candidate] => {
            *reference = candidate.clone();
            Ok(())
        }
        matches => {
            let candidate_ids = matches
                .iter()
                .map(FenceCertificate::occurrence_id)
                .collect::<Vec<_>>();
            for candidate in matches {
                roots.push(LegacyUnreconciledRoot {
                    schema,
                    occurrence: candidate.occurrence_id(),
                    candidates: candidate_ids.clone(),
                    certificate: reference.clone(),
                });
            }
            Ok(())
        }
    }
}
fn migrate_legacy_integrity_reference(
    record: &mut IntegrityRecord,
    candidates: &[FenceCertificate],
    schema: RecoverySchemaVersion,
    roots: &mut Vec<LegacyUnreconciledRoot>,
) -> Result<(), RecoveryError> {
    let IntegrityState::Valid {
        durable_fence,
        fence_occurrence,
        content_generation,
        ..
    } = &mut record.state
    else {
        return Ok(());
    };
    let matches = candidates
        .iter()
        .filter(|candidate| {
            candidate.contains_store_fence(*durable_fence)
                && candidate.covers_integrity_extent(record.extent, *content_generation)
        })
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceMissing,
        )),
        [candidate] => {
            *fence_occurrence = candidate.occurrence;
            Ok(())
        }
        matches => {
            *fence_occurrence = FenceOccurrenceId::UNASSIGNED;
            let candidate_ids = matches
                .iter()
                .map(FenceCertificate::occurrence_id)
                .collect::<Vec<_>>();
            for candidate in matches {
                roots.push(LegacyUnreconciledRoot {
                    schema,
                    occurrence: candidate.occurrence,
                    candidates: candidate_ids.clone(),
                    certificate: candidate.clone(),
                });
            }
            Ok(())
        }
    }
}

fn upsert_region(snapshot: &mut RecoverySnapshot, region: RegionId, state: RegionState) {
    if let Some(record) = snapshot
        .dirty_regions
        .iter_mut()
        .find(|record| record.region == region)
    {
        record.state = state;
        if !matches!(record.state, RegionState::Clean) {
            record.clean_generation = None;
        }
    } else {
        snapshot.dirty_regions.push(DirtyRegionRecord {
            region,
            state,
            last_clean_fence: None,
            clean_generation: None,
        });
    }
}
fn validate_snapshot_fences(snapshot: &RecoverySnapshot) -> Result<(), RecoveryError> {
    validate_snapshot_fences_with_mode(snapshot, true)
}

fn validate_migrated_snapshot_fences(snapshot: &RecoverySnapshot) -> Result<(), RecoveryError> {
    validate_snapshot_fences_with_mode(snapshot, false)
}

fn validate_snapshot_fences_with_mode(
    snapshot: &RecoverySnapshot,
    canonical: bool,
) -> Result<(), RecoveryError> {
    if snapshot.next_fence_occurrence_id == FenceOccurrenceId::UNASSIGNED {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceMissing,
        ));
    }
    let mut previous_occurrence = FenceOccurrenceId::UNASSIGNED;
    for fence in &snapshot.fences {
        if canonical {
            validate_persisted_fence(fence, fence.topology_epoch)?;
        } else {
            validate_legacy_fence(fence, fence.topology_epoch)?;
            if fence.occurrence == FenceOccurrenceId::UNASSIGNED {
                return Err(RecoveryError::InvalidTransition(
                    TransitionError::FenceOccurrenceMissing,
                ));
            }
        }
        if fence.occurrence <= previous_occurrence {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceReused,
            ));
        }
        previous_occurrence = fence.occurrence;
    }
    if snapshot.next_fence_occurrence_id <= previous_occurrence {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceReused,
        ));
    }
    for root in &snapshot.legacy_unreconciled {
        if root.schema >= CURRENT_RECOVERY_SCHEMA
            || root.occurrence == FenceOccurrenceId::UNASSIGNED
            || root.candidates.is_empty()
            || !root.candidates.contains(&root.occurrence)
            || root.candidates.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceAmbiguous,
            ));
        }
        validate_legacy_fence(&root.certificate, root.certificate.topology_epoch)?;
        for candidate in &root.candidates {
            let fence = snapshot
                .fences
                .iter()
                .find(|fence| fence.occurrence == *candidate)
                .ok_or(RecoveryError::InvalidTransition(
                    TransitionError::FenceOccurrenceMissing,
                ))?;
            if !fence.same_certificate_facts(&root.certificate) {
                return Err(RecoveryError::InvalidTransition(
                    TransitionError::FenceOccurrenceAmbiguous,
                ));
            }
        }
    }
    if let Some(session) = snapshot.writable_session.as_ref()
        && let Some(fence) = session.global_fence.as_ref()
    {
        validate_snapshot_fence_reference(snapshot, fence)?;
    }
    for region in &snapshot.dirty_regions {
        if matches!(region.state, RegionState::Clean)
            && let Some(fence) = region.last_clean_fence.as_ref()
        {
            validate_snapshot_fence_reference(snapshot, fence)?;
        }
    }
    for capture in &snapshot.coded_captures {
        if let Some(fence) = capture.clean_closure_fence.as_ref() {
            validate_snapshot_fence_reference(snapshot, fence)?;
        }
        for fence in capture.release_certificates.values() {
            validate_snapshot_fence_reference(snapshot, fence)?;
        }
    }
    validate_integrity_fence_references(snapshot)?;
    validate_root_registry(snapshot)?;
    Ok(())
}

fn validate_snapshot_fence_reference(
    snapshot: &RecoverySnapshot,
    fence: &FenceCertificate,
) -> Result<(), RecoveryError> {
    if fence.occurrence == FenceOccurrenceId::UNASSIGNED {
        if snapshot
            .legacy_unreconciled
            .iter()
            .any(|root| root.certificate.same_certificate_facts(fence))
        {
            return Ok(());
        }
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceMissing,
        ));
    }
    let stored = snapshot
        .fences
        .iter()
        .find(|candidate| candidate.occurrence == fence.occurrence)
        .ok_or(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceMissing,
        ))?;
    if !stored.same_certificate_facts(fence) {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ));
    }
    Ok(())
}

fn validate_integrity_fence_references(snapshot: &RecoverySnapshot) -> Result<(), RecoveryError> {
    for record in &snapshot.integrity_records {
        let IntegrityState::Valid {
            durable_fence,
            fence_occurrence,
            content_generation,
            ..
        } = &record.state
        else {
            continue;
        };
        if *fence_occurrence == FenceOccurrenceId::UNASSIGNED {
            if snapshot.legacy_unreconciled.iter().any(|root| {
                root.certificate.contains_store_fence(*durable_fence)
                    && root
                        .certificate
                        .covers_integrity_extent(record.extent, *content_generation)
            }) {
                continue;
            }
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceMissing,
            ));
        }
        let certificate =
            snapshot
                .fence(*fence_occurrence)
                .ok_or(RecoveryError::InvalidTransition(
                    TransitionError::FenceOccurrenceMissing,
                ))?;
        if !certificate.contains_store_fence(*durable_fence)
            || !certificate.covers_integrity_extent(record.extent, *content_generation)
        {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch,
            ));
        }
    }
    Ok(())
}

fn validate_export_limits(
    snapshot: &RecoverySnapshot,
    limits: RecoveryExportLimits,
) -> Result<(), RecoveryError> {
    if snapshot.dirty_regions.len() > limits.max_dirty_regions {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::DirtyRegion,
        ));
    }
    if snapshot.integrity_records.len() > limits.max_integrity_records {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::IntegrityEvidence,
        ));
    }
    if snapshot.roots.len() > limits.max_roots {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::ClaimRoot,
        ));
    }
    if snapshot.legacy_unreconciled.len() > limits.max_legacy_unreconciled {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::LegacyUnreconciled,
        ));
    }
    if snapshot
        .checksum_baseline
        .as_ref()
        .is_some_and(|baseline| baseline.expected_extents.len() > limits.max_integrity_records)
    {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::ChecksumBaseline,
        ));
    }
    if snapshot.coded_captures.len() > limits.max_coded_captures
        || snapshot.coded_captures.iter().any(|capture| {
            capture.scope.len() > limits.max_coded_capture_units
                || capture.membership.len() > limits.max_coded_capture_memberships
                || capture.pending_later_cuts.len() > limits.max_coded_capture_memberships
                || capture.release_authorized_operations.len()
                    > limits.max_coded_capture_memberships
        })
    {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::CodedCleanCapture,
        ));
    }
    if snapshot.fences.len() > limits.max_fences {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::StoreFence,
        ));
    }
    for fence in &snapshot.fences {
        if fence.stores.len() > limits.max_stores_per_fence
            || fence.captured_region_generations.len() > limits.max_region_captures_per_fence
            || fence.captured_integrity_generations.len() > limits.max_integrity_captures_per_fence
        {
            return Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::StoreFence,
            ));
        }
    }
    if snapshot.maintenance_checkpoints.len() > limits.max_maintenance_checkpoints {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::MaintenanceCheckpoint,
        ));
    }
    if snapshot.rebuilds.len() > limits.max_rebuilds {
        return Err(RecoveryError::ExportLimitExceeded(
            RecoveryRecordKind::OfflineRebuild,
        ));
    }
    for topology in [
        snapshot.active_topology.as_ref(),
        snapshot.pending_topology.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if topology.assignments().len() > limits.max_topology_assignments {
            return Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::Topology,
            ));
        }
    }
    for rebuild in &snapshot.rebuilds {
        if rebuild.source_topology().assignments().len() > limits.max_topology_assignments {
            return Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::OfflineRebuild,
            ));
        }
    }
    for record in &snapshot.integrity_records {
        if let IntegrityState::Valid { digest, .. } = &record.state
            && digest.len() > limits.max_digest_bytes
        {
            return Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::IntegrityEvidence,
            ));
        }
    }
    Ok(())
}

fn validate_store_fence(
    fence: &StoreFenceRef,
    expected_topology_epoch: TopologyEpoch,
) -> Result<(), RecoveryError> {
    if fence.topology_epoch != expected_topology_epoch {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceTopologyMismatch,
        ));
    }
    Ok(())
}

fn validate_fence(
    certificate: &FenceCertificate,
    expected_topology_epoch: TopologyEpoch,
) -> Result<(), RecoveryError> {
    if certificate.topology_epoch != expected_topology_epoch {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceTopologyMismatch,
        ));
    }
    if certificate.stores.is_empty() {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceEmpty,
        ));
    }

    let mut previous_store_order = None;
    let mut previous_store_id = None;
    for fence in &certificate.stores {
        validate_store_fence(fence, expected_topology_epoch)?;
        if previous_store_id == Some(fence.store_id) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceDuplicateStore,
            ));
        }
        let order = (
            fence.store_id,
            fence.fence_id,
            fence.topology_epoch,
            fence.store_incarnation,
            fence.capability_evidence_id,
            fence.through,
        );
        if previous_store_order.is_some_and(|previous| previous >= order) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceNonCanonical,
            ));
        }
        previous_store_id = Some(fence.store_id);
        previous_store_order = Some(order);
    }

    let mut previous_region = None;
    let mut previous_region_order = None;
    for (region, generation) in &certificate.captured_region_generations {
        if previous_region == Some(*region) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceDuplicateRegion,
            ));
        }
        let order = (*region, *generation);
        if previous_region_order.is_some_and(|previous| previous >= order) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceNonCanonical,
            ));
        }
        previous_region = Some(*region);
        previous_region_order = Some(order);
    }

    let mut previous_extent = None;
    let mut previous_extent_order = None;
    for (extent, generation) in &certificate.captured_integrity_generations {
        if previous_extent == Some(*extent) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceDuplicateIntegrity,
            ));
        }
        let order = (*extent, *generation);
        if previous_extent_order.is_some_and(|previous| previous >= order) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceNonCanonical,
            ));
        }
        previous_extent = Some(*extent);
        previous_extent_order = Some(order);
    }
    Ok(())
}
fn validate_legacy_fence(
    certificate: &FenceCertificate,
    expected_topology_epoch: TopologyEpoch,
) -> Result<(), RecoveryError> {
    if certificate.topology_epoch != expected_topology_epoch {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceTopologyMismatch,
        ));
    }
    if certificate.stores.is_empty() {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceEmpty,
        ));
    }
    for fence in &certificate.stores {
        validate_store_fence(fence, expected_topology_epoch)?;
    }
    Ok(())
}

fn validate_persisted_fence(
    certificate: &FenceCertificate,
    expected_topology_epoch: TopologyEpoch,
) -> Result<(), RecoveryError> {
    validate_fence(certificate, expected_topology_epoch)?;
    if certificate.occurrence == FenceOccurrenceId::UNASSIGNED {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceMissing,
        ));
    }
    Ok(())
}

fn append_fence(
    snapshot: &mut RecoverySnapshot,
    mut fence: FenceCertificate,
    expected_topology_epoch: TopologyEpoch,
) -> Result<FenceCertificate, RecoveryError> {
    validate_fence(&fence, expected_topology_epoch)?;
    let next = snapshot.next_fence_occurrence_id;
    if next == FenceOccurrenceId::UNASSIGNED {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceMissing,
        ));
    }
    let occurrence = match fence.occurrence {
        FenceOccurrenceId::UNASSIGNED => next,
        occurrence if occurrence == next => occurrence,
        _ => {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceReused,
            ));
        }
    };
    let next_occurrence = occurrence
        .0
        .checked_add(1)
        .map(FenceOccurrenceId)
        .ok_or(RecoveryError::FenceOccurrenceExhausted)?;
    fence.occurrence = occurrence;
    snapshot.fences.push(fence.clone());
    snapshot.next_fence_occurrence_id = next_occurrence;
    Ok(fence)
}
fn append_root(
    snapshot: &mut RecoverySnapshot,
    certificate: FenceCertificate,
    fact: RecoveryRootFact,
) -> Result<(), RecoveryError> {
    let generation = snapshot
        .generation
        .checked_next()
        .ok_or(RecoveryError::GenerationExhausted)?;
    append_root_at_generation(snapshot, certificate, fact, generation)
}

fn append_root_at_generation(
    snapshot: &mut RecoverySnapshot,
    certificate: FenceCertificate,
    fact: RecoveryRootFact,
    generation: RecoveryGeneration,
) -> Result<(), RecoveryError> {
    if certificate.occurrence == FenceOccurrenceId::UNASSIGNED {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceMissing,
        ));
    }
    if !snapshot.fences.iter().any(|fence| {
        fence.occurrence == certificate.occurrence && fence.same_certificate_facts(&certificate)
    }) {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ));
    }
    let id = snapshot.next_root_id;
    if id == RecoveryRootId::UNASSIGNED {
        return Err(RecoveryError::GenerationExhausted);
    }
    snapshot.next_root_id =
        id.0.checked_add(1)
            .map(RecoveryRootId)
            .ok_or(RecoveryError::GenerationExhausted)?;
    snapshot.roots.push(RecoveryClaimRoot {
        id,
        occurrence: certificate.occurrence,
        topology_epoch: certificate.topology_epoch,
        generation,
        certificate,
        fact,
    });
    Ok(())
}

fn discharge_clean_root(snapshot: &mut RecoverySnapshot, region: RegionId) {
    snapshot.roots.retain(|root| {
        !matches!(
            root.fact,
            RecoveryRootFact::CleanRegion {
                region: root_region,
                ..
            } if root_region == region
        )
    });
}

fn discharge_integrity_root(snapshot: &mut RecoverySnapshot, extent: IntegrityExtentId) {
    snapshot.roots.retain(|root| {
        !matches!(
            root.fact,
            RecoveryRootFact::ValidIntegrity {
                extent: root_extent,
                ..
            } if root_extent == extent
        )
    });
}

fn sync_coded_capture_roots(snapshot: &mut RecoverySnapshot) -> Result<(), RecoveryError> {
    let mut bindings = Vec::new();
    for capture in &snapshot.coded_captures {
        if let Some(certificate) = capture
            .clean_closure_fence
            .as_ref()
            .filter(|certificate| certificate.occurrence != FenceOccurrenceId::UNASSIGNED)
        {
            bindings.push((
                certificate.clone(),
                RecoveryRootFact::CodedCleanClosure {
                    capture: capture.capture,
                    phase: capture.phase,
                },
            ));
        }
        for (operation, certificate) in capture
            .release_certificates
            .iter()
            .filter(|(_, certificate)| certificate.occurrence != FenceOccurrenceId::UNASSIGNED)
        {
            bindings.push((
                certificate.clone(),
                RecoveryRootFact::CodedCapture {
                    capture: capture.capture,
                    operation: *operation,
                    phase: capture.phase,
                },
            ));
        }
    }
    snapshot.roots.retain(|root| {
        root.kind() != RecoveryRootKind::CodedCapture
            || bindings.iter().any(|(certificate, fact)| {
                root.occurrence == certificate.occurrence
                    && root.fact == *fact
                    && root.certificate.same_certificate_facts(certificate)
            })
    });
    for (certificate, fact) in bindings {
        if !root_exists(snapshot, &certificate, &fact) {
            append_root(snapshot, certificate, fact)?;
        }
    }
    Ok(())
}
fn root_exists(
    snapshot: &RecoverySnapshot,
    certificate: &FenceCertificate,
    fact: &RecoveryRootFact,
) -> bool {
    snapshot.roots.iter().any(|root| {
        root.occurrence == certificate.occurrence
            && root.fact == *fact
            && root.certificate.same_certificate_facts(certificate)
    })
}
/// dwv:req req.recovery-state-semantics.exact-persistence-evidence-follows-owner-qualified-claim-liveness
fn validate_derived_root_bindings(snapshot: &RecoverySnapshot) -> Result<(), RecoveryError> {
    if !snapshot.legacy_unreconciled.is_empty() {
        return Ok(());
    }

    let clean_bindings = snapshot
        .dirty_regions
        .iter()
        .filter_map(|record| {
            if !matches!(record.state, RegionState::Clean) {
                return None;
            }
            let clean_generation = record.clean_generation?;
            let certificate = record
                .last_clean_fence
                .as_ref()
                .filter(|certificate| certificate.occurrence != FenceOccurrenceId::UNASSIGNED)?;
            Some((
                certificate.clone(),
                RecoveryRootFact::CleanRegion {
                    region: record.region,
                    clean_generation,
                },
            ))
        })
        .collect::<Vec<_>>();
    for (certificate, fact) in clean_bindings {
        if !root_exists(snapshot, &certificate, &fact) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch,
            ));
        }
    }

    let integrity_bindings = snapshot
        .integrity_records
        .iter()
        .filter_map(|record| {
            let IntegrityState::Valid {
                binding,
                content_generation,
                fence_occurrence,
                digest,
                ..
            } = &record.state
            else {
                return None;
            };
            let certificate = snapshot.fence(*fence_occurrence)?.clone();
            Some((
                certificate,
                RecoveryRootFact::ValidIntegrity {
                    extent: record.extent,
                    profile: binding.profile,
                    set_generation: binding.set_generation,
                    content_generation: *content_generation,
                    digest: digest.clone(),
                },
            ))
        })
        .collect::<Vec<_>>();
    for (certificate, fact) in integrity_bindings {
        if !root_exists(snapshot, &certificate, &fact) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch,
            ));
        }
    }

    if let Some(session) = snapshot.writable_session.as_ref()
        && session.closed
        && let Some(close_generation) = session.close_generation
        && let Some(certificate) = session.global_fence.as_ref()
        && certificate.occurrence != FenceOccurrenceId::UNASSIGNED
    {
        let fact = RecoveryRootFact::WritableSession {
            session_id: session.session_id,
            close_generation,
        };
        if !root_exists(snapshot, certificate, &fact) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch,
            ));
        }
    }

    let mut capture_bindings = Vec::new();
    for capture in &snapshot.coded_captures {
        if let Some(certificate) = capture
            .clean_closure_fence
            .as_ref()
            .filter(|certificate| certificate.occurrence != FenceOccurrenceId::UNASSIGNED)
        {
            capture_bindings.push((
                certificate.clone(),
                RecoveryRootFact::CodedCleanClosure {
                    capture: capture.capture,
                    phase: capture.phase,
                },
            ));
        }
        for (operation, certificate) in capture
            .release_certificates
            .iter()
            .filter(|(_, certificate)| certificate.occurrence != FenceOccurrenceId::UNASSIGNED)
        {
            capture_bindings.push((
                certificate.clone(),
                RecoveryRootFact::CodedCapture {
                    capture: capture.capture,
                    operation: *operation,
                    phase: capture.phase,
                },
            ));
        }
    }
    for (certificate, fact) in capture_bindings {
        if !root_exists(snapshot, &certificate, &fact) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch,
            ));
        }
    }
    Ok(())
}

fn validate_root_owner_binding(
    snapshot: &RecoverySnapshot,
    root: &RecoveryClaimRoot,
) -> Result<(), RecoveryError> {
    let invalid = || {
        Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ))
    };
    match &root.fact {
        RecoveryRootFact::CleanRegion {
            region,
            clean_generation,
        } => {
            let valid = snapshot.dirty_regions.iter().any(|record| {
                record.region == *region
                    && matches!(record.state, RegionState::Clean)
                    && record
                        .clean_generation
                        .is_none_or(|generation| generation == *clean_generation)
                    && record.last_clean_fence.as_ref().is_some_and(|fence| {
                        fence.occurrence == root.occurrence
                            && fence.same_certificate_facts(&root.certificate)
                    })
                    && root.certificate.covers_region(*region, *clean_generation)
            });
            if !valid {
                return invalid();
            }
        }
        RecoveryRootFact::ValidIntegrity {
            extent,
            profile,
            set_generation,
            content_generation,
            digest,
        } => {
            let valid = snapshot.integrity_records.iter().any(|record| {
                if record.extent != *extent {
                    return false;
                }
                let IntegrityState::Valid {
                    binding,
                    content_generation: record_generation,
                    durable_fence,
                    fence_occurrence,
                    digest: record_digest,
                    ..
                } = &record.state
                else {
                    return false;
                };
                binding.extent.id == *extent
                    && binding.topology_epoch == root.topology_epoch
                    && binding.profile == *profile
                    && binding.set_generation == *set_generation
                    && *record_generation == *content_generation
                    && *record_digest == *digest
                    && *fence_occurrence == root.occurrence
                    && root.certificate.contains_store_fence(*durable_fence)
                    && root
                        .certificate
                        .covers_integrity_extent(*extent, *content_generation)
            });
            if !valid {
                return invalid();
            }
        }
        RecoveryRootFact::WritableSession {
            session_id,
            close_generation,
        } => {
            if *close_generation > snapshot.generation {
                return invalid();
            }
            if let Some(session) = snapshot
                .writable_session
                .as_ref()
                .filter(|session| session.session_id == *session_id)
            {
                let valid = session.closed
                    && session.topology_epoch == root.topology_epoch
                    && session
                        .close_generation
                        .is_none_or(|generation| generation == *close_generation)
                    && session.global_fence.as_ref().is_some_and(|fence| {
                        fence.occurrence == root.occurrence
                            && fence.same_certificate_facts(&root.certificate)
                    });
                if !valid {
                    return invalid();
                }
            }
        }
        RecoveryRootFact::CodedCapture {
            capture,
            operation,
            phase,
        } => {
            let valid = snapshot.coded_captures.iter().any(|candidate| {
                candidate.capture == *capture
                    && candidate.topology.topology_epoch() == root.topology_epoch
                    && candidate.phase == *phase
                    && candidate
                        .release_certificates
                        .get(operation)
                        .is_some_and(|certificate| {
                            certificate.occurrence == root.occurrence
                                && certificate.same_certificate_facts(&root.certificate)
                        })
            });
            if !valid {
                return invalid();
            }
        }
        RecoveryRootFact::CodedCleanClosure { capture, phase } => {
            let valid = snapshot.coded_captures.iter().any(|candidate| {
                candidate.capture == *capture
                    && candidate.topology.topology_epoch() == root.topology_epoch
                    && candidate.phase == *phase
                    && candidate.dirty_regions.iter().all(|region| {
                        root.certificate
                            .covers_region(*region, candidate.recovery_generation)
                    })
                    && candidate.checksum_extents.iter().all(|extent| {
                        root.certificate
                            .covers_integrity_extent(*extent, candidate.recovery_generation)
                    })
                    && candidate
                        .clean_closure_fence
                        .as_ref()
                        .is_some_and(|certificate| {
                            certificate.occurrence == root.occurrence
                                && certificate.same_certificate_facts(&root.certificate)
                        })
            });
            if !valid {
                return invalid();
            }
        }
        RecoveryRootFact::LegacyUnreconciled { .. } => return invalid(),
    }
    Ok(())
}

fn successor_covers_root(successor: &FenceCertificate, predecessor: &RecoveryClaimRoot) -> bool {
    if successor.topology_epoch != predecessor.certificate.topology_epoch
        || successor.fence_domain != predecessor.certificate.fence_domain
        || !successor.covers_store_bindings(&predecessor.certificate)
    {
        return false;
    }
    match predecessor.fact {
        RecoveryRootFact::CleanRegion {
            region,
            clean_generation,
        } => successor.covers_region(region, clean_generation),
        RecoveryRootFact::ValidIntegrity {
            extent,
            content_generation,
            ..
        } => successor.covers_integrity_extent(extent, content_generation),
        RecoveryRootFact::WritableSession { .. }
        | RecoveryRootFact::CodedCapture { .. }
        | RecoveryRootFact::CodedCleanClosure { .. }
        | RecoveryRootFact::LegacyUnreconciled { .. } => {
            successor.covers_certificate(&predecessor.certificate)
        }
    }
}

fn validate_root_registry(snapshot: &RecoverySnapshot) -> Result<(), RecoveryError> {
    if snapshot.next_root_id == RecoveryRootId::UNASSIGNED {
        return Err(RecoveryError::GenerationExhausted);
    }
    let mut previous_id = RecoveryRootId::UNASSIGNED;
    for root in &snapshot.roots {
        if root.id <= previous_id
            || root.generation > snapshot.generation
            || root.occurrence == FenceOccurrenceId::UNASSIGNED
            || root.topology_epoch != root.certificate.topology_epoch
        {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch,
            ));
        }
        let certificate = snapshot
            .fences
            .iter()
            .find(|fence| fence.occurrence == root.occurrence)
            .ok_or(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceMissing,
            ))?;
        if !certificate.same_certificate_facts(&root.certificate) {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch,
            ));
        }
        validate_root_owner_binding(snapshot, root)?;
        previous_id = root.id;
    }
    if snapshot.next_root_id <= previous_id {
        return Err(RecoveryError::GenerationExhausted);
    }
    Ok(())
}

fn owner_proof_root(proof: &RecoveryOwnerRetirementProof) -> RecoveryRootId {
    match proof {
        RecoveryOwnerRetirementProof::CleanRegionRebind { root, .. }
        | RecoveryOwnerRetirementProof::CleanRegionDischarge { root, .. }
        | RecoveryOwnerRetirementProof::IntegrityRebind { root, .. }
        | RecoveryOwnerRetirementProof::IntegrityDischarge { root, .. } => *root,
    }
}

fn validate_retirement_owner_state(
    plan: &RecoveryRetirementPlan,
    retired: &BTreeSet<FenceOccurrenceId>,
) -> Result<(), RecoveryError> {
    let invalid = || {
        Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ))
    };
    let mut clean_regions = Vec::new();
    let mut integrity_extents = Vec::new();
    for root in &plan.predecessor.roots {
        if !retired.contains(&root.occurrence) {
            continue;
        }
        let transition_count = plan
            .rebinds
            .iter()
            .filter(|rebind| rebind.root == root.id)
            .count()
            + plan
                .discharges
                .iter()
                .filter(|discharge| discharge.root == root.id)
                .count();
        let proofs = plan
            .owner_proofs
            .iter()
            .filter(|proof| owner_proof_root(proof) == root.id)
            .collect::<Vec<_>>();
        if transition_count != proofs.len() {
            return invalid();
        }
        if transition_count == 0 {
            continue;
        }
        if transition_count != 1 {
            return invalid();
        }
        let Some(proof) = proofs.first() else {
            return invalid();
        };
        match (proof, &root.fact) {
            (
                RecoveryOwnerRetirementProof::CleanRegionRebind {
                    root: proof_root,
                    region,
                    ..
                }
                | RecoveryOwnerRetirementProof::CleanRegionDischarge {
                    root: proof_root,
                    region,
                },
                RecoveryRootFact::CleanRegion {
                    region: root_region,
                    ..
                },
            ) if *proof_root == root.id && region == root_region => {
                clean_regions.push(*root_region);
            }
            (
                RecoveryOwnerRetirementProof::IntegrityRebind {
                    root: proof_root,
                    extent,
                    ..
                }
                | RecoveryOwnerRetirementProof::IntegrityDischarge {
                    root: proof_root,
                    extent,
                },
                RecoveryRootFact::ValidIntegrity {
                    extent: root_extent,
                    ..
                },
            ) if *proof_root == root.id && extent == root_extent => {
                integrity_extents.push(*root_extent);
            }
            _ => return invalid(),
        }
    }
    if plan.owner_proofs.iter().any(|proof| {
        !plan
            .predecessor
            .roots
            .iter()
            .any(|root| root.id == owner_proof_root(proof) && retired.contains(&root.occurrence))
    }) {
        return invalid();
    }

    let mut predecessor_state = plan.predecessor.clone();
    let mut successor_state = plan.successor.clone();
    predecessor_state.fences.clear();
    successor_state.fences.clear();
    predecessor_state.roots.clear();
    successor_state.roots.clear();
    predecessor_state
        .dirty_regions
        .retain(|record| !clean_regions.contains(&record.region));
    successor_state
        .dirty_regions
        .retain(|record| !clean_regions.contains(&record.region));
    predecessor_state
        .integrity_records
        .retain(|record| !integrity_extents.contains(&record.extent));
    successor_state
        .integrity_records
        .retain(|record| !integrity_extents.contains(&record.extent));
    successor_state.next_fence_occurrence_id = predecessor_state.next_fence_occurrence_id;
    if predecessor_state != successor_state {
        return invalid();
    }
    for region in &clean_regions {
        if plan
            .predecessor
            .dirty_regions
            .iter()
            .filter(|record| record.region == *region)
            .count()
            != 1
            || plan
                .successor
                .dirty_regions
                .iter()
                .filter(|record| record.region == *region)
                .count()
                != 1
        {
            return invalid();
        }
    }
    for extent in &integrity_extents {
        if plan
            .predecessor
            .integrity_records
            .iter()
            .filter(|record| record.extent == *extent)
            .count()
            != 1
            || plan
                .successor
                .integrity_records
                .iter()
                .filter(|record| record.extent == *extent)
                .count()
                > 1
        {
            return invalid();
        }
    }

    for root in &plan.predecessor.roots {
        if !retired.contains(&root.occurrence) {
            continue;
        }
        let rebind = plan.rebinds.iter().find(|rebind| rebind.root == root.id);
        let discharge = plan
            .discharges
            .iter()
            .find(|discharge| discharge.root == root.id);
        let proof = plan
            .owner_proofs
            .iter()
            .find(|proof| owner_proof_root(proof) == root.id);
        if let Some(rebind) = rebind {
            let proof_matches = match (&root.fact, proof) {
                (
                    RecoveryRootFact::CleanRegion { region, .. },
                    Some(RecoveryOwnerRetirementProof::CleanRegionRebind {
                        root: proof_root,
                        region: proof_region,
                        successor,
                    }),
                ) => {
                    *proof_root == root.id
                        && *proof_region == *region
                        && *successor == rebind.successor
                }
                (
                    RecoveryRootFact::ValidIntegrity { extent, .. },
                    Some(RecoveryOwnerRetirementProof::IntegrityRebind {
                        root: proof_root,
                        extent: proof_extent,
                        successor,
                    }),
                ) => {
                    *proof_root == root.id
                        && *proof_extent == *extent
                        && *successor == rebind.successor
                }
                _ => false,
            };
            if !proof_matches {
                return invalid();
            }
            let Some(successor_fence) = plan.successor.fence(rebind.successor) else {
                return invalid();
            };
            match &root.fact {
                RecoveryRootFact::CleanRegion {
                    region,
                    clean_generation,
                } => {
                    let Some(record) = plan
                        .successor
                        .dirty_regions
                        .iter()
                        .find(|record| record.region == *region)
                    else {
                        return invalid();
                    };
                    let Some(clean_fence) = record.last_clean_fence.as_ref() else {
                        return invalid();
                    };
                    if !matches!(record.state, RegionState::Clean)
                        || record.clean_generation != Some(*clean_generation)
                        || clean_fence.occurrence != rebind.successor
                        || !clean_fence.same_certificate_facts(successor_fence)
                    {
                        return invalid();
                    }
                }
                RecoveryRootFact::ValidIntegrity {
                    extent,
                    profile,
                    set_generation,
                    content_generation,
                    digest,
                } => {
                    let Some(record) = plan
                        .successor
                        .integrity_records
                        .iter()
                        .find(|record| record.extent == *extent)
                    else {
                        return invalid();
                    };
                    let IntegrityState::Valid {
                        binding,
                        content_generation: record_generation,
                        durable_fence,
                        fence_occurrence,
                        digest: record_digest,
                        ..
                    } = &record.state
                    else {
                        return invalid();
                    };
                    if binding.extent.id != *extent
                        || binding.profile != *profile
                        || binding.set_generation != *set_generation
                        || binding.topology_epoch != root.topology_epoch
                        || *record_generation != *content_generation
                        || digest.as_slice() != record_digest.as_slice()
                        || *fence_occurrence != rebind.successor
                        || !successor_fence.contains_store_fence(*durable_fence)
                        || !successor_fence.covers_integrity_extent(*extent, *content_generation)
                    {
                        return invalid();
                    }
                }
                RecoveryRootFact::WritableSession { .. }
                | RecoveryRootFact::CodedCapture { .. }
                | RecoveryRootFact::CodedCleanClosure { .. }
                | RecoveryRootFact::LegacyUnreconciled { .. } => return invalid(),
            }
        } else if let Some(discharge) = discharge {
            let proof_matches = match (&root.fact, proof) {
                (
                    RecoveryRootFact::CleanRegion { region, .. },
                    Some(RecoveryOwnerRetirementProof::CleanRegionDischarge {
                        root: proof_root,
                        region: proof_region,
                    }),
                ) => *proof_root == root.id && *proof_region == *region,
                (
                    RecoveryRootFact::ValidIntegrity { extent, .. },
                    Some(RecoveryOwnerRetirementProof::IntegrityDischarge {
                        root: proof_root,
                        extent: proof_extent,
                    }),
                ) => *proof_root == root.id && *proof_extent == *extent,
                _ => false,
            };
            if !proof_matches || discharge.occurrence != root.occurrence {
                return invalid();
            }
            match &root.fact {
                RecoveryRootFact::CleanRegion { region, .. } => {
                    let Some(record) = plan
                        .successor
                        .dirty_regions
                        .iter()
                        .find(|record| record.region == *region)
                    else {
                        return invalid();
                    };
                    if !matches!(
                        record.state,
                        RegionState::Dirty { .. } | RegionState::Indeterminate
                    ) {
                        return invalid();
                    }
                }
                RecoveryRootFact::ValidIntegrity { extent, .. } => {
                    if let Some(record) = plan
                        .successor
                        .integrity_records
                        .iter()
                        .find(|record| record.extent == *extent)
                        && !matches!(
                            record.state,
                            IntegrityState::Absent | IntegrityState::Stale { .. }
                        )
                    {
                        return invalid();
                    }
                }
                RecoveryRootFact::WritableSession { .. }
                | RecoveryRootFact::CodedCapture { .. }
                | RecoveryRootFact::CodedCleanClosure { .. }
                | RecoveryRootFact::LegacyUnreconciled { .. } => return invalid(),
            }
        }
    }
    Ok(())
}
/// dwv:req req.recovery-state-semantics.fence-retirement-preserves-an-exact-durable-predecessor
fn apply_retirement(
    snapshot: &mut RecoverySnapshot,
    plan: RecoveryRetirementPlan,
    expected_topology_epoch: TopologyEpoch,
) -> Result<(), RecoveryError> {
    if plan.predecessor != *snapshot {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ));
    }
    if plan.predecessor.topology_epoch != expected_topology_epoch
        || plan.successor.topology_epoch != expected_topology_epoch
    {
        return Err(RecoveryError::TopologyMismatch {
            expected: expected_topology_epoch,
            actual: plan.successor.topology_epoch,
        });
    }
    if plan.successor.generation != plan.predecessor.generation
        || plan.successor.next_fence_occurrence_id < plan.predecessor.next_fence_occurrence_id
        || plan.successor.next_root_id != plan.predecessor.next_root_id
    {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ));
    }
    validate_export_limits(&plan.successor, RecoveryExportLimits::default())?;
    if plan
        .retired_occurrences
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceReused,
        ));
    }
    let predecessor_ids = plan
        .predecessor
        .fences
        .iter()
        .map(FenceCertificate::occurrence_id)
        .collect::<BTreeSet<_>>();
    let successor_ids = plan
        .successor
        .fences
        .iter()
        .map(FenceCertificate::occurrence_id)
        .collect::<BTreeSet<_>>();
    let retired = plan
        .retired_occurrences
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if successor_ids
        .difference(&predecessor_ids)
        .any(|occurrence| {
            !plan
                .rebinds
                .iter()
                .any(|rebind| rebind.successor == *occurrence)
        })
        || plan.successor.roots.iter().any(|root| {
            !plan
                .predecessor
                .roots
                .iter()
                .any(|candidate| candidate.id == root.id)
        })
        || plan.predecessor.fences.iter().any(|fence| {
            plan.successor
                .fence(fence.occurrence)
                .is_some_and(|successor| !successor.same_certificate_facts(fence))
        })
    {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ));
    }
    if plan.retired_occurrences.iter().any(|occurrence| {
        !predecessor_ids.contains(occurrence) || successor_ids.contains(occurrence)
    }) {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ));
    }
    if predecessor_ids
        .difference(&successor_ids)
        .any(|occurrence| !retired.contains(occurrence))
    {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ));
    }
    if plan.predecessor.legacy_unreconciled.iter().any(|root| {
        root.candidates
            .iter()
            .any(|candidate| retired.contains(candidate))
    }) {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ));
    }
    if plan.successor.legacy_unreconciled != plan.predecessor.legacy_unreconciled {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ));
    }
    validate_retirement_owner_state(&plan, &retired)?;
    for root in &plan.predecessor.roots {
        if retired.contains(&root.occurrence) && root.kind() == RecoveryRootKind::WritableSession {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch,
            ));
        }
        let successor_root = plan
            .successor
            .roots
            .iter()
            .find(|candidate| candidate.id == root.id);
        if !retired.contains(&root.occurrence) {
            let Some(successor_root) = successor_root else {
                return Err(RecoveryError::InvalidTransition(
                    TransitionError::FenceOccurrenceBindingMismatch,
                ));
            };
            if successor_root != root {
                return Err(RecoveryError::InvalidTransition(
                    TransitionError::FenceOccurrenceBindingMismatch,
                ));
            }
            continue;
        }
        let rebind = plan
            .rebinds
            .iter()
            .filter(|rebind| rebind.root == root.id)
            .collect::<Vec<_>>();
        let discharge = plan
            .discharges
            .iter()
            .filter(|discharge| discharge.root == root.id)
            .collect::<Vec<_>>();
        if rebind.len() + discharge.len() != 1 {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch,
            ));
        }
        if let Some(rebind) = rebind.first() {
            let Some(successor_root) = successor_root else {
                return Err(RecoveryError::InvalidTransition(
                    TransitionError::FenceOccurrenceBindingMismatch,
                ));
            };
            if rebind.predecessor != root.occurrence
                || rebind.successor <= root.occurrence
                || successor_root.id != root.id
                || successor_root.topology_epoch != root.topology_epoch
                || successor_root.generation > plan.successor.generation
                || successor_root.occurrence != rebind.successor
                || successor_root.fact != root.fact
                || !successor_covers_root(&successor_root.certificate, root)
            {
                return Err(RecoveryError::InvalidTransition(
                    TransitionError::FenceOccurrenceBindingMismatch,
                ));
            }
        } else if let Some(discharge) = discharge.first()
            && (discharge.occurrence != root.occurrence || successor_root.is_some())
        {
            return Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch,
            ));
        }
    }
    if plan.rebinds.iter().any(|rebind| {
        !plan
            .predecessor
            .roots
            .iter()
            .any(|root| root.id == rebind.root && retired.contains(&root.occurrence))
    }) || plan.discharges.iter().any(|discharge| {
        !plan
            .predecessor
            .roots
            .iter()
            .any(|root| root.id == discharge.root && retired.contains(&root.occurrence))
    }) {
        return Err(RecoveryError::InvalidTransition(
            TransitionError::FenceOccurrenceBindingMismatch,
        ));
    }
    if plan.successor.legacy_unreconciled.is_empty() {
        validate_snapshot_fences(&plan.successor)?;
    } else {
        validate_migrated_snapshot_fences(&plan.successor)?;
    }
    validate_export_limits(&plan.successor, RecoveryExportLimits::default())?;
    *snapshot = plan.successor;
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryResetBoundary {
    Process,
    VirtualMachine,
    PowerLoss,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryFaultPoint {
    BeforeWriteRecoveryRecordCommit,
    AfterDirtyWriteRecoveryRecordBeforeDataParityWrite,
    AfterDataParityWriteBeforeFence,
    AfterPersistenceEvidenceBeforeRecoveryClean,
    MissingState,
    CorruptState,
    CorruptJournal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverySimulationCase {
    pub case_id: &'static str,
    pub reset: RecoveryResetBoundary,
    pub fault: RecoveryFaultPoint,
    pub observation: RecoveryCommitObservation,
    pub expected_health: RecoveryStoreHealth,
    pub expected_disposition: RecoveryDisposition,
    pub permits_data_parity_write: bool,
}

pub fn recovery_simulation_cases() -> Vec<RecoverySimulationCase> {
    vec![
        RecoverySimulationCase {
            case_id: "write-recovery-record-commit-rejected-before-data-parity-write",
            reset: RecoveryResetBoundary::Process,
            fault: RecoveryFaultPoint::BeforeWriteRecoveryRecordCommit,
            observation: RecoveryCommitObservation::Rejected,
            expected_health: RecoveryStoreHealth::Healthy,
            expected_disposition: RecoveryDisposition::ReconcileReadOnly,
            permits_data_parity_write: false,
        },
        RecoverySimulationCase {
            case_id: "write-recovery-record-ack-lost-before-data-parity-write",
            reset: RecoveryResetBoundary::PowerLoss,
            fault: RecoveryFaultPoint::BeforeWriteRecoveryRecordCommit,
            observation: RecoveryCommitObservation::Lost,
            expected_health: RecoveryStoreHealth::Stale,
            expected_disposition: RecoveryDisposition::ReconcileReadOnly,
            permits_data_parity_write: false,
        },
        RecoverySimulationCase {
            case_id: "data-parity-write-before-fence",
            reset: RecoveryResetBoundary::PowerLoss,
            fault: RecoveryFaultPoint::AfterDataParityWriteBeforeFence,
            observation: RecoveryCommitObservation::Lost,
            expected_health: RecoveryStoreHealth::Stale,
            expected_disposition: RecoveryDisposition::ReconcileReadOnly,
            permits_data_parity_write: false,
        },
        RecoverySimulationCase {
            case_id: "persistence-evidence-before-recovery-clean",
            reset: RecoveryResetBoundary::Process,
            fault: RecoveryFaultPoint::AfterPersistenceEvidenceBeforeRecoveryClean,
            observation: RecoveryCommitObservation::Lost,
            expected_health: RecoveryStoreHealth::Stale,
            expected_disposition: RecoveryDisposition::ReconcileReadOnly,
            permits_data_parity_write: false,
        },
        RecoverySimulationCase {
            case_id: "missing-state-after-reset",
            reset: RecoveryResetBoundary::VirtualMachine,
            fault: RecoveryFaultPoint::MissingState,
            observation: RecoveryCommitObservation::Corrupt,
            expected_health: RecoveryStoreHealth::Missing,
            expected_disposition: RecoveryDisposition::RebuildFromData,
            permits_data_parity_write: false,
        },
        RecoverySimulationCase {
            case_id: "corrupt-main-state-after-reset",
            reset: RecoveryResetBoundary::VirtualMachine,
            fault: RecoveryFaultPoint::CorruptState,
            observation: RecoveryCommitObservation::Corrupt,
            expected_health: RecoveryStoreHealth::Corrupt,
            expected_disposition: RecoveryDisposition::RebuildFromData,
            permits_data_parity_write: false,
        },
        RecoverySimulationCase {
            case_id: "corrupt-journal-after-power-loss",
            reset: RecoveryResetBoundary::PowerLoss,
            fault: RecoveryFaultPoint::CorruptJournal,
            observation: RecoveryCommitObservation::Corrupt,
            expected_health: RecoveryStoreHealth::Corrupt,
            expected_disposition: RecoveryDisposition::RebuildFromData,
            permits_data_parity_write: false,
        },
        RecoverySimulationCase {
            case_id: "durable-write-recovery-record-after-process-reset",
            reset: RecoveryResetBoundary::Process,
            fault: RecoveryFaultPoint::AfterDirtyWriteRecoveryRecordBeforeDataParityWrite,
            observation: RecoveryCommitObservation::Durable,
            expected_health: RecoveryStoreHealth::Healthy,
            expected_disposition: RecoveryDisposition::Proceed,
            permits_data_parity_write: true,
        },
    ]
}

#[cfg(kani)]
mod kani_verification {
    use super::*;
    use dwv_core::ByteRange;

    #[kani::proof]
    #[kani::unwind(5)]
    fn dirty_region_mapping_covers_every_intersection_once() {
        const REGION_BYTES: u64 = 4096;
        let member: u8 = kani::any();
        let start: u8 = kani::any();
        let length: u8 = kani::any();
        kani::assume(member < 4);
        kani::assume(start < 4);
        kani::assume(length > 0 && u16::from(start) + u16::from(length) <= 4);

        let regions = dirty_regions_for_range(
            u32::from(member),
            ByteRange::new(
                u64::from(start) * REGION_BYTES,
                u64::from(length) * REGION_BYTES,
            )
            .unwrap(),
            REGION_BYTES,
        )
        .unwrap();
        assert_eq!(regions.len(), usize::from(length));
        for (index, region) in regions.iter().enumerate() {
            assert_eq!(
                *region,
                RegionId(
                    (u64::from(member) << 32) | (u64::from(start) + u64::try_from(index).unwrap())
                )
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_core::ByteRange;

    fn store() -> MemoryRecoveryStore {
        MemoryRecoveryStore::new(TopologyEpoch(1))
    }

    #[test]
    fn dirty_region_mapping_is_complete_unique_and_checked() {
        assert_eq!(
            dirty_regions_for_range(7, ByteRange::new(4095, 4098).unwrap(), 4096).unwrap(),
            vec![
                RegionId(7_u64 << 32),
                RegionId((7_u64 << 32) | 1),
                RegionId((7_u64 << 32) | 2),
            ]
        );
        assert!(matches!(
            dirty_regions_for_range(0, ByteRange::empty(), 4096),
            Err(RegionMappingError::EmptyRange)
        ));
        assert!(matches!(
            dirty_regions_for_range(0, ByteRange::new(u64::MAX - 1, 1).unwrap(), 1),
            Err(RegionMappingError::RegionIdOverflow)
        ));
        assert_ne!(
            dirty_regions_for_range(1, ByteRange::new(0, 1).unwrap(), 4096).unwrap(),
            dirty_regions_for_range(2, ByteRange::new(0, 1).unwrap(), 4096).unwrap()
        );
    }

    fn fence(region: RegionId, generation: RecoveryGeneration) -> FenceCertificate {
        FenceCertificate::new(
            TopologyEpoch(1),
            FenceDomain(1),
            vec![StoreFenceRef {
                fence_id: dwv_store::FenceId(1),
                store_id: StoreId(2),
                store_incarnation: dwv_store::StoreIncarnationId(0),
                topology_epoch: TopologyEpoch(1),
                through: dwv_store::StoreWriteWatermark(10),
                capability_evidence_id: dwv_store::CapabilityEvidenceId(3),
            }],
            vec![(region, generation)],
        )
        .with_integrity_extent(IntegrityExtentId(9), generation)
    }
    fn store_fence(store_id: u64, fence_id: u64, through: u64) -> StoreFenceRef {
        StoreFenceRef {
            fence_id: dwv_store::FenceId(fence_id),
            store_id: StoreId(store_id),
            store_incarnation: dwv_store::StoreIncarnationId(0),
            topology_epoch: TopologyEpoch(1),
            through: dwv_store::StoreWriteWatermark(through),
            capability_evidence_id: dwv_store::CapabilityEvidenceId(3),
        }
    }

    #[test]
    fn fence_occurrences_are_monotonic_distinct_and_not_reused() {
        let mut recovery = store();
        let certificate = fence(RegionId(1), RecoveryGeneration(1));

        for generation in [RecoveryGeneration(0), RecoveryGeneration(1)] {
            let mut transaction = recovery.begin_protocol_txn(generation, TopologyEpoch(1));
            transaction.push(RecoveryMutation::RecordDataParityFence {
                fence: certificate.clone(),
            });
            recovery.commit_durable(transaction).unwrap();
        }

        assert_eq!(
            recovery
                .snapshot()
                .fences
                .iter()
                .map(FenceCertificate::occurrence_id)
                .collect::<Vec<_>>(),
            vec![FenceOccurrenceId(1), FenceOccurrenceId(2)]
        );
        assert_eq!(
            recovery.snapshot().next_fence_occurrence_id,
            FenceOccurrenceId(3)
        );

        let prior = recovery.snapshot().clone();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: certificate.with_occurrence(FenceOccurrenceId(1)),
        });
        assert!(matches!(
            recovery.commit_durable(transaction),
            Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceReused
            ))
        ));
        assert_eq!(recovery.snapshot(), &prior);
    }

    #[test]
    fn new_fences_require_canonical_order_and_unique_bindings() {
        let cases = [
            FenceCertificate::new(
                TopologyEpoch(1),
                FenceDomain(1),
                vec![store_fence(2, 1, 10), store_fence(1, 1, 10)],
                Vec::new(),
            ),
            FenceCertificate::new(
                TopologyEpoch(1),
                FenceDomain(1),
                vec![store_fence(1, 1, 10), store_fence(1, 1, 11)],
                Vec::new(),
            ),
            FenceCertificate::new(
                TopologyEpoch(1),
                FenceDomain(1),
                vec![store_fence(1, 1, 10), store_fence(1, 2, 10)],
                Vec::new(),
            ),
            FenceCertificate::new(
                TopologyEpoch(1),
                FenceDomain(1),
                vec![store_fence(1, 1, 10)],
                vec![
                    (RegionId(2), RecoveryGeneration(1)),
                    (RegionId(1), RecoveryGeneration(1)),
                ],
            ),
            FenceCertificate::new(
                TopologyEpoch(1),
                FenceDomain(1),
                vec![store_fence(1, 1, 10)],
                vec![
                    (RegionId(1), RecoveryGeneration(1)),
                    (RegionId(1), RecoveryGeneration(2)),
                ],
            )
            .with_integrity_extent(IntegrityExtentId(2), RecoveryGeneration(1))
            .with_integrity_extent(IntegrityExtentId(1), RecoveryGeneration(1)),
        ];

        for certificate in cases {
            let mut recovery = store();
            let mut transaction =
                recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
            transaction.push(RecoveryMutation::RecordDataParityFence { fence: certificate });
            assert!(matches!(
                recovery.commit_durable(transaction),
                Err(RecoveryError::InvalidTransition(
                    TransitionError::FenceNonCanonical
                        | TransitionError::FenceDuplicateStore
                        | TransitionError::FenceDuplicateRegion
                        | TransitionError::FenceDuplicateIntegrity
                ))
            ));
            assert!(recovery.snapshot().fences.is_empty());
        }
    }

    #[test]
    fn restored_fences_require_persisted_occurrence_identity() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(1), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();
        let mut manifest = recovery.export_manifest(RecoveryGeneration(1)).unwrap();
        manifest.snapshot.fences[0].occurrence = FenceOccurrenceId::UNASSIGNED;
        assert!(matches!(
            MemoryRecoveryStore::from_manifest(manifest),
            Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceMissing
            ))
        ));
    }
    #[test]
    fn legacy_migration_assigns_distinct_occurrences_without_coalescing() {
        let mut recovery = store();
        let certificate = fence(RegionId(1), RecoveryGeneration(1));
        for generation in [RecoveryGeneration(0), RecoveryGeneration(1)] {
            let mut transaction = recovery.begin_protocol_txn(generation, TopologyEpoch(1));
            transaction.push(RecoveryMutation::RecordDataParityFence {
                fence: certificate.clone(),
            });
            recovery.commit_durable(transaction).unwrap();
        }
        let mut legacy = recovery.export_manifest(RecoveryGeneration(2)).unwrap();
        legacy.schema = RecoverySchemaVersion(6);
        for fence in &mut legacy.snapshot.fences {
            fence.occurrence = FenceOccurrenceId::UNASSIGNED;
        }
        legacy.snapshot.next_fence_occurrence_id = FenceOccurrenceId::UNASSIGNED;

        assert!(matches!(
            MemoryRecoveryStore::from_manifest(legacy.clone()),
            Err(RecoveryError::UnsupportedSchemaMigration { .. })
        ));
        let migrated = MemoryRecoveryStore::migrate_legacy_manifest(legacy).unwrap();
        assert_eq!(migrated.schema, CURRENT_RECOVERY_SCHEMA);
        assert_eq!(
            migrated
                .snapshot
                .fences
                .iter()
                .map(FenceCertificate::occurrence_id)
                .collect::<Vec<_>>(),
            vec![FenceOccurrenceId(1), FenceOccurrenceId(2)]
        );
        assert_eq!(
            migrated.snapshot.next_fence_occurrence_id,
            FenceOccurrenceId(3)
        );
        MemoryRecoveryStore::from_manifest(migrated).unwrap();
    }

    #[test]
    fn legacy_migration_preserves_noncanonical_and_ambiguous_embedded_evidence() {
        let legacy_fence = FenceCertificate::new(
            TopologyEpoch(1),
            FenceDomain(1),
            vec![store_fence(2, 1, 10), store_fence(1, 1, 10)],
            vec![(RegionId(1), RecoveryGeneration::ZERO)],
        );
        let embedded_only = FenceCertificate::new(
            TopologyEpoch(1),
            FenceDomain(1),
            vec![store_fence(9, 1, 10)],
            vec![(RegionId(9), RecoveryGeneration::ZERO)],
        );
        let mut legacy = store().export_manifest(RecoveryGeneration::ZERO).unwrap();
        legacy.schema = RecoverySchemaVersion(6);
        legacy.snapshot.fences = vec![legacy_fence.clone(), legacy_fence.clone()];
        legacy.snapshot.next_fence_occurrence_id = FenceOccurrenceId::UNASSIGNED;
        legacy.snapshot.writable_session = Some(WritableSession {
            session_id: SessionId(7),
            topology_epoch: TopologyEpoch(1),
            dirty_envelope_generation: 0,
            closed: true,
            close_generation: Some(RecoveryGeneration(1)),
            global_fence: Some(legacy_fence),
        });
        legacy.snapshot.dirty_regions = vec![DirtyRegionRecord {
            region: RegionId(9),
            state: RegionState::Clean,
            last_clean_fence: Some(embedded_only),
            clean_generation: Some(RecoveryGeneration::ZERO),
        }];

        let migrated = MemoryRecoveryStore::migrate_legacy_manifest(legacy).unwrap();
        assert_eq!(migrated.snapshot.fences.len(), 3);
        assert_eq!(
            migrated
                .snapshot
                .fences
                .iter()
                .map(FenceCertificate::occurrence_id)
                .collect::<Vec<_>>(),
            vec![
                FenceOccurrenceId(1),
                FenceOccurrenceId(2),
                FenceOccurrenceId(3)
            ]
        );
        assert_eq!(
            migrated
                .snapshot
                .writable_session
                .as_ref()
                .and_then(|session| session.global_fence.as_ref())
                .map(FenceCertificate::occurrence_id),
            Some(FenceOccurrenceId::UNASSIGNED)
        );
        assert_eq!(
            migrated
                .snapshot
                .dirty_regions
                .first()
                .and_then(|region| region.last_clean_fence.as_ref())
                .map(FenceCertificate::occurrence_id),
            Some(FenceOccurrenceId(3))
        );
        assert!(
            migrated
                .snapshot
                .legacy_unreconciled
                .iter()
                .any(|root| root.candidates == vec![FenceOccurrenceId(1), FenceOccurrenceId(2)])
        );
        let reopened = MemoryRecoveryStore::from_manifest(migrated).unwrap();
        assert_eq!(reopened.health(), RecoveryStoreHealth::Stale);
    }

    #[test]
    fn clean_root_binds_the_latest_exact_occurrence() {
        let mut recovery = store();
        let certificate = fence(RegionId(6), RecoveryGeneration(1));
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: certificate.clone(),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(6),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence { fence: certificate });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(3), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(6),
            through_generation: RecoveryGeneration(1),
            fence_occurrence: FenceOccurrenceId(2),
        });
        recovery.commit_durable(transaction).unwrap();

        assert_eq!(
            recovery.snapshot().dirty_regions[0]
                .last_clean_fence
                .as_ref()
                .map(FenceCertificate::occurrence_id),
            Some(FenceOccurrenceId(2))
        );
        assert_eq!(
            recovery
                .snapshot()
                .roots
                .iter()
                .find(|root| root.kind() == RecoveryRootKind::CleanRegion)
                .map(|root| root.occurrence),
            Some(FenceOccurrenceId(2))
        );
    }

    #[test]
    fn clean_transition_uses_exact_fence_occurrence() {
        let mut recovery = store();
        let mut dirty = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        dirty.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(dirty).unwrap();

        let mut first = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        first.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(1), RecoveryGeneration(1)),
        });
        recovery.commit_durable(first).unwrap();
        let mut second = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        second.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(2), RecoveryGeneration(1)),
        });
        recovery.commit_durable(second).unwrap();

        let before = recovery.snapshot().clone();
        let mut wrong = recovery.begin_protocol_txn(before.generation, TopologyEpoch(1));
        wrong.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(1),
            through_generation: RecoveryGeneration(1),
            fence_occurrence: FenceOccurrenceId(2),
        });
        assert!(matches!(
            recovery.commit_durable(wrong),
            Err(RecoveryError::InvalidTransition(
                TransitionError::FenceCoverageMissing
            ))
        ));
        assert_eq!(recovery.snapshot(), &before);

        let mut right = recovery.begin_protocol_txn(before.generation, TopologyEpoch(1));
        right.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(1),
            through_generation: RecoveryGeneration(1),
            fence_occurrence: FenceOccurrenceId(1),
        });
        recovery.commit_durable(right).unwrap();
        assert_eq!(
            recovery
                .snapshot()
                .roots
                .iter()
                .find(|root| root.kind() == RecoveryRootKind::CleanRegion)
                .map(|root| root.occurrence),
            Some(FenceOccurrenceId(1))
        );
    }

    #[test]
    fn owner_transitions_create_and_discharge_qualified_roots() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(1), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(1),
            fence_occurrence: FenceOccurrenceId(1),
            through_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();
        assert!(
            recovery
                .snapshot()
                .roots
                .iter()
                .any(|root| root.kind() == RecoveryRootKind::CleanRegion)
        );
        let mut invalid_manifest = recovery.export_manifest(RecoveryGeneration(3)).unwrap();
        invalid_manifest.snapshot.roots[0].fact = RecoveryRootFact::CleanRegion {
            region: RegionId(99),
            clean_generation: RecoveryGeneration(1),
        };
        assert!(matches!(
            MemoryRecoveryStore::from_manifest(invalid_manifest),
            Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch
            ))
        ));
        let mut manifest = recovery.export_manifest(RecoveryGeneration(3)).unwrap();
        manifest.snapshot.roots.clear();
        assert!(matches!(
            MemoryRecoveryStore::from_manifest(manifest),
            Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch
            ))
        ));

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(3), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(3),
        });
        recovery.commit_durable(transaction).unwrap();
        assert!(
            recovery
                .snapshot()
                .roots
                .iter()
                .all(|root| root.kind() != RecoveryRootKind::CleanRegion)
        );

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(4), TopologyEpoch(1));
        transaction.push(RecoveryMutation::BeginWritableSession {
            session_id: SessionId(7),
            topology_epoch: TopologyEpoch(1),
            dirty_envelope_generation: 4,
        });
        recovery.commit_durable(transaction).unwrap();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(5), TopologyEpoch(1));
        transaction.push(RecoveryMutation::CloseWritableSession {
            session_id: SessionId(7),
            global_fence: fence(RegionId(2), RecoveryGeneration(5)),
        });
        recovery.commit_durable(transaction).unwrap();
        let session_root = recovery
            .snapshot()
            .roots
            .iter()
            .find(|root| root.kind() == RecoveryRootKind::WritableSession)
            .cloned()
            .unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(6), TopologyEpoch(1));
        transaction.push(RecoveryMutation::BeginWritableSession {
            session_id: SessionId(8),
            topology_epoch: TopologyEpoch(1),
            dirty_envelope_generation: 6,
        });
        recovery.commit_durable(transaction).unwrap();
        assert!(recovery.snapshot().roots.contains(&session_root));
    }

    #[test]
    fn integrity_install_uses_supplied_occurrence() {
        let mut recovery = store();
        let certificate = fence(RegionId(5), RecoveryGeneration(1));
        for generation in [RecoveryGeneration(0), RecoveryGeneration(1)] {
            let mut transaction = recovery.begin_protocol_txn(generation, TopologyEpoch(1));
            transaction.push(RecoveryMutation::RecordDataParityFence {
                fence: certificate.clone(),
            });
            recovery.commit_durable(transaction).unwrap();
        }
        let extent = ChecksumExtent {
            id: IntegrityExtentId(9),
            target: ChecksumTarget::data(dwv_core::SlotId([1; 16])),
            range: dwv_core::ByteRange::new(0, 1).unwrap(),
        };
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::InstallIntegrityDigest {
            record: IntegrityRecord {
                extent: IntegrityExtentId(9),
                state: IntegrityState::Valid {
                    binding: ChecksumEvidenceBinding {
                        extent,
                        profile: BLAKE3_256_PROFILE.id,
                        set_generation: ChecksumSetGeneration::INITIAL,
                        topology_epoch: TopologyEpoch(1),
                    },
                    content_generation: RecoveryGeneration(1),
                    durable_fence: recovery.snapshot().fences[0].stores[0],
                    fence_occurrence: FenceOccurrenceId(2),
                    digest: vec![1; 32],
                    verified_at: RecoveryGeneration(1),
                },
            },
        });
        recovery.commit_durable(transaction).unwrap();
        assert_eq!(
            recovery
                .snapshot()
                .roots
                .iter()
                .find(|root| root.kind() == RecoveryRootKind::ValidIntegrity)
                .map(|root| root.occurrence),
            Some(FenceOccurrenceId(2))
        );
    }

    #[test]
    fn generic_retirement_cannot_discharge_session_root() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::BeginWritableSession {
            session_id: SessionId(11),
            topology_epoch: TopologyEpoch(1),
            dirty_envelope_generation: 0,
        });
        recovery.commit_durable(transaction).unwrap();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        transaction.push(RecoveryMutation::CloseWritableSession {
            session_id: SessionId(11),
            global_fence: fence(RegionId(11), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();

        let predecessor = recovery.snapshot().clone();
        let root = predecessor
            .roots
            .iter()
            .find(|root| root.kind() == RecoveryRootKind::WritableSession)
            .cloned()
            .unwrap();
        let mut successor = predecessor.clone();
        successor
            .fences
            .retain(|fence| fence.occurrence != root.occurrence);
        successor.roots.retain(|candidate| candidate.id != root.id);
        let mut plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
        plan.retire(root.occurrence)
            .discharge(root.id, root.occurrence);

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RetireFences { plan });
        assert_eq!(
            recovery.commit_durable(transaction),
            Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch
            ))
        );
        assert_eq!(*recovery.snapshot(), predecessor);
    }

    #[test]
    fn valid_integrity_root_tracks_exact_fence_occurrence() {
        let mut recovery = store();
        let certificate = fence(RegionId(4), RecoveryGeneration(1));
        let store_fence = certificate.stores[0];
        let extent = ChecksumExtent {
            id: IntegrityExtentId(9),
            target: ChecksumTarget::data(dwv_core::SlotId([1; 16])),
            range: dwv_core::ByteRange::new(0, 1).unwrap(),
        };
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence { fence: certificate });
        transaction.push(RecoveryMutation::InstallIntegrityDigest {
            record: IntegrityRecord {
                extent: IntegrityExtentId(9),
                state: IntegrityState::Valid {
                    binding: ChecksumEvidenceBinding {
                        extent,
                        profile: BLAKE3_256_PROFILE.id,
                        set_generation: ChecksumSetGeneration::INITIAL,
                        topology_epoch: TopologyEpoch(1),
                    },
                    content_generation: RecoveryGeneration(1),
                    durable_fence: store_fence,
                    fence_occurrence: FenceOccurrenceId(1),
                    digest: vec![0; 32],
                    verified_at: RecoveryGeneration(1),
                },
            },
        });
        recovery.commit_durable(transaction).unwrap();
        assert!(
            recovery
                .snapshot()
                .roots
                .iter()
                .any(|root| root.kind() == RecoveryRootKind::ValidIntegrity)
        );

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkIntegrityStale {
            extent: IntegrityExtentId(9),
            stale_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();
        assert!(
            recovery
                .snapshot()
                .roots
                .iter()
                .all(|root| root.kind() != RecoveryRootKind::ValidIntegrity)
        );
    }

    #[test]
    fn retirement_rebinds_roots_and_removes_only_the_exact_predecessor() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(1), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(1),
            fence_occurrence: FenceOccurrenceId(1),
            through_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(3), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(1), RecoveryGeneration(2)),
        });
        recovery.commit_durable(transaction).unwrap();
        let predecessor = recovery.snapshot().clone();
        let root = predecessor.roots.first().cloned().unwrap();
        let successor_fence = predecessor.fences.last().cloned().unwrap();
        let mut successor = predecessor.clone();
        successor
            .fences
            .retain(|fence| fence.occurrence != FenceOccurrenceId(1));
        successor.roots[0].occurrence = successor_fence.occurrence;
        successor.dirty_regions[0].last_clean_fence = Some(successor_fence.clone());
        successor.roots[0].certificate = successor_fence.clone();
        successor.roots[0].generation = successor.generation;

        let mut unauthorized_successor = predecessor.clone();
        unauthorized_successor
            .fences
            .retain(|fence| fence.occurrence != root.occurrence);
        unauthorized_successor.dirty_regions[0].state = RegionState::Dirty {
            dirty_since: RecoveryGeneration(2),
        };
        unauthorized_successor.roots.clear();
        let mut unauthorized_plan =
            RecoveryRetirementPlan::new(predecessor.clone(), unauthorized_successor);
        unauthorized_plan
            .retire(root.occurrence)
            .discharge(root.id, root.occurrence);
        let mut unauthorized_txn =
            recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
        unauthorized_txn.push(RecoveryMutation::RetireFences {
            plan: unauthorized_plan,
        });
        assert_eq!(
            recovery.commit_durable(unauthorized_txn),
            Err(RecoveryError::InvalidTransition(
                TransitionError::FenceOccurrenceBindingMismatch
            ))
        );
        assert_eq!(recovery.snapshot(), &predecessor);

        let mut plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
        plan.retire(FenceOccurrenceId(1))
            .rebind(root.id, root.occurrence, successor_fence.occurrence)
            .authorize_clean_region_rebind(root.id, RegionId(1), successor_fence.occurrence);
        let mut transaction = recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
        transaction.push(RecoveryMutation::RetireFences { plan });
        recovery.commit_durable(transaction).unwrap();

        assert!(
            recovery
                .snapshot()
                .fences
                .iter()
                .all(|fence| fence.occurrence != FenceOccurrenceId(1))
        );
        assert_eq!(
            recovery.snapshot().roots[0].occurrence,
            successor_fence.occurrence
        );
    }

    #[test]
    fn retirement_rebinds_multiple_roots_to_independent_successors() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(1), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(1),
            through_generation: RecoveryGeneration(1),
            fence_occurrence: FenceOccurrenceId(1),
        });
        recovery.commit_durable(transaction).unwrap();

        let store_fence = recovery.snapshot().fences[0].stores[0];
        let extent = ChecksumExtent {
            id: IntegrityExtentId(9),
            target: ChecksumTarget::data(dwv_core::SlotId([1; 16])),
            range: dwv_core::ByteRange::new(0, 1).unwrap(),
        };
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(3), TopologyEpoch(1));
        transaction.push(RecoveryMutation::InstallIntegrityDigest {
            record: IntegrityRecord {
                extent: extent.id,
                state: IntegrityState::Valid {
                    binding: ChecksumEvidenceBinding {
                        extent,
                        profile: BLAKE3_256_PROFILE.id,
                        set_generation: ChecksumSetGeneration::INITIAL,
                        topology_epoch: TopologyEpoch(1),
                    },
                    content_generation: RecoveryGeneration(1),
                    durable_fence: store_fence,
                    fence_occurrence: FenceOccurrenceId(1),
                    digest: vec![7; 32],
                    verified_at: RecoveryGeneration(1),
                },
            },
        });
        recovery.commit_durable(transaction).unwrap();

        let predecessor = recovery.snapshot().clone();
        let clean_successor = FenceCertificate::new(
            TopologyEpoch(1),
            FenceDomain(1),
            vec![store_fence],
            vec![(RegionId(1), RecoveryGeneration(2))],
        )
        .with_occurrence(FenceOccurrenceId(2));
        let integrity_successor = FenceCertificate::new(
            TopologyEpoch(1),
            FenceDomain(1),
            vec![store_fence],
            Vec::new(),
        )
        .with_integrity_extent(IntegrityExtentId(9), RecoveryGeneration(1))
        .with_occurrence(FenceOccurrenceId(3));
        let clean_root = predecessor
            .roots
            .iter()
            .find(|root| root.kind() == RecoveryRootKind::CleanRegion)
            .cloned()
            .unwrap();
        let integrity_root = predecessor
            .roots
            .iter()
            .find(|root| root.kind() == RecoveryRootKind::ValidIntegrity)
            .cloned()
            .unwrap();
        let mut successor = predecessor.clone();
        successor.fences = vec![clean_successor.clone(), integrity_successor.clone()];
        successor.next_fence_occurrence_id = FenceOccurrenceId(4);
        successor.dirty_regions[0].last_clean_fence = Some(clean_successor.clone());
        successor.integrity_records[0].state = IntegrityState::Valid {
            binding: ChecksumEvidenceBinding {
                extent,
                profile: BLAKE3_256_PROFILE.id,
                set_generation: ChecksumSetGeneration::INITIAL,
                topology_epoch: TopologyEpoch(1),
            },
            content_generation: RecoveryGeneration(1),
            durable_fence: store_fence,
            fence_occurrence: FenceOccurrenceId(3),
            digest: vec![7; 32],
            verified_at: RecoveryGeneration(1),
        };
        successor.roots = vec![
            RecoveryClaimRoot {
                occurrence: clean_successor.occurrence,
                certificate: clean_successor,
                ..clean_root
            },
            RecoveryClaimRoot {
                occurrence: integrity_successor.occurrence,
                certificate: integrity_successor,
                ..integrity_root
            },
        ];
        let mut plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
        plan.retire(FenceOccurrenceId(1))
            .rebind(clean_root.id, clean_root.occurrence, FenceOccurrenceId(2))
            .authorize_clean_region_rebind(clean_root.id, RegionId(1), FenceOccurrenceId(2))
            .rebind(
                integrity_root.id,
                integrity_root.occurrence,
                FenceOccurrenceId(3),
            )
            .authorize_integrity_rebind(
                integrity_root.id,
                IntegrityExtentId(9),
                FenceOccurrenceId(3),
            );
        let mut retirement = recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
        retirement.push(RecoveryMutation::RetireFences { plan });
        recovery.commit_durable(retirement).unwrap();

        assert!(recovery.snapshot().fence(FenceOccurrenceId(1)).is_none());
        assert_eq!(
            recovery
                .snapshot()
                .roots
                .iter()
                .find(|root| root.id == clean_root.id)
                .map(|root| root.occurrence),
            Some(FenceOccurrenceId(2))
        );
        assert_eq!(
            recovery
                .snapshot()
                .roots
                .iter()
                .find(|root| root.id == integrity_root.id)
                .map(|root| root.occurrence),
            Some(FenceOccurrenceId(3))
        );
    }
    #[test]
    fn retirement_rejects_non_exact_successors_and_preserves_predecessor() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(1), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(1),
            through_generation: RecoveryGeneration(1),
            fence_occurrence: FenceOccurrenceId(1),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(3), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(2), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();

        let predecessor = recovery.snapshot().clone();
        let root = predecessor
            .roots
            .iter()
            .find(|root| root.kind() == RecoveryRootKind::CleanRegion)
            .cloned()
            .unwrap();
        let base = predecessor.fence(root.occurrence).unwrap().clone();
        let successor_for = |candidate: FenceCertificate, stale_embedded_binding: bool| {
            let mut successor = predecessor.clone();
            successor
                .fences
                .retain(|fence| fence.occurrence != root.occurrence);
            successor.fences.push(candidate.clone());
            successor.next_fence_occurrence_id = FenceOccurrenceId(4);
            successor
                .dirty_regions
                .iter_mut()
                .find(|record| record.region == RegionId(1))
                .unwrap()
                .last_clean_fence = Some(if stale_embedded_binding {
                base.clone()
            } else {
                candidate.clone()
            });
            let successor_root = successor
                .roots
                .iter_mut()
                .find(|candidate| candidate.id == root.id)
                .unwrap();
            successor_root.occurrence = candidate.occurrence;
            successor_root.certificate = candidate;
            successor_root.generation = successor.generation;
            successor
        };

        let mut changed_incarnation = base.clone().with_occurrence(FenceOccurrenceId(3));
        changed_incarnation.stores[0].store_incarnation = dwv_store::StoreIncarnationId(1);
        let mut changed_domain = base.clone().with_occurrence(FenceOccurrenceId(3));
        changed_domain.fence_domain = FenceDomain(2);
        let mut changed_capability = base.clone().with_occurrence(FenceOccurrenceId(3));
        changed_capability.stores[0].capability_evidence_id = dwv_store::CapabilityEvidenceId(4);
        let mut changed_topology = base.clone().with_occurrence(FenceOccurrenceId(3));
        changed_topology.topology_epoch = TopologyEpoch(2);
        let partial =
            fence(RegionId(2), RecoveryGeneration(1)).with_occurrence(FenceOccurrenceId(3));
        let stale_embedded_binding = base.clone().with_occurrence(FenceOccurrenceId(3));

        for (candidate, stale_binding) in [
            (changed_incarnation, false),
            (changed_domain, false),
            (changed_capability, false),
            (changed_topology, false),
            (partial, false),
            (stale_embedded_binding, true),
        ] {
            let successor = successor_for(candidate, stale_binding);
            let mut plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
            plan.retire(root.occurrence)
                .rebind(root.id, root.occurrence, FenceOccurrenceId(3))
                .authorize_clean_region_rebind(root.id, RegionId(1), FenceOccurrenceId(3));
            let mut retirement =
                recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
            retirement.push(RecoveryMutation::RetireFences { plan });
            let result = recovery.commit_durable(retirement);
            assert!(result.is_err(), "{result:?}");
            assert_eq!(recovery.snapshot(), &predecessor);
        }
    }

    #[test]
    fn equal_fence_occurrences_retire_independently_of_root_bound_copy() {
        let mut recovery = store();
        let certificate = fence(RegionId(1), RecoveryGeneration(1));
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: certificate.clone(),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence { fence: certificate });
        recovery.commit_durable(transaction).unwrap();

        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(3), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(1),
            through_generation: RecoveryGeneration(1),
            fence_occurrence: FenceOccurrenceId(1),
        });
        recovery.commit_durable(transaction).unwrap();

        let predecessor = recovery.snapshot().clone();
        let root = predecessor.roots.first().cloned().unwrap();
        let mut successor = predecessor.clone();
        successor
            .fences
            .retain(|fence| fence.occurrence != FenceOccurrenceId(2));
        let mut plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
        plan.retire(FenceOccurrenceId(2));
        let mut retirement = recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
        retirement.push(RecoveryMutation::RetireFences { plan });
        recovery.commit_durable(retirement).unwrap();

        assert!(recovery.snapshot().fence(FenceOccurrenceId(1)).is_some());
        assert!(recovery.snapshot().fence(FenceOccurrenceId(2)).is_none());
        assert_eq!(
            recovery
                .snapshot()
                .roots
                .iter()
                .find(|candidate| candidate.id == root.id)
                .map(|candidate| candidate.occurrence),
            Some(FenceOccurrenceId(1))
        );
    }

    #[test]
    fn stale_retirement_proposal_cannot_remove_after_other_successor() {
        let mut recovery = store();
        for region in [RegionId(1), RegionId(2)] {
            let generation = recovery.snapshot().generation;
            let mut transaction = recovery.begin_protocol_txn(generation, TopologyEpoch(1));
            transaction.push(RecoveryMutation::RecordDataParityFence {
                fence: fence(region, RecoveryGeneration(1)),
            });
            recovery.commit_durable(transaction).unwrap();
        }

        let predecessor = recovery.snapshot().clone();
        let mut first_successor = predecessor.clone();
        first_successor
            .fences
            .retain(|fence| fence.occurrence != FenceOccurrenceId(1));
        let mut first_plan = RecoveryRetirementPlan::new(predecessor.clone(), first_successor);
        first_plan.retire(FenceOccurrenceId(1));

        let mut stale_successor = predecessor.clone();
        stale_successor
            .fences
            .retain(|fence| fence.occurrence != FenceOccurrenceId(2));
        let mut stale_plan = RecoveryRetirementPlan::new(predecessor.clone(), stale_successor);
        stale_plan.retire(FenceOccurrenceId(2));

        let mut first = recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
        first.push(RecoveryMutation::RetireFences { plan: first_plan });
        let mut stale = recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
        stale.push(RecoveryMutation::RetireFences { plan: stale_plan });

        recovery.commit_durable(first).unwrap();
        assert_eq!(
            recovery.commit_durable(stale),
            Err(RecoveryError::GenerationMismatch {
                expected: predecessor.generation,
                actual: predecessor.generation.checked_next().unwrap(),
            })
        );
        assert!(recovery.snapshot().fence(FenceOccurrenceId(1)).is_none());
        assert!(recovery.snapshot().fence(FenceOccurrenceId(2)).is_some());
    }

    #[test]
    fn owner_discharge_proofs_remove_clean_and_integrity_roots() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(1), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(1),
            through_generation: RecoveryGeneration(1),
            fence_occurrence: FenceOccurrenceId(1),
        });
        recovery.commit_durable(transaction).unwrap();

        let predecessor = recovery.snapshot().clone();
        let root = predecessor
            .roots
            .iter()
            .find(|root| root.kind() == RecoveryRootKind::CleanRegion)
            .cloned()
            .unwrap();
        let mut successor = predecessor.clone();
        successor.fences.clear();
        successor.roots.clear();
        let record = successor
            .dirty_regions
            .iter_mut()
            .find(|record| record.region == RegionId(1))
            .unwrap();
        record.state = RegionState::Dirty {
            dirty_since: RecoveryGeneration(3),
        };
        record.clean_generation = None;
        record.last_clean_fence = None;
        let mut plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
        plan.retire(root.occurrence)
            .discharge(root.id, root.occurrence)
            .authorize_clean_region_discharge(root.id, RegionId(1));
        let mut transaction = recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
        transaction.push(RecoveryMutation::RetireFences { plan });
        recovery.commit_durable(transaction).unwrap();
        assert!(recovery.snapshot().fences.is_empty());
        assert!(recovery.snapshot().roots.is_empty());

        let mut recovery = store();
        let certificate = fence(RegionId(2), RecoveryGeneration(1));
        let store_fence = certificate.stores[0];
        let extent = ChecksumExtent {
            id: IntegrityExtentId(9),
            target: ChecksumTarget::data(dwv_core::SlotId([1; 16])),
            range: dwv_core::ByteRange::new(0, 1).unwrap(),
        };
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence { fence: certificate });
        transaction.push(RecoveryMutation::InstallIntegrityDigest {
            record: IntegrityRecord {
                extent: extent.id,
                state: IntegrityState::Valid {
                    binding: ChecksumEvidenceBinding {
                        extent,
                        profile: BLAKE3_256_PROFILE.id,
                        set_generation: ChecksumSetGeneration::INITIAL,
                        topology_epoch: TopologyEpoch(1),
                    },
                    content_generation: RecoveryGeneration(1),
                    durable_fence: store_fence,
                    fence_occurrence: FenceOccurrenceId(1),
                    digest: vec![4; 32],
                    verified_at: RecoveryGeneration(1),
                },
            },
        });
        recovery.commit_durable(transaction).unwrap();

        let predecessor = recovery.snapshot().clone();
        let root = predecessor
            .roots
            .iter()
            .find(|root| root.kind() == RecoveryRootKind::ValidIntegrity)
            .cloned()
            .unwrap();
        let mut successor = predecessor.clone();
        successor.fences.clear();
        successor.roots.clear();
        successor.integrity_records[0].state = IntegrityState::Stale {
            stale_generation: RecoveryGeneration(2),
        };
        let mut plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
        plan.retire(root.occurrence)
            .discharge(root.id, root.occurrence)
            .authorize_integrity_discharge(root.id, IntegrityExtentId(9));
        let mut transaction = recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
        transaction.push(RecoveryMutation::RetireFences { plan });
        recovery.commit_durable(transaction).unwrap();
        assert!(recovery.snapshot().fences.is_empty());
        assert!(recovery.snapshot().roots.is_empty());
    }

    #[test]
    fn clean_churn_keeps_current_roots_and_fences_bounded() {
        let mut recovery = store();
        for _ in 0..64 {
            let dirty_generation = recovery.snapshot().generation.checked_next().unwrap();
            let mut dirty =
                recovery.begin_protocol_txn(recovery.snapshot().generation, TopologyEpoch(1));
            dirty.push(RecoveryMutation::MarkRegionDirty {
                region: RegionId(1),
                mutation_generation: dirty_generation,
            });
            recovery.commit_durable(dirty).unwrap();

            let mut append =
                recovery.begin_protocol_txn(recovery.snapshot().generation, TopologyEpoch(1));
            append.push(RecoveryMutation::RecordDataParityFence {
                fence: fence(RegionId(1), dirty_generation),
            });
            recovery.commit_durable(append).unwrap();
            let occurrence = recovery.snapshot().fences.last().unwrap().occurrence;

            let mut clean =
                recovery.begin_protocol_txn(recovery.snapshot().generation, TopologyEpoch(1));
            clean.push(RecoveryMutation::MarkRegionClean {
                region: RegionId(1),
                through_generation: dirty_generation,
                fence_occurrence: occurrence,
            });
            recovery.commit_durable(clean).unwrap();

            if recovery.snapshot().fences.len() > 1 {
                let predecessor = recovery.snapshot().clone();
                let retired = predecessor.fences[0].occurrence;
                let mut successor = predecessor.clone();
                successor.fences.remove(0);
                let mut plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
                plan.retire(retired);
                let mut retirement =
                    recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
                retirement.push(RecoveryMutation::RetireFences { plan });
                recovery.commit_durable(retirement).unwrap();
            }
            assert_eq!(recovery.snapshot().fences.len(), 1);
            assert_eq!(
                recovery
                    .snapshot()
                    .roots
                    .iter()
                    .filter(|root| root.kind() == RecoveryRootKind::CleanRegion)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn unrooted_fence_retirement_stays_bounded_across_churn() {
        let mut recovery = store();
        for index in 0..64 {
            let generation = recovery.snapshot().generation;
            let mut append = recovery.begin_protocol_txn(generation, TopologyEpoch(1));
            append.push(RecoveryMutation::RecordDataParityFence {
                fence: fence(RegionId(index), RecoveryGeneration(index)),
            });
            recovery.commit_durable(append).unwrap();

            if recovery.snapshot().fences.len() > 1 {
                let predecessor = recovery.snapshot().clone();
                let retired = predecessor.fences[0].occurrence;
                let mut successor = predecessor.clone();
                successor.fences.remove(0);
                let mut plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
                plan.retire(retired);
                let mut retirement =
                    recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
                retirement.push(RecoveryMutation::RetireFences { plan });
                recovery.commit_durable(retirement).unwrap();
            }
            assert_eq!(recovery.snapshot().fences.len(), 1);
        }
        assert_eq!(
            recovery.snapshot().next_fence_occurrence_id,
            FenceOccurrenceId(65)
        );
    }

    #[test]
    fn retirement_preserves_unrelated_root_bound_evidence() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(1), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(1),
            through_generation: RecoveryGeneration(1),
            fence_occurrence: FenceOccurrenceId(1),
        });
        recovery.commit_durable(transaction).unwrap();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(3), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(2), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();

        let predecessor = recovery.snapshot().clone();
        let mut successor = predecessor.clone();
        successor
            .fences
            .retain(|fence| fence.occurrence != FenceOccurrenceId(2));
        let mut plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
        plan.retire(FenceOccurrenceId(2));
        let mut retirement = recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
        retirement.push(RecoveryMutation::RetireFences { plan });
        recovery.commit_durable(retirement).unwrap();

        assert!(recovery.snapshot().fence(FenceOccurrenceId(1)).is_some());
        assert!(recovery.snapshot().fence(FenceOccurrenceId(2)).is_none());
        assert_eq!(
            recovery.snapshot().roots[0].occurrence,
            FenceOccurrenceId(1)
        );
    }

    #[test]
    fn retirement_bound_rejection_preserves_predecessor() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(1), RecoveryGeneration(1)),
        });
        recovery.commit_durable(transaction).unwrap();
        let predecessor = recovery.snapshot().clone();
        let base = predecessor.fences[0].clone();
        let mut successor = predecessor.clone();
        for occurrence in 2..=16_385 {
            successor
                .fences
                .push(base.clone().with_occurrence(FenceOccurrenceId(occurrence)));
        }
        successor.next_fence_occurrence_id = FenceOccurrenceId(16_386);
        let plan = RecoveryRetirementPlan::new(predecessor.clone(), successor);
        let mut retirement = recovery.begin_protocol_txn(predecessor.generation, TopologyEpoch(1));
        retirement.push(RecoveryMutation::RetireFences { plan });
        assert_eq!(
            recovery.commit_durable(retirement),
            Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::StoreFence
            ))
        );
        assert_eq!(recovery.snapshot(), &predecessor);
    }

    #[test]
    fn generation_checked_commit_is_atomic() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        transaction.push(RecoveryMutation::MarkIntegrityStale {
            extent: IntegrityExtentId(1),
            stale_generation: RecoveryGeneration(1),
        });
        assert_eq!(
            recovery.commit_durable(transaction).unwrap(),
            RecoveryGeneration(1)
        );

        let before = recovery.snapshot().clone();
        let mut stale = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        stale.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(2),
            mutation_generation: RecoveryGeneration(2),
        });
        assert!(matches!(
            recovery.commit_durable(stale),
            Err(RecoveryError::GenerationMismatch { .. })
        ));
        assert_eq!(&before, recovery.snapshot());
    }

    #[test]
    fn dirty_region_cannot_be_clean_without_covering_fence() {
        let mut recovery = store();
        let mut dirty = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        dirty.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(7),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(dirty).unwrap();

        let mut clean = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        clean.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(7),
            through_generation: RecoveryGeneration(2),
            fence_occurrence: FenceOccurrenceId(1),
        });
        assert!(matches!(
            recovery.commit_durable(clean),
            Err(RecoveryError::InvalidTransition(
                TransitionError::FenceCoverageMissing
            ))
        ));
        assert!(matches!(
            recovery.snapshot().dirty_regions[0].state,
            RegionState::Dirty { .. }
        ));
    }

    #[test]
    fn fence_allows_recovery_clean_and_valid_digest() {
        let mut recovery = store();
        let mut intent = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        intent.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(7),
            mutation_generation: RecoveryGeneration(1),
        });
        intent.push(RecoveryMutation::MarkIntegrityStale {
            extent: IntegrityExtentId(9),
            stale_generation: RecoveryGeneration(1),
        });
        intent.push(RecoveryMutation::RecordDataParityFence {
            fence: fence(RegionId(7), RecoveryGeneration(1)),
        });
        recovery.commit_durable(intent).unwrap();

        let store_fence = recovery.snapshot().fences[0].stores[0];
        let mut recovery_clean =
            recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        recovery_clean.push(RecoveryMutation::MarkRegionClean {
            region: RegionId(7),
            through_generation: RecoveryGeneration(1),
            fence_occurrence: FenceOccurrenceId(1),
        });
        recovery_clean.push(RecoveryMutation::InstallIntegrityDigest {
            record: IntegrityRecord {
                extent: IntegrityExtentId(9),
                state: IntegrityState::Valid {
                    binding: ChecksumEvidenceBinding {
                        extent: ChecksumExtent {
                            id: IntegrityExtentId(9),
                            target: ChecksumTarget::data(dwv_core::SlotId([1; 16])),
                            range: dwv_core::ByteRange::new(0, 1).unwrap(),
                        },
                        profile: BLAKE3_256_PROFILE.id,
                        set_generation: ChecksumSetGeneration::INITIAL,
                        topology_epoch: TopologyEpoch(1),
                    },
                    content_generation: RecoveryGeneration(1),
                    durable_fence: store_fence,
                    fence_occurrence: FenceOccurrenceId(1),
                    digest: vec![1, 2, 3],
                    verified_at: RecoveryGeneration(2),
                },
            },
        });
        recovery.commit_durable(recovery_clean).unwrap();
        assert_eq!(
            recovery.snapshot().dirty_regions[0].state,
            RegionState::Clean
        );
        assert!(matches!(
            recovery.snapshot().integrity_records[0].state,
            IntegrityState::Valid { .. }
        ));
    }

    #[test]
    fn topology_prepare_and_commit_change_epoch_explicitly() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        let core_topology = dwv_core::TopologySnapshot::new(
            dwv_core::ArrayId([1; 16]),
            TopologyEpoch(2),
            dwv_core::CodingProfile::new(1, 1).unwrap(),
            dwv_core::ProtectedGeometry::new(64, 1).unwrap(),
            vec![
                dwv_core::TopologyAssignment::new(
                    dwv_core::SlotId([1; 16]),
                    dwv_core::MemberRole::Data,
                    dwv_core::CodingPosition(0),
                    dwv_core::AssignmentInstanceId([11; 16]),
                    dwv_core::AssignmentGeneration(4),
                ),
                dwv_core::TopologyAssignment::new(
                    dwv_core::SlotId([2; 16]),
                    dwv_core::MemberRole::Parity,
                    dwv_core::CodingPosition(1),
                    dwv_core::AssignmentInstanceId([12; 16]),
                    dwv_core::AssignmentGeneration(4),
                ),
            ],
        )
        .unwrap();
        transaction.push(RecoveryMutation::PrepareTopology {
            topology: TopologySnapshot::from_core(core_topology, vec![StoreId(2), StoreId(3)])
                .unwrap(),
        });
        transaction.push(RecoveryMutation::CommitTopology {
            topology_epoch: TopologyEpoch(2),
        });
        recovery.commit_durable(transaction).unwrap();
        assert_eq!(recovery.snapshot().topology_epoch, TopologyEpoch(2));
        assert_eq!(
            recovery
                .snapshot()
                .active_topology
                .as_ref()
                .unwrap()
                .topology_epoch(),
            TopologyEpoch(2)
        );

        let stale_epoch = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        assert!(matches!(
            recovery.commit_durable(stale_epoch),
            Err(RecoveryError::TopologyMismatch {
                expected: TopologyEpoch(1),
                actual: TopologyEpoch(2),
            })
        ));
    }

    #[test]
    fn open_session_blocks_topology_publication_and_close_records_fence() {
        let mut recovery = store();
        let mut begin = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        begin.push(RecoveryMutation::BeginWritableSession {
            session_id: SessionId(8),
            topology_epoch: TopologyEpoch(1),
            dirty_envelope_generation: 3,
        });
        recovery.commit_durable(begin).unwrap();

        let mut close = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        close.push(RecoveryMutation::CloseWritableSession {
            session_id: SessionId(8),
            global_fence: fence(RegionId(7), RecoveryGeneration(1)),
        });
        recovery.commit_durable(close).unwrap();
        assert!(
            recovery
                .snapshot()
                .writable_session
                .as_ref()
                .unwrap()
                .closed
        );
        assert_eq!(recovery.snapshot().fences.len(), 1);
    }

    #[test]
    fn semantic_export_and_health_are_conservative() {
        let mut recovery = store();
        let manifest = recovery.export_manifest(RecoveryGeneration(0)).unwrap();
        assert_eq!(manifest.snapshot, *recovery.snapshot());
        recovery.set_health(RecoveryStoreHealth::Corrupt);
        assert_eq!(recovery.verify_integrity(), RecoveryStoreHealth::Corrupt);
        assert!(matches!(
            recovery.load_assembly_snapshot(),
            Err(RecoveryError::Unhealthy(RecoveryStoreHealth::Corrupt))
        ));
        assert!(matches!(
            recovery.export_manifest(RecoveryGeneration(0)),
            Err(RecoveryError::Unhealthy(RecoveryStoreHealth::Corrupt))
        ));
    }

    #[test]
    fn semantic_schema_migration_and_export_are_versioned_and_bounded() {
        assert_eq!(current_recovery_schema().version, CURRENT_RECOVERY_SCHEMA);
        assert_eq!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(0), CURRENT_RECOVERY_SCHEMA)
                .unwrap()
                .steps,
            vec![
                RecoveryMigrationStep::InitializeSemanticSchemaV1,
                RecoveryMigrationStep::AddMetadataLossAuditV2,
                RecoveryMigrationStep::AddOfflineRebuildV3,
                RecoveryMigrationStep::AddChecksumBaselineV4,
                RecoveryMigrationStep::AddCodedCleanCaptureV5,
                RecoveryMigrationStep::AddCodedCaptureReleaseEvidenceV6,
                RecoveryMigrationStep::AddFenceOccurrenceIdentityV7,
            ]
        );
        assert_eq!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(1), CURRENT_RECOVERY_SCHEMA)
                .unwrap()
                .steps,
            vec![
                RecoveryMigrationStep::AddMetadataLossAuditV2,
                RecoveryMigrationStep::AddOfflineRebuildV3,
                RecoveryMigrationStep::AddChecksumBaselineV4,
                RecoveryMigrationStep::AddCodedCleanCaptureV5,
                RecoveryMigrationStep::AddCodedCaptureReleaseEvidenceV6,
                RecoveryMigrationStep::AddFenceOccurrenceIdentityV7,
            ]
        );
        assert_eq!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(2), CURRENT_RECOVERY_SCHEMA)
                .unwrap()
                .steps,
            vec![
                RecoveryMigrationStep::AddOfflineRebuildV3,
                RecoveryMigrationStep::AddChecksumBaselineV4,
                RecoveryMigrationStep::AddCodedCleanCaptureV5,
                RecoveryMigrationStep::AddCodedCaptureReleaseEvidenceV6,
                RecoveryMigrationStep::AddFenceOccurrenceIdentityV7,
            ]
        );
        assert_eq!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(3), CURRENT_RECOVERY_SCHEMA)
                .unwrap()
                .steps,
            vec![
                RecoveryMigrationStep::AddChecksumBaselineV4,
                RecoveryMigrationStep::AddCodedCleanCaptureV5,
                RecoveryMigrationStep::AddCodedCaptureReleaseEvidenceV6,
                RecoveryMigrationStep::AddFenceOccurrenceIdentityV7,
            ]
        );
        assert_eq!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(4), CURRENT_RECOVERY_SCHEMA)
                .unwrap()
                .steps,
            vec![
                RecoveryMigrationStep::AddCodedCleanCaptureV5,
                RecoveryMigrationStep::AddCodedCaptureReleaseEvidenceV6,
                RecoveryMigrationStep::AddFenceOccurrenceIdentityV7,
            ]
        );
        assert_eq!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(5), CURRENT_RECOVERY_SCHEMA)
                .unwrap()
                .steps,
            vec![
                RecoveryMigrationStep::AddCodedCaptureReleaseEvidenceV6,
                RecoveryMigrationStep::AddFenceOccurrenceIdentityV7,
            ]
        );
        assert!(
            current_recovery_schema()
                .records
                .contains(&RecoveryRecordKind::MetadataLossAudit)
        );
        assert!(
            current_recovery_schema()
                .records
                .contains(&RecoveryRecordKind::ChecksumBaseline)
        );
        assert!(
            current_recovery_schema()
                .records
                .contains(&RecoveryRecordKind::CodedCleanCapture)
        );
        assert!(matches!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(8), CURRENT_RECOVERY_SCHEMA),
            Err(RecoveryError::UnsupportedSchemaMigration { .. })
        ));
        assert!(matches!(
            RecoveryMigrationPlan::plan(RecoverySchemaVersion(8), RecoverySchemaVersion(8)),
            Err(RecoveryError::UnsupportedSchemaMigration { .. })
        ));

        let recovery = store();
        let manifest = recovery.export_manifest(RecoveryGeneration(0)).unwrap();
        assert_eq!(manifest.schema, CURRENT_RECOVERY_SCHEMA);
        assert!(
            recovery
                .export_manifest_with_limits(
                    RecoveryGeneration(0),
                    RecoveryExportLimits {
                        max_dirty_regions: 0,
                        ..RecoveryExportLimits::default()
                    }
                )
                .is_ok()
        );
    }

    #[test]
    fn export_limits_reject_unbounded_semantic_state() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(1),
            mutation_generation: RecoveryGeneration(1),
        });
        recovery.commit_durable(transaction).unwrap();
        assert!(matches!(
            recovery.export_manifest_with_limits(
                RecoveryGeneration(1),
                RecoveryExportLimits {
                    max_dirty_regions: 0,
                    ..RecoveryExportLimits::default()
                }
            ),
            Err(RecoveryError::ExportLimitExceeded(
                RecoveryRecordKind::DirtyRegion
            ))
        ));
    }

    #[test]
    fn non_durable_commit_observation_has_no_side_effect() {
        let mut recovery = store();
        let mut transaction = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        transaction.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(4),
            mutation_generation: RecoveryGeneration(1),
        });
        assert!(matches!(
            recovery.commit_observed(transaction, RecoveryCommitObservation::Lost),
            Err(RecoveryError::CommitNotDurable(
                RecoveryCommitObservation::Lost
            ))
        ));
        assert_eq!(recovery.snapshot().generation, RecoveryGeneration(0));
        assert!(recovery.snapshot().dirty_regions.is_empty());
    }

    #[test]
    fn durable_reopen_cut_discards_uncommitted_owner_state() {
        let mut recovery = store();
        let mut durable = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        durable.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(5),
            mutation_generation: RecoveryGeneration(1),
        });
        let generation = recovery.commit_durable(durable).unwrap();
        let cut = RecoveryReopenCut::capture(&recovery, generation).unwrap();

        let mut pending = recovery.begin_protocol_txn(generation, TopologyEpoch(1));
        pending.push(RecoveryMutation::MarkRegionDirty {
            region: RegionId(6),
            mutation_generation: RecoveryGeneration(2),
        });
        assert!(matches!(
            recovery.commit_observed(pending, RecoveryCommitObservation::Lost),
            Err(RecoveryError::CommitNotDurable(
                RecoveryCommitObservation::Lost
            ))
        ));

        let reopened = cut.reopen().unwrap();
        assert_eq!(reopened.snapshot(), &cut.manifest().snapshot);
        assert_eq!(reopened.snapshot().generation, generation);
        assert!(
            reopened
                .snapshot()
                .dirty_regions
                .iter()
                .all(|region| region.region != RegionId(6))
        );
    }

    #[test]
    fn missing_and_corrupt_state_are_not_writable() {
        let mut recovery = store();
        recovery.set_health(RecoveryStoreHealth::Missing);
        assert_eq!(
            recovery.write_policy(),
            RecoveryWritePolicy::Blocked(RecoveryStoreHealth::Missing)
        );
        assert_eq!(recovery.disposition(), RecoveryDisposition::RebuildFromData);
        recovery.set_health(RecoveryStoreHealth::Corrupt);
        assert_eq!(recovery.disposition(), RecoveryDisposition::RebuildFromData);
    }

    #[test]
    fn recovery_simulation_fixtures_cover_conservative_failures() {
        let cases = recovery_simulation_cases();
        assert!(cases.iter().any(|case| {
            case.fault == RecoveryFaultPoint::AfterDataParityWriteBeforeFence
                && !case.permits_data_parity_write
        }));
        assert!(cases.iter().any(|case| {
            case.fault == RecoveryFaultPoint::CorruptJournal
                && case.expected_health == RecoveryStoreHealth::Corrupt
        }));
    }

    #[test]
    fn integrity_digest_requires_extent_coverage_and_current_generation() {
        let mut recovery = store();
        let mut intent = recovery.begin_protocol_txn(RecoveryGeneration(0), TopologyEpoch(1));
        intent.push(RecoveryMutation::MarkIntegrityStale {
            extent: IntegrityExtentId(9),
            stale_generation: RecoveryGeneration(1),
        });
        intent.push(RecoveryMutation::RecordDataParityFence {
            fence: FenceCertificate::new(
                TopologyEpoch(1),
                FenceDomain(1),
                vec![StoreFenceRef {
                    fence_id: dwv_store::FenceId(1),
                    store_id: StoreId(2),
                    store_incarnation: dwv_store::StoreIncarnationId(0),
                    topology_epoch: TopologyEpoch(1),
                    through: dwv_store::StoreWriteWatermark(10),
                    capability_evidence_id: dwv_store::CapabilityEvidenceId(3),
                }],
                vec![],
            ),
        });
        recovery.commit_durable(intent).unwrap();
        let store_fence = recovery.snapshot().fences[0].stores[0];
        let mut digest = recovery.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(1));
        digest.push(RecoveryMutation::InstallIntegrityDigest {
            record: IntegrityRecord {
                extent: IntegrityExtentId(9),
                state: IntegrityState::Valid {
                    binding: ChecksumEvidenceBinding {
                        extent: ChecksumExtent {
                            id: IntegrityExtentId(9),
                            target: ChecksumTarget::data(dwv_core::SlotId([1; 16])),
                            range: dwv_core::ByteRange::new(0, 1).unwrap(),
                        },
                        profile: BLAKE3_256_PROFILE.id,
                        set_generation: ChecksumSetGeneration::INITIAL,
                        topology_epoch: TopologyEpoch(1),
                    },
                    content_generation: RecoveryGeneration(2),
                    durable_fence: store_fence,
                    fence_occurrence: FenceOccurrenceId(1),
                    digest: vec![1],
                    verified_at: RecoveryGeneration(2),
                },
            },
        });
        assert!(matches!(
            recovery.commit_durable(digest),
            Err(RecoveryError::InvalidTransition(
                TransitionError::IntegrityCoverageMissing
            ))
        ));
    }

    #[test]
    fn dirty_integrity_write_recovery_record_survives_semantic_export() {
        let mut recovery = MemoryRecoveryStore::new(TopologyEpoch(1));
        let evidence = WriteRecoveryRecordCommit::new(
            &mut recovery,
            TopologyEpoch(1),
            RecoveryGeneration(0),
            InvalidationTarget::new(vec![RegionId(5)], vec![IntegrityExtentId(6)]),
        )
        .commit()
        .unwrap();
        let manifest = recovery
            .export_manifest(evidence.committed_generation)
            .unwrap();
        assert_eq!(manifest.snapshot.generation, evidence.committed_generation);
        assert!(matches!(
            manifest.snapshot.dirty_regions[0].state,
            RegionState::Dirty { .. }
        ));
        assert!(matches!(
            manifest.snapshot.integrity_records[0].state,
            IntegrityState::Stale { .. }
        ));
    }
}
