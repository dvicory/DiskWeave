use super::*;
use crate::evidence::{
    BasisConformance, OperationEffect, RecoveryReconciliation, ReleaseReconciliation,
    ReleaseRequirement, ReleaseScope,
};
use anyhow::{Context, bail};
use dwv_recovery::{FenceCertificate, RecoveryGeneration, RecoveryStoreHealth};
use dwv_store::{
    CapabilityEvidenceId, ChildOperationId, CompletionDisposition, FenceId, OperationSlotToken,
    PersistenceEvidence, ReconciliationOutcome, SlotState, StoreFenceRef,
};
use dwv_transaction_ref::{
    ActionKind, ActionResult, CommittedRecoveryGeneration, ComputationResult, ParityRange,
    PlannedRead, PlannedWrite, RangeGuardToken, SemanticIoResult, StoreWatermark,
    TransactionMachine, TransactionPersistenceEvidence, TransactionPlan,
};
use quint_connect::{Config, Driver, Result, State, Step, quint_run};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
struct OperationGeneration {
    operation: String,
    generation: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum MediaEffect {
    MediaUnresolved,
    MediaTerminal,
    MediaReconciled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum ChildOutcome {
    ChildrenUnresolved,
    ChildrenTerminal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum TransactionRequirement {
    TransactionUnresolved,
    TransactionSatisfied,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum RecoveryObservation {
    RecoveryUnresolved,
    RecoveryAuthoritative,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum BasisObservation {
    BasisUnresolved,
    BasisConsumed,
    BasisDiscarded,
    BasisReconciled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum CleanupObservation {
    CleanupNotRequested,
    CleanupFailed,
    CleanupSucceeded,
}

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
struct OwnerFacts {
    mediaEffect: MediaEffect,
    children: ChildOutcome,
    requiredReconciliation: bool,
    reclaimable: bool,
    transaction: TransactionRequirement,
    recovery: RecoveryObservation,
    basis: BasisObservation,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Eq, PartialEq)]
struct BridgeState {
    active: HashSet<OperationGeneration>,
    facts: HashMap<OperationGeneration, OwnerFacts>,
    authorizations: HashSet<OperationGeneration>,
    cleanup: HashMap<OperationGeneration, CleanupObservation>,
    cleanupRequests: HashSet<OperationGeneration>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CleanupResult {
    Failed,
    Succeeded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActualBasis {
    Consumed,
    Discarded,
    Reconciled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActualMedia {
    Terminal,
    Reconciled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LedgerRecord {
    token: Option<OperationSlotToken>,
    child: Option<ChildOperationId>,
    facts: OwnerFacts,
    active: bool,
    authorization: bool,
    cleanup: CleanupObservation,
    cleanup_requested: bool,
    pending_cleanup: Option<CleanupResult>,
}

impl LedgerRecord {
    const fn new() -> Self {
        Self {
            token: None,
            child: None,
            facts: OwnerFacts {
                mediaEffect: MediaEffect::MediaUnresolved,
                children: ChildOutcome::ChildrenUnresolved,
                requiredReconciliation: false,
                reclaimable: false,
                transaction: TransactionRequirement::TransactionUnresolved,
                recovery: RecoveryObservation::RecoveryUnresolved,
                basis: BasisObservation::BasisUnresolved,
            },
            active: false,
            authorization: false,
            cleanup: CleanupObservation::CleanupNotRequested,
            cleanup_requested: false,
            pending_cleanup: None,
        }
    }
}

#[derive(Clone, Copy)]
enum Path {
    Success,
    Failure,
    Negative,
}

struct BridgeDriver {
    path: Path,
    service: Option<HealthyPortableService<FakeStore, dwv_recovery::MemoryRecoveryStore>>,
    records: HashMap<OperationGeneration, LedgerRecord>,
    transactions: HashMap<OperationGeneration, TransactionMachine>,
}

impl BridgeDriver {
    fn for_path(path: Path) -> Self {
        Self {
            path,
            service: None,
            records: HashMap::new(),
            transactions: HashMap::new(),
        }
    }
}

impl Default for BridgeDriver {
    fn default() -> Self {
        Self::for_path(Path::Success)
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
            state: &["LifecycleReleaseConnect::LifecycleRelease::state"],
            ..Config::default()
        }
    }

    fn step(&mut self, step: &Step) -> Result {
        let id = self.path_id();
        quint_connect::switch!(step {
            connectInit => self.init(),
            enterApplicable => self.enter_applicable(id)?,
            observeMediaTerminal => self.observe_media(id, ActualMedia::Terminal)?,
            observeMediaReconciled => self.observe_media(id, ActualMedia::Reconciled)?,
            observeChildrenTerminal => self.observe_children_terminal(id)?,
            recordRequiredReconciliation => self.record_required_reconciliation(id)?,
            observeReclaimable => self.observe_reclaimable(id)?,
            observeTransactionSatisfied => self.observe_transaction_satisfied(id)?,
            observeRecoveryAuthoritative => self.observe_recovery_authoritative(id)?,
            observeBasisConformance => self.observe_basis(id, self.path_basis())?,
            establishReleaseAllowed => self.establish_release_allowed(id)?,
            observeReleaseAllowed => self.observe_release_allowed(id)?,
            requestCleanup => self.request_cleanup(id)?,
            observeCleanupFailed => self.observe_cleanup_failed(id)?,
            observeCleanupSucceeded => self.observe_cleanup_succeeded(id)?,
            probeNoAuthorizationBeforeApplicability => self.probe_no_authorization_before_applicability()?,
            probeOutOfScopeOperation => self.probe_out_of_scope_operation()?,
            probeStaleGenerationRejection => self.probe_stale_generation_rejection()?,
            attemptIncompleteAuthorization => self.attempt_incomplete_authorization(id)?,
            attemptUnresolvedRecovery => self.attempt_unresolved_recovery(id)?,
            attemptUnresolvedBasis => self.attempt_unresolved_basis(id)?,
        })
    }
}

impl BridgeDriver {
    fn path_id(&self) -> OperationGeneration {
        let operation = match self.path {
            Path::Failure => "slot-b",
            Path::Success | Path::Negative => "slot-a",
        };
        OperationGeneration {
            operation: operation.to_owned(),
            generation: 0,
        }
    }

    fn path_basis(&self) -> BasisObservation {
        match self.path {
            Path::Failure => BasisObservation::BasisReconciled,
            Path::Success | Path::Negative => BasisObservation::BasisConsumed,
        }
    }

    fn init(&mut self) {
        self.service = Some(fake_service(FakeRead::Exact, ServiceConfig::default()));
        self.records = model_ids()
            .into_iter()
            .map(|id| (id, LedgerRecord::new()))
            .collect();
        self.transactions.clear();
    }

    fn service(
        &self,
    ) -> Result<&HealthyPortableService<FakeStore, dwv_recovery::MemoryRecoveryStore>> {
        self.service
            .as_ref()
            .context("LifecycleRelease Connect driver has no service")
    }

    fn service_mut(
        &mut self,
    ) -> Result<&mut HealthyPortableService<FakeStore, dwv_recovery::MemoryRecoveryStore>> {
        self.service
            .as_mut()
            .context("LifecycleRelease Connect driver has no service")
    }

    fn record(&self, id: &OperationGeneration) -> Result<LedgerRecord> {
        self.records
            .get(id)
            .copied()
            .with_context(|| format!("unknown LifecycleRelease generation {id:?}"))
    }

    fn record_mut(&mut self, id: &OperationGeneration) -> Result<&mut LedgerRecord> {
        self.records
            .get_mut(id)
            .with_context(|| format!("unknown LifecycleRelease generation {id:?}"))
    }

    fn token(&self, id: &OperationGeneration) -> Result<OperationSlotToken> {
        self.record(id)?
            .token
            .context("generation has not been admitted")
    }

    fn bridge_state(&self) -> Result<BridgeState> {
        let mut active = HashSet::new();
        let mut facts = HashMap::new();
        let mut authorizations = HashSet::new();
        let mut cleanup = HashMap::new();
        let mut cleanup_requests = HashSet::new();
        for id in model_ids() {
            let record = self.record(&id)?;
            if record.active {
                active.insert(id.clone());
            }
            if record.authorization {
                authorizations.insert(id.clone());
            }
            if record.cleanup_requested {
                cleanup_requests.insert(id.clone());
            }
            facts.insert(id.clone(), record.facts);
            cleanup.insert(id, record.cleanup);
        }
        Ok(BridgeState {
            active,
            facts,
            authorizations,
            cleanup,
            cleanupRequests: cleanup_requests,
        })
    }

    fn enter_applicable(&mut self, id: OperationGeneration) -> Result {
        let request = request_for(&id);
        let token = self.reserve_in_scope(&id, request)?;
        let child = if matches!(self.path, Path::Success) {
            let service = self.service_mut()?;
            let child = service.admission.child(token, connect_range())?;
            service.admission.submitted(token)?;
            Some(child)
        } else {
            None
        };
        let record = self.record_mut(&id)?;
        if record.token.is_some() {
            bail!("model entered an already-correlated generation {id:?}");
        }
        record.token = Some(token);
        record.child = child;
        record.active = true;
        Ok(())
    }

    fn reserve_in_scope(
        &mut self,
        id: &OperationGeneration,
        request: BlockRequest,
    ) -> Result<OperationSlotToken> {
        let token = self.service_mut()?.reserve(request)?;
        let mut machine = TransactionMachine::new(connect_plan())?;
        machine.apply(ActionResult::RangeAcquired(RangeGuardToken(1)))?;
        if machine.range_guard().is_none() {
            bail!("transaction owner did not retain the RangeAcquired result");
        }
        self.service_mut()?
            .set_release_scope(token, ReleaseScope::InScope);
        if self.service()?.release_scope(token) != ReleaseScope::InScope {
            bail!("service did not expose the exact in-scope release observation");
        }
        self.transactions.insert(id.clone(), machine);
        Ok(token)
    }

    fn observe_media(&mut self, id: OperationGeneration, media: ActualMedia) -> Result {
        let token = self.token(&id)?;
        match media {
            ActualMedia::Terminal => {
                let observation = self.service()?.operation_effect_observation(token)?;
                if observation.operation != token || observation.effect != OperationEffect::Terminal
                {
                    bail!("normal media-effect owner did not report exact-generation terminality");
                }
            }
            ActualMedia::Reconciled => {
                // AuthoritativelyReconciled is an external owner fact. This
                // bridge checks exact-generation consumption later; it does
                // not manufacture or claim the upstream reconciliation proof.
            }
        }
        let record = self.record_mut(&id)?;
        record.facts.mediaEffect = match media {
            ActualMedia::Terminal => MediaEffect::MediaTerminal,
            ActualMedia::Reconciled => MediaEffect::MediaReconciled,
        };
        Ok(())
    }

    fn observe_children_terminal(&mut self, id: OperationGeneration) -> Result {
        let (token, child) = {
            let record = self.record(&id)?;
            (
                record
                    .token
                    .context("child observation lacks exact token")?,
                record.child,
            )
        };
        if let Some(child) = child {
            self.service_mut()?.admission.complete(
                token,
                FakeStore::completion(
                    child,
                    connect_range(),
                    Some(connect_range()),
                    CompletionDisposition::Success,
                    PersistenceEvidence::VolatileOrUnknown,
                ),
            )?;
        }
        let snapshot = self.service()?.admission.snapshot(token)?;
        if snapshot.token != token || !snapshot.children.iter().all(|child| child.terminal) {
            bail!("store-owned child snapshot did not prove terminal children");
        }
        if matches!(self.path, Path::Success) && snapshot.children.is_empty() {
            bail!("successful Connect path did not exercise a registered/submitted child");
        }
        self.record_mut(&id)?.facts.children = ChildOutcome::ChildrenTerminal;
        Ok(())
    }

    fn record_required_reconciliation(&mut self, id: OperationGeneration) -> Result {
        let token = self.token(&id)?;
        self.service_mut()?
            .admission
            .record_reconciliation(token, ReconciliationOutcome::Durable)?;
        let snapshot = self.service()?.admission.snapshot(token)?;
        if snapshot.token != token
            || snapshot.reconciliation != Some(ReconciliationOutcome::Durable)
        {
            bail!("required reconciliation was not observed for the exact token");
        }
        self.record_mut(&id)?.facts.requiredReconciliation = true;
        Ok(())
    }

    fn observe_reclaimable(&mut self, id: OperationGeneration) -> Result {
        let token = self.token(&id)?;
        let snapshot = self.service()?.admission.snapshot(token)?;
        if snapshot.token != token || snapshot.state != SlotState::Reclaimable {
            bail!("reclaimable observation did not come from the exact slot snapshot");
        }
        self.record_mut(&id)?.facts.reclaimable = true;
        Ok(())
    }

    fn observe_transaction_satisfied(&mut self, id: OperationGeneration) -> Result {
        let token = self.token(&id)?;
        let requirement = if matches!(self.path, Path::Failure) {
            // Authoritative reconciliation supplies this typed owner fact for
            // the reconciled-failure profile; its upstream proof is outside
            // this Connect projection.
            ReleaseRequirement::TransactionSatisfied
        } else {
            let machine = self
                .transactions
                .get_mut(&id)
                .context("transaction observation lacks its exact-generation machine")?;
            drive_transaction_to_release(machine)?;
            transaction_requirement(Some(machine.trace()))
        };
        if !requirement.is_satisfied() {
            bail!("transaction owner did not supply a satisfied typed requirement");
        }
        let observation = self.service()?.current_release_observation(
            token,
            actual_media(self.record(&id)?.facts.mediaEffect),
            requirement,
        );
        if observation.operation != token || observation.requirement != requirement {
            bail!("transaction requirement was not bound to the exact generation");
        }
        self.record_mut(&id)?.facts.transaction = TransactionRequirement::TransactionSatisfied;
        Ok(())
    }

    fn observe_recovery_authoritative(&mut self, id: OperationGeneration) -> Result {
        let token = self.token(&id)?;
        if self.service()?.recovery_reconciliation_observation()
            != RecoveryReconciliation::Authoritative
        {
            bail!("recovery owner did not supply an authoritative observation");
        }
        let observation = self.service()?.current_release_observation(
            token,
            actual_media(self.record(&id)?.facts.mediaEffect),
            actual_requirement(self.record(&id)?.facts.transaction),
        );
        if observation.operation != token
            || observation.recovery != RecoveryReconciliation::Authoritative
        {
            bail!("recovery observation was not bound to the exact token");
        }
        self.record_mut(&id)?.facts.recovery = RecoveryObservation::RecoveryAuthoritative;
        Ok(())
    }

    fn observe_basis(&mut self, id: OperationGeneration, observation: BasisObservation) -> Result {
        // Basis conformance is an external typed owner input here. Focused
        // service evidence separately proves the normal write path publishes
        // exact-generation BasisConsumed before cleanup.
        let actual = match observation {
            BasisObservation::BasisConsumed => ActualBasis::Consumed,
            BasisObservation::BasisDiscarded => ActualBasis::Discarded,
            BasisObservation::BasisReconciled => ActualBasis::Reconciled,
            BasisObservation::BasisUnresolved => {
                bail!("the model cannot request unresolved basis observation")
            }
        };
        let token = self.token(&id)?;
        let typed = basis_conformance(actual);
        self.service_mut()?.set_basis_conformance(token, typed);
        if self.service()?.basis_conformance_observation(token) != typed {
            bail!("basis conformance was not observed for the exact token");
        }
        self.record_mut(&id)?.facts.basis = observation;
        Ok(())
    }

    fn establish_release_allowed(&mut self, id: OperationGeneration) -> Result {
        let record = self.record(&id)?;
        if !owner_facts_complete(record.facts) {
            bail!("Connect requested authorization before all model facts were observed");
        }
        let token = record.token.context("authorization lacks exact token")?;
        let observation = self.service()?.current_release_observation(
            token,
            actual_media(record.facts.mediaEffect),
            actual_requirement(record.facts.transaction),
        );
        if observation.operation != token
            || observation.recovery != RecoveryReconciliation::Authoritative
            || observation.basis != actual_basis(record.facts.basis)
        {
            bail!(
                "final lifecycle observation did not match the witnessed exact-generation ledger"
            );
        }
        if id.operation == "slot-b" {
            self.service_mut()?
                .inject_terminalization_failure(TerminalizationFault::Reclaim);
        }
        let result = self
            .service_mut()?
            .reconcile_release_authorization(observation);
        let authorization = match result {
            Ok(Some(authorization)) => {
                if id.operation == "slot-b" {
                    bail!("failure profile unexpectedly completed cleanup");
                }
                authorization
            }
            Ok(None) => bail!("in-scope lifecycle reconciliation returned no authorization"),
            Err(_) => {
                if id.operation != "slot-b" {
                    bail!("successful lifecycle profile unexpectedly failed cleanup");
                }
                self.service()?
                    .release_authorization(token)
                    .cloned()
                    .context("cleanup failure did not retain the exact certificate")?
            }
        };
        if authorization.operation != token
            || self.service()?.release_authorization(token) != Some(&authorization)
        {
            bail!("release authorization was not returned and retained for the exact token");
        }
        let cleanup_result = if id.operation == "slot-b" {
            CleanupResult::Failed
        } else {
            CleanupResult::Succeeded
        };
        let record = self.record_mut(&id)?;
        record.authorization = true;
        record.pending_cleanup = Some(cleanup_result);
        Ok(())
    }

    fn observe_release_allowed(&self, id: OperationGeneration) -> Result {
        let record = self.record(&id)?;
        let token = record
            .token
            .context("release observation lacks exact token")?;
        if !record.authorization || self.service()?.release_authorization(token).is_none() {
            bail!("release authorization was not observed for the exact generation");
        }
        Ok(())
    }

    fn request_cleanup(&mut self, id: OperationGeneration) -> Result {
        let record = self.record(&id)?;
        if !record.authorization || record.cleanup == CleanupObservation::CleanupSucceeded {
            bail!("cleanup was requested without an exact authorization");
        }
        let token = record.token.context("cleanup request lacks exact token")?;
        let pending = if let Some(result) = record.pending_cleanup {
            result
        } else {
            let result = self
                .service_mut()?
                .release(token)
                .map(|()| CleanupResult::Succeeded);
            match result {
                Ok(result) => result,
                Err(_) => CleanupResult::Failed,
            }
        };
        let record = self.record_mut(&id)?;
        record.cleanup_requested = true;
        record.pending_cleanup = Some(pending);
        Ok(())
    }

    fn observe_cleanup_failed(&mut self, id: OperationGeneration) -> Result {
        let record = self.record(&id)?;
        if record.pending_cleanup != Some(CleanupResult::Failed) {
            bail!("cleanup failure was not returned by the real cleanup path");
        }
        let token = record.token.context("cleanup failure lacks exact token")?;
        if self.service()?.release_authorization(token).is_none()
            || self.service()?.admission.snapshot(token).is_err()
        {
            bail!("cleanup failure did not retain the exact certificate and live slot");
        }
        let record = self.record_mut(&id)?;
        record.cleanup = CleanupObservation::CleanupFailed;
        record.cleanup_requested = false;
        record.pending_cleanup = None;
        Ok(())
    }

    fn observe_cleanup_succeeded(&mut self, id: OperationGeneration) -> Result {
        let record = self.record(&id)?;
        if record.pending_cleanup != Some(CleanupResult::Succeeded) {
            bail!("cleanup success was not returned by the real cleanup path");
        }
        let record = self.record_mut(&id)?;
        record.cleanup = CleanupObservation::CleanupSucceeded;
        record.cleanup_requested = false;
        record.active = false;
        record.pending_cleanup = None;
        Ok(())
    }

    fn probe_no_authorization_before_applicability(&mut self) -> Result {
        let token = self.service_mut()?.reserve(request(
            RequestId(900),
            connect_plan().topology_epoch,
            0,
            BlockOp::Write,
            connect_range(),
            DurabilityIntent::Ordinary,
        ))?;
        if self.service()?.release_scope(token) != ReleaseScope::Outside {
            bail!("pre-acquisition write did not remain outside release scope");
        }
        let authorization =
            self.service_mut()?
                .reclaim_with_authorization(&ReleaseReconciliation {
                    operation: token,
                    operation_effect: OperationEffect::Terminal,
                    requirement: ReleaseRequirement::NotApplicable,
                    recovery: RecoveryReconciliation::Authoritative,
                    basis: BasisConformance::Reconciled,
                })?;
        if authorization.is_some() || self.service()?.release_authorization(token).is_some() {
            bail!("pre-acquisition write produced a release authorization");
        }
        Ok(())
    }

    fn probe_out_of_scope_operation(&mut self) -> Result {
        let (_, evidence) = self.service_mut()?.read(request(
            RequestId(901),
            connect_plan().topology_epoch,
            0,
            BlockOp::Read,
            connect_range(),
            DurabilityIntent::Ordinary,
        ))?;
        if evidence.release_authorization.is_some() {
            bail!("routine read produced a release authorization");
        }
        Ok(())
    }

    fn probe_stale_generation_rejection(&mut self) -> Result {
        let stale = self.service_mut()?.reserve(request(
            RequestId(902),
            connect_plan().topology_epoch,
            0,
            BlockOp::Write,
            connect_range(),
            DurabilityIntent::Ordinary,
        ))?;
        let stale_observation = ReleaseReconciliation {
            operation: stale,
            operation_effect: OperationEffect::Terminal,
            requirement: ReleaseRequirement::NotApplicable,
            recovery: RecoveryReconciliation::Authoritative,
            basis: BasisConformance::Reconciled,
        };
        self.service_mut()?
            .reclaim_with_authorization(&stale_observation)?;
        let current = self.service_mut()?.reserve(request(
            RequestId(903),
            connect_plan().topology_epoch,
            0,
            BlockOp::Write,
            connect_range(),
            DurabilityIntent::Ordinary,
        ))?;
        if current.index != stale.index || current.generation == stale.generation {
            bail!("stale-generation probe did not reuse the same slot with a new generation");
        }
        let mut late = stale_observation;
        late.operation_effect = OperationEffect::AuthoritativelyReconciled;
        if self
            .service_mut()?
            .reconcile_release_authorization(late)
            .is_ok()
        {
            bail!("late stale observation unexpectedly authorized current slot state");
        }
        if self.service()?.release_authorization(stale).is_some()
            || self.service()?.admission.snapshot(current).is_err()
        {
            bail!("stale observation changed exact current-generation ownership");
        }
        self.service_mut()?
            .reclaim_with_authorization(&ReleaseReconciliation {
                operation: current,
                operation_effect: OperationEffect::Terminal,
                requirement: ReleaseRequirement::NotApplicable,
                recovery: RecoveryReconciliation::Authoritative,
                basis: BasisConformance::Reconciled,
            })?;
        Ok(())
    }

    fn attempt_incomplete_authorization(&mut self, id: OperationGeneration) -> Result {
        let token = self.token(&id)?;
        let result = self
            .service_mut()?
            .reconcile_release_authorization(ReleaseReconciliation {
                operation: token,
                operation_effect: OperationEffect::Unresolved,
                requirement: ReleaseRequirement::TransactionUnresolved,
                recovery: RecoveryReconciliation::Unresolved,
                basis: BasisConformance::Unresolved,
            });
        if result.is_ok_and(|authorization| authorization.is_some())
            || self.service()?.release_authorization(token).is_some()
        {
            bail!("incomplete owner facts produced an authorization");
        }
        Ok(())
    }

    fn attempt_unresolved_recovery(&mut self, id: OperationGeneration) -> Result {
        let token = self.token(&id)?;
        let facts = self.record(&id)?.facts;
        if !matches!(
            facts.mediaEffect,
            MediaEffect::MediaTerminal | MediaEffect::MediaReconciled
        ) || facts.children != ChildOutcome::ChildrenTerminal
            || !facts.requiredReconciliation
            || !facts.reclaimable
            || facts.transaction != TransactionRequirement::TransactionSatisfied
        {
            bail!(
                "recovery-negative probe did not first satisfy every other observed prerequisite"
            );
        }
        self.service_mut()?
            .recovery_mut()
            .set_health(RecoveryStoreHealth::Corrupt);
        let result = self
            .service_mut()?
            .reconcile_release_authorization(ReleaseReconciliation {
                operation: token,
                operation_effect: actual_media(facts.mediaEffect),
                requirement: actual_requirement(facts.transaction),
                recovery: RecoveryReconciliation::Unresolved,
                basis: BasisConformance::Consumed,
            });
        self.service_mut()?
            .recovery_mut()
            .set_health(RecoveryStoreHealth::Healthy);
        if result.is_ok_and(|authorization| authorization.is_some())
            || self.service()?.release_authorization(token).is_some()
        {
            bail!("unresolved recovery evidence produced an authorization");
        }
        Ok(())
    }

    fn attempt_unresolved_basis(&mut self, id: OperationGeneration) -> Result {
        let token = self.token(&id)?;
        let facts = self.record(&id)?.facts;
        if !matches!(
            facts.mediaEffect,
            MediaEffect::MediaTerminal | MediaEffect::MediaReconciled
        ) || facts.children != ChildOutcome::ChildrenTerminal
            || !facts.requiredReconciliation
            || !facts.reclaimable
            || facts.transaction != TransactionRequirement::TransactionSatisfied
            || facts.recovery != RecoveryObservation::RecoveryAuthoritative
        {
            bail!("basis-negative probe did not first satisfy every other observed prerequisite");
        }
        let result = self
            .service_mut()?
            .reconcile_release_authorization(ReleaseReconciliation {
                operation: token,
                operation_effect: actual_media(facts.mediaEffect),
                requirement: actual_requirement(facts.transaction),
                recovery: RecoveryReconciliation::Authoritative,
                basis: BasisConformance::Unresolved,
            });
        if result.is_ok_and(|authorization| authorization.is_some())
            || self.service()?.release_authorization(token).is_some()
        {
            bail!("unresolved basis evidence produced an authorization");
        }
        Ok(())
    }
}

fn model_ids() -> [OperationGeneration; 2] {
    [
        OperationGeneration {
            operation: "slot-a".to_owned(),
            generation: 0,
        },
        OperationGeneration {
            operation: "slot-b".to_owned(),
            generation: 0,
        },
    ]
}

fn owner_facts_complete(facts: OwnerFacts) -> bool {
    matches!(
        facts.mediaEffect,
        MediaEffect::MediaTerminal | MediaEffect::MediaReconciled
    ) && facts.children == ChildOutcome::ChildrenTerminal
        && facts.requiredReconciliation
        && facts.reclaimable
        && facts.transaction == TransactionRequirement::TransactionSatisfied
        && facts.recovery == RecoveryObservation::RecoveryAuthoritative
        && !matches!(facts.basis, BasisObservation::BasisUnresolved)
}

fn actual_media(media: MediaEffect) -> OperationEffect {
    match media {
        MediaEffect::MediaReconciled => OperationEffect::AuthoritativelyReconciled,
        MediaEffect::MediaTerminal => OperationEffect::Terminal,
        MediaEffect::MediaUnresolved => OperationEffect::Unresolved,
    }
}

fn actual_requirement(requirement: TransactionRequirement) -> ReleaseRequirement {
    match requirement {
        TransactionRequirement::TransactionSatisfied => ReleaseRequirement::TransactionSatisfied,
        TransactionRequirement::TransactionUnresolved => ReleaseRequirement::TransactionUnresolved,
    }
}

fn basis_conformance(basis: ActualBasis) -> BasisConformance {
    match basis {
        ActualBasis::Consumed => BasisConformance::Consumed,
        ActualBasis::Discarded => BasisConformance::Discarded,
        ActualBasis::Reconciled => BasisConformance::Reconciled,
    }
}

fn actual_basis(basis: BasisObservation) -> BasisConformance {
    match basis {
        BasisObservation::BasisConsumed => BasisConformance::Consumed,
        BasisObservation::BasisDiscarded => BasisConformance::Discarded,
        BasisObservation::BasisReconciled => BasisConformance::Reconciled,
        BasisObservation::BasisUnresolved => BasisConformance::Unresolved,
    }
}

fn drive_transaction_to_release(machine: &mut TransactionMachine) -> Result {
    let plan = connect_plan();
    if transaction_requirement(Some(machine.trace())) != ReleaseRequirement::TransactionUnresolved {
        bail!("transaction mapping claimed release satisfaction before ReleaseRange");
    }
    loop {
        let action = machine
            .pending_action()
            .context("transaction ended before its release boundary")?
            .kind();
        let result = match action {
            ActionKind::AcquireRange => {
                bail!("Connect transaction unexpectedly requested a second range acquisition")
            }
            ActionKind::PersistDirtyAndInvalidateIntegrity => {
                ActionResult::WriteRecoveryRecordDurable(CommittedRecoveryGeneration::new(
                    plan.recovery_generation,
                    plan.topology_epoch,
                ))
            }
            ActionKind::ReadSet => ActionResult::ReadSetComplete(SemanticIoResult::complete()),
            ActionKind::ComputeParity => {
                ActionResult::ParityComputed(ComputationResult::complete())
            }
            ActionKind::WriteSet => ActionResult::WriteSetComplete(SemanticIoResult::complete()),
            ActionKind::FlushSet => ActionResult::FlushSetComplete(connect_durable_evidence()?),
            ActionKind::CommitRecoveryClean => {
                ActionResult::RecoveryCleanCommitted(CommittedRecoveryGeneration::new(
                    plan.recovery_generation
                        .checked_next()
                        .context("Connect recovery generation exhausted")?,
                    plan.topology_epoch,
                ))
            }
            ActionKind::ReleaseRange => {
                machine.apply(ActionResult::RangeReleased)?;
                break;
            }
        };
        machine.apply(result)?;
    }

    if transaction_requirement(Some(machine.trace())) != ReleaseRequirement::TransactionSatisfied {
        bail!("completed transaction trace did not establish release satisfaction");
    }
    Ok(())
}

fn connect_durable_evidence() -> Result<TransactionPersistenceEvidence> {
    let plan = connect_plan();
    let watermark = plan
        .through
        .first()
        .context("Connect plan has no store watermark")?;
    let evidence = TransactionPersistenceEvidence::durable(FenceCertificate::new(
        plan.topology_epoch,
        plan.fence_domain,
        vec![StoreFenceRef {
            fence_id: FenceId(1),
            store_id: watermark.store,
            store_incarnation: watermark.incarnation,
            topology_epoch: plan.topology_epoch,
            through: watermark.through,
            capability_evidence_id: CapabilityEvidenceId(1),
        }],
        vec![],
    ));
    if !evidence.covers(
        plan.topology_epoch,
        plan.fence_domain,
        &plan.through,
        &plan.dirty_regions,
        &plan.checksum_extents,
        plan.recovery_generation,
    ) {
        bail!("Connect persistence evidence does not cover its transaction plan");
    }
    Ok(evidence)
}

fn connect_range() -> ByteRange {
    ByteRange::new(0, BLOCK as u64).expect("bounded Connect range")
}

fn request_for(id: &OperationGeneration) -> BlockRequest {
    let request_id = if id.operation == "slot-a" { 100 } else { 101 };
    request(
        RequestId(request_id),
        connect_plan().topology_epoch,
        0,
        BlockOp::Write,
        connect_range(),
        DurabilityIntent::Ordinary,
    )
}

// Semantic transaction values live here; dependent evidence derives from this plan.
fn connect_plan() -> TransactionPlan {
    let topology_epoch = TopologyEpoch(4);
    let recovery_generation = RecoveryGeneration(0);
    let fence_domain = FenceDomain(1);
    let range = ByteRange::new(0, 1).expect("bounded transaction range");
    let watermark = StoreWatermark::new(StoreId(1), StoreWriteWatermark(1));
    TransactionPlan::new(topology_epoch, recovery_generation, fence_domain)
        .with_ranges(vec![ParityRange::new(RegionId(0), watermark.store, range)])
        .with_reads(vec![PlannedRead::new(watermark.store, range)])
        .with_writes(vec![PlannedWrite::new(watermark.store, range)])
        .with_stores(vec![watermark.store])
        .with_watermarks(vec![watermark])
}

#[quint_run(
    spec = "../../verification/quint/LifecycleReleaseConnect.qnt",
    main = "LifecycleReleaseConnect",
    init = "connectInit",
    step = "successStep",
    max_samples = 1,
    max_steps = 12,
    seed = "22082026"
)]
fn lifecycle_release_connect_success() -> impl Driver {
    BridgeDriver::for_path(Path::Success)
}

#[quint_run(
    spec = "../../verification/quint/LifecycleReleaseConnect.qnt",
    main = "LifecycleReleaseConnect",
    init = "connectInit",
    step = "failureStep",
    max_samples = 1,
    max_steps = 12,
    seed = "1"
)]
fn lifecycle_release_connect_cleanup_failure() -> impl Driver {
    BridgeDriver::for_path(Path::Failure)
}

#[quint_run(
    spec = "../../verification/quint/LifecycleReleaseConnect.qnt",
    main = "LifecycleReleaseConnect",
    init = "connectInit",
    step = "negativeStep",
    max_samples = 1,
    max_steps = 18,
    seed = "90210"
)]
fn lifecycle_release_connect_negative_paths() -> impl Driver {
    BridgeDriver::for_path(Path::Negative)
}
