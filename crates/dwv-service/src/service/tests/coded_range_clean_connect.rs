use super::*;
use crate::evidence::ReleaseAuthorization;
use anyhow::{Context, bail};
use dwv_core::{BlockOp, ByteRange, CodedUnitId, DurabilityIntent, RequestId, TopologyEpoch};
use dwv_recovery::{
    CodedCaptureDecision, CodedCaptureId, CodedCaptureMembership, CodedCapturePhase,
    CodedCaptureScopeInput, CodedCleanCommitObservation, CodedCleanReconciliation,
    CodedLaterCutObservation, CodedLaterCutReconciliation, MemoryRecoveryStore,
};
use dwv_store::OperationSlotToken;
use dwv_transaction_ref::{
    CodedAdmissionOutcome, CodedAuthorityError, CodedClaimInput, CodedOperationPhase,
};
use quint_connect::{Config, Driver, Result, State, Step, quint_run};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap, HashSet};

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
        }
    }

    fn init(&mut self) {
        self.service = Some(fake_service(FakeRead::Exact, ServiceConfig::default()));
        self.tokens.clear();
        self.claims.clear();
        self.release_observations.clear();
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
        let request = request(
            RequestId(100 + data_slot as u64),
            TopologyEpoch(4),
            data_slot,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).expect("bounded coded range"),
            DurabilityIntent::Ordinary,
        );
        let token = self.service_mut()?.reserve(request)?;
        let claim = units
            .iter()
            .copied()
            .map(|unit| CodedUnitId(u64::from(unit)))
            .collect::<BTreeSet<_>>();
        let outcome = self
            .service_mut()?
            .coded_admit(token, CodedClaimInput::complete(claim.iter().copied()))?;
        if outcome != CodedAdmissionOutcome::Admitted {
            bail!("model admission mapped to production contention");
        }
        self.tokens.insert(operation.to_owned(), token);
        self.claims.insert(operation.to_owned(), claim);
        Ok(())
    }

    fn sampled_step(&mut self, choice: i64) -> Result<()> {
        match choice {
            0 => self.admit("opA", &[0]),
            1 => self.admit("opB", &[0, 1]),
            2 => self.admit("opC", &[1]),
            3 => self.start_capture(),
            4 => self.accept_capture(),
            5 => self.reject_capture(),
            6 => self.permit_effect("opA"),
            7 => self.permit_effect("opB"),
            8 => self.permit_effect("opC"),
            9 => self.release_operation("opA"),
            10 => self.release_operation("opB"),
            11 => self.release_operation("opC"),
            _ => bail!("sampled Connect choice {choice} is outside the bounded action set"),
        }
    }

    fn start_capture(&mut self) -> Result<()> {
        self.service_mut()?.coded_start_capture(
            CodedCaptureId(0),
            CodedCaptureScopeInput::complete([CodedUnitId(0), CodedUnitId(1)]),
        )?;
        Ok(())
    }

    fn accept_capture(&mut self) -> Result<()> {
        self.service_mut()?
            .coded_captures_mut()
            .observe_decision(CodedCaptureId(0), CodedCaptureDecision::Accepted)?;
        Ok(())
    }

    fn reject_capture(&mut self) -> Result<()> {
        self.service_mut()?
            .coded_captures_mut()
            .observe_decision(CodedCaptureId(0), CodedCaptureDecision::Rejected)?;
        Ok(())
    }

    fn request_clean(&mut self) -> Result<()> {
        self.service_mut()?
            .coded_captures_mut()
            .request_clean(CodedCaptureId(0))?;
        Ok(())
    }

    fn clean_commit(&mut self, observation: CodedCleanCommitObservation) -> Result<()> {
        self.service_mut()?
            .coded_captures_mut()
            .observe_clean_commit(CodedCaptureId(0), observation)?;
        Ok(())
    }

    fn reconcile_clean(&mut self, reconciliation: CodedCleanReconciliation) -> Result<()> {
        self.service_mut()?
            .coded_captures_mut()
            .reconcile_clean_commit(CodedCaptureId(0), reconciliation)?;
        Ok(())
    }

    fn later_cut(&mut self, observation: CodedLaterCutObservation) -> Result<()> {
        let operation = self.token("opC")?;
        self.service_mut()?.coded_captures_mut().observe_later_cut(
            CodedCaptureId(0),
            operation,
            observation,
        )?;
        Ok(())
    }

    fn later_cut_reconcile(&mut self, observation: CodedLaterCutReconciliation) -> Result<()> {
        let operation = self.token("opC")?;
        self.service_mut()?
            .coded_captures_mut()
            .reconcile_later_cut(CodedCaptureId(0), operation, observation)?;
        Ok(())
    }

    fn permit_effect(&mut self, operation: &str) -> Result<()> {
        let token = self.token(operation)?;
        if self.service_mut()?.coded_permit_effect(token)? != CodedEffectOutcome::Permitted {
            bail!("model effect permission remained blocked in production");
        }
        Ok(())
    }

    fn release_operation(&mut self, operation: &str) -> Result<()> {
        let token = self.token(operation)?;
        // LifecycleRelease owns and separately Connect-proves this certificate.
        // This bounded bridge consumes it as an opaque typed input; it does not
        // reconstruct media, child, transaction, recovery, basis, or cleanup
        // predicates.
        let authorization = ReleaseAuthorization { operation: token };
        let release = self
            .service_mut()?
            .coded_release_claim(token, &authorization)?;
        if release.operation != token {
            bail!("coded release returned the wrong generation");
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
            .insert(operation.to_owned(), release.operation);
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
        let outcome = self
            .service_mut()?
            .coded_admit(hidden, CodedClaimInput::complete([CodedUnitId(0)]))?;
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
        let hidden = self.service_mut()?.reserve(request(
            RequestId(904),
            TopologyEpoch(4),
            1,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).expect("bounded hidden range"),
            DurabilityIntent::Ordinary,
        ))?;
        let result = self
            .service_mut()?
            .coded_admit(hidden, CodedClaimInput::new([CodedUnitId(0)], false, true));
        if !result
            .as_ref()
            .err()
            .is_some_and(|error| error.to_string().contains("coded claim is incomplete"))
        {
            bail!("incomplete claim did not fail at the service seam");
        }
        self.service_mut()?.admission.reclaim(hidden, false)?;
        Ok(())
    }

    fn probe_unvalidated_claim(&mut self) -> Result<()> {
        let hidden = self.service_mut()?.reserve(request(
            RequestId(905),
            TopologyEpoch(4),
            1,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).expect("bounded hidden range"),
            DurabilityIntent::Ordinary,
        ))?;
        let result = self
            .service_mut()?
            .coded_admit(hidden, CodedClaimInput::new([CodedUnitId(0)], true, false));
        if !result
            .as_ref()
            .err()
            .is_some_and(|error| error.to_string().contains("coded claim is not validated"))
        {
            bail!("unvalidated claim did not fail at the service seam");
        }
        self.service_mut()?.admission.reclaim(hidden, false)?;
        Ok(())
    }

    fn probe_unauthorized_removal(&mut self) -> Result<()> {
        let operation = self.token("opA")?;
        let result = self
            .service_mut()?
            .coded_authority_mut()
            .release(operation, None);
        if !matches!(
            result,
            Err(CodedAuthorityError::ReleaseAuthorizationMissing(_))
        ) {
            bail!("coded removal succeeded without ReleaseAllowed");
        }
        Ok(())
    }

    fn probe_stale_generation(&mut self) -> Result<()> {
        let old = self.service_mut()?.reserve(request(
            RequestId(901),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).expect("bounded hidden range"),
            DurabilityIntent::Ordinary,
        ))?;
        self.service_mut()?.admission.reclaim(old, false)?;
        let current = self.service_mut()?.reserve(request(
            RequestId(902),
            TopologyEpoch(4),
            0,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).expect("bounded hidden range"),
            DurabilityIntent::Ordinary,
        ))?;
        if old.index != current.index || old.generation == current.generation {
            bail!("hidden stale-generation probe did not reuse one slot with a new generation");
        }
        if self
            .service_mut()?
            .coded_admit(current, CodedClaimInput::complete([CodedUnitId(1)]))?
            != CodedAdmissionOutcome::Admitted
        {
            bail!("stale-generation probe failed to admit its current generation");
        }
        let result = self
            .service_mut()?
            .coded_authority_mut()
            .release(current, Some(old));
        if !matches!(
            result,
            Err(CodedAuthorityError::ReleaseAuthorizationMismatch { .. })
        ) {
            bail!("stale authorization removed a current coded claim");
        }
        self.service_mut()?
            .coded_authority_mut()
            .release(current, Some(current))?;
        self.service_mut()?.admission.reclaim(current, false)?;
        Ok(())
    }

    fn probe_non_transitive(&mut self) -> Result<()> {
        let hidden = self.service_mut()?.reserve(request(
            RequestId(903),
            TopologyEpoch(4),
            1,
            BlockOp::Write,
            ByteRange::new(0, BLOCK as u64).expect("bounded hidden range"),
            DurabilityIntent::Ordinary,
        ))?;
        let outcome = self.service_mut()?.coded_admit(
            hidden,
            CodedClaimInput::complete([CodedUnitId(0), CodedUnitId(1)]),
        )?;
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
        if self
            .service_mut()?
            .coded_admit(hidden, CodedClaimInput::complete([CodedUnitId(0)]))?
            != CodedAdmissionOutcome::Admitted
        {
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
        self.service_mut()?
            .coded_authority_mut()
            .release(hidden, Some(hidden))?;
        self.service_mut()?.admission.reclaim(hidden, false)?;
        Ok(())
    }

    fn probe_clean_before_later_reconciliation(&mut self) -> Result<()> {
        let result = self
            .service_mut()?
            .coded_captures_mut()
            .request_clean(CodedCaptureId(0));
        if result.is_ok() {
            bail!("CLEAN became eligible while later uncertainty was unresolved");
        }
        Ok(())
    }

    fn probe_clean_before_decision(&mut self) -> Result<()> {
        let result = self
            .service_mut()?
            .coded_captures_mut()
            .request_clean(CodedCaptureId(0));
        if result.is_ok() {
            bail!("CLEAN became eligible before a decision was observed");
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
        let (phase, scope, satisfaction) = if let Some(snapshot) = capture {
            let reverse = self
                .tokens
                .iter()
                .map(|(id, token)| (*token, id.as_str()))
                .collect::<HashMap<_, _>>();
            for (operation, status) in snapshot.membership {
                let id = reverse
                    .get(&operation)
                    .with_context(|| format!("model has no operation for {operation:?}"))?;
                membership.insert((*id).to_owned(), model_membership(status));
            }
            let scope = snapshot.scope.into_iter().map(unit_name).collect();
            let satisfaction = snapshot
                .decision
                .map(model_decision)
                .unwrap_or(ModelSatisfaction::CaptureSatisfactionNotObserved);
            (model_capture_phase(snapshot.phase), scope, satisfaction)
        } else {
            (
                ModelCapturePhase::CaptureAbsent,
                HashSet::new(),
                ModelSatisfaction::CaptureSatisfactionNotObserved,
            )
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

fn model_capture_phase(phase: CodedCapturePhase) -> ModelCapturePhase {
    match phase {
        CodedCapturePhase::Open => ModelCapturePhase::CaptureOpen,
        CodedCapturePhase::CleanCommitPending => ModelCapturePhase::CaptureCommitPending,
        CodedCapturePhase::CleanCommitUnknown => ModelCapturePhase::CaptureCommitUnknown,
        CodedCapturePhase::CleanKnown => ModelCapturePhase::CaptureCleanKnown,
        CodedCapturePhase::Refused => ModelCapturePhase::CaptureRefused,
    }
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

fn model_decision(decision: CodedCaptureDecision) -> ModelSatisfaction {
    match decision {
        CodedCaptureDecision::Accepted => ModelSatisfaction::CaptureSatisfactionAccepted,
        CodedCaptureDecision::Rejected => ModelSatisfaction::CaptureSatisfactionRejected,
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
    assert_eq!(
        service
            .coded_admit(old, CodedClaimInput::complete([CodedUnitId(0)]))
            .unwrap(),
        CodedAdmissionOutcome::Admitted
    );
    // External typed LifecycleRelease input; provider correctness is separate evidence.
    let old_authorization = ReleaseAuthorization { operation: old };

    service.admission.reclaim(old, false).unwrap();
    assert!(
        service
            .coded_admit(old, CodedClaimInput::complete([CodedUnitId(1)]))
            .is_err()
    );
    assert_eq!(
        service
            .coded_release_claim(old, &old_authorization)
            .unwrap()
            .operation,
        old
    );

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
    assert_eq!(
        service
            .coded_admit(current, CodedClaimInput::complete([CodedUnitId(0)]))
            .unwrap(),
        CodedAdmissionOutcome::Admitted
    );
    let error = service
        .coded_release_claim(current, &old_authorization)
        .unwrap_err();
    assert!(error.to_string().contains("does not match operation"));
    assert!(service.coded_authority().active_claim(current).is_some());
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
    assert_eq!(
        service
            .coded_admit(operation, CodedClaimInput::complete([CodedUnitId(0)]))
            .unwrap(),
        CodedAdmissionOutcome::Admitted
    );

    // A real lifecycle certificate is established only after the operation slot
    // is Reclaimable. Keep the slot live to exercise the pre-cleanup window.
    service
        .admission
        .record_reconciliation(operation, ReconciliationOutcome::Durable)
        .unwrap();
    let authorization = ReleaseAuthorization { operation };
    service
        .coded_release_claim(operation, &authorization)
        .unwrap();

    let error = service
        .coded_admit(operation, CodedClaimInput::complete([CodedUnitId(1)]))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("coded admission requires a reserved operation slot")
    );
    assert!(service.coded_authority().active_claim(operation).is_none());
}

#[test]
fn coded_capture_and_admission_linearize_at_service_boundary() {
    let scope = CodedCaptureScopeInput::complete([CodedUnitId(0), CodedUnitId(1)]);
    let range = ByteRange::new(0, BLOCK as u64).expect("bounded coded range");

    let mut capture_first = fake_service(FakeRead::Exact, ServiceConfig::default());
    capture_first
        .coded_start_capture(CodedCaptureId(0), scope.clone())
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
    assert_eq!(
        capture_first
            .coded_admit(later, CodedClaimInput::complete([CodedUnitId(1)]))
            .unwrap(),
        CodedAdmissionOutcome::Admitted
    );
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
    assert_eq!(
        admit_first
            .coded_admit(included, CodedClaimInput::complete([CodedUnitId(0)]))
            .unwrap(),
        CodedAdmissionOutcome::Admitted
    );
    admit_first
        .coded_start_capture(CodedCaptureId(0), scope)
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
    max_steps = 9,
    seed = "22082026"
)]
fn coded_range_connect_capture() -> impl Driver {
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
    max_steps = 3,
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
    max_steps = 6,
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
    max_steps = 7,
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
    max_steps = 6,
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
    max_steps = 7,
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
    max_steps = 6,
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
    max_steps = 5,
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
            }
        }
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
            Self {
                service: fake_service(FakeRead::Exact, ServiceConfig::default()),
                tokens: BTreeMap::new(),
                next_request_id: 0,
                drop_active_claim,
            }
        }

        fn reserve(&mut self, data_slot: u8) -> OperationSlotToken {
            let request_id = RequestId(20_000 + self.next_request_id);
            self.next_request_id += 1;
            self.service
                .reserve(request(
                    request_id,
                    TopologyEpoch(4),
                    usize::from(data_slot),
                    BlockOp::Write,
                    ByteRange::new(0, BLOCK as u64).expect("bounded generated range"),
                    DurabilityIntent::Ordinary,
                ))
                .expect("generated operation admission")
        }

        fn admit(&mut self, operation: u8, data_slot: u8, claim: ClaimKind) {
            let token = self.reserve(data_slot);
            assert_eq!(
                self.service
                    .coded_admit(token, CodedClaimInput::complete(claim.units()))
                    .expect("generated coded admission"),
                CodedAdmissionOutcome::Admitted
            );
            assert_eq!(
                self.service.coded_authority().operation_phase(token),
                Some(CodedOperationPhase::Held)
            );
            self.tokens.insert(operation, token);
        }

        fn contend(&mut self, _operation: u8, data_slot: u8, claim: ClaimKind) {
            let token = self.reserve(data_slot);
            assert_eq!(
                self.service
                    .coded_admit(token, CodedClaimInput::complete(claim.units()))
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
            let authorization = ReleaseAuthorization { operation: token };
            assert_eq!(
                self.service
                    .coded_release_claim(token, &authorization)
                    .expect("generated coded release")
                    .operation,
                token
            );
            assert!(self.service.coded_authority().active_claim(token).is_none());
            self.service
                .admission
                .reclaim(token, false)
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
                .coded_release_claim(token, &ReleaseAuthorization { operation: stale })
                .expect_err("stale release unexpectedly succeeded");
            assert!(error.to_string().contains("does not match operation"));
            assert!(self.service.coded_authority().active_claim(token).is_some());
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
            }
            state.assert_invariants(ref_state);
            state
        }

        fn check_invariants(state: &Self::SystemUnderTest, ref_state: &ReferenceState) {
            state.assert_invariants(ref_state);
        }
    }

    proptest_state_machine::prop_state_machine! {
        #![proptest_config(ProptestConfig::with_cases(64))]
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
                Ok(CodedOperationMachine::<true>::test_sequential(
                    proptest::test_runner::Config::default(),
                    initial,
                    transitions,
                    seen_counter,
                ))
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
