//! Checked recovery-generation primitives shared by recovery transitions.

use std::fmt;

/// Monotonic generation of the durable recovery state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecoveryGeneration(pub u64);

impl RecoveryGeneration {
    pub const ZERO: Self = Self(0);

    pub const fn checked_add(self, amount: u64) -> Option<Self> {
        match self.0.checked_add(amount) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn checked_next(self) -> Option<Self> {
        self.checked_add(1)
    }
}

impl fmt::Display for RecoveryGeneration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationCapture {
    pub topology_epoch: dwv_core::TopologyEpoch,
    pub recovery_generation: RecoveryGeneration,
}

impl GenerationCapture {
    pub const fn new(
        topology_epoch: dwv_core::TopologyEpoch,
        recovery_generation: RecoveryGeneration,
    ) -> Self {
        Self {
            topology_epoch,
            recovery_generation,
        }
    }

    pub fn matches(self, other: Self) -> bool {
        self.topology_epoch == other.topology_epoch
            && self.recovery_generation == other.recovery_generation
    }
}
