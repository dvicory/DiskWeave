use crate::array::{ArrayPolicy, FrontendPolicy, MemberPolicy};
use dwv_codec::Geometry;
use dwv_core::{
    ArrayId, AssignmentGeneration, AssignmentInstanceId, CodingPosition, CodingProfile,
    FenceDomain, MemberRole, ProtectedGeometry, SlotId, TopologyAssignment, TopologyEpoch,
    TopologySnapshot,
};
use dwv_recovery::{
    ChecksumBaselineStatus, ChecksumEvidenceBinding, ChecksumTarget, FenceCertificate,
    IntegrityRecord, IntegrityState, MetadataLossCase, MetadataLossPlan, RecoveryCommitObservation,
    RecoveryError, RecoveryInspection, RecoveryManifest, RecoveryMutation, RecoverySnapshot,
    RecoveryStateStore, TopologySnapshot as RecoveryTopologySnapshot, assess_checksum_baseline,
    pending_checksum_baseline_extents,
};
use dwv_recovery_sqlite::SqliteRecoveryStore;
use dwv_service::{RebuildSource, ServiceConfig, ServiceError};
use dwv_store::{
    ChildOperationId, CompletionDisposition, OperationSlotToken, PersistenceEvidence, StoreId,
    StoreWriteWatermark,
};
use dwv_store_file::{FileStore, FileStoreConfig, FileSyncMode, observe_file_identity};
use dwv_verify::{
    ChecksumEvidence, DigestEvidence, MemberRef, MismatchClass, RegionDisposition, ScanConfig,
    ScanMode, VerificationReport, verify_exhaustive,
};
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const RESULT_SCHEMA: &str = "dwv.operator.v2";

type FileRebuildSource = RebuildSource<FileStore>;
const MAX_EXTENTS: usize = 1_048_576;

/// dwv:req req.operator-recovery.production-assessment-is-observational-and-multidimensional
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Success,
    Usage,
    Refused,
    Blocked,
    NotSupported,
    OperationFailed,
    ReconciliationRequired,
}

impl Outcome {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Usage => 2,
            Self::Blocked => 3,
            Self::Refused => 4,
            Self::NotSupported => 5,
            Self::OperationFailed | Self::ReconciliationRequired => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineageDisposition {
    Accepted,
    Ambiguous,
    Unproved,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LineageAssessment {
    pub disposition: LineageDisposition,
    pub reason: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)] // C0a emits only evidence-proven states; later capabilities may prove the others.
pub enum CustodyDisposition {
    ContinuityProved,
    GapObserved,
    ContinuityUnproved,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustodyAssessment {
    pub disposition: CustodyDisposition,
    pub reason: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)] // C0a conservatively emits not-yet-interpretable without basis evidence.
pub enum ProtectionBasis {
    Current,
    Prior,
    Unprotected,
    Indeterminate,
    NotYetInterpretable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct AuthorityRange {
    pub offset: u64,
    pub length: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProtectionBasisCoverage {
    pub role: String,
    pub basis: ProtectionBasis,
    pub range_count: usize,
    pub bytes: u64,
    pub exact_ranges: Vec<AuthorityRange>,
    pub detail_truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuthorityAssessment {
    pub lineage: LineageAssessment,
    pub custody: CustodyAssessment,
    pub protection_basis: Vec<ProtectionBasisCoverage>,
    pub blocker: &'static str,
    pub authorization: &'static str,
}
impl AuthorityAssessment {
    const NON_AUTHORIZATION: &'static str = "observation authorizes no publication, payload mutation, repair, reconstruction, protection-epoch transition, currentization, historical continuity, or destructive recovery action";

    fn unassessed() -> Self {
        Self {
            lineage: LineageAssessment {
                disposition: LineageDisposition::Unproved,
                reason: "lineage authority was not assessed",
            },
            custody: CustodyAssessment {
                disposition: CustodyDisposition::ContinuityUnproved,
                reason: "custody continuity was not assessed",
            },
            protection_basis: Vec::new(),
            blocker: "authority was not assessed",
            authorization: Self::NON_AUTHORIZATION,
        }
    }
}

/// dwv:req req.operator-recovery.production-assessment-is-observational-and-multidimensional
#[derive(Clone, Debug, Serialize)]
pub struct OperatorResult {
    pub schema: &'static str,
    pub kind: &'static str,
    pub command: &'static str,
    pub outcome: Outcome,
    pub reason_code: &'static str,
    pub reason: String,
    pub authority: AuthorityAssessment,
    pub recovery: RecoveryAssessment,
    pub topology: Option<TopologyAssessment>,
    pub members: Vec<MemberAssessment>,
    pub checksum: ChecksumAssessment,
    pub lifecycle: &'static str,
    pub access: &'static str,
    pub start: &'static str,
    pub parity: &'static str,
    pub damage: &'static str,
    pub redundancy: String,
    pub next_action: &'static str,
    pub publication: PublicationAssessment,
    pub verification: Option<VerificationAssessment>,
    pub recovery_plan: Option<RecoveryPlanAssessment>,
}

impl OperatorResult {
    pub(crate) fn new(
        command: &'static str,
        outcome: Outcome,
        reason_code: &'static str,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            schema: RESULT_SCHEMA,
            kind: "array-operator-result",
            command,
            outcome,
            reason_code,
            reason: reason.into(),
            authority: AuthorityAssessment::unassessed(),
            recovery: RecoveryAssessment::Unavailable {
                reason: "unassessed".into(),
            },
            topology: None,
            members: Vec::new(),
            checksum: ChecksumAssessment::NotRequired,
            lifecycle: "stopped",
            access: "none",
            start: "unassessed",
            parity: "not-verified",
            damage: "not-assessed",
            redundancy: "not-established".into(),
            next_action: "inspect",
            publication: PublicationAssessment::NotPublished,
            verification: None,
            recovery_plan: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "classification", rename_all = "kebab-case")]
pub enum RecoveryAssessment {
    Absent,
    Supported {
        generation: u64,
        topology_epoch: u64,
    },
    CorruptOrUnreadable,
    Unsupported {
        layer: String,
        version: Option<u64>,
    },
    MigrationRequired {
        layer: String,
        from: u64,
        to: u64,
    },
    ReconciliationRequired,
    Unavailable {
        reason: String,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct TopologyAssessment {
    pub array_id: String,
    pub topology_epoch: u64,
    pub data_slots: u16,
    pub parity_slots: u16,
    pub protected_length: u64,
    pub logical_block_size: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct MemberAssessment {
    pub path: PathBuf,
    pub role: String,
    pub expected_identity: String,
    pub observed_identity: Option<String>,
    pub status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum ChecksumAssessment {
    NotRequired,
    Required { total: usize },
    Partial { valid: usize, total: usize },
    Complete { total: usize },
    Invalid { reason: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum PublicationAssessment {
    NotPublished,
    Published { device_path: String },
    Unsupported { reason: String },
    ReconciliationRequired { reason: String },
}

#[derive(Clone, Debug, Serialize)]
pub struct VerificationAssessment {
    pub mode: &'static str,
    pub complete: bool,
    pub matching_regions: usize,
    pub regions: Vec<RegionAssessment>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RegionAssessment {
    pub offset: u64,
    pub length: u64,
    pub disposition: String,
    pub data_evidence: Vec<String>,
    pub parity_evidence: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecoveryPlanAssessment {
    pub plan_id: String,
    pub case: &'static str,
    pub action: &'static str,
    pub disposition: &'static str,
    pub required_evidence: &'static str,
    pub payload_write_policy: &'static str,
    pub baseline: &'static str,
    pub executable: bool,
    pub missing: Vec<String>,
}

#[derive(Clone, Debug)]
struct ObservedMember {
    policy: MemberPolicy,
    path: PathBuf,
    identity: Option<[u8; 16]>,
    status: &'static str,
}
struct ExhaustiveRun {
    report: VerificationReport,
    _data: Vec<FileRebuildSource>,
    _parity: FileRebuildSource,
}

#[derive(Debug)]
pub enum OperatorError {
    Usage(String),
    Invalid(String),
    Blocked(String),
    Unsupported(String),
    Failed(String),
    LegacyFailed { message: String, exit_code: u8 },
    Reconciliation(String),
}

impl OperatorError {
    pub const fn outcome(&self) -> Outcome {
        match self {
            Self::Usage(_) => Outcome::Usage,
            Self::Invalid(_) => Outcome::Refused,
            Self::Blocked(_) => Outcome::Blocked,
            Self::Unsupported(_) => Outcome::NotSupported,
            Self::Failed(_) | Self::LegacyFailed { .. } => Outcome::OperationFailed,
            Self::Reconciliation(_) => Outcome::ReconciliationRequired,
        }
    }
    pub const fn reason_code(&self) -> &'static str {
        match self {
            Self::Usage(_) => "usage-error",
            Self::Invalid(_) => "semantic-refusal",
            Self::Blocked(_) => "prerequisite-unavailable",
            Self::Unsupported(_) => "capability-not-supported",
            Self::Failed(_) => "operation-failed",
            Self::LegacyFailed { .. } => "legacy-operation-failed",
            Self::Reconciliation(_) => "reconciliation-required",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Usage(message)
            | Self::Invalid(message)
            | Self::Blocked(message)
            | Self::Unsupported(message)
            | Self::Failed(message)
            | Self::LegacyFailed { message, .. }
            | Self::Reconciliation(message) => message,
        }
    }
}
pub fn operation_error_result(
    policy: &ArrayPolicy,
    command: &'static str,
    error: OperatorError,
) -> OperatorResult {
    let mut result = observe(policy, command);
    result.outcome = error.outcome();
    result.reason_code = error.reason_code();
    result.reason = error.message().into();
    result.next_action = match result.outcome {
        Outcome::Usage => "correct-input",
        Outcome::Refused => "inspect",
        Outcome::Blocked => "restore-prerequisite",
        Outcome::NotSupported => "select-supported-operation",
        Outcome::OperationFailed => "inspect-operation-failure",
        Outcome::ReconciliationRequired => "reconcile",
        Outcome::Success => "none",
    };
    result
}

const MAX_AUTHORITY_DETAIL_RANGES: usize = 64;

fn authority_assessment(
    policy: &ArrayPolicy,
    recovery: &RecoveryInspection,
    members: &[ObservedMember],
    topology_matches: bool,
) -> AuthorityAssessment {
    let lineage = if !matches!(recovery, RecoveryInspection::Supported(_)) || !topology_matches {
        LineageAssessment {
            disposition: LineageDisposition::Unproved,
            reason: "independent recovery and topology evidence is unavailable or mismatched",
        }
    } else if members
        .iter()
        .any(|member| member.status == "ambiguous-clone")
    {
        LineageAssessment {
            disposition: LineageDisposition::Ambiguous,
            reason: "multiple policy entries resolve to one observed member identity",
        }
    } else if members.iter().all(|member| member.status == "recognized") {
        LineageAssessment {
            disposition: LineageDisposition::Accepted,
            reason: "recovery topology and every current member identity agree",
        }
    } else {
        LineageAssessment {
            disposition: LineageDisposition::Unproved,
            reason: "one or more required member identities are missing or changed",
        }
    };

    let custody = CustodyAssessment {
        disposition: CustodyDisposition::ContinuityUnproved,
        reason: if matches!(recovery, RecoveryInspection::Supported(_)) {
            "the raw-member observation has no certified continuity evidence for the interval"
        } else {
            "recovery evidence cannot establish custody continuity"
        },
    };

    let protection_basis = policy
        .members
        .iter()
        .filter(|member| member.role == "parity")
        .map(|member| {
            coverage_for_ranges(
                member.role.clone(),
                ProtectionBasis::NotYetInterpretable,
                &[AuthorityRange {
                    offset: 0,
                    length: policy.protected_length,
                }],
            )
            .expect("policy protected length must be representable")
        })
        .collect();

    AuthorityAssessment {
        lineage,
        custody,
        protection_basis,
        blocker: "no canonical evidence names a current, prior, or unprotected parity basis for these ranges",
        authorization: AuthorityAssessment::NON_AUTHORIZATION,
    }
}

fn coverage_for_ranges(
    role: String,
    basis: ProtectionBasis,
    ranges: &[AuthorityRange],
) -> Result<ProtectionBasisCoverage, &'static str> {
    let mut merged: Vec<AuthorityRange> = Vec::new();
    for range in ranges.iter().copied().filter(|range| range.length != 0) {
        let end = range
            .offset
            .checked_add(range.length)
            .ok_or("authority range exceeds u64")?;
        if let Some(previous) = merged.last_mut() {
            let previous_end = previous
                .offset
                .checked_add(previous.length)
                .ok_or("authority range exceeds u64")?;
            if previous_end >= range.offset {
                let merged_end = previous_end.max(end);
                previous.length = merged_end
                    .checked_sub(previous.offset)
                    .ok_or("authority range merge underflow")?;
                continue;
            }
        }
        merged.push(range);
    }
    let bytes = merged.iter().try_fold(0_u64, |total, range| {
        total
            .checked_add(range.length)
            .ok_or("authority coverage exceeds u64")
    })?;
    let detail_truncated = merged.len() > MAX_AUTHORITY_DETAIL_RANGES;
    let exact_ranges = merged
        .iter()
        .take(MAX_AUTHORITY_DETAIL_RANGES)
        .copied()
        .collect();
    Ok(ProtectionBasisCoverage {
        role,
        basis,
        range_count: merged.len(),
        bytes,
        exact_ranges,
        detail_truncated,
    })
}

/// dwv:req req.operator-recovery.production-assessment-is-observational-and-multidimensional
pub fn observe(policy: &ArrayPolicy, command: &'static str) -> OperatorResult {
    let recovery = SqliteRecoveryStore::inspect(&policy.recovery_path);
    let mut members: Vec<_> = policy
        .members
        .iter()
        .map(|member| observe_member(policy, member))
        .collect();
    for index in 0..members.len() {
        let Some(identity) = members[index].identity else {
            continue;
        };
        if members
            .iter()
            .enumerate()
            .any(|(other, member)| other != index && member.identity == Some(identity))
        {
            members[index].status = "ambiguous-clone";
        }
    }
    let topology = recovery
        .manifest()
        .and_then(|manifest| manifest.snapshot.active_topology.clone());
    let policy_topology_matches = policy_topology(policy)
        .ok()
        .is_some_and(|expected| topology.as_ref() == Some(&expected));
    let lineage_accepted =
        policy_topology_matches && members.iter().all(|member| member.status == "recognized");
    let authority = authority_assessment(policy, &recovery, &members, policy_topology_matches);
    let mut result = OperatorResult::new(
        command,
        Outcome::Success,
        "array-observed",
        "observed array without granting stronger authority",
    );
    result.authority = authority;
    result.recovery = recovery_assessment(&recovery);
    result.topology = topology.as_ref().map(topology_assessment);
    result.members = members.iter().map(member_assessment).collect();
    result.checksum = recovery
        .manifest()
        .map(checksum_assessment)
        .unwrap_or(ChecksumAssessment::NotRequired);
    result.publication = publication(policy);
    result.parity = "not-authorized";
    result.redundancy = "not-established".into();
    if !matches!(recovery, RecoveryInspection::Supported(_)) {
        result.start = "recovery-required";
        result.next_action = "recover";
    } else if !lineage_accepted {
        result.start = "member-resolution-required";
        result.next_action = "members";
    } else {
        result.start = "authority-assessment-required";
        result.next_action = "reconcile-authority";
    }
    if matches!(result.publication, PublicationAssessment::Published { .. }) {
        result.lifecycle = "online";
        result.access = "read-write";
        result.start = "already-online";
        result.next_action = "none";
    } else if matches!(recovery, RecoveryInspection::ReconciliationRequired) {
        result.outcome = Outcome::ReconciliationRequired;
        result.reason_code = "reconciliation-required";
        result.reason = "recovery commit state requires reconciliation".into();
        result.start = "reconciliation-required";
        result.next_action = "reconcile";
    } else if !matches!(recovery, RecoveryInspection::Supported(_)) {
        result.reason_code = "recovery-authority-not-current";
        result.reason = "observation completed; recovery authority is not current".into();
    }
    result
}

pub fn members_result(policy: &ArrayPolicy) -> OperatorResult {
    observe(policy, "members")
}

/// dwv:req req.operator-recovery.production-assessment-is-observational-and-multidimensional
pub fn scrub(policy: &ArrayPolicy) -> Result<OperatorResult, OperatorError> {
    let inspection = SqliteRecoveryStore::inspect(&policy.recovery_path);
    let Some(manifest) = inspection.manifest().cloned() else {
        let mut result = observe(policy, "scrub");
        result.outcome = Outcome::Blocked;
        result.reason_code = "recovery-authority-not-current";
        result.reason =
            "exhaustive verification requires a current canonical topology from recovery state"
                .into();
        result.parity = "not-established";
        result.damage = "unresolved";
        result.next_action = "recover";
        result.verification = Some(unavailable_verification(policy, &result.members));
        return Ok(result);
    };
    let topology = manifest
        .snapshot
        .active_topology
        .clone()
        .ok_or_else(|| OperatorError::Invalid("current recovery topology is missing".into()))?;
    let mut result = observe(policy, "scrub");
    result.topology = Some(topology_assessment(&topology));
    if result
        .members
        .iter()
        .any(|member| member.status != "recognized")
    {
        let unreadable = result
            .members
            .iter()
            .any(|member| member.status == "unreadable");
        result.outcome = if unreadable {
            Outcome::Blocked
        } else {
            Outcome::Refused
        };
        result.reason_code = if unreadable {
            "member-unreadable"
        } else {
            "member-identity-unresolved"
        };
        result.parity = "not-established";
        result.damage = if unreadable {
            "unreadable"
        } else {
            "unresolved"
        };
        result.reason = if unreadable {
            "exhaustive verification could not read every configured member"
        } else {
            "exhaustive verification refused members whose identity changed"
        }
        .into();
        result.next_action = if unreadable {
            "restore-member-access"
        } else {
            "resolve-member-identity"
        };
        result.verification = Some(unavailable_verification(policy, &result.members));
        return Ok(result);
    }
    let run = run_exhaustive(policy, &topology, false, Some(&manifest.snapshot))?;
    let scope_complete = run
        .report
        .regions()
        .iter()
        .all(|region| !matches!(region.disposition, RegionDisposition::Incomplete { .. }));
    result.verification = Some(verification_assessment(&run.report));
    if scope_complete && run.report.parity_consistent() {
        result.reason_code = "exhaustive-verification-matched";
        result.reason = "exhaustive verification matched every region".into();
        result.parity = "verified-consistent";
        result.damage = "none-detected";
    } else if scope_complete {
        result.reason_code = "exhaustive-verification-found-damage";
        result.reason =
            "exhaustive verification completed with unresolved or identified mismatches".into();
        result.parity = "verified-mismatch";
        result.damage = "findings-present";
    } else {
        result.reason_code = "exhaustive-verification-incomplete";
        result.reason = "verification was incomplete".into();
        result.parity = "not-established";
        result.damage = "unreadable";
    }
    Ok(result)
}

pub fn damage(policy: &ArrayPolicy) -> Result<OperatorResult, OperatorError> {
    let mut result = scrub(policy)?;
    result.command = "damage";
    result.reason_code = "damage-classified";
    result.reason = "current damage classifications from exhaustive read-only verification".into();
    Ok(result)
}

/// dwv:req req.operator-recovery.recovery-preview-is-read-only-and-apply-re-establishes-authority
pub fn recover_preview(policy: &ArrayPolicy) -> Result<OperatorResult, OperatorError> {
    let recovery = SqliteRecoveryStore::inspect(&policy.recovery_path);
    let recognized = usable_topology_from_members(policy)?;
    let (case, missing) = match &recovery {
        RecoveryInspection::Absent | RecoveryInspection::CorruptOrUnreadable { .. }
            if recognized =>
        {
            (
                MetadataLossCase::AllMetadataAllDataPresent,
                vec![
                    "explicit new array lineage".into(),
                    "separate parity target".into(),
                    "verified parity rebuild and checksum baseline".into(),
                ],
            )
        }
        RecoveryInspection::Absent | RecoveryInspection::CorruptOrUnreadable { .. } => (
            MetadataLossCase::TopologyAmbiguous,
            vec!["unambiguous independently observed member identity and topology evidence".into()],
        ),
        RecoveryInspection::Supported(_) => {
            let mut result = observe(policy, "recover");
            result.reason_code = "recovery-not-needed";
            result.reason = "recovery state is current; recovery action is not needed".into();
            return Ok(result);
        }
        RecoveryInspection::Unsupported { .. } => (
            MetadataLossCase::AllMetadataAllDataPresent,
            vec!["supported recovery-state schema or explicit new-lineage workflow".into()],
        ),
        RecoveryInspection::MigrationRequired { .. } => (
            MetadataLossCase::AllMetadataAllDataPresent,
            vec!["explicit writable recovery migration or new-lineage workflow".into()],
        ),
        RecoveryInspection::ReconciliationRequired => {
            return Err(OperatorError::Reconciliation(
                "recovery commit state requires reconciliation before preview or apply".into(),
            ));
        }
    };
    let plan = MetadataLossPlan::for_case(case);
    let mut result = observe(policy, "recover");
    result.recovery = recovery_assessment(&recovery);
    result.recovery_plan = Some(RecoveryPlanAssessment {
        plan_id: proposal_id(policy, &recovery, &plan),
        case: plan.case().id(),
        action: plan.action().id(),
        disposition: plan.disposition().id(),
        required_evidence: plan.required_evidence().id(),
        payload_write_policy: plan.payload_write_policy().id(),
        baseline: plan.baseline().id(),
        executable: false,
        missing,
    });
    result.reason_code = "recovery-not-executable";
    result.reason =
        "declarative policy and algebraic consistency do not establish historical topology authority"
            .into();
    Ok(result)
}

/// dwv:req req.operator-recovery.recovery-preview-is-read-only-and-apply-re-establishes-authority
pub fn recover_apply(policy: &ArrayPolicy, plan_id: &str) -> Result<OperatorResult, OperatorError> {
    let preview = recover_preview(policy)?;
    let Some(preview_plan) = &preview.recovery_plan else {
        return Err(OperatorError::Invalid(
            "no current recovery proposal is available".into(),
        ));
    };
    if preview_plan.plan_id != plan_id {
        return Err(OperatorError::Invalid(
            "recovery plan is stale or does not match current facts".into(),
        ));
    }
    if !preview_plan.executable {
        return Err(OperatorError::Invalid(format!(
            "recovery is not executable: {}",
            preview_plan.missing.join("; ")
        )));
    }
    Err(OperatorError::Unsupported(
        "no current production recovery plan is authorized to create fresh state".into(),
    ))
}

pub fn baseline(
    policy: &ArrayPolicy,
    max_extents: Option<usize>,
) -> Result<OperatorResult, OperatorError> {
    if max_extents == Some(0) {
        return Err(OperatorError::Usage(
            "--max-extents must be greater than zero".into(),
        ));
    }
    let manifest = current_manifest(policy)?;
    let initial_status = assess_checksum_baseline(&manifest.snapshot);
    if matches!(
        initial_status,
        ChecksumBaselineStatus::NotRequired | ChecksumBaselineStatus::Complete { .. }
    ) {
        let mut result = observe(policy, "baseline");
        match initial_status {
            ChecksumBaselineStatus::NotRequired => {
                result.reason_code = "baseline-not-required";
                result.reason = "no mandatory checksum baseline is required".into();
            }
            _ => {
                result.reason_code = "baseline-complete";
                result.reason = "mandatory checksum baseline is complete".into();
            }
        }
        return Ok(result);
    }
    let pending = pending_checksum_baseline_extents(&manifest.snapshot)
        .map_err(|reason| OperatorError::Invalid(format!("invalid checksum baseline: {reason:?}")))?
        .ok_or_else(|| OperatorError::Invalid("checksum baseline is not required".into()))?;
    let topology =
        manifest.snapshot.active_topology.clone().ok_or_else(|| {
            OperatorError::Blocked("recovery state has no active topology".into())
        })?;
    let store_ids: Vec<_> = topology
        .assignments()
        .iter()
        .map(|assignment| assignment.store_id())
        .collect();
    let mut members = open_members(policy, &topology, store_ids, true)?;
    let mut recovery = SqliteRecoveryStore::open(&policy.recovery_path)
        .map_err(|error| OperatorError::Blocked(error.to_string()))?;
    if recovery
        .snapshot()
        .map_err(|error| OperatorError::Failed(error.to_string()))?
        != manifest.snapshot
    {
        return Err(OperatorError::Invalid(
            "recovery state changed before baseline claims were acquired".into(),
        ));
    }
    let baseline = manifest
        .snapshot
        .checksum_baseline
        .as_ref()
        .ok_or_else(|| OperatorError::Invalid("checksum baseline descriptor is missing".into()))?;
    let limit = max_extents.unwrap_or(usize::MAX);
    let mut generation = manifest.snapshot.generation;
    for (operation_index, extent) in pending.into_iter().take(limit).enumerate() {
        let member_index = topology
            .assignments()
            .iter()
            .position(|assignment| match extent.target {
                ChecksumTarget::Data { slot } => {
                    assignment.role() == MemberRole::Data && assignment.slot_id() == slot
                }
                ChecksumTarget::Parity { position } => {
                    assignment.role() == MemberRole::Parity
                        && assignment.coding_position() == position
                }
                ChecksumTarget::Q { .. } => false,
            })
            .ok_or_else(|| {
                OperatorError::Invalid("checksum extent has no current member".into())
            })?;
        let assignment = &topology.assignments()[member_index];
        let expected_member = policy
            .members
            .iter()
            .find(|member| {
                member.role == role_name(assignment.role())
                    && SlotId(member.slot_id) == assignment.slot_id()
                    && CodingPosition(member.coding_position) == assignment.coding_position()
            })
            .ok_or_else(|| {
                OperatorError::Invalid("checksum target has no declarative member".into())
            })?;
        if file_identity(&members[member_index].1) != expected_member.expected_identity {
            return Err(OperatorError::Invalid(
                "member identity changed during baseline continuation".into(),
            ));
        }
        let bytes = members[member_index]
            .1
            .read_bytes(extent.range)
            .map_err(|error| OperatorError::Failed(error.to_string()))?;
        let digest = blake3::hash(&bytes);
        let completion = members[member_index].1.flush_file(
            ChildOperationId {
                slot: OperationSlotToken::new(
                    u32::try_from(operation_index)
                        .map_err(|_| OperatorError::Failed("operation ID overflow".into()))?,
                    1,
                ),
                index: 0,
            },
            StoreWriteWatermark(0),
        );
        let fence = match (completion.disposition, completion.persistence) {
            (CompletionDisposition::Success, PersistenceEvidence::DurableByFence { fence }) => {
                fence
            }
            _ => {
                return Err(OperatorError::Failed(
                    "member flush did not produce durable fence evidence".into(),
                ));
            }
        };
        let certificate = FenceCertificate::new(
            topology.topology_epoch(),
            FenceDomain(extent.id.0 + 1),
            vec![fence],
            Vec::new(),
        )
        .with_integrity_extent(extent.id, baseline.content_generation);
        let mut transaction = recovery.begin_protocol_txn(generation, topology.topology_epoch());
        transaction.push(RecoveryMutation::RecordDataParityFence { fence: certificate });
        transaction.push(RecoveryMutation::InstallIntegrityDigest {
            record: IntegrityRecord {
                extent: extent.id,
                state: IntegrityState::Valid {
                    binding: ChecksumEvidenceBinding {
                        extent,
                        profile: baseline.profile.id,
                        set_generation: baseline.set_generation,
                        topology_epoch: baseline.topology_epoch,
                    },
                    content_generation: baseline.content_generation,
                    durable_fence: fence,
                    digest: digest.as_bytes().to_vec(),
                    verified_at: generation,
                },
            },
        });
        generation = recovery
            .commit_durable(transaction)
            .map_err(map_recovery_commit_error)?;
        let committed = recovery
            .snapshot()
            .map_err(|error| OperatorError::Failed(error.to_string()))?;
        if committed.generation != generation
            || committed.active_topology.as_ref() != Some(&topology)
            || committed.checksum_baseline.as_ref() != Some(baseline)
        {
            return Err(OperatorError::Reconciliation(
                "baseline commit did not preserve current topology and checksum authority".into(),
            ));
        }
    }
    drop(recovery);
    drop(members);
    let mut result = observe(policy, "baseline");
    match result.checksum {
        ChecksumAssessment::Complete { .. } => {
            result.reason_code = "baseline-complete";
            result.reason = "mandatory checksum baseline is complete and persisted".into();
        }
        ChecksumAssessment::Partial { .. } | ChecksumAssessment::Required { .. } => {
            result.reason_code = "baseline-partial";
            result.reason = "mandatory checksum baseline remains partial".into();
        }
        ChecksumAssessment::NotRequired => {
            result.reason_code = "baseline-not-required";
            result.reason = "no mandatory checksum baseline is required".into();
        }
        ChecksumAssessment::Invalid { .. } => {
            result.reason_code = "baseline-invalid";
            result.reason = "mandatory checksum baseline is invalid".into();
        }
    }
    Ok(result)
}

/// dwv:req req.operator-recovery.start-composes-admission-and-actual-publication
pub fn start<F>(
    policy: &ArrayPolicy,
    read_only: bool,
    device_id: Option<u32>,
    on_published: F,
) -> Result<(OperatorResult, bool), OperatorError>
where
    F: FnOnce(OperatorResult) + Send + Sync + 'static,
{
    if read_only {
        return Err(OperatorError::Unsupported(
            "read-only frontend publication is not supported".into(),
        ));
    }
    let manifest = current_manifest(policy)?;
    let (topology, _topology_epoch, _protected_length, _block_size, store_ids) =
        topology_parts(&manifest)?;
    let service_members = open_members(policy, &topology, store_ids, true)?
        .into_iter()
        .map(|(assignment, store)| {
            dwv_service::MemberBinding::new(
                &dwv_core::TopologyAssignment::new(
                    assignment.slot_id(),
                    assignment.role(),
                    assignment.coding_position(),
                    assignment.assignment_instance(),
                    assignment.assignment_generation(),
                )
                .with_evidence(assignment.evidence()),
                topology.topology_epoch(),
                assignment.store_id(),
                store,
            )
        })
        .collect();
    let recovery = SqliteRecoveryStore::open(&policy.recovery_path)
        .map_err(|error| OperatorError::Blocked(error.to_string()))?;
    let service = dwv_service::HealthyPortableService::open(
        core_topology(&topology)?,
        service_members,
        recovery,
        ServiceConfig::default(),
    )
    .map_err(service_error)?;

    if policy.frontend == FrontendPolicy::None {
        return Err(OperatorError::Unsupported(
            "portable admission succeeded, but array policy selects no frontend".into(),
        ));
    }
    if topology.profile().data_slots() != 1 || topology.profile().parity_slots() != 1 {
        return Err(OperatorError::Unsupported(
            "the current Linux publication profile requires exactly one data and one parity member"
                .into(),
        ));
    }
    let callback_policy = policy.clone();
    dwv_frontend_ublk::serve_admitted_with_publication(
        service,
        device_id.map_or(-1, |id| id as i32),
        move |ready| {
            let mut result = observe(&callback_policy, "start");
            result.lifecycle = "online";
            result.access = "read-write";
            result.start = "already-online";
            result.next_action = "none";
            result.publication = PublicationAssessment::Published {
                device_path: ready
                    .get("device_path")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("owned ublk endpoint")
                    .to_owned(),
            };
            result.reason_code = "frontend-published";
            result.reason = "portable admission succeeded and the frontend published".into();
            on_published(result);
        },
    )
    .map_err(frontend_error)?;
    let mut result = observe(policy, "start");
    result.reason_code = "frontend-shut-down";
    result.reason = "frontend published and later shut down cleanly".into();
    Ok((result, true))
}

fn current_manifest(policy: &ArrayPolicy) -> Result<RecoveryManifest, OperatorError> {
    match SqliteRecoveryStore::inspect(&policy.recovery_path) {
        RecoveryInspection::Supported(manifest) => {
            let expected = policy_topology(policy)?;
            if manifest.snapshot.active_topology.as_ref() != Some(&expected) {
                return Err(OperatorError::Invalid(
                    "current recovery topology does not match the declarative array policy".into(),
                ));
            }
            Ok(*manifest)
        }
        inspection => Err(OperatorError::Blocked(format!(
            "recovery authority is {}; no current semantic state is available",
            inspection.id()
        ))),
    }
}

fn topology_parts(
    manifest: &RecoveryManifest,
) -> Result<
    (
        RecoveryTopologySnapshot,
        TopologyEpoch,
        u64,
        u32,
        Vec<StoreId>,
    ),
    OperatorError,
> {
    let topology =
        manifest.snapshot.active_topology.clone().ok_or_else(|| {
            OperatorError::Blocked("recovery state has no active topology".into())
        })?;
    Ok((
        topology.clone(),
        topology.topology_epoch(),
        topology.geometry().protected_length(),
        topology.geometry().logical_block_size(),
        topology
            .assignments()
            .iter()
            .map(|assignment| assignment.store_id())
            .collect(),
    ))
}

fn policy_topology(policy: &ArrayPolicy) -> Result<RecoveryTopologySnapshot, OperatorError> {
    let data_slots = policy
        .members
        .iter()
        .filter(|member| member.role == "data")
        .count();
    let parity_slots = policy
        .members
        .iter()
        .filter(|member| member.role == "parity")
        .count();
    let data_slots = u16::try_from(data_slots)
        .map_err(|_| OperatorError::Invalid("too many data members in array policy".into()))?;
    let parity_slots = u16::try_from(parity_slots)
        .map_err(|_| OperatorError::Invalid("too many parity members in array policy".into()))?;
    if parity_slots != 1 {
        return Err(OperatorError::Unsupported(
            "the supported recovery profile requires exactly one parity member".into(),
        ));
    }
    for (index, member) in policy.members.iter().enumerate() {
        if policy.members[..index].iter().any(|prior| {
            prior.path == member.path
                || prior.expected_identity == member.expected_identity
                || prior.store_id == member.store_id
        }) {
            return Err(OperatorError::Invalid(
                "array policy contains an aliased path, identity, or store assignment".into(),
            ));
        }
    }
    let topology = TopologySnapshot::new(
        ArrayId(policy.array_id),
        TopologyEpoch(policy.topology_epoch),
        CodingProfile::new(data_slots, parity_slots)
            .map_err(|error| OperatorError::Invalid(format!("{error:?}")))?,
        ProtectedGeometry::new(policy.protected_length, policy.logical_block_size)
            .map_err(|error| OperatorError::Invalid(format!("{error:?}")))?,
        policy
            .members
            .iter()
            .map(|member| {
                TopologyAssignment::new(
                    SlotId(member.slot_id),
                    if member.role == "data" {
                        MemberRole::Data
                    } else {
                        MemberRole::Parity
                    },
                    CodingPosition(member.coding_position),
                    AssignmentInstanceId(member.assignment_instance),
                    AssignmentGeneration(member.assignment_generation),
                )
            })
            .collect(),
    )
    .map_err(|error| OperatorError::Invalid(error.to_string()))?;
    RecoveryTopologySnapshot::from_core(
        topology,
        policy
            .members
            .iter()
            .map(|member| StoreId(member.store_id))
            .collect(),
    )
    .map_err(|error| OperatorError::Invalid(error.to_string()))
}

fn core_topology(topology: &RecoveryTopologySnapshot) -> Result<TopologySnapshot, OperatorError> {
    TopologySnapshot::new(
        topology.array_id(),
        topology.topology_epoch(),
        topology.profile(),
        topology.geometry(),
        topology
            .assignments()
            .iter()
            .map(|assignment| {
                TopologyAssignment::new(
                    assignment.slot_id(),
                    assignment.role(),
                    assignment.coding_position(),
                    assignment.assignment_instance(),
                    assignment.assignment_generation(),
                )
                .with_evidence(assignment.evidence())
            })
            .collect(),
    )
    .map_err(|error| OperatorError::Invalid(error.to_string()))
}

fn open_members(
    policy: &ArrayPolicy,
    topology: &RecoveryTopologySnapshot,
    store_ids: Vec<StoreId>,
    writable: bool,
) -> Result<Vec<(dwv_recovery::StoreAssignment, FileStore)>, OperatorError> {
    let mut opened = Vec::new();
    let mut observed_identities = Vec::with_capacity(topology.assignments().len());
    for (assignment, store_id) in topology.assignments().iter().cloned().zip(store_ids) {
        let member = policy
            .members
            .iter()
            .find(|member| {
                member.role == role_name(assignment.role())
                    && CodingPosition(member.coding_position) == assignment.coding_position()
                    && SlotId(member.slot_id) == assignment.slot_id()
            })
            .ok_or_else(|| {
                OperatorError::Invalid(format!(
                    "array policy has no candidate for {} coding position {}",
                    role_name(assignment.role()),
                    assignment.coding_position().0
                ))
            })?;
        let store = FileStore::open(
            FileStoreConfig::new(
                &member.path,
                topology.geometry().protected_length(),
                topology.geometry().logical_block_size(),
            )
            .maximum_transfer(topology.geometry().protected_length())
            .writable(writable)
            .sync_mode(FileSyncMode::SyncAll)
            .store_id(store_id)
            .topology_epoch(topology.topology_epoch()),
        )
        .map_err(|error| OperatorError::Blocked(error.to_string()))?;
        let observed = file_identity(&store);
        if observed != member.expected_identity {
            return Err(OperatorError::Invalid(format!(
                "member identity changed for {}",
                member.path.display()
            )));
        }
        if observed_identities.contains(&observed) {
            return Err(OperatorError::Invalid(
                "multiple assignments resolve to the same observed member identity".into(),
            ));
        }
        observed_identities.push(observed);
        if opened.iter().any(
            |(prior_assignment, _): &(dwv_recovery::StoreAssignment, FileStore)| {
                prior_assignment.store_id() == store_id
            },
        ) {
            return Err(OperatorError::Invalid(
                "array policy aliases one store to multiple assignments".into(),
            ));
        }
        opened.push((assignment, store));
    }
    Ok(opened)
}
fn run_exhaustive(
    policy: &ArrayPolicy,
    topology: &RecoveryTopologySnapshot,
    writable_claims: bool,
    snapshot: Option<&RecoverySnapshot>,
) -> Result<ExhaustiveRun, OperatorError> {
    let store_ids: Vec<_> = topology
        .assignments()
        .iter()
        .map(|assignment| assignment.store_id())
        .collect();
    let mut member_files = open_members(policy, topology, store_ids.clone(), writable_claims)?;
    let mut data = Vec::new();
    let mut parity = None;
    for (assignment, store) in member_files.drain(..) {
        let verification_store = RebuildSource::new(assignment.assignment_instance(), store);
        if assignment.role() == MemberRole::Data {
            data.push(verification_store);
        } else if parity.replace(verification_store).is_some() {
            return Err(OperatorError::Invalid(
                "topology contains multiple parity members for the narrow production path".into(),
            ));
        }
    }
    let mut parity =
        parity.ok_or_else(|| OperatorError::Invalid("topology has no parity member".into()))?;
    let protected_length = topology.geometry().protected_length();
    let config = ScanConfig::new(
        Geometry::new(
            vec![protected_length; usize::from(topology.profile().data_slots())],
            protected_length,
        )
        .map_err(|error| OperatorError::Invalid(error.to_string()))?,
        dwv_recovery::BLAKE3_256_PROFILE.extent_size,
    )
    .map_err(|error| OperatorError::Invalid(error.to_string()))?;
    let region_count = config
        .region_count()
        .map_err(|error| OperatorError::Invalid(error.to_string()))?;
    let evidence = match snapshot {
        Some(snapshot) => checksum_evidence(topology, snapshot, region_count)?,
        None => absent_evidence(topology, region_count)?,
    };
    let report = verify_exhaustive(&mut data, &mut parity, &config, &evidence)
        .map_err(|error| OperatorError::Failed(error.to_string()))?;
    Ok(ExhaustiveRun {
        report,
        _data: data,
        _parity: parity,
    })
}

fn observe_member(policy: &ArrayPolicy, member: &MemberPolicy) -> ObservedMember {
    let store = FileStore::open(
        FileStoreConfig::new(
            &member.path,
            policy.protected_length,
            policy.logical_block_size,
        )
        .maximum_transfer(policy.protected_length)
        .writable(false)
        .sync_mode(FileSyncMode::CallerFlush)
        .store_id(StoreId(member.store_id))
        .topology_epoch(TopologyEpoch(policy.topology_epoch)),
    );
    match store {
        Ok(store) => {
            let observed = file_identity(&store);
            ObservedMember {
                policy: member.clone(),
                path: member.path.clone(),
                identity: Some(observed),
                status: if observed == member.expected_identity {
                    "recognized"
                } else {
                    "identity-changed"
                },
            }
        }
        Err(_) => ObservedMember {
            policy: member.clone(),
            path: member.path.clone(),
            identity: None,
            status: "unreadable",
        },
    }
}

fn usable_topology_from_members(policy: &ArrayPolicy) -> Result<bool, OperatorError> {
    policy_topology(policy)?;
    Ok(policy
        .members
        .iter()
        .map(|member| observe_member(policy, member))
        .all(|member| member.status == "recognized"))
}

fn absent_evidence(
    topology: &RecoveryTopologySnapshot,
    region_count: usize,
) -> Result<ChecksumEvidence, OperatorError> {
    if region_count == 0 || region_count > MAX_EXTENTS {
        return Err(OperatorError::Invalid(
            "checksum extent count exceeds the bounded profile".into(),
        ));
    }
    let data = vec![
        vec![DigestEvidence::Absent; region_count];
        usize::from(topology.profile().data_slots())
    ];
    Ok(ChecksumEvidence::new(
        data,
        vec![DigestEvidence::Absent; region_count],
    ))
}

fn checksum_evidence(
    topology: &RecoveryTopologySnapshot,
    snapshot: &RecoverySnapshot,
    region_count: usize,
) -> Result<ChecksumEvidence, OperatorError> {
    if matches!(
        assess_checksum_baseline(snapshot),
        ChecksumBaselineStatus::NotRequired | ChecksumBaselineStatus::Invalid(_)
    ) {
        return absent_evidence(topology, region_count);
    }
    let baseline = snapshot
        .checksum_baseline
        .as_ref()
        .ok_or_else(|| OperatorError::Invalid("checksum baseline descriptor is missing".into()))?;
    let evidence_for = |target: ChecksumTarget| {
        let mut extents: Vec<_> = baseline
            .expected_extents
            .iter()
            .filter(|extent| extent.target == target)
            .collect();
        extents.sort_unstable_by_key(|extent| extent.range.offset);
        extents
            .into_iter()
            .map(|extent| {
                snapshot
                    .integrity_records
                    .iter()
                    .find(|record| record.extent == extent.id)
                    .map_or(DigestEvidence::Absent, |record| match &record.state {
                        IntegrityState::Valid { digest, .. } => digest
                            .as_slice()
                            .try_into()
                            .map(DigestEvidence::Current)
                            .unwrap_or(DigestEvidence::Conflicting),
                        IntegrityState::Stale { .. } => DigestEvidence::Stale,
                        IntegrityState::Absent => DigestEvidence::Absent,
                    })
            })
            .collect::<Vec<_>>()
    };
    let data = topology
        .assignments()
        .iter()
        .filter(|assignment| assignment.role() == MemberRole::Data)
        .map(|assignment| evidence_for(ChecksumTarget::data(assignment.slot_id())))
        .collect();
    let parity_assignment = topology
        .assignments()
        .iter()
        .find(|assignment| assignment.role() == MemberRole::Parity)
        .ok_or_else(|| OperatorError::Invalid("topology has no parity member".into()))?;
    let parity = evidence_for(ChecksumTarget::parity(parity_assignment.coding_position()));
    Ok(ChecksumEvidence::new(data, parity))
}

fn recovery_assessment(inspection: &RecoveryInspection) -> RecoveryAssessment {
    match inspection {
        RecoveryInspection::Absent => RecoveryAssessment::Absent,
        RecoveryInspection::Supported(manifest) => RecoveryAssessment::Supported {
            generation: manifest.snapshot.generation.0,
            topology_epoch: manifest.snapshot.topology_epoch.0,
        },
        RecoveryInspection::CorruptOrUnreadable { .. } => RecoveryAssessment::CorruptOrUnreadable,
        RecoveryInspection::Unsupported { layer, version, .. } => RecoveryAssessment::Unsupported {
            layer: format!("{layer:?}"),
            version: *version,
        },
        RecoveryInspection::MigrationRequired {
            layer, from, to, ..
        } => RecoveryAssessment::MigrationRequired {
            layer: format!("{layer:?}"),
            from: *from,
            to: *to,
        },
        RecoveryInspection::ReconciliationRequired => RecoveryAssessment::ReconciliationRequired,
    }
}

fn topology_assessment(topology: &RecoveryTopologySnapshot) -> TopologyAssessment {
    TopologyAssessment {
        array_id: crate::array::hex(&topology.array_id().as_bytes()),
        topology_epoch: topology.topology_epoch().0,
        data_slots: topology.profile().data_slots(),
        parity_slots: topology.profile().parity_slots(),
        protected_length: topology.geometry().protected_length(),
        logical_block_size: topology.geometry().logical_block_size(),
    }
}

fn member_assessment(member: &ObservedMember) -> MemberAssessment {
    MemberAssessment {
        path: member.path.clone(),
        role: member.policy.role.clone(),
        expected_identity: crate::array::hex(&member.policy.expected_identity),
        observed_identity: member.identity.map(|identity| crate::array::hex(&identity)),
        status: member.status,
    }
}

fn checksum_assessment(manifest: &RecoveryManifest) -> ChecksumAssessment {
    checksum_status_assessment(assess_checksum_baseline(&manifest.snapshot))
}

fn checksum_status_assessment(status: ChecksumBaselineStatus) -> ChecksumAssessment {
    match status {
        ChecksumBaselineStatus::NotRequired => ChecksumAssessment::NotRequired,
        ChecksumBaselineStatus::Required { total } => ChecksumAssessment::Required { total },
        ChecksumBaselineStatus::Partial { valid, total } => {
            ChecksumAssessment::Partial { valid, total }
        }
        ChecksumBaselineStatus::Complete { total } => ChecksumAssessment::Complete { total },
        ChecksumBaselineStatus::Invalid(reason) => ChecksumAssessment::Invalid {
            reason: format!("{reason:?}"),
        },
    }
}

fn unavailable_verification(
    policy: &ArrayPolicy,
    members: &[MemberAssessment],
) -> VerificationAssessment {
    let disposition = if members.iter().any(|member| member.status == "unreadable") {
        "unreadable-member"
    } else {
        "unresolved-member-identity"
    };
    let data_evidence: Vec<_> = members
        .iter()
        .filter(|member| member.role == "data")
        .map(|member| member.status.to_owned())
        .collect();
    let parity_evidence = members
        .iter()
        .find(|member| member.role == "parity")
        .map_or("unavailable", |member| member.status)
        .to_owned();
    let extent_size = dwv_recovery::BLAKE3_256_PROFILE.extent_size;
    let region_count = policy.protected_length.div_ceil(extent_size);
    VerificationAssessment {
        mode: "exhaustive",
        complete: false,
        matching_regions: 0,
        regions: (0..region_count)
            .map(|index| {
                let offset = index * extent_size;
                RegionAssessment {
                    offset,
                    length: extent_size.min(policy.protected_length - offset),
                    disposition: disposition.into(),
                    data_evidence: data_evidence.clone(),
                    parity_evidence: parity_evidence.clone(),
                }
            })
            .collect(),
    }
}

fn publication(policy: &ArrayPolicy) -> PublicationAssessment {
    match policy.frontend {
        FrontendPolicy::None => PublicationAssessment::NotPublished,
        FrontendPolicy::LinuxUblk => {
            let (array_identity, identity) = match current_publication_identity(policy) {
                Ok(Some(identity)) => identity,
                Ok(None) => return PublicationAssessment::NotPublished,
                Err(reason) => {
                    return PublicationAssessment::ReconciliationRequired { reason };
                }
            };
            match dwv_frontend_ublk::live_admitted_publication(array_identity, identity) {
                Ok(Some(value)) => PublicationAssessment::Published {
                    device_path: value
                        .get("device_path")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("owned ublk endpoint")
                        .to_owned(),
                },
                Ok(None) => PublicationAssessment::NotPublished,
                Err(error)
                    if error.terminal()
                        == dwv_frontend_ublk::TerminalResult::ReconciliationRequired =>
                {
                    PublicationAssessment::ReconciliationRequired {
                        reason: error.to_string(),
                    }
                }
                Err(error) => PublicationAssessment::Unsupported {
                    reason: error.to_string(),
                },
            }
        }
    }
}

fn current_publication_identity(
    policy: &ArrayPolicy,
) -> Result<Option<(ArrayId, dwv_service::PublicationIdentity)>, String> {
    let RecoveryInspection::Supported(manifest) =
        SqliteRecoveryStore::inspect(&policy.recovery_path)
    else {
        return Ok(None);
    };
    let Some(topology) = manifest.snapshot.active_topology.as_ref() else {
        return Ok(None);
    };
    let mut identities = Vec::with_capacity(topology.assignments().len());
    for assignment in topology.assignments() {
        let member = policy
            .members
            .iter()
            .find(|member| member.store_id == assignment.store_id().0)
            .ok_or_else(|| {
                format!(
                    "current topology store {:?} has no observed member",
                    assignment.store_id()
                )
            })?;
        let identity = observe_file_identity(&member.path)
            .map_err(|error| format!("cannot establish published member identity: {error}"))?;
        identities.push((assignment.store_id(), identity));
    }
    dwv_service::publication_identity(topology, &identities)
        .map(|identity| Some((topology.array_id(), identity)))
        .map_err(|error| error.to_string())
}

fn map_recovery_commit_error(error: RecoveryError) -> OperatorError {
    match error {
        RecoveryError::CommitNotDurable(
            RecoveryCommitObservation::Lost | RecoveryCommitObservation::Corrupt,
        ) => OperatorError::Reconciliation(error.to_string()),
        _ => OperatorError::Failed(error.to_string()),
    }
}

fn verification_assessment(report: &dwv_verify::VerificationReport) -> VerificationAssessment {
    VerificationAssessment {
        mode: match report.mode() {
            ScanMode::Exhaustive => "exhaustive",
            ScanMode::Sampled => "sampled",
        },
        complete: report.mode() == ScanMode::Exhaustive
            && report
                .regions()
                .iter()
                .all(|region| !matches!(region.disposition, RegionDisposition::Incomplete { .. })),
        matching_regions: report.matching_regions(),
        regions: report
            .regions()
            .iter()
            .map(|region| RegionAssessment {
                offset: region.range.offset,
                length: region.range.length,
                disposition: match region.disposition {
                    RegionDisposition::Match { evidence_complete } => {
                        if evidence_complete {
                            "match-with-current-evidence".into()
                        } else {
                            "match-without-current-evidence".into()
                        }
                    }
                    RegionDisposition::Mismatch(MismatchClass::ParityIdentified) => {
                        "known-parity-mismatch".into()
                    }
                    RegionDisposition::Mismatch(MismatchClass::DataIdentified { slot }) => {
                        format!("known-data-mismatch:{slot}")
                    }
                    RegionDisposition::Mismatch(MismatchClass::Ambiguous) => {
                        "unresolved-conflict".into()
                    }
                    RegionDisposition::Mismatch(MismatchClass::EvidenceConflict) => {
                        "conflicting-evidence".into()
                    }
                    RegionDisposition::Incomplete { member } => match member {
                        MemberRef::Data(slot) => format!("unreadable-data:{slot}"),
                        MemberRef::Parity => "unreadable-parity".into(),
                    },
                },
                data_evidence: region
                    .data_evidence
                    .iter()
                    .map(|evidence| format!("{evidence:?}"))
                    .collect(),
                parity_evidence: format!("{:?}", region.parity_evidence),
            })
            .collect(),
    }
}

fn proposal_id(
    policy: &ArrayPolicy,
    recovery: &RecoveryInspection,
    plan: &MetadataLossPlan,
) -> String {
    crate::array::hex(
        blake3::hash(canonical_proposal(policy, recovery, plan).as_bytes()).as_bytes(),
    )
}

fn canonical_proposal(
    policy: &ArrayPolicy,
    recovery: &RecoveryInspection,
    plan: &MetadataLossPlan,
) -> String {
    let observed: Vec<_> = policy
        .members
        .iter()
        .map(|member| {
            let member = observe_member(policy, member);
            serde_json::json!({
                "path": member.path,
                "identity": member.identity.map(|identity| crate::array::hex(&identity)),
                "status": member.status,
            })
        })
        .collect();
    serde_json::to_string(&serde_json::json!({
        "schema": "dwv.recovery-proposal.v1",
        "array_id": crate::array::hex(&policy.array_id),
        "topology_epoch": policy.topology_epoch,
        "protected_length": policy.protected_length,
        "logical_block_size": policy.logical_block_size,
        "recovery": policy.recovery_path,
        "recovery_fingerprint": artifact_fingerprint(&policy.recovery_path),
        "members": policy.members,
        "observed_members": observed,
        "inspection": recovery.id(),
        "case": plan.case().id(),
        "action": plan.action().id(),
        "baseline": plan.baseline().id(),
    }))
    .expect("recovery proposal serialization cannot fail")
}

fn artifact_fingerprint(path: &Path) -> String {
    let Ok(mut file) = std::fs::File::open(path) else {
        return if path.exists() {
            "unreadable".into()
        } else {
            "absent".into()
        };
    };
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => return hasher.finalize().to_hex().to_string(),
            Ok(length) => {
                hasher.update(&buffer[..length]);
            }
            Err(_) => return "unreadable".into(),
        }
    }
}

fn file_identity(store: &FileStore) -> [u8; 16] {
    store
        .capabilities_report()
        .identity
        .observations
        .first()
        .map(|observation| observation.fingerprint)
        .unwrap_or([0; 16])
}

fn frontend_error(error: dwv_frontend_ublk::AdapterError) -> OperatorError {
    match error.terminal() {
        dwv_frontend_ublk::TerminalResult::Invalid => OperatorError::Invalid(error.to_string()),
        dwv_frontend_ublk::TerminalResult::Unsupported => {
            OperatorError::Unsupported(error.to_string())
        }
        dwv_frontend_ublk::TerminalResult::ResourceExhausted => {
            OperatorError::Blocked(error.to_string())
        }
        dwv_frontend_ublk::TerminalResult::ReconciliationRequired => {
            OperatorError::Reconciliation(error.to_string())
        }
        dwv_frontend_ublk::TerminalResult::Io => OperatorError::Failed(error.to_string()),
        dwv_frontend_ublk::TerminalResult::Success => {
            OperatorError::Failed("frontend returned an invalid success error".into())
        }
    }
}

fn role_name(role: MemberRole) -> &'static str {
    match role {
        MemberRole::Data => "data",
        MemberRole::Parity => "parity",
    }
}

fn service_error(error: ServiceError) -> OperatorError {
    match error {
        ServiceError::NotServing | ServiceError::Blocked(_) => {
            OperatorError::Blocked(error.to_string())
        }
        ServiceError::Invalid { class, detail, .. } | ServiceError::Io { class, detail, .. } => {
            match class {
                dwv_service::FailureClass::Capability => OperatorError::Unsupported(detail),
                dwv_service::FailureClass::ReconciliationRequired => {
                    OperatorError::Reconciliation(detail)
                }
                _ => OperatorError::Invalid(detail),
            }
        }
        ServiceError::Terminalization { .. } => OperatorError::Reconciliation(error.to_string()),
        ServiceError::IncompleteRead { .. } => OperatorError::Invalid(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// dwv:req req.operator-recovery.production-assessment-is-observational-and-multidimensional
    #[test]
    fn uncertain_recovery_commit_is_not_reported_as_definite_failure() {
        for observation in [
            RecoveryCommitObservation::Lost,
            RecoveryCommitObservation::Corrupt,
        ] {
            assert!(matches!(
                map_recovery_commit_error(RecoveryError::CommitNotDurable(observation)),
                OperatorError::Reconciliation(_)
            ));
        }
        assert!(matches!(
            map_recovery_commit_error(RecoveryError::CommitNotDurable(
                RecoveryCommitObservation::Rejected
            )),
            OperatorError::Failed(_)
        ));
    }
    #[test]
    fn authority_coverage_merges_ranges_and_checks_totals() {
        let coverage = coverage_for_ranges(
            "parity".into(),
            ProtectionBasis::Current,
            &[
                AuthorityRange {
                    offset: 0,
                    length: 4,
                },
                AuthorityRange {
                    offset: 4,
                    length: 8,
                },
                AuthorityRange {
                    offset: 32,
                    length: 8,
                },
            ],
        )
        .unwrap();
        assert_eq!(coverage.range_count, 2);
        assert_eq!(coverage.bytes, 20);
        assert_eq!(
            coverage.exact_ranges,
            vec![
                AuthorityRange {
                    offset: 0,
                    length: 12,
                },
                AuthorityRange {
                    offset: 32,
                    length: 8,
                },
            ]
        );
    }

    #[test]
    fn authority_coverage_bounds_exception_detail() {
        let ranges: Vec<_> = (0..MAX_AUTHORITY_DETAIL_RANGES + 1)
            .map(|index| AuthorityRange {
                offset: (index as u64) * 2,
                length: 1,
            })
            .collect();
        let coverage =
            coverage_for_ranges("parity".into(), ProtectionBasis::Indeterminate, &ranges).unwrap();
        assert_eq!(coverage.range_count, MAX_AUTHORITY_DETAIL_RANGES + 1);
        assert_eq!(coverage.exact_ranges.len(), MAX_AUTHORITY_DETAIL_RANGES);
        assert!(coverage.detail_truncated);
    }
}
