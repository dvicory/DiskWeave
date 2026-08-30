use super::*;
use anyhow::{Context, bail};
use dwv_core::{
    BlockOp, ByteRange, CodedUnitId, DurabilityIntent, FenceDomain, RequestId, TopologyEpoch,
};
use dwv_recovery::{
    CodedCaptureCleanAuthorization, CodedCaptureDecision, CodedCaptureError, CodedCaptureId,
    CodedCaptureMembership, CodedCapturePhase, CodedCaptureSnapshot, CodedCleanAttemptPhase,
    CodedCleanCommitObservation, CodedCleanReconciliation, CodedGeometryOwner,
    CodedLaterCutAttemptPhase, CodedLaterCutObservation, CodedLaterCutReconciliation,
    CodedLifecycleError, CodedOwnerCleanAttempt, CodedOwnerLaterCutAttempt, CodedRangeAuthority,
    FenceCertificate, IntegrityExtentId, InvalidationTarget, MemoryRecoveryStore,
    PreparedCodedCleanCommit, PreparedCodedLaterCut, PreparedCodedOwnerTransition,
    RecoveryCleanRefusalPermit, RecoveryCleanRequest, RecoveryMutation, RecoverySnapshot,
    RecoveryStateStore, RecoveryTxn, RegionId,
};
use dwv_store::{
    CapabilityEvidenceId, FenceId, OperationSlotToken, StoreFenceRef, StoreId, StoreIncarnationId,
    StoreWriteWatermark,
};
use dwv_transaction_ref::{
    CodedAdmissionOutcome, CodedAuthorityError, CodedClaimInput, CodedOperationPhase,
};
use quint_connect::{Config, Driver, Result, State, Step, quint_run};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap, HashSet};

fn release_certificate(topology_epoch: TopologyEpoch) -> FenceCertificate {
    FenceCertificate::new(
        topology_epoch,
        FenceDomain(1),
        vec![StoreFenceRef {
            fence_id: FenceId(1),
            store_id: StoreId(1),
            topology_epoch,
            store_incarnation: StoreIncarnationId(0),
            through: StoreWriteWatermark(0),
            capability_evidence_id: CapabilityEvidenceId(1),
        }],
        Vec::new(),
    )
}

fn issued_coded_claim<R: RecoveryStateStore>(
    service: &HealthyPortableService<FakeStore, R>,
    operation: OperationSlotToken,
    member_index: usize,
    ranges: impl IntoIterator<Item = ByteRange>,
) -> std::result::Result<CodedClaimInput, ServiceError> {
    let ranges = ranges.into_iter().collect::<Vec<_>>();
    let request_range = match (ranges.first(), ranges.last()) {
        (Some(first), Some(last)) => {
            ByteRange::new(first.offset, last.end() - first.offset).expect("test coded claim range")
        }
        _ => ByteRange::empty(),
    };
    let request = request(
        RequestId(9_000),
        service.topology.topology_epoch(),
        member_index,
        BlockOp::Write,
        request_range,
        DurabilityIntent::Ordinary,
    );
    service.coded_claim_for_plan(
        member_index,
        request,
        &crate::range::RangePlan { ranges },
        operation,
    )
}

fn issued_coded_unit_claim<R: RecoveryStateStore>(
    service: &HealthyPortableService<FakeStore, R>,
    operation: OperationSlotToken,
    units: impl IntoIterator<Item = CodedUnitId>,
) -> std::result::Result<CodedClaimInput, ServiceError> {
    let snapshot = service.admission.snapshot(operation).map_err(slot_error)?;
    let member_index = service
        .members
        .iter()
        .position(|member| member.slot_id == snapshot.request.slot_id)
        .ok_or_else(|| {
            ServiceError::invalid(
                FailureClass::InvalidRequest,
                "admitted request has no member binding",
            )
        })?;
    let ranges = units
        .into_iter()
        .map(|unit| {
            ByteRange::new(unit.0 * u64::from(BLOCK), u64::from(BLOCK))
                .expect("test coded unit range")
        })
        .collect();
    service.coded_claim_for_plan(
        member_index,
        snapshot.request,
        &crate::range::RangePlan { ranges },
        operation,
    )
}
fn reopened_refusal_permit(
    snapshot: &RecoverySnapshot,
    capture: &CodedCaptureSnapshot,
) -> RecoveryCleanRefusalPermit {
    let request = capture.dirty_regions.iter().copied().fold(
        RecoveryCleanRequest::new(snapshot.topology_epoch, FenceDomain(1), snapshot.generation),
        RecoveryCleanRequest::with_region,
    );
    let request = capture
        .checksum_extents
        .iter()
        .copied()
        .fold(request, RecoveryCleanRequest::with_checksum_extent);
    match evaluate_recovery_clean(
        snapshot,
        &request,
        &release_certificate(snapshot.topology_epoch),
    )
    .expect("reopened refusal evaluation")
    {
        RecoveryCleanDecision::Refused { receipt, .. } => receipt,
        RecoveryCleanDecision::Clear { .. } => {
            panic!("inherited Open capture unexpectedly had complete CLEAN evidence")
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ModelClaim {
    units: HashSet<String>,
    complete: bool,
    validated: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum ModelOperationPhase {
    Unadmitted,
    Held,
    EffectPossible,
    Released,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ModelOperation {
    phase: ModelOperationPhase,
    claim: ModelClaim,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum ModelCapturePhase {
    CaptureAbsent,
    CaptureOpen,
    CaptureCommitPending,
    CaptureCommitUnknown,
    CaptureCleanKnown,
    CaptureRefused,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum ModelMembership {
    CaptureNotMember,
    CaptureIncluded,
    CaptureLater,
    CaptureLaterDurableAfterClean,
    CaptureLaterDurableStalesClean,
    CaptureLaterRejected,
    CaptureLaterUnknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum ModelSatisfaction {
    CaptureSatisfactionNotObserved,
    CaptureSatisfactionAccepted,
    CaptureSatisfactionRejected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ModelCapture {
    phase: ModelCapturePhase,
    scope: HashSet<String>,
    membership: HashMap<String, ModelMembership>,
    satisfaction: ModelSatisfaction,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct BridgeState {
    operations: HashMap<String, ModelOperation>,
    captures: HashMap<String, ModelCapture>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProjectionFault {
    DropActiveClaim,
    MarkEffectPossibleBeforeAdmission,
    SwapCaptureMembership,
    SuppressRelease,
}

struct BridgeDriver {
    service: Option<HealthyPortableService<FakeStore, MemoryRecoveryStore>>,
    tokens: HashMap<String, OperationSlotToken>,
    claims: HashMap<String, BTreeSet<CodedUnitId>>,
    release_observations: HashMap<String, OperationSlotToken>,
    projection_fault: Option<ProjectionFault>,
    clean_authorization: Option<CodedCaptureCleanAuthorization>,
    clean_certificate: Option<FenceCertificate>,
    clean_attempt: Option<CodedOwnerCleanAttempt>,
    later_cut_attempt: Option<CodedOwnerLaterCutAttempt>,
}

impl BridgeDriver {
    fn new() -> Self {
        Self::with_projection_fault(None)
    }

    fn with_projection_fault(projection_fault: Option<ProjectionFault>) -> Self {
        Self {
            service: None,
            tokens: HashMap::new(),
            claims: HashMap::new(),
            release_observations: HashMap::new(),
            projection_fault,
            clean_authorization: None,
            clean_certificate: None,
            clean_attempt: None,
            later_cut_attempt: None,
        }
    }

    fn init(&mut self) {
        self.service = Some(fake_service(FakeRead::Exact, ServiceConfig::default()));
        self.tokens.clear();
        self.claims.clear();
        self.release_observations.clear();
        self.clean_authorization = None;
        self.clean_certificate = None;
        self.clean_attempt = None;
        self.later_cut_attempt = None;
    }

    fn service(&self) -> Result<&HealthyPortableService<FakeStore, MemoryRecoveryStore>> {
        self.service
            .as_ref()
            .context("coded Connect driver has no service")
    }

    fn service_mut(
        &mut self,
    ) -> Result<&mut HealthyPortableService<FakeStore, MemoryRecoveryStore>> {
        self.service
            .as_mut()
            .context("coded Connect driver has no mutable service")
    }

    fn token(&self, operation: &str) -> Result<OperationSlotToken> {
        self.tokens
            .get(operation)
            .copied()
            .with_context(|| format!("operation {operation} has no production token"))
    }

    fn admit(&mut self, operation: &str, units: &[u32]) -> Result<()> {
        if self.tokens.contains_key(operation) {
            bail!("operation {operation} was admitted twice");
        }
        let data_slot = match operation {
            "opA" => 0,
            "opB" => 1,
            "opC" => 2,
            _ => bail!("unknown model operation {operation}"),
        };
        let ranges = units
            .iter()
            .copied()
            .map(|unit| {
                ByteRange::new(u64::from(unit) * u64::from(BLOCK), u64::from(BLOCK))
                    .expect("bounded model coded unit")
            })
            .collect::<Vec<_>>();
        let plan = crate::range::RangePlan { ranges };
        let request = request(
            RequestId(100 + data_slot as u64),
            TopologyEpoch(4),
            data_slot,
            BlockOp::Write,
            ByteRange::new(
                u64::from(*units.first().context("empty model coded claim")?) * u64::from(BLOCK),
                u64::try_from(units.len()).unwrap() * u64::from(BLOCK),
            )
            .expect("bounded model request"),
            DurabilityIntent::Ordinary,
        );
        let token = self.service_mut()?.reserve(request)?;
        let claim = self
            .service()?
            .coded_claim_for_plan(data_slot, request, &plan, token)?;
        let expected = units
            .iter()
            .copied()
            .map(|unit| CodedUnitId(u64::from(unit)))
            .collect::<BTreeSet<_>>();
        if claim.units() != &expected {
            bail!("production geometry owner disagreed with the model coded claim");
        }
        if !matches!(
            self.service_mut()?.coded_admit(token, claim)?,
            CodedAdmissionOutcome::Admitted(_)
        ) {
            bail!("model admission mapped to production contention");
        }
        self.tokens.insert(operation.to_owned(), token);
        self.claims.insert(operation.to_owned(), expected);
        Ok(())
    }

    fn sampled_step(&mut self, choice: i64) -> Result<()> {
        match choice {
            0 => self.admit("opA", &[0]),
            1 => self.admit("opB", &[0, 1]),
            2 => self.admit("opC", &[1]),
            3 => self.start_capture(),
            4 => self.permit_effect("opA"),
            5 => self.permit_effect("opB"),
            6 => self.permit_effect("opC"),
            7 => self.release_operation("opA"),
            8 => self.release_operation("opB"),
            9 => self.release_operation("opC"),
            _ => bail!("sampled Connect choice {choice} is outside the bounded action set"),
        }
    }
    fn start_capture(&mut self) -> Result<()> {
        self.service_mut()?.coded_start_capture(
            CodedCaptureId(0),
            [RegionId(0)],
            [IntegrityExtentId(0)],
        )?;
        Ok(())
    }

    fn accept_capture(&mut self) -> Result<()> {
        if self.clean_authorization.is_some() || self.clean_attempt.is_some() {
            bail!("CLEAN owner decided the capture twice");
        }
        let (authorization, certificate) =
            prepare_connect_clean_authorization(self.service_mut()?, None)?;
        self.clean_authorization = Some(authorization);
        self.clean_certificate = Some(certificate);
        Ok(())
    }

    fn reject_capture(&mut self) -> Result<()> {
        if self.clean_authorization.is_some() || self.clean_attempt.is_some() {
            bail!("CLEAN owner decided the capture twice");
        }
        self.persist_clean_resolution(false)?;
        self.clean_certificate = None;
        Ok(())
    }
    fn request_clean(&mut self) -> Result<()> {
        let authorization = self
            .clean_authorization
            .take()
            .context("CLEAN was requested before owner acceptance")?;
        let (authorization, certificate) =
            prepare_connect_clean_authorization(self.service_mut()?, Some(authorization))?;
        let prepared = self
            .service()?
            .coded_captures
            .prepare_clean_commit(authorization)?;
        self.clean_certificate = Some(certificate);
        self.clean_attempt = Some(prepared.into_attempt());
        Ok(())
    }

    fn clean_commit(&mut self, observation: CodedCleanCommitObservation) -> Result<()> {
        let mut attempt = self
            .clean_attempt
            .take()
            .context("CLEAN commit was observed without an exact prepared transaction")?;
        match observation {
            CodedCleanCommitObservation::Durable => {
                self.clean_certificate
                    .take()
                    .context("prepared CLEAN transaction lost its fence certificate")?;
                commit_connect_clean(self.service_mut()?, attempt.into_pending_prepared()?)?;
            }
            CodedCleanCommitObservation::Unknown | CodedCleanCommitObservation::Rejected => {
                attempt.observe(observation)?;
                if observation == CodedCleanCommitObservation::Rejected {
                    self.clean_certificate = None;
                }
                self.clean_attempt = Some(attempt);
            }
        }
        Ok(())
    }

    fn reconcile_clean(&mut self, reconciliation: CodedCleanReconciliation) -> Result<()> {
        let mut attempt = self
            .clean_attempt
            .take()
            .context("CLEAN reconciliation had no exact unknown transaction")?;
        match reconciliation {
            CodedCleanReconciliation::Durable => {
                self.clean_certificate
                    .take()
                    .context("unknown CLEAN transaction lost its fence certificate")?;
                commit_connect_clean(self.service_mut()?, attempt.into_unknown_prepared()?)?;
            }
            CodedCleanReconciliation::Rejected => {
                attempt.reject_unknown()?;
                self.clean_certificate = None;
                self.clean_attempt = Some(attempt);
            }
        }
        Ok(())
    }

    fn persist_clean_resolution(&mut self, clean: bool) -> Result<()> {
        persist_test_clean_resolution(self.service_mut()?, clean)
    }

    fn later_cut(&mut self, observation: CodedLaterCutObservation) -> Result<()> {
        let prepared = self.prepare_later_cut()?;
        match observation {
            CodedLaterCutObservation::DurableAfterClean
            | CodedLaterCutObservation::DurableStalesClean => {
                commit_connect_later_cut(self.service_mut()?, prepared)?;
            }
            CodedLaterCutObservation::Unknown | CodedLaterCutObservation::Rejected => {
                self.later_cut_attempt = Some(prepared.into_attempt(observation)?);
            }
        }
        Ok(())
    }

    fn later_cut_reconcile(&mut self, observation: CodedLaterCutReconciliation) -> Result<()> {
        let mut attempt = self
            .later_cut_attempt
            .take()
            .context("later-cut reconciliation had no exact unknown transition")?;
        match observation {
            CodedLaterCutReconciliation::DurableAfterClean
            | CodedLaterCutReconciliation::StalesClean => {
                commit_connect_later_cut(self.service_mut()?, attempt.into_unknown_prepared()?)?;
            }
            CodedLaterCutReconciliation::Rejected => {
                attempt.reject_unknown()?;
                self.later_cut_attempt = Some(attempt);
            }
        }
        Ok(())
    }

    fn prepare_later_cut(&mut self) -> Result<PreparedCodedOwnerTransition<PreparedCodedLaterCut>> {
        let operation = self.token("opC")?;
        let service = self.service_mut()?;
        let current = service.recovery.load_assembly_snapshot()?;
        Ok(service.coded_captures.prepare_later_cut(
            CodedCaptureId(0),
            operation,
            service.topology.topology_epoch(),
            current.generation,
        )?)
    }
    fn permit_effect(&mut self, operation: &str) -> Result<()> {
        let token = self.token(operation)?;
        if !matches!(
            self.service_mut()?.coded_permit_effect(token)?,
            CodedEffectOutcome::Permitted(_)
        ) {
            bail!("model effect permission remained blocked in production");
        }
        let service = self.service_mut()?;
        let current = service.recovery.load_assembly_snapshot()?;
        let mut txn = service
            .recovery
            .begin_protocol_txn(current.generation, service.topology.topology_epoch());
        txn.mark_region_dirty(RegionId(0), current.generation)
            .mark_integrity_stale(IntegrityExtentId(0), current.generation);
        let receipt = service.recovery.commit_durable_receipt(txn)?;
        service.checksums.recovery_generation = receipt.generation();
        Ok(())
    }

    fn release_operation(&mut self, operation: &str) -> Result<()> {
        let token = self.token(operation)?;
        let certificate = release_certificate(self.service()?.topology.topology_epoch());
        let authorization = self
            .service_mut()?
            .establish_release_authorization_for_test(&authoritative_release(token), &certificate)?
            .context("lifecycle owner withheld release authorization")?;
        if authorization.operation() != token {
            bail!("lifecycle owner authorized the wrong generation");
        }
        if self
            .service()?
            .coded_authority()
            .active_claim(token)
            .is_some()
        {
            bail!("coded release left the production claim active");
        }
        self.release_observations
            .insert(operation.to_owned(), authorization.operation());
        Ok(())
    }

    fn cleanup_operation(&mut self, operation: &str) -> Result<()> {
        let token = self.token(operation)?;
        self.service_mut()?.compact_resolved_capture_membership()?;
        self.service_mut()?.release(token)?;
        Ok(())
    }

    fn cleanup_capture(&mut self) -> Result<()> {
        self.service_mut()?.retire_resolved_captures()?;
        if self
            .service()?
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_some()
        {
            bail!("resolved capture remained after durable cleanup");
        }
        self.clean_authorization = None;
        self.clean_certificate = None;
        self.clean_attempt = None;
        self.later_cut_attempt = None;
        Ok(())
    }

    fn probe_conflict(&mut self) -> Result<()> {
        let hidden = self.service_mut()?.reserve(request(
            RequestId(900),
            TopologyEpoch(4),
            1,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).expect("bounded hidden range"),
            DurabilityIntent::Ordinary,
        ))?;
        let claim = issued_coded_unit_claim(self.service()?, hidden, [CodedUnitId(0)])?;
        let outcome = self.service_mut()?.coded_admit(hidden, claim)?;
        if !matches!(outcome, CodedAdmissionOutcome::Contended)
            || self.service()?.operation_readiness(hidden)
                != Some(OperationReadiness::Waiting(
                    PendingReason::AdmissionContended,
                ))
        {
            bail!("overlapping coded claim did not remain pending as contention");
        }
        self.service_mut()?.admission.reclaim(hidden, false)?;
        Ok(())
    }

    fn probe_incomplete_claim(&mut self) -> Result<()> {
        if issued_coded_claim(self.service()?, OperationSlotToken::new(0, 1), 0, []).is_ok() {
            bail!("production geometry owner accepted an empty coded claim");
        }
        Ok(())
    }

    fn probe_unvalidated_claim(&mut self) -> Result<()> {
        let end = self.service()?.topology.geometry().protected_length();
        let outside = ByteRange::new(end, u64::from(BLOCK)).expect("non-empty outside range");
        if issued_coded_claim(self.service()?, OperationSlotToken::new(0, 1), 0, [outside]).is_ok()
        {
            bail!("production geometry owner accepted an out-of-range coded claim");
        }
        Ok(())
    }

    fn probe_unauthorized_removal(&mut self) -> Result<()> {
        let operation = self.token("opA")?;
        let service = self.service_mut()?;
        if service.admission.release_permit(operation).is_ok()
            || service.coded_authority().active_claim(operation).is_none()
        {
            bail!("coded removal became possible without lifecycle release authority");
        }
        Ok(())
    }

    fn probe_stale_generation(&mut self) -> Result<()> {
        let old = self.service_mut()?.reserve(request(
            RequestId(901),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            ByteRange::new(u64::from(BLOCK), u64::from(BLOCK)).expect("bounded hidden range"),
            DurabilityIntent::Ordinary,
        ))?;
        self.service_mut()?.admission.reclaim(old, false)?;
        let current = self.service_mut()?.reserve(request(
            RequestId(902),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            ByteRange::new(u64::from(BLOCK), u64::from(BLOCK)).expect("bounded hidden range"),
            DurabilityIntent::Ordinary,
        ))?;
        if old.index != current.index || old.generation == current.generation {
            bail!("hidden stale-generation probe did not reuse one slot with a new generation");
        }
        let claim = issued_coded_unit_claim(self.service()?, current, [CodedUnitId(1)])?;
        if !matches!(
            self.service_mut()?.coded_admit(current, claim)?,
            CodedAdmissionOutcome::Admitted(_)
        ) {
            bail!("stale-generation probe failed to admit its current generation");
        }
        let service = self.service_mut()?;
        if service.admission.release_permit(old).is_ok() {
            bail!("stale generation produced a lifecycle release permit");
        }
        let certificate = release_certificate(service.topology.topology_epoch());
        let authorization = service
            .establish_release_authorization_for_test(
                &authoritative_release(current),
                &certificate,
            )?
            .context("lifecycle owner withheld the current release authorization")?;
        if authorization.operation() != current {
            bail!("lifecycle owner authorized the stale generation");
        }
        service.release(current)?;
        Ok(())
    }

    fn probe_non_transitive(&mut self) -> Result<()> {
        let hidden = self.service_mut()?.reserve(request(
            RequestId(903),
            TopologyEpoch(4),
            1,
            BlockOp::Write,
            ByteRange::new(0, 2 * u64::from(BLOCK)).expect("bounded hidden range"),
            DurabilityIntent::Ordinary,
        ))?;
        let claim =
            issued_coded_unit_claim(self.service()?, hidden, [CodedUnitId(0), CodedUnitId(1)])?;
        let outcome = self.service_mut()?.coded_admit(hidden, claim)?;
        if !matches!(outcome, CodedAdmissionOutcome::Contended) {
            bail!("non-transitive coded overlap did not report contention");
        }
        let op_a = self.token("opA")?;
        let op_c = self.token("opC")?;
        if self
            .service()?
            .coded_authority()
            .active_claim(op_a)
            .is_none()
            || self
                .service()?
                .coded_authority()
                .active_claim(op_c)
                .is_none()
        {
            bail!("non-transitive conflict disturbed an admitted claim");
        }
        self.service_mut()?.admission.reclaim(hidden, false)?;
        Ok(())
    }

    fn probe_effect_before_later_cut(&mut self) -> Result<()> {
        let token = self.token("opC")?;
        let result = self.service_mut()?.coded_permit_effect(token)?;
        if result != CodedEffectOutcome::BlockedByCapture
            || self.service()?.operation_readiness(token)
                != Some(OperationReadiness::Waiting(PendingReason::CaptureBlocked))
        {
            bail!("effect was permitted before a later cut");
        }
        Ok(())
    }

    fn probe_effect_after_later_rejection(&mut self) -> Result<()> {
        let token = self.token("opC")?;
        let result = self.service_mut()?.coded_permit_effect(token)?;
        if result != CodedEffectOutcome::BlockedByCapture
            || self.service()?.operation_readiness(token)
                != Some(OperationReadiness::Waiting(PendingReason::CaptureBlocked))
        {
            bail!("effect was permitted after a rejected later cut");
        }
        Ok(())
    }

    fn probe_rejected_future_admission(&mut self) -> Result<()> {
        let hidden = self.service_mut()?.reserve(request(
            RequestId(906),
            TopologyEpoch(4),
            1,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).expect("bounded hidden range"),
            DurabilityIntent::Ordinary,
        ))?;
        let claim = issued_coded_unit_claim(self.service()?, hidden, [CodedUnitId(0)])?;
        if !matches!(
            self.service_mut()?.coded_admit(hidden, claim)?,
            CodedAdmissionOutcome::Admitted(_)
        ) {
            bail!("future admission unexpectedly contended after capture refusal");
        }
        let snapshot = self
            .service()?
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .context("rejected capture disappeared")?;
        if !snapshot.membership.is_empty() {
            bail!("rejected capture retained a future membership obligation");
        }
        let service = self.service_mut()?;
        let certificate = release_certificate(service.topology.topology_epoch());
        let authorization = service
            .establish_release_authorization_for_test(&authoritative_release(hidden), &certificate)?
            .context("lifecycle owner withheld rejected-capture release")?;
        if authorization.operation() != hidden {
            bail!("lifecycle owner authorized the wrong rejected-capture generation");
        }
        service.release(hidden)?;
        Ok(())
    }

    fn probe_clean_before_later_reconciliation(&mut self) -> Result<()> {
        let operation = self.token("opC")?;
        if self.later_cut_attempt.as_ref().is_none_or(|attempt| {
            attempt.operation() != operation
                || attempt.phase() != CodedLaterCutAttemptPhase::Unknown
        }) {
            bail!("production later-cut attempt did not retain uncertainty");
        }
        Ok(())
    }
    fn probe_clean_before_decision(&mut self) -> Result<()> {
        let decision = self
            .service()?
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .and_then(|capture| capture.decision);
        if decision.is_some() || self.clean_authorization.is_some() {
            bail!("CLEAN-owner decision was observed unexpectedly");
        }
        Ok(())
    }

    fn bridge_state(&self) -> Result<BridgeState> {
        let mut operations = HashMap::new();
        for id in ["opA", "opB", "opC"] {
            let phase = if matches!(
                (
                    self.release_observations.get(id).copied(),
                    self.tokens.get(id).copied(),
                ),
                (Some(released), Some(token)) if released == token
            ) {
                ModelOperationPhase::Released
            } else if let Some(token) = self.tokens.get(id) {
                match self.service()?.coded_authority().operation_phase(*token) {
                    Some(CodedOperationPhase::Held) => ModelOperationPhase::Held,
                    Some(CodedOperationPhase::EffectPossible) => {
                        ModelOperationPhase::EffectPossible
                    }
                    None => ModelOperationPhase::Unadmitted,
                }
            } else {
                ModelOperationPhase::Unadmitted
            };
            let claim_units = if matches!(phase, ModelOperationPhase::Unadmitted) {
                HashSet::new()
            } else if let Some(token) = self.tokens.get(id) {
                if let Some(claim) = self.service()?.coded_authority().active_claim(*token) {
                    claim.units().iter().map(|unit| unit_name(*unit)).collect()
                } else {
                    self.claims
                        .get(id)
                        .into_iter()
                        .flat_map(|units| units.iter().map(|unit| unit_name(*unit)))
                        .collect()
                }
            } else {
                HashSet::new()
            };
            operations.insert(
                id.to_owned(),
                ModelOperation {
                    claim: ModelClaim {
                        complete: !matches!(phase, ModelOperationPhase::Unadmitted),
                        validated: !matches!(phase, ModelOperationPhase::Unadmitted),
                        units: claim_units,
                    },
                    phase,
                },
            );
        }

        let capture = self
            .service()?
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0));
        let mut membership = [
            ("opA".to_owned(), ModelMembership::CaptureNotMember),
            ("opB".to_owned(), ModelMembership::CaptureNotMember),
            ("opC".to_owned(), ModelMembership::CaptureNotMember),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let scope = if let Some(snapshot) = capture.as_ref() {
            let reverse = self
                .tokens
                .iter()
                .map(|(id, token)| (*token, id.as_str()))
                .collect::<HashMap<_, _>>();
            for (operation, status) in &snapshot.membership {
                let id = reverse
                    .get(operation)
                    .with_context(|| format!("model has no operation for {operation:?}"))?;
                membership.insert((*id).to_owned(), model_membership(*status));
            }
            // The bounded Quint model names two units; ignore concrete units outside that universe.
            snapshot
                .scope
                .iter()
                .copied()
                .filter(|unit| unit.0 < 2)
                .map(unit_name)
                .collect()
        } else {
            HashSet::new()
        };
        if let Some(attempt) = self.later_cut_attempt.as_ref() {
            let id = self
                .tokens
                .iter()
                .find_map(|(id, token)| (*token == attempt.operation()).then_some(id))
                .context("model has no operation for pending later cut")?;
            let status = match attempt.phase() {
                CodedLaterCutAttemptPhase::Unknown => ModelMembership::CaptureLaterUnknown,
                CodedLaterCutAttemptPhase::Rejected => ModelMembership::CaptureLaterRejected,
            };
            membership.insert(id.clone(), status);
        }
        let phase = self.clean_attempt.as_ref().map_or_else(
            || {
                capture
                    .as_ref()
                    .map_or(ModelCapturePhase::CaptureAbsent, |snapshot| {
                        model_capture_phase(snapshot.phase)
                    })
            },
            |attempt| match attempt.phase() {
                CodedCleanAttemptPhase::Pending => ModelCapturePhase::CaptureCommitPending,
                CodedCleanAttemptPhase::Unknown => ModelCapturePhase::CaptureCommitUnknown,
                CodedCleanAttemptPhase::Refused => ModelCapturePhase::CaptureRefused,
            },
        );
        let satisfaction = match capture.as_ref().and_then(|snapshot| snapshot.decision) {
            Some(CodedCaptureDecision::Accepted) => ModelSatisfaction::CaptureSatisfactionAccepted,
            Some(CodedCaptureDecision::Rejected) => ModelSatisfaction::CaptureSatisfactionRejected,
            None if self.clean_authorization.is_some() || self.clean_attempt.is_some() => {
                ModelSatisfaction::CaptureSatisfactionAccepted
            }
            None => ModelSatisfaction::CaptureSatisfactionNotObserved,
        };
        let mut captures = HashMap::new();
        captures.insert(
            "capture0".to_owned(),
            ModelCapture {
                phase,
                scope,
                membership,
                satisfaction,
            },
        );
        let mut state = BridgeState {
            operations,
            captures,
        };
        if let Some(fault) = self.projection_fault {
            match fault {
                ProjectionFault::DropActiveClaim => {
                    state.operations.get_mut("opA").unwrap().claim.units.clear();
                }
                ProjectionFault::MarkEffectPossibleBeforeAdmission => {
                    state.operations.get_mut("opA").unwrap().phase =
                        ModelOperationPhase::EffectPossible;
                }
                ProjectionFault::SwapCaptureMembership => {
                    let capture = state.captures.get_mut("capture0").unwrap();
                    let op_a = capture.membership.get("opA").copied().unwrap();
                    let op_c = capture.membership.get("opC").copied().unwrap();
                    capture.membership.insert("opA".to_owned(), op_c);
                    capture.membership.insert("opC".to_owned(), op_a);
                }
                ProjectionFault::SuppressRelease => {
                    if state.operations.get("opA").unwrap().phase == ModelOperationPhase::Released {
                        state.operations.get_mut("opA").unwrap().phase = ModelOperationPhase::Held;
                    }
                }
            }
        }
        Ok(state)
    }
}

fn model_capture_phase(phase: CodedCapturePhase) -> ModelCapturePhase {
    match phase {
        CodedCapturePhase::Open => ModelCapturePhase::CaptureOpen,
        CodedCapturePhase::CleanCommitPending => ModelCapturePhase::CaptureCommitPending,
        CodedCapturePhase::CleanCommitUnknown => ModelCapturePhase::CaptureCommitUnknown,
        CodedCapturePhase::CleanKnown => ModelCapturePhase::CaptureCleanKnown,
        CodedCapturePhase::Refused => ModelCapturePhase::CaptureRefused,
    }
}

fn prepare_connect_clean_authorization<R: RecoveryStateStore>(
    service: &mut HealthyPortableService<FakeStore, R>,
    prior_authorization: Option<CodedCaptureCleanAuthorization>,
) -> Result<(CodedCaptureCleanAuthorization, FenceCertificate)> {
    let current = service.recovery.load_assembly_snapshot()?;
    let capture = service
        .coded_captures
        .capture_snapshot(CodedCaptureId(0))
        .context("coded capture is missing")?;
    let request = capture.dirty_regions.iter().copied().fold(
        RecoveryCleanRequest::new(
            service.topology.topology_epoch(),
            FenceDomain(1),
            current.generation,
        ),
        |request, region| request.with_region(region),
    );
    let request = capture
        .checksum_extents
        .iter()
        .copied()
        .fold(request, RecoveryCleanRequest::with_checksum_extent);
    let mut certificate = FenceCertificate::new(
        service.topology.topology_epoch(),
        FenceDomain(1),
        vec![StoreFenceRef {
            fence_id: FenceId(1),
            store_id: StoreId(1),
            topology_epoch: service.topology.topology_epoch(),
            store_incarnation: StoreIncarnationId(0),
            through: StoreWriteWatermark(0),
            capability_evidence_id: CapabilityEvidenceId(1),
        }],
        capture
            .dirty_regions
            .iter()
            .copied()
            .map(|region| (region, current.generation))
            .collect(),
    );
    for extent in capture.checksum_extents.iter().copied() {
        certificate = certificate.with_integrity_extent(extent, current.generation);
    }
    let permit = match evaluate_recovery_clean(&current, &request, &certificate)? {
        RecoveryCleanDecision::Clear(permit) => permit,
        RecoveryCleanDecision::Refused { .. } => {
            bail!("complete CLEAN evidence was unexpectedly refused")
        }
    };
    let authorization = match prior_authorization {
        Some(authorization) => {
            service
                .coded_captures
                .refresh_clean_authorization(&current, authorization, permit)?
        }
        None => {
            let included = capture
                .membership
                .iter()
                .filter(|(_, membership)| **membership == CodedCaptureMembership::Included)
                .map(|(operation, _)| {
                    service
                        .released_coded_lifecycle_evidence(CodedCaptureId(0), *operation)
                        .context(
                            "Connect CLEAN acceptance requires production release-owner evidence",
                        )
                })
                .collect::<Result<Vec<_>>>()?;
            service
                .coded_captures
                .authorize_clean(&current, CodedCaptureId(0), permit, included)?
        }
    };
    Ok((authorization, certificate))
}

fn commit_connect_clean<R: RecoveryStateStore>(
    service: &mut HealthyPortableService<FakeStore, R>,
    prepared: PreparedCodedOwnerTransition<PreparedCodedCleanCommit>,
) -> Result<()> {
    let current = service.recovery.load_assembly_snapshot()?;
    let mut txn = RecoveryTxn::new(current.generation, service.topology.topology_epoch());
    txn.push(RecoveryMutation::ApplyCodedTransition {
        transition: prepared.transition(),
    });
    let receipt = service.recovery.commit_durable_receipt(txn)?;
    service
        .coded_captures
        .confirm_clean_commit(prepared, &receipt)?;
    service.checksums.recovery_generation = receipt.generation();
    Ok(())
}

fn commit_connect_later_cut<R: RecoveryStateStore>(
    service: &mut HealthyPortableService<FakeStore, R>,
    prepared: PreparedCodedOwnerTransition<PreparedCodedLaterCut>,
) -> Result<()> {
    let target = prepared.invalidation_target().clone();
    commit_connect_later_cut_with_target(service, prepared, target)
}

fn commit_connect_later_cut_with_target<R: RecoveryStateStore>(
    service: &mut HealthyPortableService<FakeStore, R>,
    prepared: PreparedCodedOwnerTransition<PreparedCodedLaterCut>,
    target: InvalidationTarget,
) -> Result<()> {
    let transition = prepared.transition();
    let result = service
        .checksums
        .invalidate_with_write_recovery_record_and_coded_transitions(
            &mut service.recovery,
            target,
            [transition],
        )?;
    let receipt = result
        .receipt
        .context("Connect later cut omitted its write-recovery receipt")?;
    service
        .coded_captures
        .confirm_later_cut(prepared, &receipt)?;
    Ok(())
}

fn persist_test_clean_resolution<R: RecoveryStateStore>(
    service: &mut HealthyPortableService<FakeStore, R>,
    clean: bool,
) -> Result<()> {
    let current = service.recovery.load_assembly_snapshot()?;
    let capture = service
        .coded_captures
        .capture_snapshot(CodedCaptureId(0))
        .context("coded capture is missing")?;
    let request = capture.dirty_regions.iter().copied().fold(
        RecoveryCleanRequest::new(
            service.topology.topology_epoch(),
            FenceDomain(1),
            current.generation,
        ),
        |request, region| request.with_region(region),
    );
    let request = capture
        .checksum_extents
        .iter()
        .copied()
        .fold(request, RecoveryCleanRequest::with_checksum_extent);
    let mut certificate = FenceCertificate::new(
        service.topology.topology_epoch(),
        FenceDomain(1),
        vec![StoreFenceRef {
            fence_id: FenceId(1),
            store_id: StoreId(1),
            topology_epoch: service.topology.topology_epoch(),
            store_incarnation: StoreIncarnationId(0),
            through: StoreWriteWatermark(0),
            capability_evidence_id: CapabilityEvidenceId(1),
        }],
        if clean {
            capture
                .dirty_regions
                .iter()
                .copied()
                .map(|region| (region, current.generation))
                .collect()
        } else {
            Vec::new()
        },
    );
    if clean {
        for extent in capture.checksum_extents.iter().copied() {
            certificate = certificate.with_integrity_extent(extent, current.generation);
        }
    }
    match evaluate_recovery_clean(&current, &request, &certificate)? {
        RecoveryCleanDecision::Clear(permit) if clean => {
            let included = capture
                .membership
                .iter()
                .filter(|(_, membership)| **membership == CodedCaptureMembership::Included)
                .map(|(operation, _)| {
                    service
                        .included_lifecycle_authorization_for_test(&authoritative_release(
                            *operation,
                        ))?
                        .map(|authorization| (authorization, certificate.clone()))
                        .context(
                            "Connect CLEAN resolution requires production lifecycle-owner evidence",
                        )
                })
                .collect::<Result<Vec<_>>>()?;
            let authorization = service.coded_captures.authorize_clean(
                &current,
                CodedCaptureId(0),
                permit,
                included,
            )?;
            let prepared = service.coded_captures.prepare_clean_commit(authorization)?;
            let mut txn = RecoveryTxn::new(current.generation, service.topology.topology_epoch());
            txn.push(RecoveryMutation::ApplyCodedTransition {
                transition: prepared.transition(),
            });
            let receipt = service.recovery.commit_durable_receipt(txn)?;
            service
                .coded_captures
                .confirm_clean_commit(prepared, &receipt)?;
            service.checksums.recovery_generation = receipt.generation();
        }
        RecoveryCleanDecision::Refused { receipt, .. } if !clean => {
            let prepared = service
                .coded_captures
                .prepare_clean_refusal(CodedCaptureId(0), &receipt)?;
            let mut txn = RecoveryTxn::new(current.generation, service.topology.topology_epoch());
            txn.push(RecoveryMutation::ApplyCodedTransition {
                transition: prepared.transition(),
            });
            let receipt = service.recovery.commit_durable_receipt(txn)?;
            service
                .coded_captures
                .confirm_clean_refusal(prepared, &receipt)?;
            service.checksums.recovery_generation = receipt.generation();
        }
        RecoveryCleanDecision::Clear(_) => {
            bail!("incomplete CLEAN evidence unexpectedly cleared")
        }
        RecoveryCleanDecision::Refused { .. } => {
            bail!("complete CLEAN evidence was unexpectedly refused")
        }
    }
    Ok(())
}

impl State<BridgeDriver> for BridgeState {
    fn from_driver(driver: &BridgeDriver) -> Result<Self> {
        driver.bridge_state()
    }
}

impl Driver for BridgeDriver {
    type State = BridgeState;

    fn config() -> Config {
        Config {
            state: &["CodedRangeCleanConnect::CodedRangeClean::state"],
            ..Config::default()
        }
    }

    fn step(&mut self, step: &Step) -> Result {
        quint_connect::switch!(step {
            connectInit => self.init(),
            sampledStep(choice: i64) => self.sampled_step(choice)?,
            admitOpA => self.admit("opA", &[0])?,
            admitOpB => self.admit("opB", &[0, 1])?,
            admitOpC => self.admit("opC", &[1])?,
            startCapturePath => self.start_capture()?,
            acceptCapture => self.accept_capture()?,
            rejectCapture => self.reject_capture()?,
            requestCleanPath => self.request_clean()?,
            cleanDurable => self.clean_commit(CodedCleanCommitObservation::Durable)?,
            cleanUnknown => self.clean_commit(CodedCleanCommitObservation::Unknown)?,
            cleanRejected => self.clean_commit(CodedCleanCommitObservation::Rejected)?,
            cleanReconciledDurable => {
                self.reconcile_clean(CodedCleanReconciliation::Durable)?
            },
            cleanReconciledRejected => {
                self.reconcile_clean(CodedCleanReconciliation::Rejected)?
            },
            laterAfterClean => self.later_cut(CodedLaterCutObservation::DurableAfterClean)?,
            laterStalesClean => self.later_cut(CodedLaterCutObservation::DurableStalesClean)?,
            laterUnknown => self.later_cut(CodedLaterCutObservation::Unknown)?,
            laterRejected => self.later_cut_reconcile(CodedLaterCutReconciliation::Rejected)?,
            laterRejectedObservation => {
                self.later_cut(CodedLaterCutObservation::Rejected)?
            },
            laterStalesCleanReconciled => {
                self.later_cut_reconcile(CodedLaterCutReconciliation::StalesClean)?
            },
            laterAfterCleanReconciled => {
                self.later_cut_reconcile(CodedLaterCutReconciliation::DurableAfterClean)?
            },
            permitOpA => self.permit_effect("opA")?,
            permitOpB => self.permit_effect("opB")?,
            permitOpC => self.permit_effect("opC")?,
            probeEffectAfterLaterRejection => self.probe_effect_after_later_rejection()?,
            removeOpA => self.release_operation("opA")?,
            removeOpB => self.release_operation("opB")?,
            removeOpC => self.release_operation("opC")?,
            compactOpA => self.cleanup_operation("opA")?,
            cleanupCapturePath => self.cleanup_capture()?,
            cleanupRefusedCapturePath => self.cleanup_capture()?,
            cleanupCleanCapturePath => self.cleanup_capture()?,
            probeConflict => self.probe_conflict()?,
            probeIncompleteClaim => self.probe_incomplete_claim()?,
            probeUnvalidatedClaim => self.probe_unvalidated_claim()?,
            probeUnauthorizedRemoval => self.probe_unauthorized_removal()?,
            probeStaleGeneration => self.probe_stale_generation()?,
            probeNonTransitive => self.probe_non_transitive()?,
            probeEffectBeforeLaterCut => self.probe_effect_before_later_cut()?,
            probeRejectedFutureAdmission => self.probe_rejected_future_admission()?,
            probeCleanBeforeLaterReconciliation => self.probe_clean_before_later_reconciliation()?,
            probeCleanBeforeDecision => self.probe_clean_before_decision()?,
        })
    }
}

fn unit_name(unit: CodedUnitId) -> String {
    format!("u{}", unit.0)
}

#[test]
fn coded_admission_rejects_claim_for_another_request() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    let operation = service
        .reserve(request(
            RequestId(949),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_claim(&service, operation, 0, [range]).unwrap();
    let error = service.coded_admit(operation, claim).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("not bound to the admitted request")
    );
    assert!(service.coded_authority().active_claim(operation).is_none());
}

#[test]
fn coded_admission_rejects_claim_from_foreign_topology_owner() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let range = ByteRange::new(0, BLOCK as u64).unwrap();
    let request = request(
        RequestId(950),
        TopologyEpoch(4),
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let operation = service.reserve(request).unwrap();
    let assignments = service
        .topology
        .assignments()
        .iter()
        .map(|assignment| {
            TopologyAssignment::new(
                assignment.slot_id(),
                assignment.role(),
                assignment.coding_position(),
                assignment.assignment_instance(),
                AssignmentGeneration(2),
            )
        })
        .collect();
    let foreign_topology = TopologySnapshot::new(
        service.topology.array_id(),
        service.topology.topology_epoch(),
        service.topology.profile(),
        service.topology.geometry(),
        assignments,
    )
    .unwrap();
    let foreign_assignment = foreign_topology
        .assignment_for_slot(request.slot_id)
        .cloned()
        .unwrap();
    let foreign_owner =
        CodedGeometryOwner::new(foreign_topology, DIRTY_REGION_BYTES, BLAKE3_256_PROFILE).unwrap();
    let claim = foreign_owner
        .claim_for_mutation(operation, request, &foreign_assignment, [range])
        .unwrap();

    let error = service.coded_admit(operation, claim).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("current topology and geometry owner")
    );
    assert!(service.coded_authority().active_claim(operation).is_none());
}

#[test]
fn paired_range_rejects_claim_from_foreign_geometry_owner() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let range = ByteRange::new(DIRTY_REGION_BYTES / 2, u64::from(BLOCK)).unwrap();
    let request = request(
        RequestId(951),
        TopologyEpoch(4),
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let operation = service.reserve(request).unwrap();
    let assignment = service
        .topology
        .assignment_for_slot(request.slot_id)
        .cloned()
        .unwrap();
    let canonical_claim = service
        .coded_geometry
        .claim_for_mutation(operation, request, &assignment, [range])
        .unwrap();
    let foreign_owner = CodedGeometryOwner::new(
        service.topology.clone(),
        DIRTY_REGION_BYTES / 2,
        BLAKE3_256_PROFILE,
    )
    .unwrap();
    let foreign_claim = foreign_owner
        .claim_for_mutation(operation, request, &assignment, [range])
        .unwrap();

    let mut canonical_range = CodedRangeAuthority::new(&service.coded_geometry);
    let CodedAdmissionOutcome::Admitted(canonical_admission) = canonical_range
        .admit(operation, request, canonical_claim)
        .unwrap()
    else {
        panic!("canonical claim unexpectedly contended");
    };
    let mut foreign_range = CodedRangeAuthority::new(&foreign_owner);
    let CodedAdmissionOutcome::Admitted(foreign_admission) = foreign_range
        .admit(operation, request, foreign_claim.clone())
        .unwrap()
    else {
        panic!("foreign claim unexpectedly contended in its own graph");
    };
    assert_ne!(
        canonical_admission.invalidation_target(),
        foreign_admission.invalidation_target()
    );

    let current = service.recovery.load_assembly_snapshot().unwrap();
    assert!(matches!(
        service.coded_captures.prepare_admission(
            operation,
            request,
            foreign_claim,
            current.generation,
        ),
        Err(CodedLifecycleError::Authority(
            CodedAuthorityError::ForeignGeometryAuthority
        ))
    ));
    assert!(service.coded_authority().active_claim(operation).is_none());
}

fn model_membership(membership: CodedCaptureMembership) -> ModelMembership {
    match membership {
        CodedCaptureMembership::Included => ModelMembership::CaptureIncluded,
        CodedCaptureMembership::Later => ModelMembership::CaptureLater,
        CodedCaptureMembership::LaterDurableAfterClean => {
            ModelMembership::CaptureLaterDurableAfterClean
        }
        CodedCaptureMembership::LaterDurableStalesClean => {
            ModelMembership::CaptureLaterDurableStalesClean
        }
        CodedCaptureMembership::LaterRejected => ModelMembership::CaptureLaterRejected,
        CodedCaptureMembership::LaterUnknown => ModelMembership::CaptureLaterUnknown,
    }
}

#[test]
fn coded_release_certificate_survives_slot_cleanup_and_rejects_stale_generation() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let range = ByteRange::new(0, BLOCK as u64).expect("bounded coded release range");
    let old = service
        .reserve(request(
            RequestId(950),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&service, old, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        service.coded_admit(old, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));
    // Lifecycle composition produces the only exact-generation release permit.
    let old_authorization = service
        .establish_release_authorization(&authoritative_release(old))
        .unwrap()
        .expect("complete lifecycle facts authorize the old generation");
    assert_eq!(old_authorization.operation(), old);
    service.release_after_reclaim(old).unwrap();

    let current = service
        .reserve(request(
            RequestId(951),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    assert_eq!(old.index, current.index);
    assert_ne!(old.generation, current.generation);
    let claim = issued_coded_unit_claim(&service, current, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        service.coded_admit(current, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));
    let error = service
        .coded_release_claim_with_certificate(current, &old_authorization, None)
        .unwrap_err();
    assert!(error.to_string().contains("does not match"), "{error}");
    assert!(service.coded_authority().active_claim(current).is_some());
}

#[test]
fn coded_release_rejects_authorization_from_foreign_owner_domain() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let operation = service
        .reserve(request(
            RequestId(9_503),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).expect("bounded coded release range"),
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&service, operation, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        service.coded_admit(operation, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));
    service
        .admission
        .record_reconciliation(operation, ReconciliationOutcome::Durable)
        .unwrap();
    let permit = service.admission.release_permit(operation).unwrap();
    let (foreign_owner, _) = LifecycleAuthorityOwner::new();
    let authorization = foreign_owner
        .authorize_release(
            service.admission.lifecycle_slots(),
            operation,
            true,
            true,
            true,
            true,
        )
        .expect("foreign lifecycle owner sees complete facts in its own domain");
    let authorization = ReleaseAuthorization::from_lifecycle(authorization);

    let current = service.recovery.load_assembly_snapshot().unwrap();
    assert!(matches!(
        service.coded_captures.prepare_release(
            &authorization,
            permit,
            None,
            current.generation,
        ),
        Err(CodedLifecycleError::Authority(
            CodedAuthorityError::ReleaseAuthorityDomainMismatch(found)
        )) if found == operation
    ));
    assert!(service.coded_authority().active_claim(operation).is_some());
}
#[test]
fn coded_same_generation_cannot_be_readmitted_after_release_before_slot_cleanup() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let range = ByteRange::new(0, BLOCK as u64).expect("bounded coded re-admission range");
    let operation = service
        .reserve(request(
            RequestId(952),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&service, operation, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        service.coded_admit(operation, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));

    // Establish the real lifecycle certificate without running physical slot
    // cleanup, preserving the exact-generation pre-cleanup window.
    let authorization = service
        .establish_release_authorization(&authoritative_release(operation))
        .unwrap()
        .expect("complete lifecycle facts authorize coded release");
    assert_eq!(authorization.operation(), operation);

    let claim = issued_coded_unit_claim(&service, operation, [CodedUnitId(0)]).unwrap();
    let error = service.coded_admit(operation, claim).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("coded admission requires a reserved operation slot")
    );
    assert!(service.coded_authority().active_claim(operation).is_none());
}

#[test]
fn reopened_capture_refusal_survives_failed_commit_and_restart() {
    let epoch = TopologyEpoch(4);
    let open_service = |recovery: MemoryRecoveryStore| {
        let topology = topology(epoch);
        let members = topology
            .assignments()
            .iter()
            .enumerate()
            .map(|(index, assignment)| {
                let store_id = StoreId(index as u64 + 1);
                MemberBinding::new(
                    assignment,
                    epoch,
                    store_id,
                    FakeStore::new(store_id, epoch, FakeRead::Exact),
                )
            })
            .collect();
        HealthyPortableService::open(topology, members, recovery, ServiceConfig::default()).unwrap()
    };

    let mut service = open_service(MemoryRecoveryStore::new(epoch));
    let operation = service
        .reserve(request(
            RequestId(953),
            epoch,
            0,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).unwrap(),
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&service, operation, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        service.coded_admit(operation, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));
    service
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();

    let recovery = service.recovery.clone();
    drop(service);
    let mut reopened = open_service(recovery);
    assert_eq!(reopened.state(), ServiceState::Recovering);
    assert_eq!(
        reopened
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .unwrap()
            .membership
            .get(&operation),
        Some(&CodedCaptureMembership::Included)
    );

    let current = reopened.recovery.load_assembly_snapshot().unwrap();
    let request = RecoveryCleanRequest::new(epoch, FenceDomain(1), current.generation)
        .with_region(RegionId(0))
        .with_checksum_extent(IntegrityExtentId(0));
    let certificate = FenceCertificate::new(
        epoch,
        FenceDomain(1),
        vec![dwv_store::StoreFenceRef {
            fence_id: dwv_store::FenceId(1),
            store_id: StoreId(1),
            topology_epoch: epoch,
            store_incarnation: dwv_store::StoreIncarnationId(0),
            through: StoreWriteWatermark(0),
            capability_evidence_id: dwv_store::CapabilityEvidenceId(1),
        }],
        Vec::new(),
    );
    let refusal = match evaluate_recovery_clean(&current, &request, &certificate).unwrap() {
        RecoveryCleanDecision::Refused { receipt, .. } => receipt,
        RecoveryCleanDecision::Clear { .. } => panic!("incomplete evidence unexpectedly cleared"),
    };

    reopened.recovery.set_health(RecoveryStoreHealth::Corrupt);
    assert!(
        reopened
            .resolve_reopened_coded_capture(CodedCaptureId(0), Some(refusal.clone()))
            .is_err()
    );
    assert!(
        reopened
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_some()
    );

    let mut recovery = reopened.recovery.clone();
    recovery.set_health(RecoveryStoreHealth::Healthy);
    drop(reopened);
    let mut retried = open_service(recovery);
    retried
        .resolve_reopened_coded_capture(CodedCaptureId(0), Some(refusal))
        .unwrap();
    assert!(
        retried
            .recovery
            .load_assembly_snapshot()
            .unwrap()
            .coded_captures
            .is_empty()
    );

    let recovery = retried.recovery.clone();
    drop(retried);
    let reopened_after_cleanup = open_service(recovery);
    assert!(
        !reopened_after_cleanup
            .coded_captures()
            .has_unresolved_reopen_state()
    );
}

#[test]
fn reopened_refusal_lost_ack_installs_neither_candidate_until_reopen() {
    let epoch = TopologyEpoch(4);
    let initial = LostAckRecovery::new(epoch);
    let mut service =
        fake_service_with_recovery(FakeRead::Exact, initial.clone(), ServiceConfig::default());
    let operation = service
        .reserve(request(
            RequestId(954),
            epoch,
            0,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).unwrap(),
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&service, operation, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        service.coded_admit(operation, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));
    service
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();

    let inherited = LostAckRecovery::from_store(initial.durable_store());
    drop(service);
    let mut reopened =
        fake_service_with_recovery(FakeRead::Exact, inherited.clone(), ServiceConfig::default());
    let current = reopened.recovery.load_assembly_snapshot().unwrap();
    let request = RecoveryCleanRequest::new(epoch, FenceDomain(1), current.generation)
        .with_region(RegionId(0))
        .with_checksum_extent(IntegrityExtentId(0));
    let certificate = FenceCertificate::new(
        epoch,
        FenceDomain(1),
        vec![StoreFenceRef {
            fence_id: FenceId(1),
            store_id: StoreId(1),
            topology_epoch: epoch,
            store_incarnation: StoreIncarnationId(0),
            through: StoreWriteWatermark(0),
            capability_evidence_id: CapabilityEvidenceId(1),
        }],
        Vec::new(),
    );
    let refusal = match evaluate_recovery_clean(&current, &request, &certificate).unwrap() {
        RecoveryCleanDecision::Refused { receipt, .. } => receipt,
        RecoveryCleanDecision::Clear { .. } => panic!("incomplete evidence unexpectedly cleared"),
    };

    inherited.lose_next_commit();
    assert!(
        reopened
            .resolve_reopened_coded_capture(CodedCaptureId(0), Some(refusal))
            .is_err()
    );
    assert!(
        reopened
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_some(),
        "lost acknowledgement must not install the process-local removal"
    );

    let durable = inherited.durable_store();
    drop(reopened);
    let reconciled = fake_service_with_recovery(FakeRead::Exact, durable, ServiceConfig::default());
    assert!(
        reconciled
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_none(),
        "reopen must accept the exact durably committed successor"
    );
}

fn started_lost_ack_coded_service(
    request_id: u64,
) -> (
    HealthyPortableService<FakeStore, LostAckRecovery>,
    LostAckRecovery,
    OperationSlotToken,
) {
    let epoch = TopologyEpoch(4);
    let recovery = LostAckRecovery::new(epoch);
    let mut service =
        fake_service_with_recovery(FakeRead::Exact, recovery.clone(), ServiceConfig::default());
    let operation = service
        .reserve(request(
            RequestId(request_id),
            epoch,
            0,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).unwrap(),
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&service, operation, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        service.coded_admit(operation, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));
    service
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();
    (service, recovery, operation)
}

#[test]
fn coded_lifecycle_lost_ack_reconciles_release_compaction_and_cleanup_on_reopen() {
    let (mut release, release_recovery, release_operation) = started_lost_ack_coded_service(955);
    release_recovery.lose_next_commit();
    let release_receipt = release_certificate(release.topology.topology_epoch());
    assert!(
        release
            .establish_release_authorization_for_test(
                &authoritative_release(release_operation),
                &release_receipt,
            )
            .is_err()
    );
    assert!(
        release
            .coded_authority()
            .active_claim(release_operation)
            .is_some(),
        "lost acknowledgement must not install the process-local release"
    );
    let durable = release_recovery.durable_store();
    drop(release);
    let reopened_release =
        fake_service_with_recovery(FakeRead::Exact, durable, ServiceConfig::default());
    assert!(
        reopened_release
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .unwrap()
            .release_authorized_operations
            .contains(&release_operation),
        "reopen must accept the durably committed release successor"
    );

    let (mut compaction, compaction_recovery, compaction_operation) =
        started_lost_ack_coded_service(956);
    persist_test_clean_resolution(&mut compaction, false).unwrap();
    let compaction_certificate = release_certificate(compaction.topology.topology_epoch());
    compaction
        .establish_release_authorization_for_test(
            &authoritative_release(compaction_operation),
            &compaction_certificate,
        )
        .unwrap()
        .unwrap();
    compaction_recovery.lose_next_commit();
    assert!(compaction.compact_resolved_capture_membership().is_err());
    assert_eq!(
        compaction
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .unwrap()
            .membership
            .get(&compaction_operation),
        Some(&CodedCaptureMembership::Included),
        "lost acknowledgement must not install process-local compaction"
    );
    let durable = compaction_recovery.durable_store();
    drop(compaction);
    let reopened_compaction =
        fake_service_with_recovery(FakeRead::Exact, durable, ServiceConfig::default());
    assert!(
        reopened_compaction
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .unwrap()
            .membership
            .is_empty(),
        "reopen must accept the durably committed compaction successor"
    );

    let (mut cleanup, cleanup_recovery, cleanup_operation) = started_lost_ack_coded_service(957);
    persist_test_clean_resolution(&mut cleanup, true).unwrap();
    let cleanup_certificate = release_certificate(cleanup.topology.topology_epoch());
    cleanup
        .establish_release_authorization_for_test(
            &authoritative_release(cleanup_operation),
            &cleanup_certificate,
        )
        .unwrap()
        .unwrap();
    cleanup.compact_resolved_capture_membership().unwrap();
    cleanup_recovery.lose_next_commit();
    assert!(cleanup.retire_resolved_captures().is_err());
    assert!(
        cleanup
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_some(),
        "lost acknowledgement must not install process-local capture cleanup"
    );
    let durable = cleanup_recovery.durable_store();
    drop(cleanup);
    let reopened_cleanup =
        fake_service_with_recovery(FakeRead::Exact, durable, ServiceConfig::default());
    assert!(
        reopened_cleanup
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_none(),
        "reopen must accept the durably committed cleanup successor"
    );
}
#[test]
fn persisted_refused_cleanup_handles_known_noncommit_and_lost_ack() {
    let (mut known, known_recovery, _) = started_lost_ack_coded_service(958);
    persist_test_clean_resolution(&mut known, false).unwrap();
    let mut known = fake_service_with_recovery(
        FakeRead::Exact,
        known_recovery.durable_store(),
        ServiceConfig::default(),
    );
    assert_eq!(
        known
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .unwrap()
            .phase,
        CodedCapturePhase::Refused
    );
    known.recovery.set_health(RecoveryStoreHealth::Corrupt);
    assert!(
        known
            .resolve_reopened_coded_capture(CodedCaptureId(0), None)
            .is_err()
    );
    assert!(
        known
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_some()
    );
    known.recovery.set_health(RecoveryStoreHealth::Healthy);
    known
        .resolve_reopened_coded_capture(CodedCaptureId(0), None)
        .unwrap();

    let (mut lost, lost_recovery, _) = started_lost_ack_coded_service(959);
    persist_test_clean_resolution(&mut lost, false).unwrap();
    let inherited = LostAckRecovery::from_store(lost_recovery.durable_store());
    drop(lost);
    let mut reopened =
        fake_service_with_recovery(FakeRead::Exact, inherited.clone(), ServiceConfig::default());
    inherited.lose_next_commit();
    assert!(
        reopened
            .resolve_reopened_coded_capture(CodedCaptureId(0), None)
            .is_err()
    );
    assert!(
        reopened
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_some()
    );
    let durable = inherited.durable_store();
    drop(reopened);
    let reconciled = fake_service_with_recovery(FakeRead::Exact, durable, ServiceConfig::default());
    assert!(
        reconciled
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_none()
    );
}

#[test]
fn inherited_clean_known_abandonment_handles_known_noncommit_and_lost_ack() {
    let (mut known, known_recovery, _) = started_lost_ack_coded_service(960);
    persist_test_clean_resolution(&mut known, true).unwrap();
    let mut known = fake_service_with_recovery(
        FakeRead::Exact,
        known_recovery.durable_store(),
        ServiceConfig::default(),
    );
    let inherited = known
        .coded_captures()
        .capture_snapshot(CodedCaptureId(0))
        .unwrap();
    assert_eq!(inherited.phase, CodedCapturePhase::CleanKnown);
    assert!(!inherited.membership.is_empty());
    known.recovery.set_health(RecoveryStoreHealth::Corrupt);
    assert!(
        known
            .resolve_reopened_coded_capture(CodedCaptureId(0), None)
            .is_err()
    );
    assert!(
        known
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_some()
    );
    known.recovery.set_health(RecoveryStoreHealth::Healthy);
    known
        .resolve_reopened_coded_capture(CodedCaptureId(0), None)
        .unwrap();

    let (mut lost, lost_recovery, _) = started_lost_ack_coded_service(961);
    persist_test_clean_resolution(&mut lost, true).unwrap();
    let inherited = LostAckRecovery::from_store(lost_recovery.durable_store());
    drop(lost);
    let mut reopened =
        fake_service_with_recovery(FakeRead::Exact, inherited.clone(), ServiceConfig::default());
    inherited.lose_next_commit();
    assert!(
        reopened
            .resolve_reopened_coded_capture(CodedCaptureId(0), None)
            .is_err()
    );
    assert!(
        reopened
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_some()
    );
    let durable = inherited.durable_store();
    drop(reopened);
    let reconciled = fake_service_with_recovery(FakeRead::Exact, durable, ServiceConfig::default());
    assert!(
        reconciled
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_none()
    );
}

#[test]
fn repeated_crash_before_empty_clean_cleanup_blocks_admission_until_retired() {
    let (mut service, recovery, operation) = started_lost_ack_coded_service(962);
    persist_test_clean_resolution(&mut service, true).unwrap();
    let certificate = release_certificate(service.topology.topology_epoch());
    service
        .establish_release_authorization_for_test(&authoritative_release(operation), &certificate)
        .unwrap()
        .unwrap();
    service.compact_resolved_capture_membership().unwrap();
    let capture = service
        .coded_captures()
        .capture_snapshot(CodedCaptureId(0))
        .unwrap();
    assert_eq!(capture.phase, CodedCapturePhase::CleanKnown);
    assert!(capture.membership.is_empty());

    let durable = recovery.durable_store();
    drop(service);
    let first_reopen =
        fake_service_with_recovery(FakeRead::Exact, durable, ServiceConfig::default());
    assert_eq!(first_reopen.state(), ServiceState::Recovering);
    let durable = first_reopen.recovery.clone();
    drop(first_reopen);

    let mut second_reopen =
        fake_service_with_recovery(FakeRead::Exact, durable, ServiceConfig::default());
    assert_eq!(second_reopen.state(), ServiceState::Recovering);
    second_reopen
        .resolve_reopened_coded_capture(CodedCaptureId(0), None)
        .unwrap();
    let durable = second_reopen.recovery.clone();
    drop(second_reopen);

    let serving = fake_service_with_recovery(FakeRead::Exact, durable, ServiceConfig::default());
    assert_eq!(serving.state(), ServiceState::Recovering);
    assert!(
        serving
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .is_none()
    );
    assert!(
        serving
            .recovery()
            .load_assembly_snapshot()
            .unwrap()
            .dirty_regions
            .iter()
            .any(|region| region.state != dwv_recovery::RegionState::Clean),
        "unverifiable retained history must invalidate the claimed clean state"
    );
}

#[test]
fn coded_capture_and_admission_linearize_at_service_boundary() {
    let range = ByteRange::new(0, BLOCK as u64).expect("bounded coded range");

    let mut capture_first = fake_service(FakeRead::Exact, ServiceConfig::default());
    capture_first
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();
    let later = capture_first
        .reserve(request(
            RequestId(960),
            TopologyEpoch(4),
            1,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&capture_first, later, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        capture_first.coded_admit(later, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));
    assert_eq!(
        capture_first
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .unwrap()
            .membership
            .get(&later),
        Some(&CodedCaptureMembership::Later)
    );

    let mut admit_first = fake_service(FakeRead::Exact, ServiceConfig::default());
    let included = admit_first
        .reserve(request(
            RequestId(961),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&admit_first, included, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        admit_first.coded_admit(included, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));
    admit_first
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();
    assert_eq!(
        admit_first
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .unwrap()
            .membership
            .get(&included),
        Some(&CodedCaptureMembership::Included)
    );
}

#[test]
fn capture_start_uses_the_paired_geometry_scope() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let foreign_geometry = CodedGeometryOwner::new(
        service.topology.clone(),
        DIRTY_REGION_BYTES / 2,
        BLAKE3_256_PROFILE,
    )
    .unwrap();
    let foreign_scope = foreign_geometry.capture_scope([RegionId(0)], []).unwrap();
    let paired_scope = service
        .coded_geometry
        .capture_scope([RegionId(0)], [])
        .unwrap();
    assert_ne!(foreign_scope.units(), paired_scope.units());

    service
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [])
        .unwrap();
    let snapshot = service
        .coded_captures()
        .capture_snapshot(CodedCaptureId(0))
        .unwrap();
    assert_eq!(&snapshot.scope, paired_scope.units());
    assert_ne!(&snapshot.scope, foreign_scope.units());
}

#[test]
fn lifecycle_effect_authority_rejects_unresolved_capture_ordering() {
    let range = ByteRange::new(0, BLOCK as u64).expect("bounded coded range");

    let mut observed = fake_service(FakeRead::Exact, ServiceConfig::default());
    observed
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();
    let observed_operation = observed
        .reserve(request(
            RequestId(962),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            range,
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&observed, observed_operation, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        observed.coded_admit(observed_operation, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));
    assert_eq!(
        observed
            .coded_captures()
            .capture_snapshot(CodedCaptureId(0))
            .unwrap()
            .membership
            .get(&observed_operation),
        Some(&CodedCaptureMembership::Later)
    );
    assert_eq!(
        observed
            .coded_captures
            .permit_effect(observed_operation)
            .unwrap(),
        None
    );

    assert_eq!(
        observed.coded_captures.operation_phase(observed_operation),
        Some(CodedOperationPhase::Held)
    );
}

#[test]
fn lifecycle_owner_rejects_stale_prepared_range_successor() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let first = service
        .reserve(request(
            RequestId(963),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).unwrap(),
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let second = service
        .reserve(request(
            RequestId(964),
            TopologyEpoch(4),
            1,
            BlockOp::Write,
            ByteRange::new(BLOCK as u64, BLOCK as u64).unwrap(),
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let first_request = service.admission.snapshot(first).unwrap().request;
    let second_request = service.admission.snapshot(second).unwrap().request;
    let first_claim = issued_coded_unit_claim(&service, first, [CodedUnitId(0)]).unwrap();
    let second_claim = issued_coded_unit_claim(&service, second, [CodedUnitId(1)]).unwrap();
    let generation = service
        .recovery
        .load_assembly_snapshot()
        .unwrap()
        .generation;
    let first_prepared = service
        .coded_captures
        .prepare_admission(first, first_request, first_claim, generation)
        .unwrap();
    let stale_prepared = service
        .coded_captures
        .prepare_admission(second, second_request, second_claim, generation)
        .unwrap();

    assert!(matches!(
        service
            .coded_captures
            .confirm_admission(first_prepared, None),
        Ok(CodedAdmissionOutcome::Admitted(_))
    ));
    assert_eq!(
        service
            .coded_captures
            .confirm_admission(stale_prepared, None),
        Err(CodedLifecycleError::PreparedRangeStale)
    );
    assert!(service.coded_captures.active_claim(first).is_some());
    assert!(service.coded_captures.active_claim(second).is_none());
}

#[test]
fn lifecycle_owner_rejects_foreign_prepared_capture_successor() {
    let mut source = fake_service(FakeRead::Exact, ServiceConfig::default());
    source
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();
    let (authorization, _) = prepare_connect_clean_authorization(&mut source, None).unwrap();
    let prepared = source
        .coded_captures
        .prepare_clean_commit(authorization)
        .unwrap();

    let mut destination = fake_service(FakeRead::Exact, ServiceConfig::default());
    destination
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();
    let destination_snapshot = destination.recovery.load_assembly_snapshot().unwrap();
    let mut txn = RecoveryTxn::new(
        destination_snapshot.generation,
        destination.topology.topology_epoch(),
    );
    txn.push(RecoveryMutation::ApplyCodedTransition {
        transition: prepared.transition(),
    });
    let receipt = destination.recovery.commit_durable_receipt(txn).unwrap();

    assert_eq!(
        destination
            .coded_captures
            .confirm_clean_commit(prepared, &receipt),
        Err(CodedCaptureError::ForeignCaptureAuthority)
    );
}

#[test]
fn capture_start_holds_owner_until_durable_installation() {
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    service
        .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
        .unwrap();

    assert_eq!(service.coded_captures.capture_count(), 1);
    assert!(
        service
            .coded_captures
            .capture_snapshot(CodedCaptureId(0))
            .is_some()
    );
}

#[test]
fn rejected_capture_persistence_keeps_the_live_range_predecessor() {
    let epoch = TopologyEpoch(4);
    let recovery = LostAckRecovery::new(epoch);
    let mut service =
        fake_service_with_recovery(FakeRead::Exact, recovery.clone(), ServiceConfig::default());
    let prior_high_water = service.coded_captures.admission_high_water();

    recovery.reject_commit_after(0);
    assert!(
        service
            .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
            .is_err()
    );

    assert_eq!(service.state(), ServiceState::Serving);
    assert_eq!(
        service.coded_captures.admission_high_water(),
        prior_high_water
    );
    assert_eq!(service.coded_captures.capture_count(), 0);
    assert!(service.coded_captures.is_usable());
    assert!(
        recovery
            .durable_store()
            .load_assembly_snapshot()
            .unwrap()
            .coded_captures
            .is_empty()
    );
}

#[test]
fn lost_capture_ack_invalidates_process_local_coded_authority() {
    let epoch = TopologyEpoch(4);
    let recovery = LostAckRecovery::new(epoch);
    let mut service =
        fake_service_with_recovery(FakeRead::Exact, recovery.clone(), ServiceConfig::default());
    let operation = service
        .reserve(request(
            RequestId(965),
            epoch,
            0,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).unwrap(),
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&service, operation, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        service.coded_admit(operation, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));

    recovery.lose_next_commit();
    assert!(
        service
            .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
            .is_err()
    );

    assert_eq!(service.state(), ServiceState::Recovering);
    assert!(!service.coded_captures.is_usable());
    assert_eq!(
        service.coded_captures.permit_effect(operation).unwrap(),
        None
    );
    assert!(matches!(
        service.coded_captures.prepare_later_cut(
            CodedCaptureId(0),
            operation,
            epoch,
            RecoveryGeneration::ZERO,
        ),
        Err(CodedLifecycleError::AuthorityInvalidated)
    ));
    assert!(
        recovery
            .durable_store()
            .load_assembly_snapshot()
            .unwrap()
            .coded_captures
            .iter()
            .any(|snapshot| snapshot.capture == CodedCaptureId(0))
    );
}

#[test]
fn mismatched_durable_capture_receipt_invalidates_process_local_coded_authority() {
    let epoch = TopologyEpoch(4);
    let recovery = LostAckRecovery::new(epoch);
    let mut service =
        fake_service_with_recovery(FakeRead::Exact, recovery.clone(), ServiceConfig::default());
    let operation = service
        .reserve(request(
            RequestId(966),
            epoch,
            0,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).unwrap(),
            DurabilityIntent::Ordinary,
        ))
        .unwrap();
    let claim = issued_coded_unit_claim(&service, operation, [CodedUnitId(0)]).unwrap();
    assert!(matches!(
        service.coded_admit(operation, claim).unwrap(),
        CodedAdmissionOutcome::Admitted(_)
    ));

    recovery.misreport_next_generation();
    assert!(
        service
            .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
            .is_err()
    );

    assert_eq!(service.state(), ServiceState::Recovering);
    assert!(!service.coded_captures.is_usable());
    assert_eq!(
        service.coded_captures.permit_effect(operation).unwrap(),
        None
    );
    assert!(
        recovery
            .durable_store()
            .load_assembly_snapshot()
            .unwrap()
            .coded_captures
            .iter()
            .any(|snapshot| snapshot.capture == CodedCaptureId(0))
    );
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "admissionStep",
    max_samples = 1,
    max_steps = 6,
    seed = "22082026"
)]
fn coded_range_connect_admission() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "wideClaimStep",
    max_samples = 1,
    max_steps = 3,
    seed = "22082026"
)]
fn coded_range_connect_wide_claim() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "captureStep",
    max_samples = 1,
    max_steps = 10,
    seed = "22082026"
)]
fn coded_range_connect_capture() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "cleanRetirementStep",
    max_samples = 1,
    max_steps = 9,
    seed = "22082026"
)]
fn coded_range_connect_clean_retirement() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "unknownCleanStep",
    max_samples = 1,
    max_steps = 11,
    seed = "22082026"
)]
fn coded_range_connect_unknown_clean() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "rejectedCaptureStep",
    max_samples = 1,
    max_steps = 4,
    seed = "22082026"
)]
fn coded_range_connect_rejected_capture() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "staleLaterCutStep",
    max_samples = 1,
    max_steps = 7,
    seed = "22082026"
)]
fn coded_range_connect_stale_later_cut() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "preCleanUnknownStep",
    max_samples = 1,
    max_steps = 8,
    seed = "22082026"
)]
fn coded_range_connect_pre_clean_unknown() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "laterRejectedStep",
    max_samples = 1,
    max_steps = 7,
    seed = "22082026"
)]
fn coded_range_connect_later_rejected() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "nonTransitiveStep",
    max_samples = 1,
    max_steps = 3,
    seed = "22082026"
)]
fn coded_range_connect_non_transitive() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "cleanReconcileStep",
    max_samples = 1,
    max_steps = 8,
    seed = "22082026"
)]
fn coded_range_connect_clean_reconcile() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "cleanReconcileRejectedStep",
    max_samples = 1,
    max_steps = 7,
    seed = "22082026"
)]
fn coded_range_connect_clean_reconcile_rejected() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "unknownAfterCleanReconcileStep",
    max_samples = 1,
    max_steps = 10,
    seed = "22082026"
)]
fn coded_range_connect_later_reconcile_after_clean() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "cleanRejectedStep",
    max_samples = 1,
    max_steps = 6,
    seed = "22082026"
)]
fn coded_range_connect_clean_rejected() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "noCleanStep",
    max_samples = 1,
    max_steps = 3,
    seed = "22082026"
)]
fn coded_range_connect_clean_requires_decision() -> impl Driver {
    BridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/CodedRangeCleanConnect.qnt",
    main = "CodedRangeCleanConnect",
    init = "connectInit",
    step = "negativeStep",
    max_samples = 1,
    max_steps = 6,
    seed = "90210"
)]
fn coded_range_connect_negative_paths() -> impl Driver {
    BridgeDriver::new()
}

fn connect_fault_is_rejected(fault: ProjectionFault, step: &str, max_steps: usize) -> bool {
    let config = quint_connect::runner::Config {
        test_name: format!("coded_range_connect_fault_{fault:?}"),
        gen_config: quint_connect::runner::RunConfig {
            spec: "../../verification/quint/CodedRangeCleanConnect.qnt".to_owned(),
            main: Some("CodedRangeCleanConnect".to_owned()),
            init: Some("connectInit".to_owned()),
            step: Some(step.to_owned()),
            max_samples: Some(1),
            max_steps: Some(max_steps),
            seed: "20260825".to_owned(),
        },
    };
    quint_connect::runner::run_test(BridgeDriver::with_projection_fault(Some(fault)), config)
        .is_err()
}

#[test]
fn coded_range_connect_rejects_false_projections() {
    assert!(connect_fault_is_rejected(
        ProjectionFault::MarkEffectPossibleBeforeAdmission,
        "admissionStep",
        1,
    ));
    assert!(connect_fault_is_rejected(
        ProjectionFault::DropActiveClaim,
        "admissionStep",
        1,
    ));
    assert!(connect_fault_is_rejected(
        ProjectionFault::SwapCaptureMembership,
        "captureStep",
        3,
    ));
    assert!(connect_fault_is_rejected(
        ProjectionFault::SuppressRelease,
        "admissionStep",
        5,
    ));
}

#[test]
fn coded_range_connect_rejects_later_cut_outside_write_recovery_target() {
    let mut driver = BridgeDriver::new();
    driver.init();
    driver.admit("opA", &[0]).unwrap();
    driver.start_capture().unwrap();
    driver.admit("opC", &[1]).unwrap();
    let prepared = driver.prepare_later_cut().unwrap();
    let mismatched_target = InvalidationTarget::new(vec![RegionId(1)], vec![IntegrityExtentId(0)]);

    assert!(
        commit_connect_later_cut_with_target(
            driver.service_mut().unwrap(),
            prepared,
            mismatched_target,
        )
        .is_err(),
        "Connect accepted a later cut outside the production write-recovery target"
    );
}
#[test]

fn coded_range_connect_sampled_operation_capture_paths() {
    for seed in [
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
    ] {
        let config = quint_connect::runner::Config {
            test_name: format!("coded_range_connect_sampled_{seed}"),
            gen_config: quint_connect::runner::RunConfig {
                spec: "../../verification/quint/CodedRangeCleanConnect.qnt".to_owned(),
                main: Some("CodedRangeCleanConnect".to_owned()),
                init: Some("connectInit".to_owned()),
                step: Some("sampledStep".to_owned()),
                max_samples: Some(1),
                max_steps: Some(12),
                seed: seed.to_owned(),
            },
        };
        quint_connect::runner::run_test(BridgeDriver::new(), config)
            .unwrap_or_else(|error| panic!("sampled Connect seed {seed} failed: {error}"));
    }
}

/// Production-side correspondence for the task-4.2 boundary. Basis lifecycle
/// facts remain outside this CodedRangeClean model; these observations come
/// directly from the retained service driver and its coded authority.
#[test]
fn coded_range_connect_protected_write_holds_claim_through_basis_and_write() {
    let epoch = TopologyEpoch(4);
    let range = ByteRange::new(0, BLOCK as u64).expect("bounded protected range");
    let request = request(
        RequestId(1100),
        epoch,
        0,
        BlockOp::Write,
        range,
        DurabilityIntent::Ordinary,
    );
    let mut service = fake_service(FakeRead::Exact, ServiceConfig::default());
    let submission = service
        .submit_write(request, &[0x5a; BLOCK as usize])
        .expect("protected write admission");
    service
        .grant_basis_read_permission(&submission)
        .expect("basis permission");

    for _ in 0..2 {
        let PortableWriteDrive::Work(work) = service
            .drive_write(&submission)
            .expect("basis work should be runnable")
        else {
            panic!("basis work was not emitted");
        };
        assert_eq!(
            service
                .coded_authority()
                .operation_phase(submission.operation),
            Some(CodedOperationPhase::Held)
        );
        service
            .accept_write_work(&work)
            .expect("basis work acceptance");
        let result = service
            .execute_write_work(&work)
            .expect("basis work execution");
        assert!(
            service
                .deliver_write_result(result)
                .expect("basis completion")
                .is_none()
        );
    }

    let PortableWriteDrive::Work(write) = service
        .drive_write(&submission)
        .expect("protected write work should be runnable")
    else {
        panic!("protected write was not emitted");
    };
    assert_eq!(
        service
            .coded_authority()
            .operation_phase(submission.operation),
        Some(CodedOperationPhase::EffectPossible)
    );
    assert_eq!(service.members[0].store.physical_writes, 0);
    service
        .accept_write_work(&write)
        .expect("protected write acceptance");
    let result = service
        .execute_write_work(&write)
        .expect("protected write execution");
    assert!(
        service
            .deliver_write_result(result)
            .expect("protected write completion")
            .is_none()
    );

    let evidence = finish_retained_write(&mut service, &submission);
    assert!(evidence.release_authorization.is_some());
    assert!(
        service
            .coded_authority()
            .operation_phase(submission.operation)
            .is_none()
    );
}

#[cfg(test)]
mod generated_coded_operation_properties {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::RngSeed;
    use proptest_state_machine::{ReferenceStateMachine, StateMachineTest};
    use std::collections::BTreeMap;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum ClaimKind {
        U0,
        U1,
        U01,
    }

    impl ClaimKind {
        fn units(self) -> BTreeSet<CodedUnitId> {
            match self {
                Self::U0 => [CodedUnitId(0)].into_iter().collect(),
                Self::U1 => [CodedUnitId(1)].into_iter().collect(),
                Self::U01 => [CodedUnitId(0), CodedUnitId(1)].into_iter().collect(),
            }
        }

        fn overlaps(self, other: Self) -> bool {
            self.units().intersection(&other.units()).next().is_some()
        }
    }

    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    struct ReferenceState {
        active: BTreeMap<u8, ClaimKind>,
    }

    #[derive(Clone, Copy, Debug)]
    enum Transition {
        Admit {
            operation: u8,
            data_slot: u8,
            claim: ClaimKind,
        },
        Contended {
            operation: u8,
            data_slot: u8,
            claim: ClaimKind,
        },
        Release {
            operation: u8,
        },
        StaleRelease {
            operation: u8,
        },
        Restart,
    }

    fn claim_strategy() -> impl Strategy<Value = ClaimKind> {
        prop_oneof![
            Just(ClaimKind::U0),
            Just(ClaimKind::U1),
            Just(ClaimKind::U01)
        ]
    }

    struct CodedOperationMachine<const DROP_ACTIVE_CLAIM: bool = false>;

    impl<const DROP_ACTIVE_CLAIM: bool> ReferenceStateMachine
        for CodedOperationMachine<DROP_ACTIVE_CLAIM>
    {
        type State = ReferenceState;
        type Transition = Transition;

        fn init_state() -> BoxedStrategy<Self::State> {
            Just(ReferenceState::default()).boxed()
        }

        fn transitions(_: &Self::State) -> BoxedStrategy<Self::Transition> {
            prop_oneof![
                (0_u8..3, 0_u8..3, claim_strategy()).prop_map(|(operation, data_slot, claim)| {
                    Transition::Admit {
                        operation,
                        data_slot,
                        claim,
                    }
                }),
                (0_u8..3, 0_u8..3, claim_strategy()).prop_map(|(operation, data_slot, claim)| {
                    Transition::Contended {
                        operation,
                        data_slot,
                        claim,
                    }
                }),
                (0_u8..3).prop_map(|operation| Transition::Release { operation }),
                (0_u8..3).prop_map(|operation| Transition::StaleRelease { operation }),
                Just(Transition::Restart),
            ]
            .boxed()
        }

        fn apply(mut state: Self::State, transition: &Self::Transition) -> Self::State {
            match *transition {
                Transition::Admit {
                    operation, claim, ..
                } => {
                    state.active.insert(operation, claim);
                }
                Transition::Release { operation } => {
                    state.active.remove(&operation);
                }
                Transition::Contended { .. } | Transition::StaleRelease { .. } => {}
                Transition::Restart => {}
            }
            state
        }

        fn preconditions(state: &Self::State, transition: &Self::Transition) -> bool {
            match *transition {
                Transition::Admit {
                    operation, claim, ..
                } => {
                    !state.active.contains_key(&operation)
                        && state.active.values().all(|other| !claim.overlaps(*other))
                }
                Transition::Contended {
                    operation, claim, ..
                } => {
                    !state.active.contains_key(&operation)
                        && state.active.values().any(|other| claim.overlaps(*other))
                }
                Transition::Release { operation } | Transition::StaleRelease { operation } => {
                    state.active.contains_key(&operation)
                }
                Transition::Restart => state.active.is_empty(),
            }
        }
    }

    fn open_generated_service(
        recovery: MemoryRecoveryStore,
    ) -> HealthyPortableService<FakeStore, MemoryRecoveryStore> {
        let epoch = TopologyEpoch(4);
        let topology = topology(epoch);
        let members = topology
            .assignments()
            .iter()
            .enumerate()
            .map(|(index, assignment)| {
                let store_id = StoreId(index as u64 + 1);
                MemberBinding::new(
                    assignment,
                    epoch,
                    store_id,
                    FakeStore::new(store_id, epoch, FakeRead::Exact),
                )
            })
            .collect();
        HealthyPortableService::open(topology, members, recovery, ServiceConfig::default()).unwrap()
    }

    struct ConcreteState {
        service: HealthyPortableService<FakeStore, MemoryRecoveryStore>,
        tokens: BTreeMap<u8, OperationSlotToken>,
        next_request_id: u64,
        drop_active_claim: bool,
    }

    impl ConcreteState {
        fn new() -> Self {
            Self::with_drop_active_claim(false)
        }

        fn with_drop_active_claim(drop_active_claim: bool) -> Self {
            let mut service = open_generated_service(MemoryRecoveryStore::new(TopologyEpoch(4)));
            service
                .coded_start_capture(CodedCaptureId(0), [RegionId(0)], [IntegrityExtentId(0)])
                .unwrap();
            Self {
                service,
                tokens: BTreeMap::new(),
                next_request_id: 0,
                drop_active_claim,
            }
        }

        fn reserve(&mut self, data_slot: u8, claim: ClaimKind) -> OperationSlotToken {
            let request_id = RequestId(20_000 + self.next_request_id);
            self.next_request_id += 1;
            let units = claim.units();
            let first = units.first().expect("generated claim is non-empty").0;
            let last = units.last().expect("generated claim is non-empty").0;
            self.service
                .reserve(request(
                    request_id,
                    TopologyEpoch(4),
                    usize::from(data_slot),
                    BlockOp::Write,
                    ByteRange::new(
                        first * u64::from(BLOCK),
                        (last - first + 1) * u64::from(BLOCK),
                    )
                    .expect("bounded generated range"),
                    DurabilityIntent::Ordinary,
                ))
                .expect("generated operation admission")
        }

        fn admit(&mut self, operation: u8, data_slot: u8, claim: ClaimKind) {
            let token = self.reserve(data_slot, claim);
            let input = issued_coded_unit_claim(&self.service, token, claim.units())
                .expect("generated coded claim");
            assert!(matches!(
                self.service
                    .coded_admit(token, input)
                    .expect("generated coded admission"),
                CodedAdmissionOutcome::Admitted(_)
            ));
            assert_eq!(
                self.service.coded_authority().operation_phase(token),
                Some(CodedOperationPhase::Held)
            );
            self.tokens.insert(operation, token);
        }

        fn contend(&mut self, _operation: u8, data_slot: u8, claim: ClaimKind) {
            let token = self.reserve(data_slot, claim);
            let input = issued_coded_unit_claim(&self.service, token, claim.units())
                .expect("generated coded claim");
            assert_eq!(
                self.service
                    .coded_admit(token, input)
                    .expect("generated contention admission"),
                CodedAdmissionOutcome::Contended
            );
            assert_eq!(
                self.service.operation_readiness(token),
                Some(OperationReadiness::Waiting(
                    PendingReason::AdmissionContended
                ))
            );
            self.service
                .admission
                .reclaim(token, false)
                .expect("generated contended operation cleanup");
        }

        fn release(&mut self, operation: u8) {
            let token = self
                .tokens
                .remove(&operation)
                .expect("generated active token");
            let certificate = release_certificate(self.service.topology.topology_epoch());
            let authorization = self
                .service
                .establish_release_authorization_for_test(
                    &authoritative_release(token),
                    &certificate,
                )
                .expect("generated lifecycle composition")
                .expect("generated complete owner facts authorize release");
            assert_eq!(authorization.operation(), token);
            assert!(self.service.coded_authority().active_claim(token).is_none());
            self.service
                .release_after_reclaim(token)
                .expect("generated release cleanup");
        }

        fn reject_stale_release(&mut self, operation: u8) {
            let token = *self.tokens.get(&operation).expect("generated active token");
            let stale_generation = if token.generation == u32::MAX {
                0
            } else {
                token.generation + 1
            };
            let stale = OperationSlotToken::new(token.index, stale_generation);
            let error = self
                .service
                .reconcile_release_authorization(authoritative_release(stale))
                .expect_err("stale lifecycle observations unexpectedly authorized release");
            assert!(error.to_string().contains("generation"), "{error}");
            assert!(self.service.coded_authority().active_claim(token).is_some());
        }

        fn restart(&mut self) {
            let recovery = self.service.recovery.clone();
            let mut reopened = open_generated_service(recovery);
            let captures = reopened.coded_captures().snapshots();
            for capture in captures {
                let refusal = (capture.phase == CodedCapturePhase::Open).then(|| {
                    let current = reopened.recovery.load_assembly_snapshot().unwrap();
                    reopened_refusal_permit(&current, &capture)
                });
                reopened
                    .resolve_reopened_coded_capture(capture.capture, refusal)
                    .expect("generated restart resolves inherited capture");
            }
            let durable = reopened.recovery.load_assembly_snapshot().unwrap();
            assert!(
                durable
                    .dirty_regions
                    .iter()
                    .all(|record| { !matches!(record.state, dwv_recovery::RegionState::Clean) })
            );
            assert!(durable.integrity_records.iter().all(|record| {
                matches!(record.state, dwv_recovery::IntegrityState::Stale { .. })
            }));
            let capture = CodedCaptureId(durable.next_coded_capture_id);
            reopened
                .coded_start_capture(capture, [RegionId(0)], [IntegrityExtentId(0)])
                .expect("generated restart begins a new bounded capture");
            self.service = reopened;
        }

        fn observed_active_claim(
            &self,
            token: OperationSlotToken,
        ) -> Option<BTreeSet<CodedUnitId>> {
            if self.drop_active_claim {
                None
            } else {
                self.service
                    .coded_authority()
                    .active_claim(token)
                    .map(|claim| claim.units().iter().copied().collect())
            }
        }

        fn assert_invariants(&self, expected: &ReferenceState) {
            assert_eq!(self.tokens.len(), expected.active.len());
            for (operation, claim) in &expected.active {
                let token = self.tokens.get(operation).expect("reference token missing");
                assert_eq!(
                    self.service.coded_authority().operation_phase(*token),
                    Some(CodedOperationPhase::Held)
                );
                let actual_units = self
                    .observed_active_claim(*token)
                    .expect("reference claim missing");
                assert_eq!(actual_units, claim.units());
            }
        }
    }

    impl<const DROP_ACTIVE_CLAIM: bool> StateMachineTest for CodedOperationMachine<DROP_ACTIVE_CLAIM> {
        type SystemUnderTest = ConcreteState;
        type Reference = Self;

        fn init_test(_: &ReferenceState) -> Self::SystemUnderTest {
            ConcreteState::with_drop_active_claim(DROP_ACTIVE_CLAIM)
        }

        fn apply(
            mut state: Self::SystemUnderTest,
            ref_state: &ReferenceState,
            transition: Transition,
        ) -> Self::SystemUnderTest {
            match transition {
                Transition::Admit {
                    operation,
                    data_slot,
                    claim,
                } => state.admit(operation, data_slot, claim),
                Transition::Contended {
                    operation,
                    data_slot,
                    claim,
                } => state.contend(operation, data_slot, claim),
                Transition::Release { operation } => state.release(operation),
                Transition::StaleRelease { operation } => state.reject_stale_release(operation),
                Transition::Restart => state.restart(),
            }
            state.assert_invariants(ref_state);
            state
        }

        fn check_invariants(state: &Self::SystemUnderTest, ref_state: &ReferenceState) {
            state.assert_invariants(ref_state);
        }
    }

    proptest_state_machine::prop_state_machine! {
        #![proptest_config(ProptestConfig {
            cases: 10_000,
            max_local_rejects: 400_000,
            failure_persistence: None,
            rng_seed: RngSeed::Fixed(22_082_026),
            ..ProptestConfig::default()
        })]
        #[test]
        fn generated_coded_operation_histories(sequential 1..24 => CodedOperationMachine);
    }

    #[test]
    fn generated_operation_identity_is_independent_of_target_member() {
        let mut concrete = ConcreteState::new();
        concrete.admit(0, 0, ClaimKind::U0);
        concrete.admit(1, 0, ClaimKind::U1);
        concrete.contend(2, 1, ClaimKind::U0);
        assert_eq!(concrete.tokens.len(), 2);
        let first = *concrete.tokens.get(&0).expect("first generated token");
        let second = *concrete.tokens.get(&1).expect("second generated token");
        assert_ne!(first, second);
        concrete.reject_stale_release(0);
        concrete.release(1);
        assert!(
            concrete
                .service
                .coded_authority()
                .active_claim(first)
                .is_some()
        );
        concrete.release(0);
        assert!(
            concrete
                .service
                .coded_authority()
                .active_claim(second)
                .is_none()
        );
    }

    #[test]
    fn minimized_coded_operation_history_replays_deterministically() {
        let transitions = vec![
            Transition::Restart,
            Transition::Admit {
                operation: 0,
                data_slot: 0,
                claim: ClaimKind::U0,
            },
            Transition::Contended {
                operation: 1,
                data_slot: 1,
                claim: ClaimKind::U0,
            },
            Transition::StaleRelease { operation: 0 },
            Transition::Release { operation: 0 },
            Transition::Admit {
                operation: 1,
                data_slot: 0,
                claim: ClaimKind::U1,
            },
            Transition::Release { operation: 1 },
            Transition::Restart,
        ];
        CodedOperationMachine::<false>::test_sequential(
            proptest::test_runner::Config::default(),
            ReferenceState::default(),
            transitions,
            None,
        );
    }

    #[test]
    fn generated_property_rejects_corrupted_observation_and_replays() {
        let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig {
            cases: 8,
            failure_persistence: None,
            ..ProptestConfig::default()
        });
        let result = runner.run(
            &CodedOperationMachine::<true>::sequential_strategy(1..24),
            |(initial, transitions, seen_counter)| {
                CodedOperationMachine::<true>::test_sequential(
                    proptest::test_runner::Config::default(),
                    initial,
                    transitions,
                    seen_counter,
                );
                Ok(())
            },
        );
        match result {
            Err(proptest::test_runner::TestError::Fail(reason, (initial, transitions, _))) => {
                assert!(reason.to_string().contains("reference claim missing"));
                assert_eq!(transitions.len(), 1);
                let replay = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    CodedOperationMachine::<true>::test_sequential(
                        proptest::test_runner::Config::default(),
                        initial,
                        transitions.clone(),
                        None,
                    );
                }));
                let panic = replay.expect_err("corrupted observation unexpectedly passed replay");
                let panic_message = panic
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| panic.downcast_ref::<&str>().copied())
                    .expect("corrupted observation panic had no text");
                assert!(panic_message.contains("reference claim missing"));
            }
            other => panic!("corrupted observation did not produce a shrunk failure: {other:?}"),
        }
    }
}
