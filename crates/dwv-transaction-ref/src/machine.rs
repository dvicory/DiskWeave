use crate::action::{
    ActionKind, ActionResult, CommittedRecoveryGeneration, ComputationResult, IntentRequirement,
    ParityComputationPlan, ParityRange, PlannedRead, PlannedWrite, RangeGuardToken, ResultKind,
    SemanticFailure, SemanticIoResult, StoreWatermarks, TransactionAction,
};
use crate::error::{ErrorClass, PlanError, TransactionError};
use crate::trace::Trace;
use dwv_core::{ByteRange, FenceDomain, TopologyEpoch};
use dwv_recovery::{FenceCertificate, IntegrityExtentId, RecoveryGeneration, RegionId};
use dwv_store::StoreId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransactionLimits {
    pub max_ranges: usize,
    pub max_reads: usize,
    pub max_writes: usize,
    pub max_stores: usize,
    pub max_extents: usize,
    pub max_trace_events: usize,
}

impl Default for TransactionLimits {
    fn default() -> Self {
        Self {
            max_ranges: 64,
            max_reads: 128,
            max_writes: 128,
            max_stores: 32,
            max_extents: 128,
            max_trace_events: 1024,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionPlan {
    pub topology_epoch: TopologyEpoch,
    pub recovery_generation: RecoveryGeneration,
    pub fence_domain: FenceDomain,
    pub intent: IntentRequirement,
    pub ranges: Vec<ParityRange>,
    pub dirty_regions: Vec<RegionId>,
    pub checksum_extents: Vec<IntegrityExtentId>,
    pub reads: Vec<PlannedRead>,
    pub parity: ParityComputationPlan,
    pub writes: Vec<PlannedWrite>,
    pub stores: Vec<StoreId>,
    pub through: StoreWatermarks,
    pub limits: TransactionLimits,
}

impl TransactionPlan {
    pub fn new(
        topology_epoch: TopologyEpoch,
        recovery_generation: RecoveryGeneration,
        fence_domain: FenceDomain,
    ) -> Self {
        Self {
            topology_epoch,
            recovery_generation,
            fence_domain,
            intent: IntentRequirement::FirstWrite,
            ranges: Vec::new(),
            dirty_regions: Vec::new(),
            checksum_extents: Vec::new(),
            reads: Vec::new(),
            parity: ParityComputationPlan::new(topology_epoch, ByteRange::empty(), 0),
            writes: Vec::new(),
            stores: Vec::new(),
            through: Vec::new(),
            limits: TransactionLimits::default(),
        }
    }

    pub fn with_limits(mut self, limits: TransactionLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_intent(mut self, intent: IntentRequirement) -> Self {
        self.intent = intent;
        self
    }

    pub fn with_ranges(mut self, ranges: Vec<ParityRange>) -> Self {
        self.ranges = ranges;
        self
    }

    pub fn with_dirty_regions(mut self, regions: Vec<RegionId>) -> Self {
        self.dirty_regions = regions;
        self
    }

    pub fn with_checksum_extents(mut self, extents: Vec<IntegrityExtentId>) -> Self {
        self.checksum_extents = extents;
        self
    }

    pub fn with_reads(mut self, reads: Vec<PlannedRead>) -> Self {
        self.reads = reads;
        self
    }

    pub fn with_parity(mut self, parity: ParityComputationPlan) -> Self {
        self.parity = parity;
        self
    }

    pub fn with_writes(mut self, writes: Vec<PlannedWrite>) -> Self {
        self.writes = writes;
        self
    }

    pub fn with_stores(mut self, stores: Vec<StoreId>) -> Self {
        self.stores = stores;
        self
    }

    pub fn with_watermarks(mut self, through: StoreWatermarks) -> Self {
        self.through = through;
        self
    }

    pub fn captured_intent_generation(&self) -> RecoveryGeneration {
        self.intent.generation(self.recovery_generation)
    }

    pub fn validate(&self) -> Result<(), PlanError> {
        if self.ranges.is_empty() {
            return Err(PlanError::NoRanges);
        }
        if self.stores.is_empty() {
            return Err(PlanError::NoStores);
        }
        if self.reads.is_empty() {
            return Err(PlanError::NoReads);
        }
        if self.writes.is_empty() {
            return Err(PlanError::NoWrites);
        }
        if self.ranges.len() > self.limits.max_ranges {
            return Err(PlanError::RangeCountExceeded {
                actual: self.ranges.len(),
                maximum: self.limits.max_ranges,
            });
        }
        if self.reads.len() > self.limits.max_reads {
            return Err(PlanError::ReadCountExceeded {
                actual: self.reads.len(),
                maximum: self.limits.max_reads,
            });
        }
        if self.writes.len() > self.limits.max_writes {
            return Err(PlanError::WriteCountExceeded {
                actual: self.writes.len(),
                maximum: self.limits.max_writes,
            });
        }
        if self.stores.len() > self.limits.max_stores {
            return Err(PlanError::StoreCountExceeded {
                actual: self.stores.len(),
                maximum: self.limits.max_stores,
            });
        }
        if self.checksum_extents.len() > self.limits.max_extents {
            return Err(PlanError::ExtentCountExceeded {
                actual: self.checksum_extents.len(),
                maximum: self.limits.max_extents,
            });
        }
        if self.limits.max_trace_events < 2 {
            return Err(PlanError::TraceLimitTooSmall);
        }
        if self.ranges.iter().any(|range| range.range.is_empty())
            || self.reads.iter().any(|read| read.range.is_empty())
            || self.writes.iter().any(|write| write.range.is_empty())
        {
            return Err(PlanError::EmptyRange);
        }
        for (index, store) in self.stores.iter().enumerate() {
            if self.stores[..index].contains(store) {
                return Err(PlanError::DuplicateStore);
            }
            if !self
                .through
                .iter()
                .any(|watermark| watermark.store == *store)
            {
                return Err(PlanError::MissingWatermark);
            }
        }
        if self
            .ranges
            .iter()
            .any(|range| !self.stores.contains(&range.store))
            || self
                .reads
                .iter()
                .any(|read| !self.stores.contains(&read.store))
            || self
                .writes
                .iter()
                .any(|write| !self.stores.contains(&write.store))
        {
            return Err(PlanError::MissingStore);
        }
        if self.parity.topology_epoch != self.topology_epoch {
            return Err(PlanError::TopologyGenerationOverflow);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Stage {
    AcquireRange,
    IntentCommit,
    ReadSet,
    ComputeParity,
    WriteSet,
    FlushSet,
    Checkpoint,
    Release,
    Completed,
    Aborted,
    ReconciliationRequired,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TerminalDisposition {
    Completed,
    Aborted,
    ReconciliationRequired,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Disposition {
    InProgress,
    Completed,
    Abandoned,
    Aborted,
    ReconciliationRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionState {
    pub stage: Stage,
    pub disposition: Disposition,
    pub frontend_abandoned: bool,
    pub irreversible_boundary: bool,
    pub intent_durable: bool,
    pub home_mutation_emitted: bool,
    pub captured_topology_epoch: TopologyEpoch,
    pub captured_recovery_generation: RecoveryGeneration,
    pub last_failure: Option<SemanticFailure>,
}

pub struct TransactionMachine {
    plan: TransactionPlan,
    state: TransactionState,
    pending: Option<TransactionAction>,
    range_guard: Option<RangeGuardToken>,
    trace: Trace,
    next_sequence: u64,
    crashed: bool,
    last_fence: Option<FenceCertificate>,
}

impl TransactionMachine {
    pub fn new(plan: TransactionPlan) -> Result<Self, TransactionError> {
        plan.validate().map_err(TransactionError::InvalidPlan)?;
        let captured_recovery_generation = plan.captured_intent_generation();
        let state = TransactionState {
            stage: Stage::AcquireRange,
            disposition: Disposition::InProgress,
            frontend_abandoned: false,
            irreversible_boundary: false,
            intent_durable: !plan.intent.requires_commit(),
            home_mutation_emitted: false,
            captured_topology_epoch: plan.topology_epoch,
            captured_recovery_generation,
            last_failure: None,
        };
        let mut machine = Self {
            trace: Trace::new(plan.limits.max_trace_events),
            plan,
            state,
            pending: None,
            range_guard: None,
            next_sequence: 0,
            crashed: false,
            last_fence: None,
        };
        machine.emit(TransactionAction::AcquireRange {
            ranges: machine.plan.ranges.clone(),
        })?;
        Ok(machine)
    }

    pub fn replay(
        plan: TransactionPlan,
        results: &[ActionResult],
    ) -> Result<Self, TransactionError> {
        let mut machine = Self::new(plan)?;
        for result in results {
            machine.apply(result.clone())?;
        }
        Ok(machine)
    }

    pub fn plan(&self) -> &TransactionPlan {
        &self.plan
    }

    pub fn state(&self) -> &TransactionState {
        &self.state
    }

    pub fn stage(&self) -> Stage {
        self.state.stage
    }

    pub fn disposition(&self) -> Disposition {
        self.state.disposition
    }

    pub fn terminal_disposition(&self) -> Option<TerminalDisposition> {
        match self.state.stage {
            Stage::Completed => Some(TerminalDisposition::Completed),
            Stage::Aborted => Some(TerminalDisposition::Aborted),
            Stage::ReconciliationRequired => Some(TerminalDisposition::ReconciliationRequired),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal_disposition().is_some()
    }

    pub fn pending_action(&self) -> Option<&TransactionAction> {
        self.pending.as_ref()
    }

    pub fn next_action(&self) -> Option<&TransactionAction> {
        self.pending_action()
    }

    pub fn range_guard(&self) -> Option<RangeGuardToken> {
        self.range_guard
    }

    pub fn trace(&self) -> &Trace {
        &self.trace
    }

    pub fn abandon(&mut self) -> Result<(), TransactionError> {
        if self.state.frontend_abandoned {
            return Err(TransactionError::DuplicateAbandonment);
        }
        if self.is_terminal() {
            return Err(TransactionError::AlreadyTerminal);
        }
        self.state.frontend_abandoned = true;
        self.trace.record_abandoned(self.next_sequence)?;
        self.next_sequence = self.next_sequence.saturating_add(1);
        Ok(())
    }

    pub fn daemon_crash(&mut self) -> Result<(), TransactionError> {
        if self.crashed {
            return Err(TransactionError::DaemonAlreadyCrashed);
        }
        if self.is_terminal() {
            return Err(TransactionError::AlreadyTerminal);
        }
        self.crashed = true;
        self.trace.record_daemon_crash(self.next_sequence)?;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.pending = None;
        if self.state.stage == Stage::AcquireRange && !self.state.intent_durable {
            self.transition(Stage::Aborted, None)?;
        } else {
            self.transition(
                Stage::ReconciliationRequired,
                Some(SemanticFailure::new(ErrorClass::DaemonCrash)),
            )?;
        }
        Ok(())
    }

    pub fn apply(&mut self, result: ActionResult) -> Result<(), TransactionError> {
        let action = self
            .pending
            .as_ref()
            .ok_or(TransactionError::NoPendingAction)?;
        let expected = action.kind();
        let received = result.kind();
        if received != ResultKind::Failed && !result_matches(expected, received) {
            return Err(TransactionError::UnexpectedResult { expected, received });
        }

        let pre_stage = self.state.stage;
        let action = self.pending.take().expect("pending action checked above");
        let transition = self.apply_result(action.kind(), result)?;
        self.trace.record_result(
            self.next_sequence,
            action.kind(),
            received,
            pre_stage,
            self.state.stage,
        )?;
        self.next_sequence = self.next_sequence.saturating_add(1);
        if let Some(next_stage) = transition {
            self.transition(next_stage, None)?;
            self.emit_for_stage(next_stage)?;
        }
        Ok(())
    }

    fn apply_result(
        &mut self,
        action: ActionKind,
        result: ActionResult,
    ) -> Result<Option<Stage>, TransactionError> {
        if let ActionResult::Failed(failure) = result {
            let next = if action == ActionKind::AcquireRange && !self.state.intent_durable {
                Stage::Aborted
            } else {
                Stage::ReconciliationRequired
            };
            self.transition(next, Some(failure))?;
            return Ok(None);
        }

        match (action, result) {
            (ActionKind::AcquireRange, ActionResult::RangeAcquired(token)) => {
                self.range_guard = Some(token);
                Ok(Some(if self.plan.intent.requires_commit() {
                    Stage::IntentCommit
                } else {
                    Stage::ReadSet
                }))
            }
            (
                ActionKind::PersistDirtyAndInvalidateIntegrity,
                ActionResult::RecoveryIntentDurable(committed),
            ) => {
                self.require_generation(committed)?;
                self.state.intent_durable = true;
                self.state.irreversible_boundary = true;
                Ok(Some(Stage::ReadSet))
            }
            (ActionKind::ReadSet, ActionResult::ReadSetComplete(result)) => match result {
                SemanticIoResult::Complete => Ok(Some(Stage::ComputeParity)),
                SemanticIoResult::Failed => {
                    self.reconcile(ErrorClass::ReadFailed)?;
                    Ok(None)
                }
                SemanticIoResult::Uncertain => {
                    self.reconcile(ErrorClass::ReadUncertain)?;
                    Ok(None)
                }
            },
            (ActionKind::ComputeParity, ActionResult::ParityComputed(result)) => match result {
                ComputationResult::Complete => Ok(Some(Stage::WriteSet)),
                ComputationResult::Failed => {
                    self.reconcile(ErrorClass::ParityComputationFailed)?;
                    Ok(None)
                }
            },
            (ActionKind::WriteSet, ActionResult::WriteSetComplete(result)) => match result {
                SemanticIoResult::Complete => {
                    self.state.irreversible_boundary = true;
                    self.state.home_mutation_emitted = true;
                    Ok(Some(Stage::FlushSet))
                }
                SemanticIoResult::Failed => {
                    self.state.home_mutation_emitted = true;
                    self.reconcile(ErrorClass::WriteFailed)?;
                    Ok(None)
                }
                SemanticIoResult::Uncertain => {
                    self.state.home_mutation_emitted = true;
                    self.reconcile(ErrorClass::WriteUncertain)?;
                    Ok(None)
                }
            },
            (ActionKind::FlushSet, ActionResult::FlushSetComplete(evidence)) => {
                if evidence.covers(
                    self.plan.topology_epoch,
                    self.plan.fence_domain,
                    &self.plan.through,
                    &self.plan.dirty_regions,
                    &self.plan.checksum_extents,
                    self.state.captured_recovery_generation,
                ) {
                    self.last_fence = Some(evidence.certificate.clone());
                    Ok(Some(Stage::Checkpoint))
                } else {
                    self.reconcile(if evidence.uncertain {
                        ErrorClass::FenceUncertain
                    } else {
                        ErrorClass::FenceIncomplete
                    })?;
                    Ok(None)
                }
            }
            (ActionKind::CommitCheckpointOrClear, ActionResult::CheckpointCommitted(committed)) => {
                self.require_generation_at_least(committed)?;
                Ok(Some(Stage::Release))
            }
            (ActionKind::ReleaseRange, ActionResult::RangeReleased) => {
                self.range_guard = None;
                self.state.disposition = if self.state.frontend_abandoned {
                    Disposition::Abandoned
                } else {
                    Disposition::Completed
                };
                Ok(Some(Stage::Completed))
            }
            _ => Err(TransactionError::InvalidResult(
                "result did not match action",
            )),
        }
    }

    fn emit_for_stage(&mut self, stage: Stage) -> Result<(), TransactionError> {
        match stage {
            Stage::IntentCommit => {
                self.emit(TransactionAction::PersistDirtyAndInvalidateIntegrity {
                    dirty_regions: self.plan.dirty_regions.clone(),
                    checksum_extents: self.plan.checksum_extents.clone(),
                })
            }
            Stage::ReadSet => self.emit(TransactionAction::ReadSet {
                reads: self.plan.reads.clone(),
            }),
            Stage::ComputeParity => self.emit(TransactionAction::ComputeParity {
                plan: self.plan.parity,
            }),
            Stage::WriteSet => {
                // Emitting a semantic write is the irreversible boundary: the
                // executor may have accepted child I/O even if its result is
                // later lost or uncertain.
                self.state.irreversible_boundary = true;
                self.state.home_mutation_emitted = true;
                self.emit(TransactionAction::WriteSet {
                    writes: self.plan.writes.clone(),
                })
            }
            Stage::FlushSet => self.emit(TransactionAction::FlushSet {
                stores: self.plan.stores.clone(),
                through: self.plan.through.clone(),
            }),
            Stage::Checkpoint => self.emit(TransactionAction::CommitCheckpointOrClear {
                certificate: self.checkpoint_certificate(),
            }),
            Stage::Release => self.emit(TransactionAction::ReleaseRange),
            Stage::AcquireRange
            | Stage::Completed
            | Stage::Aborted
            | Stage::ReconciliationRequired => Ok(()),
        }
    }

    fn checkpoint_certificate(&self) -> FenceCertificate {
        self.last_fence.clone().unwrap_or_else(|| {
            FenceCertificate::new(
                self.plan.topology_epoch,
                self.plan.fence_domain,
                Vec::new(),
                self.plan
                    .dirty_regions
                    .iter()
                    .map(|region| (*region, self.state.captured_recovery_generation))
                    .collect(),
            )
        })
    }

    fn emit(&mut self, action: TransactionAction) -> Result<(), TransactionError> {
        if self.trace.event_count() >= self.plan.limits.max_trace_events {
            return Err(TransactionError::TraceLimitExceeded);
        }
        self.trace.record_action(
            self.next_sequence,
            self.state.stage,
            action.kind(),
            self.state.captured_topology_epoch,
            self.state.captured_recovery_generation,
        )?;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.pending = Some(action);
        Ok(())
    }

    fn transition(
        &mut self,
        stage: Stage,
        failure: Option<SemanticFailure>,
    ) -> Result<(), TransactionError> {
        self.state.stage = stage;
        self.state.last_failure = failure;
        self.state.disposition = match stage {
            Stage::Completed => {
                if self.state.frontend_abandoned {
                    Disposition::Abandoned
                } else {
                    Disposition::Completed
                }
            }
            Stage::Aborted => Disposition::Aborted,
            Stage::ReconciliationRequired => Disposition::ReconciliationRequired,
            _ => Disposition::InProgress,
        };
        if stage == Stage::ReconciliationRequired {
            self.trace.record_reconciliation_required(
                self.next_sequence,
                failure.map(|value| value.class),
            )?;
            self.next_sequence = self.next_sequence.saturating_add(1);
        }
        Ok(())
    }

    fn reconcile(&mut self, class: ErrorClass) -> Result<(), TransactionError> {
        self.pending = None;
        self.transition(
            Stage::ReconciliationRequired,
            Some(SemanticFailure::new(class)),
        )
    }

    fn require_generation(
        &self,
        committed: CommittedRecoveryGeneration,
    ) -> Result<(), TransactionError> {
        if committed.topology_epoch != self.plan.topology_epoch {
            return Err(TransactionError::TopologyMismatch {
                expected: self.plan.topology_epoch,
                actual: committed.topology_epoch,
            });
        }
        let expected = self.state.captured_recovery_generation;
        if committed.generation != expected {
            return Err(TransactionError::RecoveryGenerationMismatch {
                expected,
                actual: committed.generation,
            });
        }
        Ok(())
    }

    fn require_generation_at_least(
        &self,
        committed: CommittedRecoveryGeneration,
    ) -> Result<(), TransactionError> {
        if committed.topology_epoch != self.plan.topology_epoch {
            return Err(TransactionError::TopologyMismatch {
                expected: self.plan.topology_epoch,
                actual: committed.topology_epoch,
            });
        }
        if committed.generation.0 < self.state.captured_recovery_generation.0 {
            return Err(TransactionError::RecoveryGenerationMismatch {
                expected: self.state.captured_recovery_generation,
                actual: committed.generation,
            });
        }
        Ok(())
    }
}

fn result_matches(action: ActionKind, result: ResultKind) -> bool {
    matches!(
        (action, result),
        (ActionKind::AcquireRange, ResultKind::RangeAcquired)
            | (
                ActionKind::PersistDirtyAndInvalidateIntegrity,
                ResultKind::RecoveryIntentDurable
            )
            | (ActionKind::ReadSet, ResultKind::ReadSetComplete)
            | (ActionKind::ComputeParity, ResultKind::ParityComputed)
            | (ActionKind::WriteSet, ResultKind::WriteSetComplete)
            | (ActionKind::FlushSet, ResultKind::FlushSetComplete)
            | (
                ActionKind::CommitCheckpointOrClear,
                ResultKind::CheckpointCommitted
            )
            | (ActionKind::ReleaseRange, ResultKind::RangeReleased)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionResult, ErrorClass, FenceEvidence, StoreWatermark};
    use dwv_core::{ByteRange, FenceDomain, TopologyEpoch};
    use dwv_recovery::{FenceCertificate, IntegrityExtentId, RecoveryGeneration, RegionId};
    use dwv_store::{CapabilityEvidenceId, FenceId, StoreFenceRef, StoreId, StoreWriteWatermark};

    fn plan() -> TransactionPlan {
        let topology = TopologyEpoch(7);
        let region = RegionId(1);
        let store = StoreId(1);
        let range = ByteRange::new(0, 4096).unwrap();
        TransactionPlan::new(topology, RecoveryGeneration(3), FenceDomain(9))
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
        let plan = plan();
        let store = StoreId(1);
        let certificate = FenceCertificate::new(
            TopologyEpoch(7),
            FenceDomain(9),
            vec![StoreFenceRef {
                fence_id: FenceId(1),
                store_id: store,
                topology_epoch: TopologyEpoch(7),
                through: StoreWriteWatermark(8),
                capability_evidence_id: CapabilityEvidenceId(1),
            }],
            vec![(RegionId(1), RecoveryGeneration(3))],
        )
        .with_integrity_extent(IntegrityExtentId(4), RecoveryGeneration(3));
        let evidence = FenceEvidence::durable(certificate);
        assert!(evidence.covers(
            plan.topology_epoch,
            plan.fence_domain,
            &plan.through,
            &plan.dirty_regions,
            &plan.checksum_extents,
            RecoveryGeneration(3),
        ));
        evidence
    }

    fn drive_to_write(machine: &mut TransactionMachine) {
        machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .unwrap();
        machine
            .apply(ActionResult::RecoveryIntentDurable(
                CommittedRecoveryGeneration::new(RecoveryGeneration(3), TopologyEpoch(7)),
            ))
            .unwrap();
        machine
            .apply(ActionResult::ReadSetComplete(SemanticIoResult::complete()))
            .unwrap();
        machine
            .apply(ActionResult::ParityComputed(ComputationResult::complete()))
            .unwrap();
    }

    #[test]
    fn legal_first_write_orders_intent_before_home_write() {
        let mut machine = TransactionMachine::new(plan()).unwrap();
        assert_eq!(
            machine.pending_action().unwrap().kind(),
            ActionKind::AcquireRange
        );
        machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .unwrap();
        assert_eq!(
            machine.pending_action().unwrap().kind(),
            ActionKind::PersistDirtyAndInvalidateIntegrity
        );
        machine
            .apply(ActionResult::RecoveryIntentDurable(
                CommittedRecoveryGeneration::new(RecoveryGeneration(3), TopologyEpoch(7)),
            ))
            .unwrap();
        assert!(machine.state().intent_durable);
        assert_eq!(
            machine.pending_action().unwrap().kind(),
            ActionKind::ReadSet
        );
    }

    #[test]
    fn failed_intent_never_emits_a_write() {
        let mut machine = TransactionMachine::new(plan()).unwrap();
        machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .unwrap();
        machine
            .apply(ActionResult::failure(ErrorClass::RecoveryIntentRejected))
            .unwrap();
        assert_eq!(machine.stage(), Stage::ReconciliationRequired);
        assert!(machine.pending_action().is_none());
        assert!(!machine.state().home_mutation_emitted);
    }

    #[test]
    fn incomplete_fence_cannot_checkpoint_or_release() {
        let mut machine = TransactionMachine::new(plan()).unwrap();
        drive_to_write(&mut machine);
        machine
            .apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))
            .unwrap();
        let incomplete = FenceEvidence::volatile(durable_fence().certificate);
        machine
            .apply(ActionResult::FlushSetComplete(incomplete))
            .unwrap();
        assert_eq!(machine.stage(), Stage::ReconciliationRequired);
        assert!(machine.pending_action().is_none());
    }

    #[test]
    fn complete_path_releases_only_after_checkpoint() {
        let mut machine = TransactionMachine::new(plan()).unwrap();
        drive_to_write(&mut machine);
        machine
            .apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))
            .unwrap();
        machine
            .apply(ActionResult::FlushSetComplete(durable_fence()))
            .unwrap();
        assert_eq!(
            machine.pending_action().unwrap().kind(),
            ActionKind::CommitCheckpointOrClear
        );
        machine
            .apply(ActionResult::CheckpointCommitted(
                CommittedRecoveryGeneration::new(RecoveryGeneration(4), TopologyEpoch(7)),
            ))
            .unwrap();
        assert_eq!(
            machine.pending_action().unwrap().kind(),
            ActionKind::ReleaseRange
        );
        machine.apply(ActionResult::RangeReleased).unwrap();
        assert_eq!(machine.stage(), Stage::Completed);
        assert_eq!(machine.disposition(), Disposition::Completed);
    }

    #[test]
    fn duplicate_or_out_of_order_result_does_not_mutate_state() {
        let mut machine = TransactionMachine::new(plan()).unwrap();
        let before = machine.state().clone();
        let error = machine
            .apply(ActionResult::ReadSetComplete(SemanticIoResult::complete()))
            .unwrap_err();
        assert!(matches!(error, TransactionError::UnexpectedResult { .. }));
        assert_eq!(*machine.state(), before);
        machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .unwrap();
        let before = machine.state().clone();
        let error = machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .unwrap_err();
        assert!(matches!(error, TransactionError::UnexpectedResult { .. }));
        assert_eq!(*machine.state(), before);
    }

    #[test]
    fn abandonment_does_not_cancel_irreversible_work() {
        let mut machine = TransactionMachine::new(plan()).unwrap();
        machine.abandon().unwrap();
        drive_to_write(&mut machine);
        machine
            .apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))
            .unwrap();
        machine
            .apply(ActionResult::FlushSetComplete(durable_fence()))
            .unwrap();
        machine
            .apply(ActionResult::CheckpointCommitted(
                CommittedRecoveryGeneration::new(RecoveryGeneration(4), TopologyEpoch(7)),
            ))
            .unwrap();
        machine.apply(ActionResult::RangeReleased).unwrap();
        assert_eq!(machine.disposition(), Disposition::Abandoned);
    }

    #[test]
    fn daemon_crash_after_write_requires_reconciliation() {
        let mut machine = TransactionMachine::new(plan()).unwrap();
        drive_to_write(&mut machine);
        machine.daemon_crash().unwrap();
        assert_eq!(machine.stage(), Stage::ReconciliationRequired);
        assert!(machine.state().home_mutation_emitted);
    }

    #[test]
    fn uncertain_results_are_conservative_at_each_mutating_boundary() {
        let mut intent = TransactionMachine::new(plan()).unwrap();
        intent
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .unwrap();
        intent
            .apply(ActionResult::failure(ErrorClass::RecoveryIntentLost))
            .unwrap();
        assert_eq!(intent.stage(), Stage::ReconciliationRequired);

        let mut read = TransactionMachine::new(plan()).unwrap();
        drive_to_write(&mut read);
        read.apply(ActionResult::WriteSetComplete(SemanticIoResult::uncertain()))
            .unwrap();
        assert_eq!(read.stage(), Stage::ReconciliationRequired);

        let mut fence = TransactionMachine::new(plan()).unwrap();
        drive_to_write(&mut fence);
        fence
            .apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))
            .unwrap();
        fence
            .apply(ActionResult::FlushSetComplete(FenceEvidence::uncertain(
                durable_fence().certificate,
            )))
            .unwrap();
        assert_eq!(fence.stage(), Stage::ReconciliationRequired);
    }

    #[test]
    fn failed_checkpoint_never_releases() {
        let mut machine = TransactionMachine::new(plan()).unwrap();
        drive_to_write(&mut machine);
        machine
            .apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))
            .unwrap();
        machine
            .apply(ActionResult::FlushSetComplete(durable_fence()))
            .unwrap();
        machine
            .apply(ActionResult::failure(ErrorClass::CheckpointFailed))
            .unwrap();
        assert_eq!(machine.stage(), Stage::ReconciliationRequired);
        assert!(machine.pending_action().is_none());
    }

    #[test]
    fn crash_with_unknown_intent_is_not_optimistically_aborted() {
        let mut machine = TransactionMachine::new(plan()).unwrap();
        machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .unwrap();
        machine.daemon_crash().unwrap();
        assert_eq!(machine.stage(), Stage::ReconciliationRequired);
        assert!(!machine.state().home_mutation_emitted);
    }

    #[test]
    fn topology_and_generation_evidence_are_checked() {
        let mut machine = TransactionMachine::new(plan()).unwrap();
        machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .unwrap();
        let before = machine.state().clone();
        let error = machine
            .apply(ActionResult::RecoveryIntentDurable(
                CommittedRecoveryGeneration::new(RecoveryGeneration(4), TopologyEpoch(7)),
            ))
            .unwrap_err();
        assert!(matches!(
            error,
            TransactionError::RecoveryGenerationMismatch { .. }
        ));
        assert_eq!(*machine.state(), before);
    }

    #[test]
    fn simulator_flush_evidence_composes_with_the_reference_boundary() {
        use dwv_sim::{Schedule, ScheduleStep, Simulator, SimulatorConfig};
        use dwv_store::{ChildOperationId, OperationSlotToken, WriteIntent};

        let operation_slot = OperationSlotToken::new(0, 1);
        let mut simulator_config = SimulatorConfig::for_size(4096);
        simulator_config.topology_epoch = TopologyEpoch(7);
        let mut simulator = Simulator::new(vec![0; 4096], simulator_config).unwrap();
        let schedule = Schedule::new(vec![
            ScheduleStep::SubmitWrite {
                operation_id: ChildOperationId {
                    slot: operation_slot,
                    index: 0,
                },
                offset: 0,
                bytes: vec![7; 4096],
                intent: WriteIntent::Ordinary,
            },
            ScheduleStep::Deliver { pending_index: 0 },
            ScheduleStep::SubmitFlush {
                operation_id: ChildOperationId {
                    slot: operation_slot,
                    index: 1,
                },
                through: StoreWriteWatermark(1),
            },
            ScheduleStep::Deliver { pending_index: 0 },
        ]);
        let simulation = simulator.run(&schedule).unwrap();
        let fence = match simulation.deliveries[1].completion.persistence {
            dwv_store::PersistenceEvidence::DurableByFence { fence } => fence,
            evidence => panic!("simulator did not produce fence evidence: {evidence:?}"),
        };
        let mut certificate = FenceCertificate::new(
            TopologyEpoch(7),
            FenceDomain(9),
            vec![fence],
            vec![(RegionId(1), RecoveryGeneration(3))],
        );
        certificate =
            certificate.with_integrity_extent(IntegrityExtentId(4), RecoveryGeneration(3));

        let mut machine =
            TransactionMachine::new(plan().with_watermarks(vec![StoreWatermark::new(
                StoreId(1),
                StoreWriteWatermark(1),
            )]))
            .unwrap();
        drive_to_write(&mut machine);
        machine
            .apply(ActionResult::WriteSetComplete(SemanticIoResult::complete()))
            .unwrap();
        machine
            .apply(ActionResult::FlushSetComplete(FenceEvidence::durable(
                certificate,
            )))
            .unwrap();
        assert_eq!(machine.stage(), Stage::Checkpoint);
        assert!(
            simulation
                .final_snapshot
                .volatile_acknowledged_writes
                .is_empty()
        );
    }

    #[test]
    fn already_dirty_skips_redundant_intent_action() {
        let mut machine =
            TransactionMachine::new(plan().with_intent(IntentRequirement::AlreadyDirty {
                durable_generation: RecoveryGeneration(3),
            }))
            .unwrap();
        machine
            .apply(ActionResult::RangeAcquired(RangeGuardToken(1)))
            .unwrap();
        assert_eq!(
            machine.pending_action().unwrap().kind(),
            ActionKind::ReadSet
        );
    }

    #[test]
    fn replay_is_deterministic() {
        let results = vec![
            ActionResult::RangeAcquired(RangeGuardToken(1)),
            ActionResult::RecoveryIntentDurable(CommittedRecoveryGeneration::new(
                RecoveryGeneration(3),
                TopologyEpoch(7),
            )),
            ActionResult::ReadSetComplete(SemanticIoResult::complete()),
            ActionResult::ParityComputed(ComputationResult::complete()),
            ActionResult::WriteSetComplete(SemanticIoResult::complete()),
            ActionResult::FlushSetComplete(durable_fence()),
            ActionResult::CheckpointCommitted(CommittedRecoveryGeneration::new(
                RecoveryGeneration(4),
                TopologyEpoch(7),
            )),
            ActionResult::RangeReleased,
        ];
        let first = TransactionMachine::replay(plan(), &results).unwrap();
        let second = TransactionMachine::replay(plan(), &results).unwrap();
        assert_eq!(first.trace(), second.trace());
        assert_eq!(first.state(), second.state());
    }
}
