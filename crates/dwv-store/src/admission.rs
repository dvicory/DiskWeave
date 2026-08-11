use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    OperationSlots,
    Buffers,
    BackendSubmissions,
    Retries,
    RangeLocks,
    BackgroundWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    pub operation_slots: usize,
    pub buffers: usize,
    pub backend_submissions: usize,
    pub retries: usize,
    pub range_locks: usize,
    pub background_work: usize,
}

impl ResourceLimits {
    pub const fn new(
        operation_slots: usize,
        buffers: usize,
        backend_submissions: usize,
        retries: usize,
        range_locks: usize,
        background_work: usize,
    ) -> Self {
        Self {
            operation_slots,
            buffers,
            backend_submissions,
            retries,
            range_locks,
            background_work,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceUsage {
    pub operation_slots: usize,
    pub buffers: usize,
    pub backend_submissions: usize,
    pub retries: usize,
    pub range_locks: usize,
    pub background_work: usize,
}

impl ResourceUsage {
    pub(crate) const fn empty() -> Self {
        Self {
            operation_slots: 0,
            buffers: 0,
            backend_submissions: 0,
            retries: 0,
            range_locks: 0,
            background_work: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionError {
    Exhausted {
        resource: ResourceKind,
        limit: usize,
        in_use: usize,
    },
    ReleaseUnderflow {
        resource: ResourceKind,
        in_use: usize,
        requested: usize,
    },
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted {
                resource,
                limit,
                in_use,
            } => write!(
                formatter,
                "{resource:?} bound {limit} reached with {in_use} in use"
            ),
            Self::ReleaseUnderflow {
                resource,
                in_use,
                requested,
            } => write!(
                formatter,
                "cannot release {requested} {resource:?} with {in_use} in use"
            ),
        }
    }
}

impl std::error::Error for AdmissionError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmissionController {
    limits: ResourceLimits,
    usage: ResourceUsage,
}

impl AdmissionController {
    pub const fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            usage: ResourceUsage::empty(),
        }
    }

    pub const fn limits(self) -> ResourceLimits {
        self.limits
    }

    pub const fn usage(self) -> ResourceUsage {
        self.usage
    }

    pub fn try_acquire(&mut self, resource: ResourceKind) -> Result<(), AdmissionError> {
        let (usage, limit) = self.usage_limit(resource);
        if *usage >= limit {
            return Err(AdmissionError::Exhausted {
                resource,
                limit,
                in_use: *usage,
            });
        }
        *usage += 1;
        Ok(())
    }

    pub fn release(&mut self, resource: ResourceKind) -> Result<(), AdmissionError> {
        let (usage, _) = self.usage_limit(resource);
        if *usage == 0 {
            return Err(AdmissionError::ReleaseUnderflow {
                resource,
                in_use: 0,
                requested: 1,
            });
        }
        *usage -= 1;
        Ok(())
    }

    fn usage_limit(&mut self, resource: ResourceKind) -> (&mut usize, usize) {
        match resource {
            ResourceKind::OperationSlots => {
                (&mut self.usage.operation_slots, self.limits.operation_slots)
            }
            ResourceKind::Buffers => (&mut self.usage.buffers, self.limits.buffers),
            ResourceKind::BackendSubmissions => (
                &mut self.usage.backend_submissions,
                self.limits.backend_submissions,
            ),
            ResourceKind::Retries => (&mut self.usage.retries, self.limits.retries),
            ResourceKind::RangeLocks => (&mut self.usage.range_locks, self.limits.range_locks),
            ResourceKind::BackgroundWork => {
                (&mut self.usage.background_work, self.limits.background_work)
            }
        }
    }
}
