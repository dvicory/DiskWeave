use crate::lease::quarantine;
use rusqlite::{Connection, params};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const CONTROL_SCHEMA_VERSION: u32 = 1;
const MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS control_schema (singleton INTEGER PRIMARY KEY CHECK(singleton = 1), version INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS inventory (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS history (id INTEGER PRIMARY KEY AUTOINCREMENT, key TEXT NOT NULL, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS jobs (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
INSERT OR IGNORE INTO control_schema(singleton, version) VALUES (1, 1);
"#;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlEntry {
    pub category: String,
    pub key: String,
    pub value: String,
}

impl ControlEntry {
    pub fn new(
        category: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            category: category.into(),
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlExport {
    pub schema_version: u32,
    pub entries: Vec<ControlEntry>,
}

#[derive(Debug)]
pub enum ControlError {
    Io(String),
    Sqlite(String),
    InvalidOutput(String),
    IntegrityFailed,
    RowLimitExceeded(usize),
}

impl fmt::Display for ControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "control projection I/O failed: {message}"),
            Self::Sqlite(message) => {
                write!(formatter, "control projection sqlite failed: {message}")
            }
            Self::InvalidOutput(message) => {
                write!(formatter, "invalid control projection output: {message}")
            }
            Self::IntegrityFailed => write!(formatter, "control projection integrity check failed"),
            Self::RowLimitExceeded(limit) => {
                write!(formatter, "control export exceeded row limit {limit}")
            }
        }
    }
}

impl std::error::Error for ControlError {}

impl From<rusqlite::Error> for ControlError {
    /// A stored value that cannot be read as its expected type is malformed
    /// projection output, not an engine failure.
    fn from(error: rusqlite::Error) -> Self {
        match error {
            rusqlite::Error::FromSqlConversionFailure(..)
            | rusqlite::Error::InvalidColumnType(..)
            | rusqlite::Error::IntegralValueOutOfRange(..) => {
                Self::InvalidOutput(error.to_string())
            }
            error => Self::Sqlite(error.to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ControlProjection {
    database_path: PathBuf,
    row_limit: usize,
}

impl ControlProjection {
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
            row_limit: 4096,
        }
    }

    pub fn row_limit(mut self, row_limit: usize) -> Self {
        self.row_limit = row_limit;
        self
    }
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// Opens one short-lived connection for a single projection operation.
    /// There is deliberately no connection pool.
    ///
    /// rusqlite installs a 5-second busy handler on every connection. The
    /// projection disables it so lock contention fails immediately, as it did
    /// when each call ran the `sqlite3` shell.
    fn connect(&self) -> Result<Connection, ControlError> {
        if let Some(parent) = self.database_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|error| ControlError::Io(error.to_string()))?;
        }
        let connection = Connection::open(&self.database_path)?;
        connection.busy_timeout(Duration::ZERO)?;
        Ok(connection)
    }

    pub fn initialize(&self) -> Result<(), ControlError> {
        self.connect()?.execute_batch(MIGRATION)?;
        Ok(())
    }

    pub fn integrity_check(&self) -> Result<bool, ControlError> {
        let rows = self
            .connect()?
            .prepare("PRAGMA integrity_check")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows.as_slice() == ["ok"])
    }

    pub fn ensure_healthy(&self) -> Result<(), ControlError> {
        if !self.database_path.exists() {
            return self.initialize();
        }
        if !self.integrity_check()? {
            return Err(ControlError::IntegrityFailed);
        }
        self.schema_version().map(|_| ())
    }

    fn schema_version(&self) -> Result<u32, ControlError> {
        let version: i64 = match self.connect()?.query_row(
            "SELECT version FROM control_schema WHERE singleton=1",
            [],
            |row| row.get(0),
        ) {
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(ControlError::InvalidOutput(
                    "missing schema version".to_owned(),
                ));
            }
            result => result?,
        };
        u32::try_from(version)
            .map_err(|_| ControlError::InvalidOutput("invalid schema version".to_owned()))
    }

    pub fn add_inventory(&self, key: &str, value: &str) -> Result<(), ControlError> {
        self.initialize_if_missing()?;
        self.connect()?.execute(
            "INSERT OR REPLACE INTO inventory(key,value) VALUES (?1,?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn add_history(&self, key: &str, value: &str) -> Result<(), ControlError> {
        self.initialize_if_missing()?;
        self.connect()?.execute(
            "INSERT INTO history(key,value) VALUES (?1,?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn upsert_job(&self, key: &str, value: &str) -> Result<(), ControlError> {
        self.initialize_if_missing()?;
        self.connect()?.execute(
            "INSERT OR REPLACE INTO jobs(key,value) VALUES (?1,?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn export(&self) -> Result<ControlExport, ControlError> {
        self.ensure_healthy()?;
        let limit = i64::try_from(self.row_limit.saturating_add(1))
            .map_err(|_| ControlError::RowLimitExceeded(self.row_limit))?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT 'inventory', key, value FROM inventory UNION ALL SELECT 'history', key, value FROM history UNION ALL SELECT 'job', key, value FROM jobs ORDER BY 1, 2 LIMIT ?1",
        )?;
        let entries = statement
            .query_map(params![limit], |row| {
                Ok(ControlEntry::new(
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        if entries.len() > self.row_limit {
            return Err(ControlError::RowLimitExceeded(self.row_limit));
        }
        Ok(ControlExport {
            schema_version: self.schema_version()?,
            entries,
        })
    }

    pub fn rebuild(&self, entries: &[ControlEntry]) -> Result<(), ControlError> {
        if self.database_path.exists() && !self.integrity_check().unwrap_or(false) {
            quarantine(&self.database_path).map_err(|error| ControlError::Io(error.to_string()))?;
        }
        self.initialize()?;
        self.connect()?
            .execute_batch("DELETE FROM inventory; DELETE FROM history; DELETE FROM jobs;")?;
        for entry in entries {
            match entry.category.as_str() {
                "inventory" => self.add_inventory(&entry.key, &entry.value)?,
                "history" => self.add_history(&entry.key, &entry.value)?,
                "job" => self.upsert_job(&entry.key, &entry.value)?,
                _ => {
                    return Err(ControlError::InvalidOutput(format!(
                        "unknown category {}",
                        entry.category
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn rebuild_if_unhealthy(&self, entries: &[ControlEntry]) -> Result<(), ControlError> {
        if self.ensure_healthy().is_ok() {
            return Ok(());
        }
        self.rebuild(entries)
    }

    pub fn delete(&self) -> Result<(), ControlError> {
        if self.database_path.exists() {
            fs::remove_file(&self.database_path)
                .map_err(|error| ControlError::Io(error.to_string()))?;
        }
        Ok(())
    }

    fn initialize_if_missing(&self) -> Result<(), ControlError> {
        if self.database_path.exists() {
            self.ensure_healthy()
        } else {
            self.initialize()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    fn path() -> PathBuf {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("dwv-control-{}-{id}.sqlite3", std::process::id()))
    }

    #[test]
    fn projection_exports_and_rebuilds_without_payload_authority() {
        let database = path();
        let payload = database.with_extension("payload");
        let projection = ControlProjection::new(&database);
        let _ = projection.delete();
        fs::write(&payload, b"direct payload").unwrap();
        projection.initialize().unwrap();
        projection.add_inventory("store", "file-a").unwrap();
        projection.add_history("event", "opened").unwrap();
        projection.upsert_job("scrub", "pending").unwrap();
        assert_eq!(
            projection.export().unwrap().schema_version,
            CONTROL_SCHEMA_VERSION
        );
        let entries = projection.export().unwrap().entries;
        projection.delete().unwrap();
        assert_eq!(fs::read(&payload).unwrap(), b"direct payload");
        projection.rebuild(&entries).unwrap();
        assert_eq!(projection.export().unwrap().entries, entries);
        let _ = projection.delete();
        let _ = fs::remove_file(payload);
    }

    #[test]
    fn sql_values_are_exported_semantically_with_quotes_and_delimiters() {
        let database = path();
        let projection = ControlProjection::new(&database);
        let _ = projection.delete();
        projection.add_inventory("key|one", "value 'one'").unwrap();
        let export = projection.export().unwrap();
        assert!(
            export
                .entries
                .contains(&ControlEntry::new("inventory", "key|one", "value 'one'"))
        );
        let _ = projection.delete();
    }

    #[test]
    fn lock_contention_fails_immediately_instead_of_waiting() {
        let database = path();
        let projection = ControlProjection::new(&database);
        projection.initialize().unwrap();
        let holder = Connection::open(&database).unwrap();
        holder.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let started = std::time::Instant::now();
        assert!(matches!(
            projection.add_inventory("key", "value"),
            Err(ControlError::Sqlite(_))
        ));
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "contention must not wait on a busy handler"
        );
        drop(holder);
        projection.add_inventory("key", "value").unwrap();
        let _ = projection.delete();
    }
}
