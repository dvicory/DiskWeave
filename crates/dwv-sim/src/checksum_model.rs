//! Deterministic checksum race and crash schedules for OS-011.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumCutPoint {
    BeforeRead,
    AfterReadBeforeFence,
    AfterFenceBeforeCommit,
    AfterInvalidation,
    AfterMigrationRecord,
    AfterMigrationSwitch,
    PowerLoss,
    Restart,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumModelOutcome {
    Absent,
    Stale,
    Valid,
    OldSetStillActive,
    NewSetActive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChecksumModelEvent {
    ReadCaptured { generation: u64 },
    FenceCaptured { generation: u64 },
    Invalidated { generation: u64 },
    ResultRejected,
    ValidCommitted { generation: u64 },
    MigrationBuilding,
    MigrationInterrupted,
    MigrationSwitched,
    PowerLoss,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChecksumModelSchedule {
    pub cut_point: ChecksumCutPoint,
    pub writer_invalidates: bool,
    pub migration_has_all_records: bool,
}

impl ChecksumModelSchedule {
    pub const fn race(cut_point: ChecksumCutPoint) -> Self {
        Self {
            cut_point,
            writer_invalidates: true,
            migration_has_all_records: false,
        }
    }

    pub const fn migration(cut_point: ChecksumCutPoint, has_all_records: bool) -> Self {
        Self {
            cut_point,
            writer_invalidates: false,
            migration_has_all_records: has_all_records,
        }
    }

    pub fn replay(&self) -> ChecksumModelState {
        let mut state = ChecksumModelState::default();
        state
            .events
            .push(ChecksumModelEvent::ReadCaptured { generation: 0 });
        match self.cut_point {
            ChecksumCutPoint::BeforeRead => {
                state.outcome = Some(ChecksumModelOutcome::Absent);
            }
            ChecksumCutPoint::AfterReadBeforeFence => {
                state.events.push(ChecksumModelEvent::ResultRejected);
                state.outcome = Some(ChecksumModelOutcome::Stale);
            }
            ChecksumCutPoint::AfterFenceBeforeCommit => {
                state
                    .events
                    .push(ChecksumModelEvent::FenceCaptured { generation: 0 });
                if self.writer_invalidates {
                    state
                        .events
                        .push(ChecksumModelEvent::Invalidated { generation: 1 });
                    state.events.push(ChecksumModelEvent::ResultRejected);
                    state.outcome = Some(ChecksumModelOutcome::Stale);
                } else {
                    state
                        .events
                        .push(ChecksumModelEvent::ValidCommitted { generation: 0 });
                    state.outcome = Some(ChecksumModelOutcome::Valid);
                }
            }
            ChecksumCutPoint::AfterInvalidation | ChecksumCutPoint::Restart => {
                state
                    .events
                    .push(ChecksumModelEvent::Invalidated { generation: 1 });
                state.outcome = Some(ChecksumModelOutcome::Stale);
            }
            ChecksumCutPoint::PowerLoss => {
                state.events.push(ChecksumModelEvent::PowerLoss);
                state
                    .events
                    .push(ChecksumModelEvent::Invalidated { generation: 1 });
                state.outcome = Some(ChecksumModelOutcome::Stale);
            }
            ChecksumCutPoint::AfterMigrationRecord => {
                state.events.push(ChecksumModelEvent::MigrationBuilding);
                state.events.push(ChecksumModelEvent::MigrationInterrupted);
                state.outcome = Some(ChecksumModelOutcome::OldSetStillActive);
            }
            ChecksumCutPoint::AfterMigrationSwitch => {
                state.events.push(ChecksumModelEvent::MigrationBuilding);
                if self.migration_has_all_records {
                    state.events.push(ChecksumModelEvent::MigrationSwitched);
                    state.outcome = Some(ChecksumModelOutcome::NewSetActive);
                } else {
                    state.events.push(ChecksumModelEvent::MigrationInterrupted);
                    state.outcome = Some(ChecksumModelOutcome::OldSetStillActive);
                }
            }
        }
        state
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ChecksumModelState {
    pub outcome: Option<ChecksumModelOutcome>,
    pub events: Vec<ChecksumModelEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalidation_race_never_leaves_a_false_valid() {
        let state = ChecksumModelSchedule::race(ChecksumCutPoint::AfterFenceBeforeCommit).replay();
        assert_eq!(state.outcome, Some(ChecksumModelOutcome::Stale));
        assert!(state.events.contains(&ChecksumModelEvent::ResultRejected));
    }

    #[test]
    fn every_invalidation_cut_point_is_conservative() {
        for cut_point in [
            ChecksumCutPoint::BeforeRead,
            ChecksumCutPoint::AfterReadBeforeFence,
            ChecksumCutPoint::AfterFenceBeforeCommit,
            ChecksumCutPoint::AfterInvalidation,
            ChecksumCutPoint::PowerLoss,
            ChecksumCutPoint::Restart,
        ] {
            let state = ChecksumModelSchedule::race(cut_point).replay();
            assert_ne!(state.outcome, Some(ChecksumModelOutcome::Valid));
        }
    }

    #[test]
    fn incomplete_migration_keeps_old_set_active_after_restart() {
        let state = ChecksumModelSchedule::migration(ChecksumCutPoint::AfterMigrationSwitch, false)
            .replay();
        assert_eq!(state.outcome, Some(ChecksumModelOutcome::OldSetStillActive));
    }

    #[test]
    fn complete_migration_can_switch_once() {
        let state =
            ChecksumModelSchedule::migration(ChecksumCutPoint::AfterMigrationSwitch, true).replay();
        assert_eq!(state.outcome, Some(ChecksumModelOutcome::NewSetActive));
    }
}
