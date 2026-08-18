use dwv_core::{ByteRange, FenceDomain, TopologyEpoch};
use dwv_recovery::{FenceCertificate, IntegrityExtentId, RecoveryGeneration, RegionId};
use dwv_store::{CapabilityEvidenceId, FenceId, StoreId, StoreIncarnationId, StoreWriteWatermark};
use dwv_transaction_ref::{
    ActionResult, CommittedRecoveryGeneration, ComputationResult, FenceEvidence,
    ParityComputationPlan, ParityRange, PlannedRead, PlannedWrite, RangeGuardToken,
    SemanticIoResult, Stage, StoreWatermark, TransactionMachine, TransactionPlan,
};
use itf::Value;
use quint_connect::{Config, Driver, Result, State, Step, quint_run};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum Intent {
    NoIntent,
    IntentPending,
    IntentDurable,
    IntentUnknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum Home {
    HomeUnmodified,
    HomeVolatile,
    HomeDurable,
    HomeUnknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum Recovery {
    RecoveryClean,
    RecoveryDirty,
    RecoveryIndeterminate,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum Obligation {
    Unowned,
    InFlight,
    Handoff,
    Terminal,
    Aborted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum IntentObservation {
    IntentCommitUnknown,
    IntentCommitRejected,
    IntentCommitDurable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum HomeObservation {
    HomeEffectUnknown,
    HomeEffectIndeterminate,
    HomeEffectDurable,
}

#[derive(Debug, Deserialize)]
struct ModelState {
    intent: Intent,
    #[serde(rename = "intentObservation")]
    intent_observation: IntentObservation,
    home: Home,
    #[serde(rename = "homeObservation")]
    home_observation: HomeObservation,
    #[serde(rename = "homeReconciled")]
    home_reconciled: bool,
    recovery: Recovery,
    obligation: Obligation,
    invalidated: bool,
    #[serde(rename = "rangeHeld")]
    range_held: bool,
    #[serde(rename = "terminalPendingRelease")]
    terminal_pending_release: bool,
    #[serde(rename = "rangeReleased")]
    range_released: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
struct BridgeState {
    intent: Intent,
    intent_observation: IntentObservation,
    home: Home,
    home_observation: HomeObservation,
    home_reconciled: bool,
    recovery: Recovery,
    obligation: Obligation,
    invalidated: bool,
    range_held: bool,
    terminal_pending_release: bool,
    range_released: bool,
}

impl From<ModelState> for BridgeState {
    fn from(state: ModelState) -> Self {
        Self {
            intent: state.intent,
            intent_observation: state.intent_observation,
            home: state.home,
            home_observation: state.home_observation,
            home_reconciled: state.home_reconciled,
            recovery: state.recovery,
            obligation: state.obligation,
            invalidated: state.invalidated,
            range_held: state.range_held,
            terminal_pending_release: state.terminal_pending_release,
            range_released: state.range_released,
        }
    }
}

impl BridgeState {
    fn initial() -> Self {
        Self {
            intent: Intent::NoIntent,
            intent_observation: IntentObservation::IntentCommitUnknown,
            home_reconciled: false,
            home: Home::HomeUnmodified,
            home_observation: HomeObservation::HomeEffectUnknown,
            recovery: Recovery::RecoveryClean,
            obligation: Obligation::Unowned,
            invalidated: false,
            range_held: false,
            terminal_pending_release: false,
            range_released: false,
        }
    }

    fn released() -> Self {
        Self {
            range_released: true,
            ..Self::initial()
        }
    }

    fn from_machine(machine: &TransactionMachine, home_reconciled: bool) -> Self {
        let stage = machine.stage();
        let (intent, intent_observation) = match stage {
            Stage::AcquireRange => (Intent::NoIntent, IntentObservation::IntentCommitUnknown),
            Stage::IntentCommit => (
                Intent::IntentPending,
                IntentObservation::IntentCommitUnknown,
            ),
            Stage::Aborted | Stage::Completed => {
                (Intent::NoIntent, IntentObservation::IntentCommitUnknown)
            }
            _ => (
                Intent::IntentDurable,
                IntentObservation::IntentCommitDurable,
            ),
        };
        let (home, home_observation) = match stage {
            Stage::WriteSet | Stage::FlushSet => {
                (Home::HomeVolatile, HomeObservation::HomeEffectUnknown)
            }
            Stage::Checkpoint | Stage::Release => {
                (Home::HomeDurable, HomeObservation::HomeEffectDurable)
            }
            Stage::ReconciliationRequired => {
                (Home::HomeUnknown, HomeObservation::HomeEffectIndeterminate)
            }
            _ => (Home::HomeUnmodified, HomeObservation::HomeEffectUnknown),
        };
        let (recovery, obligation, terminal_pending_release) = match stage {
            Stage::ReconciliationRequired => {
                (Recovery::RecoveryIndeterminate, Obligation::Handoff, false)
            }
            Stage::Aborted => (Recovery::RecoveryClean, Obligation::Aborted, true),
            Stage::Release => (Recovery::RecoveryClean, Obligation::Terminal, true),
            Stage::Completed => (Recovery::RecoveryClean, Obligation::Unowned, false),
            _ => (Recovery::RecoveryDirty, Obligation::InFlight, false),
        };
        Self {
            intent,
            intent_observation,
            home,
            home_observation,
            home_reconciled,
            recovery,
            obligation,
            invalidated: machine.state().intent_durable,
            range_held: machine.range_guard().is_some(),
            terminal_pending_release,
            range_released: false,
        }
    }
}

impl State<BridgeDriver> for BridgeState {
    fn from_driver(driver: &BridgeDriver) -> Result<Self> {
        Ok(if driver.released {
            Self::released()
        } else {
            driver
                .machine
                .as_ref()
                .map(|machine| Self::from_machine(machine, driver.home_reconciled))
                .unwrap_or_else(Self::initial)
        })
    }

    fn from_spec(value: Value) -> Result<Self> {
        Ok(ModelState::deserialize(value)?.into())
    }
}

#[derive(Default)]
struct BridgeDriver {
    machine: Option<TransactionMachine>,
    released: bool,
    home_reconciled: bool,
}

impl Driver for BridgeDriver {
    type State = BridgeState;

    fn config() -> Config {
        Config {
            state: &["RecoveryProtocolConnect::RecoveryProtocol::state"],
            ..Config::default()
        }
    }

    fn step(&mut self, step: &Step) -> Result {
        quint_connect::switch!(step {
            init => self.init(),
            begin => self.begin()?,
            acceptIntent => self.accept_intent()?,
            mutate => self.mutate()?,
            makeHomeDurable => self.make_home_durable()?,
            checkpoint => self.checkpoint()?,
            release => self.release()?,
        })
    }
}

impl BridgeDriver {
    fn init(&mut self) {
        self.machine = None;
        self.released = false;
        self.home_reconciled = false;
    }
    fn begin(&mut self) -> Result {
        let mut machine = TransactionMachine::new(plan())?;
        machine.apply(ActionResult::RangeAcquired(RangeGuardToken(1)))?;
        self.machine = Some(machine);
        self.released = false;
        self.home_reconciled = false;
        Ok(())
    }

    fn accept_intent(&mut self) -> Result {
        self.machine_mut()?
            .apply(ActionResult::RecoveryIntentDurable(
                CommittedRecoveryGeneration::new(RecoveryGeneration(3), TopologyEpoch(7)),
            ))?;
        Ok(())
    }
    fn mutate(&mut self) -> Result {
        let stage = self.machine_mut()?.stage();
        if stage == Stage::ReadSet {
            let machine = self.machine_mut()?;
            machine.apply(ActionResult::ReadSetComplete(SemanticIoResult::complete()))?;
            machine.apply(ActionResult::ParityComputed(ComputationResult::complete()))?;
            machine.apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))?;
        }
        if stage == Stage::ReconciliationRequired {
            self.home_reconciled = false;
        }
        Ok(())
    }

    fn make_home_durable(&mut self) -> Result {
        self.machine_mut()?
            .apply(ActionResult::FlushSetComplete(durable_fence()))?;
        self.home_reconciled = true;
        Ok(())
    }

    fn checkpoint(&mut self) -> Result {
        self.machine_mut()?
            .apply(ActionResult::CheckpointCommitted(
                CommittedRecoveryGeneration::new(RecoveryGeneration(4), TopologyEpoch(7)),
            ))?;
        Ok(())
    }

    fn release(&mut self) -> Result {
        let machine = self.machine_mut()?;
        machine.apply(ActionResult::RangeReleased)?;
        assert!(
            machine.apply(ActionResult::RangeReleased).is_err(),
            "dwv-transaction-ref accepted a stale release result"
        );
        self.released = true;
        Ok(())
    }

    fn machine_mut(&mut self) -> Result<&mut TransactionMachine> {
        self.machine
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("bridge action has no transaction machine"))
    }
}

fn plan() -> TransactionPlan {
    let topology = TopologyEpoch(7);
    let generation = RecoveryGeneration(3);
    let region = RegionId(1);
    let store = StoreId(1);
    let range = ByteRange::new(0, 4096).expect("non-empty bridge range");
    TransactionPlan::new(topology, generation, FenceDomain(9))
        .with_ranges(vec![ParityRange::new(region, store, range)])
        .with_dirty_regions(vec![region])
        .with_checksum_extents(vec![IntegrityExtentId(4)])
        .with_reads(vec![PlannedRead::new(store, range)])
        .with_parity(ParityComputationPlan::new(topology, range, 2))
        .with_writes(vec![PlannedWrite::new(store, range)])
        .with_stores(vec![store])
        .with_watermarks(vec![StoreWatermark::new(store, StoreWriteWatermark(8))])
}

fn durable_fence() -> FenceEvidence {
    let topology = TopologyEpoch(7);
    let generation = RecoveryGeneration(3);
    let store = StoreId(1);
    FenceEvidence::durable(
        FenceCertificate::new(
            topology,
            FenceDomain(9),
            vec![dwv_store::StoreFenceRef {
                fence_id: FenceId(1),
                store_id: store,
                store_incarnation: StoreIncarnationId(0),
                topology_epoch: topology,
                through: StoreWriteWatermark(8),
                capability_evidence_id: CapabilityEvidenceId(1),
            }],
            vec![(RegionId(1), generation)],
        )
        .with_integrity_extent(IntegrityExtentId(4), generation),
    )
}

#[quint_run(
    spec = "../../verification/quint/RecoveryProtocolConnect.qnt",
    main = "RecoveryProtocolConnect",
    step = "connectStep",
    max_samples = 1,
    max_steps = 12,
    seed = "22082026"
)]
fn quint_connect_bridge() -> impl Driver {
    BridgeDriver::default()
}

#[quint_run(
    spec = "../../verification/quint/RecoveryProtocolConnect.qnt",
    main = "RecoveryProtocolConnect",
    step = "connectStep",
    max_samples = 1,
    max_steps = 12,
    seed = "1"
)]
fn quint_connect_bridge_second_seed() -> impl Driver {
    BridgeDriver::default()
}
