use crate::lease::{FileIdentityError, observe_file_identity};
use dwv_store::{
    CapabilityEvidenceId, CapabilitySupport, IdentitySourceSet, StoreCapabilities, TornWriteModel,
    VolatileCacheModel,
};
use std::fmt;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SparseBehavior {
    Requested,
    Regular,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileCapabilityReport {
    pub capabilities: StoreCapabilities,
    pub identity: dwv_store::IdentityObservationSet,
    pub sparse_behavior: SparseBehavior,
    pub path: PathBuf,
}

#[derive(Debug)]
pub enum CapabilityProbeError {
    Io(std::io::Error),
    Identity(FileIdentityError),
    NotRegularFile(PathBuf),
    InvalidGeometry(String),
}

impl fmt::Display for CapabilityProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "capability probe failed: {error}"),
            Self::Identity(error) => write!(formatter, "identity probe failed: {error}"),
            Self::NotRegularFile(path) => {
                write!(formatter, "not a regular file: {}", path.display())
            }
            Self::InvalidGeometry(message) => write!(formatter, "invalid file geometry: {message}"),
        }
    }
}

impl std::error::Error for CapabilityProbeError {}

impl From<std::io::Error> for CapabilityProbeError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn probe_file_capabilities(
    path: &Path,
    protected_length: u64,
    logical_block_size: u32,
    maximum_transfer: u64,
    evidence_id: CapabilityEvidenceId,
    sparse_behavior: SparseBehavior,
) -> Result<FileCapabilityReport, CapabilityProbeError> {
    if logical_block_size == 0 || protected_length % u64::from(logical_block_size) != 0 {
        return Err(CapabilityProbeError::InvalidGeometry(
            "protected length must be a multiple of the non-zero logical block size".to_owned(),
        ));
    }
    if maximum_transfer == 0 || maximum_transfer % u64::from(logical_block_size) != 0 {
        return Err(CapabilityProbeError::InvalidGeometry(
            "maximum transfer must be a non-zero block multiple".to_owned(),
        ));
    }
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(CapabilityProbeError::NotRegularFile(path.to_path_buf()));
    }
    if metadata.len() != protected_length {
        return Err(CapabilityProbeError::InvalidGeometry(format!(
            "file length {} differs from protected length {protected_length}",
            metadata.len()
        )));
    }
    let identity = observe_file_identity(path).map_err(CapabilityProbeError::Identity)?;
    Ok(FileCapabilityReport {
        capabilities: capabilities_for(
            protected_length,
            logical_block_size,
            maximum_transfer,
            evidence_id,
            sparse_behavior,
            &metadata,
        ),
        identity,
        sparse_behavior,
        path: path.to_path_buf(),
    })
}

pub(crate) fn capabilities_for(
    protected_length: u64,
    logical_block_size: u32,
    maximum_transfer: u64,
    evidence_id: CapabilityEvidenceId,
    sparse_behavior: SparseBehavior,
    _metadata: &Metadata,
) -> StoreCapabilities {
    let mut capabilities = StoreCapabilities::portable_demo(
        protected_length,
        logical_block_size,
        maximum_transfer,
        evidence_id,
    );
    // These are facts about this adapter, not claims about a physical medium.
    capabilities.write_zeroes = CapabilitySupport::declared();
    capabilities.sparse_allocation = match sparse_behavior {
        SparseBehavior::Requested | SparseBehavior::Regular => CapabilitySupport::probed(),
        SparseBehavior::Unknown => CapabilitySupport::Unknown,
    };
    capabilities.cancellation = CapabilitySupport::Unsupported;
    capabilities.stable_identity_sources = IdentitySourceSet {
        stable_device_id: false,
        serial: false,
        filesystem_id: false,
        world_wide_name: false,
        file_id: true,
        path: false,
    };
    capabilities.torn_write_model = TornWriteModel::Unknown;
    capabilities.volatile_cache = VolatileCacheModel::Unknown;
    // Host fsync is exposed as adapter-level evidence, never physical FUA.
    capabilities.durable_flush = CapabilitySupport::probed();
    capabilities.fua = CapabilitySupport::Unsupported;
    capabilities.discard = CapabilitySupport::Unsupported;
    capabilities
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_store::Evidence;
    use std::fs;

    #[test]
    fn probe_reports_geometry_and_does_not_certify_physical_properties() {
        let path = std::env::temp_dir().join(format!("dwv-capabilities-{}", std::process::id()));
        let file = fs::File::create(&path).unwrap();
        file.set_len(4096).unwrap();
        let report = probe_file_capabilities(
            &path,
            4096,
            512,
            4096,
            CapabilityEvidenceId(7),
            SparseBehavior::Requested,
        )
        .unwrap();
        assert_eq!(report.capabilities.logical_length, Evidence::Known(4096));
        assert_eq!(report.capabilities.fua, CapabilitySupport::Unsupported);
        assert_eq!(report.capabilities.discard, CapabilitySupport::Unsupported);
        assert_eq!(
            report.capabilities.volatile_cache,
            VolatileCacheModel::Unknown
        );
        let _ = fs::remove_file(path);
    }
}
