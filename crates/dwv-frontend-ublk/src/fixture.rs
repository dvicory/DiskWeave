use crate::{AdapterError, DATA_SLOT, FRONTEND_ID, LOGICAL_BLOCK_SIZE, MAX_TRANSFER};
use dwv_core::{
    ArrayId, AssignmentGeneration, AssignmentInstanceId, BlockOp, BlockRequest, ByteRange,
    CodingPosition, CodingProfile, DurabilityIntent, FenceDomain, MemberRole, OrderingIntent,
    ProtectedGeometry, RequestId, SlotId, SubmissionSequence, TopologyAssignment, TopologyEpoch,
    TopologySnapshot,
};
use dwv_recovery::{
    CodedCaptureDecision, CodedCleanCommitObservation, MemoryRecoveryStore, RecoveryGeneration,
    RecoveryMutation, RecoveryStateStore, TopologySnapshot as RecoveryTopologySnapshot,
};
use dwv_recovery_sqlite::SqliteRecoveryStore;
use dwv_service::{HealthyPortableService, MemberBinding, ServiceConfig};
use dwv_store::StoreId;
use dwv_store_file::{FileStore, FileStoreConfig, FileSyncMode};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const MANIFEST_FILE: &str = "fixture.json";
const MARKER_FILE: &str = ".diskweave-ublk-fixture";
const CONTRACT: &str = "dwv.ublk.fixture.v1";
const DEFAULT_EPOCH: u64 = 1;
const DATA_STORE: StoreId = StoreId(10);
const PARITY_STORE: StoreId = StoreId(20);
const ARRAY_ID: [u8; 16] = [0x31; 16];
pub(crate) const PARITY_SLOT: SlotId = SlotId([2; 16]);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FixtureManifest {
    pub schema: u32,
    pub contract: String,
    pub protected_length: u64,
    pub logical_block_size: u32,
    pub topology_epoch: u64,
    pub array_id: [u8; 16],
    pub data_files: Vec<String>,
    pub data_identities: Vec<[u8; 16]>,
    pub parity_file: String,
    pub parity_identity: [u8; 16],
    pub recovery_file: String,
}

impl FixtureManifest {
    pub fn digest(&self) -> Result<String, AdapterError> {
        let bytes =
            serde_json::to_vec(self).map_err(|error| AdapterError::Io(error.to_string()))?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FixtureInspection {
    pub contract: String,
    pub protected_length: u64,
    pub topology_epoch: u64,
    pub fixture_digest: String,
    pub data_hash: String,
    pub parity_hash: String,
    pub parity_matches_data: bool,
    pub recovery_generation: u64,
    pub recovery_integrity: String,
}

pub struct Fixture {
    root: PathBuf,
    manifest: FixtureManifest,
}

pub struct OpenFixture {
    root: PathBuf,
    manifest: FixtureManifest,
    service: HealthyPortableService<FileStore, SqliteRecoveryStore>,
}

impl Fixture {
    pub fn initialize(root: &Path, size: u64) -> Result<FixtureManifest, AdapterError> {
        validate_size(size)?;
        if root.exists() {
            if !root.is_dir() {
                return Err(AdapterError::Conflict(
                    "fixture root is not a directory".into(),
                ));
            }
            if fs::read_dir(root)
                .map_err(io)?
                .next()
                .transpose()
                .map_err(io)?
                .is_some()
            {
                return Err(AdapterError::Conflict("fixture root is not empty".into()));
            }
        } else {
            fs::create_dir_all(root).map_err(io)?;
        }
        let root = fs::canonicalize(root).map_err(io)?;
        fs::write(
            root.join(MARKER_FILE),
            b"diskweave disposable linux ublk fixture\n",
        )
        .map_err(io)?;

        let mut manifest = FixtureManifest {
            schema: 1,
            contract: CONTRACT.into(),
            protected_length: size,
            logical_block_size: LOGICAL_BLOCK_SIZE,
            topology_epoch: DEFAULT_EPOCH,
            array_id: ARRAY_ID,
            data_files: vec!["data.raw".into()],
            data_identities: vec![[0; 16]],
            parity_file: "parity.raw".into(),
            parity_identity: [0; 16],
            recovery_file: "recovery.sqlite3".into(),
        };

        let data = create_store(&root, &manifest.data_files[0], size, DATA_STORE)?;
        let parity = create_store(&root, &manifest.parity_file, size, PARITY_STORE)?;
        manifest.data_identities[0] = identity(&data)?;
        manifest.parity_identity = identity(&parity)?;
        drop(data);
        drop(parity);

        let recovery_topology = recovery_topology(&manifest)?;
        let mut memory = MemoryRecoveryStore::new(TopologyEpoch(0));
        let mut transaction = memory.begin_protocol_txn(RecoveryGeneration::ZERO, TopologyEpoch(0));
        transaction.push(RecoveryMutation::PrepareTopology {
            topology: recovery_topology,
        });
        transaction.push(RecoveryMutation::CommitTopology {
            topology_epoch: TopologyEpoch(DEFAULT_EPOCH),
        });
        let generation = memory
            .commit_durable(transaction)
            .map_err(|error| AdapterError::Io(error.to_string()))?;
        let recovery_manifest = memory
            .export_manifest(generation)
            .map_err(|error| AdapterError::Io(error.to_string()))?;
        let recovery =
            SqliteRecoveryStore::create_new(root.join(&manifest.recovery_file), recovery_manifest)
                .map_err(|error| AdapterError::Io(error.to_string()))?;
        drop(recovery);

        atomic_manifest(&root, &manifest)?;
        Ok(manifest)
    }

    pub fn load(root: &Path) -> Result<Self, AdapterError> {
        let root = fs::canonicalize(root)
            .map_err(|error| AdapterError::Conflict(format!("fixture unavailable: {error}")))?;
        let marker = fs::read_to_string(root.join(MARKER_FILE)).map_err(|error| {
            AdapterError::Conflict(format!("fixture marker unavailable: {error}"))
        })?;
        if marker != "diskweave disposable linux ublk fixture\n" {
            return Err(AdapterError::Conflict(
                "fixture marker is not recognized".into(),
            ));
        }
        let manifest: FixtureManifest = serde_json::from_slice(
            &fs::read(root.join(MANIFEST_FILE)).map_err(io)?,
        )
        .map_err(|error| AdapterError::Conflict(format!("fixture manifest is invalid: {error}")))?;
        if manifest.schema != 1 || manifest.contract != CONTRACT {
            return Err(AdapterError::Unsupported("fixture contract"));
        }
        validate_size(manifest.protected_length)?;
        if manifest.logical_block_size != LOGICAL_BLOCK_SIZE {
            return Err(AdapterError::Unsupported("fixture logical block size"));
        }
        if manifest.data_files.len() != 1 || manifest.data_identities.len() != 1 {
            return Err(AdapterError::Unsupported(
                "initial publication profile requires exactly one data slot",
            ));
        }
        for relative in manifest
            .data_files
            .iter()
            .chain([&manifest.parity_file, &manifest.recovery_file])
        {
            owned_path(&root, relative, true)?;
        }
        let recovery = SqliteRecoveryStore::open(root.join(&manifest.recovery_file))
            .map_err(|error| AdapterError::Conflict(error.to_string()))?;
        let snapshot = recovery
            .load_assembly_snapshot()
            .map_err(|error| AdapterError::Conflict(error.to_string()))?;
        let expected_topology = recovery_topology(&manifest)?;
        if snapshot.topology_epoch != TopologyEpoch(manifest.topology_epoch)
            || snapshot.active_topology.as_ref() != Some(&expected_topology)
            || !matches!(
                recovery.verify_integrity(),
                dwv_recovery::RecoveryStoreHealth::Healthy
            )
        {
            return Err(AdapterError::Conflict(
                "fixture and recovery authority disagree".into(),
            ));
        }
        drop(recovery);
        Ok(Self { root, manifest })
    }

    pub fn manifest(&self) -> &FixtureManifest {
        &self.manifest
    }

    pub fn open(self) -> Result<OpenFixture, AdapterError> {
        let data = open_store(
            &self.root,
            &self.manifest.data_files[0],
            &self.manifest,
            DATA_STORE,
            true,
        )?;
        let data_identity = identity(&data)?;
        if data_identity != self.manifest.data_identities[0] {
            return Err(AdapterError::Conflict("data identity changed".into()));
        }
        let parity = open_store(
            &self.root,
            &self.manifest.parity_file,
            &self.manifest,
            PARITY_STORE,
            true,
        )?;
        let parity_identity = identity(&parity)?;
        if parity_identity != self.manifest.parity_identity {
            return Err(AdapterError::Conflict("parity identity changed".into()));
        }
        if data_identity == parity_identity {
            return Err(AdapterError::Conflict("data and parity alias".into()));
        }
        let recovery = SqliteRecoveryStore::open(self.root.join(&self.manifest.recovery_file))
            .map_err(|error| AdapterError::Conflict(error.to_string()))?;
        let topology = topology(&self.manifest)?;
        let epoch = topology.topology_epoch();
        let members = vec![
            MemberBinding::new(
                topology
                    .assignment_for_slot(DATA_SLOT)
                    .expect("fixture topology has a data assignment"),
                epoch,
                DATA_STORE,
                data,
            ),
            MemberBinding::new(
                topology
                    .assignment_for_slot(PARITY_SLOT)
                    .expect("fixture topology has a parity assignment"),
                epoch,
                PARITY_STORE,
                parity,
            ),
        ];
        let service = HealthyPortableService::open(
            topology,
            members,
            recovery,
            ServiceConfig {
                maximum_transfer: Some(MAX_TRANSFER),
                ..ServiceConfig::default()
            },
        )
        .map_err(|error| AdapterError::Conflict(error.to_string()))?;
        Ok(OpenFixture {
            root: self.root,
            manifest: self.manifest,
            service,
        })
    }

    pub fn inspect(&self) -> Result<FixtureInspection, AdapterError> {
        let data = open_store(
            &self.root,
            &self.manifest.data_files[0],
            &self.manifest,
            DATA_STORE,
            false,
        )?;
        let parity = open_store(
            &self.root,
            &self.manifest.parity_file,
            &self.manifest,
            PARITY_STORE,
            false,
        )?;
        if identity(&data)? != self.manifest.data_identities[0]
            || identity(&parity)? != self.manifest.parity_identity
        {
            return Err(AdapterError::Conflict("fixture identity changed".into()));
        }
        if identity(&data)? == identity(&parity)? {
            return Err(AdapterError::Conflict("data and parity alias".into()));
        }
        drop(data);
        drop(parity);
        let data_bytes =
            fs::read(owned_path(&self.root, &self.manifest.data_files[0], true)?).map_err(io)?;
        let parity_bytes =
            fs::read(owned_path(&self.root, &self.manifest.parity_file, true)?).map_err(io)?;
        let recovery = SqliteRecoveryStore::open(self.root.join(&self.manifest.recovery_file))
            .map_err(|error| AdapterError::Conflict(error.to_string()))?;
        let snapshot = recovery
            .load_assembly_snapshot()
            .map_err(|error| AdapterError::Io(error.to_string()))?;
        let integrity = recovery.verify_integrity();
        Ok(FixtureInspection {
            contract: CONTRACT.into(),
            protected_length: self.manifest.protected_length,
            topology_epoch: self.manifest.topology_epoch,
            fixture_digest: self.manifest.digest()?,
            data_hash: blake3::hash(&data_bytes).to_hex().to_string(),
            parity_hash: blake3::hash(&parity_bytes).to_hex().to_string(),
            parity_matches_data: data_bytes == parity_bytes,
            recovery_generation: snapshot.generation.0,
            recovery_integrity: format!("{integrity:?}"),
        })
    }
}

impl OpenFixture {
    pub fn manifest(&self) -> &FixtureManifest {
        &self.manifest
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn into_service(self) -> HealthyPortableService<FileStore, SqliteRecoveryStore> {
        self.service
    }

    pub fn execute(
        &mut self,
        request: BlockRequest,
        write_bytes: Option<&[u8]>,
    ) -> Result<Vec<u8>, AdapterError> {
        match request.op {
            BlockOp::Read => self
                .service
                .read(request)
                .map(|(bytes, _)| bytes)
                .map_err(|error| AdapterError::Io(error.to_string())),
            BlockOp::Write => {
                let bytes = write_bytes.ok_or(AdapterError::Invalid("write buffer is missing"))?;
                if bytes.len() as u64 != request.range.length {
                    return Err(AdapterError::Invalid(
                        "write buffer length differs from request",
                    ));
                }
                self.service
                    .write_with_clean_capture(
                        request,
                        bytes,
                        CodedCaptureDecision::Accepted,
                        CodedCleanCommitObservation::Durable,
                    )
                    .map(|_| Vec::new())
                    .map_err(|error| AdapterError::Io(error.to_string()))
            }
            BlockOp::Flush => self
                .service
                .flush(request)
                .map(|_| Vec::new())
                .map_err(|error| AdapterError::Io(error.to_string())),
            _ => Err(AdapterError::Unsupported("normalized operation")),
        }
    }

    pub fn flush(&mut self, request_id: u64) -> Result<(), AdapterError> {
        let request = BlockRequest::new(
            RequestId(request_id),
            FRONTEND_ID,
            DATA_SLOT,
            TopologyEpoch(self.manifest.topology_epoch),
            BlockOp::Flush,
            ByteRange::empty(),
            None,
            OrderingIntent {
                submission_sequence: SubmissionSequence(request_id),
                preflush: false,
                fence_domain: FenceDomain(1),
            },
            DurabilityIntent::ExplicitFlush,
        );
        self.service
            .flush(request)
            .map(|_| ())
            .map_err(|error| AdapterError::ReconciliationRequired(error.to_string()))
    }
}

fn topology(manifest: &FixtureManifest) -> Result<TopologySnapshot, AdapterError> {
    TopologySnapshot::new(
        ArrayId(manifest.array_id),
        TopologyEpoch(manifest.topology_epoch),
        CodingProfile::new(1, 1).map_err(|error| AdapterError::Io(format!("{error:?}")))?,
        ProtectedGeometry::new(manifest.protected_length, manifest.logical_block_size)
            .map_err(|error| AdapterError::Io(format!("{error:?}")))?,
        vec![
            TopologyAssignment::new(
                DATA_SLOT,
                MemberRole::Data,
                CodingPosition(0),
                AssignmentInstanceId([11; 16]),
                AssignmentGeneration(1),
            ),
            TopologyAssignment::new(
                PARITY_SLOT,
                MemberRole::Parity,
                CodingPosition(1),
                AssignmentInstanceId([12; 16]),
                AssignmentGeneration(1),
            ),
        ],
    )
    .map_err(|error| AdapterError::Io(error.to_string()))
}

fn recovery_topology(manifest: &FixtureManifest) -> Result<RecoveryTopologySnapshot, AdapterError> {
    RecoveryTopologySnapshot::from_core(topology(manifest)?, vec![DATA_STORE, PARITY_STORE])
        .map_err(|error| AdapterError::Io(error.to_string()))
}

fn validate_size(size: u64) -> Result<(), AdapterError> {
    if !(16 * 1024 * 1024..=1024 * 1024 * 1024).contains(&size)
        || !size.is_multiple_of(u64::from(LOGICAL_BLOCK_SIZE))
    {
        return Err(AdapterError::Invalid(
            "fixture size must be 512-byte aligned between 16 MiB and 1 GiB",
        ));
    }
    Ok(())
}

fn create_store(
    root: &Path,
    relative: &str,
    length: u64,
    store_id: StoreId,
) -> Result<FileStore, AdapterError> {
    FileStore::open(
        FileStoreConfig::new(
            owned_path(root, relative, false)?,
            length,
            LOGICAL_BLOCK_SIZE,
        )
        .maximum_transfer(MAX_TRANSFER)
        .writable(true)
        .create_new(true)
        .sparse(false)
        .sync_mode(FileSyncMode::SyncAll)
        .store_id(store_id)
        .topology_epoch(TopologyEpoch(DEFAULT_EPOCH)),
    )
    .map_err(|error| AdapterError::Io(error.to_string()))
}

fn open_store(
    root: &Path,
    relative: &str,
    manifest: &FixtureManifest,
    store_id: StoreId,
    writable: bool,
) -> Result<FileStore, AdapterError> {
    FileStore::open(
        FileStoreConfig::new(
            owned_path(root, relative, true)?,
            manifest.protected_length,
            manifest.logical_block_size,
        )
        .maximum_transfer(MAX_TRANSFER)
        .writable(writable)
        .sync_mode(FileSyncMode::SyncAll)
        .store_id(store_id)
        .topology_epoch(TopologyEpoch(manifest.topology_epoch)),
    )
    .map_err(|error| AdapterError::Conflict(error.to_string()))
}

fn identity(store: &FileStore) -> Result<[u8; 16], AdapterError> {
    store
        .capabilities_report()
        .identity
        .observations
        .first()
        .map(|observation| observation.fingerprint)
        .ok_or_else(|| AdapterError::Conflict("file identity is unavailable".into()))
}

fn owned_path(root: &Path, relative: &str, must_exist: bool) -> Result<PathBuf, AdapterError> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(AdapterError::Conflict(
            "fixture path escapes its owned root".into(),
        ));
    }
    let root = fs::canonicalize(root).map_err(io)?;
    let joined = root.join(relative);
    let checked = if must_exist {
        fs::canonicalize(&joined).map_err(io)?
    } else {
        let parent = joined
            .parent()
            .ok_or(AdapterError::Invalid("fixture entry has no parent"))?;
        let parent = fs::canonicalize(parent).map_err(io)?;
        parent.join(
            joined
                .file_name()
                .ok_or(AdapterError::Invalid("fixture entry has no name"))?,
        )
    };
    if !checked.starts_with(&root) {
        return Err(AdapterError::Conflict(
            "fixture path escapes its owned root".into(),
        ));
    }
    Ok(checked)
}

fn atomic_manifest(root: &Path, manifest: &FixtureManifest) -> Result<(), AdapterError> {
    let destination = root.join(MANIFEST_FILE);
    let temporary = root.join("fixture.json.new");
    let bytes =
        serde_json::to_vec_pretty(manifest).map_err(|error| AdapterError::Io(error.to_string()))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(io)?;
    file.write_all(&bytes).map_err(io)?;
    file.sync_all().map_err(io)?;
    drop(file);
    fs::rename(&temporary, &destination).map_err(io)?;
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(io)
}

fn io(error: std::io::Error) -> AdapterError {
    AdapterError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_core::{
        BufferToken, DurabilityIntent, FenceDomain, FrontendId, OrderingIntent, SubmissionSequence,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dwv-ublk-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn fixture_routes_io_and_remains_plainly_readable() {
        let root = temp_root("roundtrip");
        Fixture::initialize(&root, 16 * 1024 * 1024).unwrap();
        let fixture = Fixture::load(&root).unwrap();
        let epoch = TopologyEpoch(fixture.manifest().topology_epoch);
        let mut opened = fixture.open().unwrap();
        let range = dwv_core::ByteRange::new(4096, 4096).unwrap();
        let write = BlockRequest::new(
            RequestId(1),
            FrontendId(1),
            DATA_SLOT,
            epoch,
            BlockOp::Write,
            range,
            Some(BufferToken::new(0, 1)),
            OrderingIntent {
                submission_sequence: SubmissionSequence(1),
                preflush: false,
                fence_domain: FenceDomain(1),
            },
            DurabilityIntent::Ordinary,
        );
        opened.execute(write, Some(&vec![0xa5; 4096])).unwrap();
        opened.flush(2).unwrap();
        drop(opened);
        let inspection = Fixture::load(&root).unwrap().inspect().unwrap();
        assert!(inspection.parity_matches_data);
        let bytes = fs::read(root.join("data.raw")).unwrap();
        assert_eq!(&bytes[4096..8192], &[0xa5; 4096]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fixture_rejects_escape_replacement_and_wider_profile() {
        let undersized = temp_root("undersized");
        assert!(matches!(
            Fixture::initialize(&undersized, 4096),
            Err(AdapterError::Invalid(_))
        ));
        assert!(!undersized.exists());

        let root = temp_root("refusal");
        let mut manifest = Fixture::initialize(&root, 16 * 1024 * 1024).unwrap();
        manifest.data_files[0] = "../outside.raw".into();
        fs::write(
            root.join(MANIFEST_FILE),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            Fixture::load(&root),
            Err(AdapterError::Conflict(_))
        ));

        manifest.data_files = vec!["data.raw".into(), "data.raw".into()];
        manifest.data_identities.push(manifest.data_identities[0]);
        fs::write(
            root.join(MANIFEST_FILE),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            Fixture::load(&root),
            Err(AdapterError::Unsupported(_))
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fixture_rejects_replacement_resize_alias_and_stale_epoch() {
        let replaced = temp_root("replaced");
        Fixture::initialize(&replaced, 16 * 1024 * 1024).unwrap();
        fs::rename(replaced.join("data.raw"), replaced.join("old.raw")).unwrap();
        fs::write(replaced.join("data.raw"), vec![0; 16 * 1024 * 1024]).unwrap();
        assert!(Fixture::load(&replaced).unwrap().open().is_err());
        fs::remove_dir_all(replaced).unwrap();

        let resized = temp_root("resized");
        Fixture::initialize(&resized, 16 * 1024 * 1024).unwrap();
        OpenOptions::new()
            .write(true)
            .open(resized.join("data.raw"))
            .unwrap()
            .set_len(16 * 1024 * 1024 - 512)
            .unwrap();
        assert!(Fixture::load(&resized).unwrap().open().is_err());
        fs::remove_dir_all(resized).unwrap();

        let aliased = temp_root("aliased");
        let mut manifest = Fixture::initialize(&aliased, 16 * 1024 * 1024).unwrap();
        manifest.parity_file = manifest.data_files[0].clone();
        manifest.parity_identity = manifest.data_identities[0];
        fs::write(
            aliased.join(MANIFEST_FILE),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(Fixture::load(&aliased).unwrap().open().is_err());
        fs::remove_dir_all(aliased).unwrap();

        let stale = temp_root("stale");
        let mut manifest = Fixture::initialize(&stale, 16 * 1024 * 1024).unwrap();
        manifest.topology_epoch += 1;
        fs::write(
            stale.join(MANIFEST_FILE),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            Fixture::load(&stale),
            Err(AdapterError::Conflict(_))
        ));
        fs::remove_dir_all(stale).unwrap();
    }

    #[test]
    fn failed_partial_claim_is_released() {
        let root = temp_root("claim-release");
        let manifest = Fixture::initialize(&root, 16 * 1024 * 1024).unwrap();
        let parity_claim =
            open_store(&root, &manifest.parity_file, &manifest, PARITY_STORE, true).unwrap();
        assert!(Fixture::load(&root).unwrap().open().is_err());
        drop(parity_claim);
        let opened = Fixture::load(&root).unwrap().open().unwrap();
        drop(opened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovery_lock_and_corruption_fail_closed_without_payload_mutation() {
        let root = temp_root("recovery-failure");
        let manifest = Fixture::initialize(&root, 16 * 1024 * 1024).unwrap();
        let data_before = fs::read(root.join(&manifest.data_files[0])).unwrap();
        let parity_before = fs::read(root.join(&manifest.parity_file)).unwrap();
        let recovery_path = root.join(&manifest.recovery_file);

        let claim = SqliteRecoveryStore::open(&recovery_path).unwrap();
        assert!(matches!(
            Fixture::load(&root),
            Err(AdapterError::Conflict(_))
        ));
        drop(claim);
        Fixture::load(&root).unwrap();

        fs::write(&recovery_path, b"corrupt recovery authority").unwrap();
        assert!(matches!(
            Fixture::load(&root),
            Err(AdapterError::Conflict(_))
        ));
        fs::remove_file(&recovery_path).unwrap();
        assert!(matches!(Fixture::load(&root), Err(AdapterError::Io(_))));
        assert_eq!(
            fs::read(root.join(&manifest.data_files[0])).unwrap(),
            data_before
        );
        assert_eq!(
            fs::read(root.join(&manifest.parity_file)).unwrap(),
            parity_before
        );
        fs::remove_dir_all(root).unwrap();
    }
}
