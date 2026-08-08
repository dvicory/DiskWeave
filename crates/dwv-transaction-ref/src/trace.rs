use crate::action::{ActionKind, ActionResult, ResultKind};
use crate::error::{ErrorClass, TransactionError};
use crate::machine::{Stage, TransactionMachine, TransactionPlan};
use dwv_core::TopologyEpoch;
use dwv_recovery::RecoveryGeneration;

pub const TRACE_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceEvent {
    ActionEmitted {
        sequence: u64,
        stage: Stage,
        kind: ActionKind,
        topology_epoch: TopologyEpoch,
        recovery_generation: RecoveryGeneration,
    },
    ResultApplied {
        sequence: u64,
        action: ActionKind,
        result: ResultKind,
        pre_stage: Stage,
        post_stage: Stage,
    },
    FrontendAbandoned {
        sequence: u64,
    },
    DaemonCrashed {
        sequence: u64,
    },
    ReconciliationRequired {
        sequence: u64,
        error: Option<ErrorClass>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trace {
    pub version: u16,
    events: Vec<TraceEvent>,
    max_events: usize,
}

impl Trace {
    pub(crate) fn new(max_events: usize) -> Self {
        Self {
            version: TRACE_VERSION,
            events: Vec::new(),
            max_events,
        }
    }

    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn events(&self) -> &[TraceEvent] {
        &self.events
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn replay(
        plan: TransactionPlan,
        results: &[ActionResult],
    ) -> Result<TransactionMachine, TransactionError> {
        TransactionMachine::replay(plan, results)
    }

    pub(crate) fn record_action(
        &mut self,
        sequence: u64,
        stage: Stage,
        kind: ActionKind,
        topology_epoch: TopologyEpoch,
        recovery_generation: RecoveryGeneration,
    ) -> Result<(), TransactionError> {
        self.push(TraceEvent::ActionEmitted {
            sequence,
            stage,
            kind,
            topology_epoch,
            recovery_generation,
        })
    }

    pub(crate) fn record_result(
        &mut self,
        sequence: u64,
        action: ActionKind,
        result: ResultKind,
        pre_stage: Stage,
        post_stage: Stage,
    ) -> Result<(), TransactionError> {
        self.push(TraceEvent::ResultApplied {
            sequence,
            action,
            result,
            pre_stage,
            post_stage,
        })
    }

    pub(crate) fn record_abandoned(&mut self, sequence: u64) -> Result<(), TransactionError> {
        self.push(TraceEvent::FrontendAbandoned { sequence })
    }

    pub(crate) fn record_daemon_crash(&mut self, sequence: u64) -> Result<(), TransactionError> {
        self.push(TraceEvent::DaemonCrashed { sequence })
    }

    pub(crate) fn record_reconciliation_required(
        &mut self,
        sequence: u64,
        error: Option<ErrorClass>,
    ) -> Result<(), TransactionError> {
        self.push(TraceEvent::ReconciliationRequired { sequence, error })
    }

    fn push(&mut self, event: TraceEvent) -> Result<(), TransactionError> {
        if self.events.len() >= self.max_events {
            return Err(TransactionError::TraceLimitExceeded);
        }
        self.events.push(event);
        Ok(())
    }
}
