use crate::{
    BaselineDisposition, ChecksumExtent, ChecksumProfile, ChecksumSetGeneration, ChecksumTarget,
    ExtentError, IntegrityState, RecoveryGeneration, RecoverySnapshot, TopologySnapshot,
};
use dwv_core::{MemberRole, TopologyEpoch};
use dwv_store::StoreId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum ChecksumBaselineProvenance {
    PostRecoveryNewlyCalculated,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ChecksumBaseline {
    pub topology_epoch: TopologyEpoch,
    pub profile: ChecksumProfile,
    pub set_generation: ChecksumSetGeneration,
    pub content_generation: RecoveryGeneration,
    pub provenance: ChecksumBaselineProvenance,
    pub expected_extents: Vec<ChecksumExtent>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumBaselineInvalidReason {
    MissingTopology,
    TopologyMismatch,
    UnsupportedProfile,
    ExtentSetMismatch,
    UnexpectedRecord,
    DuplicateRecord,
    InvalidCurrentEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumBaselineStatus {
    NotRequired,
    Required { total: usize },
    Partial { valid: usize, total: usize },
    Complete { total: usize },
    Invalid(ChecksumBaselineInvalidReason),
}

pub fn new_checksum_baseline(
    topology: &TopologySnapshot,
    content_generation: RecoveryGeneration,
    profile: ChecksumProfile,
    set_generation: ChecksumSetGeneration,
) -> Result<ChecksumBaseline, ExtentError> {
    profile.validate().map_err(ExtentError::InvalidProfile)?;
    Ok(ChecksumBaseline {
        topology_epoch: topology.topology_epoch(),
        profile,
        set_generation,
        content_generation,
        provenance: ChecksumBaselineProvenance::PostRecoveryNewlyCalculated,
        expected_extents: expected_checksum_extents(topology, profile)?,
    })
}

/// dwv:req req.checksum-plane.current-baseline-completion-is-persisted-and-exact
pub fn assess_checksum_baseline(snapshot: &RecoverySnapshot) -> ChecksumBaselineStatus {
    let required = snapshot.metadata_loss_audit.is_some_and(|audit| {
        matches!(
            audit.baseline,
            BaselineDisposition::NewChecksumBaselineRequired
                | BaselineDisposition::NewParityAndChecksumBaselineRequired
        )
    });
    if !required {
        return ChecksumBaselineStatus::NotRequired;
    }

    let Some(topology) = snapshot.active_topology.as_ref() else {
        return ChecksumBaselineStatus::Invalid(ChecksumBaselineInvalidReason::MissingTopology);
    };
    let Some(baseline) = snapshot.checksum_baseline.as_ref() else {
        return ChecksumBaselineStatus::Required { total: 0 };
    };
    if baseline.topology_epoch != snapshot.topology_epoch
        || baseline.topology_epoch != topology.topology_epoch()
    {
        return ChecksumBaselineStatus::Invalid(ChecksumBaselineInvalidReason::TopologyMismatch);
    }
    if !baseline.profile.is_supported() {
        return ChecksumBaselineStatus::Invalid(ChecksumBaselineInvalidReason::UnsupportedProfile);
    }
    let Ok(expected) = expected_checksum_extents(topology, baseline.profile) else {
        return ChecksumBaselineStatus::Invalid(ChecksumBaselineInvalidReason::ExtentSetMismatch);
    };
    if baseline.expected_extents != expected {
        return ChecksumBaselineStatus::Invalid(ChecksumBaselineInvalidReason::ExtentSetMismatch);
    }
    if snapshot
        .integrity_records
        .iter()
        .any(|record| !expected.iter().any(|extent| extent.id == record.extent))
    {
        return ChecksumBaselineStatus::Invalid(ChecksumBaselineInvalidReason::UnexpectedRecord);
    }

    let mut valid = 0;
    for extent in &expected {
        let mut records = snapshot
            .integrity_records
            .iter()
            .filter(|record| record.extent == extent.id);
        let Some(record) = records.next() else {
            continue;
        };
        if records.next().is_some() {
            return ChecksumBaselineStatus::Invalid(ChecksumBaselineInvalidReason::DuplicateRecord);
        }
        let IntegrityState::Valid {
            binding,
            content_generation,
            durable_fence,
            fence_occurrence,
            digest,
            verified_at,
        } = &record.state
        else {
            continue;
        };
        if binding.extent != *extent
            || binding.profile != baseline.profile.id
            || binding.set_generation != baseline.set_generation
            || binding.topology_epoch != baseline.topology_epoch
        {
            return ChecksumBaselineStatus::Invalid(
                ChecksumBaselineInvalidReason::InvalidCurrentEvidence,
            );
        }
        if *content_generation != baseline.content_generation
            || *verified_at < baseline.content_generation
            || *verified_at > snapshot.generation
            || durable_fence.topology_epoch != baseline.topology_epoch
            || digest.len() != usize::from(baseline.profile.digest_size)
            || target_store(topology, extent.target) != Some(durable_fence.store_id)
            || !snapshot
                .fence(*fence_occurrence)
                .is_some_and(|certificate| {
                    certificate.contains_store_fence(*durable_fence)
                        && certificate.covers_integrity_extent(extent.id, *content_generation)
                })
        {
            return ChecksumBaselineStatus::Invalid(
                ChecksumBaselineInvalidReason::InvalidCurrentEvidence,
            );
        }
        valid += 1;
    }

    if valid == expected.len() {
        ChecksumBaselineStatus::Complete {
            total: expected.len(),
        }
    } else if valid == 0 {
        ChecksumBaselineStatus::Required {
            total: expected.len(),
        }
    } else {
        ChecksumBaselineStatus::Partial {
            valid,
            total: expected.len(),
        }
    }
}
pub fn pending_checksum_baseline_extents(
    snapshot: &RecoverySnapshot,
) -> Result<Option<Vec<ChecksumExtent>>, ChecksumBaselineInvalidReason> {
    match assess_checksum_baseline(snapshot) {
        ChecksumBaselineStatus::NotRequired => return Ok(None),
        ChecksumBaselineStatus::Invalid(reason) => return Err(reason),
        ChecksumBaselineStatus::Required { .. }
        | ChecksumBaselineStatus::Partial { .. }
        | ChecksumBaselineStatus::Complete { .. } => {}
    }
    let baseline = snapshot
        .checksum_baseline
        .as_ref()
        .ok_or(ChecksumBaselineInvalidReason::ExtentSetMismatch)?;
    Ok(Some(
        baseline
            .expected_extents
            .iter()
            .filter(|extent| {
                !snapshot.integrity_records.iter().any(|record| {
                    record.extent == extent.id
                        && matches!(record.state, IntegrityState::Valid { .. })
                })
            })
            .cloned()
            .collect(),
    ))
}

pub fn expected_checksum_extents(
    topology: &TopologySnapshot,
    profile: ChecksumProfile,
) -> Result<Vec<ChecksumExtent>, ExtentError> {
    profile.validate().map_err(ExtentError::InvalidProfile)?;
    let per_member = topology
        .geometry()
        .protected_length()
        .div_ceil(profile.extent_size);
    let mut extents = Vec::new();
    for assignment in topology.assignments() {
        let target = match assignment.role() {
            MemberRole::Data => ChecksumTarget::data(assignment.slot_id()),
            MemberRole::Parity => ChecksumTarget::parity(assignment.coding_position()),
        };
        let first_id = u64::from(assignment.coding_position().0)
            .checked_mul(per_member)
            .ok_or(ExtentError::Overflow)?;
        extents.extend(ChecksumExtent::partition(
            target,
            topology.geometry().protected_length(),
            profile.extent_size,
            crate::IntegrityExtentId(first_id),
        )?);
    }
    extents.sort_unstable_by_key(|extent| extent.id);
    Ok(extents)
}

fn target_store(topology: &TopologySnapshot, target: ChecksumTarget) -> Option<StoreId> {
    topology
        .assignments()
        .iter()
        .find(|assignment| match target {
            ChecksumTarget::Data { slot } => {
                assignment.role() == MemberRole::Data && assignment.slot_id() == slot
            }
            ChecksumTarget::Parity { position } | ChecksumTarget::Q { position } => {
                assignment.role() == MemberRole::Parity && assignment.coding_position() == position
            }
        })
        .map(|assignment| assignment.store_id())
}
