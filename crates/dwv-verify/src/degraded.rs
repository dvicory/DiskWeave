//! Evidence-gated single-XOR known-erasure reads.

use crate::{VerificationIdentity, VerificationStore};
use dwv_codec::{Geometry, ParityCodec, XorReference};
use dwv_core::{ArrayId, ByteRange, CodingPosition, MemberRole, SlotId, TopologyEpoch};
use dwv_recovery::RecoveryGeneration;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconstructionRangeEvidence {
    ParityClean,
    ReplayProven,
    Dirty,
    Indeterminate,
    Uncovered,
}

impl ReconstructionRangeEvidence {
    const fn authorizes(self) -> bool {
        matches!(self, Self::ParityClean | Self::ReplayProven)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconstructionSourceState {
    Available,
    Missing,
    ExcludedByIntegrity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnownErasureAuthorization {
    array_id: ArrayId,
    topology_epoch: TopologyEpoch,
    recovery_generation: RecoveryGeneration,
    missing_slot: usize,
    stable_slot: SlotId,
    coding_position: CodingPosition,
    range: ByteRange,
    geometry: Geometry,
    data_identities: Vec<Option<VerificationIdentity>>,
    parity_identity: VerificationIdentity,
}

impl KnownErasureAuthorization {
    pub const fn array_id(&self) -> ArrayId {
        self.array_id
    }

    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch
    }

    pub const fn recovery_generation(&self) -> RecoveryGeneration {
        self.recovery_generation
    }

    pub const fn missing_slot(&self) -> usize {
        self.missing_slot
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

    pub const fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    pub(crate) fn aliases_identity(&self, identity: VerificationIdentity) -> bool {
        self.parity_identity == identity
            || self
                .data_identities
                .iter()
                .flatten()
                .any(|source| *source == identity)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DegradedReadTelemetry {
    pub array_id: ArrayId,
    pub topology_epoch: TopologyEpoch,
    pub recovery_generation: RecoveryGeneration,
    pub stable_slot: SlotId,
    pub coding_position: CodingPosition,
    pub range: ByteRange,
    pub source_count: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DegradedReadOutcome {
    pub bytes: Vec<u8>,
    pub telemetry: DegradedReadTelemetry,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DegradedReadError {
    UnsupportedProfile,
    GeometryMismatch,
    RangeOutsideMissingSlot,
    RangeNotReconstructable(ReconstructionRangeEvidence),
    WritesNotQuiesced,
    SourceCountMismatch,
    ErasureCount {
        actual: usize,
    },
    MissingRoleMismatch,
    ExcludedSource {
        position: usize,
    },
    AliasedSource,
    StaleTopology {
        expected: TopologyEpoch,
        actual: TopologyEpoch,
    },
    StaleRecoveryGeneration {
        expected: RecoveryGeneration,
        actual: RecoveryGeneration,
    },
    SourceIdentityChanged,
    Store(String),
    Codec(String),
}

impl fmt::Display for DegradedReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProfile => {
                formatter.write_str("known-erasure reads require single XOR parity")
            }
            Self::GeometryMismatch => {
                formatter.write_str("codec geometry does not match the validated topology")
            }
            Self::RangeOutsideMissingSlot => {
                formatter.write_str("requested range is outside the missing slot")
            }
            Self::RangeNotReconstructable(evidence) => {
                write!(
                    formatter,
                    "range evidence {evidence:?} does not authorize reconstruction"
                )
            }
            Self::WritesNotQuiesced => {
                formatter.write_str("known-erasure reads require quiesced writes")
            }
            Self::SourceCountMismatch => {
                formatter.write_str("source state does not match the coding profile")
            }
            Self::ErasureCount { actual } => write!(
                formatter,
                "single parity requires exactly one known erasure, got {actual}"
            ),
            Self::MissingRoleMismatch => {
                formatter.write_str("the known erasure is not the missing data role")
            }
            Self::ExcludedSource { position } => write!(
                formatter,
                "source position {position} is excluded by current integrity evidence"
            ),
            Self::AliasedSource => {
                formatter.write_str("two reconstruction sources have the same identity")
            }
            Self::StaleTopology { expected, actual } => write!(
                formatter,
                "topology changed from {expected:?} to {actual:?}"
            ),
            Self::StaleRecoveryGeneration { expected, actual } => write!(
                formatter,
                "recovery generation changed from {expected:?} to {actual:?}"
            ),
            Self::SourceIdentityChanged => {
                formatter.write_str("reconstruction source identity changed after authorization")
            }
            Self::Store(message) => write!(formatter, "degraded read store failure: {message}"),
            Self::Codec(message) => write!(formatter, "degraded read codec failure: {message}"),
        }
    }
}

impl std::error::Error for DegradedReadError {}

#[allow(clippy::too_many_arguments)]
pub fn authorize_known_erasure<S: VerificationStore>(
    topology: &dwv_core::TopologySnapshot,
    recovery_generation: RecoveryGeneration,
    geometry: Geometry,
    range: ByteRange,
    range_evidence: ReconstructionRangeEvidence,
    writes_quiesced: bool,
    data: &[Option<&S>],
    parity: &S,
    source_states: &[ReconstructionSourceState],
) -> Result<KnownErasureAuthorization, DegradedReadError> {
    if topology.profile().parity_slots() != 1 {
        return Err(DegradedReadError::UnsupportedProfile);
    }
    if geometry.data_count() != usize::from(topology.profile().data_slots())
        || geometry.parity_length() != topology.geometry().parity_length()
        || data.len() != geometry.data_count()
        || source_states.len() != data.len() + 1
    {
        return Err(DegradedReadError::GeometryMismatch);
    }
    if !range_evidence.authorizes() {
        return Err(DegradedReadError::RangeNotReconstructable(range_evidence));
    }
    if !writes_quiesced {
        return Err(DegradedReadError::WritesNotQuiesced);
    }

    let missing = data
        .iter()
        .enumerate()
        .filter_map(|(slot, source)| source.is_none().then_some(slot))
        .collect::<Vec<_>>();
    if missing.len() != 1 {
        return Err(DegradedReadError::ErasureCount {
            actual: missing.len(),
        });
    }
    let missing_slot = missing[0];
    if source_states[missing_slot] != ReconstructionSourceState::Missing
        || source_states[data.len()] != ReconstructionSourceState::Available
        || data.iter().enumerate().any(|(slot, source)| {
            slot != missing_slot
                && (source.is_none() || source_states[slot] != ReconstructionSourceState::Available)
        })
    {
        if let Some(position) = source_states
            .iter()
            .position(|state| *state == ReconstructionSourceState::ExcludedByIntegrity)
        {
            return Err(DegradedReadError::ExcludedSource { position });
        }
        return Err(DegradedReadError::MissingRoleMismatch);
    }
    if range.is_empty()
        || geometry
            .data_length(missing_slot)
            .is_none_or(|length| range.end() > length)
    {
        return Err(DegradedReadError::RangeOutsideMissingSlot);
    }

    let coding_position = CodingPosition(
        u16::try_from(missing_slot).map_err(|_| DegradedReadError::GeometryMismatch)?,
    );
    let assignment = topology
        .assignment_for_position(coding_position)
        .filter(|assignment| assignment.role() == MemberRole::Data)
        .ok_or(DegradedReadError::MissingRoleMismatch)?;
    let data_identities = data
        .iter()
        .map(|source| source.map(VerificationStore::identity))
        .collect::<Vec<_>>();
    let parity_identity = parity.identity();
    let mut identities = data_identities
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    identities.push(parity_identity);
    for (index, identity) in identities.iter().enumerate() {
        if identities[..index].contains(identity) {
            return Err(DegradedReadError::AliasedSource);
        }
    }

    Ok(KnownErasureAuthorization {
        array_id: topology.array_id(),
        topology_epoch: topology.topology_epoch(),
        recovery_generation,
        missing_slot,
        stable_slot: assignment.slot_id(),
        coding_position,
        range,
        geometry,
        data_identities,
        parity_identity,
    })
}

pub fn read_known_erasure<S: VerificationStore>(
    authorization: &KnownErasureAuthorization,
    current_topology_epoch: TopologyEpoch,
    current_recovery_generation: RecoveryGeneration,
    data: &mut [Option<S>],
    parity: &mut S,
) -> Result<DegradedReadOutcome, DegradedReadError> {
    if current_topology_epoch != authorization.topology_epoch {
        return Err(DegradedReadError::StaleTopology {
            expected: authorization.topology_epoch,
            actual: current_topology_epoch,
        });
    }
    if current_recovery_generation != authorization.recovery_generation {
        return Err(DegradedReadError::StaleRecoveryGeneration {
            expected: authorization.recovery_generation,
            actual: current_recovery_generation,
        });
    }
    if data.len() != authorization.data_identities.len()
        || parity.identity() != authorization.parity_identity
        || data
            .iter()
            .zip(&authorization.data_identities)
            .any(|(source, expected)| match (source, expected) {
                (Some(source), Some(expected)) => source.identity() != *expected,
                (None, None) => false,
                _ => true,
            })
    {
        return Err(DegradedReadError::SourceIdentityChanged);
    }

    let range = authorization.range;
    let mut survivor_bytes = Vec::with_capacity(data.len());
    for (slot, source) in data.iter_mut().enumerate() {
        if slot == authorization.missing_slot {
            survivor_bytes.push(None);
            continue;
        }
        let source = source
            .as_mut()
            .ok_or(DegradedReadError::SourceIdentityChanged)?;
        survivor_bytes.push(Some(read_survivor_range(
            source,
            &authorization.geometry,
            slot,
            range,
        )?));
    }
    let parity_bytes = parity
        .read_exact(range)
        .map_err(|error| DegradedReadError::Store(error.to_string()))?;
    if parity_bytes.len() as u64 != range.length {
        return Err(DegradedReadError::Store("short parity read".to_owned()));
    }

    let local_geometry = Geometry::new(vec![range.length; data.len()], range.length)
        .map_err(|error| DegradedReadError::Codec(error.to_string()))?;
    let local_range = dwv_codec::ByteRange::new(0, range.length)
        .map_err(|error| DegradedReadError::Codec(error.to_string()))?;
    let survivor_refs = survivor_bytes
        .iter()
        .map(|bytes| bytes.as_deref())
        .collect::<Vec<_>>();
    let bytes = XorReference
        .reconstruct(
            &local_geometry,
            &parity_bytes,
            authorization.missing_slot,
            local_range,
            &survivor_refs,
        )
        .map_err(|error| DegradedReadError::Codec(error.to_string()))?;
    let source_count = u16::try_from(data.len()).unwrap_or(u16::MAX);
    Ok(DegradedReadOutcome {
        bytes,
        telemetry: DegradedReadTelemetry {
            array_id: authorization.array_id,
            topology_epoch: authorization.topology_epoch,
            recovery_generation: authorization.recovery_generation,
            stable_slot: authorization.stable_slot,
            coding_position: authorization.coding_position,
            range,
            source_count,
        },
    })
}

fn read_survivor_range<S: VerificationStore>(
    source: &mut S,
    geometry: &Geometry,
    slot: usize,
    range: ByteRange,
) -> Result<Vec<u8>, DegradedReadError> {
    let output_len = usize::try_from(range.length)
        .map_err(|_| DegradedReadError::Store("range length does not fit memory".to_owned()))?;
    let member_length = geometry
        .data_length(slot)
        .ok_or(DegradedReadError::GeometryMismatch)?;
    if range.offset >= member_length {
        return Ok(vec![0; output_len]);
    }
    let physical_length = range.end().min(member_length) - range.offset;
    let physical_range = ByteRange::new(range.offset, physical_length)
        .map_err(|error| DegradedReadError::Store(error.to_string()))?;
    let bytes = source
        .read_exact(physical_range)
        .map_err(|error| DegradedReadError::Store(error.to_string()))?;
    if bytes.len() as u64 != physical_length {
        return Err(DegradedReadError::Store("short survivor read".to_owned()));
    }
    let mut output = vec![0; output_len];
    output[..bytes.len()].copy_from_slice(&bytes);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VerificationStoreError;
    use dwv_core::{
        AssignmentGeneration, AssignmentInstanceId, CodingProfile, ProtectedGeometry,
        TopologyAssignment, TopologySnapshot,
    };

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct MemoryStore {
        bytes: Vec<u8>,
        identity: VerificationIdentity,
        writes: usize,
        short: bool,
    }

    impl MemoryStore {
        fn new(bytes: Vec<u8>, identity: u8) -> Self {
            Self {
                bytes,
                identity: VerificationIdentity([identity; 16]),
                writes: 0,
                short: false,
            }
        }
    }

    impl VerificationStore for MemoryStore {
        fn identity(&self) -> VerificationIdentity {
            self.identity
        }

        fn read_exact(&mut self, range: ByteRange) -> Result<Vec<u8>, VerificationStoreError> {
            let start = usize::try_from(range.offset).unwrap();
            let mut end = start + usize::try_from(range.length).unwrap();
            if self.short && end > start {
                end -= 1;
            }
            self.bytes
                .get(start..end)
                .map(ToOwned::to_owned)
                .ok_or_else(|| VerificationStoreError::new("range outside memory store"))
        }

        fn write_exact(
            &mut self,
            range: ByteRange,
            bytes: &[u8],
        ) -> Result<(), VerificationStoreError> {
            let start = usize::try_from(range.offset).unwrap();
            self.bytes[start..start + bytes.len()].copy_from_slice(bytes);
            self.writes += 1;
            Ok(())
        }
    }

    fn topology() -> TopologySnapshot {
        let assignments = (0..4)
            .map(|position| {
                TopologyAssignment::new(
                    SlotId([position as u8 + 1; 16]),
                    if position < 3 {
                        MemberRole::Data
                    } else {
                        MemberRole::Parity
                    },
                    CodingPosition(position),
                    AssignmentInstanceId([position as u8 + 11; 16]),
                    AssignmentGeneration(1),
                )
            })
            .collect();
        TopologySnapshot::new(
            ArrayId([7; 16]),
            TopologyEpoch(4),
            CodingProfile::new(3, 1).unwrap(),
            ProtectedGeometry::new(8, 1).unwrap(),
            assignments,
        )
        .unwrap()
    }

    fn fixture(
        missing: usize,
    ) -> (
        Vec<Option<MemoryStore>>,
        MemoryStore,
        Geometry,
        Vec<Vec<u8>>,
    ) {
        let images = vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![20, 21, 22, 23],
            vec![30, 31, 32, 33, 34, 35, 36, 37],
        ];
        let geometry = Geometry::new(vec![8, 4, 8], 8).unwrap();
        let refs = images.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let parity = XorReference.compute_parity(&geometry, &refs).unwrap();
        let data = images
            .iter()
            .enumerate()
            .map(|(slot, bytes)| {
                (slot != missing).then(|| MemoryStore::new(bytes.clone(), slot as u8 + 1))
            })
            .collect();
        (data, MemoryStore::new(parity, 9), geometry, images)
    }

    fn authorize(
        data: &[Option<MemoryStore>],
        parity: &MemoryStore,
        geometry: Geometry,
        range: ByteRange,
    ) -> Result<KnownErasureAuthorization, DegradedReadError> {
        let refs = data.iter().map(Option::as_ref).collect::<Vec<_>>();
        let states = data
            .iter()
            .map(|source| {
                if source.is_some() {
                    ReconstructionSourceState::Available
                } else {
                    ReconstructionSourceState::Missing
                }
            })
            .chain([ReconstructionSourceState::Available])
            .collect::<Vec<_>>();
        authorize_known_erasure(
            &topology(),
            RecoveryGeneration(3),
            geometry,
            range,
            ReconstructionRangeEvidence::ParityClean,
            true,
            &refs,
            parity,
            &states,
        )
    }

    #[test]
    fn middle_range_reconstructs_exactly_without_writes() {
        let (mut data, mut parity, geometry, images) = fixture(0);
        let range = ByteRange::new(2, 4).unwrap();
        let authorization = authorize(&data, &parity, geometry, range).unwrap();
        let outcome = read_known_erasure(
            &authorization,
            TopologyEpoch(4),
            RecoveryGeneration(3),
            &mut data,
            &mut parity,
        )
        .unwrap();
        assert_eq!(outcome.bytes, images[0][2..6]);
        assert_eq!(outcome.telemetry.array_id, ArrayId([7; 16]));
        assert_eq!(outcome.telemetry.topology_epoch, TopologyEpoch(4));
        assert_eq!(outcome.telemetry.recovery_generation, RecoveryGeneration(3));
        assert_eq!(outcome.telemetry.stable_slot, SlotId([1; 16]));
        assert_eq!(outcome.telemetry.coding_position, CodingPosition(0));
        assert_eq!(outcome.telemetry.range, range);
        assert_eq!(outcome.telemetry.source_count, 3);
        assert!(data.iter().flatten().all(|source| source.writes == 0));
        assert_eq!(parity.writes, 0);
    }

    #[test]
    fn every_nonempty_range_of_every_data_slot_reconstructs_exactly() {
        for missing_slot in 0..3 {
            let (_, _, _, images) = fixture(missing_slot);
            let member_length = images[missing_slot].len();
            for offset in 0..member_length {
                for end in (offset + 1)..=member_length {
                    let (mut data, mut parity, geometry, _) = fixture(missing_slot);
                    let range = ByteRange::new(offset as u64, (end - offset) as u64).unwrap();
                    let authorization = authorize(&data, &parity, geometry, range).unwrap();
                    let outcome = read_known_erasure(
                        &authorization,
                        TopologyEpoch(4),
                        RecoveryGeneration(3),
                        &mut data,
                        &mut parity,
                    )
                    .unwrap();
                    assert_eq!(outcome.bytes, images[missing_slot][offset..end]);
                    assert!(data.iter().flatten().all(|source| source.writes == 0));
                    assert_eq!(parity.writes, 0);
                }
            }
        }
    }

    #[test]
    fn shorter_survivor_contributes_documented_zero_tail() {
        let (mut data, mut parity, geometry, images) = fixture(2);
        let range = ByteRange::new(4, 4).unwrap();
        let authorization = authorize(&data, &parity, geometry, range).unwrap();
        let outcome = read_known_erasure(
            &authorization,
            TopologyEpoch(4),
            RecoveryGeneration(3),
            &mut data,
            &mut parity,
        )
        .unwrap();
        assert_eq!(outcome.bytes, images[2][4..8]);
    }

    #[test]
    fn dirty_excluded_and_multiple_erasure_inputs_fail_closed() {
        let (data, parity, geometry, _) = fixture(0);
        let refs = data.iter().map(Option::as_ref).collect::<Vec<_>>();
        let mut states = vec![
            ReconstructionSourceState::Missing,
            ReconstructionSourceState::Available,
            ReconstructionSourceState::Available,
            ReconstructionSourceState::Available,
        ];
        for evidence in [
            ReconstructionRangeEvidence::Dirty,
            ReconstructionRangeEvidence::Indeterminate,
            ReconstructionRangeEvidence::Uncovered,
        ] {
            assert_eq!(
                authorize_known_erasure(
                    &topology(),
                    RecoveryGeneration(3),
                    geometry.clone(),
                    ByteRange::new(0, 4).unwrap(),
                    evidence,
                    true,
                    &refs,
                    &parity,
                    &states,
                ),
                Err(DegradedReadError::RangeNotReconstructable(evidence))
            );
        }
        states[1] = ReconstructionSourceState::ExcludedByIntegrity;
        assert!(matches!(
            authorize_known_erasure(
                &topology(),
                RecoveryGeneration(3),
                geometry.clone(),
                ByteRange::new(0, 4).unwrap(),
                ReconstructionRangeEvidence::ParityClean,
                true,
                &refs,
                &parity,
                &states,
            ),
            Err(DegradedReadError::ExcludedSource { position: 1 })
        ));
        let mut two_missing = data.clone();
        two_missing[1] = None;
        let refs = two_missing.iter().map(Option::as_ref).collect::<Vec<_>>();
        assert!(matches!(
            authorize_known_erasure(
                &topology(),
                RecoveryGeneration(3),
                geometry,
                ByteRange::new(0, 4).unwrap(),
                ReconstructionRangeEvidence::ParityClean,
                true,
                &refs,
                &parity,
                &[
                    ReconstructionSourceState::Missing,
                    ReconstructionSourceState::Missing,
                    ReconstructionSourceState::Available,
                    ReconstructionSourceState::Available
                ],
            ),
            Err(DegradedReadError::ErasureCount { actual: 2 })
        ));

        let (mut aliased, parity, geometry, _) = fixture(0);
        let duplicate_identity = aliased[2].as_ref().unwrap().identity;
        aliased[1].as_mut().unwrap().identity = duplicate_identity;
        let refs = aliased.iter().map(Option::as_ref).collect::<Vec<_>>();
        assert!(matches!(
            authorize_known_erasure(
                &topology(),
                RecoveryGeneration(3),
                geometry,
                ByteRange::new(0, 4).unwrap(),
                ReconstructionRangeEvidence::ParityClean,
                true,
                &refs,
                &parity,
                &[
                    ReconstructionSourceState::Missing,
                    ReconstructionSourceState::Available,
                    ReconstructionSourceState::Available,
                    ReconstructionSourceState::Available
                ],
            ),
            Err(DegradedReadError::AliasedSource)
        ));
    }

    #[test]
    fn stale_generation_identity_change_and_short_reads_fail_before_output() {
        let (mut data, mut parity, geometry, _) = fixture(0);
        let authorization =
            authorize(&data, &parity, geometry, ByteRange::new(0, 4).unwrap()).unwrap();
        assert!(matches!(
            read_known_erasure(
                &authorization,
                TopologyEpoch(4),
                RecoveryGeneration(4),
                &mut data,
                &mut parity,
            ),
            Err(DegradedReadError::StaleRecoveryGeneration { .. })
        ));
        data[1].as_mut().unwrap().identity = VerificationIdentity([88; 16]);
        assert!(matches!(
            read_known_erasure(
                &authorization,
                TopologyEpoch(4),
                RecoveryGeneration(3),
                &mut data,
                &mut parity,
            ),
            Err(DegradedReadError::SourceIdentityChanged)
        ));

        let (mut data, mut parity, geometry, _) = fixture(0);
        let authorization =
            authorize(&data, &parity, geometry, ByteRange::new(0, 4).unwrap()).unwrap();
        data[1].as_mut().unwrap().short = true;
        assert!(matches!(
            read_known_erasure(
                &authorization, TopologyEpoch(4), RecoveryGeneration(3), &mut data, &mut parity,
            ),
            Err(DegradedReadError::Store(message)) if message.contains("short")
        ));
    }
}
