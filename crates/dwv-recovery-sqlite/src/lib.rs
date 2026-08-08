//! Evaluation-only SQLite adapter for the recovery-state semantic port.
//!
//! This crate intentionally invokes the host `sqlite3` executable instead of
//! selecting a Rust SQLite binding for the portable reference. It provides a
//! small executable prototype for migrations, candidate mode evaluation,
//! semantic-header persistence, export, and integrity handling. Production
//! bindings and durability claims remain separate decisions.

use dwv_recovery::{
    MetadataLossAuthorization, RecoveryGeneration, RecoveryManifest, RecoverySchemaVersion,
    RecoveryStoreHealth, SqliteEvaluationCase, SqliteJournalMode, SqliteSynchronousMode,
    TopologySnapshot,
};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

pub const RECOVERY_SQLITE_SCHEMA_V1: &str = include_str!("../migrations/0001_recovery_state.sql");

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SqlitePrototypeError {
    Io(String),
    SqliteUnavailable,
    CommandFailed { status: Option<i32>, stderr: String },
    InvalidOutput(String),
    MissingState,
    ExistingTarget,
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
            Self::ExistingTarget => {
                write!(formatter, "recovery target already exists and is preserved")
            }
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
        self.run(RECOVERY_SQLITE_SCHEMA_V1).map(|_| ())
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
        let payload = semantic_payload(manifest);
        let sql = format!(
            "BEGIN IMMEDIATE;\nDELETE FROM recovery_state;\nINSERT INTO recovery_state(singleton, schema_version, generation, topology_epoch, manifest_payload) VALUES (1, {}, {}, {}, '{}');\nCOMMIT;\n",
            manifest.schema.0,
            manifest.snapshot.generation.0,
            manifest.snapshot.topology_epoch.0,
            quote_sql(&payload),
        );
        self.run(&sql).map(|_| ())
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
        RecoveryStateStore, SqliteCheckpointPolicy, SqliteEvaluationMatrix, SqliteJournalMode,
        SqliteSynchronousMode,
    };
    use dwv_store::StoreId;

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
        assert_eq!(adapter.storage_schema_version().unwrap(), 1);

        let recovery = MemoryRecoveryStore::new(TopologyEpoch(1));
        let manifest = recovery.export_manifest(RecoveryGeneration(0)).unwrap();
        for candidate in SqliteEvaluationMatrix::baseline().cases {
            let report = adapter.configure(candidate).unwrap();
            assert_eq!(report.candidate, candidate);
            adapter.write_manifest(&manifest).unwrap();
            assert!(adapter.integrity_check().unwrap());
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
        assert_eq!(recreated.storage_schema_version().unwrap(), 1);
        assert_eq!(header.generation, RecoveryGeneration::ZERO);
        assert_eq!(header.topology_epoch, 1);
        assert!(
            header
                .payload
                .contains("metadata_loss_case=all-data-p-uncertified")
        );
        assert!(header.payload.contains("metadata_loss_matrix=1"));
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
}
