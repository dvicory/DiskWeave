//! Parallel checksum-set migration with conservative interruption behavior.

use crate::{
    ChecksumExtent, ChecksumProfile, ChecksumRecord, ChecksumSet, ChecksumSetState, ChecksumState,
};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileMigration {
    pub active: ChecksumSet,
    pub building: Option<ChecksumSet>,
    required: BTreeMap<crate::IntegrityExtentId, ChecksumExtent>,
    valid: BTreeMap<crate::IntegrityExtentId, ChecksumRecord>,
}

impl ProfileMigration {
    pub fn begin(
        active: ChecksumSet,
        profile: ChecksumProfile,
        required: Vec<ChecksumExtent>,
    ) -> Result<Self, MigrationError> {
        let generation = active
            .generation
            .next()
            .ok_or(MigrationError::GenerationExhausted)?;
        let building = ChecksumSet::new(generation, profile).map_err(MigrationError::Profile)?;
        Ok(Self {
            active,
            building: Some(building),
            required: required
                .into_iter()
                .map(|extent| (extent.id, extent))
                .collect(),
            valid: BTreeMap::new(),
        })
    }

    pub fn add_valid(&mut self, record: ChecksumRecord) -> Result<(), MigrationError> {
        let building = self
            .building
            .as_ref()
            .ok_or(MigrationError::NoBuildInProgress)?;
        if building.state != ChecksumSetState::Building
            || record.set_generation != building.generation
            || record.state != ChecksumState::Valid
            || !record.is_valid()
            || record.profile != building.profile.id
        {
            return Err(MigrationError::RecordNotForBuildingSet);
        }
        if self.required.get(&record.extent.id) != Some(&record.extent) {
            return Err(MigrationError::UnexpectedExtent);
        }
        self.valid.insert(record.extent.id, record);
        Ok(())
    }

    pub fn ready(&self) -> bool {
        self.building
            .as_ref()
            .is_some_and(|set| set.state == ChecksumSetState::Building)
            && self.valid.len() == self.required.len()
            && self.required.keys().all(|id| self.valid.contains_key(id))
    }

    pub fn switch(mut self) -> Result<(ChecksumSet, Vec<ChecksumRecord>), MigrationError> {
        if !self.ready() {
            return Err(MigrationError::Incomplete);
        }
        let mut active = self.building.take().expect("ready checked");
        active.state = ChecksumSetState::Active;
        Ok((active, self.valid.into_values().collect()))
    }

    pub fn interrupt(&mut self) {
        if let Some(building) = self.building.as_mut() {
            building.state = ChecksumSetState::Interrupted;
        }
    }

    pub fn restart(&self) -> &ChecksumSet {
        &self.active
    }

    pub fn valid_records(&self) -> impl Iterator<Item = &ChecksumRecord> {
        self.valid.values()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationError {
    GenerationExhausted,
    Profile(crate::ProfileError),
    NoBuildInProgress,
    RecordNotForBuildingSet,
    UnexpectedExtent,
    Incomplete,
    ActiveSetNotReady,
    RecordNotForActiveSet,
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "checksum migration failed: {self:?}")
    }
}

impl std::error::Error for MigrationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationOutcome {
    Active,
    Building,
    Interrupted,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BLAKE3_256_PROFILE, ChecksumProfileId, ChecksumSetGeneration, ChecksumTarget,
        ContentGeneration, FenceEvidence, IntegrityExtentId,
    };
    use dwv_core::{ByteRange, SlotId};

    fn extent(id: u64) -> ChecksumExtent {
        ChecksumExtent::new(
            IntegrityExtentId(id),
            ChecksumTarget::data(SlotId([1; 16])),
            ByteRange::new(id * 4, 4).unwrap(),
            4,
        )
        .unwrap()
    }

    fn valid(extent: ChecksumExtent, set: ChecksumSetGeneration) -> ChecksumRecord {
        ChecksumRecord::valid(
            extent,
            ChecksumProfileId(1),
            set,
            ContentGeneration::ZERO,
            [extent.id.0 as u8; 32],
            FenceEvidence {
                fence: dwv_store::StoreFenceRef {
                    fence_id: dwv_store::FenceId(1),
                    store_id: dwv_store::StoreId(1),
                    store_incarnation: dwv_store::StoreIncarnationId(0),
                    topology_epoch: dwv_core::TopologyEpoch(1),
                    through: dwv_store::StoreWriteWatermark(1),
                    capability_evidence_id: dwv_store::CapabilityEvidenceId(1),
                },
                topology_epoch: dwv_core::TopologyEpoch(1),
                recovery_generation: crate::RecoveryGeneration::ZERO,
            },
        )
    }

    #[test]
    fn interrupted_migration_keeps_old_set_interpretable() {
        let active = ChecksumSet {
            generation: ChecksumSetGeneration::INITIAL,
            profile: BLAKE3_256_PROFILE,
            state: ChecksumSetState::Active,
        };
        let mut migration = ProfileMigration::begin(
            active.clone(),
            BLAKE3_256_PROFILE,
            vec![extent(1), extent(2)],
        )
        .unwrap();
        migration
            .add_valid(valid(extent(1), ChecksumSetGeneration(2)))
            .unwrap();
        migration.interrupt();
        assert_eq!(migration.restart(), &active);
        assert_eq!(
            migration.add_valid(valid(extent(2), ChecksumSetGeneration(2))),
            Err(MigrationError::RecordNotForBuildingSet)
        );
        assert_eq!(
            migration.building.unwrap().state,
            ChecksumSetState::Interrupted
        );
    }

    #[test]
    fn switch_requires_every_extent() {
        let active = ChecksumSet {
            generation: ChecksumSetGeneration::INITIAL,
            profile: BLAKE3_256_PROFILE,
            state: ChecksumSetState::Active,
        };
        let mut migration =
            ProfileMigration::begin(active, BLAKE3_256_PROFILE, vec![extent(1)]).unwrap();
        assert_eq!(migration.clone().switch(), Err(MigrationError::Incomplete));
        migration
            .add_valid(valid(extent(1), ChecksumSetGeneration(2)))
            .unwrap();
        let (set, records) = migration.switch().unwrap();
        assert_eq!(set.state, ChecksumSetState::Active);
        assert_eq!(records.len(), 1);
    }
}
