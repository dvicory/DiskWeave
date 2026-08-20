//! Small deterministic model for OS-010 cut points and overlap schedules.
//!
//! This is intentionally not a device model.  It records only the semantic
//! ordering boundary needed to prove that dirty evidence survives interruption.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryCutPoint {
    DataParityWriteBeforeWriteRecoveryRecord,
    AfterWriteRecoveryRecordBeforeDataParityWrite,
    AfterDataParityWriteBeforeFence,
    AfterPersistenceEvidenceBeforeRecoveryClean,
    AfterRecoveryClean,
    RestartAfterCrash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryWrite {
    pub transaction_id: u64,
    pub regions: Vec<u64>,
    pub checksum_extents: Vec<u64>,
}

impl RecoveryWrite {
    pub fn new(transaction_id: u64, regions: Vec<u64>, checksum_extents: Vec<u64>) -> Self {
        Self {
            transaction_id,
            regions: sorted_unique(regions),
            checksum_extents: sorted_unique(checksum_extents),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverySchedule {
    pub writes: Vec<RecoveryWrite>,
    pub cut_point: RecoveryCutPoint,
}

impl RecoverySchedule {
    pub fn new(mut writes: Vec<RecoveryWrite>, cut_point: RecoveryCutPoint) -> Self {
        writes.sort_by_key(|write| write.transaction_id);
        Self { writes, cut_point }
    }

    pub fn first_write(cut_point: RecoveryCutPoint) -> Self {
        Self::new(vec![RecoveryWrite::new(1, vec![1], vec![10])], cut_point)
    }

    pub fn overlapping_writes(cut_point: RecoveryCutPoint) -> Self {
        Self::new(
            vec![
                RecoveryWrite::new(2, vec![2, 3], vec![20, 21]),
                RecoveryWrite::new(1, vec![1, 2], vec![10, 20]),
            ],
            cut_point,
        )
    }

    pub fn replay(&self) -> RecoveryModelState {
        let mut state = RecoveryModelState::default();
        let regions = self
            .writes
            .iter()
            .flat_map(|write| write.regions.iter().copied())
            .collect();
        let extents = self
            .writes
            .iter()
            .flat_map(|write| write.checksum_extents.iter().copied())
            .collect();
        let regions = sorted_unique(regions);
        let extents = sorted_unique(extents);

        match self.cut_point {
            RecoveryCutPoint::DataParityWriteBeforeWriteRecoveryRecord => {
                state.outcome = Some(RecoveryModelOutcome::Blocked);
                state
                    .events
                    .push(RecoveryModelEvent::WriteRecoveryRecordRefused);
            }
            RecoveryCutPoint::AfterWriteRecoveryRecordBeforeDataParityWrite => {
                state.mark_write_recovery_record(regions, extents);
                state.outcome = Some(RecoveryModelOutcome::Interrupted);
            }
            RecoveryCutPoint::AfterDataParityWriteBeforeFence => {
                state.mark_write_recovery_record(regions, extents);
                state.data_parity_write_emitted = true;
                state.outcome = Some(RecoveryModelOutcome::ReconciliationRequired);
            }
            RecoveryCutPoint::AfterPersistenceEvidenceBeforeRecoveryClean => {
                state.mark_write_recovery_record(regions, extents);
                state.data_parity_write_emitted = true;
                state.fence_durable = true;
                state.outcome = Some(RecoveryModelOutcome::Interrupted);
            }
            RecoveryCutPoint::AfterRecoveryClean => {
                state.mark_write_recovery_record(regions, extents);
                state.data_parity_write_emitted = true;
                state.fence_durable = true;
                state.recovery_clean = true;
                state.dirty_regions.clear();
                state.session_dirty = false;
                state.outcome = Some(RecoveryModelOutcome::RecoveryCleanWithStaleIntegrity);
            }
            RecoveryCutPoint::RestartAfterCrash => {
                state.mark_write_recovery_record(regions, extents);
                state.data_parity_write_emitted = true;
                state.outcome = Some(RecoveryModelOutcome::ReconciliationRequired);
            }
        }
        state.events.push(RecoveryModelEvent::StateDigest {
            dirty_regions: state.dirty_regions.clone(),
            stale_extents: state.stale_extents.clone(),
            session_dirty: state.session_dirty,
        });
        state
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryModelOutcome {
    Blocked,
    Interrupted,
    ReconciliationRequired,
    RecoveryCleanWithStaleIntegrity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryModelEvent {
    WriteRecoveryRecordRefused,
    WriteRecoveryRecordDurable,
    DataParityWriteEmitted,
    FenceDurable,
    RecoveryCleanDurable,
    StateDigest {
        dirty_regions: Vec<u64>,
        stale_extents: Vec<u64>,
        session_dirty: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct RecoveryModelState {
    pub dirty_regions: Vec<u64>,
    pub stale_extents: Vec<u64>,
    pub session_dirty: bool,
    pub data_parity_write_emitted: bool,
    pub fence_durable: bool,
    pub recovery_clean: bool,
    pub outcome: Option<RecoveryModelOutcome>,
    pub events: Vec<RecoveryModelEvent>,
}

impl RecoveryModelState {
    fn mark_write_recovery_record(&mut self, regions: Vec<u64>, extents: Vec<u64>) {
        self.dirty_regions = regions;
        self.stale_extents = extents;
        self.session_dirty = true;
        self.events
            .push(RecoveryModelEvent::WriteRecoveryRecordDurable);
    }
}

fn sorted_unique(mut values: Vec<u64>) -> Vec<u64> {
    values.sort_unstable();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pre_recovery_clean_cut_point_preserves_dirty_evidence() {
        for cut_point in [
            RecoveryCutPoint::AfterWriteRecoveryRecordBeforeDataParityWrite,
            RecoveryCutPoint::AfterDataParityWriteBeforeFence,
            RecoveryCutPoint::AfterPersistenceEvidenceBeforeRecoveryClean,
            RecoveryCutPoint::RestartAfterCrash,
        ] {
            let state = RecoverySchedule::first_write(cut_point).replay();
            assert_eq!(state.dirty_regions, vec![1]);
            assert_eq!(state.stale_extents, vec![10]);
            assert!(state.session_dirty);
        }
    }

    #[test]
    fn data_parity_write_before_durable_write_recovery_record_is_refused_without_io() {
        let state = RecoverySchedule::first_write(
            RecoveryCutPoint::DataParityWriteBeforeWriteRecoveryRecord,
        )
        .replay();
        assert!(!state.data_parity_write_emitted);
        assert!(state.dirty_regions.is_empty());
        assert_eq!(state.outcome, Some(RecoveryModelOutcome::Blocked));
        assert!(
            state
                .events
                .contains(&RecoveryModelEvent::WriteRecoveryRecordRefused)
        );
    }

    #[test]
    fn crash_after_data_parity_write_before_recovery_clean_remains_dirty() {
        let state =
            RecoverySchedule::first_write(RecoveryCutPoint::AfterDataParityWriteBeforeFence)
                .replay();
        assert!(state.data_parity_write_emitted);
        assert!(!state.recovery_clean);
        assert_eq!(state.dirty_regions, vec![1]);
        assert_eq!(
            state.outcome,
            Some(RecoveryModelOutcome::ReconciliationRequired)
        );
    }

    #[test]
    fn overlap_schedule_is_sorted_and_unioned_deterministically() {
        let schedule = RecoverySchedule::overlapping_writes(
            RecoveryCutPoint::AfterWriteRecoveryRecordBeforeDataParityWrite,
        );
        let state = schedule.replay();
        assert_eq!(state.dirty_regions, vec![1, 2, 3]);
        assert_eq!(state.stale_extents, vec![10, 20, 21]);
        assert_eq!(state, schedule.replay());
    }

    #[test]
    fn recovery_clean_does_not_infer_checksum_validity() {
        let state = RecoverySchedule::first_write(RecoveryCutPoint::AfterRecoveryClean).replay();
        assert!(state.dirty_regions.is_empty());
        assert_eq!(state.stale_extents, vec![10]);
        assert_eq!(
            state.outcome,
            Some(RecoveryModelOutcome::RecoveryCleanWithStaleIntegrity)
        );
    }
}
