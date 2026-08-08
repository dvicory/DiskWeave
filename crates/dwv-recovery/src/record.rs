//! Generational checksum records and conservative validity states.

use crate::{
    ChecksumExtent, ChecksumProfileId, ChecksumSetGeneration, RecoveryGeneration, StoreFenceRef,
};

pub type Digest = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContentGeneration(pub RecoveryGeneration);

impl ContentGeneration {
    pub const ZERO: Self = Self(RecoveryGeneration::ZERO);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumState {
    Absent,
    Stale,
    Valid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FenceEvidence {
    pub fence: StoreFenceRef,
    pub topology_epoch: dwv_core::TopologyEpoch,
    pub recovery_generation: RecoveryGeneration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChecksumRecord {
    pub extent: ChecksumExtent,
    pub profile: ChecksumProfileId,
    pub set_generation: ChecksumSetGeneration,
    pub content_generation: ContentGeneration,
    pub state: ChecksumState,
    pub digest: Option<Digest>,
    pub fence: Option<FenceEvidence>,
}

impl ChecksumRecord {
    pub fn absent(
        extent: ChecksumExtent,
        profile: ChecksumProfileId,
        set_generation: ChecksumSetGeneration,
    ) -> Self {
        Self {
            extent,
            profile,
            set_generation,
            content_generation: ContentGeneration::ZERO,
            state: ChecksumState::Absent,
            digest: None,
            fence: None,
        }
    }

    pub fn stale_from(&self, generation: ContentGeneration) -> Self {
        let mut next = self.clone();
        next.content_generation = ContentGeneration(RecoveryGeneration(
            self.content_generation.0.0.max(generation.0.0),
        ));
        next.state = ChecksumState::Stale;
        next.digest = None;
        next.fence = None;
        next
    }

    pub fn valid(
        extent: ChecksumExtent,
        profile: ChecksumProfileId,
        set_generation: ChecksumSetGeneration,
        content_generation: ContentGeneration,
        digest: Digest,
        fence: FenceEvidence,
    ) -> Self {
        Self {
            extent,
            profile,
            set_generation,
            content_generation,
            state: ChecksumState::Valid,
            digest: Some(digest),
            fence: Some(fence),
        }
    }

    pub const fn is_valid(&self) -> bool {
        matches!(self.state, ChecksumState::Valid) && self.digest.is_some() && self.fence.is_some()
    }
}
