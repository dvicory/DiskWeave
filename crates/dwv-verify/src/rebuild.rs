//! Deterministic separate-target offline rebuild mechanics.

use crate::{
    DegradedReadError, KnownErasureAuthorization, VerificationIdentity, VerificationStore,
    VerificationStoreError, read_known_erasure,
};
use dwv_core::{ArrayId, AssignmentInstanceId, ByteRange, CodingPosition, SlotId, TopologyEpoch};
use dwv_recovery::{
    Blake3Provider, Digest, DigestProvider, RebuildId, RebuildState, RecoveryGeneration,
};
use dwv_store::{StoreFenceRef, StoreId};
use std::fmt;

pub trait RebuildTarget: VerificationStore {
    /// Establishes durable replacement bytes through the adapter's captured
    /// write-operation watermark and returns that exact fence.
    /// Implementations must fail when durability is unknown.
    fn flush_rebuild(&mut self) -> Result<StoreFenceRef, VerificationStoreError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RebuildBinding {
    rebuild_id: RebuildId,
    replacement_assignment: AssignmentInstanceId,
    replacement_store: StoreId,
}

impl RebuildBinding {
    pub fn from_recovery(rebuild: &RebuildState) -> Self {
        Self {
            rebuild_id: rebuild.id(),
            replacement_assignment: rebuild.replacement_assignment(),
            replacement_store: rebuild.replacement_store(),
        }
    }

    pub const fn rebuild_id(self) -> RebuildId {
        self.rebuild_id
    }

    pub const fn replacement_assignment(self) -> AssignmentInstanceId {
        self.replacement_assignment
    }

    pub const fn replacement_store(self) -> StoreId {
        self.replacement_store
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebuildChunkReceipt {
    rebuild: RebuildBinding,
    array_id: ArrayId,
    topology_epoch: TopologyEpoch,
    recovery_generation: RecoveryGeneration,
    stable_slot: SlotId,
    coding_position: CodingPosition,
    range: ByteRange,
    replacement_identity: VerificationIdentity,
    digest: Digest,
    durable_through: u64,
    durable_fence: StoreFenceRef,
}

impl RebuildChunkReceipt {
    pub const fn rebuild(&self) -> RebuildBinding {
        self.rebuild
    }
    pub const fn array_id(&self) -> ArrayId {
        self.array_id
    }
    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch
    }
    pub const fn recovery_generation(&self) -> RecoveryGeneration {
        self.recovery_generation
    }
    pub const fn stable_slot(&self) -> SlotId {
        self.stable_slot
    }
    pub const fn coding_position(&self) -> CodingPosition {
        self.coding_position
    }
    pub const fn range(&self) -> ByteRange {
        self.range
    }
    pub const fn replacement_identity(&self) -> VerificationIdentity {
        self.replacement_identity
    }
    pub const fn digest(&self) -> Digest {
        self.digest
    }
    pub const fn durable_through(&self) -> u64 {
        self.durable_through
    }
    pub const fn durable_fence(&self) -> StoreFenceRef {
        self.durable_fence
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebuildVerificationReceipt {
    rebuild: RebuildBinding,
    array_id: ArrayId,
    topology_epoch: TopologyEpoch,
    recovery_generation: RecoveryGeneration,
    stable_slot: SlotId,
    coding_position: CodingPosition,
    replacement_identity: VerificationIdentity,
    protected_length: u64,
    digest: Digest,
}

impl RebuildVerificationReceipt {
    pub const fn rebuild(&self) -> RebuildBinding {
        self.rebuild
    }
    pub const fn array_id(&self) -> ArrayId {
        self.array_id
    }
    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch
    }
    pub const fn recovery_generation(&self) -> RecoveryGeneration {
        self.recovery_generation
    }
    pub const fn stable_slot(&self) -> SlotId {
        self.stable_slot
    }
    pub const fn coding_position(&self) -> CodingPosition {
        self.coding_position
    }
    pub const fn replacement_identity(&self) -> VerificationIdentity {
        self.replacement_identity
    }
    pub const fn protected_length(&self) -> u64 {
        self.protected_length
    }
    pub const fn digest(&self) -> Digest {
        self.digest
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RebuildError {
    InvalidPlan(&'static str),
    ReplacementAliasesSource,
    ReplacementIdentityChanged,
    DegradedRead(DegradedReadError),
    Store(String),
    VerificationFailed,
    Digest(String),
}

impl fmt::Display for RebuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPlan(message) => write!(formatter, "invalid rebuild plan: {message}"),
            Self::ReplacementAliasesSource => {
                formatter.write_str("replacement aliases a rebuild source")
            }
            Self::ReplacementIdentityChanged => {
                formatter.write_str("replacement identity changed during rebuild")
            }
            Self::DegradedRead(error) => write!(formatter, "rebuild decode failed: {error}"),
            Self::Store(message) => write!(formatter, "rebuild target failed: {message}"),
            Self::VerificationFailed => {
                formatter.write_str("replacement readback differs from reconstructed bytes")
            }
            Self::Digest(message) => write!(formatter, "rebuild digest failed: {message}"),
        }
    }
}

impl std::error::Error for RebuildError {}

pub fn plan_rebuild_ranges(
    protected_length: u64,
    chunk_size: u64,
    cursor: u64,
    logical_block_size: u32,
) -> Result<RebuildRangePlan, RebuildError> {
    let block = u64::from(logical_block_size);
    if protected_length == 0
        || chunk_size == 0
        || block == 0
        || cursor > protected_length
        || !protected_length.is_multiple_of(block)
        || !chunk_size.is_multiple_of(block)
        || !cursor.is_multiple_of(block)
    {
        return Err(RebuildError::InvalidPlan(
            "length/chunk/block must be non-zero, bounded, and block-aligned",
        ));
    }
    Ok(RebuildRangePlan {
        protected_length,
        chunk_size,
        next_offset: cursor,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebuildRangePlan {
    protected_length: u64,
    chunk_size: u64,
    next_offset: u64,
}

impl Iterator for RebuildRangePlan {
    type Item = ByteRange;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_offset >= self.protected_length {
            return None;
        }
        let offset = self.next_offset;
        let length = self.chunk_size.min(self.protected_length - offset);
        self.next_offset += length;
        Some(ByteRange::new(offset, length).expect("validated rebuild range cannot overflow"))
    }
}

pub fn execute_rebuild_chunk<S: VerificationStore, T: RebuildTarget>(
    rebuild: RebuildBinding,
    authorization: &KnownErasureAuthorization,
    current_topology_epoch: TopologyEpoch,
    current_recovery_generation: RecoveryGeneration,
    data: &mut [Option<S>],
    parity: &mut S,
    replacement: &mut T,
) -> Result<RebuildChunkReceipt, RebuildError> {
    let replacement_identity = replacement.identity();
    if authorization.aliases_identity(replacement_identity) {
        return Err(RebuildError::ReplacementAliasesSource);
    }
    let reconstructed = read_known_erasure(
        authorization,
        current_topology_epoch,
        current_recovery_generation,
        data,
        parity,
    )
    .map_err(RebuildError::DegradedRead)?;
    replacement
        .write_exact(authorization.range(), &reconstructed.bytes)
        .map_err(|error| RebuildError::Store(error.to_string()))?;
    let readback = replacement
        .read_exact(authorization.range())
        .map_err(|error| RebuildError::Store(error.to_string()))?;
    if readback != reconstructed.bytes {
        return Err(RebuildError::VerificationFailed);
    }

    // Re-read every source after target write so the receipt reflects the
    // equation at the durability boundary, not merely the first decode.
    let equation_check = read_known_erasure(
        authorization,
        current_topology_epoch,
        current_recovery_generation,
        data,
        parity,
    )
    .map_err(RebuildError::DegradedRead)?;
    if equation_check.bytes != readback {
        return Err(RebuildError::VerificationFailed);
    }
    let durable_fence = replacement
        .flush_rebuild()
        .map_err(|error| RebuildError::Store(error.to_string()))?;
    if replacement.identity() != replacement_identity {
        return Err(RebuildError::ReplacementIdentityChanged);
    }
    let digest = Blake3Provider
        .digest(&readback)
        .map_err(|error| RebuildError::Digest(error.to_string()))?;
    Ok(RebuildChunkReceipt {
        rebuild,
        array_id: authorization.array_id(),
        topology_epoch: authorization.topology_epoch(),
        recovery_generation: authorization.recovery_generation(),
        stable_slot: authorization.stable_slot(),
        coding_position: authorization.coding_position(),
        range: authorization.range(),
        replacement_identity,
        digest,
        durable_through: authorization.range().end(),
        durable_fence,
    })
}

pub fn verify_complete_rebuild<S: VerificationStore, T: RebuildTarget>(
    rebuild: RebuildBinding,
    authorization: &KnownErasureAuthorization,
    current_topology_epoch: TopologyEpoch,
    current_recovery_generation: RecoveryGeneration,
    data: &mut [Option<S>],
    parity: &mut S,
    replacement: &mut T,
) -> Result<RebuildVerificationReceipt, RebuildError> {
    let protected_length = authorization
        .geometry()
        .data_length(authorization.missing_slot())
        .ok_or(RebuildError::InvalidPlan(
            "missing slot is outside geometry",
        ))?;
    if authorization.range().offset != 0 || authorization.range().length != protected_length {
        return Err(RebuildError::InvalidPlan(
            "final verification must cover the complete missing member",
        ));
    }
    let replacement_identity = replacement.identity();
    if authorization.aliases_identity(replacement_identity) {
        return Err(RebuildError::ReplacementAliasesSource);
    }
    let expected = read_known_erasure(
        authorization,
        current_topology_epoch,
        current_recovery_generation,
        data,
        parity,
    )
    .map_err(RebuildError::DegradedRead)?;
    let actual = replacement
        .read_exact(authorization.range())
        .map_err(|error| RebuildError::Store(error.to_string()))?;
    if actual != expected.bytes {
        return Err(RebuildError::VerificationFailed);
    }
    let digest = Blake3Provider
        .digest(&actual)
        .map_err(|error| RebuildError::Digest(error.to_string()))?;
    Ok(RebuildVerificationReceipt {
        rebuild,
        array_id: authorization.array_id(),
        topology_epoch: authorization.topology_epoch(),
        recovery_generation: authorization.recovery_generation(),
        stable_slot: authorization.stable_slot(),
        coding_position: authorization.coding_position(),
        replacement_identity,
        protected_length,
        digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ReconstructionRangeEvidence, ReconstructionSourceState, authorize_known_erasure};
    use dwv_codec::{Geometry, ParityCodec, XorReference};
    use dwv_core::{
        AssignmentGeneration, AssignmentInstanceId, CodingProfile, MemberRole, ProtectedGeometry,
        TopologyAssignment, TopologySnapshot,
    };
    use dwv_recovery::{RebuildTargetIdentity, TopologySnapshot as RecoveryTopologySnapshot};
    use dwv_store::{CapabilityEvidenceId, FenceId, StoreWriteWatermark};

    #[derive(Clone)]
    struct MemoryStore {
        bytes: Vec<u8>,
        identity: VerificationIdentity,
        writes: usize,
        flushes: usize,
        fail_flush: bool,
        fail_write: bool,
        corrupt_readback: bool,
        reads: usize,
        mutate_on_second_read: bool,
    }

    impl MemoryStore {
        fn new(bytes: Vec<u8>, identity: u8) -> Self {
            Self {
                bytes,
                identity: VerificationIdentity([identity; 16]),
                writes: 0,
                flushes: 0,
                fail_flush: false,
                fail_write: false,
                corrupt_readback: false,
                reads: 0,
                mutate_on_second_read: false,
            }
        }
    }

    impl VerificationStore for MemoryStore {
        fn identity(&self) -> VerificationIdentity {
            self.identity
        }

        fn read_exact(&mut self, range: ByteRange) -> Result<Vec<u8>, VerificationStoreError> {
            self.reads += 1;
            let start = usize::try_from(range.offset).unwrap();
            let end = start + usize::try_from(range.length).unwrap();
            let mut bytes = self
                .bytes
                .get(start..end)
                .ok_or_else(|| VerificationStoreError::new("range outside store"))?
                .to_vec();
            if self.corrupt_readback && !bytes.is_empty() {
                bytes[0] ^= 0xff;
            }
            if self.mutate_on_second_read && self.reads == 2 && !bytes.is_empty() {
                bytes[0] ^= 0x80;
            }
            Ok(bytes)
        }

        fn write_exact(
            &mut self,
            range: ByteRange,
            bytes: &[u8],
        ) -> Result<(), VerificationStoreError> {
            if self.fail_write {
                return Err(VerificationStoreError::new("injected target write failure"));
            }
            let start = usize::try_from(range.offset).unwrap();
            self.bytes[start..start + bytes.len()].copy_from_slice(bytes);
            self.writes += 1;
            Ok(())
        }
    }

    impl RebuildTarget for MemoryStore {
        fn flush_rebuild(&mut self) -> Result<StoreFenceRef, VerificationStoreError> {
            if self.fail_flush {
                return Err(VerificationStoreError::new("injected flush failure"));
            }
            self.flushes += 1;
            Ok(StoreFenceRef {
                fence_id: FenceId(self.flushes as u64),
                store_id: StoreId(90),
                topology_epoch: TopologyEpoch(2),
                through: StoreWriteWatermark(self.writes as u64),
                capability_evidence_id: CapabilityEvidenceId(1),
            })
        }
    }

    fn topology() -> TopologySnapshot {
        TopologySnapshot::new(
            ArrayId([4; 16]),
            TopologyEpoch(2),
            CodingProfile::new(2, 1).unwrap(),
            ProtectedGeometry::new(8, 1).unwrap(),
            vec![
                TopologyAssignment::new(
                    SlotId([1; 16]),
                    MemberRole::Data,
                    CodingPosition(0),
                    AssignmentInstanceId([11; 16]),
                    AssignmentGeneration(1),
                ),
                TopologyAssignment::new(
                    SlotId([2; 16]),
                    MemberRole::Data,
                    CodingPosition(1),
                    AssignmentInstanceId([12; 16]),
                    AssignmentGeneration(1),
                ),
                TopologyAssignment::new(
                    SlotId([3; 16]),
                    MemberRole::Parity,
                    CodingPosition(2),
                    AssignmentInstanceId([13; 16]),
                    AssignmentGeneration(1),
                ),
            ],
        )
        .unwrap()
    }

    fn binding() -> RebuildBinding {
        let recovery_topology = RecoveryTopologySnapshot::from_core(
            topology(),
            vec![StoreId(1), StoreId(2), StoreId(3)],
        )
        .unwrap();
        let rebuild = RebuildState::prepare(
            RebuildId::from_bytes([4; 16]),
            recovery_topology,
            RecoveryGeneration(5),
            SlotId([1; 16]),
            CodingPosition(0),
            AssignmentInstanceId([90; 16]),
            StoreId(90),
            RebuildTargetIdentity::from_bytes([9; 16]),
        )
        .unwrap();
        RebuildBinding::from_recovery(&rebuild)
    }

    fn fixture(
        range: ByteRange,
    ) -> (
        Vec<Option<MemoryStore>>,
        MemoryStore,
        MemoryStore,
        KnownErasureAuthorization,
        Vec<u8>,
    ) {
        let missing = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let survivor = vec![20, 21, 22, 23, 24, 25, 26, 27];
        let geometry = Geometry::new(vec![8, 8], 8).unwrap();
        let parity = XorReference
            .compute_parity(&geometry, &[&missing, &survivor])
            .unwrap();
        let data = vec![None, Some(MemoryStore::new(survivor, 2))];
        let parity = MemoryStore::new(parity, 3);
        let refs = data.iter().map(Option::as_ref).collect::<Vec<_>>();
        let authorization = authorize_known_erasure(
            &topology(),
            RecoveryGeneration(5),
            geometry,
            range,
            ReconstructionRangeEvidence::ParityClean,
            true,
            &refs,
            &parity,
            &[
                ReconstructionSourceState::Missing,
                ReconstructionSourceState::Available,
                ReconstructionSourceState::Available,
            ],
        )
        .unwrap();
        (
            data,
            parity,
            MemoryStore::new(vec![0; 8], 9),
            authorization,
            missing,
        )
    }

    #[test]
    fn planner_is_deterministic_from_the_first_unprocessed_byte() {
        assert_eq!(
            plan_rebuild_ranges(10, 4, 4, 1)
                .unwrap()
                .collect::<Vec<_>>(),
            vec![ByteRange::new(4, 4).unwrap(), ByteRange::new(8, 2).unwrap()]
        );
        assert!(plan_rebuild_ranges(10, 0, 0, 1).is_err());
        assert!(plan_rebuild_ranges(10, 4, 11, 1).is_err());
        assert!(plan_rebuild_ranges(10, 4, 0, 4).is_err());
        assert!(plan_rebuild_ranges(12, 3, 0, 4).is_err());
    }

    #[test]
    fn chunk_receipt_exists_only_after_readback_equation_and_flush() {
        let range = ByteRange::new(0, 4).unwrap();
        let (mut data, mut parity, mut replacement, authorization, missing) = fixture(range);
        let receipt = execute_rebuild_chunk(
            binding(),
            &authorization,
            TopologyEpoch(2),
            RecoveryGeneration(5),
            &mut data,
            &mut parity,
            &mut replacement,
        )
        .unwrap();
        assert_eq!(&replacement.bytes[..4], &missing[..4]);
        assert_eq!(replacement.writes, 1);
        assert_eq!(replacement.flushes, 1);
        assert_eq!(receipt.durable_through(), 4);
        assert_eq!(receipt.replacement_identity(), replacement.identity);
        assert!(data.iter().flatten().all(|source| source.writes == 0));
        assert_eq!(parity.writes, 0);
    }

    #[test]
    fn replay_before_checkpoint_rewrites_the_same_verified_bytes() {
        let range = ByteRange::new(0, 4).unwrap();
        let (mut data, mut parity, mut replacement, authorization, missing) = fixture(range);
        let first = execute_rebuild_chunk(
            binding(),
            &authorization,
            TopologyEpoch(2),
            RecoveryGeneration(5),
            &mut data,
            &mut parity,
            &mut replacement,
        )
        .unwrap();
        let first_bytes = replacement.bytes.clone();

        let replay = execute_rebuild_chunk(
            binding(),
            &authorization,
            TopologyEpoch(2),
            RecoveryGeneration(5),
            &mut data,
            &mut parity,
            &mut replacement,
        )
        .unwrap();
        assert_eq!(&replacement.bytes[..4], &missing[..4]);
        assert_eq!(replacement.bytes, first_bytes);
        assert_eq!(replay.digest(), first.digest());
        assert_eq!(replay.range(), first.range());
        assert_eq!(replacement.writes, 2);
        assert_eq!(replacement.flushes, 2);
    }

    #[test]
    fn alias_readback_and_durability_failures_produce_no_receipt() {
        let range = ByteRange::new(0, 4).unwrap();
        let (mut data, mut parity, mut replacement, authorization, _) = fixture(range);
        replacement.identity = parity.identity;
        assert!(matches!(
            execute_rebuild_chunk(
                binding(),
                &authorization,
                TopologyEpoch(2),
                RecoveryGeneration(5),
                &mut data,
                &mut parity,
                &mut replacement,
            ),
            Err(RebuildError::ReplacementAliasesSource)
        ));
        assert_eq!(replacement.writes, 0);

        let (mut data, mut parity, mut replacement, authorization, _) = fixture(range);
        replacement.fail_write = true;
        assert!(matches!(
            execute_rebuild_chunk(
                binding(),
                &authorization,
                TopologyEpoch(2),
                RecoveryGeneration(5),
                &mut data,
                &mut parity,
                &mut replacement,
            ),
            Err(RebuildError::Store(message)) if message.contains("write")
        ));
        assert_eq!(replacement.writes, 0);
        assert_eq!(replacement.flushes, 0);

        let (mut data, mut parity, mut replacement, authorization, _) = fixture(range);
        replacement.corrupt_readback = true;
        assert!(matches!(
            execute_rebuild_chunk(
                binding(),
                &authorization,
                TopologyEpoch(2),
                RecoveryGeneration(5),
                &mut data,
                &mut parity,
                &mut replacement,
            ),
            Err(RebuildError::VerificationFailed)
        ));
        assert_eq!(replacement.flushes, 0);

        let (mut data, mut parity, mut replacement, authorization, _) = fixture(range);
        replacement.fail_flush = true;
        assert!(matches!(
            execute_rebuild_chunk(
                binding(),
                &authorization,
                TopologyEpoch(2),
                RecoveryGeneration(5),
                &mut data,
                &mut parity,
                &mut replacement,
            ),
            Err(RebuildError::Store(message)) if message.contains("flush")
        ));
        assert_eq!(replacement.writes, 1);
        assert_eq!(replacement.flushes, 0);

        let (mut data, mut parity, mut replacement, authorization, _) = fixture(range);
        data[1].as_mut().unwrap().mutate_on_second_read = true;
        assert!(matches!(
            execute_rebuild_chunk(
                binding(),
                &authorization,
                TopologyEpoch(2),
                RecoveryGeneration(5),
                &mut data,
                &mut parity,
                &mut replacement,
            ),
            Err(RebuildError::VerificationFailed)
        ));
        assert_eq!(replacement.writes, 1);
        assert_eq!(replacement.flushes, 0);
    }

    #[test]
    fn complete_verification_requires_full_coverage_and_exact_replacement() {
        let full = ByteRange::new(0, 8).unwrap();
        let (mut data, mut parity, mut replacement, authorization, missing) = fixture(full);
        replacement.bytes.copy_from_slice(&missing);
        let receipt = verify_complete_rebuild(
            binding(),
            &authorization,
            TopologyEpoch(2),
            RecoveryGeneration(5),
            &mut data,
            &mut parity,
            &mut replacement,
        )
        .unwrap();
        assert_eq!(receipt.protected_length(), 8);
        assert_eq!(receipt.stable_slot(), SlotId([1; 16]));

        replacement.bytes[7] ^= 1;
        assert!(matches!(
            verify_complete_rebuild(
                binding(),
                &authorization,
                TopologyEpoch(2),
                RecoveryGeneration(5),
                &mut data,
                &mut parity,
                &mut replacement,
            ),
            Err(RebuildError::VerificationFailed)
        ));

        let (_, _, mut partial_target, partial, _) = fixture(ByteRange::new(0, 4).unwrap());
        let (mut partial_data, mut partial_parity, _, _, _) =
            fixture(ByteRange::new(0, 4).unwrap());
        assert!(matches!(
            verify_complete_rebuild(
                binding(),
                &partial,
                TopologyEpoch(2),
                RecoveryGeneration(5),
                &mut partial_data,
                &mut partial_parity,
                &mut partial_target,
            ),
            Err(RebuildError::InvalidPlan(_))
        ));
    }
}
