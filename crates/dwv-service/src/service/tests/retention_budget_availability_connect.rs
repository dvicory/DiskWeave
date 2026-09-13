use super::release_authorization::ReleaseAuthorizationLedger;
use crate::evidence::ReleaseAuthorization;
use anyhow::{Context, bail};
use dwv_core::{
    BlockOp, BlockRequest, ByteRange, DurabilityIntent, FenceDomain, FrontendId, OrderingIntent,
    RequestId, SlotId, SubmissionSequence, TopologyEpoch,
};
use dwv_lifecycle_authority::LifecycleAuthorityOwner;
use dwv_store::{OperationSlotTable, OperationSlotToken, ReconciliationOutcome, ResourceLimits};
use quint_connect::{Config, Driver, Result, State, Step, quint_run};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ModelOperation {
    slot: i64,
    generation: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum ModelOutcome {
    OutcomeNone,
    OutcomeAdmitted,
    OutcomeRefused,
    OutcomeRetained,
    OutcomeExhausted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum ModelOutcomeRef {
    NoOutcome,
    ForOperation(ModelOperation),
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ModelBudgetState {
    retained: HashMap<String, HashSet<i64>>,
    consumers: HashMap<String, HashSet<i64>>,
    outcome: ModelOutcome,
    outcomeOp: ModelOutcomeRef,
}

struct AvailabilityBridgeDriver {
    table: OperationSlotTable,
    owner: LifecycleAuthorityOwner,
    ledger: ReleaseAuthorizationLedger,
    consumers: HashSet<(u32, u32)>,
    tokens: HashMap<(u32, u32), OperationSlotToken>,
    authorizations: HashMap<(u32, u32), ReleaseAuthorization>,
    occupants: HashMap<u32, OperationSlotToken>,
    last_outcome: ModelOutcome,
    last_op: Option<ModelOperation>,
    next_request: u64,
}

impl AvailabilityBridgeDriver {
    fn new() -> Self {
        let (owner, _verifier) = LifecycleAuthorityOwner::new();
        Self {
            table: OperationSlotTable::new(ResourceLimits::new(2, 0, 0, 0, 0, 0)),
            owner,
            ledger: ReleaseAuthorizationLedger::new(2, 2),
            consumers: HashSet::new(),
            tokens: HashMap::new(),
            authorizations: HashMap::new(),
            occupants: HashMap::new(),
            last_outcome: ModelOutcome::OutcomeNone,
            last_op: None,
            next_request: 800,
        }
    }

    fn build_request(&mut self, slot_hint: u32) -> BlockRequest {
        self.next_request += 1;
        BlockRequest::new(
            RequestId(self.next_request),
            FrontendId(1),
            SlotId([slot_hint as u8; 16]),
            TopologyEpoch(1),
            BlockOp::Write,
            ByteRange::new(0, 1).expect("Connect range is valid"),
            None,
            OrderingIntent {
                submission_sequence: SubmissionSequence(self.next_request),
                preflush: false,
                fence_domain: FenceDomain(1),
            },
            DurabilityIntent::Ordinary,
        )
    }

    /// Reserve exactly the requested slot index, holding wrong-index
    /// reservations as filler and releasing a previous occupant first.
    /// Slot lifecycle stays with the table owner; generations advance on
    /// every reserve exactly as production advances them.
    fn produce(&mut self, slot: u32, request_hint: u32) -> Result<OperationSlotToken> {
        if let Some(occupant) = self.occupants.remove(&slot) {
            self.table
                .release(occupant)
                .map_err(|error| anyhow::anyhow!("Connect release failed: {error:?}"))?;
        }
        loop {
            let request = self.build_request(request_hint);
            let token = self
                .table
                .reserve(request)
                .map_err(|error| anyhow::anyhow!("Connect reserve failed: {error:?}"))?;
            self.table
                .record_reconciliation(token, ReconciliationOutcome::Durable)
                .map_err(|error| anyhow::anyhow!("Connect reconcile failed: {error:?}"))?;
            if token.index == slot {
                self.occupants.insert(slot, token);
                return Ok(token);
            }
            self.occupants.insert(token.index, token);
        }
    }

    fn authorize(&self, token: OperationSlotToken) -> Result<ReleaseAuthorization> {
        self.owner
            .authorize_release(&self.table, token, true, true, true, true)
            .map(ReleaseAuthorization::from_lifecycle)
            .context("Connect operation lacks complete release facts")
    }

    fn consumer_key(token: OperationSlotToken) -> (u32, u32) {
        (token.index, token.generation)
    }

    fn snapshot_consumers(&self) -> HashSet<(u32, u32)> {
        self.consumers.clone()
    }

    fn note(&mut self, outcome: ModelOutcome, slot: u32, generation: u32) {
        self.last_outcome = outcome;
        self.last_op = Some(ModelOperation {
            slot: i64::from(slot),
            generation: i64::from(generation),
        });
    }

    fn clear_note(&mut self) {
        self.last_outcome = ModelOutcome::OutcomeNone;
        self.last_op = None;
    }

    fn init(&mut self) -> Result {
        *self = Self::new();
        Ok(())
    }
    /// Mirror `remember_release_authorization`: retire the lowest discharged
    /// entry at this index first (bucket iteration is token order), then
    /// run the cleanup-composed retain. Occupied re-retains reuse the
    /// stored authorization exactly as production reuses the established one.
    fn retain(&mut self, slot: u32, generation: u32) -> Result {
        let token = match self.tokens.get(&(slot, generation)) {
            Some(token) => *token,
            None => {
                let produced = self.produce(slot, slot)?;
                if produced.generation != generation {
                    bail!(
                        "Connect script produced generation {} for model generation {generation} at slot {slot}",
                        produced.generation
                    );
                }
                produced
            }
        };
        let authorization = match self.authorizations.get(&(slot, generation)) {
            Some(authorization) => authorization.clone(),
            None => self.authorize(token)?,
        };
        let consumed = self.snapshot_consumers();
        let probe = OperationSlotToken::new(slot, 0);
        let victim = self
            .ledger
            .operations_at_index(probe)
            .find(|existing| !consumed.contains(&Self::consumer_key(*existing)));
        if let Some(victim) = victim {
            self.ledger.retire(victim);
        }
        let consumed = self.snapshot_consumers();
        let stored = authorization.clone();
        match self.ledger.retain_with_cleanup(authorization, |existing| {
            !consumed.contains(&Self::consumer_key(existing))
        }) {
            Ok(()) => {
                if self.ledger.observe(token).is_none() {
                    bail!("retained authorization is not observable");
                }
                self.authorizations.insert((slot, generation), stored);
                self.note(ModelOutcome::OutcomeRetained, slot, generation);
            }
            Err(returned) => {
                if returned.operation() != token {
                    bail!("exhausted retain returned a different authorization");
                }
                if self.ledger.observe(token).is_some() {
                    bail!("failed retain left the refused entry observable");
                }
                self.note(ModelOutcome::OutcomeExhausted, slot, generation);
            }
        }
        self.tokens.insert((slot, generation), token);
        Ok(())
    }

    fn request(&mut self, slot: u32, generation: u32) -> Result {
        let token = match self.tokens.get(&(slot, generation)) {
            Some(token) => *token,
            None => OperationSlotToken::new(slot, generation),
        };
        let consumed = self.snapshot_consumers();
        let blocked = self.ledger.is_full_and_consumed(token, |existing| {
            consumed.contains(&Self::consumer_key(existing))
        });
        if blocked {
            if self.ledger.observe(token).is_some() {
                bail!("blocked probe observes the probed entry");
            }
            self.note(ModelOutcome::OutcomeRefused, slot, generation);
        } else {
            self.note(ModelOutcome::OutcomeAdmitted, slot, generation);
        }
        Ok(())
    }

    fn consume(&mut self, slot: u32, generation: u32) -> Result {
        if !self.tokens.contains_key(&(slot, generation)) {
            bail!("consumer flag for an unproduced operation");
        }
        self.consumers.insert((slot, generation));
        self.clear_note();
        Ok(())
    }

    fn discharge(&mut self, slot: u32, generation: u32) -> Result {
        self.consumers.remove(&(slot, generation));
        self.clear_note();
        Ok(())
    }

    fn retire(&mut self, slot: u32, generation: u32) -> Result {
        let token = self
            .tokens
            .get(&(slot, generation))
            .copied()
            .context("retire of an unproduced operation")?;
        if self.ledger.retire(token).is_none() {
            bail!("exact retirement removed nothing");
        }
        if self.ledger.observe(token).is_some() {
            bail!("retired authorization is still observable");
        }
        self.clear_note();
        Ok(())
    }

    fn probe_unknown_observe(&mut self) -> Result {
        let unknown = OperationSlotToken::new(9, 9);
        if self.ledger.observe(unknown).is_some() {
            bail!("unknown identity is observable");
        }
        let consumed = self.snapshot_consumers();
        if self.ledger.is_full_and_consumed(unknown, |existing| {
            consumed.contains(&Self::consumer_key(existing))
        }) {
            bail!("unknown identity reports blocked");
        }
        Ok(())
    }
}

impl State<AvailabilityBridgeDriver> for ModelBudgetState {
    fn from_driver(driver: &AvailabilityBridgeDriver) -> Result<Self> {
        let mut retained: HashMap<String, HashSet<i64>> = HashMap::new();
        let mut consumers: HashMap<String, HashSet<i64>> = HashMap::new();
        for slot in [0u32, 1u32] {
            let probe = OperationSlotToken::new(slot, 0);
            let generations = driver
                .ledger
                .operations_at_index(probe)
                .filter(|token| driver.ledger.observe(*token).is_some())
                .map(|token| i64::from(token.generation))
                .collect::<HashSet<_>>();
            let flags = driver
                .consumers
                .iter()
                .filter(|(index, _)| *index == slot)
                .map(|(_, generation)| i64::from(*generation))
                .collect::<HashSet<_>>();
            retained.insert(slot.to_string(), generations);
            consumers.insert(slot.to_string(), flags);
        }
        Ok(Self {
            retained,
            consumers,
            outcome: driver.last_outcome,
            outcomeOp: driver
                .last_op
                .clone()
                .map_or(ModelOutcomeRef::NoOutcome, ModelOutcomeRef::ForOperation),
        })
    }
}

impl Driver for AvailabilityBridgeDriver {
    type State = ModelBudgetState;

    fn config() -> Config {
        Config {
            state: &["RetentionBudgetAvailabilityConnect::RetentionBudgetAvailability::state"],
            ..Config::default()
        }
    }

    fn step(&mut self, step: &Step) -> Result {
        quint_connect::switch!(step {
            connectInit => self.init()?,
            retainS0G1 => self.retain(0, 1)?,
            retainS0G2 => self.retain(0, 2)?,
            retainS0G3 => self.retain(0, 3)?,
            retainOccupiedS0G2 => self.retain(0, 2)?,
            retainS1G1 => self.retain(1, 1)?,
            retainS1G2 => self.retain(1, 2)?,
            retainS1G3 => self.retain(1, 3)?,
            consumeS0G1 => self.consume(0, 1)?,
            consumeS0G2 => self.consume(0, 2)?,
            consumeS1G1 => self.consume(1, 1)?,
            consumeS1G2 => self.consume(1, 2)?,
            dischargeS0G1 => self.discharge(0, 1)?,
            requestS0G3 => self.request(0, 3)?,
            requestS0G1 => self.request(0, 1)?,
            retireS0G1 => self.retire(0, 1)?,
            probeUnknownObserve => self.probe_unknown_observe()?,
        })
    }
}

#[quint_run(
    spec = "../../verification/quint/RetentionBudgetAvailabilityConnect.qnt",
    main = "RetentionBudgetAvailabilityConnect",
    init = "connectInit",
    step = "refusalStep",
    max_samples = 1,
    max_steps = 7,
    seed = "9101"
)]
fn retention_budget_availability_connect_refusal() -> impl Driver {
    AvailabilityBridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/RetentionBudgetAvailabilityConnect.qnt",
    main = "RetentionBudgetAvailabilityConnect",
    init = "connectInit",
    step = "sweepStep",
    max_samples = 1,
    max_steps = 8,
    seed = "9102"
)]
fn retention_budget_availability_connect_sweep() -> impl Driver {
    AvailabilityBridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/RetentionBudgetAvailabilityConnect.qnt",
    main = "RetentionBudgetAvailabilityConnect",
    init = "connectInit",
    step = "exhaustedStep",
    max_samples = 1,
    max_steps = 7,
    seed = "9103"
)]
fn retention_budget_availability_connect_exhausted() -> impl Driver {
    AvailabilityBridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/RetentionBudgetAvailabilityConnect.qnt",
    main = "RetentionBudgetAvailabilityConnect",
    init = "connectInit",
    step = "reuseStep",
    max_samples = 1,
    max_steps = 9,
    seed = "9104"
)]
fn retention_budget_availability_connect_reuse() -> impl Driver {
    AvailabilityBridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/RetentionBudgetAvailabilityConnect.qnt",
    main = "RetentionBudgetAvailabilityConnect",
    init = "connectInit",
    step = "retireStep",
    max_samples = 1,
    max_steps = 4,
    seed = "9105"
)]
fn retention_budget_availability_connect_retire() -> impl Driver {
    AvailabilityBridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/RetentionBudgetAvailabilityConnect.qnt",
    main = "RetentionBudgetAvailabilityConnect",
    init = "connectInit",
    step = "identityStep",
    max_samples = 1,
    max_steps = 4,
    seed = "9106"
)]
fn retention_budget_availability_connect_identity_probe() -> impl Driver {
    AvailabilityBridgeDriver::new()
}

#[quint_run(
    spec = "../../verification/quint/RetentionBudgetAvailabilityConnect.qnt",
    main = "RetentionBudgetAvailabilityConnect",
    init = "connectInit",
    step = "occupiedStep",
    max_samples = 1,
    max_steps = 8,
    seed = "9107"
)]
fn retention_budget_availability_connect_occupied() -> impl Driver {
    AvailabilityBridgeDriver::new()
}
