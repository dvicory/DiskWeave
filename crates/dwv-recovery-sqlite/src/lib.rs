//! Portable SQLite-backed recovery-state adapter.
//!
//! `SqliteRecoveryStore` implements the semantic recovery port with an
//! exclusive process lease, complete-manifest validation, generation-checked
//! persistence, and candidate-before-publish commits. `SqlitePrototype`
//! remains an evaluation helper for migrations, configuration candidates, and
//! metadata-loss experiments; neither type alone certifies physical
//! power-loss behavior.

use dwv_recovery::{
    CURRENT_RECOVERY_SCHEMA, MemoryRecoveryStore, RecoveryCommitObservation, RecoveryDisposition,
    RecoveryError, RecoveryFormatLayer, RecoveryGeneration, RecoveryInspection, RecoveryManifest,
    RecoveryMigrationPlan, RecoverySchemaVersion, RecoverySnapshot, RecoveryStateStore,
    RecoveryStoreHealth, RecoveryTxn, TopologySnapshot,
};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

pub const RECOVERY_SQLITE_SCHEMA_V1: &str = include_str!("../migrations/0001_recovery_state.sql");
pub const RECOVERY_SQLITE_SCHEMA_V2: &str =
    include_str!("../migrations/0002_complete_manifest.sql");
pub const CURRENT_RECOVERY_SQLITE_SCHEMA: u64 = 2;
pub const MAX_MANIFEST_JSON_BYTES: usize = 16 * 1024 * 1024;
/// The artifact cap leaves room for SQLite pages and duplicated bounded manifest data.
pub const MAX_RECOVERY_ARTIFACT_BYTES: u64 = (MAX_MANIFEST_JSON_BYTES as u64) * 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqliteJournalMode {
    Wal,
    Rollback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqliteSynchronousMode {
    Normal,
    Full,
    Extra,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqliteCheckpointPolicy {
    Automatic,
    Explicit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SqliteEvaluationCase {
    pub journal_mode: SqliteJournalMode,
    pub synchronous: SqliteSynchronousMode,
    pub checkpoint: SqliteCheckpointPolicy,
    pub connections: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteEvaluationMatrix {
    pub cases: Vec<SqliteEvaluationCase>,
}

impl SqliteEvaluationMatrix {
    pub fn baseline() -> Self {
        Self {
            cases: vec![
                SqliteEvaluationCase {
                    journal_mode: SqliteJournalMode::Wal,
                    synchronous: SqliteSynchronousMode::Normal,
                    checkpoint: SqliteCheckpointPolicy::Automatic,
                    connections: 1,
                },
                SqliteEvaluationCase {
                    journal_mode: SqliteJournalMode::Wal,
                    synchronous: SqliteSynchronousMode::Full,
                    checkpoint: SqliteCheckpointPolicy::Explicit,
                    connections: 1,
                },
                SqliteEvaluationCase {
                    journal_mode: SqliteJournalMode::Rollback,
                    synchronous: SqliteSynchronousMode::Full,
                    checkpoint: SqliteCheckpointPolicy::Explicit,
                    connections: 2,
                },
            ],
        }
    }

    pub fn fixtures(&self) -> Vec<SqliteEvaluationFixture> {
        let scenarios = [
            (
                "process-reset-after-durable-dirty-intent",
                SqliteResetBoundary::Process,
                SqliteFailurePoint::AfterDurableDirtyIntent,
                RecoveryCommitObservation::Durable,
                RecoveryStoreHealth::Healthy,
                RecoveryDisposition::Proceed,
            ),
            (
                "vm-reset-after-durable-checkpoint",
                SqliteResetBoundary::VirtualMachine,
                SqliteFailurePoint::AfterDurableCheckpoint,
                RecoveryCommitObservation::Durable,
                RecoveryStoreHealth::Healthy,
                RecoveryDisposition::Proceed,
            ),
            (
                "power-loss-during-journal-sync",
                SqliteResetBoundary::PowerLoss,
                SqliteFailurePoint::DuringJournalSync,
                RecoveryCommitObservation::Lost,
                RecoveryStoreHealth::Stale,
                RecoveryDisposition::ReconcileReadOnly,
            ),
            (
                "commit-rejected-before-home-mutation",
                SqliteResetBoundary::Process,
                SqliteFailurePoint::CommitRejected,
                RecoveryCommitObservation::Rejected,
                RecoveryStoreHealth::Healthy,
                RecoveryDisposition::ReconcileReadOnly,
            ),
            (
                "missing-database-after-reset",
                SqliteResetBoundary::VirtualMachine,
                SqliteFailurePoint::MissingDatabase,
                RecoveryCommitObservation::Corrupt,
                RecoveryStoreHealth::Missing,
                RecoveryDisposition::RebuildFromData,
            ),
            (
                "corrupt-main-state-after-reset",
                SqliteResetBoundary::VirtualMachine,
                SqliteFailurePoint::CorruptMainState,
                RecoveryCommitObservation::Corrupt,
                RecoveryStoreHealth::Corrupt,
                RecoveryDisposition::RebuildFromData,
            ),
            (
                "corrupt-journal-after-power-loss",
                SqliteResetBoundary::PowerLoss,
                SqliteFailurePoint::CorruptJournal,
                RecoveryCommitObservation::Corrupt,
                RecoveryStoreHealth::Corrupt,
                RecoveryDisposition::RebuildFromData,
            ),
        ];

        self.cases
            .iter()
            .enumerate()
            .flat_map(|(candidate_index, candidate)| {
                scenarios.iter().map(
                    move |(
                        scenario_id,
                        reset,
                        failure,
                        observation,
                        expected_health,
                        expected_disposition,
                    )| SqliteEvaluationFixture {
                        case_id: format!("candidate-{candidate_index}-{scenario_id}"),
                        candidate: *candidate,
                        reset: *reset,
                        failure: *failure,
                        observation: *observation,
                        expected_health: *expected_health,
                        expected_disposition: *expected_disposition,
                        permits_home_mutation: *expected_disposition
                            == RecoveryDisposition::Proceed,
                    },
                )
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqliteResetBoundary {
    Process,
    VirtualMachine,
    PowerLoss,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqliteFailurePoint {
    AfterDurableDirtyIntent,
    AfterDurableCheckpoint,
    DuringJournalSync,
    CommitRejected,
    MissingDatabase,
    CorruptMainState,
    CorruptJournal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteEvaluationFixture {
    pub case_id: String,
    pub candidate: SqliteEvaluationCase,
    pub reset: SqliteResetBoundary,
    pub failure: SqliteFailurePoint,
    pub observation: RecoveryCommitObservation,
    pub expected_health: RecoveryStoreHealth,
    pub expected_disposition: RecoveryDisposition,
    pub permits_home_mutation: bool,
}

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
                SqliteCheckpointPolicy::Automatic => 1000,
                SqliteCheckpointPolicy::Explicit => 0,
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
        self.load_manifest_with(false)
    }

    fn load_manifest_with(
        &self,
        read_only: bool,
    ) -> Result<RecoveryManifest, SqlitePrototypeError> {
        if !self.integrity_check_with(read_only)? {
            return Err(SqlitePrototypeError::ManifestIntegrity);
        }
        let storage_schema = self.storage_schema_version_with(read_only)?;
        if storage_schema != CURRENT_RECOVERY_SQLITE_SCHEMA {
            return Err(SqlitePrototypeError::UnsupportedStorageSchema(
                storage_schema,
            ));
        }
        let manifest_length_output = self.run_with(
            "SELECT length(CAST(manifest_json AS BLOB)) FROM recovery_state WHERE singleton=1;\n",
            read_only,
        )?;
        let manifest_length_line = manifest_length_output
            .lines()
            .next()
            .filter(|line| !line.is_empty())
            .ok_or(SqlitePrototypeError::MissingCompleteManifest)?;
        let manifest_length = parse_u64(Some(manifest_length_line), "manifest_json_length")?;
        if manifest_length == 0 {
            return Err(SqlitePrototypeError::MissingCompleteManifest);
        }
        let manifest_length = usize::try_from(manifest_length).unwrap_or(usize::MAX);
        if manifest_length > MAX_MANIFEST_JSON_BYTES {
            return Err(SqlitePrototypeError::ManifestTooLarge {
                actual: manifest_length,
                maximum: MAX_MANIFEST_JSON_BYTES,
            });
        }
        let output = self.run_with(
            "SELECT schema_version || '|' || generation || '|' || topology_epoch || '|' || hex(CAST(manifest_json AS BLOB)) || '|' || manifest_digest FROM recovery_state WHERE singleton=1;\n",
            read_only,
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

    /// Returns the adapter's physical SQLite schema version. This is distinct
    /// from the semantic recovery manifest schema stored in each row.
    pub fn storage_schema_version(&self) -> Result<u64, SqlitePrototypeError> {
        self.storage_schema_version_with(false)
    }

    fn storage_schema_version_with(&self, read_only: bool) -> Result<u64, SqlitePrototypeError> {
        parse_u64(
            self.run_with("PRAGMA user_version;\n", read_only)?
                .lines()
                .next(),
            "user_version",
        )
    }

    fn semantic_schema_version_with(&self, read_only: bool) -> Result<u64, SqlitePrototypeError> {
        let output = match self.run_with(
            "SELECT schema_version FROM recovery_state WHERE singleton=1;\n",
            read_only,
        ) {
            Err(SqlitePrototypeError::CommandFailed { stderr, .. })
                if stderr.contains("no such table") =>
            {
                return Err(SqlitePrototypeError::MissingState);
            }
            result => result?,
        };
        parse_u64(output.lines().next(), "schema_version")
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
        self.integrity_check_with(false)
    }

    fn integrity_check_with(&self, read_only: bool) -> Result<bool, SqlitePrototypeError> {
        Ok(self
            .run_with("PRAGMA integrity_check;\n", read_only)?
            .trim()
            == "ok")
    }

    pub fn delete_state(&self) -> Result<(), SqlitePrototypeError> {
        self.run("DELETE FROM recovery_state;\n").map(|_| ())
    }

    pub fn corrupt_generation_for_fixture(&self) -> Result<(), SqlitePrototypeError> {
        self.run("UPDATE recovery_state SET generation='corrupt' WHERE singleton=1;\n")
            .map(|_| ())
    }

    fn run(&self, sql: &str) -> Result<String, SqlitePrototypeError> {
        self.run_with(sql, false)
    }

    fn run_with(&self, sql: &str, read_only: bool) -> Result<String, SqlitePrototypeError> {
        if !self.available() {
            return Err(SqlitePrototypeError::SqliteUnavailable);
        }
        let mut command = Command::new(&self.sqlite_program);
        command.arg("-batch").arg("-noheader");
        if read_only {
            command.arg("-readonly");
        }
        let mut child = command
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
    ReconciliationRequired,
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
            Self::ReconciliationRequired => {
                formatter.write_str("SQLite recovery commit requires reconciliation")
            }
        }
    }
}

impl std::error::Error for SqliteRecoveryStoreError {}

/// Exclusive claim for publishing or replacing one recovery-state artifact.
///
/// The claim uses the same crash-releasing sidecar lock as writable stores but
/// does not create, initialize, migrate, or inspect the target artifact.
pub struct SqliteRecoveryClaim {
    lock: File,
}

impl Drop for SqliteRecoveryClaim {
    fn drop(&mut self) {
        let _ = File::unlock(&self.lock);
    }
}

pub struct SqliteRecoveryStore {
    prototype: SqlitePrototype,
    memory: MemoryRecoveryStore,
    _lock: File,
    #[cfg(test)]
    failure_point: Option<SqliteCommitFailurePoint>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SqliteCommitFailurePoint {
    AfterCommitIntent,
    AfterManifestWrite,
}

impl SqliteRecoveryStore {
    pub fn claim(
        database_path: impl Into<PathBuf>,
    ) -> Result<SqliteRecoveryClaim, SqliteRecoveryStoreError> {
        let database_path = database_path.into();
        let (_lock_path, lock) = acquire_lock(&database_path)?;
        Ok(SqliteRecoveryClaim { lock })
    }

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
                #[cfg(test)]
                failure_point: None,
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
            reconcile_commit_intent_before_mutation(&prototype)?;
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
                #[cfg(test)]
                failure_point: None,
            }),
            Err(error) => Err(error),
        }
    }

    /// Observes recovery state without taking a writer lease or running DDL,
    /// initialization, migration, or commit-intent cleanup.
    /// dwv:req req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations
    /// dwv:req req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout
    pub fn inspect(database_path: impl Into<PathBuf>) -> RecoveryInspection {
        let database_path = database_path.into();
        if !database_path.exists() {
            return RecoveryInspection::Absent;
        }
        if !database_path.is_file() {
            return RecoveryInspection::CorruptOrUnreadable {
                storage_version: None,
                semantic_version: None,
            };
        }
        let artifact_size = match std::fs::metadata(&database_path) {
            Ok(metadata) => metadata.len(),
            Err(_) => {
                return RecoveryInspection::CorruptOrUnreadable {
                    storage_version: None,
                    semantic_version: None,
                };
            }
        };
        if artifact_size > MAX_RECOVERY_ARTIFACT_BYTES {
            return RecoveryInspection::CorruptOrUnreadable {
                storage_version: None,
                semantic_version: None,
            };
        }
        if commit_intent_path(&database_path).exists() {
            return RecoveryInspection::ReconciliationRequired;
        }
        let prototype = SqlitePrototype::new(database_path);
        let storage_schema = match prototype.storage_schema_version_with(true) {
            Ok(version) => version,
            Err(SqlitePrototypeError::SqliteUnavailable) => {
                return RecoveryInspection::Unsupported {
                    layer: RecoveryFormatLayer::Storage,
                    version: None,
                    storage_version: None,
                };
            }
            Err(_) => {
                return RecoveryInspection::CorruptOrUnreadable {
                    storage_version: None,
                    semantic_version: None,
                };
            }
        };
        if storage_schema != CURRENT_RECOVERY_SQLITE_SCHEMA {
            return if storage_schema < CURRENT_RECOVERY_SQLITE_SCHEMA {
                RecoveryInspection::MigrationRequired {
                    layer: RecoveryFormatLayer::Storage,
                    from: storage_schema,
                    to: CURRENT_RECOVERY_SQLITE_SCHEMA,
                    storage_version: None,
                }
            } else {
                RecoveryInspection::Unsupported {
                    layer: RecoveryFormatLayer::Storage,
                    version: Some(storage_schema),
                    storage_version: None,
                }
            };
        }

        let semantic_schema = match prototype.semantic_schema_version_with(true) {
            Ok(version) => version,
            Err(_) => {
                return RecoveryInspection::CorruptOrUnreadable {
                    storage_version: Some(storage_schema),
                    semantic_version: None,
                };
            }
        };
        if semantic_schema != u64::from(CURRENT_RECOVERY_SCHEMA.0) {
            let from = u16::try_from(semantic_schema)
                .ok()
                .map(RecoverySchemaVersion);
            return if from.is_some_and(|from| {
                RecoveryMigrationPlan::plan(from, CURRENT_RECOVERY_SCHEMA).is_ok()
            }) {
                RecoveryInspection::MigrationRequired {
                    layer: RecoveryFormatLayer::Semantic,
                    from: semantic_schema,
                    to: u64::from(CURRENT_RECOVERY_SCHEMA.0),
                    storage_version: Some(storage_schema),
                }
            } else {
                RecoveryInspection::Unsupported {
                    layer: RecoveryFormatLayer::Semantic,
                    version: Some(semantic_schema),
                    storage_version: Some(storage_schema),
                }
            };
        }
        match prototype.load_manifest_with(true) {
            Ok(manifest) => RecoveryInspection::Supported(Box::new(manifest)),
            Err(SqlitePrototypeError::SqliteUnavailable) => RecoveryInspection::Unsupported {
                layer: RecoveryFormatLayer::Storage,
                version: None,
                storage_version: None,
            },
            Err(_) => RecoveryInspection::CorruptOrUnreadable {
                storage_version: Some(storage_schema),
                semantic_version: Some(semantic_schema),
            },
        }
    }

    pub fn database_path(&self) -> &Path {
        self.prototype.database_path()
    }

    #[cfg(test)]
    fn fail_at(&mut self, failure_point: SqliteCommitFailurePoint) {
        self.failure_point = Some(failure_point);
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct CommitIntent {
    prior_generation: RecoveryGeneration,
    prior_digest: String,
    proposed_generation: RecoveryGeneration,
    proposed_digest: String,
}

impl CommitIntent {
    fn encode(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}\n",
            self.prior_generation.0,
            self.prior_digest,
            self.proposed_generation.0,
            self.proposed_digest
        )
    }

    fn decode(bytes: &[u8]) -> Result<Self, SqliteRecoveryStoreError> {
        let value = std::str::from_utf8(bytes)
            .map_err(|_| SqliteRecoveryStoreError::ReconciliationRequired)?;
        let mut lines = value.lines();
        let prior_generation = lines
            .next()
            .and_then(|value| value.parse().ok())
            .map(RecoveryGeneration)
            .ok_or(SqliteRecoveryStoreError::ReconciliationRequired)?;
        let prior_digest = lines
            .next()
            .filter(|value| value.len() == 64)
            .map(str::to_owned)
            .ok_or(SqliteRecoveryStoreError::ReconciliationRequired)?;
        let proposed_generation = lines
            .next()
            .and_then(|value| value.parse().ok())
            .map(RecoveryGeneration)
            .ok_or(SqliteRecoveryStoreError::ReconciliationRequired)?;
        let proposed_digest = lines
            .next()
            .filter(|value| value.len() == 64)
            .map(str::to_owned)
            .ok_or(SqliteRecoveryStoreError::ReconciliationRequired)?;
        if lines.next().is_some() {
            return Err(SqliteRecoveryStoreError::ReconciliationRequired);
        }
        Ok(Self {
            prior_generation,
            prior_digest,
            proposed_generation,
            proposed_digest,
        })
    }
}

fn commit_intent_path(database_path: &Path) -> PathBuf {
    let mut path = database_path.as_os_str().to_owned();
    path.push(".commit-intent");
    PathBuf::from(path)
}

fn manifest_digest(manifest: &RecoveryManifest) -> Result<String, SqliteRecoveryStoreError> {
    let manifest = serialize_manifest(manifest).map_err(SqliteRecoveryStoreError::Prototype)?;
    Ok(blake3::hash(manifest.as_bytes()).to_hex().to_string())
}

fn sync_parent(path: &Path) -> Result<(), SqliteRecoveryStoreError> {
    File::open(path.parent().unwrap_or_else(|| Path::new(".")))
        .and_then(|directory| directory.sync_all())
        .map_err(|error| SqliteRecoveryStoreError::Io(error.to_string()))
}

fn write_commit_intent(
    database_path: &Path,
    intent: &CommitIntent,
) -> Result<(), SqliteRecoveryStoreError> {
    let path = commit_intent_path(database_path);
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| SqliteRecoveryStoreError::Io(error.to_string()))?;
    file.write_all(intent.encode().as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|error| SqliteRecoveryStoreError::Io(error.to_string()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| SqliteRecoveryStoreError::Io(error.to_string()))?;
    sync_parent(&path)
}

fn remove_commit_intent(database_path: &Path) -> Result<(), SqliteRecoveryStoreError> {
    let path = commit_intent_path(database_path);
    std::fs::remove_file(&path).map_err(|error| SqliteRecoveryStoreError::Io(error.to_string()))?;
    sync_parent(&path)
}

fn validate_commit_intent(
    prototype: &SqlitePrototype,
    read_only: bool,
) -> Result<(), SqliteRecoveryStoreError> {
    let path = commit_intent_path(prototype.database_path());
    if !path.exists() {
        return Ok(());
    }
    let intent = CommitIntent::decode(
        &std::fs::read(&path).map_err(|error| SqliteRecoveryStoreError::Io(error.to_string()))?,
    )?;
    let manifest = prototype
        .load_manifest_with(read_only)
        .map_err(|_| SqliteRecoveryStoreError::ReconciliationRequired)?;
    let observed = (manifest.snapshot.generation, manifest_digest(&manifest)?);
    if observed != (intent.prior_generation, intent.prior_digest)
        && observed != (intent.proposed_generation, intent.proposed_digest)
    {
        return Err(SqliteRecoveryStoreError::ReconciliationRequired);
    }
    Ok(())
}

/// Reconciliation must interpret current durable state before initialization can run DDL.
/// dwv:req req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations
fn reconcile_commit_intent_before_mutation(
    prototype: &SqlitePrototype,
) -> Result<(), SqliteRecoveryStoreError> {
    if !commit_intent_path(prototype.database_path()).exists() {
        return Ok(());
    }
    let storage_schema = prototype
        .storage_schema_version_with(true)
        .map_err(|_| SqliteRecoveryStoreError::ReconciliationRequired)?;
    let semantic_schema = prototype
        .semantic_schema_version_with(true)
        .map_err(|_| SqliteRecoveryStoreError::ReconciliationRequired)?;
    if storage_schema != CURRENT_RECOVERY_SQLITE_SCHEMA
        || semantic_schema != u64::from(CURRENT_RECOVERY_SCHEMA.0)
    {
        return Err(SqliteRecoveryStoreError::ReconciliationRequired);
    }
    reconcile_commit_intent(prototype)
}

fn reconcile_commit_intent(prototype: &SqlitePrototype) -> Result<(), SqliteRecoveryStoreError> {
    if !commit_intent_path(prototype.database_path()).exists() {
        return Ok(());
    }
    validate_commit_intent(prototype, true)?;
    remove_commit_intent(prototype.database_path())
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
        if commit_intent_path(self.prototype.database_path()).exists() {
            return RecoveryStoreHealth::Corrupt;
        }
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
        let prior = self.memory.export_manifest(expected)?;
        let mut candidate = self.memory.clone();
        let generation = candidate.commit_durable(txn)?;
        let proposed = candidate.export_manifest(generation)?;
        let intent = CommitIntent {
            prior_generation: expected,
            prior_digest: manifest_digest(&prior).map_err(|_| {
                RecoveryError::CommitNotDurable(RecoveryCommitObservation::Rejected)
            })?,
            proposed_generation: generation,
            proposed_digest: manifest_digest(&proposed).map_err(|_| {
                RecoveryError::CommitNotDurable(RecoveryCommitObservation::Rejected)
            })?,
        };
        write_commit_intent(self.prototype.database_path(), &intent)
            .map_err(|_| RecoveryError::CommitNotDurable(RecoveryCommitObservation::Rejected))?;
        #[cfg(test)]
        if self.failure_point == Some(SqliteCommitFailurePoint::AfterCommitIntent) {
            self.failure_point = None;
            return Err(RecoveryError::CommitNotDurable(
                RecoveryCommitObservation::Lost,
            ));
        }
        self.prototype
            .write_manifest_if_generation(expected, &proposed)
            .map_err(|_| RecoveryError::CommitNotDurable(RecoveryCommitObservation::Lost))?;
        #[cfg(test)]
        if self.failure_point == Some(SqliteCommitFailurePoint::AfterManifestWrite) {
            self.failure_point = None;
            return Err(RecoveryError::CommitNotDurable(
                RecoveryCommitObservation::Lost,
            ));
        }
        remove_commit_intent(self.prototype.database_path())
            .map_err(|_| RecoveryError::CommitNotDurable(RecoveryCommitObservation::Lost))?;
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
        MemoryRecoveryStore, RebuildId, RebuildState, RebuildTargetIdentity, RecoveryMutation,
        RecoveryStateStore,
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
    fn publication_claim_excludes_writable_store_without_creating_the_target() {
        let path = temp_database();
        let claim = SqliteRecoveryStore::claim(&path).unwrap();
        assert!(!path.exists());
        let initial = MemoryRecoveryStore::new(TopologyEpoch(0));
        let manifest = initial.export_manifest(RecoveryGeneration::ZERO).unwrap();
        assert!(matches!(
            SqliteRecoveryStore::create_new(&path, manifest.clone()),
            Err(SqliteRecoveryStoreError::LockHeld(_))
        ));
        drop(claim);
        let store = SqliteRecoveryStore::create_new(&path, manifest).unwrap();
        drop(store);
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
    fn uncertain_commit_reopens_to_exactly_prior_or_proposed_state() {
        for (failure_point, expected_generation) in [
            (
                SqliteCommitFailurePoint::AfterCommitIntent,
                RecoveryGeneration::ZERO,
            ),
            (
                SqliteCommitFailurePoint::AfterManifestWrite,
                RecoveryGeneration(1),
            ),
        ] {
            let path = temp_database();
            let initial = MemoryRecoveryStore::new(TopologyEpoch(0));
            let manifest = initial.export_manifest(RecoveryGeneration::ZERO).unwrap();
            let mut store = SqliteRecoveryStore::create_new(&path, manifest).unwrap();
            let mut checkpoint =
                store.begin_protocol_txn(RecoveryGeneration::ZERO, TopologyEpoch(0));
            checkpoint.push(RecoveryMutation::RecordMaintenanceCheckpoint {
                job: dwv_recovery::JobId(9),
                cursor: dwv_recovery::RecoveryCursor(4),
            });
            store.fail_at(failure_point);
            assert_eq!(
                store.commit_durable(checkpoint),
                Err(RecoveryError::CommitNotDurable(
                    RecoveryCommitObservation::Lost
                ))
            );
            assert_eq!(store.verify_integrity(), RecoveryStoreHealth::Corrupt);
            drop(store);
            assert_eq!(
                SqliteRecoveryStore::inspect(&path),
                RecoveryInspection::ReconciliationRequired
            );
            assert!(commit_intent_path(&path).exists());

            let reopened = SqliteRecoveryStore::open(&path).unwrap();
            let snapshot = reopened.snapshot().unwrap();
            assert_eq!(snapshot.generation, expected_generation);
            assert_eq!(
                snapshot.maintenance_checkpoints.len(),
                usize::from(expected_generation == RecoveryGeneration(1))
            );
            assert!(!commit_intent_path(&path).exists());
            drop(reopened);
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn uncertain_commit_refuses_an_unrecognized_durable_state() {
        let path = temp_database();
        let initial = MemoryRecoveryStore::new(TopologyEpoch(0));
        let manifest = initial.export_manifest(RecoveryGeneration::ZERO).unwrap();
        let mut store = SqliteRecoveryStore::create_new(&path, manifest.clone()).unwrap();
        let mut proposed = store.begin_protocol_txn(RecoveryGeneration::ZERO, TopologyEpoch(0));
        proposed.push(RecoveryMutation::RecordMaintenanceCheckpoint {
            job: dwv_recovery::JobId(9),
            cursor: dwv_recovery::RecoveryCursor(4),
        });
        store.fail_at(SqliteCommitFailurePoint::AfterCommitIntent);
        assert!(matches!(
            store.commit_durable(proposed),
            Err(RecoveryError::CommitNotDurable(
                RecoveryCommitObservation::Lost
            ))
        ));

        let mut conflicting = MemoryRecoveryStore::from_manifest(manifest).unwrap();
        let mut transaction =
            conflicting.begin_protocol_txn(RecoveryGeneration::ZERO, TopologyEpoch(0));
        transaction.push(RecoveryMutation::RecordMaintenanceCheckpoint {
            job: dwv_recovery::JobId(9),
            cursor: dwv_recovery::RecoveryCursor(8),
        });
        conflicting.commit_durable(transaction).unwrap();
        store
            .prototype
            .write_manifest(&conflicting.export_manifest(RecoveryGeneration(1)).unwrap())
            .unwrap();
        drop(store);
        assert_eq!(
            SqliteRecoveryStore::inspect(&path),
            RecoveryInspection::ReconciliationRequired
        );

        assert!(matches!(
            SqliteRecoveryStore::open(&path),
            Err(SqliteRecoveryStoreError::ReconciliationRequired)
        ));
        assert!(commit_intent_path(&path).exists());
        let _ = std::fs::remove_file(commit_intent_path(&path));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn inspection_is_read_only_and_classifies_supported_missing_and_migration() {
        let path = temp_database();
        assert_eq!(
            SqliteRecoveryStore::inspect(&path),
            RecoveryInspection::Absent
        );
        assert!(!path.exists());

        let adapter = SqlitePrototype::new(&path);
        assert!(adapter.available());
        adapter.initialize().unwrap();
        let recovery = MemoryRecoveryStore::new(TopologyEpoch(1));
        let manifest = recovery.export_manifest(RecoveryGeneration::ZERO).unwrap();
        adapter.write_manifest(&manifest).unwrap();
        let before_bytes = std::fs::read(&path).unwrap();
        let before_metadata = std::fs::metadata(&path).unwrap();

        assert_eq!(
            SqliteRecoveryStore::inspect(&path),
            RecoveryInspection::Supported(Box::new(manifest))
        );
        let after_metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before_bytes);
        assert_eq!(after_metadata.len(), before_metadata.len());
        assert_eq!(
            after_metadata.permissions().readonly(),
            before_metadata.permissions().readonly()
        );
        assert_eq!(
            after_metadata.modified().unwrap(),
            before_metadata.modified().unwrap()
        );

        adapter
            .run("UPDATE recovery_state SET schema_version=3 WHERE singleton=1;\n")
            .unwrap();
        assert_eq!(
            SqliteRecoveryStore::inspect(&path),
            RecoveryInspection::MigrationRequired {
                layer: RecoveryFormatLayer::Semantic,
                from: 3,
                to: u64::from(CURRENT_RECOVERY_SCHEMA.0),
                storage_version: Some(CURRENT_RECOVERY_SQLITE_SCHEMA),
            }
        );
        std::fs::write(commit_intent_path(&path), b"uninterpreted intent").unwrap();
        let before_open = std::fs::read(&path).unwrap();
        assert_eq!(
            SqliteRecoveryStore::inspect(&path),
            RecoveryInspection::ReconciliationRequired
        );
        assert!(matches!(
            SqliteRecoveryStore::open(&path),
            Err(SqliteRecoveryStoreError::ReconciliationRequired)
        ));
        assert_eq!(std::fs::read(&path).unwrap(), before_open);
        assert!(commit_intent_path(&path).exists());
        let _ = std::fs::remove_file(commit_intent_path(&path));

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

    #[test]
    fn evaluation_fixtures_cover_candidates_and_conservative_failures() {
        let matrix = SqliteEvaluationMatrix::baseline();
        let fixtures = matrix.fixtures();
        assert_eq!(fixtures.len(), matrix.cases.len() * 7);
        assert!(fixtures.iter().any(|fixture| {
            fixture.failure == SqliteFailurePoint::MissingDatabase
                && !fixture.permits_home_mutation
                && fixture.expected_disposition == RecoveryDisposition::RebuildFromData
        }));
        assert!(fixtures.iter().any(|fixture| {
            fixture.failure == SqliteFailurePoint::CommitRejected
                && fixture.expected_disposition == RecoveryDisposition::ReconcileReadOnly
                && !fixture.permits_home_mutation
        }));
    }
}
