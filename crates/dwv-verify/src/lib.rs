//! Portable exhaustive parity verification and conservative repair planning.
//!
//! This crate reports parity consistency and evidence disposition without
//! owning filesystem, recovery-database, frontend, or runtime semantics.

mod degraded;
mod error;
mod evidence;
mod rebuild;
mod repair;
mod report;
mod scan;
mod store;

pub use degraded::{
    DegradedReadError, DegradedReadOutcome, DegradedReadTelemetry, KnownErasureAuthorization,
    ReconstructionRangeEvidence, ReconstructionSourceState, authorize_known_erasure,
    read_known_erasure,
};
pub use error::VerificationError;
pub use evidence::{ChecksumEvidence, DigestEvidence, EvidenceKind};
pub use rebuild::{
    RebuildBinding, RebuildChunkReceipt, RebuildError, RebuildRangePlan, RebuildTarget,
    RebuildVerificationReceipt, execute_rebuild_chunk, plan_rebuild_ranges,
    verify_complete_rebuild,
};
pub use repair::{
    RepairCandidate, RepairOutcome, RepairPlan, RepairRefusal, RepairTarget, apply_repair,
    plan_repairs,
};
pub use report::{
    EvidenceStatus, MemberRef, MismatchClass, RegionDisposition, RegionReport, ScanMode,
    VerificationReport,
};
pub use scan::{DEFAULT_MAX_REGIONS, ScanConfig, verify_exhaustive, verify_sampled};
pub use store::{VerificationIdentity, VerificationStore, VerificationStoreError};

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_codec::Geometry;
    use dwv_core::ByteRange;
    use dwv_recovery::{Blake3Provider, Digest, DigestProvider};

    const LENGTH: usize = 16;
    const REGION: u64 = 4;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct MemoryStore {
        bytes: Vec<u8>,
        identity: VerificationIdentity,
        writes: usize,
        fail_reads: bool,
        fail_writes: bool,
    }

    impl MemoryStore {
        fn new(bytes: Vec<u8>) -> Self {
            let mut identity = [0_u8; 16];
            for (index, byte) in bytes.iter().enumerate() {
                identity[index % identity.len()] = identity[index % identity.len()]
                    .wrapping_add(*byte)
                    .wrapping_add(index as u8);
            }
            Self {
                bytes,
                identity: VerificationIdentity(identity),
                writes: 0,
                fail_reads: false,
                fail_writes: false,
            }
        }

        fn failing_write(mut self) -> Self {
            self.fail_writes = true;
            self
        }

        fn with_identity(mut self, identity: [u8; 16]) -> Self {
            self.identity = VerificationIdentity(identity);
            self
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
            let start = usize::try_from(range.offset)
                .map_err(|_| VerificationStoreError::new("offset does not fit memory"))?;
            let end = start
                .checked_add(
                    usize::try_from(range.length)
                        .map_err(|_| VerificationStoreError::new("length does not fit memory"))?,
                )
                .ok_or_else(|| VerificationStoreError::new("range overflows memory"))?;
            self.bytes
                .get(start..end)
                .map(ToOwned::to_owned)
                .ok_or_else(|| VerificationStoreError::new("short memory store"))
        }

        fn write_exact(
            &mut self,
            range: ByteRange,
            bytes: &[u8],
        ) -> Result<(), VerificationStoreError> {
            if self.fail_writes {
                return Err(VerificationStoreError::new("injected write failure"));
            }
            if bytes.len() as u64 != range.length {
                return Err(VerificationStoreError::new("write length mismatch"));
            }
            let start = usize::try_from(range.offset)
                .map_err(|_| VerificationStoreError::new("offset does not fit memory"))?;
            let end = start
                .checked_add(bytes.len())
                .ok_or_else(|| VerificationStoreError::new("range overflows memory"))?;
            let destination = self
                .bytes
                .get_mut(start..end)
                .ok_or_else(|| VerificationStoreError::new("short memory target"))?;
            destination.copy_from_slice(bytes);
            self.writes += 1;
            Ok(())
        }
    }

    fn config() -> ScanConfig {
        ScanConfig::new(
            Geometry::new(vec![LENGTH as u64, LENGTH as u64], LENGTH as u64).unwrap(),
            REGION,
        )
        .unwrap()
    }

    fn data_images() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let data0 = (0..LENGTH).map(|index| index as u8).collect::<Vec<_>>();
        let data1 = (0..LENGTH)
            .map(|index| 0xa0_u8.wrapping_add(index as u8))
            .collect::<Vec<_>>();
        let parity = data0
            .iter()
            .zip(&data1)
            .map(|(left, right)| left ^ right)
            .collect::<Vec<_>>();
        (data0, data1, parity)
    }

    fn ranges() -> Vec<ByteRange> {
        (0..4)
            .map(|index| ByteRange::new(index * REGION, REGION).unwrap())
            .collect()
    }

    fn evidence_for(data: &[Vec<u8>], parity: &[u8], ranges: &[ByteRange]) -> ChecksumEvidence {
        let provider = Blake3Provider;
        let digest = |bytes: &[u8], range: ByteRange| {
            DigestEvidence::Current(
                provider
                    .digest(&bytes[range.offset as usize..range.end() as usize])
                    .unwrap(),
            )
        };
        ChecksumEvidence::new(
            data.iter()
                .map(|member| {
                    ranges
                        .iter()
                        .copied()
                        .map(|range| digest(member, range))
                        .collect()
                })
                .collect(),
            ranges
                .iter()
                .copied()
                .map(|range| digest(parity, range))
                .collect(),
        )
    }

    fn source_stores() -> (Vec<MemoryStore>, MemoryStore) {
        let (data0, data1, parity) = data_images();
        (
            vec![MemoryStore::new(data0), MemoryStore::new(data1)],
            MemoryStore::new(parity),
        )
    }

    #[test]
    fn exhaustive_matching_scan_is_read_only_and_not_clean_authority() {
        let (data0, data1, parity_image) = data_images();
        let evidence = evidence_for(&[data0.clone(), data1.clone()], &parity_image, &ranges());
        let (mut data, mut parity) = source_stores();
        let report = verify_exhaustive(&mut data, &mut parity, &config(), &evidence).unwrap();

        assert!(report.parity_consistent);
        assert!(report.exhaustive_complete);
        assert!(!report.can_authorize_clean());
        assert_eq!(report.payload_writes, 0);
        assert_eq!(report.matching_regions(), 4);
        assert!(data.iter().all(|store| store.writes == 0));
        assert_eq!(parity.writes, 0);
    }

    #[test]
    fn sampled_agreement_remains_non_exhaustive() {
        let (data0, data1, parity_image) = data_images();
        let sample = vec![ranges()[0]];
        let evidence = evidence_for(&[data0.clone(), data1.clone()], &parity_image, &sample);
        let (mut data, mut parity) = source_stores();
        let report = verify_sampled(&mut data, &mut parity, &config(), &evidence, &sample).unwrap();

        assert_eq!(report.mode, ScanMode::Sampled);
        assert!(report.parity_consistent);
        assert!(!report.exhaustive_complete);
        assert!(!report.can_authorize_clean());
        assert_eq!(report.payload_writes, 0);
    }

    #[test]
    fn parity_mismatch_is_repaired_to_a_separate_target() {
        let (data0, data1, parity_image) = data_images();
        let evidence = evidence_for(&[data0.clone(), data1.clone()], &parity_image, &ranges());
        let (mut data, mut parity) = source_stores();
        parity.bytes[1] ^= 0xff;
        let report = verify_exhaustive(&mut data, &mut parity, &config(), &evidence).unwrap();
        let plan = plan_repairs(&report);
        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].target(), RepairTarget::Parity);

        let original_parity = parity.bytes.clone();
        let mut target = MemoryStore::new(vec![0; LENGTH]);
        let outcome = apply_repair(
            &config(),
            &mut data,
            &mut parity,
            &mut target,
            plan.candidates[0].clone(),
        )
        .unwrap();
        assert_eq!(outcome.target(), RepairTarget::Parity);
        assert_eq!(outcome.target_identity(), target.identity());
        assert_ne!(outcome.digest(), Digest::default());
        assert_eq!(&target.bytes[..4], &parity_image[..4]);
        assert_eq!(parity.bytes, original_parity);
        assert_eq!(target.writes, 1);
    }

    #[test]
    fn data_mismatch_is_reconstructed_to_a_separate_target() {
        let (data0, data1, parity_image) = data_images();
        let evidence = evidence_for(&[data0.clone(), data1.clone()], &parity_image, &ranges());
        let (mut data, mut parity) = source_stores();
        data[1].bytes[6] ^= 0x55;
        let report = verify_exhaustive(&mut data, &mut parity, &config(), &evidence).unwrap();
        let plan = plan_repairs(&report);
        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].target(), RepairTarget::Data { slot: 1 });

        let original_data = data[1].bytes.clone();
        let mut target = MemoryStore::new(vec![0; LENGTH]);
        apply_repair(
            &config(),
            &mut data,
            &mut parity,
            &mut target,
            plan.candidates[0].clone(),
        )
        .unwrap();
        assert_eq!(&target.bytes[4..8], &data1[4..8]);
        assert_eq!(data[1].bytes, original_data);
    }

    #[test]
    fn missing_multiple_or_conflicting_evidence_refuses_repair() {
        let (data0, data1, _parity_image) = data_images();
        let (mut data, mut parity) = source_stores();
        data[0].bytes[0] ^= 1;
        parity.bytes[0] ^= 2;
        let absent = ChecksumEvidence::new(
            vec![vec![DigestEvidence::Absent; 4]; 2],
            vec![DigestEvidence::Absent; 4],
        );
        let report = verify_exhaustive(&mut data, &mut parity, &config(), &absent).unwrap();
        let plan = plan_repairs(&report);
        assert!(plan.candidates.is_empty());
        assert_eq!(plan.refused.len(), 1);

        let (mut clean_data, mut conflicting_parity) = source_stores();
        conflicting_parity.bytes[0] ^= 3;
        let conflicting = evidence_for(
            &[data0.clone(), data1.clone()],
            &conflicting_parity.bytes,
            &ranges(),
        );
        let report = verify_exhaustive(
            &mut clean_data,
            &mut conflicting_parity,
            &config(),
            &conflicting,
        )
        .unwrap();
        assert!(matches!(
            report.regions[0].disposition,
            RegionDisposition::Mismatch(MismatchClass::EvidenceConflict)
        ));
        assert!(plan_repairs(&report).candidates.is_empty());
    }

    #[test]
    fn failed_repair_preserves_source_members() {
        let (data0, data1, parity_image) = data_images();
        let evidence = evidence_for(&[data0, data1], &parity_image, &ranges());
        let (mut data, mut parity) = source_stores();
        parity.bytes[4] ^= 0x80;
        let report = verify_exhaustive(&mut data, &mut parity, &config(), &evidence).unwrap();
        let candidate = plan_repairs(&report).candidates[0].clone();
        let original_data = data.clone();
        let original_parity = parity.clone();
        let mut target = MemoryStore::new(vec![0; LENGTH]).failing_write();
        assert!(apply_repair(&config(), &mut data, &mut parity, &mut target, candidate).is_err());
        assert_eq!(data, original_data);
        assert_eq!(parity, original_parity);
    }

    #[test]
    fn aliased_source_or_repair_target_is_rejected() {
        let (data0, data1, parity_image) = data_images();
        let evidence = evidence_for(&[data0, data1], &parity_image, &ranges());
        let (mut data, mut parity) = source_stores();
        parity.bytes[0] ^= 0x40;
        let report = verify_exhaustive(&mut data, &mut parity, &config(), &evidence).unwrap();
        let candidate = plan_repairs(&report).candidates[0].clone();
        let target_identity = data[0].identity.0;
        let mut target = MemoryStore::new(vec![0; LENGTH]).with_identity(target_identity);
        assert!(matches!(
            apply_repair(&config(), &mut data, &mut parity, &mut target, candidate),
            Err(VerificationError::Repair(message)) if message.contains("aliases")
        ));

        let mut aliased_data = vec![
            MemoryStore::new(data[0].bytes.clone()).with_identity([9; 16]),
            MemoryStore::new(data[1].bytes.clone()).with_identity([9; 16]),
        ];
        let mut aliased_parity = MemoryStore::new(parity.bytes.clone()).with_identity([10; 16]);
        assert!(matches!(
            verify_exhaustive(&mut aliased_data, &mut aliased_parity, &config(), &evidence),
            Err(VerificationError::InvalidConfig(message)) if message.contains("aliased")
        ));
    }

    #[test]
    fn repair_candidate_is_bound_to_the_exact_scanned_sources() {
        let (data0, data1, parity_image) = data_images();
        let evidence = evidence_for(&[data0, data1], &parity_image, &ranges());
        let (mut data, mut parity) = source_stores();
        parity.bytes[0] ^= 0x40;
        let report = verify_exhaustive(&mut data, &mut parity, &config(), &evidence).unwrap();
        let candidate = plan_repairs(&report).candidates.remove(0);
        data[0].identity = VerificationIdentity([99; 16]);
        let mut target = MemoryStore::new(vec![0; LENGTH]);
        assert!(matches!(
            apply_repair(&config(), &mut data, &mut parity, &mut target, candidate),
            Err(VerificationError::Repair(message)) if message.contains("differ from the verification run")
        ));
        assert_eq!(target.writes, 0);
    }

    #[test]
    fn incomplete_and_overbound_scans_fail_closed() {
        let (data0, data1, parity_image) = data_images();
        let evidence = evidence_for(&[data0.clone(), data1.clone()], &parity_image, &ranges());
        let (mut data, mut parity) = source_stores();
        data[0].bytes.pop();
        let report = verify_exhaustive(&mut data, &mut parity, &config(), &evidence).unwrap();
        assert!(matches!(
            report.regions[3].disposition,
            RegionDisposition::Incomplete {
                member: MemberRef::Data(0)
            }
        ));
        assert!(!report.exhaustive_complete);

        let bounded = config().with_max_regions(2);
        let (mut data, mut parity) = source_stores();
        assert!(matches!(
            verify_exhaustive(&mut data, &mut parity, &bounded, &evidence),
            Err(VerificationError::InvalidConfig(_))
        ));
    }
}
