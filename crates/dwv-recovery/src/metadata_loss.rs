//! Conservative metadata-loss planning and fresh-state authorization.
//!
//! Plans are portable: they carry no paths, payload bytes, storage-engine layout, or frontend state.

use crate::{
    BLAKE3_256_PROFILE, ChecksumSetGeneration, MemoryRecoveryStore, RecoveryError,
    RecoveryGeneration, RecoveryManifest, RecoveryStateStore, RecoveryStoreHealth,
    TopologySnapshot, new_checksum_baseline,
};
use dwv_core::{ArrayId, TopologyEpoch};
use std::fmt;

pub const METADATA_LOSS_MATRIX_VERSION: u16 = 3;

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub enum MetadataLossCase {
    AllDataSingleParityCertified,
    AllDataSingleParityUncertified,
    AllDataDualParityCertified,
    AllDataDualParityUncertified,
    AllDataParityMissing,
    OneDataSingleParityCertified,
    OneDataSingleParityUncertified,
    OneDataDualParityCertified,
    OneDataDualParityUncertified,
    TwoDataDualParityCertified,
    TwoDataDualParityCodingLost,
    AllMetadataAllDataPresent,
    TopologyAmbiguous,
    ParityIdentityAmbiguous,
    RecoveryBackupValid,
    RecoveryReplicasDisagree,
    ChecksumEvidenceSurvives,
    ChecksumEvidenceUnavailable,
}

impl MetadataLossCase {
    pub const ALL: [Self; 18] = [
        Self::AllDataSingleParityCertified,
        Self::AllDataSingleParityUncertified,
        Self::AllDataDualParityCertified,
        Self::AllDataDualParityUncertified,
        Self::AllDataParityMissing,
        Self::OneDataSingleParityCertified,
        Self::OneDataSingleParityUncertified,
        Self::OneDataDualParityCertified,
        Self::OneDataDualParityUncertified,
        Self::TwoDataDualParityCertified,
        Self::TwoDataDualParityCodingLost,
        Self::AllMetadataAllDataPresent,
        Self::TopologyAmbiguous,
        Self::ParityIdentityAmbiguous,
        Self::RecoveryBackupValid,
        Self::RecoveryReplicasDisagree,
        Self::ChecksumEvidenceSurvives,
        Self::ChecksumEvidenceUnavailable,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::AllDataSingleParityCertified => "all-data-p-certified-clean",
            Self::AllDataSingleParityUncertified => "all-data-p-uncertified",
            Self::AllDataDualParityCertified => "all-data-pq-certified-clean",
            Self::AllDataDualParityUncertified => "all-data-pq-uncertified",
            Self::AllDataParityMissing => "all-data-parity-missing",
            Self::OneDataSingleParityCertified => "one-data-missing-p-certified-clean",
            Self::OneDataSingleParityUncertified => "one-data-missing-p-uncertified",
            Self::OneDataDualParityCertified => "one-data-missing-pq-certified-clean",
            Self::OneDataDualParityUncertified => "one-data-missing-pq-uncertified",
            Self::TwoDataDualParityCertified => "two-data-missing-pq-certified-clean",
            Self::TwoDataDualParityCodingLost => "two-data-missing-pq-coding-lost",
            Self::AllMetadataAllDataPresent => "all-metadata-lost-all-data-present",
            Self::TopologyAmbiguous => "topology-identifiers-ambiguous",
            Self::ParityIdentityAmbiguous => "parity-candidate-ambiguous",
            Self::RecoveryBackupValid => "recovery-backup-valid",
            Self::RecoveryReplicasDisagree => "recovery-replicas-disagree",
            Self::ChecksumEvidenceSurvives => "checksum-evidence-survives",
            Self::ChecksumEvidenceUnavailable => "checksum-evidence-unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum MetadataLossAction {
    RecreateFromCertifiedEnvelope,
    ExhaustiveVerifyThenRecreate,
    RebuildParityToSeparateTarget,
    ReadOnlyDecodeToReplacement,
    RefuseAutomaticDecode,
    RefuseAuthorityUnavailable,
    RefuseAmbiguousTopology,
    RefuseAmbiguousParity,
    RestoreValidatedBackup,
    ReconcileReplicasConservatively,
    EvidenceGatedSelectiveRepair,
    RequireDataAuthoritativeRebaseline,
}

impl MetadataLossAction {
    pub const fn id(self) -> &'static str {
        match self {
            Self::RecreateFromCertifiedEnvelope => "recreate-from-certified-envelope",
            Self::ExhaustiveVerifyThenRecreate => "exhaustive-verify-then-recreate",
            Self::RebuildParityToSeparateTarget => "rebuild-parity-to-separate-target",
            Self::ReadOnlyDecodeToReplacement => "read-only-decode-to-replacement",
            Self::RefuseAutomaticDecode => "refuse-automatic-decode",
            Self::RefuseAuthorityUnavailable => "refuse-authority-unavailable",
            Self::RefuseAmbiguousTopology => "refuse-ambiguous-topology",
            Self::RefuseAmbiguousParity => "refuse-ambiguous-parity",
            Self::RestoreValidatedBackup => "restore-validated-backup",
            Self::ReconcileReplicasConservatively => "reconcile-replicas-conservatively",
            Self::EvidenceGatedSelectiveRepair => "evidence-gated-selective-repair",
            Self::RequireDataAuthoritativeRebaseline => "require-data-authoritative-rebaseline",
        }
    }

    const fn refuses_automatic_authorization(self) -> bool {
        matches!(
            self,
            Self::RefuseAutomaticDecode
                | Self::RefuseAuthorityUnavailable
                | Self::RefuseAmbiguousTopology
                | Self::RefuseAmbiguousParity
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataLossDisposition {
    CertifiedFastPath,
    ExhaustiveVerification,
    RebuildParity,
    ReadOnlyRecovery,
    Refused,
    RestoreBackup,
    ReconcileReplicas,
    EvidenceGatedRepair,
    ExplicitRebaseline,
}

impl MetadataLossDisposition {
    pub const fn id(self) -> &'static str {
        match self {
            Self::CertifiedFastPath => "certified-fast-path",
            Self::ExhaustiveVerification => "exhaustive-verification",
            Self::RebuildParity => "rebuild-parity",
            Self::ReadOnlyRecovery => "read-only-recovery",
            Self::Refused => "refused",
            Self::RestoreBackup => "restore-backup",
            Self::ReconcileReplicas => "reconcile-replicas",
            Self::EvidenceGatedRepair => "evidence-gated-repair",
            Self::ExplicitRebaseline => "explicit-rebaseline",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceRequirement {
    ValidatedCertificateReceipt,
    ExhaustiveEquationScan,
    ExplicitDataAuthority,
    CustodyLossAuthorityModel,
    UnambiguousIdentity,
    HistoricalCodingPositions,
    ValidatedRecoveryBackup,
    ReconciledRecoveryReplicas,
    CurrentChecksumEvidence,
}

impl EvidenceRequirement {
    pub const fn id(self) -> &'static str {
        match self {
            Self::ValidatedCertificateReceipt => "validated-certificate-receipt",
            Self::ExhaustiveEquationScan => "complete-exhaustive-equation-scan",
            Self::ExplicitDataAuthority => "explicit-data-authoritative-rebaseline",
            Self::CustodyLossAuthorityModel => "canonical-custody-loss-authority-model",
            Self::UnambiguousIdentity => "unambiguous-identity-and-topology",
            Self::HistoricalCodingPositions => "exact-historical-coding-positions",
            Self::ValidatedRecoveryBackup => "validated-recovery-backup",
            Self::ReconciledRecoveryReplicas => "reconciled-recovery-replicas",
            Self::CurrentChecksumEvidence => "current-independent-checksum-evidence",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadWritePolicy {
    None,
    SeparateParityTargetOnly,
    SeparateReplacementTargetOnly,
    NoAutomaticWrite,
}

impl PayloadWritePolicy {
    pub const fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SeparateParityTargetOnly => "separate-parity-target-only",
            Self::SeparateReplacementTargetOnly => "separate-replacement-target-only",
            Self::NoAutomaticWrite => "no-automatic-write",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum BaselineDisposition {
    NewChecksumBaselineRequired,
    NewParityAndChecksumBaselineRequired,
    RetainValidatedHistoricalEvidence,
    NotEstablished,
}

impl BaselineDisposition {
    pub const fn id(self) -> &'static str {
        match self {
            Self::NewChecksumBaselineRequired => "new-checksum-baseline-required",
            Self::NewParityAndChecksumBaselineRequired => {
                "new-parity-and-checksum-baseline-required"
            }
            Self::RetainValidatedHistoricalEvidence => "retain-validated-historical-evidence",
            Self::NotEstablished => "not-established",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum MetadataLossVerification {
    ExhaustiveMatches,
    IdentifiedRepairPending,
    ExhaustiveIdentifiedRepairs,
    ExplicitDataAuthoritativeRebaseline,
    AmbiguousMismatch,
    IncompleteScan,
    ValidatedBackup,
    ReconciledReplicas,
    ConflictingReplicas,
}

impl MetadataLossVerification {
    pub const fn id(self) -> &'static str {
        match self {
            Self::ExhaustiveMatches => "exhaustive-matches",
            Self::IdentifiedRepairPending => "identified-repair-pending",
            Self::ExhaustiveIdentifiedRepairs => "exhaustive-identified-repairs",
            Self::ExplicitDataAuthoritativeRebaseline => "explicit-data-authoritative-rebaseline",
            Self::AmbiguousMismatch => "ambiguous-mismatch",
            Self::IncompleteScan => "incomplete-scan",
            Self::ValidatedBackup => "validated-backup",
            Self::ReconciledReplicas => "reconciled-replicas",
            Self::ConflictingReplicas => "conflicting-replicas",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataLossPlan {
    case: MetadataLossCase,
    action: MetadataLossAction,
    disposition: MetadataLossDisposition,
    required_evidence: EvidenceRequirement,
    payload_write_policy: PayloadWritePolicy,
    baseline: BaselineDisposition,
    requires_exhaustive_verification: bool,
    requires_certificate_receipt: bool,
    requires_operator_confirmation: bool,
    creates_fresh_state: bool,
}

/// dwv:req req.metadata-loss-recovery.identity-and-topology-ambiguity-fails-closed
impl MetadataLossPlan {
    pub fn for_case(case: MetadataLossCase) -> Self {
        use BaselineDisposition as Baseline;
        use EvidenceRequirement as Evidence;
        use MetadataLossAction as Action;
        use MetadataLossDisposition as Disposition;
        use PayloadWritePolicy as Writes;

        match case {
            MetadataLossCase::AllDataSingleParityCertified => Self::new(
                case,
                Action::RecreateFromCertifiedEnvelope,
                Disposition::CertifiedFastPath,
                Evidence::ValidatedCertificateReceipt,
                Writes::None,
                Baseline::NewChecksumBaselineRequired,
                false,
                true,
                false,
                false,
            ),
            MetadataLossCase::AllDataDualParityCertified => Self::new(
                case,
                Action::RecreateFromCertifiedEnvelope,
                Disposition::CertifiedFastPath,
                Evidence::ValidatedCertificateReceipt,
                Writes::None,
                Baseline::NewChecksumBaselineRequired,
                false,
                true,
                false,
                false,
            ),
            MetadataLossCase::AllDataSingleParityUncertified
            | MetadataLossCase::AllDataDualParityUncertified => Self::new(
                case,
                Action::RefuseAuthorityUnavailable,
                Disposition::Refused,
                Evidence::CustodyLossAuthorityModel,
                Writes::NoAutomaticWrite,
                Baseline::NotEstablished,
                false,
                false,
                false,
                false,
            ),
            MetadataLossCase::AllDataParityMissing => Self::new(
                case,
                Action::RebuildParityToSeparateTarget,
                Disposition::RebuildParity,
                Evidence::ExplicitDataAuthority,
                Writes::SeparateParityTargetOnly,
                Baseline::NewParityAndChecksumBaselineRequired,
                true,
                false,
                true,
                false,
            ),
            MetadataLossCase::OneDataSingleParityCertified
            | MetadataLossCase::OneDataDualParityCertified
            | MetadataLossCase::TwoDataDualParityCertified => Self::new(
                case,
                Action::ReadOnlyDecodeToReplacement,
                Disposition::ReadOnlyRecovery,
                Evidence::ValidatedCertificateReceipt,
                Writes::SeparateReplacementTargetOnly,
                Baseline::NewChecksumBaselineRequired,
                false,
                true,
                false,
                false,
            ),
            MetadataLossCase::OneDataSingleParityUncertified
            | MetadataLossCase::OneDataDualParityUncertified => Self::new(
                case,
                Action::RefuseAutomaticDecode,
                Disposition::Refused,
                Evidence::ValidatedCertificateReceipt,
                Writes::NoAutomaticWrite,
                Baseline::NotEstablished,
                false,
                true,
                true,
                false,
            ),
            MetadataLossCase::TwoDataDualParityCodingLost => Self::new(
                case,
                Action::RefuseAutomaticDecode,
                Disposition::Refused,
                Evidence::HistoricalCodingPositions,
                Writes::NoAutomaticWrite,
                Baseline::NotEstablished,
                false,
                false,
                true,
                false,
            ),
            MetadataLossCase::AllMetadataAllDataPresent => Self::new(
                case,
                Action::RefuseAuthorityUnavailable,
                Disposition::Refused,
                Evidence::CustodyLossAuthorityModel,
                Writes::NoAutomaticWrite,
                Baseline::NotEstablished,
                false,
                false,
                false,
                false,
            ),
            MetadataLossCase::TopologyAmbiguous => Self::new(
                case,
                Action::RefuseAmbiguousTopology,
                Disposition::Refused,
                Evidence::UnambiguousIdentity,
                Writes::NoAutomaticWrite,
                Baseline::NotEstablished,
                false,
                false,
                true,
                false,
            ),
            MetadataLossCase::ParityIdentityAmbiguous => Self::new(
                case,
                Action::RefuseAmbiguousParity,
                Disposition::Refused,
                Evidence::UnambiguousIdentity,
                Writes::NoAutomaticWrite,
                Baseline::NotEstablished,
                false,
                false,
                true,
                false,
            ),
            MetadataLossCase::RecoveryBackupValid => Self::new(
                case,
                Action::RestoreValidatedBackup,
                Disposition::RestoreBackup,
                Evidence::ValidatedRecoveryBackup,
                Writes::None,
                Baseline::RetainValidatedHistoricalEvidence,
                false,
                false,
                false,
                false,
            ),
            MetadataLossCase::RecoveryReplicasDisagree => Self::new(
                case,
                Action::ReconcileReplicasConservatively,
                Disposition::ReconcileReplicas,
                Evidence::ReconciledRecoveryReplicas,
                Writes::NoAutomaticWrite,
                Baseline::NotEstablished,
                false,
                false,
                true,
                false,
            ),
            MetadataLossCase::ChecksumEvidenceSurvives => Self::new(
                case,
                Action::EvidenceGatedSelectiveRepair,
                Disposition::EvidenceGatedRepair,
                Evidence::CurrentChecksumEvidence,
                Writes::SeparateReplacementTargetOnly,
                Baseline::NewChecksumBaselineRequired,
                false,
                false,
                false,
                false,
            ),
            MetadataLossCase::ChecksumEvidenceUnavailable => Self::new(
                case,
                Action::RequireDataAuthoritativeRebaseline,
                Disposition::ExplicitRebaseline,
                Evidence::ExplicitDataAuthority,
                Writes::NoAutomaticWrite,
                Baseline::NewParityAndChecksumBaselineRequired,
                true,
                false,
                true,
                false,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    const fn new(
        case: MetadataLossCase,
        action: MetadataLossAction,
        disposition: MetadataLossDisposition,
        required_evidence: EvidenceRequirement,
        payload_write_policy: PayloadWritePolicy,
        baseline: BaselineDisposition,
        requires_exhaustive_verification: bool,
        requires_certificate_receipt: bool,
        requires_operator_confirmation: bool,
        creates_fresh_state: bool,
    ) -> Self {
        Self {
            case,
            action,
            disposition,
            required_evidence,
            payload_write_policy,
            baseline,
            requires_exhaustive_verification,
            requires_certificate_receipt,
            requires_operator_confirmation,
            creates_fresh_state,
        }
    }

    pub fn all() -> Vec<Self> {
        MetadataLossCase::ALL
            .into_iter()
            .map(Self::for_case)
            .collect()
    }

    pub const fn case(self) -> MetadataLossCase {
        self.case
    }

    pub const fn action(self) -> MetadataLossAction {
        self.action
    }

    pub const fn disposition(self) -> MetadataLossDisposition {
        self.disposition
    }

    pub const fn required_evidence(self) -> EvidenceRequirement {
        self.required_evidence
    }

    pub const fn payload_write_policy(self) -> PayloadWritePolicy {
        self.payload_write_policy
    }

    pub const fn baseline(self) -> BaselineDisposition {
        self.baseline
    }

    pub const fn requires_exhaustive_verification(self) -> bool {
        self.requires_exhaustive_verification
    }

    pub const fn requires_certificate_receipt(self) -> bool {
        self.requires_certificate_receipt
    }

    pub const fn requires_operator_confirmation(self) -> bool {
        self.requires_operator_confirmation
    }

    pub const fn creates_fresh_state(self) -> bool {
        self.creates_fresh_state
    }

    pub fn authorize(
        self,
        verification: MetadataLossVerification,
    ) -> Result<MetadataLossAuthorization, MetadataLossError> {
        self.authorize_inner(verification, false)
    }

    pub fn authorize_with_operator_confirmation(
        self,
        verification: MetadataLossVerification,
    ) -> Result<MetadataLossAuthorization, MetadataLossError> {
        self.authorize_inner(verification, true)
    }

    fn authorize_inner(
        self,
        verification: MetadataLossVerification,
        operator_confirmed: bool,
    ) -> Result<MetadataLossAuthorization, MetadataLossError> {
        if self.required_evidence == EvidenceRequirement::ValidatedCertificateReceipt {
            return Err(MetadataLossError::CertificateReceiptUnavailable(self.case));
        }
        if self.action.refuses_automatic_authorization() {
            return Err(MetadataLossError::RefusedCase(self.case));
        }
        if !evidence_satisfies(self.required_evidence, verification) {
            return Err(MetadataLossError::InsufficientEvidence {
                case: self.case,
                required: self.required_evidence,
                actual: verification,
            });
        }
        if (self.requires_operator_confirmation
            || verification == MetadataLossVerification::ExplicitDataAuthoritativeRebaseline)
            && !operator_confirmed
        {
            return Err(MetadataLossError::OperatorConfirmationRequired(self.case));
        }
        Ok(MetadataLossAuthorization {
            plan: self,
            verification,
        })
    }
}

impl fmt::Display for MetadataLossPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} action={} disposition={} evidence={} exhaustive={} certificate_receipt_required={} certificate_receipt_available=false confirmation={} payload_writes={} baseline={} fresh_state={}",
            self.case.id(),
            self.action.id(),
            self.disposition.id(),
            self.required_evidence.id(),
            self.requires_exhaustive_verification,
            self.requires_certificate_receipt,
            self.requires_operator_confirmation,
            self.payload_write_policy.id(),
            self.baseline.id(),
            self.creates_fresh_state,
        )
    }
}

pub fn render_metadata_loss_matrix() -> String {
    use std::fmt::Write as _;

    let mut output = String::new();
    writeln!(
        output,
        "diskweave metadata-loss matrix v{METADATA_LOSS_MATRIX_VERSION} dry-run"
    )
    .expect("writing to a string cannot fail");
    for plan in MetadataLossPlan::all() {
        writeln!(output, "{plan}").expect("writing to a string cannot fail");
    }
    output
}

fn evidence_satisfies(required: EvidenceRequirement, actual: MetadataLossVerification) -> bool {
    match required {
        EvidenceRequirement::ValidatedCertificateReceipt => false,
        EvidenceRequirement::CustodyLossAuthorityModel => false,
        EvidenceRequirement::ExhaustiveEquationScan => matches!(
            actual,
            MetadataLossVerification::ExhaustiveMatches
                | MetadataLossVerification::ExhaustiveIdentifiedRepairs
                | MetadataLossVerification::ExplicitDataAuthoritativeRebaseline
        ),
        EvidenceRequirement::ExplicitDataAuthority => {
            actual == MetadataLossVerification::ExplicitDataAuthoritativeRebaseline
        }
        EvidenceRequirement::UnambiguousIdentity
        | EvidenceRequirement::HistoricalCodingPositions => false,
        EvidenceRequirement::ValidatedRecoveryBackup => {
            actual == MetadataLossVerification::ValidatedBackup
        }
        EvidenceRequirement::ReconciledRecoveryReplicas => {
            actual == MetadataLossVerification::ReconciledReplicas
        }
        EvidenceRequirement::CurrentChecksumEvidence => {
            actual == MetadataLossVerification::ExhaustiveIdentifiedRepairs
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataLossAuthorization {
    plan: MetadataLossPlan,
    verification: MetadataLossVerification,
}

impl MetadataLossAuthorization {
    pub const fn plan(self) -> MetadataLossPlan {
        self.plan
    }

    pub const fn verification(self) -> MetadataLossVerification {
        self.verification
    }

    pub fn fresh_manifest(
        self,
        topology: TopologySnapshot,
        source_health: RecoveryStoreHealth,
    ) -> Result<RecoveryManifest, MetadataLossError> {
        create_fresh_manifest(self, topology, source_health)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MetadataLossAudit {
    pub matrix_version: u16,
    pub lineage_id: ArrayId,
    pub case: MetadataLossCase,
    pub action: MetadataLossAction,
    pub verification: MetadataLossVerification,
    pub baseline: BaselineDisposition,
    pub source_health: RecoveryStoreHealth,
    pub topology_epoch: TopologyEpoch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataLossError {
    CertificateReceiptUnavailable(MetadataLossCase),
    RefusedCase(MetadataLossCase),
    InsufficientEvidence {
        case: MetadataLossCase,
        required: EvidenceRequirement,
        actual: MetadataLossVerification,
    },
    ActionDoesNotCreateFreshState(MetadataLossAction),
    OperatorConfirmationRequired(MetadataLossCase),
    InvalidSourceHealth(RecoveryStoreHealth),
    InvalidFreshTopology(&'static str),
    Recovery(RecoveryError),
}

impl fmt::Display for MetadataLossError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CertificateReceiptUnavailable(case) => write!(
                formatter,
                "metadata-loss case {} requires a validator-issued certificate receipt, but no receipt capability is available",
                case.id(),
            ),
            Self::RefusedCase(case) => {
                write!(
                    formatter,
                    "metadata-loss case {} refuses automatic recovery",
                    case.id()
                )
            }
            Self::InsufficientEvidence {
                case,
                required,
                actual,
            } => write!(
                formatter,
                "metadata-loss case {} requires {}, got {actual:?}",
                case.id(),
                required.id(),
            ),
            Self::ActionDoesNotCreateFreshState(action) => write!(
                formatter,
                "metadata-loss action {} does not create fresh recovery state",
                action.id(),
            ),
            Self::OperatorConfirmationRequired(case) => write!(
                formatter,
                "metadata-loss case {} requires explicit operator confirmation",
                case.id(),
            ),
            Self::InvalidSourceHealth(health) => write!(
                formatter,
                "fresh metadata-loss state requires missing, corrupt, or stale source state; got {health:?}",
            ),
            Self::InvalidFreshTopology(message) => {
                write!(formatter, "invalid fresh recovery topology: {message}")
            }
            Self::Recovery(error) => write!(formatter, "fresh recovery state failed: {error}"),
        }
    }
}

impl std::error::Error for MetadataLossError {}

impl From<RecoveryError> for MetadataLossError {
    fn from(error: RecoveryError) -> Self {
        Self::Recovery(error)
    }
}

pub fn create_fresh_manifest(
    authorization: MetadataLossAuthorization,
    topology: TopologySnapshot,
    source_health: RecoveryStoreHealth,
) -> Result<RecoveryManifest, MetadataLossError> {
    if !authorization.plan.creates_fresh_state {
        return Err(MetadataLossError::ActionDoesNotCreateFreshState(
            authorization.plan.action,
        ));
    }
    if source_health == RecoveryStoreHealth::Healthy {
        return Err(MetadataLossError::InvalidSourceHealth(source_health));
    }
    validate_fresh_topology(&topology)?;
    let topology_epoch = topology.topology_epoch();
    let lineage_id = topology.array_id();
    let audit = MetadataLossAudit {
        matrix_version: METADATA_LOSS_MATRIX_VERSION,
        lineage_id,
        case: authorization.plan.case,
        action: authorization.plan.action,
        verification: authorization.verification,
        baseline: authorization.plan.baseline,
        source_health,
        topology_epoch,
    };
    let baseline = matches!(
        audit.baseline,
        BaselineDisposition::NewChecksumBaselineRequired
            | BaselineDisposition::NewParityAndChecksumBaselineRequired
    )
    .then(|| {
        new_checksum_baseline(
            &topology,
            RecoveryGeneration::ZERO,
            BLAKE3_256_PROFILE,
            ChecksumSetGeneration::INITIAL,
        )
    })
    .transpose()
    .map_err(|error| MetadataLossError::Recovery(RecoveryError::ChecksumBaseline(error)))?;
    let mut recovery = MemoryRecoveryStore::with_active_topology(topology);
    if let Some(baseline) = baseline {
        recovery.set_checksum_baseline(baseline);
    }
    recovery.set_metadata_loss_audit(audit);
    recovery
        .export_manifest(RecoveryGeneration::ZERO)
        .map_err(MetadataLossError::Recovery)
}

fn validate_fresh_topology(topology: &TopologySnapshot) -> Result<(), MetadataLossError> {
    if topology.topology_epoch() == TopologyEpoch(0) {
        return Err(MetadataLossError::InvalidFreshTopology(
            "topology epoch must be non-zero",
        ));
    }
    if topology.profile().parity_slots() != 1 {
        return Err(MetadataLossError::InvalidFreshTopology(
            "fresh-state execution supports exactly one parity slot",
        ));
    }
    if topology
        .assignments()
        .iter()
        .any(|assignment| assignment.assignment_generation().0 == 0)
    {
        return Err(MetadataLossError::InvalidFreshTopology(
            "assignment generations must be non-zero",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_core::{
        AssignmentGeneration, AssignmentInstanceId, CodingPosition, CodingProfile, MemberRole,
        ProtectedGeometry, SlotId, TopologyAssignment, TopologySnapshot as CoreTopologySnapshot,
    };
    use dwv_store::StoreId;
    use std::collections::BTreeSet;

    fn topology(lineage: ArrayId) -> TopologySnapshot {
        let assignments = vec![
            TopologyAssignment::new(
                SlotId([1; 16]),
                MemberRole::Data,
                CodingPosition(0),
                AssignmentInstanceId([11; 16]),
                AssignmentGeneration(1),
            ),
            TopologyAssignment::new(
                SlotId([2; 16]),
                MemberRole::Parity,
                CodingPosition(1),
                AssignmentInstanceId([12; 16]),
                AssignmentGeneration(1),
            ),
        ];
        let core = CoreTopologySnapshot::new(
            lineage,
            TopologyEpoch(1),
            CodingProfile::new(1, 1).unwrap(),
            ProtectedGeometry::new(4096, 512).unwrap(),
            assignments,
        )
        .unwrap();
        TopologySnapshot::from_core(core, vec![StoreId(1), StoreId(2)]).unwrap()
    }

    #[test]
    fn every_matrix_case_has_one_stable_conservative_plan() {
        let plans = MetadataLossPlan::all();
        let identifiers = plans
            .iter()
            .map(|plan| plan.case.id())
            .collect::<BTreeSet<_>>();
        assert_eq!(plans.len(), MetadataLossCase::ALL.len());
        assert_eq!(identifiers.len(), MetadataLossCase::ALL.len());
        assert_eq!(plans.len(), 18);
        let expected = [
            (
                MetadataLossCase::AllDataSingleParityCertified,
                MetadataLossAction::RecreateFromCertifiedEnvelope,
                EvidenceRequirement::ValidatedCertificateReceipt,
                PayloadWritePolicy::None,
                BaselineDisposition::NewChecksumBaselineRequired,
                false,
            ),
            (
                MetadataLossCase::AllDataSingleParityUncertified,
                MetadataLossAction::RefuseAuthorityUnavailable,
                EvidenceRequirement::CustodyLossAuthorityModel,
                PayloadWritePolicy::NoAutomaticWrite,
                BaselineDisposition::NotEstablished,
                false,
            ),
            (
                MetadataLossCase::AllDataDualParityCertified,
                MetadataLossAction::RecreateFromCertifiedEnvelope,
                EvidenceRequirement::ValidatedCertificateReceipt,
                PayloadWritePolicy::None,
                BaselineDisposition::NewChecksumBaselineRequired,
                false,
            ),
            (
                MetadataLossCase::AllDataDualParityUncertified,
                MetadataLossAction::RefuseAuthorityUnavailable,
                EvidenceRequirement::CustodyLossAuthorityModel,
                PayloadWritePolicy::NoAutomaticWrite,
                BaselineDisposition::NotEstablished,
                false,
            ),
            (
                MetadataLossCase::AllDataParityMissing,
                MetadataLossAction::RebuildParityToSeparateTarget,
                EvidenceRequirement::ExplicitDataAuthority,
                PayloadWritePolicy::SeparateParityTargetOnly,
                BaselineDisposition::NewParityAndChecksumBaselineRequired,
                false,
            ),
            (
                MetadataLossCase::OneDataSingleParityCertified,
                MetadataLossAction::ReadOnlyDecodeToReplacement,
                EvidenceRequirement::ValidatedCertificateReceipt,
                PayloadWritePolicy::SeparateReplacementTargetOnly,
                BaselineDisposition::NewChecksumBaselineRequired,
                false,
            ),
            (
                MetadataLossCase::OneDataSingleParityUncertified,
                MetadataLossAction::RefuseAutomaticDecode,
                EvidenceRequirement::ValidatedCertificateReceipt,
                PayloadWritePolicy::NoAutomaticWrite,
                BaselineDisposition::NotEstablished,
                false,
            ),
            (
                MetadataLossCase::OneDataDualParityCertified,
                MetadataLossAction::ReadOnlyDecodeToReplacement,
                EvidenceRequirement::ValidatedCertificateReceipt,
                PayloadWritePolicy::SeparateReplacementTargetOnly,
                BaselineDisposition::NewChecksumBaselineRequired,
                false,
            ),
            (
                MetadataLossCase::OneDataDualParityUncertified,
                MetadataLossAction::RefuseAutomaticDecode,
                EvidenceRequirement::ValidatedCertificateReceipt,
                PayloadWritePolicy::NoAutomaticWrite,
                BaselineDisposition::NotEstablished,
                false,
            ),
            (
                MetadataLossCase::TwoDataDualParityCertified,
                MetadataLossAction::ReadOnlyDecodeToReplacement,
                EvidenceRequirement::ValidatedCertificateReceipt,
                PayloadWritePolicy::SeparateReplacementTargetOnly,
                BaselineDisposition::NewChecksumBaselineRequired,
                false,
            ),
            (
                MetadataLossCase::TwoDataDualParityCodingLost,
                MetadataLossAction::RefuseAutomaticDecode,
                EvidenceRequirement::HistoricalCodingPositions,
                PayloadWritePolicy::NoAutomaticWrite,
                BaselineDisposition::NotEstablished,
                false,
            ),
            (
                MetadataLossCase::AllMetadataAllDataPresent,
                MetadataLossAction::RefuseAuthorityUnavailable,
                EvidenceRequirement::CustodyLossAuthorityModel,
                PayloadWritePolicy::NoAutomaticWrite,
                BaselineDisposition::NotEstablished,
                false,
            ),
            (
                MetadataLossCase::TopologyAmbiguous,
                MetadataLossAction::RefuseAmbiguousTopology,
                EvidenceRequirement::UnambiguousIdentity,
                PayloadWritePolicy::NoAutomaticWrite,
                BaselineDisposition::NotEstablished,
                false,
            ),
            (
                MetadataLossCase::ParityIdentityAmbiguous,
                MetadataLossAction::RefuseAmbiguousParity,
                EvidenceRequirement::UnambiguousIdentity,
                PayloadWritePolicy::NoAutomaticWrite,
                BaselineDisposition::NotEstablished,
                false,
            ),
            (
                MetadataLossCase::RecoveryBackupValid,
                MetadataLossAction::RestoreValidatedBackup,
                EvidenceRequirement::ValidatedRecoveryBackup,
                PayloadWritePolicy::None,
                BaselineDisposition::RetainValidatedHistoricalEvidence,
                false,
            ),
            (
                MetadataLossCase::RecoveryReplicasDisagree,
                MetadataLossAction::ReconcileReplicasConservatively,
                EvidenceRequirement::ReconciledRecoveryReplicas,
                PayloadWritePolicy::NoAutomaticWrite,
                BaselineDisposition::NotEstablished,
                false,
            ),
            (
                MetadataLossCase::ChecksumEvidenceSurvives,
                MetadataLossAction::EvidenceGatedSelectiveRepair,
                EvidenceRequirement::CurrentChecksumEvidence,
                PayloadWritePolicy::SeparateReplacementTargetOnly,
                BaselineDisposition::NewChecksumBaselineRequired,
                false,
            ),
            (
                MetadataLossCase::ChecksumEvidenceUnavailable,
                MetadataLossAction::RequireDataAuthoritativeRebaseline,
                EvidenceRequirement::ExplicitDataAuthority,
                PayloadWritePolicy::NoAutomaticWrite,
                BaselineDisposition::NewParityAndChecksumBaselineRequired,
                false,
            ),
        ];
        for (case, action, evidence, writes, baseline, fresh) in expected {
            let plan = MetadataLossPlan::for_case(case);
            assert_eq!(plan.action(), action, "{} action", case.id());
            assert_eq!(plan.required_evidence(), evidence, "{} evidence", case.id());
            assert_eq!(plan.payload_write_policy(), writes, "{} writes", case.id());
            assert_eq!(plan.baseline(), baseline, "{} baseline", case.id());
            assert_eq!(plan.creates_fresh_state(), fresh, "{} fresh", case.id());
            assert_eq!(
                plan.requires_certificate_receipt(),
                plan.required_evidence() == EvidenceRequirement::ValidatedCertificateReceipt,
                "{} receipt requirement",
                case.id()
            );
        }
        assert!(plans.iter().all(|plan| !plan.to_string().is_empty()));
        assert!(plans.iter().all(|plan| {
            matches!(
                plan.payload_write_policy,
                PayloadWritePolicy::None
                    | PayloadWritePolicy::SeparateParityTargetOnly
                    | PayloadWritePolicy::SeparateReplacementTargetOnly
                    | PayloadWritePolicy::NoAutomaticWrite
            )
        }));
        let first = render_metadata_loss_matrix();
        let second = render_metadata_loss_matrix();
        assert_eq!(first, second);
        assert_eq!(first.lines().count(), MetadataLossCase::ALL.len() + 1);
        assert!(first.contains("all-data-p-certified-clean"));
        assert!(first.contains("checksum-evidence-unavailable"));
        assert!(
            first.contains("certificate_receipt_required=true certificate_receipt_available=false")
        );
    }

    #[test]
    fn certificate_gated_cases_fail_closed_without_a_receipt_capability() {
        let public_verifications = [
            MetadataLossVerification::ExhaustiveMatches,
            MetadataLossVerification::IdentifiedRepairPending,
            MetadataLossVerification::ExhaustiveIdentifiedRepairs,
            MetadataLossVerification::ExplicitDataAuthoritativeRebaseline,
            MetadataLossVerification::AmbiguousMismatch,
            MetadataLossVerification::IncompleteScan,
            MetadataLossVerification::ValidatedBackup,
            MetadataLossVerification::ReconciledReplicas,
            MetadataLossVerification::ConflictingReplicas,
        ];
        for plan in MetadataLossPlan::all().into_iter().filter(|plan| {
            plan.required_evidence() == EvidenceRequirement::ValidatedCertificateReceipt
        }) {
            for verification in public_verifications {
                assert!(matches!(
                    plan.authorize(verification),
                    Err(MetadataLossError::CertificateReceiptUnavailable(case))
                        if case == plan.case()
                ));
                assert!(matches!(
                    plan.authorize_with_operator_confirmation(verification),
                    Err(MetadataLossError::CertificateReceiptUnavailable(case))
                        if case == plan.case()
                ));
            }
        }
    }

    /// dwv:req req.metadata-loss-recovery.identity-and-topology-ambiguity-fails-closed
    #[test]
    fn uncertified_lost_custody_cases_refuse_every_public_evidence() {
        let public_verifications = [
            MetadataLossVerification::ExhaustiveMatches,
            MetadataLossVerification::IdentifiedRepairPending,
            MetadataLossVerification::ExhaustiveIdentifiedRepairs,
            MetadataLossVerification::ExplicitDataAuthoritativeRebaseline,
            MetadataLossVerification::AmbiguousMismatch,
            MetadataLossVerification::IncompleteScan,
            MetadataLossVerification::ValidatedBackup,
            MetadataLossVerification::ReconciledReplicas,
            MetadataLossVerification::ConflictingReplicas,
        ];
        for case in [
            MetadataLossCase::AllDataSingleParityUncertified,
            MetadataLossCase::AllDataDualParityUncertified,
            MetadataLossCase::AllMetadataAllDataPresent,
        ] {
            let plan = MetadataLossPlan::for_case(case);
            assert_eq!(
                plan.action(),
                MetadataLossAction::RefuseAuthorityUnavailable
            );
            assert_eq!(plan.disposition(), MetadataLossDisposition::Refused);
            assert_eq!(
                plan.required_evidence(),
                EvidenceRequirement::CustodyLossAuthorityModel
            );
            assert_eq!(
                plan.payload_write_policy(),
                PayloadWritePolicy::NoAutomaticWrite
            );
            assert_eq!(plan.baseline(), BaselineDisposition::NotEstablished);
            assert!(!plan.creates_fresh_state());
            for verification in public_verifications {
                assert_eq!(
                    plan.authorize(verification),
                    Err(MetadataLossError::RefusedCase(case))
                );
                assert_eq!(
                    plan.authorize_with_operator_confirmation(verification),
                    Err(MetadataLossError::RefusedCase(case))
                );
            }
        }
    }

    #[test]
    fn ambiguous_identity_and_lost_coding_positions_refuse_automatic_recovery() {
        for case in [
            MetadataLossCase::TopologyAmbiguous,
            MetadataLossCase::ParityIdentityAmbiguous,
            MetadataLossCase::TwoDataDualParityCodingLost,
        ] {
            let plan = MetadataLossPlan::for_case(case);
            assert_eq!(plan.disposition, MetadataLossDisposition::Refused);
            assert_eq!(
                plan.payload_write_policy,
                PayloadWritePolicy::NoAutomaticWrite
            );
            assert!(matches!(
                plan.authorize(MetadataLossVerification::ExplicitDataAuthoritativeRebaseline),
                Err(MetadataLossError::RefusedCase(actual)) if actual == case
            ));
        }
    }

    #[test]
    fn invalid_or_noncreating_plans_cannot_emit_fresh_state() {
        let backup = MetadataLossPlan::for_case(MetadataLossCase::RecoveryBackupValid)
            .authorize(MetadataLossVerification::ValidatedBackup)
            .unwrap();
        assert!(matches!(
            backup.fresh_manifest(topology(ArrayId([1; 16])), RecoveryStoreHealth::Missing,),
            Err(MetadataLossError::ActionDoesNotCreateFreshState(_))
        ));
        let replica_plan = MetadataLossPlan::for_case(MetadataLossCase::RecoveryReplicasDisagree);
        assert!(matches!(
            replica_plan.authorize(MetadataLossVerification::ReconciledReplicas),
            Err(MetadataLossError::OperatorConfirmationRequired(_))
        ));
        let replica_authorization = replica_plan
            .authorize_with_operator_confirmation(MetadataLossVerification::ReconciledReplicas)
            .unwrap();
        assert!(matches!(
            replica_authorization
                .fresh_manifest(topology(ArrayId([2; 16])), RecoveryStoreHealth::Stale),
            Err(MetadataLossError::ActionDoesNotCreateFreshState(_))
        ));

        assert!(matches!(
            TopologySnapshot::from_core(
                CoreTopologySnapshot::new(
                    ArrayId([1; 16]),
                    TopologyEpoch(1),
                    CodingProfile::new(1, 1).unwrap(),
                    ProtectedGeometry::new(4096, 512).unwrap(),
                    vec![
                        TopologyAssignment::new(
                            SlotId([1; 16]),
                            MemberRole::Data,
                            CodingPosition(0),
                            AssignmentInstanceId([1; 16]),
                            AssignmentGeneration(1)
                        ),
                        TopologyAssignment::new(
                            SlotId([2; 16]),
                            MemberRole::Parity,
                            CodingPosition(1),
                            AssignmentInstanceId([2; 16]),
                            AssignmentGeneration(1)
                        ),
                    ],
                )
                .unwrap(),
                vec![StoreId(1), StoreId(1)],
            ),
            Err(crate::RecoveryTopologyError::DuplicateStore(StoreId(1)))
        ));
    }
}
