use super::*;
use anyhow::{Context, bail};
use dwv_store::{
    ChildOperationSnapshot, CompletionDisposition, OperationSlotToken, ReconciliationOutcome,
    SlotState, StoreId,
};
use quint_connect::{Config, Driver, Result, State, Step, quint_run};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
struct ModelOperationGeneration {
    operation: String,
    generation: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum ModelSlotState {
    SlotReserved,
    SlotSubmitted,
    SlotPartiallyCompleted,
    SlotCompletionUncertain,
    SlotAwaitingReconciliation,
    SlotReclaimable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum ModelSlotChildState {
    SlotChildRegistered,
    SlotChildAccepted,
    SlotChildTerminal,
    SlotChildRefused,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum ModelWorkState {
    WorkPlanned,
    WorkEmitted,
    WorkAccepted,
    WorkExecuted,
    WorkDelivered,
    WorkRefused,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum ModelCompletionDisposition {
    DispositionNone,
    DispositionSuccess,
    DispositionShort,
    DispositionFailed,
    DispositionUncertain,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum ModelReconciliationState {
    ReconciliationNone,
    ReconciliationDurable,
    ReconciliationUncertainRetained,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum ModelChildAction {
    ActionBasisData,
    ActionBasisParity,
    ActionDataWrite,
    ActionParityWrite,
    ActionDataFlush,
    ActionParityFlush,
    ActionGeneric,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ModelChildIdentity {
    operation: ModelOperationGeneration,
    store: String,
    incarnation: String,
    topology: String,
    range: String,
    kind: ModelChildAction,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ModelChild {
    slotState: ModelSlotChildState,
    work: ModelWorkState,
    identity: ModelChildIdentity,
    completion: ModelCompletionDisposition,
    duplicateSeen: bool,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ModelCoreState {
    operation: ModelOperationGeneration,
    slot: ModelSlotState,
    children: HashMap<String, ModelChild>,
    acceptedAtAbandon: HashSet<String>,
    reconciliation: ModelReconciliationState,
    abandoned: bool,
}

#[derive(Clone, Copy, Debug)]
enum Probe {
    Success,
    Short,
    Failure,
    Uncertainty,
    Identity,
}

impl Probe {
    const fn read(self) -> FakeRead {
        match self {
            Self::Success | Self::Identity => FakeRead::Exact,
            Self::Short => FakeRead::Short,
            Self::Failure => FakeRead::Failed,
            Self::Uncertainty => FakeRead::Uncertain,
        }
    }
}

fn probe_matches(probe: Probe, disposition: &CompletionDisposition) -> bool {
    matches!(
        (probe, disposition),
        (
            Probe::Success | Probe::Identity,
            CompletionDisposition::Success
        ) | (Probe::Short, CompletionDisposition::Short)
            | (Probe::Failure, CompletionDisposition::Failed(_))
            | (Probe::Uncertainty, CompletionDisposition::Uncertain)
    )
}

struct CoreBridgeDriver {
    probe: Probe,
    service: Option<HealthyPortableService<FakeStore, dwv_recovery::MemoryRecoveryStore>>,
    submission: Option<PortableWriteSubmission>,
    emitted: HashMap<String, PortableWriteWork>,
    accepted: HashMap<String, AcceptedPortableWriteWork>,
    results: HashMap<String, PortableWriteResult>,
    delivered: HashMap<String, PortableWriteResult>,
    accepted_at_abandon: HashSet<String>,
    basis_granted: bool,
}

impl CoreBridgeDriver {
    const CHILD_IDS: [&'static str; 6] = [
        "basis-data",
        "basis-parity",
        "data-write",
        "parity-write",
        "data-flush",
        "parity-flush",
    ];

    fn new(probe: Probe) -> Self {
        Self {
            probe,
            service: None,
            submission: None,
            emitted: HashMap::new(),
            accepted: HashMap::new(),
            results: HashMap::new(),
            delivered: HashMap::new(),
            accepted_at_abandon: HashSet::new(),
            basis_granted: false,
        }
    }

    fn service(
        &self,
    ) -> Result<&HealthyPortableService<FakeStore, dwv_recovery::MemoryRecoveryStore>> {
        self.service
            .as_ref()
            .context("retained-operation core Connect driver has no service")
    }

    fn service_mut(
        &mut self,
    ) -> Result<&mut HealthyPortableService<FakeStore, dwv_recovery::MemoryRecoveryStore>> {
        self.service
            .as_mut()
            .context("retained-operation core Connect driver has no service")
    }

    fn submission(&self) -> Result<PortableWriteSubmission> {
        self.submission
            .context("retained-operation core Connect driver has no submission")
    }

    fn token(&self) -> Result<OperationSlotToken> {
        Ok(self.submission()?.operation)
    }

    fn init(&mut self) -> Result {
        let epoch = TopologyEpoch(4);
        let request = request(
            RequestId(700),
            epoch,
            0,
            BlockOp::Write,
            ByteRange::new(0, u64::from(BLOCK)).expect("bounded Connect range"),
            DurabilityIntent::Ordinary,
        );
        let mut service = fake_service(self.probe.read(), ServiceConfig::default());
        service.defer_release_for_test();
        let submission = service.submit_write(request, &[0x5a; BLOCK as usize])?;
        let index =
            usize::try_from(submission.operation.index).context("Connect slot index overflow")?;
        let retained = service
            .write_drivers
            .get(index)
            .and_then(Option::as_ref)
            .context("submit did not retain a write driver")?;
        if retained.machine.pending_action().is_none() {
            bail!("submit returned without a retained transaction action");
        }
        self.submission = Some(submission);
        self.service = Some(service);
        Ok(())
    }

    fn emit(&mut self, id: &str) -> Result {
        if !self.basis_granted {
            let submission = self.submission()?;
            self.service_mut()?
                .grant_basis_read_permission(&submission)?;
            self.basis_granted = true;
        }
        let submission = self.submission()?;
        let turn = self.service_mut()?.drive_write(&submission)?;
        let PortableWriteDrive::Work(work) = turn else {
            bail!("service did not emit retained child {id}");
        };
        if child_name(work.identity.operation_id.index) != Some(id) {
            bail!(
                "service emitted {:?} while model requested {id}",
                work.identity
            );
        }
        self.emitted.insert(id.to_owned(), work);
        Ok(())
    }

    fn accept(&mut self, id: &str) -> Result {
        let work = self
            .emitted
            .remove(id)
            .with_context(|| format!("model accepted child {id} before emission"))?;
        let accepted = self.service_mut()?.accept_write_work(&work)?;
        self.accepted.insert(id.to_owned(), accepted);
        Ok(())
    }

    fn execute(&mut self, id: &str) -> Result {
        let accepted = self
            .accepted
            .remove(id)
            .with_context(|| format!("model executed child {id} before acceptance"))?;
        let result = self.service_mut()?.execute_write_work(accepted)?;
        if !probe_matches(self.probe, &result.completion.disposition) {
            bail!(
                "probe {:?} produced unexpected completion {:?}",
                self.probe,
                result.completion.disposition
            );
        }
        self.results.insert(id.to_owned(), result);
        Ok(())
    }

    fn deliver(&mut self, id: &str) -> Result {
        let result = self
            .results
            .remove(id)
            .with_context(|| format!("model delivered child {id} before execution"))?;
        if !probe_matches(self.probe, &result.completion.disposition) {
            bail!(
                "probe {:?} delivered unexpected completion {:?}",
                self.probe,
                result.completion.disposition
            );
        }
        self.delivered.insert(id.to_owned(), result.clone());
        let probe = self.probe;
        match (probe, self.service_mut()?.deliver_write_result(result)) {
            (Probe::Success | Probe::Identity, Ok(_)) => {}
            (Probe::Success | Probe::Identity, Err(error)) => {
                bail!("successful Connect delivery failed: {error}");
            }
            (Probe::Short | Probe::Failure | Probe::Uncertainty, Ok(_)) => {
                bail!("conservative Connect delivery unexpectedly succeeded");
            }
            (
                Probe::Short | Probe::Failure | Probe::Uncertainty,
                Err(ServiceError::Io {
                    completion: Some(completion),
                    ..
                }),
            ) if probe_matches(probe, &completion.disposition) => {}
            (_, Err(error)) => {
                bail!("Connect delivery returned an unexpected error: {error}");
            }
        }
        Ok(())
    }

    fn duplicate(&mut self, id: &str) -> Result {
        let result = self
            .delivered
            .get(id)
            .cloned()
            .with_context(|| format!("model duplicated child {id} before delivery"))?;
        self.service_mut()?.deliver_write_result(result)?;
        Ok(())
    }

    fn duplicate_mismatched(&mut self, id: &str) -> Result {
        let mut result = self
            .delivered
            .get(id)
            .cloned()
            .with_context(|| format!("model duplicated child {id} before delivery"))?;
        result.completion.disposition =
            CompletionDisposition::Failed(StoreError::BackendFailure { code: 99 });
        self.service_mut()?.deliver_write_result(result)?;
        Ok(())
    }

    fn abandon(&mut self) -> Result {
        let token = self.token()?;
        let snapshot = self.service()?.admission.snapshot(token)?;
        self.accepted_at_abandon = snapshot
            .children
            .iter()
            .filter(|child| child.submission.is_some() && !child.terminal)
            .filter_map(|child| child_name(child.operation_id.index))
            .map(str::to_owned)
            .collect();
        self.service_mut()?.abandon(token)?;
        Ok(())
    }

    fn reconcile(&mut self) -> Result {
        let token = self.token()?;
        self.service_mut()?
            .admission
            .record_reconciliation(token, ReconciliationOutcome::Durable)?;
        Ok(())
    }

    fn probe_identity_rejection(&mut self) -> Result {
        let work = self
            .emitted
            .get("basis-data")
            .cloned()
            .context("identity probe has no emitted basis work")?;
        let mut generation = work.clone();
        generation.identity.operation_id.slot.generation = generation
            .identity
            .operation_id
            .slot
            .generation
            .wrapping_add(1);
        let mut store = work.clone();
        store.identity.store_id = StoreId(99);
        let mut incarnation = work.clone();
        incarnation.identity.store_incarnation = StoreIncarnationId(99);
        let mut topology = work.clone();
        topology.identity.topology_epoch = TopologyEpoch(99);
        let mut range = work.clone();
        range.range = ByteRange::new(1, 1).expect("bounded mismatch range");
        let mut action = work;
        if let PortableWriteAction::BasisRead { destination } = &mut action.action {
            destination.index = destination.index.wrapping_add(1);
        }
        for (name, mismatched) in [
            ("operation generation", generation),
            ("store", store),
            ("store incarnation", incarnation),
            ("topology", topology),
            ("range", range),
            ("action", action),
        ] {
            let token = self.token()?;
            let before = self.service()?.admission.snapshot(token)?;
            let error = self
                .service_mut()?
                .accept_write_work(&mismatched)
                .expect_err("mismatched identity was accepted");
            let after = self.service()?.admission.snapshot(token)?;
            if before != after {
                bail!("identity mismatch {name} mutated the current slot");
            }
            if !matches!(
                error,
                ServiceError::Io {
                    class: FailureClass::Identity | FailureClass::Admission,
                    ..
                }
            ) {
                bail!("identity mismatch {name} returned the wrong error: {error:?}");
            }
        }
        Ok(())
    }

    fn probe_completion_rejection(&mut self) -> Result {
        let mut result = self
            .results
            .get("basis-data")
            .cloned()
            .context("completion identity probe has no executed basis result")?;
        result.completion.operation_id.slot.generation = result
            .completion
            .operation_id
            .slot
            .generation
            .wrapping_add(1);
        let token = self.token()?;
        let before = self.service()?.admission.snapshot(token)?;
        let error = self
            .service_mut()?
            .deliver_write_result(result)
            .expect_err("mismatched completion identity was accepted");
        let after = self.service()?.admission.snapshot(token)?;
        if before != after {
            bail!("mismatched completion identity mutated the current slot");
        }
        if !matches!(
            error,
            ServiceError::Io {
                class: FailureClass::Admission,
                completion: Some(_),
                ..
            }
        ) {
            bail!("completion identity probe returned the wrong error: {error:?}");
        }
        Ok(())
    }

    fn model_state(&self) -> Result<ModelCoreState> {
        let token = self.token()?;
        let service = self.service()?;
        let snapshot = service.admission.snapshot(token)?;
        let index =
            usize::try_from(token.index).context("Connect slot index does not fit usize")?;
        let driver = service
            .write_drivers
            .get(index)
            .and_then(Option::as_ref)
            .context("retained write driver was not present")?;
        let operation = ModelOperationGeneration {
            operation: "slot-a".to_owned(),
            generation: i64::from(token.generation),
        };
        let mut children = HashMap::new();
        for child in &snapshot.children {
            let id = child_name(child.operation_id.index).with_context(|| {
                format!(
                    "unexpected Connect child index {}",
                    child.operation_id.index
                )
            })?;
            let pending = driver
                .work
                .iter()
                .find(|pending| pending.work.identity.operation_id == child.operation_id);
            let slot_state = if child.refused_before_acceptance {
                ModelSlotChildState::SlotChildRefused
            } else if child.terminal {
                ModelSlotChildState::SlotChildTerminal
            } else if child.submission.is_some() {
                ModelSlotChildState::SlotChildAccepted
            } else {
                ModelSlotChildState::SlotChildRegistered
            };
            let work = if child.refused_before_acceptance {
                ModelWorkState::WorkRefused
            } else if self.results.contains_key(id) {
                ModelWorkState::WorkExecuted
            } else {
                pending.map_or(ModelWorkState::WorkPlanned, |pending| match pending.state {
                    WriteWorkState::Planned => ModelWorkState::WorkPlanned,
                    WriteWorkState::Emitted => ModelWorkState::WorkEmitted,
                    WriteWorkState::Accepted => ModelWorkState::WorkAccepted,
                    WriteWorkState::Completed => ModelWorkState::WorkDelivered,
                })
            };
            if child.submission.is_some() && pending.is_none() {
                bail!("accepted child {id} was absent from the production driver");
            }
            let completion = child
                .completion
                .as_ref()
                .map(model_disposition)
                .unwrap_or(ModelCompletionDisposition::DispositionNone);
            let identity = match pending {
                Some(pending) => {
                    if pending.work.range != child.requested {
                        bail!("production range for child {id} disagreed with the slot");
                    }
                    model_identity(&operation, id, &pending.work)?
                }
                None => {
                    let (kind, member_index) =
                        if driver.data_write_children.contains(&child.operation_id) {
                            (ModelChildAction::ActionDataWrite, driver.member_index)
                        } else if driver.parity_write_children.contains(&child.operation_id) {
                            (ModelChildAction::ActionParityWrite, driver.parity_index)
                        } else if driver.flush_children[0] == child.operation_id {
                            (ModelChildAction::ActionDataFlush, driver.member_index)
                        } else if driver.flush_children[1] == child.operation_id {
                            (ModelChildAction::ActionParityFlush, driver.parity_index)
                        } else {
                            bail!("production driver omitted child {id} and its plan");
                        };
                    if kind != model_kind(id) {
                        bail!("production child plan disagreed with child name {id}");
                    }
                    let member = service
                        .members
                        .get(member_index)
                        .with_context(|| format!("missing production member for child {id}"))?;
                    ModelChildIdentity {
                        operation: operation.clone(),
                        store: model_store(member.store_id)?.to_owned(),
                        incarnation: format!("incarnation-{}", member.store.incarnation().0),
                        topology: format!("topology-{}", member.topology_epoch.0),
                        range: model_range(child.requested)?,
                        kind,
                    }
                }
            };
            children.insert(
                id.to_owned(),
                ModelChild {
                    slotState: slot_child_state(child),
                    work,
                    identity,
                    completion,
                    duplicateSeen: child.duplicate_deliveries != 0,
                },
            );
            if slot_state != slot_child_state(child) {
                bail!("internal slot-state mapping disagreement for {id}");
            }
        }
        if children.len() != Self::CHILD_IDS.len() {
            bail!("service exposed {} children instead of six", children.len());
        }
        Ok(ModelCoreState {
            operation,
            slot: model_slot_state(snapshot.state)?,
            children,
            acceptedAtAbandon: self.accepted_at_abandon.clone(),
            reconciliation: snapshot
                .reconciliation
                .map(model_reconciliation)
                .unwrap_or(ModelReconciliationState::ReconciliationNone),
            abandoned: snapshot.abandoned,
        })
    }
}

impl State<CoreBridgeDriver> for ModelCoreState {
    fn from_driver(driver: &CoreBridgeDriver) -> Result<Self> {
        driver.model_state()
    }
}

impl Driver for CoreBridgeDriver {
    type State = ModelCoreState;

    fn config() -> Config {
        Config {
            state: &[
                "PortableOperationExecutionCoreConnect::PortableOperationExecutionCore::state",
            ],
            ..Config::default()
        }
    }

    fn step(&mut self, step: &Step) -> Result {
        quint_connect::switch!(step {
            connectInit => self.init()?,
            emitBasisData => self.emit("basis-data")?,
            emitBasisParity => self.emit("basis-parity")?,
            emitDataWrite => self.emit("data-write")?,
            emitParityWrite => self.emit("parity-write")?,
            emitDataFlush => self.emit("data-flush")?,
            emitParityFlush => self.emit("parity-flush")?,
            acceptBasisData => self.accept("basis-data")?,
            acceptBasisParity => self.accept("basis-parity")?,
            acceptDataWrite => self.accept("data-write")?,
            acceptParityWrite => self.accept("parity-write")?,
            acceptDataFlush => self.accept("data-flush")?,
            acceptParityFlush => self.accept("parity-flush")?,
            executeBasisData => self.execute("basis-data")?,
            executeBasisParity => self.execute("basis-parity")?,
            executeDataWrite => self.execute("data-write")?,
            executeParityWrite => self.execute("parity-write")?,
            executeDataFlush => self.execute("data-flush")?,
            executeParityFlush => self.execute("parity-flush")?,
            deliverBasisData => self.deliver("basis-data")?,
            deliverShortBasisData => self.deliver("basis-data")?,
            deliverFailureBasisData => self.deliver("basis-data")?,
            deliverUncertaintyBasisData => self.deliver("basis-data")?,
            deliverFailureBasisDataAfterParity => self.deliver("basis-data")?,
            deliverFailureBasisParityAfterFailure => self.deliver("basis-parity")?,
            deliverBasisParity => self.deliver("basis-parity")?,
            deliverDataWrite => self.deliver("data-write")?,
            deliverParityWrite => self.deliver("parity-write")?,
            deliverDataFlush => self.deliver("data-flush")?,
            deliverParityFlush => self.deliver("parity-flush")?,
            duplicateParityWrite => self.duplicate("parity-write")?,
            duplicateParityWriteMismatched => self.duplicate_mismatched("parity-write")?,
            abandonDeliverBasisData => self.deliver("basis-data")?,
            abandonDeliverBasisParity => self.deliver("basis-parity")?,
            abandonReconcileDurable => self.reconcile()?,
            abandonOperation => self.abandon()?,
            reconcileDurable => self.reconcile()?,
            reconcileAfterFailureSibling => self.reconcile()?,
            probeIdentityRejection => self.probe_identity_rejection()?,
            probeCompletionRejection => self.probe_completion_rejection()?,
        })
    }
}

fn child_name(index: u32) -> Option<&'static str> {
    CoreBridgeDriver::CHILD_IDS
        .get(usize::try_from(index).ok()?)
        .copied()
}

fn model_kind(id: &str) -> ModelChildAction {
    match id {
        "basis-data" => ModelChildAction::ActionBasisData,
        "basis-parity" => ModelChildAction::ActionBasisParity,
        "data-write" => ModelChildAction::ActionDataWrite,
        "parity-write" => ModelChildAction::ActionParityWrite,
        "data-flush" => ModelChildAction::ActionDataFlush,
        "parity-flush" => ModelChildAction::ActionParityFlush,
        _ => ModelChildAction::ActionGeneric,
    }
}
fn model_kind_from_work(id: &str, work: &PortableWriteWork) -> Result<ModelChildAction> {
    let kind = match (&work.action, work.identity.store_id) {
        (PortableWriteAction::BasisRead { destination }, StoreId(1)) => {
            if *destination != work.identity.operation_id {
                bail!("child {id} has a mismatched basis-read destination");
            }
            ModelChildAction::ActionBasisData
        }
        (PortableWriteAction::BasisRead { destination }, StoreId(3)) => {
            if *destination != work.identity.operation_id {
                bail!("child {id} has a mismatched basis-read destination");
            }
            ModelChildAction::ActionBasisParity
        }
        (PortableWriteAction::Write { .. }, StoreId(1)) => ModelChildAction::ActionDataWrite,
        (PortableWriteAction::Write { .. }, StoreId(3)) => ModelChildAction::ActionParityWrite,
        (PortableWriteAction::Flush { .. }, StoreId(1)) => ModelChildAction::ActionDataFlush,
        (PortableWriteAction::Flush { .. }, StoreId(3)) => ModelChildAction::ActionParityFlush,
        (_, store) => bail!("child {id} has an unexpected action/store pair: {store:?}"),
    };
    if kind != model_kind(id) {
        bail!("child {id} has an unexpected observed action kind: {kind:?}");
    }
    Ok(kind)
}

fn model_store(store_id: StoreId) -> Result<&'static str> {
    match store_id {
        StoreId(1) => Ok("data"),
        StoreId(3) => Ok("parity"),
        other => bail!("unexpected production store {other:?}"),
    }
}

fn model_range(range: ByteRange) -> Result<String> {
    if range == ByteRange::empty() {
        Ok("empty".to_owned())
    } else if range == ByteRange::new(0, u64::from(BLOCK)).expect("bounded range") {
        Ok("block-0".to_owned())
    } else {
        bail!("unexpected production range {range:?}")
    }
}

fn model_identity(
    operation: &ModelOperationGeneration,
    id: &str,
    work: &PortableWriteWork,
) -> Result<ModelChildIdentity> {
    let identity = work.identity;
    if work.submission.operation != identity.operation_id.slot
        || identity.operation_id.slot.generation as i64 != operation.generation
        || identity.operation_id.slot.index != 0
    {
        bail!("child {id} has the wrong operation slot identity: {identity:?}");
    }
    let store = model_store(identity.store_id)?;
    let range = model_range(work.range)?;
    Ok(ModelChildIdentity {
        operation: operation.clone(),
        store: store.to_owned(),
        incarnation: format!("incarnation-{}", identity.store_incarnation.0),
        topology: format!("topology-{}", identity.topology_epoch.0),
        range: range.to_owned(),
        kind: model_kind_from_work(id, work)?,
    })
}

fn slot_child_state(child: &ChildOperationSnapshot) -> ModelSlotChildState {
    if child.refused_before_acceptance {
        ModelSlotChildState::SlotChildRefused
    } else if child.terminal {
        ModelSlotChildState::SlotChildTerminal
    } else if child.submission.is_some() {
        ModelSlotChildState::SlotChildAccepted
    } else {
        ModelSlotChildState::SlotChildRegistered
    }
}

fn model_slot_state(state: SlotState) -> Result<ModelSlotState> {
    Ok(match state {
        SlotState::Reserved => ModelSlotState::SlotReserved,
        SlotState::Submitted => ModelSlotState::SlotSubmitted,
        SlotState::PartiallyCompleted => ModelSlotState::SlotPartiallyCompleted,
        SlotState::CompletionUncertain => ModelSlotState::SlotCompletionUncertain,
        SlotState::AwaitingReconciliation => ModelSlotState::SlotAwaitingReconciliation,
        SlotState::Reclaimable => ModelSlotState::SlotReclaimable,
        SlotState::Draining => bail!("Connect core path unexpectedly entered drain state"),
    })
}

fn model_reconciliation(outcome: ReconciliationOutcome) -> ModelReconciliationState {
    match outcome {
        ReconciliationOutcome::Durable => ModelReconciliationState::ReconciliationDurable,
        ReconciliationOutcome::UncertainRetained => {
            ModelReconciliationState::ReconciliationUncertainRetained
        }
        ReconciliationOutcome::Invalidated => ModelReconciliationState::ReconciliationNone,
    }
}

fn model_disposition(completion: &StoreCompletion) -> ModelCompletionDisposition {
    match completion.disposition {
        CompletionDisposition::Success => ModelCompletionDisposition::DispositionSuccess,
        CompletionDisposition::Short => ModelCompletionDisposition::DispositionShort,
        CompletionDisposition::Failed(_) => ModelCompletionDisposition::DispositionFailed,
        CompletionDisposition::Uncertain => ModelCompletionDisposition::DispositionUncertain,
        CompletionDisposition::Duplicate => ModelCompletionDisposition::DispositionNone,
    }
}

#[quint_run(
    spec = "../../verification/quint/PortableOperationExecutionCoreConnect.qnt",
    main = "PortableOperationExecutionCoreConnect",
    init = "connectInit",
    step = "successStep",
    max_samples = 1,
    max_steps = 28,
    seed = "9001"
)]
fn portable_operation_core_connect_success() -> impl Driver {
    CoreBridgeDriver::new(Probe::Success)
}

#[quint_run(
    spec = "../../verification/quint/PortableOperationExecutionCoreConnect.qnt",
    main = "PortableOperationExecutionCoreConnect",
    init = "connectInit",
    step = "duplicateDispositionStep",
    max_samples = 1,
    max_steps = 18,
    seed = "9007"
)]
fn portable_operation_core_connect_duplicate_disposition_probe() -> impl Driver {
    CoreBridgeDriver::new(Probe::Success)
}

#[quint_run(
    spec = "../../verification/quint/PortableOperationExecutionCoreConnect.qnt",
    main = "PortableOperationExecutionCoreConnect",
    init = "connectInit",
    step = "abandonmentStep",
    max_samples = 1,
    max_steps = 12,
    seed = "9002"
)]
fn portable_operation_core_connect_abandonment() -> impl Driver {
    CoreBridgeDriver::new(Probe::Success)
}

#[quint_run(
    spec = "../../verification/quint/PortableOperationExecutionCoreConnect.qnt",
    main = "PortableOperationExecutionCoreConnect",
    init = "connectInit",
    step = "failureStep",
    max_samples = 1,
    max_steps = 5,
    seed = "9003"
)]
fn portable_operation_core_connect_failure_probe() -> impl Driver {
    CoreBridgeDriver::new(Probe::Failure)
}

#[quint_run(
    spec = "../../verification/quint/PortableOperationExecutionCoreConnect.qnt",
    main = "PortableOperationExecutionCoreConnect",
    init = "connectInit",
    step = "shortStep",
    max_samples = 1,
    max_steps = 5,
    seed = "9006"
)]
fn portable_operation_core_connect_short_probe() -> impl Driver {
    CoreBridgeDriver::new(Probe::Short)
}

#[quint_run(
    spec = "../../verification/quint/PortableOperationExecutionCoreConnect.qnt",
    main = "PortableOperationExecutionCoreConnect",
    init = "connectInit",
    step = "uncertaintyStep",
    max_samples = 1,
    max_steps = 5,
    seed = "9004"
)]
fn portable_operation_core_connect_uncertainty_probe() -> impl Driver {
    CoreBridgeDriver::new(Probe::Uncertainty)
}

#[quint_run(
    spec = "../../verification/quint/PortableOperationExecutionCoreConnect.qnt",
    main = "PortableOperationExecutionCoreConnect",
    init = "connectInit",
    step = "identityStep",
    max_samples = 1,
    max_steps = 4,
    seed = "9005"
)]
fn portable_operation_core_connect_identity_probe() -> impl Driver {
    CoreBridgeDriver::new(Probe::Identity)
}

#[quint_run(
    spec = "../../verification/quint/PortableOperationExecutionCoreConnect.qnt",
    main = "PortableOperationExecutionCoreConnect",
    init = "connectInit",
    step = "resultIdentityStep",
    max_samples = 1,
    max_steps = 5,
    seed = "9008"
)]
fn portable_operation_core_connect_result_identity_probe() -> impl Driver {
    CoreBridgeDriver::new(Probe::Identity)
}

#[quint_run(
    spec = "../../verification/quint/PortableOperationExecutionCoreConnect.qnt",
    main = "PortableOperationExecutionCoreConnect",
    init = "connectInit",
    step = "failureSiblingStep",
    max_samples = 1,
    max_steps = 10,
    seed = "9009"
)]
fn portable_operation_core_connect_failure_preserves_sibling() -> impl Driver {
    CoreBridgeDriver::new(Probe::Failure)
}
