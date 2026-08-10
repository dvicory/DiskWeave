//! Portable SQLite-backed recovery-state adapter.
//!
//! `SqliteRecoveryStore` implements the semantic recovery port with an
//! exclusive process lease, complete-manifest validation, generation-checked
//! persistence, and candidate-before-publish commits. `SqlitePrototype`
//! remains an evaluation helper for migrations, configuration candidates, and
//! metadata-loss experiments; neither type alone certifies physical
//! power-loss behavior.

use dwv_recovery::{
    MemoryRecoveryStore, MetadataLossAuthorization, RecoveryError, RecoveryGeneration,
    RecoveryManifest, RecoverySchemaVersion, RecoverySnapshot, RecoveryStateStore,
    RecoveryStoreHealth, RecoveryTxn, SqliteEvaluationCase, SqliteJournalMode,
    SqliteSynchronousMode, TopologySnapshot,
};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

pub const RECOVERY_SQLITE_SCHEMA_V1: &str = include_str!("../migrations/0001_recovery_state.sql");
pub const RECOVERY_SQLITE_SCHEMA_V2: &str =
    include_str!("../migrations/0002_complete_manifest.sql");
pub const CURRENT_RECOVERY_SQLITE_SCHEMA: u64 = 2;
pub const MAX_MANIFEST_JSON_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SqlitePrototypeError {
    Io(String),
    SqliteUnavailable,
    CommandFailed { status: Option<i32>, stderr: String },
    InvalidOutput(String),
    MissingState,
    MissingCompleteManifest,
    ExistingTarget,
    UnsupportedStorageSchema(u64),
    GenerationConflict { expected: RecoveryGeneration },
    ManifestTooLarge { actual: usize, maximum: usize },
    ManifestIntegrity,
    Semantic(String),
}

impl fmt::Display for SqlitePrototypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "sqlite prototype I/O failed: {message}"),
            Self::SqliteUnavailable => write!(formatter, "sqlite3 executable is unavailable"),
            Self::CommandFailed { status, stderr } => {
                write!(formatter, "sqlite3 command failed ({status:?}): {stderr}")
            }
            Self::InvalidOutput(message) => write!(formatter, "invalid sqlite output: {message}"),
            Self::MissingState => write!(formatter, "recovery state is missing"),
            Self::MissingCompleteManifest => {
                write!(formatter, "recovery state has no complete durable manifest")
            }
            Self::ExistingTarget => {
                write!(formatter, "recovery target already exists and is preserved")
            }
            Self::UnsupportedStorageSchema(version) => {
                write!(formatter, "unsupported SQLite recovery schema {version}")
            }
            Self::GenerationConflict { expected } => {
                write!(formatter, "recovery generation conflict at {expected:?}")
            }
            Self::ManifestTooLarge { actual, maximum } => write!(
                formatter,
                "serialized recovery manifest is {actual} bytes, maximum is {maximum}"
            ),
            Self::ManifestIntegrity => write!(formatter, "recovery manifest integrity failed"),
            Self::Semantic(message) => write!(formatter, "semantic recovery failed: {message}"),
        }
    }
}

impl std::error::Error for SqlitePrototypeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteConfigurationReport {
    pub candidate: SqliteEvaluationCase,
    pub journal_mode: String,
    pub synchronous: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticHeader {
    pub schema: RecoverySchemaVersion,
    pub generation: RecoveryGeneration,
    pub topology_epoch: u64,
    pub payload: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlitePrototype {
    database_path: PathBuf,
    sqlite_program: PathBuf,
}

impl SqlitePrototype {
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
            sqlite_program: PathBuf::from("sqlite3"),
        }
    }

    pub fn with_program(
        database_path: impl Into<PathBuf>,
        sqlite_program: impl Into<PathBuf>,
    ) -> Self {
        Self {
            database_path: database_path.into(),
            sqlite_program: sqlite_program.into(),
        }
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn available(&self) -> bool {
        Command::new(&self.sqlite_program)
            .arg("-version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    pub fn initialize(&self) -> Result<(), SqlitePrototypeError> {
        match self.storage_schema_version()? {
            0 => {
                self.run(RECOVERY_SQLITE_SCHEMA_V1)?;
                self.run(RECOVERY_SQLITE_SCHEMA_V2)?;
                Ok(())
            }
            1 => self.run(RECOVERY_SQLITE_SCHEMA_V2).map(|_| ()),
            CURRENT_RECOVERY_SQLITE_SCHEMA => Ok(()),
            version => Err(SqlitePrototypeError::UnsupportedStorageSchema(version)),
        }
    }

    pub fn configure(
        &self,
        candidate: SqliteEvaluationCase,
    ) -> Result<SqliteConfigurationReport, SqlitePrototypeError> {
        let journal = match candidate.journal_mode {
            SqliteJournalMode::Wal => "WAL",
            SqliteJournalMode::Rollback => "DELETE",
        };
        let synchronous = match candidate.synchronous {
            SqliteSynchronousMode::Normal => "NORMAL",
            SqliteSynchronousMode::Full => "FULL",
            SqliteSynchronousMode::Extra => "EXTRA",
        };
        let output = self.run(&format!(
            "PRAGMA journal_mode={journal};\nPRAGMA synchronous={synchronous};\nPRAGMA wal_autocheckpoint={};\n",
            match candidate.checkpoint {
                dwv_recovery::SqliteCheckpointPolicy::Automatic => 1000,
                dwv_recovery::SqliteCheckpointPolicy::Explicit => 0,
            }
        ))?;
        Ok(SqliteConfigurationReport {
            candidate,
            journal_mode: output.trim().to_owned(),
            synchronous: synchronous.to_owned(),
        })
    }

    pub fn write_manifest(&self, manifest: &RecoveryManifest) -> Result<(), SqlitePrototypeError> {
        if self.storage_schema_version()? != CURRENT_RECOVERY_SQLITE_SCHEMA {
            return Err(SqlitePrototypeError::UnsupportedStorageSchema(
                self.storage_schema_version()?,
            ));
        }
        let payload = semantic_payload(manifest);
        let manifest_json = serialize_manifest(manifest)?;
        let manifest_digest = blake3::hash(manifest_json.as_bytes()).to_hex();
        let sql = format!(
            "PRAGMA foreign_keys=ON;\nPRAGMA synchronous=FULL;\nBEGIN IMMEDIATE;\nDELETE FROM recovery_state;\nINSERT INTO recovery_state(singleton, schema_version, generation, topology_epoch, manifest_payload, manifest_json, manifest_digest) VALUES (1, {}, {}, {}, '{}', '{}', '{}');\nCOMMIT;\n",
            manifest.schema.0,
            manifest.snapshot.generation.0,
            manifest.snapshot.topology_epoch.0,
            quote_sql(&payload),
            quote_sql(&manifest_json),
            manifest_digest,
        );
        self.run(&sql).map(|_| ())
    }
    /// Atomically replaces the current manifest only when its generation
    /// matches `expected`. The caller publishes its in-memory candidate only
    /// after this method succeeds.
    pub fn write_manifest_if_generation(
        &self,
        expected: RecoveryGeneration,
        manifest: &RecoveryManifest,
    ) -> Result<(), SqlitePrototypeError> {
        if self.storage_schema_version()? != CURRENT_RECOVERY_SQLITE_SCHEMA {
            return Err(SqlitePrototypeError::UnsupportedStorageSchema(
                self.storage_schema_version()?,
            ));
        }
        let next_generation = expected
            .0
            .checked_add(1)
            .ok_or(SqlitePrototypeError::ManifestIntegrity)?;
        if manifest.schema != dwv_recovery::CURRENT_RECOVERY_SCHEMA
            || manifest.snapshot.generation.0 != next_generation
        {
            return Err(SqlitePrototypeError::ManifestIntegrity);
        }
        let payload = semantic_payload(manifest);
        let manifest_json = serialize_manifest(manifest)?;
        let manifest_digest = blake3::hash(manifest_json.as_bytes()).to_hex();
        let output = self.run(&format!(
            "PRAGMA foreign_keys=ON;\nPRAGMA synchronous=FULL;\nBEGIN IMMEDIATE;\nUPDATE recovery_state SET schema_version={}, generation={}, topology_epoch={}, manifest_payload='{}', manifest_json='{}', manifest_digest='{}' WHERE singleton=1 AND generation={};\nSELECT changes();\nCOMMIT;\n",
            manifest.schema.0,
            manifest.snapshot.generation.0,
            manifest.snapshot.topology_epoch.0,
            quote_sql(&payload),
            quote_sql(&manifest_json),
            manifest_digest,
            expected.0,
        ))?;
        if output.trim() != "1" {
            return Err(SqlitePrototypeError::GenerationConflict { expected });
        }
        Ok(())
    }

    pub fn load_manifest(&self) -> Result<RecoveryManifest, SqlitePrototypeError> {
        if !self.integrity_check()? {
            return Err(SqlitePrototypeError::ManifestIntegrity);
        }
        let storage_schema = self.storage_schema_version()?;
        if storage_schema != CURRENT_RECOVERY_SQLITE_SCHEMA {
            return Err(SqlitePrototypeError::UnsupportedStorageSchema(
                storage_schema,
            ));
        }
        let output = self.run(
            "SELECT schema_version || '|' || generation || '|' || topology_epoch || '|' || hex(CAST(manifest_json AS BLOB)) || '|' || manifest_digest FROM recovery_state WHERE singleton=1;\n",
        )?;
        let line = output
            .lines()
            .next()
            .ok_or(SqlitePrototypeError::MissingState)?;
        let mut fields = line.splitn(5, '|');
        let schema = parse_u64(fields.next(), "schema_version")?;
        let generation = parse_u64(fields.next(), "generation")?;
        let topology_epoch = parse_u64(fields.next(), "topology_epoch")?;
        let manifest_hex = fields
            .next()
            .ok_or(SqlitePrototypeError::MissingCompleteManifest)?;
        let expected_digest = fields
            .next()
            .ok_or(SqlitePrototypeError::MissingCompleteManifest)?;
        if manifest_hex.is_empty() || expected_digest.is_empty() {
            return Err(SqlitePrototypeError::MissingCompleteManifest);
        }
        let manifest_bytes = decode_hex(manifest_hex)?;
        if manifest_bytes.len() > MAX_MANIFEST_JSON_BYTES {
            return Err(SqlitePrototypeError::ManifestTooLarge {
                actual: manifest_bytes.len(),
                maximum: MAX_MANIFEST_JSON_BYTES,
            });
        }
        let actual_digest = blake3::hash(&manifest_bytes).to_hex();
        if actual_digest.as_str() != expected_digest {
            return Err(SqlitePrototypeError::ManifestIntegrity);
        }
        let manifest: RecoveryManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| SqlitePrototypeError::InvalidOutput(error.to_string()))?;
        if u64::from(manifest.schema.0) != schema
            || manifest.snapshot.generation.0 != generation
            || manifest.snapshot.topology_epoch.0 != topology_epoch
        {
            return Err(SqlitePrototypeError::ManifestIntegrity);
        }
        validate_manifest_topologies(&manifest)?;
        let validated = MemoryRecoveryStore::from_manifest(manifest)
            .map_err(|error| SqlitePrototypeError::Semantic(error.to_string()))?;
        validated
            .export_manifest(
                validated
                    .load_assembly_snapshot()
                    .map_err(|error| SqlitePrototypeError::Semantic(error.to_string()))?
                    .generation,
            )
            .map_err(|error| SqlitePrototypeError::Semantic(error.to_string()))
    }

    pub fn recreate_from_metadata_loss(
        &self,
        authorization: MetadataLossAuthorization,
        topology: TopologySnapshot,
        source_health: RecoveryStoreHealth,
    ) -> Result<SemanticHeader, SqlitePrototypeError> {
        let manifest = authorization
            .fresh_manifest(topology, source_health)
            .map_err(|error| SqlitePrototypeError::Semantic(error.to_string()))?;
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.database_path)
        {
            Ok(file) => drop(file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(SqlitePrototypeError::ExistingTarget);
            }
            Err(error) => return Err(SqlitePrototypeError::Io(error.to_string())),
        }
        let result = (|| {
            self.initialize()?;
            self.write_manifest(&manifest)?;
            self.export_header()
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&self.database_path);
        }
        result
    }

    /// Returns the adapter's physical SQLite schema version. This is distinct
    /// from the semantic recovery manifest schema stored in each row.
    pub fn storage_schema_version(&self) -> Result<u64, SqlitePrototypeError> {
        parse_u64(
            self.run("PRAGMA user_version;\n")?.lines().next(),
            "user_version",
        )
    }

    pub fn export_header(&self) -> Result<SemanticHeader, SqlitePrototypeError> {
        let output = match self.run(
            "SELECT schema_version || '|' || generation || '|' || topology_epoch || '|' || manifest_payload FROM recovery_state WHERE singleton=1;\n",
        ) {
            Err(SqlitePrototypeError::CommandFailed { stderr, .. })
                if stderr.contains("no such table") =>
            {
                return Err(SqlitePrototypeError::MissingState);
            }
            result => result?,
        };
        let line = output
            .lines()
            .next()
            .ok_or(SqlitePrototypeError::MissingState)?;
        let mut fields = line.splitn(4, '|');
        let schema = parse_u64(fields.next(), "schema_version")?;
        let generation = parse_u64(fields.next(), "generation")?;
        let topology_epoch = parse_u64(fields.next(), "topology_epoch")?;
        let payload = fields
            .next()
            .ok_or_else(|| SqlitePrototypeError::InvalidOutput("missing payload".to_owned()))?;
        Ok(SemanticHeader {
            schema: RecoverySchemaVersion(u16::try_from(schema).map_err(|_| {
                SqlitePrototypeError::InvalidOutput("schema version exceeds u16".to_owned())
            })?),
            generation: RecoveryGeneration(generation),
            topology_epoch,
            payload: payload.to_owned(),
        })
    }

    pub fn integrity_check(&self) -> Result<bool, SqlitePrototypeError> {
        Ok(self.run("PRAGMA integrity_check;\n")?.trim() == "ok")
    }

    pub fn delete_state(&self) -> Result<(), SqlitePrototypeError> {
        self.run("DELETE FROM recovery_state;\n").map(|_| ())
    }

    pub fn corrupt_generation_for_fixture(&self) -> Result<(), SqlitePrototypeError> {
        self.run("UPDATE recovery_state SET generation='corrupt' WHERE singleton=1;\n")
            .map(|_| ())
    }

    fn run(&self, sql: &str) -> Result<String, SqlitePrototypeError> {
        if !self.available() {
            return Err(SqlitePrototypeError::SqliteUnavailable);
        }
        let mut child = Command::new(&self.sqlite_program)
            .arg("-batch")
            .arg("-noheader")
            .arg(&self.database_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| SqlitePrototypeError::Io(error.to_string()))?;
        {
            use std::io::Write;
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| SqlitePrototypeError::Io("sqlite stdin unavailable".to_owned()))?;
            stdin
                .write_all(sql.as_bytes())
                .map_err(|error| SqlitePrototypeError::Io(error.to_string()))?;
        }
        let output = child
            .wait_with_output()
            .map_err(|error| SqlitePrototypeError::Io(error.to_string()))?;
        if !output.status.success() {
            return Err(SqlitePrototypeError::CommandFailed {
                status: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}
#[derive(Debug)]
pub enum SqliteRecoveryStoreError {
    Io(String),
    LockHeld(PathBuf),
    ExistingTarget,
    MissingTarget,
    Prototype(SqlitePrototypeError),
    Semantic(String),
}

impl fmt::Display for SqliteRecoveryStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "SQLite recovery I/O failed: {message}"),
            Self::LockHeld(path) => {
                write!(
                    formatter,
                    "SQLite recovery store is already locked: {}",
                    path.display()
                )
            }
            Self::ExistingTarget => write!(formatter, "SQLite recovery target already exists"),
            Self::MissingTarget => write!(formatter, "SQLite recovery target is missing"),
            Self::Prototype(error) => error.fmt(formatter),
            Self::Semantic(message) => {
                write!(formatter, "SQLite recovery state is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for SqliteRecoveryStoreError {}

pub struct SqliteRecoveryStore {
    prototype: SqlitePrototype,
    memory: MemoryRecoveryStore,
    _lock: File,
}

impl SqliteRecoveryStore {
    pub fn create_new(
        database_path: impl Into<PathBuf>,
        manifest: RecoveryManifest,
    ) -> Result<Self, SqliteRecoveryStoreError> {
        let database_path = database_path.into();
        let (_lock_path, lock) = acquire_lock(&database_path)?;
        if database_path.exists() {
            return Err(SqliteRecoveryStoreError::ExistingTarget);
        }
        let prototype = SqlitePrototype::new(&database_path);
        let result = (|| {
            prototype
                .initialize()
                .map_err(SqliteRecoveryStoreError::Prototype)?;
            prototype
                .write_manifest(&manifest)
                .map_err(SqliteRecoveryStoreError::Prototype)?;
            let persisted = prototype
                .load_manifest()
                .map_err(SqliteRecoveryStoreError::Prototype)?;
            MemoryRecoveryStore::from_manifest(persisted)
                .map_err(|error| SqliteRecoveryStoreError::Semantic(error.to_string()))
        })();
        match result {
            Ok(memory) => Ok(Self {
                prototype,
                memory,
                _lock: lock,
            }),
            Err(error) => {
                let _ = std::fs::remove_file(&database_path);
                Err(error)
            }
        }
    }

    pub fn open(database_path: impl Into<PathBuf>) -> Result<Self, SqliteRecoveryStoreError> {
        let database_path = database_path.into();
        let (_lock_path, lock) = acquire_lock(&database_path)?;
        let result = (|| {
            if !database_path.is_file() {
                return Err(SqliteRecoveryStoreError::MissingTarget);
            }
            let prototype = SqlitePrototype::new(&database_path);
            prototype
                .initialize()
                .map_err(SqliteRecoveryStoreError::Prototype)?;
            let manifest = prototype
                .load_manifest()
                .map_err(SqliteRecoveryStoreError::Prototype)?;
            let memory = MemoryRecoveryStore::from_manifest(manifest)
                .map_err(|error| SqliteRecoveryStoreError::Semantic(error.to_string()))?;
            Ok((prototype, memory))
        })();
        match result {
            Ok((prototype, memory)) => Ok(Self {
                prototype,
                memory,
                _lock: lock,
            }),
            Err(error) => Err(error),
        }
    }

    pub fn database_path(&self) -> &Path {
        self.prototype.database_path()
    }

    pub fn snapshot(&self) -> Result<RecoverySnapshot, RecoveryError> {
        self.load_assembly_snapshot()
    }

    fn health_for(error: &SqlitePrototypeError) -> RecoveryStoreHealth {
        match error {
            SqlitePrototypeError::MissingState | SqlitePrototypeError::MissingCompleteManifest => {
                RecoveryStoreHealth::Missing
            }
            SqlitePrototypeError::GenerationConflict { .. } => RecoveryStoreHealth::Stale,
            _ => RecoveryStoreHealth::Corrupt,
        }
    }
}

impl RecoveryStateStore for SqliteRecoveryStore {
    fn load_assembly_snapshot(&self) -> Result<RecoverySnapshot, RecoveryError> {
        let health = self.verify_integrity();
        if health != RecoveryStoreHealth::Healthy {
            return Err(RecoveryError::Unhealthy(health));
        }
        self.memory.load_assembly_snapshot()
    }

    fn verify_integrity(&self) -> RecoveryStoreHealth {
        let Ok(valid) = self.prototype.integrity_check() else {
            return RecoveryStoreHealth::Corrupt;
        };
        if !valid {
            return RecoveryStoreHealth::Corrupt;
        }
        match self.prototype.load_manifest() {
            Ok(manifest)
                if manifest.snapshot.generation == self.memory.snapshot().generation
                    && manifest.snapshot.topology_epoch
                        == self.memory.snapshot().topology_epoch =>
            {
                RecoveryStoreHealth::Healthy
            }
            Ok(_) => RecoveryStoreHealth::Stale,
            Err(error) => Self::health_for(&error),
        }
    }

    fn begin_protocol_txn(
        &self,
        expected: RecoveryGeneration,
        topology_epoch: dwv_core::TopologyEpoch,
    ) -> RecoveryTxn {
        self.memory.begin_protocol_txn(expected, topology_epoch)
    }

    fn commit_durable(&mut self, txn: RecoveryTxn) -> Result<RecoveryGeneration, RecoveryError> {
        let health = self.verify_integrity();
        if health != RecoveryStoreHealth::Healthy {
            return Err(RecoveryError::Unhealthy(health));
        }
        let expected = self.memory.snapshot().generation;
        let mut candidate = self.memory.clone();
        let generation = candidate.commit_durable(txn)?;
        let manifest = candidate.export_manifest(generation)?;
        self.prototype
            .write_manifest_if_generation(expected, &manifest)
            .map_err(|error| RecoveryError::Unhealthy(Self::health_for(&error)))?;
        self.memory = candidate;
        Ok(generation)
    }

    fn export_manifest(
        &self,
        generation: RecoveryGeneration,
    ) -> Result<RecoveryManifest, RecoveryError> {
        self.memory.export_manifest(generation)
    }
}

impl Drop for SqliteRecoveryStore {
    fn drop(&mut self) {
        let _ = File::unlock(&self._lock);
    }
}

fn acquire_lock(database_path: &Path) -> Result<(PathBuf, File), SqliteRecoveryStoreError> {
    let mut lock_path = database_path.as_os_str().to_os_string();
    lock_path.push(".dwv-lock");
    let lock_path = PathBuf::from(lock_path);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| SqliteRecoveryStoreError::Io(error.to_string()))?;
    match file.try_lock() {
        Ok(()) => Ok((lock_path, file)),
        Err(std::fs::TryLockError::WouldBlock) => {
            Err(SqliteRecoveryStoreError::LockHeld(lock_path))
        }
        Err(error) => Err(SqliteRecoveryStoreError::Io(
            std::io::Error::from(error).to_string(),
        )),
    }
}

fn semantic_payload(manifest: &RecoveryManifest) -> String {
    let audit = manifest.snapshot.metadata_loss_audit;
    let mut lineage = String::new();
    if let Some(audit) = audit {
        use std::fmt::Write as _;
        for byte in audit.lineage_id.as_bytes() {
            write!(lineage, "{byte:02x}").expect("writing to a string cannot fail");
        }
    } else {
        lineage.push_str("none");
    }
    format!(
        "schema={};generation={};topology={};dirty={};integrity={};fences={};sessions={};maintenance={};metadata_loss_matrix={};metadata_loss_lineage={};metadata_loss_case={};metadata_loss_action={};metadata_loss_verification={};metadata_loss_baseline={};metadata_loss_source_health={};metadata_loss_topology={}",
        manifest.schema.0,
        manifest.snapshot.generation.0,
        manifest.snapshot.topology_epoch.0,
        manifest.snapshot.dirty_regions.len(),
        manifest.snapshot.integrity_records.len(),
        manifest.snapshot.fences.len(),
        if manifest.snapshot.writable_session.is_some() {
            1
        } else {
            0
        },
        manifest.snapshot.maintenance_checkpoints.len(),
        audit.map_or(0, |audit| audit.matrix_version),
        lineage,
        audit.map_or("none", |audit| audit.case.id()),
        audit.map_or("none", |audit| audit.action.id()),
        audit.map_or("none", |audit| audit.verification.id()),
        audit.map_or("none", |audit| audit.baseline.id()),
        audit.map_or("none".to_owned(), |audit| format!(
            "{:?}",
            audit.source_health
        )),
        audit.map_or(0, |audit| audit.topology_epoch.0),
    )
}

fn serialize_manifest(manifest: &RecoveryManifest) -> Result<String, SqlitePrototypeError> {
    let json = serde_json::to_string(manifest)
        .map_err(|error| SqlitePrototypeError::InvalidOutput(error.to_string()))?;
    if json.len() > MAX_MANIFEST_JSON_BYTES {
        return Err(SqlitePrototypeError::ManifestTooLarge {
            actual: json.len(),
            maximum: MAX_MANIFEST_JSON_BYTES,
        });
    }
    Ok(json)
}

fn decode_hex(value: &str) -> Result<Vec<u8>, SqlitePrototypeError> {
    if !value.len().is_multiple_of(2) {
        return Err(SqlitePrototypeError::InvalidOutput(
            "manifest hex has an odd length".to_owned(),
        ));
    }
    let decoded_len = value.len() / 2;
    if decoded_len > MAX_MANIFEST_JSON_BYTES {
        return Err(SqlitePrototypeError::ManifestTooLarge {
            actual: decoded_len,
            maximum: MAX_MANIFEST_JSON_BYTES,
        });
    }

    fn nibble(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = nibble(pair[0]).ok_or_else(|| {
                SqlitePrototypeError::InvalidOutput("manifest contains invalid hex".to_owned())
            })?;
            let low = nibble(pair[1]).ok_or_else(|| {
                SqlitePrototypeError::InvalidOutput("manifest contains invalid hex".to_owned())
            })?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn validate_manifest_topologies(manifest: &RecoveryManifest) -> Result<(), SqlitePrototypeError> {
    for topology in [
        manifest.snapshot.active_topology.as_ref(),
        manifest.snapshot.pending_topology.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        let profile = dwv_core::CodingProfile::new(
            topology.profile().data_slots(),
            topology.profile().parity_slots(),
        )
        .map_err(|error| SqlitePrototypeError::Semantic(format!("{error:?}")))?;
        let geometry = dwv_core::ProtectedGeometry::with_parity_length(
            topology.geometry().protected_length(),
            topology.geometry().logical_block_size(),
            topology.geometry().parity_length(),
        )
        .map_err(|error| SqlitePrototypeError::Semantic(format!("{error:?}")))?;
        let assignments = topology
            .assignments()
            .iter()
            .map(|assignment| {
                dwv_core::TopologyAssignment::new(
                    assignment.slot_id(),
                    assignment.role(),
                    assignment.coding_position(),
                    assignment.assignment_instance(),
                    assignment.assignment_generation(),
                )
                .with_evidence(assignment.evidence())
            })
            .collect();
        let core = dwv_core::TopologySnapshot::new(
            topology.array_id(),
            topology.topology_epoch(),
            profile,
            geometry,
            assignments,
        )
        .map_err(|error| SqlitePrototypeError::Semantic(error.to_string()))?;
        TopologySnapshot::from_core(
            core,
            topology
                .assignments()
                .iter()
                .map(|assignment| assignment.store_id())
                .collect(),
        )
        .map_err(|error| SqlitePrototypeError::Semantic(error.to_string()))?;
    }
    Ok(())
}

fn quote_sql(value: &str) -> String {
    value.replace('\'', "''")
}

fn parse_u64(value: Option<&str>, field: &str) -> Result<u64, SqlitePrototypeError> {
    let value =
        value.ok_or_else(|| SqlitePrototypeError::InvalidOutput(format!("missing {field}")))?;
    value
        .parse()
        .map_err(|_| SqlitePrototypeError::InvalidOutput(format!("invalid {field}: {value}")))
}

#[cfg(test)]
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(test)]
mod tests {
    use super::*;
    use dwv_core::{
        ArrayId, AssignmentGeneration, AssignmentInstanceId, CodingPosition, CodingProfile,
        MemberRole, ProtectedGeometry, SlotId, TopologyAssignment, TopologyEpoch,
        TopologySnapshot as CoreTopologySnapshot,
    };
    use dwv_recovery::{
        MemoryRecoveryStore, MetadataLossCase, MetadataLossPlan, MetadataLossVerification,
        RebuildId, RebuildState, RebuildTargetIdentity, RecoveryMutation, RecoveryStateStore,
        SqliteCheckpointPolicy, SqliteEvaluationMatrix, SqliteJournalMode, SqliteSynchronousMode,
    };
    use dwv_store::{CapabilityEvidenceId, FenceId, StoreFenceRef, StoreId, StoreWriteWatermark};

    fn temp_database() -> PathBuf {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "diskweave-recovery-{id}-{}.sqlite3",
            std::process::id()
        ))
    }

    fn topology(lineage: ArrayId) -> TopologySnapshot {
        let core = CoreTopologySnapshot::new(
            lineage,
            TopologyEpoch(1),
            CodingProfile::new(1, 1).unwrap(),
            ProtectedGeometry::new(4096, 512).unwrap(),
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
                    MemberRole::Parity,
                    CodingPosition(1),
                    AssignmentInstanceId([12; 16]),
                    AssignmentGeneration(1),
                ),
            ],
        )
        .unwrap();
        TopologySnapshot::from_core(core, vec![StoreId(1), StoreId(2)]).unwrap()
    }

    #[test]
    fn migration_modes_export_integrity_and_missing_state_are_executable() {
        let path = temp_database();
        let adapter = SqlitePrototype::new(&path);
        assert!(
            adapter.available(),
            "sqlite3 is required for this prototype test"
        );
        adapter.initialize().unwrap();
        assert_eq!(
            adapter.storage_schema_version().unwrap(),
            CURRENT_RECOVERY_SQLITE_SCHEMA
        );

        let recovery = MemoryRecoveryStore::new(TopologyEpoch(1));
        let manifest = recovery.export_manifest(RecoveryGeneration(0)).unwrap();
        for candidate in SqliteEvaluationMatrix::baseline().cases {
            let report = adapter.configure(candidate).unwrap();
            assert_eq!(report.candidate, candidate);
            adapter.write_manifest(&manifest).unwrap();
            assert!(adapter.integrity_check().unwrap());
            assert_eq!(adapter.load_manifest().unwrap(), manifest);
            let header = adapter.export_header().unwrap();
            assert_eq!(header.schema, manifest.schema);
            assert_eq!(header.generation, manifest.snapshot.generation);
            assert_eq!(header.topology_epoch, manifest.snapshot.topology_epoch.0);
            assert!(header.payload.contains("dirty=0"));
        }

        drop(recovery);
        let reopened = SqlitePrototype::new(&path);
        assert_eq!(
            reopened.export_header().unwrap().generation,
            RecoveryGeneration(0)
        );
        reopened.delete_state().unwrap();
        assert_eq!(
            reopened.export_header(),
            Err(SqlitePrototypeError::MissingState)
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn checkpointed_rebuild_survives_sqlite_close_reopen_and_resumes_exactly() {
        let path = temp_database();
        let source = topology(ArrayId([8; 16]));
        let replacement_store = StoreId(20);
        let rebuild_id = RebuildId::from_bytes([0x16; 16]);
        let mut recovery = MemoryRecoveryStore::new(TopologyEpoch(0));
        let mut install = recovery.begin_protocol_txn(RecoveryGeneration::ZERO, TopologyEpoch(0));
        install.push(RecoveryMutation::PrepareTopology {
            topology: source.clone(),
        });
        install.push(RecoveryMutation::CommitTopology {
            topology_epoch: TopologyEpoch(1),
        });
        let source_generation = recovery.commit_durable(install).unwrap();
        assert_eq!(source_generation, RecoveryGeneration(1));
        let manifest = recovery.export_manifest(source_generation).unwrap();
        let mut durable = SqliteRecoveryStore::create_new(&path, manifest).unwrap();
        let rebuild = RebuildState::prepare(
            rebuild_id,
            source,
            source_generation,
            SlotId([1; 16]),
            CodingPosition(0),
            AssignmentInstanceId([21; 16]),
            replacement_store,
            RebuildTargetIdentity::from_bytes([31; 16]),
        )
        .unwrap();
        let mut begin = durable.begin_protocol_txn(source_generation, TopologyEpoch(1));
        begin.begin_offline_rebuild(rebuild);
        assert_eq!(
            durable.commit_durable(begin).unwrap(),
            RecoveryGeneration(2)
        );

        let first_fence = StoreFenceRef {
            fence_id: FenceId(1),
            store_id: replacement_store,
            store_incarnation: dwv_store::StoreIncarnationId(0),
            topology_epoch: TopologyEpoch(1),
            through: StoreWriteWatermark(1),
            capability_evidence_id: CapabilityEvidenceId(7),
        };
        let first_receipt = durable.snapshot().unwrap().rebuilds[0]
            .durable_chunk_receipt(RecoveryGeneration(2), 1024, first_fence)
            .unwrap();
        let mut advance = durable.begin_protocol_txn(RecoveryGeneration(2), TopologyEpoch(1));
        advance.advance_offline_rebuild(first_receipt);
        assert_eq!(
            durable.commit_durable(advance).unwrap(),
            RecoveryGeneration(3)
        );
        drop(durable);
        drop(recovery);

        let mut restored = SqliteRecoveryStore::open(&path).unwrap();
        let restored_rebuild = &restored.snapshot().unwrap().rebuilds[0];
        assert_eq!(restored_rebuild.id(), rebuild_id);
        assert_eq!(restored_rebuild.cursor().offset(), 1024);
        assert_eq!(restored_rebuild.last_replacement_fence(), Some(first_fence));

        let second_fence = StoreFenceRef {
            fence_id: FenceId(2),
            through: StoreWriteWatermark(2),
            ..first_fence
        };
        let second_receipt = restored.snapshot().unwrap().rebuilds[0]
            .durable_chunk_receipt(RecoveryGeneration(3), 1536, second_fence)
            .unwrap();
        let mut resume = restored.begin_protocol_txn(RecoveryGeneration(3), TopologyEpoch(1));
        resume.advance_offline_rebuild(second_receipt);
        assert_eq!(
            restored.commit_durable(resume).unwrap(),
            RecoveryGeneration(4)
        );
        assert_eq!(
            restored.snapshot().unwrap().rebuilds[0].cursor().offset(),
            1536
        );
        drop(restored);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn durable_store_is_exclusive_and_reopens_generation_checked_state() {
        let path = temp_database();
        let initial = MemoryRecoveryStore::new(TopologyEpoch(0));
        let manifest = initial.export_manifest(RecoveryGeneration::ZERO).unwrap();
        let mut store = SqliteRecoveryStore::create_new(&path, manifest).unwrap();
        assert_eq!(store.verify_integrity(), RecoveryStoreHealth::Healthy);
        assert!(matches!(
            SqliteRecoveryStore::open(&path),
            Err(SqliteRecoveryStoreError::LockHeld(_))
        ));

        let mut checkpoint = store.begin_protocol_txn(RecoveryGeneration::ZERO, TopologyEpoch(0));
        checkpoint.push(RecoveryMutation::RecordMaintenanceCheckpoint {
            job: dwv_recovery::JobId(9),
            cursor: dwv_recovery::RecoveryCursor(4),
        });
        assert_eq!(
            store.commit_durable(checkpoint).unwrap(),
            RecoveryGeneration(1)
        );
        drop(store);

        let mut reopened = SqliteRecoveryStore::open(&path).unwrap();
        assert_eq!(
            reopened.snapshot().unwrap().generation,
            RecoveryGeneration(1)
        );
        assert_eq!(
            reopened.snapshot().unwrap().maintenance_checkpoints[0].cursor,
            dwv_recovery::RecoveryCursor(4)
        );

        let mut external = MemoryRecoveryStore::from_manifest(
            reopened.export_manifest(RecoveryGeneration(1)).unwrap(),
        )
        .unwrap();
        let mut external_txn = external.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(0));
        external_txn.push(RecoveryMutation::RecordMaintenanceCheckpoint {
            job: dwv_recovery::JobId(9),
            cursor: dwv_recovery::RecoveryCursor(8),
        });
        external.commit_durable(external_txn).unwrap();
        SqlitePrototype::new(&path)
            .write_manifest(&external.export_manifest(RecoveryGeneration(2)).unwrap())
            .unwrap();

        assert_eq!(reopened.verify_integrity(), RecoveryStoreHealth::Stale);
        let mut rejected = reopened.begin_protocol_txn(RecoveryGeneration(1), TopologyEpoch(0));
        rejected.push(RecoveryMutation::RecordMaintenanceCheckpoint {
            job: dwv_recovery::JobId(9),
            cursor: dwv_recovery::RecoveryCursor(12),
        });
        assert_eq!(
            reopened.commit_durable(rejected),
            Err(RecoveryError::Unhealthy(RecoveryStoreHealth::Stale))
        );
        assert_eq!(
            reopened
                .export_manifest(RecoveryGeneration(1))
                .unwrap()
                .snapshot
                .maintenance_checkpoints[0]
                .cursor,
            dwv_recovery::RecoveryCursor(4)
        );

        drop(reopened);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn corruption_is_visible_without_becoming_a_clean_state() {
        let path = temp_database();
        let adapter = SqlitePrototype::new(&path);
        assert!(adapter.available());
        adapter.initialize().unwrap();
        let recovery = MemoryRecoveryStore::new(TopologyEpoch(1));
        adapter
            .write_manifest(&recovery.export_manifest(RecoveryGeneration(0)).unwrap())
            .unwrap();
        adapter.corrupt_generation_for_fixture().unwrap();
        assert!(adapter.integrity_check().unwrap());
        assert!(matches!(
            adapter.export_header(),
            Err(SqlitePrototypeError::InvalidOutput(_))
        ));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn conservative_baseline_is_a_candidate_not_a_portable_semantic_choice() {
        let candidate = SqliteEvaluationCase {
            journal_mode: SqliteJournalMode::Rollback,
            synchronous: SqliteSynchronousMode::Full,
            checkpoint: SqliteCheckpointPolicy::Explicit,
            connections: 2,
        };
        assert!(
            SqliteEvaluationMatrix::baseline()
                .cases
                .contains(&candidate)
        );
    }

    #[test]
    fn deleting_recovery_database_does_not_delete_direct_data_fixture() {
        let path = temp_database();
        let direct_data_path = path.with_extension("direct-data");
        let adapter = SqlitePrototype::new(&path);
        assert!(adapter.available());
        adapter.initialize().unwrap();
        std::fs::write(&direct_data_path, b"direct media bytes").unwrap();

        drop(adapter);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            std::fs::read(&direct_data_path).unwrap(),
            b"direct media bytes"
        );
        assert_eq!(
            SqlitePrototype::new(&path).export_header(),
            Err(SqlitePrototypeError::MissingState)
        );

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(direct_data_path);
    }

    #[test]
    fn metadata_loss_recreates_separate_state_and_preserves_regular_file_payloads() {
        let path = temp_database();
        let data_path = path.with_extension("data.raw");
        let parity_path = path.with_extension("parity.raw");
        let data_bytes = vec![0x5a; 4096];
        let parity_bytes = vec![0xa5; 4096];
        std::fs::write(&data_path, &data_bytes).unwrap();
        std::fs::write(&parity_path, &parity_bytes).unwrap();

        let original = SqlitePrototype::new(&path);
        assert!(original.available());
        original.initialize().unwrap();
        let recovery = MemoryRecoveryStore::new(TopologyEpoch(1));
        original
            .write_manifest(&recovery.export_manifest(RecoveryGeneration::ZERO).unwrap())
            .unwrap();
        drop(original);
        std::fs::remove_file(&path).unwrap();

        let authorization =
            MetadataLossPlan::for_case(MetadataLossCase::AllDataSingleParityUncertified)
                .authorize(MetadataLossVerification::ExhaustiveMatches)
                .unwrap();
        let recreated = SqlitePrototype::new(&path);
        let header = recreated
            .recreate_from_metadata_loss(
                authorization,
                topology(ArrayId([4; 16])),
                RecoveryStoreHealth::Missing,
            )
            .unwrap();
        assert_eq!(header.schema, dwv_recovery::CURRENT_RECOVERY_SCHEMA);
        assert_eq!(
            recreated.storage_schema_version().unwrap(),
            CURRENT_RECOVERY_SQLITE_SCHEMA
        );
        assert_eq!(header.generation, RecoveryGeneration::ZERO);
        assert_eq!(header.topology_epoch, 1);
        assert!(
            header
                .payload
                .contains("metadata_loss_case=all-data-p-uncertified")
        );
        assert!(header.payload.contains("metadata_loss_matrix=2"));
        assert!(
            header
                .payload
                .contains("metadata_loss_lineage=04040404040404040404040404040404")
        );
        assert!(
            header
                .payload
                .contains("metadata_loss_action=exhaustive-verify-then-recreate")
        );
        assert!(
            header
                .payload
                .contains("metadata_loss_verification=exhaustive-matches")
        );
        assert!(
            header
                .payload
                .contains("metadata_loss_baseline=new-checksum-baseline-required")
        );
        assert_eq!(std::fs::read(&data_path).unwrap(), data_bytes);
        assert_eq!(std::fs::read(&parity_path).unwrap(), parity_bytes);
        assert_eq!(
            recreated.recreate_from_metadata_loss(
                authorization,
                topology(ArrayId([4; 16])),
                RecoveryStoreHealth::Missing,
            ),
            Err(SqlitePrototypeError::ExistingTarget)
        );

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(data_path);
        let _ = std::fs::remove_file(parity_path);
    }

    #[test]
    fn failed_recreation_releases_its_atomic_target_reservation() {
        let path = temp_database();
        let adapter = SqlitePrototype::with_program(&path, "definitely-not-a-sqlite-program");
        let authorization =
            MetadataLossPlan::for_case(MetadataLossCase::AllDataSingleParityUncertified)
                .authorize(MetadataLossVerification::ExhaustiveMatches)
                .unwrap();
        assert_eq!(
            adapter.recreate_from_metadata_loss(
                authorization,
                topology(ArrayId([6; 16])),
                RecoveryStoreHealth::Missing,
            ),
            Err(SqlitePrototypeError::SqliteUnavailable)
        );
        assert!(!path.exists());
    }

    #[test]
    fn recovery_lock_holder_process() {
        let Ok(path) = std::env::var("DWV_TEST_RECOVERY_LOCK_PATH") else {
            return;
        };
        let _claim = acquire_lock(Path::new(&path)).unwrap();
        std::fs::write(std::env::var("DWV_TEST_RECOVERY_READY").unwrap(), b"ready").unwrap();
        loop {
            std::thread::park();
        }
    }

    #[test]
    fn process_death_releases_recovery_claim() {
        let path = temp_database();
        let ready_path = path.with_extension("ready");
        let _ = std::fs::remove_file(&ready_path);
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "tests::recovery_lock_holder_process",
                "--exact",
                "--nocapture",
            ])
            .env("DWV_TEST_RECOVERY_LOCK_PATH", &path)
            .env("DWV_TEST_RECOVERY_READY", &ready_path)
            .stdout(std::process::Stdio::null())
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
            acquire_lock(&path),
            Err(SqliteRecoveryStoreError::LockHeld(_))
        ));
        child.kill().unwrap();
        child.wait().unwrap();
        let (marker, claim) = acquire_lock(&path).unwrap();
        drop(claim);
        std::fs::remove_file(marker).unwrap();
        std::fs::remove_file(ready_path).unwrap();
    }
}
