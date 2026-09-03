use crate::capabilities::{
    CapabilityProbeError, FileCapabilityReport, SparseBehavior, probe_file_capabilities,
};
use crate::lease::{
    FileIdentityError, FileLease, FileLeaseError, IdentityComparison, compare_identity,
    identity_from_metadata, observe_file_identity,
};
use dwv_store::{
    BufferToken, ByteRange, CapabilityEvidenceId, ChildOperationId, CompletedRangeSet,
    CompletionDisposition, IdentityObservationSet, OperationId, PersistenceEvidence,
    RandomAccessStore, StoreCapabilities, StoreCompletion, StoreError, StoreId, StoreIncarnationId,
    StoreOperation, StoreRequest, StoreRequestKind, StoreWriteWatermark, TopologyEpoch,
    WriteIntent,
};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::{fmt, mem};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileSyncMode {
    CallerFlush,
    SyncData,
    SyncAll,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncEvidence {
    HostFileSync,
    Uncertain,
}

#[derive(Clone, Debug)]
pub struct FileStoreConfig {
    pub path: PathBuf,
    pub protected_length: u64,
    pub logical_block_size: u32,
    pub maximum_transfer: u64,
    pub writable: bool,
    pub create: bool,
    pub create_new: bool,
    pub sparse: bool,
    pub sync_mode: FileSyncMode,
    pub store_id: StoreId,
    pub topology_epoch: TopologyEpoch,
    pub lock_path: Option<PathBuf>,
}

impl FileStoreConfig {
    pub fn new(path: impl Into<PathBuf>, protected_length: u64, logical_block_size: u32) -> Self {
        Self {
            path: path.into(),
            protected_length,
            logical_block_size,
            maximum_transfer: protected_length.max(u64::from(logical_block_size)),
            writable: true,
            create: false,
            create_new: false,
            sparse: false,
            sync_mode: FileSyncMode::CallerFlush,
            store_id: StoreId(0),
            topology_epoch: TopologyEpoch(0),
            lock_path: None,
        }
    }

    pub fn maximum_transfer(mut self, value: u64) -> Self {
        self.maximum_transfer = value;
        self
    }
    pub fn create(mut self, value: bool) -> Self {
        self.create = value;
        self
    }
    pub fn create_new(mut self, value: bool) -> Self {
        self.create_new = value;
        self
    }
    pub fn writable(mut self, value: bool) -> Self {
        self.writable = value;
        self
    }
    pub fn sparse(mut self, value: bool) -> Self {
        self.sparse = value;
        self
    }
    pub fn sync_mode(mut self, value: FileSyncMode) -> Self {
        self.sync_mode = value;
        self
    }
    pub fn store_id(mut self, value: StoreId) -> Self {
        self.store_id = value;
        self
    }
    pub fn topology_epoch(mut self, value: TopologyEpoch) -> Self {
        self.topology_epoch = value;
        self
    }
    pub fn lock_path(mut self, value: impl Into<PathBuf>) -> Self {
        self.lock_path = Some(value.into());
        self
    }
}

#[derive(Debug)]
pub enum FileStoreError {
    Io(io::Error),
    Store(StoreError),
    Lease(FileLeaseError),
    Identity(FileIdentityError),
    Capability(CapabilityProbeError),
    InvalidGeometry(String),
}

impl fmt::Display for FileStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "file store I/O failed: {error}"),
            Self::Store(error) => write!(formatter, "file store request failed: {error}"),
            Self::Lease(error) => write!(formatter, "file store lease failed: {error}"),
            Self::Identity(error) => write!(formatter, "file store identity failed: {error}"),
            Self::Capability(error) => write!(formatter, "file store capability failed: {error}"),
            Self::InvalidGeometry(message) => {
                write!(formatter, "invalid file store geometry: {message}")
            }
        }
    }
}

impl std::error::Error for FileStoreError {}
impl From<io::Error> for FileStoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<FileLeaseError> for FileStoreError {
    fn from(error: FileLeaseError) -> Self {
        Self::Lease(error)
    }
}
impl From<FileIdentityError> for FileStoreError {
    fn from(error: FileIdentityError) -> Self {
        Self::Identity(error)
    }
}
impl From<CapabilityProbeError> for FileStoreError {
    fn from(error: CapabilityProbeError) -> Self {
        Self::Capability(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadProgress {
    pub bytes: Vec<u8>,
    pub completed: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileOwnershipResource {
    BackingPayload,
    LeaseMarker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileWriterClaimDisposition {
    Released,
    Held,
    Uncertain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileReleaseCauseKind {
    OperationFailed,
    ObservationLost,
    Unclassifiable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileReleaseCause {
    resource: FileOwnershipResource,
    kind: FileReleaseCauseKind,
    os_error_code: Option<i32>,
    message: String,
}

impl FileReleaseCause {
    fn unlock_failed(resource: FileOwnershipResource, error: &io::Error) -> Self {
        Self {
            resource,
            kind: FileReleaseCauseKind::OperationFailed,
            os_error_code: error.raw_os_error(),
            message: error.to_string(),
        }
    }

    fn marker_release_failed(error: &FileLeaseError) -> Self {
        match error {
            FileLeaseError::Io(error) => {
                Self::unlock_failed(FileOwnershipResource::LeaseMarker, error)
            }
            FileLeaseError::AlreadyHeld(path) => Self {
                resource: FileOwnershipResource::LeaseMarker,
                kind: FileReleaseCauseKind::OperationFailed,
                os_error_code: None,
                message: format!(
                    "lease marker unexpectedly reported held: {}",
                    path.display()
                ),
            },
        }
    }

    pub const fn resource(&self) -> FileOwnershipResource {
        self.resource
    }

    pub const fn kind(&self) -> FileReleaseCauseKind {
        self.kind
    }

    pub const fn os_error_code(&self) -> Option<i32> {
        self.os_error_code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Identifier unique only within one live file-backed owner process.
///
/// It is not persisted or interpreted after process death.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileWriterAcquisitionId(u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileWriterClaimToken {
    store_id: StoreId,
    incarnation: StoreIncarnationId,
    topology_epoch: TopologyEpoch,
    acquisition_id: FileWriterAcquisitionId,
}

impl FileWriterClaimToken {
    pub const fn store_id(&self) -> StoreId {
        self.store_id
    }

    pub const fn incarnation(&self) -> StoreIncarnationId {
        self.incarnation
    }

    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch
    }

    pub const fn acquisition_id(&self) -> &FileWriterAcquisitionId {
        &self.acquisition_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileWriterAliasBinding {
    backing_path: PathBuf,
    lease_path: PathBuf,
}

impl FileWriterAliasBinding {
    pub fn backing_path(&self) -> &Path {
        &self.backing_path
    }

    pub fn lease_path(&self) -> &Path {
        &self.lease_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileWriterClaimBinding {
    token: FileWriterClaimToken,
    backing_identity: IdentityObservationSet,
    alias_binding: FileWriterAliasBinding,
    protected_length: u64,
    logical_block_size: u32,
}

impl FileWriterClaimBinding {
    pub const fn token(&self) -> &FileWriterClaimToken {
        &self.token
    }

    pub const fn backing_identity(&self) -> &IdentityObservationSet {
        &self.backing_identity
    }

    pub const fn alias_binding(&self) -> &FileWriterAliasBinding {
        &self.alias_binding
    }

    pub const fn protected_length(&self) -> u64 {
        self.protected_length
    }

    pub const fn logical_block_size(&self) -> u32 {
        self.logical_block_size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileWriterClaimReleaseObservation {
    binding: FileWriterClaimBinding,
    disposition: FileWriterClaimDisposition,
    cause: Option<FileReleaseCause>,
}

impl FileWriterClaimReleaseObservation {
    pub const fn binding(&self) -> &FileWriterClaimBinding {
        &self.binding
    }

    pub const fn disposition(&self) -> FileWriterClaimDisposition {
        self.disposition
    }

    pub const fn cause(&self) -> Option<&FileReleaseCause> {
        self.cause.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileBackedReleaseDisposition {
    ReleasedAll,
    Residual,
    Uncertain,
}

/// Owner-issued evidence for one explicit release attempt.
///
/// This value describes ownership; it never carries or replaces live ownership
/// resources. An incomplete result keeps those resources in
/// [`UnresolvedFileWriterSet`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileBackedReleaseObservation {
    members: Vec<FileWriterClaimReleaseObservation>,
    disposition: FileBackedReleaseDisposition,
}

impl FileBackedReleaseObservation {
    pub fn members(&self) -> &[FileWriterClaimReleaseObservation] {
        &self.members
    }

    pub const fn disposition(&self) -> FileBackedReleaseDisposition {
        self.disposition
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileWriterSetAcceptanceFailure {
    EmptyExpectedSet,
    DuplicateExpectedStore,
    DuplicateStore,
    MemberSetMismatch,
    AlreadyAccepted,
    ReadOnlyMember,
}

#[derive(Debug)]
pub struct FileWriterSetAcceptanceError {
    failure: FileWriterSetAcceptanceFailure,
    stores: Vec<FileStore>,
}

impl FileWriterSetAcceptanceError {
    pub const fn failure(&self) -> FileWriterSetAcceptanceFailure {
        self.failure
    }

    pub fn into_stores(self) -> Vec<FileStore> {
        self.stores
    }
}

impl fmt::Display for FileWriterSetAcceptanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "file writer set acceptance failed: {:?}",
            self.failure
        )
    }
}

impl std::error::Error for FileWriterSetAcceptanceError {}

/// One exact accepted set retaining the original writable stores and claims.
///
/// Store operations remain available through [`Self::store_mut`]. Explicit
/// release consumes this value. Drop transfers its exact ownership to the
/// process-local file-owner quarantine.
#[must_use = "release or quarantine the accepted set; dropping it transfers custody to quarantine"]
pub struct AcceptedFileWriterSet {
    claims: Vec<FileWriterClaimBinding>,
    stores: Vec<FileStore>,
}

impl fmt::Debug for AcceptedFileWriterSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AcceptedFileWriterSet")
            .field("members", &self.claims.len())
            .finish()
    }
}

/// dwv:req req.file-backed-stores.post-acquisition-writer-claim-release-is-exact-and-owner-observed
impl AcceptedFileWriterSet {
    pub fn accept(
        expected_store_ids: impl IntoIterator<Item = StoreId>,
        stores: Vec<FileStore>,
    ) -> Result<Self, FileWriterSetAcceptanceError> {
        let mut expected = expected_store_ids.into_iter().collect::<Vec<_>>();
        expected.sort_unstable_by_key(|store_id| store_id.0);
        let failure = if expected.is_empty() {
            Some(FileWriterSetAcceptanceFailure::EmptyExpectedSet)
        } else if expected.windows(2).any(|ids| ids[0] == ids[1]) {
            Some(FileWriterSetAcceptanceFailure::DuplicateExpectedStore)
        } else if stores.iter().any(|store| !store.handle.is_writable()) {
            Some(FileWriterSetAcceptanceFailure::ReadOnlyMember)
        } else {
            let mut actual = stores.iter().map(FileStore::store_id).collect::<Vec<_>>();
            actual.sort_unstable_by_key(|store_id| store_id.0);
            if actual.windows(2).any(|ids| ids[0] == ids[1]) {
                Some(FileWriterSetAcceptanceFailure::DuplicateStore)
            } else if actual != expected {
                Some(FileWriterSetAcceptanceFailure::MemberSetMismatch)
            } else {
                None
            }
        };
        if let Some(failure) = failure {
            return Err(FileWriterSetAcceptanceError { failure, stores });
        }
        let mut claims = stores
            .iter()
            .map(|store| {
                store
                    .writer_claim_binding()
                    .expect("accepted writer set members are writable")
            })
            .collect::<Vec<_>>();
        claims.sort_unstable_by_key(|binding| binding.token.store_id.0);
        if let Err(failure) = mark_file_writer_set_accepted(&claims) {
            return Err(FileWriterSetAcceptanceError { failure, stores });
        }
        Ok(Self { claims, stores })
    }

    pub fn stores(&self) -> &[FileStore] {
        &self.stores
    }

    pub fn store(&self, store_id: StoreId) -> Option<&FileStore> {
        self.stores
            .iter()
            .find(|store| store.store_id() == store_id)
    }

    /// Borrows one member through a non-extractable store-operation guard.
    pub fn store_mut(&mut self, store_id: StoreId) -> Option<AcceptedFileStore<'_>> {
        self.stores
            .iter_mut()
            .find(|store| store.store_id() == store_id)
            .map(|store| AcceptedFileStore { store })
    }

    pub fn release(mut self) -> FileWriterSetReleaseResult {
        self.take_unresolved()
            .expect("accepted writer sets contain live claims")
            .retry_release()
    }

    fn take_unresolved(&mut self) -> Option<UnresolvedFileWriterSet> {
        if self.stores.is_empty() {
            return None;
        }
        let observations = self
            .claims
            .iter()
            .cloned()
            .map(|binding| FileWriterClaimReleaseObservation {
                binding,
                disposition: FileWriterClaimDisposition::Held,
                cause: None,
            })
            .collect();
        self.claims.clear();
        Some(UnresolvedFileWriterSet {
            claims: mem::take(&mut self.stores)
                .into_iter()
                .map(FileStore::into_held_writer_claim)
                .collect(),
            released_reservations: Vec::new(),
            released: Vec::new(),
            observation: FileBackedReleaseObservation {
                members: observations,
                disposition: FileBackedReleaseDisposition::Residual,
            },
        })
    }
}

impl Drop for AcceptedFileWriterSet {
    fn drop(&mut self) {
        if let Some(unresolved) = self.take_unresolved() {
            let _ = unresolved.quarantine();
        }
    }
}

/// Mutable store-operation access that cannot move a store out of its accepted set.
pub struct AcceptedFileStore<'a> {
    store: &'a mut FileStore,
}

impl AcceptedFileStore<'_> {
    pub fn as_store(&self) -> &FileStore {
        self.store
    }
}

impl RandomAccessStore for AcceptedFileStore<'_> {
    fn store_id(&self) -> StoreId {
        self.store.store_id()
    }

    fn topology_epoch(&self) -> TopologyEpoch {
        self.store.topology_epoch()
    }

    fn incarnation(&self) -> StoreIncarnationId {
        self.store.incarnation()
    }

    fn identity_observations(&self) -> IdentityObservationSet {
        self.store.identity.clone()
    }

    fn current_identity_observations(&self) -> Result<IdentityObservationSet, StoreError> {
        <FileStore as RandomAccessStore>::current_identity_observations(self.store)
    }

    fn capabilities(&self) -> StoreCapabilities {
        self.store.capabilities.clone()
    }

    fn length(&self) -> u64 {
        self.store.config.protected_length
    }

    fn highest_accepted_watermark(&self) -> Option<StoreWriteWatermark> {
        self.store.highest_accepted_watermark
    }

    fn read_at(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        destination: &mut [u8],
    ) -> StoreCompletion {
        <FileStore as RandomAccessStore>::read_at(self.store, operation_id, range, destination)
    }

    fn write_at(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        source: &[u8],
        intent: WriteIntent,
    ) -> StoreCompletion {
        <FileStore as RandomAccessStore>::write_at(self.store, operation_id, range, source, intent)
    }

    fn flush(
        &mut self,
        operation_id: ChildOperationId,
        through: StoreWriteWatermark,
    ) -> StoreCompletion {
        <FileStore as RandomAccessStore>::flush(self.store, operation_id, through)
    }

    fn write_zeroes(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        intent: WriteIntent,
    ) -> StoreCompletion {
        <FileStore as RandomAccessStore>::write_zeroes(self.store, operation_id, range, intent)
    }

    fn discard(&mut self, operation_id: ChildOperationId, range: ByteRange) -> StoreCompletion {
        <FileStore as RandomAccessStore>::discard(self.store, operation_id, range)
    }
}

/// Owner-issued proof that every claim in one accepted set was released.
///
/// Its private construction prevents a residual or uncertain observation from
/// being relabeled as successful release.
#[must_use = "released-all proof must be consumed by the ownership coordinator"]
pub struct ReleasedFileWriterSet {
    observation: FileBackedReleaseObservation,
}

impl ReleasedFileWriterSet {
    pub const fn observation(&self) -> &FileBackedReleaseObservation {
        &self.observation
    }
}

/// `Ok` is owner-issued released-all proof; `Err` retains unresolved ownership.
pub type FileWriterSetReleaseResult = Result<ReleasedFileWriterSet, UnresolvedFileWriterSet>;

/// Same-owner capability retaining every unresolved claim resource and the
/// lightweight process reservations for members already definitely released.
///
/// It exposes observations but no writable store access or fresh-acquisition
/// authority. Consume it with [`Self::retry_release`] to retry the same claims.
#[must_use = "retry or quarantine unresolved ownership; dropping it transfers custody to quarantine"]
pub struct UnresolvedFileWriterSet {
    claims: Vec<HeldFileWriterClaim>,
    released: Vec<FileWriterClaimReleaseObservation>,
    released_reservations: Vec<ProcessFileClaim>,
    observation: FileBackedReleaseObservation,
}

impl fmt::Debug for UnresolvedFileWriterSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UnresolvedFileWriterSet")
            .field("unresolved_members", &self.claims.len())
            .field("observation", &self.observation)
            .finish()
    }
}

impl UnresolvedFileWriterSet {
    pub const fn observation(&self) -> &FileBackedReleaseObservation {
        &self.observation
    }

    /// Transfers unresolved resources to the process-local file-owner quarantine.
    pub fn quarantine(mut self) -> FileWriterQuarantineId {
        quarantine_unresolved(&mut self)
            .expect("unresolved writer sets always contain at least one live claim")
    }

    /// Retries release of only the retained exact claims.
    pub fn retry_release(mut self) -> FileWriterSetReleaseResult {
        let claims = mem::take(&mut self.claims);
        let mut released = mem::take(&mut self.released);
        let mut released_reservations = mem::take(&mut self.released_reservations);
        let mut unresolved = Vec::new();
        let mut observations = Vec::new();
        for claim in claims {
            match claim.release() {
                HeldFileWriterClaimRelease::Released {
                    observation,
                    process_claim,
                } => {
                    released.push(observation);
                    released_reservations.push(process_claim);
                }
                HeldFileWriterClaimRelease::Unresolved { claim, observation } => {
                    observations.push(observation);
                    unresolved.push(claim);
                }
            }
        }
        observations.extend(released.iter().cloned());
        observations.sort_unstable_by_key(|observation| observation.binding.token.store_id.0);
        if unresolved.is_empty() {
            let released_set = ReleasedFileWriterSet {
                observation: FileBackedReleaseObservation {
                    members: observations,
                    disposition: FileBackedReleaseDisposition::ReleasedAll,
                },
            };
            drop(released_reservations);
            return Ok(released_set);
        }
        let disposition = if observations
            .iter()
            .any(|member| member.disposition == FileWriterClaimDisposition::Held)
        {
            FileBackedReleaseDisposition::Residual
        } else {
            FileBackedReleaseDisposition::Uncertain
        };
        Err(Self {
            claims: unresolved,
            released,
            released_reservations,
            observation: FileBackedReleaseObservation {
                members: observations,
                disposition,
            },
        })
    }

    fn take_quarantined(
        &mut self,
        id: Option<FileWriterQuarantineId>,
    ) -> Option<QuarantinedFileWriterSet> {
        let first_acquisition_id = self
            .claims
            .iter()
            .map(|claim| claim.binding.token.acquisition_id.0)
            .min()?;
        let id = id.unwrap_or(FileWriterQuarantineId(first_acquisition_id));
        Some(QuarantinedFileWriterSet {
            id,
            claims: mem::take(&mut self.claims),
            released: mem::take(&mut self.released),
            released_reservations: mem::take(&mut self.released_reservations),
            observation: mem::replace(
                &mut self.observation,
                FileBackedReleaseObservation {
                    members: Vec::new(),
                    disposition: FileBackedReleaseDisposition::Uncertain,
                },
            ),
        })
    }
}

impl Drop for UnresolvedFileWriterSet {
    fn drop(&mut self) {
        let _ = quarantine_unresolved(self);
    }
}

/// Process-local locator for an explicitly owned quarantined writer set.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[must_use = "retain the ID to retry this quarantined release directly"]
pub struct FileWriterQuarantineId(u64);

/// Descriptive snapshot of one process-local quarantined writer set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileWriterQuarantineStatus {
    id: FileWriterQuarantineId,
    observation: FileBackedReleaseObservation,
}

impl FileWriterQuarantineStatus {
    pub const fn id(&self) -> FileWriterQuarantineId {
        self.id
    }

    pub const fn observation(&self) -> &FileBackedReleaseObservation {
        &self.observation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileWriterQuarantineRetryFailure {
    NotFound,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileWriterQuarantineRetryError {
    failure: FileWriterQuarantineRetryFailure,
    observation: Option<FileBackedReleaseObservation>,
}

impl FileWriterQuarantineRetryError {
    pub const fn failure(&self) -> FileWriterQuarantineRetryFailure {
        self.failure
    }

    pub const fn observation(&self) -> Option<&FileBackedReleaseObservation> {
        self.observation.as_ref()
    }
}

impl fmt::Display for FileWriterQuarantineRetryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "quarantined file writer release retry failed: {:?}",
            self.failure
        )
    }
}

impl std::error::Error for FileWriterQuarantineRetryError {}

/// Returns descriptive snapshots of process-local quarantined ownership.
pub fn quarantined_file_writer_sets() -> Vec<FileWriterQuarantineStatus> {
    let registry = file_writer_quarantine_registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    registry
        .entries
        .iter()
        .map(|entry| FileWriterQuarantineStatus {
            id: entry.id,
            observation: entry.observation.clone(),
        })
        .collect()
}

/// Retries one quarantined release without holding the quarantine lock.
pub fn retry_quarantined_file_writer_set(
    id: FileWriterQuarantineId,
) -> Result<ReleasedFileWriterSet, FileWriterQuarantineRetryError> {
    let quarantined = {
        let mut registry = file_writer_quarantine_registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(index) = registry.entries.iter().position(|entry| entry.id == id) else {
            return Err(FileWriterQuarantineRetryError {
                failure: FileWriterQuarantineRetryFailure::NotFound,
                observation: None,
            });
        };
        registry.entries.swap_remove(index)
    };
    match quarantined.into_unresolved().retry_release() {
        Ok(released) => Ok(released),
        Err(mut unresolved) => {
            let observation = unresolved.observation.clone();
            let reinserted = quarantine_unresolved_with_id(&mut unresolved, Some(id))
                .expect("failed quarantine retry retains a live claim");
            debug_assert_eq!(reinserted, id);
            Err(FileWriterQuarantineRetryError {
                failure: FileWriterQuarantineRetryFailure::Unresolved,
                observation: Some(observation),
            })
        }
    }
}

struct QuarantinedFileWriterSet {
    id: FileWriterQuarantineId,
    claims: Vec<HeldFileWriterClaim>,
    released: Vec<FileWriterClaimReleaseObservation>,
    released_reservations: Vec<ProcessFileClaim>,
    observation: FileBackedReleaseObservation,
}

impl QuarantinedFileWriterSet {
    fn into_unresolved(self) -> UnresolvedFileWriterSet {
        UnresolvedFileWriterSet {
            claims: self.claims,
            released: self.released,
            released_reservations: self.released_reservations,
            observation: self.observation,
        }
    }
}

#[derive(Default)]
struct FileWriterQuarantineRegistry {
    entries: Vec<QuarantinedFileWriterSet>,
}

static FILE_WRITER_QUARANTINE_REGISTRY: LazyLock<Mutex<FileWriterQuarantineRegistry>> =
    LazyLock::new(|| Mutex::new(FileWriterQuarantineRegistry::default()));

fn file_writer_quarantine_registry() -> &'static Mutex<FileWriterQuarantineRegistry> {
    &FILE_WRITER_QUARANTINE_REGISTRY
}

fn quarantine_unresolved(
    unresolved: &mut UnresolvedFileWriterSet,
) -> Option<FileWriterQuarantineId> {
    quarantine_unresolved_with_id(unresolved, None)
}

fn quarantine_unresolved_with_id(
    unresolved: &mut UnresolvedFileWriterSet,
    id: Option<FileWriterQuarantineId>,
) -> Option<FileWriterQuarantineId> {
    let quarantined = unresolved.take_quarantined(id)?;
    let id = quarantined.id;
    let mut registry = file_writer_quarantine_registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    debug_assert!(registry.entries.iter().all(|entry| entry.id != id));
    registry.entries.push(quarantined);
    Some(id)
}

struct ActiveProcessFileClaim {
    acquisition_id: u64,
    identity: IdentityObservationSet,
    accepted: bool,
}

struct ProcessFileClaimRegistry {
    next_acquisition_id: u64,
    active: Vec<ActiveProcessFileClaim>,
}

static PROCESS_FILE_CLAIM_REGISTRY: LazyLock<Mutex<ProcessFileClaimRegistry>> =
    LazyLock::new(|| {
        Mutex::new(ProcessFileClaimRegistry {
            next_acquisition_id: 1,
            active: Vec::new(),
        })
    });

fn process_file_claim_registry() -> &'static Mutex<ProcessFileClaimRegistry> {
    &PROCESS_FILE_CLAIM_REGISTRY
}

fn mark_file_writer_set_accepted(
    claims: &[FileWriterClaimBinding],
) -> Result<(), FileWriterSetAcceptanceFailure> {
    let mut registry = process_file_claim_registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let is_active = |claim: &FileWriterClaimBinding| {
        registry
            .active
            .iter()
            .any(|active| active.acquisition_id == claim.token.acquisition_id.0 && !active.accepted)
    };
    if !claims.iter().all(is_active) {
        return Err(FileWriterSetAcceptanceFailure::AlreadyAccepted);
    }
    for claim in claims {
        registry
            .active
            .iter_mut()
            .find(|active| active.acquisition_id == claim.token.acquisition_id.0)
            .expect("validated live claim remains registered")
            .accepted = true;
    }
    Ok(())
}

struct ProcessFileClaim {
    acquisition_id: FileWriterAcquisitionId,
}

impl ProcessFileClaim {
    fn acquire(identity: &IdentityObservationSet, path: &Path) -> Result<Self, FileStoreError> {
        let mut registry = process_file_claim_registry()
            .lock()
            .map_err(|_| io::Error::other("file claim registry is poisoned"))?;
        if registry
            .active
            .iter()
            .any(|active| active.identity == *identity)
        {
            return Err(FileLeaseError::AlreadyHeld(path.to_path_buf()).into());
        }
        let acquisition_id = registry.next_acquisition_id;
        registry.next_acquisition_id = acquisition_id
            .checked_add(1)
            .ok_or_else(|| io::Error::other("file acquisition identity exhausted"))?;
        registry.active.push(ActiveProcessFileClaim {
            acquisition_id,
            identity: identity.clone(),
            accepted: false,
        });
        Ok(Self {
            acquisition_id: FileWriterAcquisitionId(acquisition_id),
        })
    }
}

impl Drop for ProcessFileClaim {
    fn drop(&mut self) {
        if let Ok(mut registry) = process_file_claim_registry().lock() {
            registry
                .active
                .retain(|active| active.acquisition_id != self.acquisition_id.0);
        }
    }
}

struct HeldFileWriterClaim {
    binding: FileWriterClaimBinding,
    ownership: HeldFileOwnership,
}

enum HeldFileWriterClaimRelease {
    Released {
        observation: FileWriterClaimReleaseObservation,
        process_claim: ProcessFileClaim,
    },
    Unresolved {
        claim: HeldFileWriterClaim,
        observation: FileWriterClaimReleaseObservation,
    },
}

impl HeldFileWriterClaim {
    fn release(mut self) -> HeldFileWriterClaimRelease {
        if self.ownership.stage == FileOwnershipReleaseStage::Held {
            if let Err(error) = self.ownership.unlock_payload() {
                let observation = FileWriterClaimReleaseObservation {
                    binding: self.binding.clone(),
                    disposition: FileWriterClaimDisposition::Held,
                    cause: Some(FileReleaseCause::unlock_failed(
                        FileOwnershipResource::BackingPayload,
                        &error,
                    )),
                };
                return HeldFileWriterClaimRelease::Unresolved {
                    claim: self,
                    observation,
                };
            }
            self.ownership.stage = FileOwnershipReleaseStage::PayloadReleased;
        }
        if let Err(error) = self.ownership.release_marker() {
            let observation = FileWriterClaimReleaseObservation {
                binding: self.binding.clone(),
                disposition: FileWriterClaimDisposition::Uncertain,
                cause: Some(FileReleaseCause::marker_release_failed(&error)),
            };
            return HeldFileWriterClaimRelease::Unresolved {
                claim: self,
                observation,
            };
        }
        HeldFileWriterClaimRelease::Released {
            observation: FileWriterClaimReleaseObservation {
                binding: self.binding,
                disposition: FileWriterClaimDisposition::Released,
                cause: None,
            },
            process_claim: self
                .ownership
                .process_claim
                .take()
                .expect("held ownership retains its process claim"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileOwnershipReleaseStage {
    Held,
    PayloadReleased,
}

struct HeldFileOwnership {
    payload: Option<File>,
    marker: FileLease,
    stage: FileOwnershipReleaseStage,
    process_claim: Option<ProcessFileClaim>,
    #[cfg(test)]
    payload_release_faults: u8,
    #[cfg(test)]
    marker_release_faults: u8,
}

impl HeldFileOwnership {
    fn new(payload: File, marker: FileLease, process_claim: ProcessFileClaim) -> Self {
        Self {
            payload: Some(payload),
            marker,
            stage: FileOwnershipReleaseStage::Held,
            process_claim: Some(process_claim),
            #[cfg(test)]
            payload_release_faults: 0,
            #[cfg(test)]
            marker_release_faults: 0,
        }
    }

    fn unlock_payload(&mut self) -> io::Result<()> {
        #[cfg(test)]
        if self.payload_release_faults > 0 {
            self.payload_release_faults -= 1;
            return Err(io::Error::other("injected payload unlock failure"));
        }
        let payload = self
            .payload
            .as_ref()
            .expect("held ownership retains its payload descriptor");
        File::unlock(payload)?;
        drop(self.payload.take());
        Ok(())
    }

    fn release_marker(&mut self) -> Result<(), FileLeaseError> {
        #[cfg(test)]
        if self.marker_release_faults > 0 {
            self.marker_release_faults -= 1;
            return Err(FileLeaseError::Io(io::Error::other(
                "injected marker unlock failure",
            )));
        }
        self.marker.release()
    }
}

impl Drop for HeldFileOwnership {
    fn drop(&mut self) {
        if self.stage == FileOwnershipReleaseStage::Held
            && let Some(payload) = self.payload.as_ref()
        {
            let _ = File::unlock(payload);
        }
    }
}

enum FileStoreHandle {
    ReadOnly(File),
    Writable(HeldFileOwnership),
}

impl FileStoreHandle {
    fn file(&self) -> &File {
        match self {
            Self::ReadOnly(file) => file,
            Self::Writable(ownership) => ownership
                .payload
                .as_ref()
                .expect("writable stores retain their payload descriptor"),
        }
    }

    const fn is_writable(&self) -> bool {
        matches!(self, Self::Writable(_))
    }

    fn lease(&self) -> Option<&FileLease> {
        match self {
            Self::ReadOnly(_) => None,
            Self::Writable(ownership) => Some(&ownership.marker),
        }
    }

    fn acquisition_id(&self) -> Option<&FileWriterAcquisitionId> {
        match self {
            Self::ReadOnly(_) => None,
            Self::Writable(ownership) => ownership
                .process_claim
                .as_ref()
                .map(|claim| &claim.acquisition_id),
        }
    }
}

pub struct FileStore {
    handle: FileStoreHandle,
    config: FileStoreConfig,
    capabilities: StoreCapabilities,
    identity: IdentityObservationSet,
    incarnation: StoreIncarnationId,
    next_write_watermark: u64,
    highest_accepted_watermark: Option<StoreWriteWatermark>,
}

impl fmt::Debug for FileStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileStore")
            .field("path", &self.config.path)
            .field("protected_length", &self.config.protected_length)
            .field("writable", &self.handle.is_writable())
            .finish()
    }
}

impl FileStore {
    pub fn open(config: FileStoreConfig) -> Result<Self, FileStoreError> {
        validate_config(&config)?;
        let mut lease = if config.writable {
            Some(match &config.lock_path {
                Some(path) => FileLease::acquire_at(path.clone())?,
                None => FileLease::acquire(&config.path)?,
            })
        } else {
            None
        };
        let mut options = OpenOptions::new();
        options.read(true).write(config.writable);
        if config.create {
            options.create(true);
        } else if config.create_new {
            options.create_new(true);
        }
        let file = match options.open(&config.path) {
            Ok(file) => file,
            Err(error) => {
                drop(lease.take());
                return Err(FileStoreError::Io(error));
            }
        };
        if config.writable {
            match file.try_lock() {
                Ok(()) => {}
                Err(std::fs::TryLockError::WouldBlock) => {
                    drop(lease.take());
                    return Err(FileLeaseError::AlreadyHeld(config.path.clone()).into());
                }
                Err(error) => {
                    drop(lease.take());
                    return Err(FileStoreError::Io(error.into()));
                }
            }
        }
        let mut created_path = if config.create_new {
            match CreatedPathReservation::new(config.path.clone(), &file) {
                Ok(reservation) => Some(reservation),
                Err(error) => return Err(FileStoreError::Io(error)),
            }
        } else {
            None
        };
        let mut length = file.metadata()?.len();
        if config.protected_length != 0 && (config.create_new || (config.create && length == 0)) {
            file.set_len(config.protected_length)?;
            length = config.protected_length;
        }
        if length != config.protected_length {
            return Err(FileStoreError::InvalidGeometry(format!(
                "file length {length} differs from protected length {}",
                config.protected_length
            )));
        }
        let report = probe_file_capabilities(
            &config.path,
            config.protected_length,
            config.logical_block_size,
            config.maximum_transfer,
            evidence_id(&config.path, &config),
            if config.sparse {
                SparseBehavior::Requested
            } else {
                SparseBehavior::Regular
            },
        )?;
        let process_claim = if lease.is_some() {
            Some(ProcessFileClaim::acquire(&report.identity, &config.path)?)
        } else {
            None
        };
        if let Some(reservation) = created_path.as_mut() {
            reservation.commit();
        }
        let incarnation = StoreIncarnationId(
            lease
                .as_ref()
                .map_or(report.capabilities.evidence_id.0, FileLease::incarnation),
        );
        let handle = match (lease, process_claim) {
            (Some(marker), Some(process_claim)) => {
                FileStoreHandle::Writable(HeldFileOwnership::new(file, marker, process_claim))
            }
            (None, None) => FileStoreHandle::ReadOnly(file),
            _ => unreachable!("writable lease and process claim are acquired together"),
        };
        Ok(Self {
            handle,
            config,
            capabilities: report.capabilities,
            identity: report.identity,
            incarnation,
            next_write_watermark: 1,
            highest_accepted_watermark: None,
        })
    }

    pub fn path(&self) -> &Path {
        &self.config.path
    }
    pub const fn incarnation(&self) -> StoreIncarnationId {
        self.incarnation
    }

    pub const fn highest_accepted_watermark(&self) -> Option<StoreWriteWatermark> {
        self.highest_accepted_watermark
    }
    pub fn capabilities_report(&self) -> FileCapabilityReport {
        FileCapabilityReport {
            capabilities: self.capabilities.clone(),
            identity: self.identity.clone(),
            sparse_behavior: if self.config.sparse {
                SparseBehavior::Requested
            } else {
                SparseBehavior::Regular
            },
            path: self.config.path.clone(),
        }
    }
    pub fn lease(&self) -> Option<&FileLease> {
        self.handle.lease()
    }

    pub fn writer_claim_binding(&self) -> Option<FileWriterClaimBinding> {
        let lease = self.handle.lease()?;
        Some(FileWriterClaimBinding {
            token: FileWriterClaimToken {
                store_id: self.store_id(),
                incarnation: self.incarnation,
                topology_epoch: self.config.topology_epoch,
                acquisition_id: self
                    .handle
                    .acquisition_id()
                    .expect("writable stores have process claims")
                    .clone(),
            },
            backing_identity: self.identity.clone(),
            alias_binding: FileWriterAliasBinding {
                backing_path: self.config.path.clone(),
                lease_path: lease.path().to_path_buf(),
            },
            protected_length: self.config.protected_length,
            logical_block_size: self.config.logical_block_size,
        })
    }

    fn into_held_writer_claim(self) -> HeldFileWriterClaim {
        let binding = self
            .writer_claim_binding()
            .expect("accepted writer set members are writable");
        let FileStoreHandle::Writable(ownership) = self.handle else {
            unreachable!("accepted writer set members are writable");
        };
        HeldFileWriterClaim { binding, ownership }
    }

    #[cfg(test)]
    fn fail_next_release(&mut self, resource: FileOwnershipResource) {
        let FileStoreHandle::Writable(ownership) = &mut self.handle else {
            panic!("release faults require a writable store");
        };
        match resource {
            FileOwnershipResource::BackingPayload => {
                ownership.payload_release_faults += 1;
            }
            FileOwnershipResource::LeaseMarker => {
                ownership.marker_release_faults += 1;
            }
        }
    }
    pub fn identity_is_current(&self) -> Result<IdentityComparison, FileStoreError> {
        compare_identity(&self.identity, &self.config.path).map_err(FileStoreError::Identity)
    }

    pub fn read_bytes(&mut self, range: ByteRange) -> Result<Vec<u8>, FileStoreError> {
        let progress = self.read_progress(range).map_err(FileStoreError::Store)?;
        if progress.completed != range.length {
            return Err(FileStoreError::Store(StoreError::BackendFailure {
                code: 5,
            }));
        }
        Ok(progress.bytes)
    }

    pub fn write_bytes(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        bytes: &[u8],
        intent: WriteIntent,
    ) -> StoreCompletion {
        if bytes.len() as u64 != range.length {
            return failed(
                operation_id,
                range,
                StoreError::BackendFailure { code: 22 },
                0,
            );
        }
        if let Err(error) = self.validate(range, StoreOperation::Write, Some(intent), true) {
            return failed(operation_id, range, error, 0);
        }
        let watermark = match self.accept_write() {
            Ok(watermark) => watermark,
            Err(error) => return failed(operation_id, range, error, 0),
        };
        if intent_uses_preflush(intent)
            && let Err(error) = self.sync_file()
        {
            return failed(
                operation_id,
                range,
                StoreError::BackendFailure {
                    code: io_code(&error),
                },
                0,
            )
            .with_write_watermark(watermark);
        }
        let progress = write_all_at(self.handle.file(), range.offset, bytes);
        let completed = progress.completed;
        if completed != range.length {
            let disposition = progress
                .error
                .map_or(CompletionDisposition::Short, |error| {
                    CompletionDisposition::Failed(StoreError::BackendFailure {
                        code: io_code(&error),
                    })
                });
            return completion(
                operation_id,
                range,
                completed,
                disposition,
                PersistenceEvidence::VolatileOrUnknown,
            )
            .with_write_watermark(watermark);
        }
        match self.config.sync_mode {
            FileSyncMode::CallerFlush => completion(
                operation_id,
                range,
                completed,
                CompletionDisposition::Success,
                PersistenceEvidence::VolatileOrUnknown,
            )
            .with_write_watermark(watermark),
            FileSyncMode::SyncData | FileSyncMode::SyncAll => match self.sync_file() {
                Ok(()) => completion(
                    operation_id,
                    range,
                    completed,
                    CompletionDisposition::Success,
                    self.fence_evidence_through(watermark),
                )
                .with_write_watermark(watermark),
                Err(_error) => completion(
                    operation_id,
                    range,
                    completed,
                    CompletionDisposition::Uncertain,
                    PersistenceEvidence::VolatileOrUnknown,
                )
                .with_write_watermark(watermark),
            },
        }
    }

    pub fn write_zeroes_bytes(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        intent: WriteIntent,
    ) -> StoreCompletion {
        if let Err(error) = self.validate(range, StoreOperation::WriteZeroes, Some(intent), false) {
            return failed(operation_id, range, error, 0);
        }
        let watermark = match self.accept_write() {
            Ok(watermark) => watermark,
            Err(error) => return failed(operation_id, range, error, 0),
        };
        if intent_uses_preflush(intent)
            && let Err(error) = self.sync_file()
        {
            return failed(
                operation_id,
                range,
                StoreError::BackendFailure {
                    code: io_code(&error),
                },
                0,
            )
            .with_write_watermark(watermark);
        }
        let chunk =
            usize::try_from(self.config.maximum_transfer.min(1024 * 1024)).unwrap_or(1024 * 1024);
        let zeros = vec![0_u8; chunk];
        let mut offset = range.offset;
        let mut remaining = range.length;
        while remaining != 0 {
            let length = usize::try_from(remaining.min(zeros.len() as u64)).unwrap_or(zeros.len());
            let progress = write_all_at(self.handle.file(), offset, &zeros[..length]);
            if progress.completed != length as u64 {
                let completed = offset.saturating_sub(range.offset) + progress.completed;
                let disposition = progress
                    .error
                    .map_or(CompletionDisposition::Short, |error| {
                        CompletionDisposition::Failed(StoreError::BackendFailure {
                            code: io_code(&error),
                        })
                    });
                return completion(
                    operation_id,
                    range,
                    completed,
                    disposition,
                    PersistenceEvidence::VolatileOrUnknown,
                )
                .with_write_watermark(watermark);
            }
            offset += length as u64;
            remaining -= length as u64;
        }
        if matches!(
            self.config.sync_mode,
            FileSyncMode::SyncData | FileSyncMode::SyncAll
        ) {
            if self.sync_file().is_err() {
                return completion(
                    operation_id,
                    range,
                    range.length,
                    CompletionDisposition::Uncertain,
                    PersistenceEvidence::VolatileOrUnknown,
                )
                .with_write_watermark(watermark);
            }
            return completion(
                operation_id,
                range,
                range.length,
                CompletionDisposition::Success,
                self.fence_evidence_through(watermark),
            )
            .with_write_watermark(watermark);
        }
        completion(
            operation_id,
            range,
            range.length,
            CompletionDisposition::Success,
            PersistenceEvidence::VolatileOrUnknown,
        )
        .with_write_watermark(watermark)
    }

    pub fn flush_file(
        &mut self,
        operation_id: ChildOperationId,
        through: StoreWriteWatermark,
    ) -> StoreCompletion {
        if through.0
            > self
                .highest_accepted_watermark
                .map_or(0, |watermark| watermark.0)
        {
            return failed(
                operation_id,
                ByteRange::empty(),
                StoreError::BackendFailure { code: 22 },
                0,
            );
        }
        match self.sync_file() {
            Ok(()) => completion(
                operation_id,
                ByteRange::empty(),
                0,
                CompletionDisposition::Success,
                self.fence_evidence_through(through),
            ),
            Err(error) => completion(
                operation_id,
                ByteRange::empty(),
                0,
                CompletionDisposition::Failed(StoreError::BackendFailure {
                    code: io_code(&error),
                }),
                PersistenceEvidence::VolatileOrUnknown,
            ),
        }
    }

    pub fn read_progress(&mut self, range: ByteRange) -> Result<ReadProgress, StoreError> {
        let length = usize::try_from(range.length).map_err(|_| StoreError::TransferTooLarge {
            length: range.length,
            maximum: usize::MAX as u64,
        })?;
        let mut bytes = vec![0_u8; length];
        let completed = self.read_into(range, &mut bytes)?;
        bytes.truncate(completed as usize);
        Ok(ReadProgress { bytes, completed })
    }

    fn read_into(&mut self, range: ByteRange, bytes: &mut [u8]) -> Result<u64, StoreError> {
        if bytes.len() as u64 != range.length {
            return Err(StoreError::BackendFailure { code: 22 });
        }
        self.validate(range, StoreOperation::Read, None, true)?;
        let mut completed = 0_u64;
        while completed < range.length {
            match read_at(
                self.handle.file(),
                &mut bytes[completed as usize..],
                range.offset + completed,
            ) {
                Ok(0) => break,
                Ok(read) => completed += read as u64,
                Err(error) => {
                    return Err(StoreError::BackendFailure {
                        code: io_code(&error),
                    });
                }
            }
        }
        Ok(completed)
    }

    fn validate(
        &self,
        range: ByteRange,
        operation: StoreOperation,
        intent: Option<WriteIntent>,
        buffer: bool,
    ) -> Result<(), StoreError> {
        let kind = match operation {
            StoreOperation::Read => StoreRequestKind::Read { range },
            StoreOperation::Write => StoreRequestKind::Write {
                range,
                intent: intent.unwrap_or(WriteIntent::Ordinary),
            },
            StoreOperation::WriteZeroes => StoreRequestKind::WriteZeroes {
                range,
                intent: intent.unwrap_or(WriteIntent::Ordinary),
            },
            StoreOperation::Discard => StoreRequestKind::Discard { range },
            StoreOperation::Flush => StoreRequestKind::Flush {
                through: StoreWriteWatermark(0),
            },
        };
        StoreRequest {
            operation_id: OperationId(0),
            store_id: self.store_id(),
            topology_epoch: self.config.topology_epoch,
            kind,
            buffer: buffer.then_some(BufferToken::new(0, 0)),
        }
        .validate(&self.capabilities)
    }

    fn sync_file(&self) -> io::Result<()> {
        match self.config.sync_mode {
            FileSyncMode::SyncData => self.handle.file().sync_data(),
            FileSyncMode::CallerFlush | FileSyncMode::SyncAll => self.handle.file().sync_all(),
        }
    }
    pub fn store_id(&self) -> StoreId {
        if self.config.store_id == StoreId(0) {
            StoreId(evidence_id(&self.config.path, &self.config).0)
        } else {
            self.config.store_id
        }
    }
    pub const fn topology_epoch(&self) -> TopologyEpoch {
        self.config.topology_epoch
    }
    fn accept_write(&mut self) -> Result<StoreWriteWatermark, StoreError> {
        let watermark = StoreWriteWatermark(self.next_write_watermark);
        self.next_write_watermark = self
            .next_write_watermark
            .checked_add(1)
            .ok_or(StoreError::BackendFailure { code: 75 })?;
        self.highest_accepted_watermark = Some(watermark);
        Ok(watermark)
    }
    fn fence_evidence_through(&self, through: StoreWriteWatermark) -> PersistenceEvidence {
        PersistenceEvidence::DurableByFence {
            fence: dwv_store::StoreFenceRef {
                fence_id: dwv_store::FenceId(through.0),
                store_id: self.store_id(),
                store_incarnation: self.incarnation,
                topology_epoch: self.config.topology_epoch,
                through,
                capability_evidence_id: self.capabilities.evidence_id,
            },
        }
    }
}

impl RandomAccessStore for FileStore {
    fn store_id(&self) -> StoreId {
        self.store_id()
    }

    fn topology_epoch(&self) -> TopologyEpoch {
        self.topology_epoch()
    }

    fn incarnation(&self) -> StoreIncarnationId {
        self.incarnation()
    }

    fn identity_observations(&self) -> IdentityObservationSet {
        self.identity.clone()
    }

    fn current_identity_observations(&self) -> Result<IdentityObservationSet, StoreError> {
        observe_file_identity(&self.config.path).map_err(|error| match error {
            FileIdentityError::Io(error) => StoreError::BackendFailure {
                code: io_code(&error),
            },
            FileIdentityError::NotARegularFile(_) => StoreError::BackendFailure { code: 22 },
        })
    }

    fn capabilities(&self) -> StoreCapabilities {
        self.capabilities.clone()
    }

    fn length(&self) -> u64 {
        self.config.protected_length
    }

    fn highest_accepted_watermark(&self) -> Option<StoreWriteWatermark> {
        self.highest_accepted_watermark
    }

    fn read_at(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        destination: &mut [u8],
    ) -> StoreCompletion {
        match self.read_into(range, destination) {
            Ok(completed) if completed == range.length => completion(
                operation_id,
                range,
                completed,
                CompletionDisposition::Success,
                PersistenceEvidence::VolatileOrUnknown,
            ),
            Ok(completed) => completion(
                operation_id,
                range,
                completed,
                CompletionDisposition::Short,
                PersistenceEvidence::VolatileOrUnknown,
            ),
            Err(error) => failed(operation_id, range, error, 0),
        }
    }

    fn write_at(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        source: &[u8],
        intent: WriteIntent,
    ) -> StoreCompletion {
        self.write_bytes(operation_id, range, source, intent)
    }

    fn flush(
        &mut self,
        operation_id: ChildOperationId,
        through: StoreWriteWatermark,
    ) -> StoreCompletion {
        self.flush_file(operation_id, through)
    }
    fn write_zeroes(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        intent: WriteIntent,
    ) -> StoreCompletion {
        self.write_zeroes_bytes(operation_id, range, intent)
    }
    fn discard(&mut self, operation_id: ChildOperationId, range: ByteRange) -> StoreCompletion {
        failed(
            operation_id,
            range,
            StoreError::UnsupportedOperation(StoreOperation::Discard),
            0,
        )
    }
}

fn validate_config(config: &FileStoreConfig) -> Result<(), FileStoreError> {
    if config.logical_block_size == 0
        || !config
            .protected_length
            .is_multiple_of(u64::from(config.logical_block_size))
    {
        return Err(FileStoreError::InvalidGeometry(
            "protected length must be block aligned".to_owned(),
        ));
    }
    if config.maximum_transfer == 0
        || !config
            .maximum_transfer
            .is_multiple_of(u64::from(config.logical_block_size))
    {
        return Err(FileStoreError::InvalidGeometry(
            "maximum transfer must be a non-zero block multiple".to_owned(),
        ));
    }
    if config.create && !config.writable {
        return Err(FileStoreError::InvalidGeometry(
            "creation requires writable access".to_owned(),
        ));
    }
    if config.create_new && !config.writable {
        return Err(FileStoreError::InvalidGeometry(
            "exclusive creation requires writable access".to_owned(),
        ));
    }
    if config.create && config.create_new {
        return Err(FileStoreError::InvalidGeometry(
            "ordinary and exclusive creation are mutually exclusive".to_owned(),
        ));
    }
    Ok(())
}

struct CreatedPathReservation {
    path: PathBuf,
    identity: IdentityObservationSet,
    committed: bool,
}

impl CreatedPathReservation {
    fn new(path: PathBuf, file: &File) -> io::Result<Self> {
        Ok(Self {
            path,
            identity: identity_from_metadata(&file.metadata()?),
            committed: false,
        })
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for CreatedPathReservation {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if matches!(
            compare_identity(&self.identity, &self.path),
            Ok(IdentityComparison::Unchanged)
        ) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn evidence_id(path: &Path, config: &FileStoreConfig) -> CapabilityEvidenceId {
    let metadata = std::fs::metadata(path).ok();
    let identity = metadata.map(|metadata| identity_from_metadata(&metadata));
    let mut value = config.protected_length
        ^ (u64::from(config.logical_block_size) << 32)
        ^ config.maximum_transfer;
    if let Some(identity) = identity {
        for observation in identity.observations {
            for byte in observation.fingerprint {
                value = value.rotate_left(5) ^ u64::from(byte);
            }
        }
    }
    CapabilityEvidenceId(value)
}

fn intent_uses_preflush(intent: WriteIntent) -> bool {
    matches!(intent, WriteIntent::Preflush | WriteIntent::FuaAndPreflush)
}

fn write_all_at(file: &File, mut offset: u64, bytes: &[u8]) -> Progress {
    let mut completed = 0_usize;
    while completed < bytes.len() {
        match write_at(file, &bytes[completed..], offset) {
            Ok(0) => {
                return Progress {
                    completed: completed as u64,
                    error: None,
                };
            }
            Ok(written) => {
                completed += written;
                offset += written as u64;
            }
            Err(error) => {
                return Progress {
                    completed: completed as u64,
                    error: Some(error),
                };
            }
        }
    }
    Progress {
        completed: completed as u64,
        error: None,
    }
}

struct Progress {
    completed: u64,
    error: Option<io::Error>,
}

#[cfg(unix)]
fn read_at(file: &File, bytes: &mut [u8], offset: u64) -> io::Result<usize> {
    use std::os::unix::fs::FileExt;
    file.read_at(bytes, offset)
}
#[cfg(unix)]
fn write_at(file: &File, bytes: &[u8], offset: u64) -> io::Result<usize> {
    use std::os::unix::fs::FileExt;
    file.write_at(bytes, offset)
}

fn io_code(error: &io::Error) -> u16 {
    error
        .raw_os_error()
        .and_then(|code| u16::try_from(code).ok())
        .unwrap_or(5)
}

fn completion(
    operation_id: ChildOperationId,
    requested: ByteRange,
    completed_length: u64,
    disposition: CompletionDisposition,
    persistence: PersistenceEvidence,
) -> StoreCompletion {
    let completed = if completed_length == 0 {
        CompletedRangeSet::empty()
    } else {
        CompletedRangeSet::new(vec![ByteRange {
            offset: requested.offset,
            length: completed_length,
        }])
        .expect("completion prefix is bounded by request")
    };
    StoreCompletion::new(operation_id, requested, completed, disposition, persistence)
        .expect("file completion is valid")
}

fn failed(
    operation_id: ChildOperationId,
    requested: ByteRange,
    error: StoreError,
    completed_length: u64,
) -> StoreCompletion {
    completion(
        operation_id,
        requested,
        completed_length,
        CompletionDisposition::Failed(error),
        PersistenceEvidence::VolatileOrUnknown,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lease::default_lease_path;
    use dwv_store::{OperationSlotToken, RandomAccessStore};
    use std::fs;
    use std::process::{Command, Stdio};

    fn operation(index: u32) -> ChildOperationId {
        ChildOperationId {
            slot: OperationSlotToken::new(1, 1),
            index,
        }
    }
    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("dwv-file-{name}-{}", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => panic!("failed to remove {}: {error}", path.display()),
        }
    }

    fn registered_claim_count(acquisition_ids: &[FileWriterAcquisitionId]) -> usize {
        let registry = process_file_claim_registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        registry
            .active
            .iter()
            .filter(|active| {
                acquisition_ids
                    .iter()
                    .any(|id| id.0 == active.acquisition_id)
            })
            .count()
    }

    fn assert_lock_available(path: &Path) {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .unwrap();
        file.try_lock().unwrap();
        File::unlock(&file).unwrap();
    }

    fn release_writer_set(
        expected_store_ids: impl IntoIterator<Item = StoreId>,
        stores: Vec<FileStore>,
    ) -> FileWriterSetReleaseResult {
        AcceptedFileWriterSet::accept(expected_store_ids, stores)
            .unwrap()
            .release()
    }

    #[test]
    fn hard_link_alias_cannot_bypass_member_claim() {
        let path = temp("member-claim");
        let alias = temp("member-claim-alias");
        remove_if_present(&path);
        remove_if_present(&alias);
        remove_if_present(&default_lease_path(&path));
        remove_if_present(&default_lease_path(&alias));
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        fs::hard_link(&path, &alias).unwrap();
        let first = FileStore::open(FileStoreConfig::new(&path, 4096, 512)).unwrap();
        assert!(matches!(
            FileStore::open(FileStoreConfig::new(&alias, 4096, 512)),
            Err(FileStoreError::Lease(FileLeaseError::AlreadyHeld(_)))
        ));
        drop(first);
        FileStore::open(FileStoreConfig::new(&alias, 4096, 512)).unwrap();
        remove_if_present(&default_lease_path(&path));
        remove_if_present(&default_lease_path(&alias));
        remove_if_present(&alias);
        remove_if_present(&path);
    }

    #[test]
    fn create_new_atomically_reserves_and_initializes_sparse_length() {
        let path = temp("create-new");
        let lock_path = temp("create-new-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);

        let store = FileStore::open(
            FileStoreConfig::new(&path, 8192, 512)
                .create_new(true)
                .sparse(true)
                .lock_path(&lock_path),
        )
        .unwrap();

        assert_eq!(store.length(), 8192);
        assert_eq!(fs::metadata(&path).unwrap().len(), 8192);
        drop(store);
        assert!(FileLease::acquire_at(&lock_path).is_ok());
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn create_new_never_opens_or_overwrites_an_existing_nonempty_path() {
        let path = temp("create-new-existing");
        let lock_path = temp("create-new-existing-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        let original = vec![0xa5_u8; 4096];
        fs::write(&path, &original).unwrap();

        let error = FileStore::open(
            FileStoreConfig::new(&path, 4096, 512)
                .create_new(true)
                .sparse(true)
                .lock_path(&lock_path),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            FileStoreError::Io(ref error) if error.kind() == io::ErrorKind::AlreadyExists
        ));
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(FileLease::acquire_at(&lock_path).is_ok());
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn failed_create_new_releases_lease_and_allows_a_clean_retry() {
        let path = temp("create-new-retry");
        let lock_path = temp("create-new-retry-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        fs::write(&path, vec![0x3c_u8; 4096]).unwrap();
        let config = FileStoreConfig::new(&path, 4096, 512)
            .create_new(true)
            .lock_path(&lock_path);

        assert!(FileStore::open(config.clone()).is_err());
        assert!(FileLease::acquire_at(&lock_path).is_ok());
        assert_eq!(fs::read(&path).unwrap(), vec![0x3c_u8; 4096]);

        remove_if_present(&path);
        let store = FileStore::open(config).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), 4096);
        drop(store);
        assert!(FileLease::acquire_at(&lock_path).is_ok());
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn uncommitted_create_new_reservation_removes_created_path_and_lease() {
        let path = temp("create-new-rollback");
        let lock_path = temp("create-new-rollback-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);

        let lease = FileLease::acquire_at(&lock_path).unwrap();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let reservation = CreatedPathReservation::new(path.clone(), &file).unwrap();
        file.set_len(4096).unwrap();

        drop(file);
        drop(reservation);
        drop(lease);

        assert!(!path.exists());
        assert!(FileLease::acquire_at(&lock_path).is_ok());
        remove_if_present(&lock_path);
    }

    #[test]
    fn ordinary_and_exclusive_creation_are_rejected_together() {
        let path = temp("create-modes");
        let lock_path = temp("create-modes-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);

        let error = FileStore::open(
            FileStoreConfig::new(&path, 4096, 512)
                .create(true)
                .create_new(true)
                .lock_path(&lock_path),
        )
        .unwrap_err();

        assert!(matches!(error, FileStoreError::InvalidGeometry(_)));
        assert!(!path.exists());
        assert!(!lock_path.exists());
    }

    #[test]
    fn sparse_holes_fixed_geometry_and_direct_data_equality() {
        let path = temp("sparse");
        let mut store = FileStore::open(
            FileStoreConfig::new(&path, 8192, 512)
                .create(true)
                .sparse(true),
        )
        .unwrap();
        assert_eq!(store.length(), 8192);
        assert_eq!(
            store
                .read_bytes(ByteRange::new(4096, 512).unwrap())
                .unwrap(),
            vec![0; 512]
        );
        let bytes = vec![0x5a; 512];
        let result = store.write_bytes(
            operation(1),
            ByteRange::new(1024, 512).unwrap(),
            &bytes,
            WriteIntent::Ordinary,
        );
        assert_eq!(result.disposition, CompletionDisposition::Success);
        assert_eq!(
            store
                .read_bytes(ByteRange::new(1024, 512).unwrap())
                .unwrap(),
            bytes
        );
        drop(store);
        assert_eq!(fs::metadata(&path).unwrap().len(), 8192);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn out_of_range_does_not_mutate_and_discard_is_conservative() {
        let path = temp("range");
        fs::write(&path, vec![1_u8; 4096]).unwrap();
        let mut store = FileStore::open(FileStoreConfig::new(&path, 4096, 512)).unwrap();
        let result = store.write_bytes(
            operation(1),
            ByteRange::new(4096, 512).unwrap(),
            &[2; 512],
            WriteIntent::Ordinary,
        );
        assert!(matches!(
            result.disposition,
            CompletionDisposition::Failed(StoreError::RangeOutsideStore { .. })
        ));
        assert_eq!(fs::read(&path).unwrap(), vec![1_u8; 4096]);
        let discard = store.discard(operation(2), ByteRange::new(0, 512).unwrap());
        assert_eq!(
            discard.disposition,
            CompletionDisposition::Failed(StoreError::UnsupportedOperation(
                StoreOperation::Discard
            ))
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn write_watermarks_are_monotonic_and_bound_to_each_incarnation() {
        let path = temp("watermark-incarnation");
        remove_if_present(&path);
        remove_if_present(&default_lease_path(&path));
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let mut first = FileStore::open(
            FileStoreConfig::new(&path, 4096, 512).sync_mode(FileSyncMode::CallerFlush),
        )
        .unwrap();
        let first_incarnation = first.incarnation();
        let first_write = first.write_bytes(
            operation(1),
            ByteRange::new(0, 512).unwrap(),
            &[1; 512],
            WriteIntent::Ordinary,
        );
        let second_write = first.write_bytes(
            operation(2),
            ByteRange::new(512, 512).unwrap(),
            &[2; 512],
            WriteIntent::Ordinary,
        );
        assert_eq!(first_write.write_watermark, Some(StoreWriteWatermark(1)));
        assert_eq!(second_write.write_watermark, Some(StoreWriteWatermark(2)));
        drop(first);

        let mut reopened = FileStore::open(
            FileStoreConfig::new(&path, 4096, 512).sync_mode(FileSyncMode::CallerFlush),
        )
        .unwrap();
        assert!(reopened.incarnation().0 > first_incarnation.0);
        let reopened_write = reopened.write_bytes(
            operation(3),
            ByteRange::new(1024, 512).unwrap(),
            &[3; 512],
            WriteIntent::Ordinary,
        );
        assert_eq!(reopened_write.write_watermark, Some(StoreWriteWatermark(1)));
        let fence = reopened.flush_file(operation(4), StoreWriteWatermark(1));
        let PersistenceEvidence::DurableByFence { fence } = fence.persistence else {
            panic!("flush lacks durable fence");
        };
        assert_eq!(fence.store_incarnation, reopened.incarnation());
        drop(reopened);
        remove_if_present(&default_lease_path(&path));
        remove_if_present(&path);
    }

    #[test]
    fn trait_borrows_payloads_and_flushes_with_exact_completion_evidence() {
        let path = temp("trait");
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let mut store = FileStore::open(
            FileStoreConfig::new(&path, 4096, 512).sync_mode(FileSyncMode::SyncAll),
        )
        .unwrap();
        let source = vec![7_u8; 512];
        let write = store.write_at(
            operation(3),
            ByteRange::new(0, 512).unwrap(),
            &source,
            WriteIntent::Ordinary,
        );
        assert!(write.persistence.is_durable());
        let flush = store.flush(operation(4), write.write_watermark.unwrap());
        assert!(flush.persistence.is_durable());
        let mut destination = vec![0; 512];
        let read = store.read_at(
            operation(5),
            ByteRange::new(0, 512).unwrap(),
            &mut destination,
        );
        assert_eq!(read.disposition, CompletionDisposition::Success);
        assert_eq!(destination, source);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn replacing_a_path_invalidates_the_captured_identity() {
        let path = temp("identity");
        let replacement = temp("identity-replacement");
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let store = FileStore::open(FileStoreConfig::new(&path, 4096, 512)).unwrap();
        fs::rename(&path, &replacement).unwrap();
        fs::write(&path, vec![1_u8; 4096]).unwrap();
        assert_eq!(
            store.identity_is_current().unwrap(),
            IdentityComparison::Changed
        );
        drop(store);
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(replacement);
    }

    #[test]
    fn accepted_writer_set_release_accounts_for_payload_and_marker_locks() {
        let path = temp("writer-set-release");
        let lock_path = temp("writer-set-release-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(7))
            .lock_path(&lock_path);
        let store = FileStore::open(config.clone()).unwrap();
        let binding = store.writer_claim_binding().unwrap();
        assert_eq!(binding.token().store_id(), StoreId(7));
        assert_eq!(binding.alias_binding().backing_path(), path);
        assert_eq!(binding.alias_binding().lease_path(), lock_path);

        let release = release_writer_set([StoreId(7)], vec![store]);
        let released = release.expect("complete release did not release all ownership");
        let observation = released.observation();
        assert_eq!(
            observation.disposition(),
            FileBackedReleaseDisposition::ReleasedAll
        );
        assert_eq!(
            observation.members()[0].disposition(),
            FileWriterClaimDisposition::Released
        );
        assert!(observation.members()[0].cause().is_none());

        drop(FileStore::open(config).unwrap());
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn payload_release_failure_retains_complete_claim_for_retry() {
        let path = temp("payload-release-failure");
        let lock_path = temp("payload-release-failure-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(8))
            .lock_path(&lock_path);
        let mut store = FileStore::open(config.clone()).unwrap();
        store.fail_next_release(FileOwnershipResource::BackingPayload);

        let release = release_writer_set([StoreId(8)], vec![store]);
        let Err(unresolved) = release else {
            panic!("injected payload unlock failure reported released-all");
        };
        let member = &unresolved.observation().members()[0];
        assert_eq!(
            unresolved.observation().disposition(),
            FileBackedReleaseDisposition::Residual
        );
        assert_eq!(member.disposition(), FileWriterClaimDisposition::Held);
        assert_eq!(
            member.cause().unwrap().resource(),
            FileOwnershipResource::BackingPayload
        );
        assert_eq!(
            member.cause().unwrap().kind(),
            FileReleaseCauseKind::OperationFailed
        );
        assert!(matches!(
            FileStore::open(config.clone()),
            Err(FileStoreError::Lease(FileLeaseError::AlreadyHeld(_)))
        ));

        let released = unresolved
            .retry_release()
            .expect("retry did not release retained payload and marker ownership");
        let observation = released.observation();
        assert_eq!(
            observation.disposition(),
            FileBackedReleaseDisposition::ReleasedAll
        );
        drop(FileStore::open(config).unwrap());
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn marker_release_failure_keeps_marker_after_payload_unlock() {
        let path = temp("marker-release-failure");
        let lock_path = temp("marker-release-failure-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(9))
            .lock_path(&lock_path);
        let mut store = FileStore::open(config.clone()).unwrap();
        store.fail_next_release(FileOwnershipResource::LeaseMarker);

        let release = release_writer_set([StoreId(9)], vec![store]);
        let Err(unresolved) = release else {
            panic!("injected marker unlock failure reported released-all");
        };
        let member = &unresolved.observation().members()[0];
        assert_eq!(
            unresolved.observation().disposition(),
            FileBackedReleaseDisposition::Uncertain
        );
        assert_eq!(member.disposition(), FileWriterClaimDisposition::Uncertain);
        assert_eq!(
            member.cause().unwrap().resource(),
            FileOwnershipResource::LeaseMarker
        );

        let payload = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        payload.try_lock().unwrap();
        File::unlock(&payload).unwrap();
        assert!(matches!(
            FileLease::acquire_at(&lock_path),
            Err(FileLeaseError::AlreadyHeld(_))
        ));

        assert!(unresolved.retry_release().is_ok());
        drop(FileStore::open(config).unwrap());
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn incomplete_member_set_is_not_accepted_or_released() {
        let path = temp("writer-set-incomplete");
        let lock_path = temp("writer-set-incomplete-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(10))
            .lock_path(&lock_path);
        let stores = vec![FileStore::open(config.clone()).unwrap()];

        let error = AcceptedFileWriterSet::accept([StoreId(10), StoreId(11)], stores).unwrap_err();
        assert_eq!(
            error.failure(),
            FileWriterSetAcceptanceFailure::MemberSetMismatch
        );
        let stores = error.into_stores();
        assert!(matches!(
            FileStore::open(config.clone()),
            Err(FileStoreError::Lease(FileLeaseError::AlreadyHeld(_)))
        ));
        drop(stores);
        drop(FileStore::open(config).unwrap());
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn file_store_holder_process() {
        let Ok(path) = std::env::var("DWV_TEST_FILE_STORE_PATH") else {
            return;
        };
        let lock_path = std::env::var("DWV_TEST_FILE_STORE_LOCK").unwrap();
        let _store =
            FileStore::open(FileStoreConfig::new(path, 4096, 512).lock_path(lock_path)).unwrap();
        fs::write(
            std::env::var("DWV_TEST_FILE_STORE_READY").unwrap(),
            b"ready",
        )
        .unwrap();
        loop {
            std::thread::park();
        }
    }

    #[test]
    fn process_death_releases_payload_and_marker_locks() {
        let path = temp("process-death");
        let lock_path = temp("process-death-lock");
        let ready_path = temp("process-death-ready");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        remove_if_present(&ready_path);
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "file_store::tests::file_store_holder_process",
                "--exact",
                "--nocapture",
            ])
            .env("DWV_TEST_FILE_STORE_PATH", &path)
            .env("DWV_TEST_FILE_STORE_LOCK", &lock_path)
            .env("DWV_TEST_FILE_STORE_READY", &ready_path)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        for _ in 0..100 {
            if ready_path.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(ready_path.exists());
        assert!(matches!(
            FileStore::open(FileStoreConfig::new(&path, 4096, 512).lock_path(&lock_path)),
            Err(FileStoreError::Lease(FileLeaseError::AlreadyHeld(_)))
        ));

        child.kill().unwrap();
        child.wait().unwrap();
        drop(
            FileStore::open(FileStoreConfig::new(&path, 4096, 512).lock_path(&lock_path)).unwrap(),
        );
        remove_if_present(&ready_path);
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn writer_set_acceptance_rejects_every_invalid_shape() {
        let path = temp("acceptance-shapes");
        let lock_path = temp("acceptance-shapes-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(12))
            .lock_path(&lock_path);

        let stores = vec![FileStore::open(config.clone()).unwrap()];
        let error = AcceptedFileWriterSet::accept([], stores).unwrap_err();
        assert_eq!(
            error.failure(),
            FileWriterSetAcceptanceFailure::EmptyExpectedSet
        );
        drop(error.into_stores());

        let stores = vec![FileStore::open(config.clone()).unwrap()];
        let error = AcceptedFileWriterSet::accept([StoreId(12), StoreId(12)], stores).unwrap_err();
        assert_eq!(
            error.failure(),
            FileWriterSetAcceptanceFailure::DuplicateExpectedStore
        );
        drop(error.into_stores());

        let second_path = temp("acceptance-shapes-second");
        let second_lock_path = temp("acceptance-shapes-second-lock");
        remove_if_present(&second_path);
        remove_if_present(&second_lock_path);
        fs::write(&second_path, vec![0_u8; 4096]).unwrap();
        let second_config = FileStoreConfig::new(&second_path, 4096, 512)
            .store_id(StoreId(12))
            .lock_path(&second_lock_path);
        let stores = vec![
            FileStore::open(config.clone()).unwrap(),
            FileStore::open(second_config).unwrap(),
        ];
        let error = AcceptedFileWriterSet::accept([StoreId(12)], stores).unwrap_err();
        assert_eq!(
            error.failure(),
            FileWriterSetAcceptanceFailure::DuplicateStore
        );
        drop(error.into_stores());

        let stores = vec![FileStore::open(config.clone().writable(false)).unwrap()];
        let error = AcceptedFileWriterSet::accept([StoreId(12)], stores).unwrap_err();
        assert_eq!(
            error.failure(),
            FileWriterSetAcceptanceFailure::ReadOnlyMember
        );
        drop(error.into_stores());

        remove_if_present(&second_lock_path);
        remove_if_present(&second_path);
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn dropped_accepted_set_is_quarantined_until_owner_release() {
        let path = temp("accepted-drop");
        let lock_path = temp("accepted-drop-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(13))
            .lock_path(&lock_path);
        let store = FileStore::open(config.clone()).unwrap();
        let mut accepted = AcceptedFileWriterSet::accept([StoreId(13)], vec![store]).unwrap();
        let mut destination = [0_u8; 512];
        let read = accepted.store_mut(StoreId(13)).unwrap().read_at(
            operation(90),
            ByteRange::new(0, 512).unwrap(),
            &mut destination,
        );
        assert_eq!(read.disposition, CompletionDisposition::Success);
        drop(accepted);

        let status = quarantined_file_writer_sets()
            .into_iter()
            .find(|status| {
                status
                    .observation()
                    .members()
                    .iter()
                    .any(|member| member.binding().token().store_id() == StoreId(13))
            })
            .expect("dropped accepted ownership is quarantined");
        assert_eq!(
            status.observation().disposition(),
            FileBackedReleaseDisposition::Residual
        );
        assert_eq!(
            status.observation().members()[0].disposition(),
            FileWriterClaimDisposition::Held
        );
        assert!(status.observation().members()[0].cause().is_none());
        assert!(matches!(
            FileStore::open(config.clone()),
            Err(FileStoreError::Lease(FileLeaseError::AlreadyHeld(_)))
        ));

        assert_eq!(
            retry_quarantined_file_writer_set(status.id())
                .unwrap()
                .observation()
                .disposition(),
            FileBackedReleaseDisposition::ReleasedAll
        );
        assert!(
            quarantined_file_writer_sets()
                .iter()
                .all(|entry| entry.id() != status.id())
        );
        drop(FileStore::open(config).unwrap());
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn mixed_member_release_aggregates_released_and_held_conservatively() {
        let first_path = temp("mixed-release-first");
        let first_lock = temp("mixed-release-first-lock");
        let second_path = temp("mixed-release-second");
        let second_lock = temp("mixed-release-second-lock");
        for path in [&first_path, &first_lock, &second_path, &second_lock] {
            remove_if_present(path);
        }
        fs::write(&first_path, vec![0_u8; 4096]).unwrap();
        fs::write(&second_path, vec![0_u8; 4096]).unwrap();
        let first_config = FileStoreConfig::new(&first_path, 4096, 512)
            .store_id(StoreId(20))
            .lock_path(&first_lock);
        let second_config = FileStoreConfig::new(&second_path, 4096, 512)
            .store_id(StoreId(21))
            .lock_path(&second_lock);
        let first = FileStore::open(first_config).unwrap();
        let mut second = FileStore::open(second_config).unwrap();
        second.fail_next_release(FileOwnershipResource::BackingPayload);

        let Err(unresolved) = release_writer_set([StoreId(20), StoreId(21)], vec![first, second])
        else {
            panic!("mixed released and held members reported released-all");
        };
        assert_eq!(
            unresolved.observation().disposition(),
            FileBackedReleaseDisposition::Residual
        );
        assert_eq!(
            unresolved.observation().members()[0].disposition(),
            FileWriterClaimDisposition::Released
        );
        assert_eq!(
            unresolved.observation().members()[1].disposition(),
            FileWriterClaimDisposition::Held
        );
        assert!(unresolved.retry_release().is_ok());

        for path in [&first_lock, &first_path, &second_lock, &second_path] {
            remove_if_present(path);
        }
    }

    #[test]
    fn claim_token_is_fresh_after_disposable_marker_recreation() {
        let path = temp("fresh-claim-token");
        let lock_path = temp("fresh-claim-token-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(22))
            .lock_path(&lock_path);
        let first = FileStore::open(config.clone()).unwrap();
        let first_token = first.writer_claim_binding().unwrap().token().clone();
        assert!(release_writer_set([StoreId(22)], vec![first]).is_ok());
        remove_if_present(&lock_path);

        let second = FileStore::open(config).unwrap();
        let second_token = second.writer_claim_binding().unwrap().token().clone();
        assert_ne!(first_token, second_token);
        drop(second);
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn dropped_unresolved_set_retains_released_member_reservation_until_released_all() {
        let first_path = temp("quarantine-retry-first");
        let first_lock_path = temp("quarantine-retry-first-lock");
        let second_path = temp("quarantine-retry-second");
        let second_lock_path = temp("quarantine-retry-second-lock");
        for path in [
            &first_path,
            &first_lock_path,
            &second_path,
            &second_lock_path,
        ] {
            remove_if_present(path);
        }
        fs::write(&first_path, vec![0_u8; 4096]).unwrap();
        fs::write(&second_path, vec![0_u8; 4096]).unwrap();
        let first_config = FileStoreConfig::new(&first_path, 4096, 512)
            .store_id(StoreId(25))
            .lock_path(&first_lock_path);
        let second_config = FileStoreConfig::new(&second_path, 4096, 512)
            .store_id(StoreId(26))
            .lock_path(&second_lock_path);
        let mut first = FileStore::open(first_config.clone()).unwrap();
        let mut second = FileStore::open(second_config.clone()).unwrap();
        let acquisition_ids = [
            first
                .writer_claim_binding()
                .unwrap()
                .token()
                .acquisition_id()
                .clone(),
            second
                .writer_claim_binding()
                .unwrap()
                .token()
                .acquisition_id()
                .clone(),
        ];
        first.fail_next_release(FileOwnershipResource::BackingPayload);
        for _ in 0..3 {
            second.fail_next_release(FileOwnershipResource::BackingPayload);
        }
        let Err(unresolved) = release_writer_set([StoreId(25), StoreId(26)], vec![first, second])
        else {
            panic!("injected release failures reported released-all");
        };
        drop(unresolved);

        let status = quarantined_file_writer_sets()
            .into_iter()
            .find(|status| {
                status
                    .observation()
                    .members()
                    .iter()
                    .any(|member| member.binding().token().store_id() == StoreId(25))
            })
            .expect("dropped unresolved set is owned by quarantine");
        let id = status.id();
        assert_eq!(registered_claim_count(&acquisition_ids), 2);

        let Err(error) = retry_quarantined_file_writer_set(id) else {
            panic!("remaining injected failure unexpectedly released quarantine");
        };
        assert_eq!(
            error.failure(),
            FileWriterQuarantineRetryFailure::Unresolved
        );
        let observation = error.observation().unwrap();
        assert_eq!(
            observation.disposition(),
            FileBackedReleaseDisposition::Residual
        );
        assert_eq!(
            observation.members()[0].disposition(),
            FileWriterClaimDisposition::Released
        );
        assert_eq!(
            observation.members()[1].disposition(),
            FileWriterClaimDisposition::Held
        );
        {
            let registry = file_writer_quarantine_registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let quarantined = registry
                .entries
                .iter()
                .find(|entry| entry.id == id)
                .unwrap();
            assert_eq!(quarantined.claims.len(), 1);
            assert_eq!(quarantined.released_reservations.len(), 1);
        }
        assert_lock_available(&first_path);
        assert_lock_available(&first_lock_path);
        assert!(matches!(
            FileStore::open(first_config.clone()),
            Err(FileStoreError::Lease(FileLeaseError::AlreadyHeld(_)))
        ));

        let Err(error) = retry_quarantined_file_writer_set(id) else {
            panic!("second remaining failure unexpectedly released quarantine");
        };
        assert_eq!(
            error.failure(),
            FileWriterQuarantineRetryFailure::Unresolved
        );
        assert_eq!(registered_claim_count(&acquisition_ids), 2);
        {
            let registry = file_writer_quarantine_registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            assert_eq!(
                registry
                    .entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .unwrap()
                    .released_reservations
                    .len(),
                1
            );
        }

        assert_eq!(
            retry_quarantined_file_writer_set(id)
                .unwrap()
                .observation()
                .disposition(),
            FileBackedReleaseDisposition::ReleasedAll
        );
        assert_eq!(registered_claim_count(&acquisition_ids), 0);
        assert!(
            quarantined_file_writer_sets()
                .iter()
                .all(|status| status.id() != id)
        );
        let first = FileStore::open(first_config).unwrap();
        let second = FileStore::open(second_config).unwrap();
        drop((first, second));
        for path in [
            &first_lock_path,
            &first_path,
            &second_lock_path,
            &second_path,
        ] {
            remove_if_present(path);
        }
    }

    #[test]
    fn successful_release_cycles_remove_process_claim_history() {
        let path = temp("release-cycle-boundedness");
        let lock_path = temp("release-cycle-boundedness-lock");
        remove_if_present(&path);
        remove_if_present(&lock_path);
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(27))
            .lock_path(&lock_path);
        let mut completed_acquisitions = Vec::new();

        for _ in 0..4 {
            let store = FileStore::open(config.clone()).unwrap();
            let acquisition_id = store
                .writer_claim_binding()
                .unwrap()
                .token()
                .acquisition_id()
                .clone();
            assert_eq!(
                registered_claim_count(std::slice::from_ref(&acquisition_id)),
                1
            );
            assert!(release_writer_set([StoreId(27)], vec![store]).is_ok());
            completed_acquisitions.push(acquisition_id);
            assert_eq!(registered_claim_count(&completed_acquisitions), 0);
        }

        assert_lock_available(&path);
        assert_lock_available(&lock_path);
        remove_if_present(&lock_path);
        remove_if_present(&path);
    }

    #[test]
    fn accepted_set_rejects_same_store_id_replacement_claim() {
        let path = temp("accepted-binding");
        let original_path = temp("accepted-binding-original");
        let first_lock_path = temp("accepted-binding-first-lock");
        let replacement_lock_path = temp("accepted-binding-replacement-lock");
        for path in [
            &path,
            &original_path,
            &first_lock_path,
            &replacement_lock_path,
        ] {
            remove_if_present(path);
        }
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let first_config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(24))
            .lock_path(&first_lock_path);
        let first = FileStore::open(first_config).unwrap();
        let first_claim = first.writer_claim_binding().unwrap();
        let accepted_first = AcceptedFileWriterSet::accept([StoreId(24)], vec![first]).unwrap();

        fs::rename(&path, &original_path).unwrap();
        fs::write(&path, vec![1_u8; 4096]).unwrap();
        let replacement_config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(24))
            .lock_path(&replacement_lock_path);
        let replacement = FileStore::open(replacement_config.clone()).unwrap();
        let replacement_claim = replacement.writer_claim_binding().unwrap();
        let accepted_replacement =
            AcceptedFileWriterSet::accept([StoreId(24)], vec![replacement]).unwrap();
        assert_ne!(first_claim, replacement_claim);
        assert!(matches!(
            FileStore::open(replacement_config),
            Err(FileStoreError::Lease(FileLeaseError::AlreadyHeld(_)))
        ));

        let released_replacement = accepted_replacement.release().unwrap();
        assert_eq!(
            released_replacement.observation().members()[0].binding(),
            &replacement_claim
        );
        let released_first = accepted_first.release().unwrap();
        assert_eq!(
            released_first.observation().members()[0].binding(),
            &first_claim
        );

        for path in [
            &replacement_lock_path,
            &first_lock_path,
            &original_path,
            &path,
        ] {
            remove_if_present(path);
        }
    }

    #[test]
    fn unresolved_drop_holder_process() {
        let Ok(path) = std::env::var("DWV_TEST_UNRESOLVED_PATH") else {
            return;
        };
        let lock_path = std::env::var("DWV_TEST_UNRESOLVED_LOCK").unwrap();
        let resource = match std::env::var("DWV_TEST_UNRESOLVED_RESOURCE")
            .unwrap()
            .as_str()
        {
            "payload" => FileOwnershipResource::BackingPayload,
            "marker" => FileOwnershipResource::LeaseMarker,
            value => panic!("unknown release resource {value}"),
        };
        let config = FileStoreConfig::new(&path, 4096, 512)
            .store_id(StoreId(23))
            .lock_path(&lock_path);
        let mut store = FileStore::open(config.clone()).unwrap();
        store.fail_next_release(resource);
        let Err(unresolved) = release_writer_set([StoreId(23)], vec![store]) else {
            panic!("injected release failure reported released-all");
        };
        drop(unresolved);
        remove_if_present(Path::new(&lock_path));
        assert!(matches!(
            FileStore::open(config),
            Err(FileStoreError::Lease(FileLeaseError::AlreadyHeld(_)))
        ));
        fs::write(
            std::env::var("DWV_TEST_UNRESOLVED_READY").unwrap(),
            b"ready",
        )
        .unwrap();
        loop {
            std::thread::park();
        }
    }

    #[test]
    fn dropped_unresolved_ownership_blocks_until_process_death() {
        for resource in ["payload", "marker"] {
            let path = temp(&format!("unresolved-drop-{resource}"));
            let lock_path = temp(&format!("unresolved-drop-{resource}-lock"));
            let ready_path = temp(&format!("unresolved-drop-{resource}-ready"));
            for path in [&path, &lock_path, &ready_path] {
                remove_if_present(path);
            }
            fs::write(&path, vec![0_u8; 4096]).unwrap();
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "file_store::tests::unresolved_drop_holder_process",
                    "--exact",
                    "--nocapture",
                ])
                .env("DWV_TEST_UNRESOLVED_PATH", &path)
                .env("DWV_TEST_UNRESOLVED_LOCK", &lock_path)
                .env("DWV_TEST_UNRESOLVED_READY", &ready_path)
                .env("DWV_TEST_UNRESOLVED_RESOURCE", resource)
                .stdout(Stdio::null())
                .spawn()
                .unwrap();
            for _ in 0..100 {
                if ready_path.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert!(ready_path.exists());
            if resource == "payload" {
                assert!(matches!(
                    FileStore::open(FileStoreConfig::new(&path, 4096, 512).lock_path(&lock_path)),
                    Err(FileStoreError::Lease(FileLeaseError::AlreadyHeld(_)))
                ));
            }
            child.kill().unwrap();
            child.wait().unwrap();
            drop(
                FileStore::open(FileStoreConfig::new(&path, 4096, 512).lock_path(&lock_path))
                    .unwrap(),
            );
            for path in [&ready_path, &lock_path, &path] {
                remove_if_present(path);
            }
        }
    }
}
