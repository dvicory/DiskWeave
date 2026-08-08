use dwv_store::{
    IdentityAssessment, IdentityComparison as StoreIdentityComparison, IdentityObservation,
    IdentityObservationSet,
};
use std::fmt;
use std::fs::{File, Metadata, OpenOptions, remove_file, rename};
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

/// An atomic, ephemeral, single-writer claim represented by an exclusive lock
/// file.  It is intentionally not a payload sidecar and has no recovery
/// meaning after the owning process exits.
pub struct FileLease {
    path: PathBuf,
    marker: Option<File>,
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
        options.write(true).create_new(true);
        let mut marker = match options.open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(FileLeaseError::AlreadyHeld(path));
            }
            Err(error) => return Err(FileLeaseError::Io(error)),
        };
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let _ = writeln!(
            marker,
            "pid={} acquired_ns={}",
            std::process::id(),
            stamp.as_nanos()
        );
        Ok(Self {
            path,
            marker: Some(marker),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn release(&mut self) -> Result<(), FileLeaseError> {
        if self.marker.take().is_some() {
            remove_file(&self.path).map_err(FileLeaseError::Io)?;
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
        let _ = fs::remove_file(root);
    }
}
