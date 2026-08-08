use crate::{
    RepairOutcome, RepairPlan, VerificationError, VerificationIdentity, VerificationReport,
    VerificationStore, apply_repair, plan_repairs,
};
use dwv_core::TopologyEpoch;
use dwv_recovery::{ChecksumSetGeneration, RecoveryGeneration};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScrubContext {
    pub topology_epoch: TopologyEpoch,
    pub recovery_generation: RecoveryGeneration,
    pub checksum_set_generation: ChecksumSetGeneration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScrubPlan {
    report: VerificationReport,
    repairs: RepairPlan,
    data_identities: Vec<VerificationIdentity>,
    parity_identity: VerificationIdentity,
    replacement_identity: VerificationIdentity,
    context: ScrubContext,
}

impl ScrubPlan {
    pub fn report(&self) -> &VerificationReport {
        &self.report
    }

    pub fn repairs(&self) -> &RepairPlan {
        &self.repairs
    }

    pub const fn context(&self) -> ScrubContext {
        self.context
    }

    pub fn validate<S: VerificationStore>(
        &self,
        data: &[S],
        parity: &S,
        replacement: &S,
        context: ScrubContext,
    ) -> Result<(), VerificationError> {
        if self.report.mode() != crate::ScanMode::Exhaustive {
            return Err(VerificationError::Repair(
                "sampled scrub reports cannot authorize repair".to_owned(),
            ));
        }
        if context != self.context {
            return Err(VerificationError::Repair(
                "scrub plan generation is stale".to_owned(),
            ));
        }
        if data
            .iter()
            .map(VerificationStore::identity)
            .collect::<Vec<_>>()
            != self.data_identities
            || parity.identity() != self.parity_identity
        {
            return Err(VerificationError::Repair(
                "scrub plan sources differ from the scan".to_owned(),
            ));
        }
        if replacement.identity() != self.replacement_identity {
            return Err(VerificationError::Repair(
                "scrub replacement identity changed".to_owned(),
            ));
        }
        if self.repairs.candidates.len() != 1 {
            return Err(VerificationError::Repair(
                "scrub requires exactly one uniquely identified repair candidate".to_owned(),
            ));
        }
        Ok(())
    }
}

pub fn plan_scrub<S: VerificationStore>(
    report: VerificationReport,
    data: &[S],
    parity: &S,
    replacement: &S,
    context: ScrubContext,
) -> ScrubPlan {
    ScrubPlan {
        repairs: plan_repairs(&report),
        report,
        data_identities: data.iter().map(VerificationStore::identity).collect(),
        parity_identity: parity.identity(),
        replacement_identity: replacement.identity(),
        context,
    }
}

pub fn apply_scrub<S: VerificationStore>(
    config: &crate::ScanConfig,
    plan: &ScrubPlan,
    data: &mut [S],
    parity: &mut S,
    replacement: &mut S,
    context: ScrubContext,
) -> Result<Vec<RepairOutcome>, VerificationError> {
    plan.validate(data, parity, replacement, context)?;
    plan.repairs
        .candidates
        .iter()
        .cloned()
        .map(|candidate| apply_repair(config, data, parity, replacement, candidate))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChecksumEvidence, DigestEvidence, ScanConfig, VerificationStoreError, verify_exhaustive,
    };
    use dwv_codec::Geometry;
    use dwv_core::ByteRange;

    #[derive(Clone)]
    struct MemoryStore {
        id: VerificationIdentity,
        bytes: Vec<u8>,
    }

    impl VerificationStore for MemoryStore {
        fn identity(&self) -> VerificationIdentity {
            self.id
        }

        fn read_exact(&mut self, range: ByteRange) -> Result<Vec<u8>, VerificationStoreError> {
            let start = usize::try_from(range.offset).unwrap();
            let end = start + usize::try_from(range.length).unwrap();
            Ok(self.bytes[start..end].to_vec())
        }

        fn write_exact(
            &mut self,
            range: ByteRange,
            bytes: &[u8],
        ) -> Result<(), VerificationStoreError> {
            let start = usize::try_from(range.offset).unwrap();
            self.bytes[start..start + bytes.len()].copy_from_slice(bytes);
            Ok(())
        }
    }

    fn fixture() -> (Vec<MemoryStore>, MemoryStore, MemoryStore, ScanConfig) {
        let data = vec![
            MemoryStore {
                id: VerificationIdentity([1; 16]),
                bytes: vec![1, 2, 3, 4],
            },
            MemoryStore {
                id: VerificationIdentity([2; 16]),
                bytes: vec![5, 6, 7, 8],
            },
        ];
        let parity = MemoryStore {
            id: VerificationIdentity([3; 16]),
            bytes: vec![4, 4, 4, 12],
        };
        let replacement = MemoryStore {
            id: VerificationIdentity([4; 16]),
            bytes: vec![0; 4],
        };
        let config = ScanConfig::new(Geometry::new(vec![4, 4], 4).unwrap(), 4).unwrap();
        (data, parity, replacement, config)
    }

    #[test]
    fn stale_scrub_plan_is_rejected_before_target_write() {
        let (mut data, mut parity, mut replacement, config) = fixture();
        let evidence = ChecksumEvidence::new(
            vec![vec![DigestEvidence::Absent], vec![DigestEvidence::Absent]],
            vec![DigestEvidence::Absent],
        );
        let report = verify_exhaustive(&mut data, &mut parity, &config, &evidence).unwrap();
        let plan = plan_scrub(
            report,
            &data,
            &parity,
            &replacement,
            ScrubContext {
                topology_epoch: TopologyEpoch(1),
                recovery_generation: RecoveryGeneration::ZERO,
                checksum_set_generation: ChecksumSetGeneration::INITIAL,
            },
        );
        let before = replacement.bytes.clone();
        let result = apply_scrub(
            &config,
            &plan,
            &mut data,
            &mut parity,
            &mut replacement,
            ScrubContext {
                topology_epoch: TopologyEpoch(2),
                recovery_generation: RecoveryGeneration::ZERO,
                checksum_set_generation: ChecksumSetGeneration::INITIAL,
            },
        );
        assert!(result.is_err());
        assert_eq!(replacement.bytes, before);
    }
}
