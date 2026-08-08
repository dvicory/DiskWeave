//! Boundary between exhaustive verification reports and metadata-loss policy.

use dwv_recovery::MetadataLossVerification;
use dwv_verify::{
    MismatchClass, RegionDisposition, RepairOutcome, RepairTarget, ScanMode, VerificationReport,
};

pub fn classify_metadata_loss_verification(
    report: &VerificationReport,
    verified_repairs: &[RepairOutcome],
) -> MetadataLossVerification {
    if report.mode() != ScanMode::Exhaustive
        || report.payload_writes() != 0
        || report
            .regions()
            .iter()
            .any(|region| matches!(region.disposition, RegionDisposition::Incomplete { .. }))
    {
        return MetadataLossVerification::IncompleteScan;
    }

    let mut expected_repairs = Vec::new();
    for region in report.regions() {
        match region.disposition {
            RegionDisposition::Match { .. } => {}
            RegionDisposition::Mismatch(MismatchClass::ParityIdentified) => {
                expected_repairs.push((region.range, RepairTarget::Parity));
            }
            RegionDisposition::Mismatch(MismatchClass::DataIdentified { slot }) => {
                expected_repairs.push((region.range, RepairTarget::Data { slot }));
            }
            RegionDisposition::Mismatch(
                MismatchClass::Ambiguous | MismatchClass::EvidenceConflict,
            ) => return MetadataLossVerification::AmbiguousMismatch,
            RegionDisposition::Incomplete { .. } => unreachable!("handled above"),
        }
    }

    if expected_repairs.is_empty() {
        if report.exhaustive_complete() && report.parity_consistent() {
            MetadataLossVerification::ExhaustiveMatches
        } else {
            MetadataLossVerification::IncompleteScan
        }
    } else if repairs_exactly_cover(report, &expected_repairs, verified_repairs) {
        MetadataLossVerification::ExhaustiveIdentifiedRepairs
    } else {
        MetadataLossVerification::IdentifiedRepairPending
    }
}

fn repairs_exactly_cover(
    report: &VerificationReport,
    expected: &[(dwv_core::ByteRange, RepairTarget)],
    outcomes: &[RepairOutcome],
) -> bool {
    expected.len() == outcomes.len()
        && expected.iter().all(|(range, target)| {
            outcomes
                .iter()
                .filter(|outcome| outcome.verifies(report, *range, *target))
                .count()
                == 1
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_codec::Geometry;
    use dwv_core::ByteRange;
    use dwv_recovery::{Blake3Provider, DigestProvider};
    use dwv_verify::{
        ChecksumEvidence, DigestEvidence, RepairCandidate, ScanConfig, VerificationIdentity,
        VerificationStore, VerificationStoreError, apply_repair, plan_repairs, verify_exhaustive,
        verify_sampled,
    };

    #[derive(Clone)]
    struct MemoryStore {
        bytes: Vec<u8>,
        identity: VerificationIdentity,
        fail_reads: bool,
    }

    impl MemoryStore {
        fn new(bytes: Vec<u8>, identity: u8) -> Self {
            Self {
                bytes,
                identity: VerificationIdentity([identity; 16]),
                fail_reads: false,
            }
        }
    }

    impl VerificationStore for MemoryStore {
        fn identity(&self) -> VerificationIdentity {
            self.identity
        }

        fn read_exact(&mut self, range: ByteRange) -> Result<Vec<u8>, VerificationStoreError> {
            if self.fail_reads {
                return Err(VerificationStoreError::new("injected read failure"));
            }
            let start = usize::try_from(range.offset).unwrap();
            let end = start + usize::try_from(range.length).unwrap();
            self.bytes
                .get(start..end)
                .map(ToOwned::to_owned)
                .ok_or_else(|| VerificationStoreError::new("short read"))
        }

        fn write_exact(
            &mut self,
            range: ByteRange,
            bytes: &[u8],
        ) -> Result<(), VerificationStoreError> {
            let start = usize::try_from(range.offset).unwrap();
            let end = start + bytes.len();
            self.bytes[start..end].copy_from_slice(bytes);
            Ok(())
        }
    }

    fn fixture() -> (Vec<MemoryStore>, MemoryStore, ScanConfig, ChecksumEvidence) {
        let data0 = vec![1, 2, 3, 4];
        let data1 = vec![8, 7, 6, 5];
        let parity = data0
            .iter()
            .zip(&data1)
            .map(|(left, right)| left ^ right)
            .collect::<Vec<_>>();
        let provider = Blake3Provider;
        let digest =
            |bytes: &[u8]| DigestEvidence::Current(provider.digest(bytes).expect("fixture digest"));
        let evidence = ChecksumEvidence::new(
            vec![vec![digest(&data0)], vec![digest(&data1)]],
            vec![digest(&parity)],
        );
        (
            vec![MemoryStore::new(data0, 1), MemoryStore::new(data1, 2)],
            MemoryStore::new(parity, 3),
            ScanConfig::new(Geometry::new(vec![4, 4], 4).unwrap(), 4).unwrap(),
            evidence,
        )
    }

    fn apply_candidate(
        report: &VerificationReport,
        data: &mut [MemoryStore],
        parity: &mut MemoryStore,
        config: &ScanConfig,
        candidate: RepairCandidate,
    ) -> RepairOutcome {
        let mut target = MemoryStore::new(vec![0; 4], 9);
        let outcome = apply_repair(config, data, parity, &mut target, candidate).unwrap();
        assert!(outcome.verifies(report, outcome.range(), outcome.target()));
        outcome
    }

    #[test]
    fn exhaustive_matches_map_to_new_baseline_evidence() {
        let (mut data, mut parity, config, evidence) = fixture();
        let report = verify_exhaustive(&mut data, &mut parity, &config, &evidence).unwrap();
        assert_eq!(
            classify_metadata_loss_verification(&report, &[]),
            MetadataLossVerification::ExhaustiveMatches
        );
    }

    #[test]
    fn identified_mismatch_requires_every_verified_repair_outcome() {
        let (mut data, mut parity, config, evidence) = fixture();
        parity.bytes[0] ^= 0xff;
        let report = verify_exhaustive(&mut data, &mut parity, &config, &evidence).unwrap();
        assert_eq!(
            classify_metadata_loss_verification(&report, &[]),
            MetadataLossVerification::IdentifiedRepairPending
        );
        let candidate = plan_repairs(&report).candidates.remove(0);
        let outcome = apply_candidate(&report, &mut data, &mut parity, &config, candidate);
        assert_eq!(
            classify_metadata_loss_verification(&report, std::slice::from_ref(&outcome)),
            MetadataLossVerification::ExhaustiveIdentifiedRepairs
        );

        let (mut other_data, mut other_parity, other_config, other_evidence) = fixture();
        other_parity.bytes[0] ^= 0xff;
        let other_report = verify_exhaustive(
            &mut other_data,
            &mut other_parity,
            &other_config,
            &other_evidence,
        )
        .unwrap();
        assert_eq!(
            classify_metadata_loss_verification(&other_report, &[outcome]),
            MetadataLossVerification::IdentifiedRepairPending
        );
    }

    #[test]
    fn ambiguous_sampled_and_incomplete_reports_fail_closed() {
        let (mut data, mut parity, config, _evidence) = fixture();
        parity.bytes[0] ^= 0xff;
        let absent = ChecksumEvidence::new(
            vec![vec![DigestEvidence::Absent]; 2],
            vec![DigestEvidence::Absent],
        );
        let ambiguous = verify_exhaustive(&mut data, &mut parity, &config, &absent).unwrap();
        assert_eq!(
            classify_metadata_loss_verification(&ambiguous, &[]),
            MetadataLossVerification::AmbiguousMismatch
        );

        let (mut data, mut parity, config, evidence) = fixture();
        let ranges = [ByteRange::new(0, 4).unwrap()];
        let sampled = verify_sampled(&mut data, &mut parity, &config, &evidence, &ranges).unwrap();
        assert_eq!(
            classify_metadata_loss_verification(&sampled, &[]),
            MetadataLossVerification::IncompleteScan
        );

        let (mut data, mut parity, config, evidence) = fixture();
        data[0].fail_reads = true;
        let incomplete = verify_exhaustive(&mut data, &mut parity, &config, &evidence).unwrap();
        assert_eq!(
            classify_metadata_loss_verification(&incomplete, &[]),
            MetadataLossVerification::IncompleteScan
        );
    }
}
