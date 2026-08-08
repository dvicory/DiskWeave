//! Recovery-grounded file-source authorization for known erasures.

use dwv_codec::Geometry;
use dwv_core::{AssignmentInstanceId, ByteRange, CodingPosition, MemberRole, TopologySnapshot};
use dwv_recovery::{
    RecoveryError, RecoveryStateStore, RecoveryStoreHealth, RegionState,
    TopologySnapshot as RecoveryTopologySnapshot,
};
use dwv_store::{RandomAccessStore, StoreId};
use dwv_store_file::{FileStore, FileStoreConfig, FileStoreError};
use dwv_verify::{
    DegradedReadError, KnownErasureAuthorization, ReconstructionRangeEvidence,
    ReconstructionSourceState, VerificationIdentity, VerificationStore, VerificationStoreError,
    authorize_known_erasure,
};
use std::fmt;

/// Read-only rebuild/degraded source that holds `FileStore`'s exclusive lease.
/// The underlying writable handle is never exposed and payload writes are
/// rejected at this adapter boundary.
pub struct FileRebuildSource {
    store_id: StoreId,
    assignment_instance: AssignmentInstanceId,
    store: FileStore,
}

impl FileRebuildSource {
    pub fn open(
        config: FileStoreConfig,
        store_id: StoreId,
        assignment_instance: AssignmentInstanceId,
    ) -> Result<Self, FileStoreError> {
        let store = FileStore::open(config.writable(true).create(false).store_id(store_id))?;
        Ok(Self {
            store_id,
            assignment_instance,
            store,
        })
    }

    pub const fn store_id(&self) -> StoreId {
        self.store_id
    }

    pub const fn assignment_instance(&self) -> AssignmentInstanceId {
        self.assignment_instance
    }

    pub fn protected_length(&self) -> u64 {
        self.store.length()
    }
}

impl VerificationStore for FileRebuildSource {
    fn identity(&self) -> VerificationIdentity {
        VerificationIdentity(
            self.store
                .capabilities_report()
                .identity
                .observations
                .first()
                .map(|observation| observation.fingerprint)
                .unwrap_or([0; 16]),
        )
    }

    fn read_exact(&mut self, range: ByteRange) -> Result<Vec<u8>, VerificationStoreError> {
        self.store
            .read_bytes(range)
            .map_err(|error| VerificationStoreError::new(error.to_string()))
    }

    fn write_exact(
        &mut self,
        _range: ByteRange,
        _bytes: &[u8],
    ) -> Result<(), VerificationStoreError> {
        Err(VerificationStoreError::new(
            "offline reconstruction sources are read-only",
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OfflineAuthorizationError {
    Recovery(RecoveryError),
    RecoveryUnhealthy(RecoveryStoreHealth),
    NoActiveTopology,
    TopologyMismatch,
    SourceCountMismatch,
    SourceBindingMismatch { position: CodingPosition },
    RecoveryNotGloballyClean,
    UnsupportedHeterogeneousGeometry,
    Degraded(DegradedReadError),
}

impl fmt::Display for OfflineAuthorizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "offline reconstruction authorization refused: {self:?}"
        )
    }
}

impl std::error::Error for OfflineAuthorizationError {}

impl From<RecoveryError> for OfflineAuthorizationError {
    fn from(error: RecoveryError) -> Self {
        Self::Recovery(error)
    }
}

impl From<DegradedReadError> for OfflineAuthorizationError {
    fn from(error: DegradedReadError) -> Self {
        Self::Degraded(error)
    }
}

/// Produces the portable authorization only from a healthy, active, globally
/// clean recovery snapshot and exact source-to-assignment mappings. Until a
/// range-indexed recovery evidence API exists, any dirty/indeterminate record
/// conservatively refuses every range.
pub fn authorize_file_known_erasure<R: RecoveryStateStore>(
    recovery: &R,
    topology: &TopologySnapshot,
    geometry: Geometry,
    range: ByteRange,
    data: &[Option<&FileRebuildSource>],
    parity: &FileRebuildSource,
) -> Result<KnownErasureAuthorization, OfflineAuthorizationError> {
    let health = recovery.verify_integrity();
    if health != RecoveryStoreHealth::Healthy {
        return Err(OfflineAuthorizationError::RecoveryUnhealthy(health));
    }
    let snapshot = recovery.load_assembly_snapshot()?;
    let active = snapshot
        .active_topology
        .as_ref()
        .ok_or(OfflineAuthorizationError::NoActiveTopology)?;
    validate_topology(active, topology)?;
    if snapshot
        .writable_session
        .as_ref()
        .is_some_and(|session| !session.closed)
        || snapshot
            .dirty_regions
            .iter()
            .any(|record| record.state != RegionState::Clean)
    {
        return Err(OfflineAuthorizationError::RecoveryNotGloballyClean);
    }

    let data_count = usize::from(topology.profile().data_slots());
    if data.len() != data_count
        || geometry.data_count() != data_count
        || geometry.parity_length() != topology.geometry().parity_length()
    {
        return Err(OfflineAuthorizationError::SourceCountMismatch);
    }
    let protected_length = topology.geometry().protected_length();
    if geometry
        .data_lengths()
        .iter()
        .any(|length| *length != protected_length)
    {
        return Err(OfflineAuthorizationError::UnsupportedHeterogeneousGeometry);
    }

    let mut states = Vec::with_capacity(data_count + 1);
    for (position, source) in data.iter().enumerate() {
        let position = CodingPosition(
            u16::try_from(position).map_err(|_| OfflineAuthorizationError::SourceCountMismatch)?,
        );
        let assignment = active
            .assignments()
            .iter()
            .find(|assignment| assignment.coding_position() == position)
            .filter(|assignment| assignment.role() == MemberRole::Data)
            .ok_or(OfflineAuthorizationError::SourceBindingMismatch { position })?;
        match source {
            Some(source)
                if source.store_id() == assignment.store_id()
                    && source.assignment_instance() == assignment.assignment_instance()
                    && source.protected_length() == protected_length =>
            {
                states.push(ReconstructionSourceState::Available);
            }
            None => states.push(ReconstructionSourceState::Missing),
            Some(_) => {
                return Err(OfflineAuthorizationError::SourceBindingMismatch { position });
            }
        }
    }
    let parity_position = CodingPosition(
        u16::try_from(data_count).map_err(|_| OfflineAuthorizationError::SourceCountMismatch)?,
    );
    let parity_assignment = active
        .assignments()
        .iter()
        .find(|assignment| assignment.coding_position() == parity_position)
        .filter(|assignment| assignment.role() == MemberRole::Parity)
        .ok_or(OfflineAuthorizationError::SourceBindingMismatch {
            position: parity_position,
        })?;
    if parity.store_id() != parity_assignment.store_id()
        || parity.assignment_instance() != parity_assignment.assignment_instance()
        || parity.protected_length() != geometry.parity_length()
    {
        return Err(OfflineAuthorizationError::SourceBindingMismatch {
            position: parity_position,
        });
    }
    states.push(ReconstructionSourceState::Available);

    authorize_known_erasure(
        topology,
        snapshot.generation,
        geometry,
        range,
        ReconstructionRangeEvidence::ParityClean,
        true,
        data,
        parity,
        &states,
    )
    .map_err(Into::into)
}

fn validate_topology(
    recovery: &RecoveryTopologySnapshot,
    topology: &TopologySnapshot,
) -> Result<(), OfflineAuthorizationError> {
    let matches = recovery.array_id() == topology.array_id()
        && recovery.topology_epoch() == topology.topology_epoch()
        && recovery.profile() == topology.profile()
        && recovery.geometry() == topology.geometry()
        && recovery.assignments().len() == topology.assignments().len()
        && recovery.assignments().iter().all(|assignment| {
            topology
                .assignment_for_position(assignment.coding_position())
                .is_some_and(|core| {
                    core.slot_id() == assignment.slot_id()
                        && core.role() == assignment.role()
                        && core.assignment_instance() == assignment.assignment_instance()
                        && core.assignment_generation() == assignment.assignment_generation()
                })
        });
    if matches {
        Ok(())
    } else {
        Err(OfflineAuthorizationError::TopologyMismatch)
    }
}
