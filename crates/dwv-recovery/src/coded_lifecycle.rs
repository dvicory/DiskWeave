use crate::coded_authority::CodedAdmissionAuthorityId;
use crate::{
    ChecksumProfile, ChecksumSetGeneration, CodedAdmissionOutcome, CodedAuthorityError,
    CodedAuthorityFrontier, CodedCaptureCleanAuthorization, CodedCaptureCoordinator,
    CodedCaptureError, CodedCaptureId, CodedCaptureReconciliationReceipt, CodedCaptureSnapshot,
    CodedCaptureTransition, CodedClaim, CodedClaimInput, CodedClaimRelease, CodedCleanAttempt,
    CodedCleanAttemptPhase, CodedCleanCommitObservation, CodedCleanKnownCleanupAuthorization,
    CodedEffectPermit, CodedGeometryOwner, CodedInheritedCaptureAbandonmentAuthorization,
    CodedLaterCutAttempt, CodedLaterCutAttemptPhase, CodedLaterCutObservation,
    CodedMembershipCompactionAuthorization, CodedOperationPhase, CodedRangeAuthority,
    CodedRefusedCleanupAuthorization, CodedReopenResolution, DurableRecoveryCommit,
    FenceCertificate, InvalidationTarget, PreparedCodedCaptureUpdates, PreparedCodedCleanCommit,
    PreparedCodedCleanKnownCleanup, PreparedCodedInheritedCaptureAbandonment,
    PreparedCodedLaterCut, PreparedCodedMembershipCompaction, PreparedCodedRefusal,
    PreparedCodedRefusedCleanup, RecoveryCleanPermit, RecoveryCleanRefusalPermit,
    RecoveryCommitObservation, RecoveryError, RecoveryGeneration, RecoveryMutation,
    RecoverySnapshot, RecoveryStateStore, RecoveryTxn,
};
use dwv_core::{BlockRequest, TopologyEpoch, TopologySnapshot};
use dwv_lifecycle_authority::{
    IncludedLifecycleAuthorization, LifecycleAuthorityVerifier, ReleaseAuthorization,
};
use dwv_store::{OperationReleasePermit, OperationSlotToken};
use std::fmt;
use std::marker::PhantomData;

/// Service-owned release authority accepted by the canonical coded consumer.
///
/// Implementations choose the exact capability type accepted by one
/// `CodedLifecycleAuthority` specialization. A different wrapper type cannot
/// be substituted even when it contains semantically similar values.
pub trait CodedReleaseAuthority {
    #[doc(hidden)]
    fn lifecycle_release(&self) -> &ReleaseAuthorization;
}

/// Service-owned Included lifecycle authority accepted by coded CLEAN.
pub trait CodedIncludedAuthority {
    #[doc(hidden)]
    fn lifecycle_included(&self) -> &IncludedLifecycleAuthorization;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodedLifecycleError {
    Authority(CodedAuthorityError),
    Capture(CodedCaptureError),
    PreparedRangeStale,
    DurableObservationRequired,
    AuthorityInvalidated,
}

impl fmt::Display for CodedLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authority(error) => error.fmt(formatter),
            Self::Capture(error) => error.fmt(formatter),
            Self::PreparedRangeStale => {
                formatter.write_str("prepared coded range transition is stale or foreign")
            }
            Self::DurableObservationRequired => {
                formatter.write_str("coded range transition requires exact durable observation")
            }
            Self::AuthorityInvalidated => formatter.write_str(
                "coded lifecycle authority was invalidated by an uncertain durable transition",
            ),
        }
    }
}

impl std::error::Error for CodedLifecycleError {}

impl From<CodedAuthorityError> for CodedLifecycleError {
    fn from(error: CodedAuthorityError) -> Self {
        Self::Authority(error)
    }
}

impl From<CodedCaptureError> for CodedLifecycleError {
    fn from(error: CodedCaptureError) -> Self {
        Self::Capture(error)
    }
}

#[must_use = "prepared admission must be durably confirmed or discarded"]
pub struct PreparedCodedAdmission {
    prior_range_revision: u64,
    range: CodedRangeAuthority,
    outcome: CodedAdmissionOutcome,
    capture_updates: Option<PreparedCodedCaptureUpdates>,
}

impl PreparedCodedAdmission {
    pub const fn outcome(&self) -> &CodedAdmissionOutcome {
        &self.outcome
    }

    pub fn transition(&self) -> Option<CodedCaptureTransition> {
        self.capture_updates
            .as_ref()
            .map(PreparedCodedCaptureUpdates::transition)
    }
}

#[must_use = "prepared release must be durably confirmed or discarded"]
pub struct PreparedCodedRelease {
    prior_range_revision: u64,
    range: CodedRangeAuthority,
    release: CodedClaimRelease,
    capture_updates: Option<PreparedCodedCaptureUpdates>,
}

impl PreparedCodedRelease {
    pub const fn release(&self) -> CodedClaimRelease {
        self.release
    }

    pub fn transition(&self) -> Option<CodedCaptureTransition> {
        self.capture_updates
            .as_ref()
            .map(PreparedCodedCaptureUpdates::transition)
    }
}

#[must_use = "prepared owner transition must be durably confirmed or discarded"]
pub struct PreparedCodedOwnerTransition<T> {
    authority_id: u64,
    inner: T,
}

impl<T> PreparedCodedOwnerTransition<T> {
    fn new(authority_id: u64, inner: T) -> Self {
        Self {
            authority_id,
            inner,
        }
    }

    fn into_inner(self, authority_id: u64) -> Result<T, CodedCaptureError> {
        if self.authority_id != authority_id {
            return Err(CodedCaptureError::ForeignCaptureAuthority);
        }
        Ok(self.inner)
    }
}

macro_rules! prepared_transition_projection {
    ($($prepared:ty),+ $(,)?) => {
        $(
            impl PreparedCodedOwnerTransition<$prepared> {
                pub fn transition(&self) -> CodedCaptureTransition {
                    self.inner.transition()
                }
            }
        )+
    };
}

prepared_transition_projection!(
    PreparedCodedCaptureUpdates,
    PreparedCodedRefusal,
    PreparedCodedCleanCommit,
    PreparedCodedLaterCut,
    PreparedCodedMembershipCompaction,
    PreparedCodedRefusedCleanup,
    PreparedCodedCleanKnownCleanup,
    PreparedCodedInheritedCaptureAbandonment,
);
impl PreparedCodedOwnerTransition<PreparedCodedLaterCut> {
    pub fn invalidation_target(&self) -> &InvalidationTarget {
        self.inner.invalidation_target()
    }
}

pub struct CodedOwnerCleanAttempt {
    authority_id: u64,
    inner: CodedCleanAttempt,
}

impl PreparedCodedOwnerTransition<PreparedCodedCleanCommit> {
    pub fn into_attempt(self) -> CodedOwnerCleanAttempt {
        CodedOwnerCleanAttempt {
            authority_id: self.authority_id,
            inner: CodedCleanAttempt::new(self.inner),
        }
    }
}

impl CodedOwnerCleanAttempt {
    pub const fn phase(&self) -> CodedCleanAttemptPhase {
        self.inner.phase()
    }

    pub fn observe(
        &mut self,
        observation: CodedCleanCommitObservation,
    ) -> Result<(), CodedCaptureError> {
        self.inner.observe(observation)
    }

    pub fn reject_unknown(&mut self) -> Result<(), CodedCaptureError> {
        self.inner.reject_unknown()
    }

    pub fn into_pending_prepared(
        self,
    ) -> Result<PreparedCodedOwnerTransition<PreparedCodedCleanCommit>, CodedCaptureError> {
        Ok(PreparedCodedOwnerTransition::new(
            self.authority_id,
            self.inner.into_pending_prepared()?,
        ))
    }

    pub fn into_unknown_prepared(
        self,
    ) -> Result<PreparedCodedOwnerTransition<PreparedCodedCleanCommit>, CodedCaptureError> {
        Ok(PreparedCodedOwnerTransition::new(
            self.authority_id,
            self.inner.into_unknown_prepared()?,
        ))
    }
}

pub struct CodedOwnerLaterCutAttempt {
    authority_id: u64,
    inner: CodedLaterCutAttempt,
}

impl PreparedCodedOwnerTransition<PreparedCodedLaterCut> {
    pub fn into_attempt(
        self,
        observation: CodedLaterCutObservation,
    ) -> Result<CodedOwnerLaterCutAttempt, CodedCaptureError> {
        Ok(CodedOwnerLaterCutAttempt {
            authority_id: self.authority_id,
            inner: CodedLaterCutAttempt::new(self.inner, observation)?,
        })
    }
}

impl CodedOwnerLaterCutAttempt {
    pub const fn phase(&self) -> CodedLaterCutAttemptPhase {
        self.inner.phase()
    }

    pub const fn operation(&self) -> OperationSlotToken {
        self.inner.operation()
    }

    pub fn reject_unknown(&mut self) -> Result<(), CodedCaptureError> {
        self.inner.reject_unknown()
    }

    pub fn into_unknown_prepared(
        self,
    ) -> Result<PreparedCodedOwnerTransition<PreparedCodedLaterCut>, CodedCaptureError> {
        Ok(PreparedCodedOwnerTransition::new(
            self.authority_id,
            self.inner.into_unknown_prepared()?,
        ))
    }
}

/// Coded consumers bound to one service-selected lifecycle verifier and one
/// admission/geometry authority graph.
///
/// This type exclusively owns the mutable range and capture subowners.
/// Callers can inspect value snapshots and can request composed transitions,
/// but cannot extract, fork, replace, or independently mutate either owner.
///
/// Authority-bearing subowners are not exposed:
///
/// ```compile_fail
/// use dwv_recovery::{CodedLifecycleAuthority, CodedRangeAuthority};
///
/// fn extract(authority: &CodedLifecycleAuthority<(), ()>) -> &CodedRangeAuthority {
///     authority.range()
/// }
/// ```
///
/// ```compile_fail
/// use dwv_recovery::{CodedCaptureCoordinator, CodedLifecycleAuthority};
///
/// fn replace(
///     authority: &mut CodedLifecycleAuthority<(), ()>,
///     replacement: CodedCaptureCoordinator,
/// ) {
///     let captures: &mut CodedCaptureCoordinator = authority;
///     *captures = replacement;
/// }
/// ```
///
/// Mutable owner identities are not forkable:
///
/// ```compile_fail
/// use dwv_recovery::CodedRangeAuthority;
///
/// fn fork(authority: &CodedRangeAuthority) -> CodedRangeAuthority {
///     authority.clone()
/// }
/// ```
///
/// ```compile_fail
/// use dwv_recovery::CodedCaptureCoordinator;
///
/// fn fork(authority: &CodedCaptureCoordinator) -> CodedCaptureCoordinator {
///     authority.clone()
/// }
/// ```
///
/// Raw capture owners cannot install prepared transitions:
///
/// ```compile_fail
/// use dwv_recovery::{
///     CodedCaptureCoordinator, DurableRecoveryCommit, PreparedCodedCleanCommit,
/// };
///
/// fn install(
///     owner: &mut CodedCaptureCoordinator,
///     prepared: PreparedCodedCleanCommit,
///     receipt: &DurableRecoveryCommit,
/// ) {
///     owner.confirm_clean_commit(prepared, receipt).unwrap();
/// }
/// ```
pub struct CodedLifecycleAuthority<R, I> {
    range: CodedRangeAuthority,
    captures: CodedCaptureCoordinator,
    geometry: CodedGeometryOwner,
    authority_usable: bool,
    capability_types: PhantomData<fn(&R, &I)>,
}

impl<R, I> CodedLifecycleAuthority<R, I> {
    pub fn from_snapshots_with_verifier(
        snapshots: impl IntoIterator<Item = CodedCaptureSnapshot>,
        topology: &TopologySnapshot,
        recovery_generation: RecoveryGeneration,
        checksum_profile: ChecksumProfile,
        checksum_set_generation: ChecksumSetGeneration,
        dirty_region_bytes: u64,
        verifier: LifecycleAuthorityVerifier,
    ) -> Result<(Self, CodedGeometryOwner), CodedCaptureError> {
        let geometry =
            CodedGeometryOwner::new(topology.clone(), dirty_region_bytes, checksum_profile)?;
        let admission_authority = CodedAdmissionAuthorityId::new();
        let range = CodedRangeAuthority::new_with_authorities(
            verifier.clone(),
            admission_authority,
            geometry.authority(),
        );
        let captures = CodedCaptureCoordinator::from_snapshots_for_owner(
            snapshots,
            topology,
            recovery_generation,
            checksum_profile,
            checksum_set_generation,
            dirty_region_bytes,
            verifier,
            admission_authority,
        )?;
        Ok((
            Self {
                geometry: geometry.clone(),
                range,
                captures,
                authority_usable: true,
                capability_types: PhantomData,
            },
            geometry,
        ))
    }

    pub const fn is_usable(&self) -> bool {
        self.authority_usable
    }

    /// Revoke process-local authority when durable coded state may differ from it.
    ///
    /// Only reconstruction from authoritative recovery state can establish a
    /// usable owner after this boundary.
    pub fn invalidate_after_uncertain_persistence(&mut self) {
        self.authority_usable = false;
    }

    fn require_lifecycle_authority(&self) -> Result<(), CodedLifecycleError> {
        self.authority_usable
            .then_some(())
            .ok_or(CodedLifecycleError::AuthorityInvalidated)
    }

    fn require_capture_authority(&self) -> Result<(), CodedCaptureError> {
        self.authority_usable
            .then_some(())
            .ok_or(CodedCaptureError::AuthorityInvalidated)
    }

    pub fn operation_phase(&self, operation: OperationSlotToken) -> Option<CodedOperationPhase> {
        self.range.operation_phase(operation)
    }

    pub fn active_claim(&self, operation: OperationSlotToken) -> Option<&CodedClaim> {
        self.range.active_claim(operation)
    }

    pub fn admission_sequence(&self, operation: OperationSlotToken) -> Option<u64> {
        self.range.admission_sequence(operation)
    }

    pub const fn admission_high_water(&self) -> u64 {
        self.range.admission_high_water()
    }

    pub fn retention_frontier(&self) -> CodedAuthorityFrontier {
        self.range.retention_frontier()
    }

    pub fn invalidation_target(&self, operation: OperationSlotToken) -> Option<InvalidationTarget> {
        self.range
            .admission(operation)
            .map(|admission| admission.invalidation_target().clone())
    }

    pub fn snapshots(&self) -> Vec<CodedCaptureSnapshot> {
        self.captures.snapshots()
    }

    pub fn capture_count(&self) -> usize {
        self.captures.capture_count()
    }

    pub fn capture_snapshot(&self, capture: CodedCaptureId) -> Option<CodedCaptureSnapshot> {
        self.captures.capture_snapshot(capture)
    }

    pub fn capture_owner_revalidated(&self, capture: CodedCaptureId) -> bool {
        self.authority_usable && self.captures.capture_owner_revalidated(capture)
    }

    pub fn has_unresolved_reopen_state(&self) -> bool {
        !self.authority_usable || self.captures.has_unresolved_reopen_state()
    }

    pub fn prepare_admission(
        &self,
        operation: OperationSlotToken,
        request: BlockRequest,
        claim: CodedClaimInput,
        expected_generation: RecoveryGeneration,
    ) -> Result<PreparedCodedAdmission, CodedLifecycleError> {
        self.require_lifecycle_authority()?;
        let prior_range_revision = self.range.state_revision();
        let mut range = self.range.fork_candidate();
        let outcome = range.admit(operation, request, claim)?;
        let capture_updates = match &outcome {
            CodedAdmissionOutcome::Admitted(admission) => self
                .captures
                .prepare_admission_observation(admission, expected_generation)?,
            CodedAdmissionOutcome::Contended => None,
        };
        Ok(PreparedCodedAdmission {
            prior_range_revision,
            range,
            outcome,
            capture_updates,
        })
    }

    pub fn confirm_admission(
        &mut self,
        prepared: PreparedCodedAdmission,
        receipt: Option<&DurableRecoveryCommit>,
    ) -> Result<CodedAdmissionOutcome, CodedLifecycleError> {
        self.require_lifecycle_authority()?;
        if matches!(prepared.outcome, CodedAdmissionOutcome::Contended) {
            return Ok(prepared.outcome);
        }
        self.validate_range_predecessor(prepared.prior_range_revision, &prepared.range)?;
        if let Some(updates) = prepared.capture_updates {
            self.captures.confirm_prepared_updates(
                updates,
                receipt.ok_or(CodedLifecycleError::DurableObservationRequired)?,
            )?;
        }
        self.range = prepared.range;
        Ok(prepared.outcome)
    }

    pub fn effect_allowed(&self, operation: OperationSlotToken) -> bool {
        self.authority_usable
            && self
                .range
                .active_claim(operation)
                .is_some_and(|claim| self.captures.effect_allowed_for(operation, claim.units()))
    }

    /// Issue media-effect authority only after range and capture owners agree.
    pub fn permit_effect(
        &mut self,
        operation: OperationSlotToken,
    ) -> Result<Option<CodedEffectPermit>, CodedAuthorityError> {
        if !self.effect_allowed(operation) {
            return Ok(None);
        }
        self.range.permit_effect(operation).map(Some)
    }

    /// Establish one capture while retaining the owner's exclusive mutable
    /// borrow across durable persistence and live installation.
    ///
    /// The owner constructs and submits the exact recovery transaction itself.
    /// Ordinary callers cannot substitute a persistence callback or interleave
    /// another range decision at the capture frontier:
    ///
    /// ```compile_fail
    /// use dwv_recovery::{
    ///     ChecksumSetGeneration, CodedCaptureId, CodedLifecycleAuthority,
    ///     IntegrityExtentId, InvalidationTarget, RecoveryGeneration, RegionId,
    /// };
    /// use dwv_store::OperationSlotToken;
    ///
    /// fn interleave<R, I>(
    ///     owner: &mut CodedLifecycleAuthority<R, I>,
    ///     capture: CodedCaptureId,
    ///     generation: RecoveryGeneration,
    ///     checksum_set: ChecksumSetGeneration,
    ///     region: RegionId,
    ///     extent: IntegrityExtentId,
    ///     operation: OperationSlotToken,
    /// ) {
    ///     let _ = owner.start_capture(
    ///         capture,
    ///         generation,
    ///         checksum_set,
    ///         InvalidationTarget::new(vec![region], vec![extent]),
    ///         |_transition| {
    ///             let _ = owner.permit_effect(operation);
    ///         },
    ///     );
    /// }
    /// ```
    pub fn start_capture(
        &mut self,
        capture: CodedCaptureId,
        expected_generation: RecoveryGeneration,
        checksum_set_generation: ChecksumSetGeneration,
        target: InvalidationTarget,
        topology_epoch: TopologyEpoch,
        recovery: &mut impl RecoveryStateStore,
    ) -> Result<Result<DurableRecoveryCommit, RecoveryError>, CodedLifecycleError> {
        self.require_lifecycle_authority()?;
        let prior_range_revision = self.range.state_revision();
        let mut range = self.range.fork_candidate();
        let scope = self
            .geometry
            .capture_scope(target.regions, target.checksum_extents)?;
        let establishment = range
            .capture_boundary()
            .ok_or(CodedCaptureError::CaptureSequenceExhausted)?;
        let prepared = PreparedCodedOwnerTransition::new(
            self.captures.authority_id(),
            self.captures.prepare_capture_start(
                capture,
                expected_generation,
                checksum_set_generation,
                scope,
                establishment,
            )?,
        );
        let mut txn = RecoveryTxn::new(expected_generation, topology_epoch);
        txn.push(RecoveryMutation::ApplyCodedTransition {
            transition: prepared.transition(),
        });
        let receipt = match recovery.commit_durable_receipt(txn) {
            Ok(receipt) => receipt,
            Err(error @ RecoveryError::CommitNotDurable(RecoveryCommitObservation::Rejected)) => {
                return Ok(Err(error));
            }
            Err(error) => {
                self.authority_usable = false;
                return Ok(Err(error));
            }
        };
        let installation = self
            .validate_range_predecessor(prior_range_revision, &range)
            .and_then(|()| {
                self.confirm_prepared_updates(prepared, &receipt)
                    .map_err(CodedLifecycleError::from)
            });
        if let Err(error) = installation {
            self.authority_usable = false;
            return Err(error);
        }
        self.range = range;
        Ok(Ok(receipt))
    }

    pub fn prepare_release(
        &self,
        authorization: &R,
        permit: OperationReleasePermit,
        certificate: Option<&FenceCertificate>,
        expected_generation: RecoveryGeneration,
    ) -> Result<PreparedCodedRelease, CodedLifecycleError>
    where
        R: CodedReleaseAuthority,
    {
        self.require_lifecycle_authority()?;
        let prior_range_revision = self.range.state_revision();
        let mut range = self.range.fork_candidate();
        let release = range.release(authorization.lifecycle_release(), permit)?;
        let capture_updates = self.captures.prepare_release_observation(
            &release,
            certificate,
            expected_generation,
        )?;
        Ok(PreparedCodedRelease {
            prior_range_revision,
            range,
            release,
            capture_updates,
        })
    }

    pub fn confirm_release(
        &mut self,
        prepared: PreparedCodedRelease,
        receipt: Option<&DurableRecoveryCommit>,
    ) -> Result<CodedClaimRelease, CodedLifecycleError> {
        self.require_lifecycle_authority()?;
        self.validate_range_predecessor(prepared.prior_range_revision, &prepared.range)?;
        if let Some(updates) = prepared.capture_updates {
            self.captures.confirm_prepared_updates(
                updates,
                receipt.ok_or(CodedLifecycleError::DurableObservationRequired)?,
            )?;
        }
        self.range = prepared.range;
        Ok(prepared.release)
    }

    fn validate_range_predecessor(
        &self,
        prior_range_revision: u64,
        candidate: &CodedRangeAuthority,
    ) -> Result<(), CodedLifecycleError> {
        if self.range.state_revision() != prior_range_revision
            || !self.range.belongs_to_same_owner(candidate)
        {
            return Err(CodedLifecycleError::PreparedRangeStale);
        }
        Ok(())
    }

    pub fn authorize_clean(
        &self,
        recovery: &RecoverySnapshot,
        capture: CodedCaptureId,
        permit: RecoveryCleanPermit,
        included: impl IntoIterator<Item = (I, FenceCertificate)>,
    ) -> Result<CodedCaptureCleanAuthorization, CodedCaptureError>
    where
        I: CodedIncludedAuthority,
    {
        self.require_capture_authority()?;
        self.captures.authorize_clean(
            recovery,
            capture,
            permit,
            included.into_iter().map(|(authorization, certificate)| {
                (authorization.lifecycle_included().clone(), certificate)
            }),
        )
    }

    pub fn refresh_clean_authorization(
        &self,
        recovery: &RecoverySnapshot,
        authorization: CodedCaptureCleanAuthorization,
        permit: RecoveryCleanPermit,
    ) -> Result<CodedCaptureCleanAuthorization, CodedCaptureError> {
        self.require_capture_authority()?;
        self.captures
            .refresh_clean_authorization(recovery, authorization, permit)
    }

    pub fn resolve_capture(
        &self,
        recovery: &RecoverySnapshot,
        capture: CodedCaptureId,
        refusal: Option<RecoveryCleanRefusalPermit>,
    ) -> Result<CodedReopenResolution, CodedCaptureError> {
        self.require_capture_authority()?;
        self.captures.resolve_capture(recovery, capture, refusal)
    }

    pub fn confirm_prepared_updates(
        &mut self,
        prepared: PreparedCodedOwnerTransition<PreparedCodedCaptureUpdates>,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = prepared.into_inner(self.captures.authority_id())?;
        self.captures.confirm_prepared_updates(prepared, receipt)
    }

    pub fn prepare_reconciliation(
        &self,
        receipt: CodedCaptureReconciliationReceipt,
        expected_generation: RecoveryGeneration,
    ) -> Result<PreparedCodedOwnerTransition<PreparedCodedCaptureUpdates>, CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = self
            .captures
            .prepare_reconciliation(receipt, expected_generation)?;
        Ok(PreparedCodedOwnerTransition::new(
            self.captures.authority_id(),
            prepared,
        ))
    }

    pub fn prepare_clean_refusal(
        &self,
        capture: CodedCaptureId,
        refusal: &RecoveryCleanRefusalPermit,
    ) -> Result<PreparedCodedOwnerTransition<PreparedCodedRefusal>, CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = self.captures.prepare_clean_refusal(capture, refusal)?;
        Ok(PreparedCodedOwnerTransition::new(
            self.captures.authority_id(),
            prepared,
        ))
    }

    pub fn confirm_clean_refusal(
        &mut self,
        prepared: PreparedCodedOwnerTransition<PreparedCodedRefusal>,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = prepared.into_inner(self.captures.authority_id())?;
        self.captures.confirm_clean_refusal(prepared, receipt)
    }

    pub fn prepare_clean_commit(
        &self,
        authorization: CodedCaptureCleanAuthorization,
    ) -> Result<PreparedCodedOwnerTransition<PreparedCodedCleanCommit>, CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = self.captures.prepare_clean_commit(authorization)?;
        Ok(PreparedCodedOwnerTransition::new(
            self.captures.authority_id(),
            prepared,
        ))
    }

    pub fn confirm_clean_commit(
        &mut self,
        prepared: PreparedCodedOwnerTransition<PreparedCodedCleanCommit>,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = prepared.into_inner(self.captures.authority_id())?;
        self.captures.confirm_clean_commit(prepared, receipt)
    }

    pub fn prepare_later_cut(
        &self,
        capture: CodedCaptureId,
        operation: OperationSlotToken,
        topology_epoch: TopologyEpoch,
        expected_generation: RecoveryGeneration,
    ) -> Result<PreparedCodedOwnerTransition<PreparedCodedLaterCut>, CodedLifecycleError> {
        self.require_lifecycle_authority()?;
        let admission = self
            .range
            .admission(operation)
            .ok_or(CodedAuthorityError::OperationNotAdmitted(operation))?;
        let prepared = self.captures.prepare_later_cut(
            capture,
            &admission,
            topology_epoch,
            expected_generation,
        )?;
        Ok(PreparedCodedOwnerTransition::new(
            self.captures.authority_id(),
            prepared,
        ))
    }

    pub fn confirm_later_cut(
        &mut self,
        prepared: PreparedCodedOwnerTransition<PreparedCodedLaterCut>,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = prepared.into_inner(self.captures.authority_id())?;
        self.captures.confirm_later_cut(prepared, receipt)
    }

    pub fn prepare_membership_compaction(
        &self,
        authorization: CodedMembershipCompactionAuthorization,
    ) -> Result<PreparedCodedOwnerTransition<PreparedCodedMembershipCompaction>, CodedCaptureError>
    {
        self.require_capture_authority()?;
        let prepared = self.captures.prepare_membership_compaction(authorization)?;
        Ok(PreparedCodedOwnerTransition::new(
            self.captures.authority_id(),
            prepared,
        ))
    }

    pub fn confirm_membership_compaction(
        &mut self,
        prepared: PreparedCodedOwnerTransition<PreparedCodedMembershipCompaction>,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = prepared.into_inner(self.captures.authority_id())?;
        self.captures
            .confirm_membership_compaction(prepared, receipt)
    }

    pub fn prepare_refused_cleanup(
        &self,
        authorization: CodedRefusedCleanupAuthorization,
    ) -> Result<PreparedCodedOwnerTransition<PreparedCodedRefusedCleanup>, CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = self.captures.prepare_refused_cleanup(authorization)?;
        Ok(PreparedCodedOwnerTransition::new(
            self.captures.authority_id(),
            prepared,
        ))
    }

    pub fn prepare_clean_known_cleanup(
        &self,
        authorization: CodedCleanKnownCleanupAuthorization,
    ) -> Result<PreparedCodedOwnerTransition<PreparedCodedCleanKnownCleanup>, CodedCaptureError>
    {
        self.require_capture_authority()?;
        let prepared = self.captures.prepare_clean_known_cleanup(authorization)?;
        Ok(PreparedCodedOwnerTransition::new(
            self.captures.authority_id(),
            prepared,
        ))
    }

    pub fn prepare_inherited_capture_abandonment(
        &self,
        authorization: CodedInheritedCaptureAbandonmentAuthorization,
    ) -> Result<
        PreparedCodedOwnerTransition<PreparedCodedInheritedCaptureAbandonment>,
        CodedCaptureError,
    > {
        self.require_capture_authority()?;
        let prepared = self
            .captures
            .prepare_inherited_capture_abandonment(authorization)?;
        Ok(PreparedCodedOwnerTransition::new(
            self.captures.authority_id(),
            prepared,
        ))
    }

    pub fn confirm_refused_cleanup(
        &mut self,
        prepared: PreparedCodedOwnerTransition<PreparedCodedRefusedCleanup>,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = prepared.into_inner(self.captures.authority_id())?;
        self.captures.confirm_refused_cleanup(prepared, receipt)
    }

    pub fn confirm_clean_known_cleanup(
        &mut self,
        prepared: PreparedCodedOwnerTransition<PreparedCodedCleanKnownCleanup>,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = prepared.into_inner(self.captures.authority_id())?;
        self.captures.confirm_clean_known_cleanup(prepared, receipt)
    }

    pub fn confirm_inherited_capture_abandonment(
        &mut self,
        prepared: PreparedCodedOwnerTransition<PreparedCodedInheritedCaptureAbandonment>,
        receipt: &DurableRecoveryCommit,
    ) -> Result<(), CodedCaptureError> {
        self.require_capture_authority()?;
        let prepared = prepared.into_inner(self.captures.authority_id())?;
        self.captures
            .confirm_inherited_capture_abandonment(prepared, receipt)
    }
}
