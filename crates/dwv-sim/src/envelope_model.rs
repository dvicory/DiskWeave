use dwv_format::{
    CopyAssessment, DIRTY_REGION_BYTES, EnvelopeRecord, EvidenceState, FormatError, HEADER_BYTES,
    MigrationDecision, MigrationState, Profile, ProfileConfig, ProfileLayout, SessionState,
    assess_copies, encode_copy, select_migrated_profile,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvelopeCutPoint {
    MarkDirty,
    ProtectedMutation,
    DurableFence,
    RecoveryCheckpoint,
    PublishClean,
    BeginMigration { target: Profile },
    CompleteMigration,
    Crash,
    TornCopyA,
    TornCopyB,
    DisagreeGeneration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvelopeSchedule {
    steps: Vec<EnvelopeCutPoint>,
}

impl EnvelopeSchedule {
    pub fn new(steps: Vec<EnvelopeCutPoint>) -> Self {
        Self { steps }
    }

    pub fn push(&mut self, step: EnvelopeCutPoint) {
        self.steps.push(step);
    }

    pub fn as_slice(&self) -> &[EnvelopeCutPoint] {
        &self.steps
    }
}

pub struct EnvelopeModelOutcome {
    pub profile: Profile,
    pub assessment: CopyAssessment,
    pub migration: Option<MigrationDecision>,
    pub metadata_writes: u64,
    pub protected_writes: u64,
    pub verification_bytes: u64,
    pub clean_publish_blocked: bool,
    pub crashed: bool,
}

pub fn run_envelope_schedule(
    profile: Profile,
    parity_extent_length: u64,
    protected_data_length: u64,
    schedule: &EnvelopeSchedule,
) -> Result<EnvelopeModelOutcome, FormatError> {
    let layout =
        ProfileConfig::new(profile, parity_extent_length, protected_data_length).layout()?;
    if profile == Profile::BareParity {
        return Ok(EnvelopeModelOutcome {
            profile,
            assessment: assess_copies(None, None, Some(1)),
            migration: None,
            metadata_writes: 0,
            protected_writes: schedule
                .as_slice()
                .iter()
                .filter(|step| matches!(step, EnvelopeCutPoint::ProtectedMutation))
                .count() as u64,
            verification_bytes: protected_data_length,
            clean_publish_blocked: false,
            crashed: schedule
                .as_slice()
                .iter()
                .any(|step| matches!(step, EnvelopeCutPoint::Crash)),
        });
    }

    let mut first = base_record(profile, &layout, protected_data_length, 0);
    let mut second = base_record(profile, &layout, protected_data_length, 1);
    let mut copy_a = encode_copy(&first)?;
    let mut copy_b = encode_copy(&second)?;
    let mut metadata_writes = 0;
    let mut protected_writes = 0;
    let mut fenced = false;
    let mut checkpointed = false;
    let mut clean_publish_blocked = false;
    let mut crashed = false;
    let mut migration_target = None;

    for step in schedule.as_slice() {
        match step {
            EnvelopeCutPoint::MarkDirty => {
                first.session_state = SessionState::Dirty;
                second.session_state = SessionState::Dirty;
                first.session_generation += 1;
                second.session_generation += 1;
                first.copy_generation += 1;
                second.copy_generation += 1;
                copy_a = encode_copy(&first)?;
                copy_b = encode_copy(&second)?;
                metadata_writes += 2;
            }
            EnvelopeCutPoint::ProtectedMutation => protected_writes += 1,
            EnvelopeCutPoint::DurableFence => fenced = true,
            EnvelopeCutPoint::RecoveryCheckpoint => checkpointed = true,
            EnvelopeCutPoint::PublishClean => {
                if fenced && checkpointed {
                    first.session_state = SessionState::Clean;
                    second.session_state = SessionState::Clean;
                    first.copy_generation += 1;
                    second.copy_generation += 1;
                    copy_a = encode_copy(&first)?;
                    copy_b = encode_copy(&second)?;
                    metadata_writes += 2;
                } else {
                    clean_publish_blocked = true;
                }
            }
            EnvelopeCutPoint::BeginMigration { target } => {
                migration_target = Some(*target);
                first.migration_state = MigrationState::Preparing;
                second.migration_state = MigrationState::Preparing;
                copy_a = encode_copy(&first)?;
                copy_b = encode_copy(&second)?;
                metadata_writes += 2;
            }
            EnvelopeCutPoint::CompleteMigration => {
                if migration_target.is_some() {
                    first.migration_state = MigrationState::Complete;
                    second.migration_state = MigrationState::Complete;
                    first.copy_generation += 1;
                    second.copy_generation += 1;
                    copy_a = encode_copy(&first)?;
                    copy_b = encode_copy(&second)?;
                    metadata_writes += 2;
                }
            }
            EnvelopeCutPoint::Crash => {
                crashed = true;
                break;
            }
            EnvelopeCutPoint::TornCopyA => copy_a[..HEADER_BYTES / 2].fill(0),
            EnvelopeCutPoint::TornCopyB => copy_b[..HEADER_BYTES / 2].fill(0),
            EnvelopeCutPoint::DisagreeGeneration => {
                second.session_generation += 1;
                copy_b = encode_copy(&second)?;
            }
        }
    }

    let assessment = assess_copies(Some(&copy_a), Some(&copy_b), Some(1));
    let verification_bytes = match (profile, assessment.state) {
        (_, EvidenceState::Clean) => 0,
        (Profile::EnvelopeBitmap, EvidenceState::Dirty) => {
            dirty_bitmap_bytes(&first, protected_data_length)
        }
        (_, _) => protected_data_length,
    };
    let migration = migration_target
        .map(|target| select_migrated_profile(profile, target, Some(&copy_a), Some(&copy_b)));
    Ok(EnvelopeModelOutcome {
        profile,
        assessment,
        migration,
        metadata_writes,
        protected_writes,
        verification_bytes,
        clean_publish_blocked,
        crashed,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileCost {
    pub profile: Profile,
    pub layout: Result<ProfileLayout, FormatError>,
    pub metadata_writes_per_session: u64,
    pub dirty_verification_bytes: u64,
}

pub fn compare_profile_costs(
    parity_extent_length: u64,
    protected_data_length: u64,
) -> [ProfileCost; 3] {
    Profile::ALL.map(|profile| {
        let layout =
            ProfileConfig::new(profile, parity_extent_length, protected_data_length).layout();
        let (metadata_writes_per_session, dirty_verification_bytes) = match profile {
            Profile::BareParity => (0, protected_data_length),
            Profile::RedundantEnvelope => (2, protected_data_length),
            Profile::EnvelopeBitmap => (3, DIRTY_REGION_BYTES.min(protected_data_length)),
        };
        ProfileCost {
            profile,
            layout,
            metadata_writes_per_session,
            dirty_verification_bytes,
        }
    })
}

pub fn run_interrupted_migration(
    prior: Profile,
    target: Profile,
    old_copy_a: Option<&[u8]>,
    old_copy_b: Option<&[u8]>,
    new_copy_a: Option<&[u8]>,
    new_copy_b: Option<&[u8]>,
) -> MigrationDecision {
    let _ = (old_copy_a, old_copy_b);
    dwv_format::select_migrated_profile(prior, target, new_copy_a, new_copy_b)
}

fn base_record(
    profile: Profile,
    layout: &ProfileLayout,
    logical_length: u64,
    copy_slot: u8,
) -> EnvelopeRecord {
    EnvelopeRecord {
        profile,
        compatible_features: 0,
        required_features: (profile == Profile::EnvelopeBitmap) as u32,
        array_id: [1; 16],
        parity_device_id: [2; 16],
        parity_role: 0,
        codec_profile: 1,
        payload: layout.payload,
        logical_length,
        logical_block_size: 4096,
        stripe_width: 2,
        topology_generation: 1,
        session_generation: 1,
        last_global_clean_checkpoint: 0,
        last_full_verified_checkpoint: None,
        session_state: SessionState::Closed,
        migration_state: MigrationState::Stable,
        copy_generation: 1,
        copy_slot,
        bitmap_region_bytes: if profile == Profile::EnvelopeBitmap {
            DIRTY_REGION_BYTES
        } else {
            0
        },
        bitmap: vec![0; layout.bitmap_bytes as usize],
    }
}

fn dirty_bitmap_bytes(record: &EnvelopeRecord, protected_data_length: u64) -> u64 {
    if record.bitmap.is_empty() {
        return protected_data_length;
    }
    record
        .bitmap
        .iter()
        .map(|byte| byte.count_ones() as u64)
        .sum::<u64>()
        .saturating_mul(record.bitmap_region_bytes)
        .min(protected_data_length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_after_dirty_is_not_clean() {
        let schedule = EnvelopeSchedule::new(vec![
            EnvelopeCutPoint::MarkDirty,
            EnvelopeCutPoint::ProtectedMutation,
            EnvelopeCutPoint::Crash,
        ]);
        let outcome =
            run_envelope_schedule(Profile::RedundantEnvelope, 16_384, 8_192, &schedule).unwrap();
        assert!(outcome.crashed);
        assert_eq!(outcome.assessment.state, EvidenceState::Dirty);
        assert_eq!(outcome.verification_bytes, 8_192);
    }

    #[test]
    fn torn_or_disagreeing_copies_are_unknown() {
        let torn = EnvelopeSchedule::new(vec![EnvelopeCutPoint::TornCopyA]);
        assert_eq!(
            run_envelope_schedule(Profile::RedundantEnvelope, 16_384, 8_192, &torn)
                .unwrap()
                .assessment
                .state,
            EvidenceState::Unknown
        );
        let conflict = EnvelopeSchedule::new(vec![EnvelopeCutPoint::DisagreeGeneration]);
        assert_eq!(
            run_envelope_schedule(Profile::RedundantEnvelope, 16_384, 8_192, &conflict)
                .unwrap()
                .assessment
                .state,
            EvidenceState::Unknown
        );
    }

    #[test]
    fn clean_requires_fence_and_checkpoint() {
        let early = EnvelopeSchedule::new(vec![EnvelopeCutPoint::PublishClean]);
        let outcome =
            run_envelope_schedule(Profile::RedundantEnvelope, 16_384, 8_192, &early).unwrap();
        assert!(outcome.clean_publish_blocked);
        assert_eq!(outcome.assessment.state, EvidenceState::Clean);
        let complete = EnvelopeSchedule::new(vec![
            EnvelopeCutPoint::MarkDirty,
            EnvelopeCutPoint::ProtectedMutation,
            EnvelopeCutPoint::DurableFence,
            EnvelopeCutPoint::RecoveryCheckpoint,
            EnvelopeCutPoint::PublishClean,
        ]);
        let outcome =
            run_envelope_schedule(Profile::RedundantEnvelope, 16_384, 8_192, &complete).unwrap();
        assert_eq!(outcome.assessment.state, EvidenceState::Clean);
        assert!(!outcome.assessment.clean_recovery_authorized);
    }

    #[test]
    fn costs_show_profile_overhead() {
        let costs = compare_profile_costs(16_384, 8_192);
        assert_eq!(costs[0].metadata_writes_per_session, 0);
        assert_eq!(costs[1].metadata_writes_per_session, 2);
        assert!(costs[1].layout.is_ok());
    }

    #[test]
    fn interrupted_migration_keeps_prior_profile() {
        let schedule = EnvelopeSchedule::new(vec![
            EnvelopeCutPoint::BeginMigration {
                target: Profile::RedundantEnvelope,
            },
            EnvelopeCutPoint::Crash,
        ]);
        let outcome =
            run_envelope_schedule(Profile::RedundantEnvelope, 16_384, 8_192, &schedule).unwrap();
        assert_eq!(
            outcome.migration,
            Some(MigrationDecision::RetainedPrior {
                profile: Profile::RedundantEnvelope,
                reason: dwv_format::CopyReason::MatchingCopies,
            })
        );
    }
}
