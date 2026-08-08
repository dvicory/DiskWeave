use crate::lease::quarantine;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
    SqliteUnavailable,
    CommandFailed { status: Option<i32>, stderr: String },
    InvalidOutput(String),
    IntegrityFailed,
    RowLimitExceeded(usize),
}

impl fmt::Display for ControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "control projection I/O failed: {message}"),
            Self::SqliteUnavailable => write!(formatter, "sqlite3 executable is unavailable"),
            Self::CommandFailed { status, stderr } => {
                write!(formatter, "sqlite3 command failed ({status:?}): {stderr}")
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

#[derive(Clone, Debug)]
pub struct ControlProjection {
    database_path: PathBuf,
    sqlite_program: PathBuf,
    row_limit: usize,
}

impl ControlProjection {
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
            sqlite_program: PathBuf::from("sqlite3"),
            row_limit: 4096,
        }
    }

    pub fn with_program(
        database_path: impl Into<PathBuf>,
        sqlite_program: impl Into<PathBuf>,
    ) -> Self {
        Self {
            database_path: database_path.into(),
            sqlite_program: sqlite_program.into(),
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
    pub fn available(&self) -> bool {
        Command::new(&self.sqlite_program)
            .arg("-version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    pub fn initialize(&self) -> Result<(), ControlError> {
        self.run(MIGRATION).map(|_| ())
    }

    pub fn integrity_check(&self) -> Result<bool, ControlError> {
        Ok(self.run("PRAGMA integrity_check;")?.trim() == "ok")
    }

    pub fn ensure_healthy(&self) -> Result<(), ControlError> {
        if !self.database_path.exists() {
            return self.initialize();
        }
        if !self.integrity_check()? {
            return Err(ControlError::IntegrityFailed);
        }
        self.run("SELECT version FROM control_schema WHERE singleton=1;")
            .and_then(|output| parse_version(&output))
            .map(|_| ())
    }

    pub fn add_inventory(&self, key: &str, value: &str) -> Result<(), ControlError> {
        self.initialize_if_missing()?;
        self.run(&format!(
            "INSERT OR REPLACE INTO inventory(key,value) VALUES ('{}','{}');",
            quote(key),
            quote(value)
        ))
        .map(|_| ())
    }

    pub fn add_history(&self, key: &str, value: &str) -> Result<(), ControlError> {
        self.initialize_if_missing()?;
        self.run(&format!(
            "INSERT INTO history(key,value) VALUES ('{}','{}');",
            quote(key),
            quote(value)
        ))
        .map(|_| ())
    }

    pub fn upsert_job(&self, key: &str, value: &str) -> Result<(), ControlError> {
        self.initialize_if_missing()?;
        self.run(&format!(
            "INSERT OR REPLACE INTO jobs(key,value) VALUES ('{}','{}');",
            quote(key),
            quote(value)
        ))
        .map(|_| ())
    }

    pub fn export(&self) -> Result<ControlExport, ControlError> {
        self.ensure_healthy()?;
        let query = format!(
            "SELECT 'inventory',hex(key),hex(value) FROM inventory UNION ALL SELECT 'history',hex(key),hex(value) FROM history UNION ALL SELECT 'job',hex(key),hex(value) FROM jobs ORDER BY 1,2 LIMIT {};",
            self.row_limit.saturating_add(1)
        );
        let lines: Vec<_> = self.run(&query)?.lines().map(str::to_owned).collect();
        if lines.len() > self.row_limit {
            return Err(ControlError::RowLimitExceeded(self.row_limit));
        }
        let mut entries = Vec::with_capacity(lines.len());
        for line in lines {
            let mut fields = line.split('|');
            let category = fields
                .next()
                .ok_or_else(|| ControlError::InvalidOutput("missing category".to_owned()))?;
            let key = decode_hex(fields.next().unwrap_or_default())?;
            let value = decode_hex(fields.next().unwrap_or_default())?;
            entries.push(ControlEntry::new(category, key, value));
        }
        let version =
            parse_version(&self.run("SELECT version FROM control_schema WHERE singleton=1;")?)?;
        Ok(ControlExport {
            schema_version: version,
            entries,
        })
    }

    pub fn rebuild(&self, entries: &[ControlEntry]) -> Result<(), ControlError> {
        if self.database_path.exists() && !self.integrity_check().unwrap_or(false) {
            quarantine(&self.database_path).map_err(|error| ControlError::Io(error.to_string()))?;
        }
        self.initialize()?;
        self.run("DELETE FROM inventory; DELETE FROM history; DELETE FROM jobs;")?;
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

    fn run(&self, sql: &str) -> Result<String, ControlError> {
        if !self.available() {
            return Err(ControlError::SqliteUnavailable);
        }
        if let Some(parent) = self.database_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|error| ControlError::Io(error.to_string()))?;
        }
        let mut child = Command::new(&self.sqlite_program)
            .arg("-batch")
            .arg("-noheader")
            .arg(&self.database_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| ControlError::Io(error.to_string()))?;
        child
            .stdin
            .as_mut()
            .ok_or_else(|| ControlError::Io("sqlite stdin unavailable".to_owned()))?
            .write_all(sql.as_bytes())
            .map_err(|error| ControlError::Io(error.to_string()))?;
        let output = child
            .wait_with_output()
            .map_err(|error| ControlError::Io(error.to_string()))?;
        if !output.status.success() {
            return Err(ControlError::CommandFailed {
                status: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

fn parse_version(output: &str) -> Result<u32, ControlError> {
    output
        .trim()
        .parse()
        .map_err(|_| ControlError::InvalidOutput("invalid schema version".to_owned()))
}
fn quote(value: &str) -> String {
    value.replace('\'', "''")
}
fn decode_hex(value: &str) -> Result<String, ControlError> {
    if !value.len().is_multiple_of(2) {
        return Err(ControlError::InvalidOutput("odd hex field".to_owned()));
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ControlError::InvalidOutput("invalid hex field".to_owned()))?;
    String::from_utf8(bytes)
        .map_err(|_| ControlError::InvalidOutput("control values must be UTF-8".to_owned()))
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
        if !projection.available() {
            return;
        }
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
        if !projection.available() {
            return;
        }
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
}
