use dwv_store::{
    IdentityAssessment, IdentityComparison as StoreIdentityComparison, IdentityObservation,
    IdentityObservationSet,
};
use std::fmt;
use std::fs::{File, Metadata, OpenOptions, rename};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityComparison {
    Unchanged,
    Changed,
    Ambiguous,
}

impl From<StoreIdentityComparison> for IdentityComparison {
    fn from(value: StoreIdentityComparison) -> Self {
        match value {
            StoreIdentityComparison::Unchanged => Self::Unchanged,
            StoreIdentityComparison::Changed => Self::Changed,
            StoreIdentityComparison::Ambiguous => Self::Ambiguous,
        }
    }
}

#[derive(Debug)]
pub enum FileIdentityError {
    Io(std::io::Error),
    NotARegularFile(PathBuf),
}

impl fmt::Display for FileIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "file identity probe failed: {error}"),
            Self::NotARegularFile(path) => {
                write!(formatter, "not a regular file: {}", path.display())
            }
        }
    }
}

impl std::error::Error for FileIdentityError {}

impl From<std::io::Error> for FileIdentityError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Observe identity using filesystem metadata, never the path alone.
pub fn observe_file_identity(path: &Path) -> Result<IdentityObservationSet, FileIdentityError> {
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(FileIdentityError::NotARegularFile(path.to_path_buf()));
    }
    Ok(identity_from_metadata(&metadata))
}

pub(crate) fn identity_from_metadata(metadata: &Metadata) -> IdentityObservationSet {
    let mut fingerprint = [0_u8; 16];
    let (first, second) = metadata_identity_parts(metadata);
    fingerprint[..8].copy_from_slice(&first.to_le_bytes());
    fingerprint[8..].copy_from_slice(&second.to_le_bytes());
    IdentityObservationSet::new(
        vec![IdentityObservation {
            source: dwv_store::IdentitySourceKind::FileId,
            fingerprint,
        }],
        IdentityAssessment::Confirmed,
    )
}

#[cfg(unix)]
fn metadata_identity_parts(metadata: &Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    (metadata.dev(), metadata.ino())
}

#[cfg(not(unix))]
fn metadata_identity_parts(metadata: &Metadata) -> (u64, u64) {
    // The crate is intended for Unix hosts.  This fallback deliberately does
    // not claim a stable identity on other platforms.
    (metadata.len(), 0)
}

pub(crate) fn compare_identity(
    captured: &IdentityObservationSet,
    path: &Path,
) -> Result<IdentityComparison, FileIdentityError> {
    let observed = observe_file_identity(path)?;
    Ok(captured.compare(&observed).into())
}

#[derive(Debug)]
pub enum FileLeaseError {
    Io(std::io::Error),
    AlreadyHeld(PathBuf),
}

impl fmt::Display for FileLeaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "file lease failed: {error}"),
            Self::AlreadyHeld(path) => {
                write!(formatter, "file lease already held: {}", path.display())
            }
        }
    }
}

impl std::error::Error for FileLeaseError {}

/// A crash-releasing single-writer claim held by an operating-system advisory
/// lock on an open descriptor. Each successful acquisition durably appends a
/// byte and uses the resulting file length as a monotonic store incarnation.
pub struct FileLease {
    path: PathBuf,
    marker: Option<File>,
    incarnation: u64,
}

impl fmt::Debug for FileLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileLease")
            .field("path", &self.path)
            .finish()
    }
}

impl FileLease {
    pub fn acquire(backing_path: &Path) -> Result<Self, FileLeaseError> {
        Self::acquire_at(default_lease_path(backing_path))
    }

    pub fn acquire_at(path: impl Into<PathBuf>) -> Result<Self, FileLeaseError> {
        let path = path.into();
        let mut options = OpenOptions::new();
        options.read(true).append(true).create(true);
        let mut marker = options.open(&path).map_err(FileLeaseError::Io)?;
        match marker.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(FileLeaseError::AlreadyHeld(path));
            }
            Err(error) => return Err(FileLeaseError::Io(error.into())),
        }
        let incarnation = marker
            .metadata()
            .and_then(|metadata| {
                metadata
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| std::io::Error::other("store incarnation exhausted"))
            })
            .map_err(FileLeaseError::Io)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        writeln!(
            marker,
            "incarnation={incarnation} pid={} acquired_ns={}",
            std::process::id(),
            stamp.as_nanos()
        )
        .and_then(|_| marker.sync_data())
        .map_err(FileLeaseError::Io)?;
        Ok(Self {
            path,
            marker: Some(marker),
            incarnation,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn incarnation(&self) -> u64 {
        self.incarnation
    }

    pub fn release(&mut self) -> Result<(), FileLeaseError> {
        if let Some(marker) = self.marker.take() {
            File::unlock(&marker).map_err(FileLeaseError::Io)?;
        }
        Ok(())
    }
}

impl Drop for FileLease {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

pub(crate) fn default_lease_path(backing_path: &Path) -> PathBuf {
    let mut path = backing_path.as_os_str().to_os_string();
    path.push(".dwv-lease");
    PathBuf::from(path)
}

#[derive(Debug)]
pub enum AliasError {
    Identity(FileIdentityError),
    SameBackingAndExport(PathBuf),
    AmbiguousIdentity,
}

impl fmt::Display for AliasError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(error) => write!(formatter, "alias identity probe failed: {error}"),
            Self::SameBackingAndExport(path) => {
                write!(formatter, "backing/export aliases: {}", path.display())
            }
            Self::AmbiguousIdentity => write!(formatter, "backing/export identity is ambiguous"),
        }
    }
}

impl std::error::Error for AliasError {}

/// Refuse an active endpoint if both paths resolve to the same file identity.
pub fn reject_backing_export_alias(
    backing_path: &Path,
    export_path: &Path,
) -> Result<(), AliasError> {
    let backing = observe_file_identity(backing_path).map_err(AliasError::Identity)?;
    let export = observe_file_identity(export_path).map_err(AliasError::Identity)?;
    match backing.compare(&export) {
        StoreIdentityComparison::Unchanged => {
            Err(AliasError::SameBackingAndExport(backing_path.to_path_buf()))
        }
        StoreIdentityComparison::Changed => Ok(()),
        StoreIdentityComparison::Ambiguous => Err(AliasError::AmbiguousIdentity),
    }
}

pub(crate) fn quarantine_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(format!(".corrupt-{}", std::process::id()));
    PathBuf::from(value)
}

pub(crate) fn quarantine(path: &Path) -> Result<PathBuf, std::io::Error> {
    let target = quarantine_path(path);
    rename(path, &target)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::{Command, Stdio};

    #[test]
    fn lease_is_atomic_and_identity_is_path_independent() {
        let root = std::env::temp_dir().join(format!("dwv-lease-{}", std::process::id()));
        let _ = fs::remove_file(&root);
        fs::write(&root, b"payload").unwrap();
        let mut lease = FileLease::acquire(&root).unwrap();
        assert!(matches!(
            FileLease::acquire(&root),
            Err(FileLeaseError::AlreadyHeld(_))
        ));
        let identity = observe_file_identity(&root).unwrap();
        let alias = root.with_file_name(format!("alias-{}", std::process::id()));
        std::fs::hard_link(&root, &alias).unwrap();
        assert_eq!(
            identity.compare(&observe_file_identity(&alias).unwrap()),
            StoreIdentityComparison::Unchanged
        );
        lease.release().unwrap();
        let _ = fs::remove_file(alias);
        let _ = fs::remove_file(default_lease_path(&root));
        let _ = fs::remove_file(root);
    }

    #[test]
    fn lease_holder_process() {
        let Ok(path) = std::env::var("DWV_TEST_LEASE_PATH") else {
            return;
        };
        let _lease = FileLease::acquire_at(path).unwrap();
        fs::write(std::env::var("DWV_TEST_LEASE_READY").unwrap(), b"ready").unwrap();
        loop {
            std::thread::park();
        }
    }

    #[test]
    fn process_death_releases_lease() {
        let marker = std::env::temp_dir().join(format!("dwv-crash-lease-{}", std::process::id()));
        let ready_path = marker.with_extension("ready");
        let _ = fs::remove_file(&ready_path);
        let _ = fs::remove_file(&marker);
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "lease::tests::lease_holder_process",
                "--exact",
                "--nocapture",
            ])
            .env("DWV_TEST_LEASE_PATH", &marker)
            .env("DWV_TEST_LEASE_READY", &ready_path)
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
            FileLease::acquire_at(&marker),
            Err(FileLeaseError::AlreadyHeld(_))
        ));
        child.kill().unwrap();
        child.wait().unwrap();
        FileLease::acquire_at(&marker).unwrap();
        fs::remove_file(marker).unwrap();
        fs::remove_file(ready_path).unwrap();
    }
}
