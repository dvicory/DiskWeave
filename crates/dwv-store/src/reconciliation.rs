use crate::*;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotState {
    Reserved,
    Submitted,
    PartiallyCompleted,
    Draining,
    CompletionUncertain,
    ReconciliationRequired,
    Reclaimable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DrainState {
    NotRequired,
    Required,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconciliationOutcome {
    Durable,
    UncertainRetained,
    Invalidated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreInvalidationReason {
    Disappeared,
    IdentityChanged,
    GeometryChanged,
    TopologyEpochChanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryEligibility {
    Allowed,
    ReconcileFirst,
    Forbidden,
    CallerDecision,
}

pub const fn retry_eligibility(
    idempotent: bool,
    effect_uncertain: bool,
    duplicate_delivery_possible: bool,
) -> RetryEligibility {
    if effect_uncertain {
        if !idempotent {
            RetryEligibility::Forbidden
        } else if duplicate_delivery_possible {
            RetryEligibility::ReconcileFirst
        } else {
            RetryEligibility::Allowed
        }
    } else if idempotent {
        RetryEligibility::Allowed
    } else {
        RetryEligibility::CallerDecision
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SlotError {
    StaleGeneration {
        token: OperationSlotToken,
    },
    GenerationExhausted {
        index: u32,
    },
    InvalidState {
        token: OperationSlotToken,
        state: SlotState,
    },
    ChildUnknown {
        operation_id: ChildOperationId,
    },
    UnexpectedDuplicate {
        operation_id: ChildOperationId,
    },
    Completion(StoreError),
    Admission(AdmissionError),
    ChildrenNotTerminal,
    DrainIncomplete,
}

impl fmt::Display for SlotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleGeneration { token } => write!(formatter, "stale operation slot {token:?}"),
            Self::GenerationExhausted { index } => {
                write!(formatter, "operation slot {index} generation exhausted")
            }
            Self::InvalidState { token, state } => {
                write!(formatter, "operation slot {token:?} is in state {state:?}")
            }
            Self::ChildUnknown { operation_id } => {
                write!(formatter, "unknown child operation {operation_id:?}")
            }
            Self::UnexpectedDuplicate { operation_id } => {
                write!(
                    formatter,
                    "duplicate completion arrived before terminal completion for {operation_id:?}"
                )
            }
            Self::Completion(error) => write!(formatter, "completion rejected: {error}"),
            Self::Admission(error) => write!(formatter, "admission rejected: {error}"),
            Self::ChildrenNotTerminal => write!(formatter, "child operations are not terminal"),
            Self::DrainIncomplete => write!(formatter, "required backend drain is incomplete"),
        }
    }
}

impl std::error::Error for SlotError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionHandling {
    Accepted,
    DuplicateIgnored,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildOperationSnapshot {
    pub operation_id: ChildOperationId,
    pub requested: ByteRange,
    pub terminal: bool,
    pub duplicate_deliveries: u32,
    pub completion: Option<StoreCompletion>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotSnapshot {
    pub token: OperationSlotToken,
    pub request: BlockRequest,
    pub state: SlotState,
    pub buffers: Vec<BufferToken>,
    pub frontend_tags: Vec<FrontendTag>,
    pub children: Vec<ChildOperationSnapshot>,
    pub submitted_watermark: Option<StoreWriteWatermark>,
    pub drain_state: DrainState,
    pub reconciliation: Option<ReconciliationOutcome>,
    pub terminal_evidence: Vec<StoreCompletion>,
    pub invalidated_by: Option<StoreInvalidationReason>,
    pub abandoned: bool,
    pub retry_count: usize,
}

#[derive(Clone, Debug)]
struct ChildRecord {
    operation_id: ChildOperationId,
    requested: ByteRange,
    terminal: bool,
    duplicate_deliveries: u32,
    completion: Option<StoreCompletion>,
}

#[derive(Clone, Debug)]
struct SlotRecord {
    token: OperationSlotToken,
    request: BlockRequest,
    state: SlotState,
    buffers: Vec<BufferToken>,
    frontend_tags: Vec<FrontendTag>,
    children: Vec<ChildRecord>,
    submitted_watermark: Option<StoreWriteWatermark>,
    drain_state: DrainState,
    reconciliation: Option<ReconciliationOutcome>,
    terminal_evidence: Vec<StoreCompletion>,
    invalidated_by: Option<StoreInvalidationReason>,
    abandoned: bool,
    retry_count: usize,
}

/// dwv:req req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
pub struct OperationSlotTable {
    slots: Vec<Option<SlotRecord>>,
    next_generations: Vec<u32>,
    admission: AdmissionController,
}

impl OperationSlotTable {
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            slots: (0..limits.operation_slots).map(|_| None).collect(),
            next_generations: vec![1; limits.operation_slots],
            admission: AdmissionController::new(limits),
        }
    }

    pub const fn limits(&self) -> ResourceLimits {
        self.admission.limits()
    }

    pub const fn usage(&self) -> ResourceUsage {
        self.admission.usage()
    }

    pub fn reserve(&mut self, request: BlockRequest) -> Result<OperationSlotToken, SlotError> {
        let index = self.slots.iter().position(Option::is_none).ok_or_else(|| {
            SlotError::Admission(AdmissionError::Exhausted {
                resource: ResourceKind::OperationSlots,
                limit: self.limits().operation_slots,
                in_use: self.usage().operation_slots,
            })
        })?;
        let generation = self.next_generations[index];
        let next = generation
            .checked_add(1)
            .ok_or(SlotError::GenerationExhausted {
                index: index as u32,
            })?;
        self.admission
            .try_acquire(ResourceKind::OperationSlots)
            .map_err(SlotError::Admission)?;
        if request.buffer.is_some()
            && let Err(error) = self.admission.try_acquire(ResourceKind::Buffers)
        {
            self.admission
                .release(ResourceKind::OperationSlots)
                .expect("operation slot acquisition must be balanced");
            return Err(SlotError::Admission(error));
        }
        self.next_generations[index] = next;
        let token = OperationSlotToken::new(index as u32, generation);
        self.slots[index] = Some(SlotRecord {
            token,
            request,
            state: SlotState::Reserved,
            buffers: request.buffer.into_iter().collect(),
            frontend_tags: Vec::new(),
            children: Vec::new(),
            submitted_watermark: None,
            drain_state: DrainState::NotRequired,
            reconciliation: None,
            terminal_evidence: Vec::new(),
            invalidated_by: None,
            abandoned: false,
            retry_count: 0,
        });
        Ok(token)
    }

    pub fn attach_frontend_tag(
        &mut self,
        token: OperationSlotToken,
        tag: FrontendTag,
    ) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        if slot.state == SlotState::Reclaimable {
            return Err(SlotError::InvalidState {
                token,
                state: slot.state,
            });
        }
        slot.frontend_tags.push(tag);
        Ok(())
    }

    pub fn register_child(
        &mut self,
        token: OperationSlotToken,
        requested: ByteRange,
    ) -> Result<ChildOperationId, SlotError> {
        checked_end(requested).map_err(SlotError::Completion)?;
        let index = self.active_index(token)?;
        let child_index = {
            let slot = self.slots[index].as_ref().expect("active index has a slot");
            if slot.state == SlotState::Reclaimable {
                return Err(SlotError::InvalidState {
                    token,
                    state: slot.state,
                });
            }
            u32::try_from(slot.children.len()).map_err(|_| {
                SlotError::Admission(AdmissionError::Exhausted {
                    resource: ResourceKind::BackendSubmissions,
                    limit: self.limits().backend_submissions,
                    in_use: self.usage().backend_submissions,
                })
            })?
        };
        self.admission
            .try_acquire(ResourceKind::BackendSubmissions)
            .map_err(SlotError::Admission)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        let operation_id = ChildOperationId {
            slot: token,
            index: child_index,
        };
        slot.children.push(ChildRecord {
            operation_id,
            requested,
            terminal: false,
            duplicate_deliveries: 0,
            completion: None,
        });
        Ok(operation_id)
    }

    pub fn mark_submitted(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        match slot.state {
            SlotState::Reserved => slot.state = SlotState::Submitted,
            SlotState::Submitted => {}
            state => {
                return Err(SlotError::InvalidState { token, state });
            }
        }
        Ok(())
    }

    pub fn record_submitted_watermark(
        &mut self,
        token: OperationSlotToken,
        watermark: StoreWriteWatermark,
    ) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        slot.submitted_watermark = Some(
            slot.submitted_watermark
                .map_or(watermark, |current| current.max(watermark)),
        );
        Ok(())
    }

    pub fn mark_abandoned(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        self.slots[index]
            .as_mut()
            .expect("active index has a slot")
            .abandoned = true;
        Ok(())
    }

    pub fn mark_drain_required(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        if slot.state == SlotState::Reclaimable {
            return Err(SlotError::InvalidState {
                token,
                state: slot.state,
            });
        }
        slot.drain_state = DrainState::Required;
        if slot.children.iter().all(|child| child.terminal) {
            slot.state = if slot.children.iter().any(|child| {
                child.completion.as_ref().is_some_and(|completion| {
                    completion.disposition == CompletionDisposition::Uncertain
                })
            }) {
                SlotState::CompletionUncertain
            } else {
                SlotState::Draining
            };
        }
        Ok(())
    }

    pub fn complete_drain(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        if slot.drain_state != DrainState::Required {
            return Err(SlotError::InvalidState {
                token,
                state: slot.state,
            });
        }
        slot.drain_state = DrainState::Complete;
        if slot.children.iter().all(|child| child.terminal) {
            slot.state = if slot.children.iter().any(|child| {
                child.completion.as_ref().is_some_and(|completion| {
                    completion.disposition == CompletionDisposition::Uncertain
                })
            }) {
                SlotState::CompletionUncertain
            } else {
                SlotState::ReconciliationRequired
            };
        }
        Ok(())
    }

    pub fn apply_completion(
        &mut self,
        token: OperationSlotToken,
        completion: StoreCompletion,
    ) -> Result<CompletionHandling, SlotError> {
        let index = self.active_index(token)?;
        if completion.operation_id.slot != token {
            return Err(SlotError::StaleGeneration { token });
        }
        let child_index = usize::try_from(completion.operation_id.index).map_err(|_| {
            SlotError::ChildUnknown {
                operation_id: completion.operation_id,
            }
        })?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        let child = slot
            .children
            .get_mut(child_index)
            .ok_or(SlotError::ChildUnknown {
                operation_id: completion.operation_id,
            })?;
        if child.operation_id != completion.operation_id {
            return Err(SlotError::ChildUnknown {
                operation_id: completion.operation_id,
            });
        }
        completion.validate().map_err(SlotError::Completion)?;
        if completion.requested != child.requested {
            return Err(SlotError::Completion(StoreError::InvalidCompletion(
                CompletionError::CompletedRangeOutsideRequest,
            )));
        }
        if child.terminal {
            child.duplicate_deliveries = child.duplicate_deliveries.saturating_add(1);
            return Ok(CompletionHandling::DuplicateIgnored);
        }
        if completion.disposition == CompletionDisposition::Duplicate {
            return Err(SlotError::UnexpectedDuplicate {
                operation_id: completion.operation_id,
            });
        }

        child.terminal = true;
        child.completion = Some(completion.clone());
        slot.terminal_evidence.push(completion);
        let all_terminal = slot.children.iter().all(|child| child.terminal);
        if all_terminal {
            let uncertain = slot.children.iter().any(|child| {
                child.completion.as_ref().is_some_and(|completion| {
                    completion.disposition == CompletionDisposition::Uncertain
                })
            });
            slot.state = if uncertain {
                SlotState::CompletionUncertain
            } else if slot.drain_state == DrainState::Required {
                SlotState::Draining
            } else {
                SlotState::ReconciliationRequired
            };
        } else {
            slot.state = SlotState::PartiallyCompleted;
        }
        Ok(CompletionHandling::Accepted)
    }

    pub fn record_reconciliation(
        &mut self,
        token: OperationSlotToken,
        outcome: ReconciliationOutcome,
    ) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        if !slot.children.iter().all(|child| child.terminal) {
            return Err(SlotError::ChildrenNotTerminal);
        }
        if slot.drain_state == DrainState::Required {
            return Err(SlotError::DrainIncomplete);
        }
        slot.reconciliation = Some(outcome);
        slot.state = SlotState::Reclaimable;
        Ok(())
    }

    pub fn invalidate(
        &mut self,
        token: OperationSlotToken,
        reason: StoreInvalidationReason,
    ) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_mut().expect("active index has a slot");
        slot.invalidated_by = Some(reason);
        slot.reconciliation = None;
        slot.state = SlotState::CompletionUncertain;
        Ok(())
    }

    pub fn record_retry(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        self.admission
            .try_acquire(ResourceKind::Retries)
            .map_err(SlotError::Admission)?;
        self.slots[index]
            .as_mut()
            .expect("active index has a slot")
            .retry_count += 1;
        Ok(())
    }

    pub fn snapshot(&self, token: OperationSlotToken) -> Result<SlotSnapshot, SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_ref().expect("active index has a slot");
        Ok(SlotSnapshot {
            token: slot.token,
            request: slot.request,
            state: slot.state,
            buffers: slot.buffers.clone(),
            frontend_tags: slot.frontend_tags.clone(),
            children: slot
                .children
                .iter()
                .map(|child| ChildOperationSnapshot {
                    operation_id: child.operation_id,
                    requested: child.requested,
                    terminal: child.terminal,
                    duplicate_deliveries: child.duplicate_deliveries,
                    completion: child.completion.clone(),
                })
                .collect(),
            submitted_watermark: slot.submitted_watermark,
            drain_state: slot.drain_state,
            reconciliation: slot.reconciliation,
            terminal_evidence: slot.terminal_evidence.clone(),
            invalidated_by: slot.invalidated_by,
            abandoned: slot.abandoned,
            retry_count: slot.retry_count,
        })
    }

    pub fn release(&mut self, token: OperationSlotToken) -> Result<(), SlotError> {
        let index = self.active_index(token)?;
        let slot = self.slots[index].as_ref().expect("active index has a slot");
        if slot.state != SlotState::Reclaimable {
            return Err(SlotError::InvalidState {
                token,
                state: slot.state,
            });
        }
        let slot = self.slots[index].take().expect("active index has a slot");
        self.admission
            .release(ResourceKind::OperationSlots)
            .map_err(SlotError::Admission)?;
        for _ in slot.buffers {
            self.admission
                .release(ResourceKind::Buffers)
                .map_err(SlotError::Admission)?;
        }
        for _ in slot.children {
            self.admission
                .release(ResourceKind::BackendSubmissions)
                .map_err(SlotError::Admission)?;
        }
        for _ in 0..slot.retry_count {
            self.admission
                .release(ResourceKind::Retries)
                .map_err(SlotError::Admission)?;
        }
        Ok(())
    }

    fn active_index(&self, token: OperationSlotToken) -> Result<usize, SlotError> {
        let index = usize::try_from(token.index).ok();
        let Some(index) = index else {
            return Err(SlotError::StaleGeneration { token });
        };
        let Some(slot) = self.slots.get(index).and_then(Option::as_ref) else {
            return Err(SlotError::StaleGeneration { token });
        };
        if slot.token != token {
            return Err(SlotError::StaleGeneration { token });
        }
        Ok(index)
    }
}
