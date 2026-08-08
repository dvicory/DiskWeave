use crate::capabilities::{
    CapabilityProbeError, FileCapabilityReport, SparseBehavior, probe_file_capabilities,
};
use crate::lease::{
    FileIdentityError, FileLease, FileLeaseError, IdentityComparison, compare_identity,
    identity_from_metadata,
};
use dwv_store::{
    BufferToken, ByteRange, CapabilityEvidenceId, ChildOperationId, CompletedRangeSet,
    CompletionDisposition, IdentityObservationSet, OperationId, PersistenceEvidence,
    RandomAccessStore, StoreCapabilities, StoreCompletion, StoreError, StoreId, StoreOperation,
    StoreRequest, StoreRequestKind, StoreWriteWatermark, TopologyEpoch, WriteIntent,
};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

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

pub struct FileStore {
    file: File,
    config: FileStoreConfig,
    capabilities: StoreCapabilities,
    identity: IdentityObservationSet,
    lease: Option<FileLease>,
    buffers: BTreeMap<BufferToken, Vec<u8>>,
}

impl fmt::Debug for FileStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileStore")
            .field("path", &self.config.path)
            .field("protected_length", &self.config.protected_length)
            .field("writable", &self.config.writable)
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
        let mut created_path = if config.create_new {
            match CreatedPathReservation::new(config.path.clone(), &file) {
                Ok(reservation) => Some(reservation),
                Err(error) => {
                    drop(lease.take());
                    return Err(FileStoreError::Io(error));
                }
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
            drop(lease);
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
        if let Some(reservation) = created_path.as_mut() {
            reservation.commit();
        }
        Ok(Self {
            file,
            config,
            capabilities: report.capabilities,
            identity: report.identity,
            lease,
            buffers: BTreeMap::new(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.config.path
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
        self.lease.as_ref()
    }
    pub fn identity_is_current(&self) -> Result<IdentityComparison, FileStoreError> {
        compare_identity(&self.identity, &self.config.path).map_err(FileStoreError::Identity)
    }
    pub fn register_buffer(&mut self, token: BufferToken, bytes: Vec<u8>) {
        self.buffers.insert(token, bytes);
    }
    pub fn take_read_buffer(&mut self, token: BufferToken) -> Option<Vec<u8>> {
        self.buffers.remove(&token)
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
            );
        }
        let progress = write_all_at(&self.file, range.offset, bytes);
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
            );
        }
        match self.config.sync_mode {
            FileSyncMode::CallerFlush => completion(
                operation_id,
                range,
                completed,
                CompletionDisposition::Success,
                PersistenceEvidence::VolatileOrUnknown,
            ),
            FileSyncMode::SyncData | FileSyncMode::SyncAll => match self.sync_file() {
                Ok(()) => completion(
                    operation_id,
                    range,
                    completed,
                    CompletionDisposition::Success,
                    self.fence_evidence(operation_id),
                ),
                Err(_error) => completion(
                    operation_id,
                    range,
                    completed,
                    CompletionDisposition::Uncertain,
                    PersistenceEvidence::VolatileOrUnknown,
                ),
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
            );
        }
        let chunk =
            usize::try_from(self.config.maximum_transfer.min(1024 * 1024)).unwrap_or(1024 * 1024);
        let zeros = vec![0_u8; chunk];
        let mut offset = range.offset;
        let mut remaining = range.length;
        while remaining != 0 {
            let length = usize::try_from(remaining.min(zeros.len() as u64)).unwrap_or(zeros.len());
            let progress = write_all_at(&self.file, offset, &zeros[..length]);
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
                );
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
                );
            }
            return completion(
                operation_id,
                range,
                range.length,
                CompletionDisposition::Success,
                self.fence_evidence(operation_id),
            );
        }
        completion(
            operation_id,
            range,
            range.length,
            CompletionDisposition::Success,
            PersistenceEvidence::VolatileOrUnknown,
        )
    }

    pub fn flush_file(
        &mut self,
        operation_id: ChildOperationId,
        through: StoreWriteWatermark,
    ) -> StoreCompletion {
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
        self.validate(range, StoreOperation::Read, None, true)?;
        let length = usize::try_from(range.length).map_err(|_| StoreError::TransferTooLarge {
            length: range.length,
            maximum: usize::MAX as u64,
        })?;
        let mut bytes = vec![0_u8; length];
        let mut completed = 0_u64;
        while completed < range.length {
            match read_at(
                &self.file,
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
        bytes.truncate(completed as usize);
        Ok(ReadProgress { bytes, completed })
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
            FileSyncMode::SyncData => self.file.sync_data(),
            FileSyncMode::CallerFlush | FileSyncMode::SyncAll => self.file.sync_all(),
        }
    }
    fn store_id(&self) -> StoreId {
        if self.config.store_id == StoreId(0) {
            StoreId(evidence_id(&self.config.path, &self.config).0)
        } else {
            self.config.store_id
        }
    }
    fn fence_evidence(&self, operation_id: ChildOperationId) -> PersistenceEvidence {
        self.fence_evidence_through(StoreWriteWatermark(u64::from(operation_id.index) + 1))
    }
    fn fence_evidence_through(&self, through: StoreWriteWatermark) -> PersistenceEvidence {
        PersistenceEvidence::DurableByFence {
            fence: dwv_store::StoreFenceRef {
                fence_id: dwv_store::FenceId(through.0),
                store_id: self.store_id(),
                topology_epoch: self.config.topology_epoch,
                through,
                capability_evidence_id: self.capabilities.evidence_id,
            },
        }
    }
}

impl RandomAccessStore for FileStore {
    fn identity_observations(&self) -> IdentityObservationSet {
        self.identity.clone()
    }
    fn capabilities(&self) -> StoreCapabilities {
        self.capabilities.clone()
    }
    fn length(&self) -> u64 {
        self.config.protected_length
    }

    fn read_at(
        &mut self,
        operation_id: ChildOperationId,
        range: ByteRange,
        destination: BufferToken,
    ) -> StoreCompletion {
        if let Err(error) = self.validate(range, StoreOperation::Read, None, true) {
            return failed(operation_id, range, error, 0);
        }
        match self.read_progress(range) {
            Ok(progress) if progress.completed == range.length => {
                self.buffers.insert(destination, progress.bytes);
                completion(
                    operation_id,
                    range,
                    range.length,
                    CompletionDisposition::Success,
                    PersistenceEvidence::VolatileOrUnknown,
                )
            }
            Ok(progress) => completion(
                operation_id,
                range,
                progress.completed,
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
        source: BufferToken,
        intent: WriteIntent,
    ) -> StoreCompletion {
        let bytes = match self.buffers.get(&source).cloned() {
            Some(bytes) => bytes,
            None => {
                return failed(
                    operation_id,
                    range,
                    StoreError::MissingBuffer {
                        operation: StoreOperation::Write,
                    },
                    0,
                );
            }
        };
        self.write_bytes(operation_id, range, &bytes, intent)
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
    use dwv_store::{OperationSlotToken, RandomAccessStore};
    use std::fs;

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
        assert!(!lock_path.exists());
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
        assert!(!lock_path.exists());
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
        assert!(!lock_path.exists());
        assert_eq!(fs::read(&path).unwrap(), vec![0x3c_u8; 4096]);

        remove_if_present(&path);
        let store = FileStore::open(config).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), 4096);
        drop(store);
        assert!(!lock_path.exists());
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
        assert!(!lock_path.exists());
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
    fn trait_buffers_and_flush_produce_exact_completion_evidence() {
        let path = temp("trait");
        fs::write(&path, vec![0_u8; 4096]).unwrap();
        let mut store = FileStore::open(
            FileStoreConfig::new(&path, 4096, 512).sync_mode(FileSyncMode::SyncAll),
        )
        .unwrap();
        let token = BufferToken::new(2, 1);
        store.register_buffer(token, vec![7_u8; 512]);
        let write = store.write_at(
            operation(3),
            ByteRange::new(0, 512).unwrap(),
            token,
            WriteIntent::Ordinary,
        );
        assert!(write.persistence.is_durable());
        let flush = store.flush(operation(4), StoreWriteWatermark(3));
        assert!(flush.persistence.is_durable());
        let destination = BufferToken::new(3, 1);
        let read = store.read_at(operation(5), ByteRange::new(0, 512).unwrap(), destination);
        assert_eq!(read.disposition, CompletionDisposition::Success);
        assert_eq!(
            store.take_read_buffer(destination).unwrap(),
            vec![7_u8; 512]
        );
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
}
