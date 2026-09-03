use dwv_core::{ByteRange, FenceDomain, TopologyEpoch};
use dwv_recovery::{FenceCertificate, IntegrityExtentId, RecoveryGeneration, RegionId};
use dwv_store::{CapabilityEvidenceId, FenceId, StoreId, StoreIncarnationId, StoreWriteWatermark};
use dwv_transaction_ref::{
    ActionResult, CommittedRecoveryGeneration, ComputationResult, ParityComputationPlan,
    ParityRange, PlannedRead, PlannedWrite, RangeGuardToken, SemanticIoResult, Stage,
    StoreWatermark, TransactionMachine, TransactionPersistenceEvidence, TransactionPlan,
};
use itf::Value;
use quint_connect::{Config, Driver, Result, State, Step, quint_run};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum WriteRecoveryRecordState {
    NoWriteRecoveryRecord,
    WriteRecoveryRecordPending,
    WriteRecoveryRecordDurable,
    WriteRecoveryRecordUnknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum DataParityWriteState {
    NoDataParityWrites,
    DataParityWritesAwaitingDurability,
    DataParityWritesDurable,
    DataParityWritesUnknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum RecoveryState {
    RecoveryClean,
    RecoveryDirty,
    RecoveryIndeterminate,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
enum WriteLifecycleState {
    Unowned,
    NormalProcessing,
    InterruptedProcessing,
    CompletedAwaitingRelease,
    AbortedAwaitingRelease,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum WriteRecoveryRecordCommitObservation {
    CommitUnknown,
    CommitRejected,
    CommitDurable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "tag", content = "value")]
#[allow(clippy::enum_variant_names)]
enum DataParityWriteObservation {
    NoDataParityWriteObservation,
    DataParityWriteEffectIndeterminate,
    DataParityWriteEffectDurable,
}

#[derive(Debug, Deserialize)]
struct ModelState {
    #[serde(rename = "writeRecoveryRecord")]
    write_recovery_record: WriteRecoveryRecordState,
    #[serde(rename = "writeRecoveryRecordCommitObservation")]
    write_recovery_record_commit_observation: WriteRecoveryRecordCommitObservation,
    #[serde(rename = "dataParityWriteState")]
    data_parity_write_state: DataParityWriteState,
    #[serde(rename = "dataParityWriteObservation")]
    data_parity_write_observation: DataParityWriteObservation,
    #[serde(rename = "dataParityWriteObservationFinalized")]
    data_parity_write_observation_finalized: bool,
    #[serde(rename = "recoveryState")]
    recovery_state: RecoveryState,
    #[serde(rename = "writeLifecycle")]
    write_lifecycle: WriteLifecycleState,
    #[serde(rename = "integrityClaimsInvalidated")]
    integrity_claims_invalidated: bool,
    #[serde(rename = "writeKnownAppliedRegions")]
    write_known_applied_regions: Vec<String>,
    #[serde(rename = "writeAttemptedRegions")]
    write_attempted_regions: Vec<String>,
    #[serde(rename = "storesWithPersistenceEvidence")]
    stores_with_persistence_evidence: Vec<String>,
    #[serde(rename = "recoveryCleanRegions")]
    recovery_clean_regions: Vec<String>,
    #[serde(rename = "rangeOwned")]
    range_owned: bool,
    #[serde(rename = "releasePending")]
    release_pending: bool,
    #[serde(rename = "rangeReleased")]
    range_released: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct BridgeState {
    write_recovery_record: WriteRecoveryRecordState,
    write_recovery_record_commit_observation: WriteRecoveryRecordCommitObservation,
    data_parity_write_state: DataParityWriteState,
    data_parity_write_observation: DataParityWriteObservation,
    data_parity_write_observation_finalized: bool,
    recovery_state: RecoveryState,
    write_lifecycle: WriteLifecycleState,
    integrity_claims_invalidated: bool,
    write_known_applied_regions: Vec<String>,
    write_attempted_regions: Vec<String>,
    stores_with_persistence_evidence: Vec<String>,
    recovery_clean_regions: Vec<String>,
    range_owned: bool,
    release_pending: bool,
    range_released: bool,
}

impl From<ModelState> for BridgeState {
    fn from(state: ModelState) -> Self {
        Self {
            write_recovery_record: state.write_recovery_record,
            write_recovery_record_commit_observation: state
                .write_recovery_record_commit_observation,
            data_parity_write_state: state.data_parity_write_state,
            data_parity_write_observation: state.data_parity_write_observation,
            data_parity_write_observation_finalized: state.data_parity_write_observation_finalized,
            recovery_state: state.recovery_state,
            write_lifecycle: state.write_lifecycle,
            integrity_claims_invalidated: state.integrity_claims_invalidated,
            write_known_applied_regions: state.write_known_applied_regions,
            write_attempted_regions: state.write_attempted_regions,
            stores_with_persistence_evidence: state.stores_with_persistence_evidence,
            recovery_clean_regions: state.recovery_clean_regions,
            range_owned: state.range_owned,
            release_pending: state.release_pending,
            range_released: state.range_released,
        }
    }
}

impl BridgeState {
    fn initial() -> Self {
        Self {
            write_recovery_record: WriteRecoveryRecordState::NoWriteRecoveryRecord,
            write_recovery_record_commit_observation:
                WriteRecoveryRecordCommitObservation::CommitUnknown,
            data_parity_write_state: DataParityWriteState::NoDataParityWrites,
            data_parity_write_observation: DataParityWriteObservation::NoDataParityWriteObservation,
            data_parity_write_observation_finalized: false,
            recovery_state: RecoveryState::RecoveryClean,
            write_lifecycle: WriteLifecycleState::Unowned,
            integrity_claims_invalidated: false,
            write_known_applied_regions: Vec::new(),
            write_attempted_regions: Vec::new(),
            stores_with_persistence_evidence: Vec::new(),
            recovery_clean_regions: Vec::new(),
            range_owned: false,
            release_pending: false,
            range_released: false,
        }
    }

    fn released() -> Self {
        Self {
            range_released: true,
            ..Self::initial()
        }
    }

    fn from_machine(
        machine: &TransactionMachine,
        data_parity_write_observation_finalized: bool,
    ) -> Self {
        let stage = machine.stage();
        let (write_recovery_record, write_recovery_record_commit_observation) = match stage {
            Stage::AcquireRange => (
                WriteRecoveryRecordState::NoWriteRecoveryRecord,
                WriteRecoveryRecordCommitObservation::CommitUnknown,
            ),
            Stage::WriteRecoveryRecordCommit => (
                WriteRecoveryRecordState::WriteRecoveryRecordPending,
                WriteRecoveryRecordCommitObservation::CommitUnknown,
            ),
            Stage::Aborted => (
                WriteRecoveryRecordState::NoWriteRecoveryRecord,
                WriteRecoveryRecordCommitObservation::CommitRejected,
            ),
            Stage::Completed => (
                WriteRecoveryRecordState::NoWriteRecoveryRecord,
                WriteRecoveryRecordCommitObservation::CommitUnknown,
            ),
            _ => (
                WriteRecoveryRecordState::WriteRecoveryRecordDurable,
                WriteRecoveryRecordCommitObservation::CommitDurable,
            ),
        };
        let (data_parity_write_state, data_parity_write_observation) = match stage {
            Stage::WriteSet | Stage::FlushSet => (
                DataParityWriteState::DataParityWritesAwaitingDurability,
                DataParityWriteObservation::NoDataParityWriteObservation,
            ),
            Stage::RecoveryClean | Stage::Release => (
                DataParityWriteState::DataParityWritesDurable,
                DataParityWriteObservation::DataParityWriteEffectDurable,
            ),
            Stage::AwaitingReconciliation => (
                DataParityWriteState::DataParityWritesUnknown,
                DataParityWriteObservation::DataParityWriteEffectIndeterminate,
            ),
            _ => (
                DataParityWriteState::NoDataParityWrites,
                DataParityWriteObservation::NoDataParityWriteObservation,
            ),
        };
        let (recovery_state, write_lifecycle, release_pending) = match stage {
            Stage::AwaitingReconciliation => (
                RecoveryState::RecoveryIndeterminate,
                WriteLifecycleState::InterruptedProcessing,
                false,
            ),
            Stage::Aborted => (
                RecoveryState::RecoveryClean,
                WriteLifecycleState::AbortedAwaitingRelease,
                true,
            ),
            Stage::Release => (
                RecoveryState::RecoveryClean,
                WriteLifecycleState::CompletedAwaitingRelease,
                true,
            ),
            Stage::Completed => (
                RecoveryState::RecoveryClean,
                WriteLifecycleState::Unowned,
                false,
            ),
            _ => (
                RecoveryState::RecoveryDirty,
                WriteLifecycleState::NormalProcessing,
                false,
            ),
        };
        let data_regions = if matches!(
            stage,
            Stage::WriteSet | Stage::FlushSet | Stage::RecoveryClean | Stage::Release
        ) {
            vec!["data".to_owned()]
        } else {
            Vec::new()
        };
        let recovery_clean_regions = if stage == Stage::Release {
            vec!["data".to_owned()]
        } else {
            Vec::new()
        };
        Self {
            write_recovery_record,
            write_recovery_record_commit_observation,
            data_parity_write_state,
            data_parity_write_observation,
            data_parity_write_observation_finalized,
            recovery_state,
            write_lifecycle,
            integrity_claims_invalidated: machine.state().write_recovery_record_durable
                && stage != Stage::Completed,
            write_known_applied_regions: data_regions.clone(),
            write_attempted_regions: data_regions,
            stores_with_persistence_evidence: Vec::new(),
            recovery_clean_regions,
            range_owned: machine.range_guard().is_some(),
            release_pending,
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
                .map(|machine| {
                    Self::from_machine(machine, driver.data_parity_write_observation_finalized)
                })
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
    data_parity_write_observation_finalized: bool,
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
            startWrite => self.start_write()?,
            confirmWriteRecoveryRecordDurable => self.confirm_write_recovery_record_durable()?,
            attemptDataParityWrite => self.attempt_data_parity_write()?,
            confirmDataParityWritesDurable => self.confirm_data_parity_writes_durable()?,
            commitRecoveryClean => self.commit_recovery_clean()?,
            releaseRange => self.release_range()?,
        })
    }
}

impl BridgeDriver {
    fn init(&mut self) {
        self.machine = None;
        self.released = false;
        self.data_parity_write_observation_finalized = false;
    }

    fn start_write(&mut self) -> Result {
        let mut machine = TransactionMachine::new(plan())?;
        machine.apply(ActionResult::RangeAcquired(RangeGuardToken(1)))?;
        self.machine = Some(machine);
        self.released = false;
        self.data_parity_write_observation_finalized = false;
        Ok(())
    }

    fn confirm_write_recovery_record_durable(&mut self) -> Result {
        self.machine_mut()?
            .apply(ActionResult::WriteRecoveryRecordDurable(
                CommittedRecoveryGeneration::new(RecoveryGeneration(3), TopologyEpoch(7)),
            ))?;
        Ok(())
    }

    fn attempt_data_parity_write(&mut self) -> Result {
        let stage = self.machine_mut()?.stage();
        if stage == Stage::ReadSet {
            let machine = self.machine_mut()?;
            machine.apply(ActionResult::ReadSetComplete(SemanticIoResult::complete()))?;
            machine.apply(ActionResult::ParityComputed(ComputationResult::complete()))?;
            machine.apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))?;
        }
        if stage == Stage::AwaitingReconciliation {
            self.data_parity_write_observation_finalized = false;
        }
        Ok(())
    }

    fn confirm_data_parity_writes_durable(&mut self) -> Result {
        self.machine_mut()?
            .apply(ActionResult::FlushSetComplete(durable_fence()))?;
        self.data_parity_write_observation_finalized = true;
        Ok(())
    }

    fn commit_recovery_clean(&mut self) -> Result {
        self.machine_mut()?
            .apply(ActionResult::RecoveryCleanCommitted(
                CommittedRecoveryGeneration::new(RecoveryGeneration(4), TopologyEpoch(7)),
            ))?;
        Ok(())
    }

    fn release_range(&mut self) -> Result {
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

fn durable_fence() -> TransactionPersistenceEvidence {
    let topology = TopologyEpoch(7);
    let generation = RecoveryGeneration(3);
    let store = StoreId(1);
    TransactionPersistenceEvidence::durable(
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
