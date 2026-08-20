//! Isolated `procmachines` comparison adapter for DiskWeave transactions.
//!
//! The dependency is intentionally hidden behind semantic DiskWeave actions,
//! results, and terminal classifications.

use dwv_recovery::FenceCertificate;
use dwv_store::{CapabilityEvidenceId, FenceId, StoreFenceRef};
use dwv_transaction_ref::{
    ActionKind, ActionResult, ResultKind, Stage, TransactionAction, TransactionPlan,
};
use procmachines::{
    IoExchange, IoSink, IoStream, Loop, PROC_MACHINE_BUILDER, ProcMachine, ProcMachineJobs,
    RefLockable, TaskEnd, wait_loop,
};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CandidateTerminal {
    Completed,
    Aborted,
    ReconciliationRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedEvent {
    pub sequence: u64,
    pub action: ActionKind,
    pub result: ResultKind,
    pub pre_stage: Stage,
    pub post_stage: Stage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateRun {
    pub events: Vec<NormalizedEvent>,
    pub terminal: CandidateTerminal,
    pub metrics: CandidateMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComparisonMismatch {
    Candidate(CandidateError),
    Reference(String),
    Trace { index: usize },
    Terminal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComparisonReport {
    pub equivalent: bool,
    pub reference_events: Vec<NormalizedEvent>,
    pub reference_terminal: Option<CandidateTerminal>,
    pub candidate: Option<CandidateRun>,
    pub metrics: CandidateMetrics,
    pub reference_elapsed_nanos: u128,
    pub reference_machine_size_bytes: usize,
    pub mismatch: Option<ComparisonMismatch>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CandidateMetrics {
    pub polls: u64,
    pub synchronization_operations: u64,
    pub action_count: u64,
    pub result_count: u64,
    pub io_size_bytes: usize,
    pub elapsed_nanos: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateError {
    MachineClosed,
    MissingAction,
    MissingTerminal,
    ResultClosed,
    SpinLimitExceeded,
}

impl std::fmt::Display for CandidateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "procedural candidate error: {self:?}")
    }
}

impl std::error::Error for CandidateError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CandidateControl {
    Abandon,
    DaemonCrash,
    PowerLoss,
}

enum ResultWait {
    Result(ActionResult),
    Terminal(CandidateTerminal),
}

struct CandidateIo {
    actions: IoExchange<TransactionAction>,
    results: IoExchange<ActionResult>,
    controls: IoExchange<CandidateControl>,
    terminal: IoExchange<CandidateTerminal>,
    plan: TransactionPlan,
}

impl CandidateIo {
    fn new(plan: TransactionPlan) -> Self {
        Self {
            actions: IoExchange::new(),
            results: IoExchange::new(),
            controls: IoExchange::new(),
            terminal: IoExchange::new(),
            plan,
        }
    }
}

async fn candidate_task(io: Pin<&CandidateIo>) -> TaskEnd {
    let actions = semantic_actions(&io.plan);
    let mut terminal = None;
    let mut abandoned = false;
    for action in actions {
        let mut pending_action = Some(action.clone());
        let sent = wait_loop(
            |cx| match io.actions.prod_poll_send(cx, &mut pending_action) {
                Poll::Ready(Ok(_)) => Loop::Done(true),
                Poll::Ready(Err(_)) => Loop::Done(false),
                Poll::Pending => Loop::Wait,
            },
        )
        .await;
        if !sent {
            return TaskEnd();
        }

        let outcome = wait_loop(|cx| {
            match io.controls.con_poll_read(cx) {
                Poll::Ready(Ok(Some(CandidateControl::Abandon))) => {
                    abandoned = true;
                    return Loop::Again;
                }
                Poll::Ready(Ok(Some(CandidateControl::DaemonCrash)))
                | Poll::Ready(Ok(Some(CandidateControl::PowerLoss))) => {
                    let terminal = if action.kind() == ActionKind::AcquireRange
                        && io.plan.write_recovery_record.requires_commit()
                    {
                        CandidateTerminal::Aborted
                    } else {
                        CandidateTerminal::ReconciliationRequired
                    };
                    return Loop::Done(ResultWait::Terminal(terminal));
                }
                Poll::Ready(Ok(None)) | Poll::Ready(Err(_)) | Poll::Pending => {}
            }
            match io.results.con_poll_read(cx) {
                Poll::Ready(Ok(Some(result))) => Loop::Done(ResultWait::Result(result)),
                Poll::Ready(Ok(None)) | Poll::Ready(Err(_)) => Loop::Done(ResultWait::Terminal(
                    CandidateTerminal::ReconciliationRequired,
                )),
                Poll::Pending => Loop::Wait,
            }
        })
        .await;
        let result = match outcome {
            ResultWait::Result(result) => result,
            ResultWait::Terminal(outcome) => {
                terminal = Some(outcome);
                break;
            }
        };
        if let Some(outcome) = classify_result(&io.plan, action.kind(), &result) {
            terminal = Some(outcome);
            break;
        }
    }
    let _ = abandoned;
    let terminal = terminal.unwrap_or(CandidateTerminal::Completed);
    let mut pending_terminal = Some(terminal);
    let _ = wait_loop(
        |cx| match io.terminal.prod_poll_send(cx, &mut pending_terminal) {
            Poll::Ready(Ok(_)) | Poll::Ready(Err(_)) => Loop::Done(()),
            Poll::Pending => Loop::Wait,
        },
    )
    .await;
    TaskEnd()
}

pub struct CandidateMachine {
    machine: Arc<dyn ProcMachine<IO = CandidateIo>>,
    metrics: CandidateMetrics,
}

impl CandidateMachine {
    pub fn new(plan: TransactionPlan) -> Self {
        let machine: Arc<dyn ProcMachine<IO = CandidateIo>> = PROC_MACHINE_BUILDER
            .with(candidate_task)
            .build_std(CandidateIo::new(plan));
        Self {
            machine,
            metrics: CandidateMetrics {
                polls: 0,
                synchronization_operations: 0,
                action_count: 0,
                result_count: 0,
                io_size_bytes: std::mem::size_of::<CandidateIo>(),
                elapsed_nanos: 0,
            },
        }
    }

    pub fn is_done(&self) -> bool {
        self.machine.is_done()
    }
    fn requires_commit(&self) -> bool {
        self.machine
            .lock_ref()
            .plan
            .write_recovery_record
            .requires_commit()
    }
    fn send_control(&mut self, control: CandidateControl) -> Result<(), CandidateError> {
        self.metrics.polls += 1;
        self.metrics.synchronization_operations += 1;
        let mut pending = Some(control);
        let mut context = Context::from_waker(Waker::noop());
        let io = self.machine.lock_ref();
        match io.controls.prod_poll_send(&mut context, &mut pending) {
            Poll::Ready(Ok(_)) => Ok(()),
            Poll::Pending | Poll::Ready(Err(_)) => Err(CandidateError::ResultClosed),
        }
    }

    pub fn abandon(&mut self) -> Result<(), CandidateError> {
        self.send_control(CandidateControl::Abandon)
    }

    pub fn daemon_crash(&mut self) -> Result<(), CandidateError> {
        self.send_control(CandidateControl::DaemonCrash)
    }

    pub fn power_loss(&mut self) -> Result<(), CandidateError> {
        self.send_control(CandidateControl::PowerLoss)
    }

    pub fn poll_action(&mut self) -> Result<Option<TransactionAction>, CandidateError> {
        self.metrics.polls += 1;
        let mut context = Context::from_waker(Waker::noop());
        let io = self.machine.lock_ref();
        match io.actions.con_poll_read(&mut context) {
            Poll::Ready(Ok(Some(action))) => {
                self.metrics.action_count += 1;
                Ok(Some(action))
            }
            Poll::Ready(Ok(None)) => Err(CandidateError::MachineClosed),
            Poll::Ready(Err(_)) => Err(CandidateError::MachineClosed),
            Poll::Pending => Ok(None),
        }
    }

    pub fn submit_result(&mut self, result: ActionResult) -> Result<(), CandidateError> {
        self.metrics.polls += 1;
        self.metrics.synchronization_operations += 1;
        let mut pending = Some(result);
        let mut context = Context::from_waker(Waker::noop());
        let io = self.machine.lock_ref();
        match io.results.prod_poll_send(&mut context, &mut pending) {
            Poll::Ready(Ok(_)) => {
                self.metrics.result_count += 1;
                Ok(())
            }
            Poll::Pending => Err(CandidateError::ResultClosed),
            Poll::Ready(Err(_)) => Err(CandidateError::ResultClosed),
        }
    }

    pub fn poll_terminal(&mut self) -> Result<Option<CandidateTerminal>, CandidateError> {
        self.metrics.polls += 1;
        let mut context = Context::from_waker(Waker::noop());
        let io = self.machine.lock_ref();
        match io.terminal.con_poll_read(&mut context) {
            Poll::Ready(Ok(Some(terminal))) => Ok(Some(terminal)),
            Poll::Ready(Ok(None)) | Poll::Ready(Err(_)) => Err(CandidateError::MissingTerminal),
            Poll::Pending => Ok(None),
        }
    }

    pub fn metrics(&self) -> CandidateMetrics {
        self.metrics
    }
}

pub fn semantic_actions(plan: &TransactionPlan) -> Vec<TransactionAction> {
    let generation = plan.captured_write_recovery_record_generation();
    let certificate = plan.checksum_extents.iter().fold(
        FenceCertificate::new(
            plan.topology_epoch,
            plan.fence_domain,
            plan.through
                .iter()
                .map(|watermark| StoreFenceRef {
                    fence_id: FenceId(watermark.store.0),
                    store_id: watermark.store,
                    store_incarnation: watermark.incarnation,
                    topology_epoch: plan.topology_epoch,
                    through: watermark.through,
                    capability_evidence_id: CapabilityEvidenceId(watermark.store.0),
                })
                .collect(),
            plan.dirty_regions
                .iter()
                .map(|region| (*region, generation))
                .collect(),
        ),
        |certificate, extent| certificate.with_integrity_extent(*extent, generation),
    );
    let mut actions = vec![TransactionAction::AcquireRange {
        ranges: plan.ranges.clone(),
    }];
    if plan.write_recovery_record.requires_commit() {
        actions.push(TransactionAction::PersistDirtyAndInvalidateIntegrity {
            dirty_regions: plan.dirty_regions.clone(),
            checksum_extents: plan.checksum_extents.clone(),
        });
    }
    actions.extend([
        TransactionAction::ReadSet {
            reads: plan.reads.clone(),
        },
        TransactionAction::ComputeParity { plan: plan.parity },
        TransactionAction::WriteSet {
            writes: plan.writes.clone(),
        },
        TransactionAction::FlushSet {
            stores: plan.stores.clone(),
            through: plan.through.clone(),
        },
        TransactionAction::CommitRecoveryClean { certificate },
        TransactionAction::ReleaseRange,
    ]);
    actions
}

pub fn drive_candidate(
    plan: TransactionPlan,
    results: &[ActionResult],
) -> Result<CandidateRun, CandidateError> {
    let started = Instant::now();
    let mut machine = CandidateMachine::new(plan);
    let mut events = Vec::new();
    let requires_commit = machine.requires_commit();
    for (sequence, expected_result) in results.iter().enumerate() {
        let action = poll_until_action(&mut machine)?;
        if !result_matches(action.kind(), expected_result.kind()) {
            return Err(CandidateError::MissingAction);
        }
        machine.submit_result(expected_result.clone())?;
        events.push(NormalizedEvent {
            sequence: sequence as u64,
            action: action.kind(),
            result: expected_result.kind(),
            pre_stage: stage_for_action(action.kind()),
            post_stage: stage_after_result(requires_commit, action.kind(), expected_result),
        });
        if machine.is_done() {
            break;
        }
    }
    let terminal = poll_until_terminal(&mut machine)?;
    let mut metrics = machine.metrics();
    metrics.elapsed_nanos = started.elapsed().as_nanos();
    Ok(CandidateRun {
        events,
        terminal,
        metrics,
    })
}
pub fn compare_schedule(plan: TransactionPlan, results: &[ActionResult]) -> ComparisonReport {
    let reference_started = Instant::now();
    let mut reference = match dwv_transaction_ref::TransactionMachine::new(plan.clone()) {
        Ok(machine) => machine,
        Err(error) => {
            return ComparisonReport {
                equivalent: false,
                reference_events: Vec::new(),
                reference_terminal: None,
                candidate: None,
                metrics: CandidateMetrics::default(),
                reference_elapsed_nanos: reference_started.elapsed().as_nanos(),
                reference_machine_size_bytes: std::mem::size_of::<
                    dwv_transaction_ref::TransactionMachine,
                >(),
                mismatch: Some(ComparisonMismatch::Reference(error.to_string())),
            };
        }
    };
    let mut reference_events = Vec::new();
    for (sequence, result) in results.iter().enumerate() {
        let Some(action) = reference.next_action().cloned() else {
            break;
        };
        let pre_stage = reference.stage();
        if let Err(error) = reference.apply(result.clone()) {
            return ComparisonReport {
                equivalent: false,
                reference_events,
                reference_terminal: terminal_for_reference(&reference),
                candidate: None,
                metrics: CandidateMetrics::default(),
                reference_elapsed_nanos: reference_started.elapsed().as_nanos(),
                reference_machine_size_bytes: std::mem::size_of::<
                    dwv_transaction_ref::TransactionMachine,
                >(),
                mismatch: Some(ComparisonMismatch::Reference(error.to_string())),
            };
        }
        reference_events.push(NormalizedEvent {
            sequence: sequence as u64,
            action: action.kind(),
            result: result.kind(),
            pre_stage,
            post_stage: reference.stage(),
        });
        if reference.is_terminal() {
            break;
        }
    }

    let reference_elapsed_nanos = reference_started.elapsed().as_nanos();
    let reference_terminal = terminal_for_reference(&reference);
    let candidate = match drive_candidate(plan, results) {
        Ok(candidate) => candidate,
        Err(error) => {
            return ComparisonReport {
                equivalent: false,
                reference_events,
                reference_terminal,
                candidate: None,
                metrics: CandidateMetrics::default(),
                reference_elapsed_nanos: reference_started.elapsed().as_nanos(),
                reference_machine_size_bytes: std::mem::size_of::<
                    dwv_transaction_ref::TransactionMachine,
                >(),
                mismatch: Some(ComparisonMismatch::Candidate(error)),
            };
        }
    };
    let trace_mismatch_index = candidate
        .events
        .iter()
        .zip(&reference_events)
        .position(|(candidate, reference)| candidate != reference)
        .or_else(|| {
            (candidate.events.len() != reference_events.len())
                .then_some(candidate.events.len().min(reference_events.len()))
        });
    let terminal_matches = Some(candidate.terminal) == reference_terminal;
    let candidate_metrics = candidate.metrics;
    ComparisonReport {
        equivalent: trace_mismatch_index.is_none() && terminal_matches,
        reference_events,
        reference_terminal,
        candidate: Some(candidate),
        metrics: candidate_metrics,
        reference_elapsed_nanos,
        reference_machine_size_bytes: std::mem::size_of::<dwv_transaction_ref::TransactionMachine>(
        ),
        mismatch: if let Some(index) = trace_mismatch_index {
            Some(ComparisonMismatch::Trace { index })
        } else if !terminal_matches {
            Some(ComparisonMismatch::Terminal)
        } else {
            None
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComparisonMeasurement {
    pub repetitions: usize,
    pub median_candidate_elapsed_nanos: u128,
    pub median_reference_elapsed_nanos: u128,
    pub synchronization_operations: u64,
    pub candidate_io_size_bytes: usize,
    pub reference_machine_size_bytes: usize,
}

pub fn measure_schedule(
    plan: TransactionPlan,
    results: &[ActionResult],
    repetitions: usize,
) -> Result<ComparisonMeasurement, ComparisonMismatch> {
    let repetitions = repetitions.max(1);
    let mut candidate_elapsed = Vec::with_capacity(repetitions);
    let mut reference_elapsed = Vec::with_capacity(repetitions);
    let mut last = None;
    for _ in 0..repetitions {
        let report = compare_schedule(plan.clone(), results);
        if !report.equivalent {
            return Err(report.mismatch.unwrap_or(ComparisonMismatch::Terminal));
        }
        candidate_elapsed.push(report.metrics.elapsed_nanos);
        reference_elapsed.push(report.reference_elapsed_nanos);
        last = Some(report);
    }
    candidate_elapsed.sort_unstable();
    reference_elapsed.sort_unstable();
    let report = last.expect("repetitions is at least one");
    Ok(ComparisonMeasurement {
        repetitions,
        median_candidate_elapsed_nanos: candidate_elapsed[candidate_elapsed.len() / 2],
        median_reference_elapsed_nanos: reference_elapsed[reference_elapsed.len() / 2],
        synchronization_operations: report.metrics.synchronization_operations,
        candidate_io_size_bytes: report.metrics.io_size_bytes,
        reference_machine_size_bytes: report.reference_machine_size_bytes,
    })
}

fn terminal_for_reference(
    machine: &dwv_transaction_ref::TransactionMachine,
) -> Option<CandidateTerminal> {
    match machine.terminal_disposition()? {
        dwv_transaction_ref::TerminalDisposition::Completed => Some(CandidateTerminal::Completed),
        dwv_transaction_ref::TerminalDisposition::Aborted => Some(CandidateTerminal::Aborted),
        dwv_transaction_ref::TerminalDisposition::ReconciliationRequired => {
            Some(CandidateTerminal::ReconciliationRequired)
        }
    }
}

fn result_matches(action: ActionKind, result: ResultKind) -> bool {
    result == ResultKind::Failed
        || matches!(
            (action, result),
            (ActionKind::AcquireRange, ResultKind::RangeAcquired)
                | (
                    ActionKind::PersistDirtyAndInvalidateIntegrity,
                    ResultKind::WriteRecoveryRecordDurable
                )
                | (ActionKind::ReadSet, ResultKind::ReadSetComplete)
                | (ActionKind::ComputeParity, ResultKind::ParityComputed)
                | (ActionKind::WriteSet, ResultKind::WriteSetComplete)
                | (ActionKind::FlushSet, ResultKind::FlushSetComplete)
                | (
                    ActionKind::CommitRecoveryClean,
                    ResultKind::RecoveryCleanCommitted
                )
                | (ActionKind::ReleaseRange, ResultKind::RangeReleased)
        )
}

fn poll_until_action(machine: &mut CandidateMachine) -> Result<TransactionAction, CandidateError> {
    for _ in 0..1024 {
        if let Some(action) = machine.poll_action()? {
            return Ok(action);
        }
    }
    Err(CandidateError::SpinLimitExceeded)
}

fn poll_until_terminal(
    machine: &mut CandidateMachine,
) -> Result<CandidateTerminal, CandidateError> {
    for _ in 0..1024 {
        if let Some(terminal) = machine.poll_terminal()? {
            return Ok(terminal);
        }
    }
    Err(CandidateError::SpinLimitExceeded)
}

fn classify_result(
    plan: &TransactionPlan,
    action: ActionKind,
    result: &ActionResult,
) -> Option<CandidateTerminal> {
    let failed = match result {
        ActionResult::Failed(_) => true,
        ActionResult::WriteRecoveryRecordDurable(committed) => {
            committed.topology_epoch != plan.topology_epoch
                || committed.generation != plan.captured_write_recovery_record_generation()
        }
        ActionResult::WriteRecoveryRecordDurableWithEvidence(evidence) => {
            !evidence.durable
                || evidence.topology_epoch != plan.topology_epoch
                || evidence.captured_generation != plan.captured_write_recovery_record_generation()
                || evidence.committed_generation.0 < evidence.captured_generation.0
                || evidence.target != plan.invalidation_target()
        }
        ActionResult::RecoveryCleanCommitted(committed) => {
            committed.topology_epoch != plan.topology_epoch
                || committed.generation.0 < plan.captured_write_recovery_record_generation().0
        }
        ActionResult::ReadSetComplete(value) => {
            !matches!(value, dwv_transaction_ref::SemanticIoResult::Complete)
        }
        ActionResult::ParityComputed(value) => {
            !matches!(value, dwv_transaction_ref::ComputationResult::Complete)
        }

        ActionResult::WriteSetComplete(value) => {
            !matches!(value, dwv_transaction_ref::SemanticIoResult::Complete)
        }
        ActionResult::FlushSetComplete(evidence) => !evidence.covers(
            plan.topology_epoch,
            plan.fence_domain,
            &plan.through,
            &plan.dirty_regions,
            &plan.checksum_extents,
            plan.captured_write_recovery_record_generation(),
        ),
        _ => false,
    };
    if !failed {
        return None;
    }
    if action == ActionKind::AcquireRange && plan.write_recovery_record.requires_commit() {
        Some(CandidateTerminal::Aborted)
    } else {
        Some(CandidateTerminal::ReconciliationRequired)
    }
}

fn stage_for_action(action: ActionKind) -> Stage {
    match action {
        ActionKind::AcquireRange => Stage::AcquireRange,
        ActionKind::PersistDirtyAndInvalidateIntegrity => Stage::WriteRecoveryRecordCommit,
        ActionKind::ReadSet => Stage::ReadSet,
        ActionKind::ComputeParity => Stage::ComputeParity,
        ActionKind::WriteSet => Stage::WriteSet,
        ActionKind::FlushSet => Stage::FlushSet,
        ActionKind::CommitRecoveryClean => Stage::RecoveryClean,
        ActionKind::ReleaseRange => Stage::Release,
    }
}

fn stage_after_result(requires_commit: bool, action: ActionKind, result: &ActionResult) -> Stage {
    if classify_result_for_stage(result) {
        return if action == ActionKind::AcquireRange && requires_commit {
            Stage::Aborted
        } else {
            Stage::AwaitingReconciliation
        };
    }
    match action {
        ActionKind::AcquireRange => {
            if requires_commit {
                Stage::WriteRecoveryRecordCommit
            } else {
                Stage::ReadSet
            }
        }
        ActionKind::PersistDirtyAndInvalidateIntegrity => Stage::ReadSet,
        ActionKind::ReadSet => Stage::ComputeParity,
        ActionKind::ComputeParity => Stage::WriteSet,
        ActionKind::WriteSet => Stage::FlushSet,
        ActionKind::FlushSet => Stage::RecoveryClean,
        ActionKind::CommitRecoveryClean => Stage::Release,
        ActionKind::ReleaseRange => Stage::Completed,
    }
}

fn classify_result_for_stage(result: &ActionResult) -> bool {
    matches!(
        result,
        ActionResult::Failed(_)
            | ActionResult::ReadSetComplete(dwv_transaction_ref::SemanticIoResult::Failed)
            | ActionResult::ReadSetComplete(dwv_transaction_ref::SemanticIoResult::Uncertain)
            | ActionResult::ParityComputed(dwv_transaction_ref::ComputationResult::Failed)
            | ActionResult::WriteSetComplete(dwv_transaction_ref::SemanticIoResult::Failed)
            | ActionResult::WriteSetComplete(dwv_transaction_ref::SemanticIoResult::Uncertain)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_emits_semantic_actions_without_procmachine_types() {
        let plan = test_plan();
        let actions = semantic_actions(&plan);
        assert_eq!(actions.first().unwrap().kind(), ActionKind::AcquireRange);
        assert_eq!(actions.last().unwrap().kind(), ActionKind::ReleaseRange);
        assert_eq!(actions.len(), 8);
    }
    #[test]
    fn candidate_trace_matches_reference_machine() {
        let plan = test_plan();
        let results = successful_results(&plan);
        let mut reference = dwv_transaction_ref::TransactionMachine::new(plan.clone()).unwrap();
        let mut expected = Vec::new();
        for result in &results {
            let action = reference.next_action().unwrap().kind();
            let pre_stage = reference.stage();
            reference.apply(result.clone()).unwrap();
            expected.push((action, result.kind(), pre_stage, reference.stage()));
        }

        let candidate = drive_candidate(plan, &results).unwrap();
        let actual: Vec<_> = candidate
            .events
            .iter()
            .map(|event| {
                (
                    event.action,
                    event.result,
                    event.pre_stage,
                    event.post_stage,
                )
            })
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(
            candidate.terminal,
            match reference.terminal_disposition().unwrap() {
                dwv_transaction_ref::TerminalDisposition::Completed => CandidateTerminal::Completed,
                dwv_transaction_ref::TerminalDisposition::Aborted => CandidateTerminal::Aborted,
                dwv_transaction_ref::TerminalDisposition::ReconciliationRequired => {
                    CandidateTerminal::ReconciliationRequired
                }
            }
        );
    }

    #[test]
    fn candidate_completes_with_external_semantic_results() {
        let plan = test_plan();
        let results = successful_results(&plan);
        let run = drive_candidate(plan, &results).unwrap();

        assert_eq!(run.terminal, CandidateTerminal::Completed);
        assert_eq!(run.events.len(), results.len());
        assert!(run.metrics.action_count >= results.len() as u64);
    }

    #[test]
    fn candidate_requires_reconciliation_after_acquisition_failure() {
        let run = drive_candidate(
            test_plan(),
            &[ActionResult::failure(
                dwv_transaction_ref::ErrorClass::RangeAcquisitionFailed,
            )],
        )
        .unwrap();

        assert_eq!(run.terminal, CandidateTerminal::Aborted);
        assert_eq!(run.events[0].post_stage, Stage::Aborted);
    }

    #[test]
    fn comparison_accepts_failure_before_durable_write_recovery_record() {
        let report = compare_schedule(
            test_plan(),
            &[ActionResult::failure(
                dwv_transaction_ref::ErrorClass::RangeAcquisitionFailed,
            )],
        );

        assert!(report.equivalent);
        assert_eq!(report.reference_terminal, Some(CandidateTerminal::Aborted));
        assert!(report.candidate.unwrap().terminal == CandidateTerminal::Aborted);
    }

    #[test]
    fn comparison_rejects_irreversible_uncertainty_and_stale_generation() {
        let mut uncertain = successful_results(&test_plan());
        uncertain[4] =
            ActionResult::WriteSetComplete(dwv_transaction_ref::SemanticIoResult::Uncertain);
        let report = compare_schedule(test_plan(), &uncertain);
        assert!(report.equivalent);
        assert_eq!(
            report.reference_terminal,
            Some(CandidateTerminal::ReconciliationRequired)
        );

        let mut stale = successful_results(&test_plan());
        stale[1] = ActionResult::WriteRecoveryRecordDurable(
            dwv_transaction_ref::CommittedRecoveryGeneration::new(
                dwv_recovery::RecoveryGeneration(3),
                dwv_core::TopologyEpoch(1),
            ),
        );

        let report = compare_schedule(test_plan(), &stale);
        assert!(!report.equivalent);
        assert!(matches!(
            report.mismatch,
            Some(ComparisonMismatch::Reference(_))
        ));
    }

    #[test]
    fn comparison_never_clears_without_durable_fence_evidence() {
        let plan = test_plan();
        let mut results = successful_results(&plan);
        let certificate = match &results[5] {
            ActionResult::FlushSetComplete(evidence) => evidence.certificate.clone(),
            _ => unreachable!(),
        };
        results[5] = ActionResult::FlushSetComplete(
            dwv_transaction_ref::TransactionPersistenceEvidence::volatile(certificate),
        );

        let candidate = drive_candidate(plan.clone(), &results).unwrap();
        assert_eq!(
            candidate.terminal,
            CandidateTerminal::ReconciliationRequired
        );

        let report = compare_schedule(plan, &results);
        assert!(!report.equivalent);
        assert!(report.mismatch.is_some());
    }
    #[test]
    fn short_io_maps_to_the_same_failure_class_as_eio() {
        let mut short = successful_results(&test_plan());
        short[2] = ActionResult::ReadSetComplete(dwv_transaction_ref::SemanticIoResult::Failed);
        let report = compare_schedule(test_plan(), &short);

        assert!(report.equivalent);
        assert_eq!(
            report.reference_terminal,
            Some(CandidateTerminal::ReconciliationRequired)
        );
    }

    #[test]
    fn comparison_ignores_duplicate_terminal_delivery() {
        let plan = test_plan();
        let mut results = successful_results(&plan);
        results.push(ActionResult::RangeReleased);

        assert!(compare_schedule(plan, &results).equivalent);
    }

    #[test]
    fn delayed_and_out_of_order_delivery_stay_at_semantic_boundary() {
        let mut machine = CandidateMachine::new(test_plan());
        assert_eq!(
            machine.poll_action().unwrap().unwrap().kind(),
            ActionKind::AcquireRange
        );
        for _ in 0..4 {
            assert!(machine.poll_action().unwrap().is_none());
        }
        machine
            .submit_result(ActionResult::RangeAcquired(
                dwv_transaction_ref::RangeGuardToken::new(1),
            ))
            .unwrap();
        assert_eq!(
            machine.poll_action().unwrap().unwrap().kind(),
            ActionKind::PersistDirtyAndInvalidateIntegrity
        );

        let report = compare_schedule(
            test_plan(),
            &[ActionResult::ReadSetComplete(
                dwv_transaction_ref::SemanticIoResult::Complete,
            )],
        );
        assert!(!report.equivalent);
    }

    #[test]
    fn abandonment_preserves_semantic_ownership_until_result() {
        let plan = test_plan();
        let results = successful_results(&plan);
        let mut reference = dwv_transaction_ref::TransactionMachine::new(plan.clone()).unwrap();
        reference.abandon().unwrap();
        for result in &results {
            reference.apply(result.clone()).unwrap();
        }
        assert_eq!(
            reference.terminal_disposition(),
            Some(dwv_transaction_ref::TerminalDisposition::Completed)
        );

        let mut candidate = CandidateMachine::new(plan);
        assert_eq!(
            candidate.poll_action().unwrap().unwrap().kind(),
            ActionKind::AcquireRange
        );
        candidate.abandon().unwrap();
        candidate.submit_result(results[0].clone()).unwrap();
        for result in &results[1..] {
            candidate.poll_action().unwrap();
            candidate.submit_result(result.clone()).unwrap();
        }
        assert_eq!(
            poll_until_terminal(&mut candidate).unwrap(),
            CandidateTerminal::Completed
        );
    }

    #[test]
    fn abandonment_after_durable_write_recovery_record_keeps_completion_owned() {
        let plan = test_plan();
        let results = successful_results(&plan);
        let mut reference = dwv_transaction_ref::TransactionMachine::new(plan.clone()).unwrap();
        for result in &results[..2] {
            reference.apply(result.clone()).unwrap();
        }
        reference.abandon().unwrap();
        for result in &results[2..] {
            reference.apply(result.clone()).unwrap();
        }
        assert_eq!(
            reference.terminal_disposition(),
            Some(dwv_transaction_ref::TerminalDisposition::Completed)
        );

        let mut candidate = CandidateMachine::new(plan);
        for result in &results[..2] {
            candidate.poll_action().unwrap();
            candidate.submit_result(result.clone()).unwrap();
        }
        candidate.abandon().unwrap();
        for result in &results[2..] {
            candidate.poll_action().unwrap();
            candidate.submit_result(result.clone()).unwrap();
        }
        assert_eq!(
            poll_until_terminal(&mut candidate).unwrap(),
            CandidateTerminal::Completed
        );
    }

    #[test]
    fn crash_and_power_loss_preserve_pre_and_post_write_recovery_record_classes() {
        let mut before_write_recovery_record = CandidateMachine::new(test_plan());
        before_write_recovery_record.poll_action().unwrap();
        before_write_recovery_record.daemon_crash().unwrap();
        assert_eq!(
            poll_until_terminal(&mut before_write_recovery_record).unwrap(),
            CandidateTerminal::Aborted
        );

        let mut after_write_recovery_record = CandidateMachine::new(test_plan());
        after_write_recovery_record.poll_action().unwrap();
        after_write_recovery_record
            .submit_result(ActionResult::RangeAcquired(
                dwv_transaction_ref::RangeGuardToken::new(1),
            ))
            .unwrap();
        after_write_recovery_record.poll_action().unwrap();
        after_write_recovery_record.power_loss().unwrap();
        assert_eq!(
            poll_until_terminal(&mut after_write_recovery_record).unwrap(),
            CandidateTerminal::ReconciliationRequired
        );
    }

    #[test]
    fn crash_and_power_loss_match_at_every_suspension_point() {
        let plan = test_plan();
        let results = successful_results(&plan);
        for index in 0..results.len() {
            let mut reference = dwv_transaction_ref::TransactionMachine::new(plan.clone()).unwrap();
            for result in results.iter().take(index) {
                reference.apply(result.clone()).unwrap();
            }
            reference.daemon_crash().unwrap();
            let expected = terminal_for_reference(&reference).unwrap();

            let mut candidate = CandidateMachine::new(plan.clone());
            for result in results.iter().take(index) {
                candidate.poll_action().unwrap();
                candidate.submit_result(result.clone()).unwrap();
            }
            candidate.poll_action().unwrap();
            candidate.daemon_crash().unwrap();
            assert_eq!(poll_until_terminal(&mut candidate).unwrap(), expected);

            let mut power_loss = CandidateMachine::new(plan.clone());
            for result in results.iter().take(index) {
                power_loss.poll_action().unwrap();
                power_loss.submit_result(result.clone()).unwrap();
            }
            power_loss.poll_action().unwrap();
            power_loss.power_loss().unwrap();
            assert_eq!(poll_until_terminal(&mut power_loss).unwrap(), expected);
        }
    }

    #[test]
    fn comparison_report_records_equivalence_and_metrics() {
        let plan = test_plan();
        let results = successful_results(&plan);
        let report = compare_schedule(plan, &results);

        assert!(report.equivalent);
        assert!(report.mismatch.is_none());
        assert_eq!(report.reference_events.len(), results.len());
        assert!(report.candidate.unwrap().metrics.synchronization_operations > 0);
    }

    #[test]
    fn measurement_reports_median_and_structural_sizes() {
        let plan = test_plan();
        let results = successful_results(&plan);
        let measurement = measure_schedule(plan, &results, 3).unwrap();

        assert_eq!(measurement.repetitions, 3);
        assert!(measurement.median_candidate_elapsed_nanos > 0);
        assert!(measurement.median_reference_elapsed_nanos > 0);
        assert!(measurement.synchronization_operations > 0);
        assert!(measurement.candidate_io_size_bytes > 0);
        assert!(measurement.reference_machine_size_bytes > 0);
    }

    fn successful_results(plan: &TransactionPlan) -> Vec<ActionResult> {
        use dwv_core::TopologyEpoch;
        use dwv_recovery::RecoveryGeneration;
        use dwv_transaction_ref::{
            CommittedRecoveryGeneration, ComputationResult, RangeGuardToken, SemanticIoResult,
            TransactionPersistenceEvidence,
        };

        let certificate = semantic_actions(plan)
            .into_iter()
            .find_map(|action| match action {
                TransactionAction::CommitRecoveryClean { certificate } => Some(certificate),
                _ => None,
            })
            .unwrap();
        let generation = CommittedRecoveryGeneration::new(RecoveryGeneration(2), TopologyEpoch(1));
        vec![
            ActionResult::RangeAcquired(RangeGuardToken::new(1)),
            ActionResult::WriteRecoveryRecordDurable(generation),
            ActionResult::ReadSetComplete(SemanticIoResult::Complete),
            ActionResult::ParityComputed(ComputationResult::Complete),
            ActionResult::WriteSetComplete(SemanticIoResult::Complete),
            ActionResult::FlushSetComplete(TransactionPersistenceEvidence::durable(certificate)),
            ActionResult::RecoveryCleanCommitted(generation),
            ActionResult::RangeReleased,
        ]
    }

    fn test_plan() -> TransactionPlan {
        use dwv_core::{ByteRange, FenceDomain, TopologyEpoch};
        use dwv_recovery::{IntegrityExtentId, RecoveryGeneration, RegionId};
        use dwv_store::{StoreId, StoreWriteWatermark};
        use dwv_transaction_ref::{ParityComputationPlan, ParityRange, PlannedRead, PlannedWrite};

        TransactionPlan::new(TopologyEpoch(1), RecoveryGeneration(2), FenceDomain(3))
            .with_ranges(vec![ParityRange::new(
                RegionId(1),
                StoreId(1),
                ByteRange::new(0, 512).unwrap(),
            )])
            .with_dirty_regions(vec![RegionId(1)])
            .with_checksum_extents(vec![IntegrityExtentId(1)])
            .with_reads(vec![PlannedRead::new(
                StoreId(1),
                ByteRange::new(0, 512).unwrap(),
            )])
            .with_parity(ParityComputationPlan::new(
                TopologyEpoch(1),
                ByteRange::new(0, 512).unwrap(),
                1,
            ))
            .with_writes(vec![PlannedWrite::new(
                StoreId(1),
                ByteRange::new(0, 512).unwrap(),
            )])
            .with_stores(vec![StoreId(1)])
            .with_watermarks(vec![dwv_transaction_ref::StoreWatermark::new(
                StoreId(1),
                StoreWriteWatermark(1),
            )])
    }
}
