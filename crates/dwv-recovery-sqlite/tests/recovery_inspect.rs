use dwv_core::TopologyEpoch;
use dwv_recovery::{
    CURRENT_RECOVERY_SCHEMA, MemoryRecoveryStore, RecoveryGeneration, RecoveryManifest,
    RecoveryStateStore,
};
use dwv_recovery_sqlite::{
    CURRENT_RECOVERY_SQLITE_SCHEMA, MAX_MANIFEST_JSON_BYTES, MAX_RECOVERY_ARTIFACT_BYTES,
    SqlitePrototype, SqliteRecoveryStore,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const RESULT_OUTPUT_BOUND: usize = MAX_MANIFEST_JSON_BYTES + 64 * 1024;
static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
    fn new() -> Self {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("dwv-recovery-inspect-{}-{id}", std::process::id()));
        fs::create_dir(&path).expect("fixture directory should be unique");
        Self(path)
    }

    fn artifact(&self) -> PathBuf {
        self.0.join("recovery.sqlite3")
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum EntrySnapshot {
    File(Vec<u8>),
    Directory,
    Symlink,
    Other,
}

fn directory_snapshot(root: &Path) -> Vec<(String, EntrySnapshot)> {
    let mut entries = fs::read_dir(root)
        .expect("fixture directory should remain readable")
        .map(|entry| {
            let entry = entry.expect("fixture directory entry should remain readable");
            let name = entry.file_name().to_string_lossy().into_owned();
            let file_type = entry
                .file_type()
                .expect("fixture directory entry type should remain readable");
            let snapshot = if file_type.is_file() {
                EntrySnapshot::File(
                    fs::read(entry.path()).expect("fixture file should be readable"),
                )
            } else if file_type.is_dir() {
                EntrySnapshot::Directory
            } else if file_type.is_symlink() {
                EntrySnapshot::Symlink
            } else {
                EntrySnapshot::Other
            };
            (name, snapshot)
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}

fn artifact_snapshot(path: &Path) -> Option<EntrySnapshot> {
    let file_type = fs::symlink_metadata(path).ok()?.file_type();
    Some(if file_type.is_file() {
        EntrySnapshot::File(fs::read(path).expect("artifact should be readable"))
    } else if file_type.is_dir() {
        EntrySnapshot::Directory
    } else if file_type.is_symlink() {
        EntrySnapshot::Symlink
    } else {
        EntrySnapshot::Other
    })
}

fn assert_unchanged(
    label: &str,
    root: &Path,
    artifact: &Path,
    before_directory: &[(String, EntrySnapshot)],
    before_artifact: &Option<EntrySnapshot>,
) {
    assert_eq!(
        directory_snapshot(root),
        before_directory,
        "{label}: adjacent directory entries changed"
    );
    assert_eq!(
        artifact_snapshot(artifact),
        before_artifact.clone(),
        "{label}: supplied artifact changed"
    );
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn run_sql(path: &Path, sql: &str) {
    assert!(
        SqlitePrototype::new(path).available(),
        "sqlite3 is required for recovery inspection fixtures"
    );
    let mut child = Command::new("sqlite3")
        .args(["-batch", "-noheader"])
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sqlite3 should start");
    let mut stdin = child
        .stdin
        .take()
        .expect("sqlite3 stdin should be available");
    stdin
        .write_all(sql.as_bytes())
        .expect("sqlite3 fixture SQL should be accepted");
    drop(stdin);
    let output = child
        .wait_with_output()
        .expect("sqlite3 fixture should finish");
    assert!(
        output.status.success(),
        "sqlite3 fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_supported(path: &Path) -> RecoveryManifest {
    let memory = MemoryRecoveryStore::new(TopologyEpoch(1));
    let manifest = memory
        .export_manifest(RecoveryGeneration::ZERO)
        .expect("memory recovery fixture should export");
    let store = SqliteRecoveryStore::create_new(path, manifest.clone())
        .expect("supported SQLite recovery fixture should be created");
    drop(store);
    manifest
}

struct CaseExpectation {
    classification: &'static str,
    format: Value,
    manifest: Value,
    next_action: &'static str,
}

fn expectation(
    classification: &'static str,
    format: Value,
    manifest: Value,
    next_action: &'static str,
) -> CaseExpectation {
    CaseExpectation {
        classification,
        format,
        manifest,
        next_action,
    }
}

fn unknown_format_case(classification: &'static str, next_action: &'static str) -> CaseExpectation {
    expectation(
        classification,
        json!({"layer": null, "version": null}),
        Value::Null,
        next_action,
    )
}

fn supported_case(path: &Path) -> CaseExpectation {
    let manifest = create_supported(path);
    expectation(
        "supported",
        json!({
            "layer": "storage-and-semantic",
            "storage_version": CURRENT_RECOVERY_SQLITE_SCHEMA,
            "semantic_version": u64::from(CURRENT_RECOVERY_SCHEMA.0),
        }),
        serde_json::to_value(manifest).expect("supported manifest should serialize"),
        "none",
    )
}

fn absent_case(_path: &Path) -> CaseExpectation {
    expectation(
        "absent",
        json!({"layer": null, "version": null}),
        Value::Null,
        "provide-artifact",
    )
}

fn corrupt_case(path: &Path) -> CaseExpectation {
    fs::write(path, b"not a SQLite database").expect("corrupt fixture should be written");
    unknown_format_case("corrupt-or-unreadable", "inspect-recovery-options")
}

fn unsupported_case(path: &Path) -> CaseExpectation {
    run_sql(path, "PRAGMA user_version=999;\n");
    expectation(
        "unsupported",
        json!({"layer": "storage", "version": 999}),
        Value::Null,
        "use-compatible-inspector",
    )
}

fn migration_case(path: &Path) -> CaseExpectation {
    run_sql(path, "PRAGMA user_version=1;\n");
    expectation(
        "migration-required",
        json!({
            "layer": "storage",
            "from": 1,
            "to": CURRENT_RECOVERY_SQLITE_SCHEMA,
        }),
        Value::Null,
        "review-migration",
    )
}
fn semantic_migration_case(path: &Path) -> CaseExpectation {
    create_supported(path);
    run_sql(
        path,
        "UPDATE recovery_state SET schema_version=3 WHERE singleton=1;\n",
    );
    expectation(
        "migration-required",
        json!({
            "layer": "semantic",
            "from": 3,
            "to": u64::from(CURRENT_RECOVERY_SCHEMA.0),
            "storage_version": CURRENT_RECOVERY_SQLITE_SCHEMA,
        }),
        Value::Null,
        "review-migration",
    )
}

fn semantic_unsupported_case(path: &Path) -> CaseExpectation {
    create_supported(path);
    run_sql(
        path,
        "UPDATE recovery_state SET schema_version=999 WHERE singleton=1;\n",
    );
    expectation(
        "unsupported",
        json!({
            "layer": "semantic",
            "version": 999,
            "storage_version": CURRENT_RECOVERY_SQLITE_SCHEMA,
        }),
        Value::Null,
        "use-compatible-inspector",
    )
}

fn reconciliation_case(path: &Path) -> CaseExpectation {
    create_supported(path);
    fs::write(
        sidecar(path, ".commit-intent"),
        b"uninterpreted uncertain commit",
    )
    .expect("commit-intent fixture should be written");
    fs::write(sidecar(path, "-journal"), b"uninterpreted journal bytes")
        .expect("journal-like fixture should be written");
    unknown_format_case("reconciliation-required", "reconcile-before-use")
}

fn run_inspector(json_mode: bool, artifact: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dwv-recovery-inspect"));
    if json_mode {
        command.arg("--json");
    }
    if let Some(artifact) = artifact {
        command.arg(artifact);
    }
    command.output().expect("recovery inspector should start")
}

fn assert_output_bound(label: &str, output: &Output) {
    assert!(
        output.stdout.len() <= RESULT_OUTPUT_BOUND,
        "{label}: stdout exceeded the active result bound"
    );
    assert!(
        output.stderr.len() <= RESULT_OUTPUT_BOUND,
        "{label}: stderr exceeded the active result bound"
    );
}

fn json_semantics(value: &Value) -> Value {
    json!({
        "schema": value.get("schema").expect("JSON result schema is required"),
        "command": value.get("command").expect("JSON result command is required"),
        "outcome": value.get("outcome").expect("JSON result outcome is required"),
        "classification": value
            .get("classification")
            .expect("JSON classification is required"),
        "format_claim": value
            .get("format_claim")
            .expect("JSON format claim is required"),
        "experimental": value
            .get("experimental")
            .expect("JSON experimental status is required"),
        "format": value.get("format").expect("JSON format is required"),
        "manifest": value.get("manifest").expect("JSON manifest is required"),
        "non_authorization": value
            .get("non_authorization")
            .expect("JSON non-authorization statement is required"),
        "next_action": value
            .get("next_action")
            .expect("JSON next action is required"),
    })
}

fn human_semantics(output: &Output) -> Value {
    let fields = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let line = std::str::from_utf8(line).expect("human output should be UTF-8");
            let (key, value) = line
                .split_once(": ")
                .expect("human output should use key/value lines");
            (key.to_owned(), value.to_owned())
        })
        .collect::<BTreeMap<_, _>>();
    let field = |key: &str| {
        fields
            .get(key)
            .unwrap_or_else(|| panic!("missing human field {key}"))
    };
    json!({
        "schema": field("schema"),
        "command": field("command"),
        "outcome": field("outcome"),
        "classification": field("classification"),
        "format_claim": field("format_claim"),
        "experimental": field("experimental")
            .parse::<bool>()
            .expect("human experimental field should be boolean"),
        "format": serde_json::from_str::<Value>(field("format"))
            .expect("human format should be JSON"),
        "manifest": serde_json::from_str::<Value>(field("manifest"))
            .expect("human manifest should be JSON"),
        "non_authorization": field("non_authorization"),
        "next_action": field("next_action"),
    })
}

fn assert_contract(label: &str, semantics: &Value, expected: &CaseExpectation) {
    assert_eq!(semantics["schema"], "dwv.recovery-inspection.v1", "{label}");
    assert_eq!(semantics["command"], "dwv-recovery-inspect", "{label}");
    assert_eq!(semantics["outcome"], "observed", "{label}");
    assert_eq!(
        semantics["classification"], expected.classification,
        "{label}"
    );
    assert_eq!(semantics["format"], expected.format, "{label}");
    assert_eq!(semantics["manifest"], expected.manifest, "{label}");
    assert_eq!(semantics["next_action"], expected.next_action, "{label}");
    assert_eq!(semantics["format_claim"], "experimental", "{label}");
    assert_eq!(semantics["experimental"], true, "{label}");

    let non_authorization = semantics["non_authorization"]
        .as_str()
        .expect("non-authorization statement should be a string");
    for phrase in [
        "no writable format interpretation",
        "publication",
        "payload or recovery mutation",
        "repair",
        "migration",
        "parity or integrity claim",
        "lineage or custody claim",
        "historical recovery claim",
        "stable-format claim",
    ] {
        assert!(
            non_authorization.contains(phrase),
            "{label}: non-authorization statement lost {phrase:?}"
        );
    }
}

fn inspect_case(label: &str, setup: fn(&Path) -> CaseExpectation) {
    let fixture = FixtureDirectory::new();
    let artifact = fixture.artifact();
    let expected = setup(&artifact);
    let before_directory = directory_snapshot(&fixture.0);
    let before_artifact = artifact_snapshot(&artifact);

    let human = run_inspector(false, Some(&artifact));
    assert_eq!(human.status.code(), Some(0), "{label}: human status");
    assert_output_bound(label, &human);
    let human_semantics = human_semantics(&human);
    assert_contract(label, &human_semantics, &expected);
    assert_unchanged(
        label,
        &fixture.0,
        &artifact,
        &before_directory,
        &before_artifact,
    );

    let json_output = run_inspector(true, Some(&artifact));
    assert_eq!(json_output.status.code(), Some(0), "{label}: JSON status");
    assert_output_bound(label, &json_output);
    let json_value: Value =
        serde_json::from_slice(&json_output.stdout).expect("JSON inspection result should parse");
    let json_semantics = json_semantics(&json_value);
    assert_contract(label, &json_semantics, &expected);
    assert_eq!(
        human_semantics, json_semantics,
        "{label}: renderer meaning differs"
    );
    assert_unchanged(
        label,
        &fixture.0,
        &artifact,
        &before_directory,
        &before_artifact,
    );
}

/// dwv:req req.independent-recovery-inspection.independent-recovery-state-inspection-is-bounded-and-non-authorizing
#[test]
#[allow(clippy::type_complexity)]
fn every_disposition_is_a_successful_read_only_observation() {
    let cases: &[(&str, fn(&Path) -> CaseExpectation)] = &[
        ("supported", supported_case),
        ("absent", absent_case),
        ("corrupt", corrupt_case),
        ("unsupported", unsupported_case),
        ("migration", migration_case),
        ("semantic-migration", semantic_migration_case),
        ("semantic-unsupported", semantic_unsupported_case),
        ("reconciliation", reconciliation_case),
    ];
    for (label, setup) in cases {
        inspect_case(label, *setup);
    }
}

#[test]
fn malformed_invocation_is_usage_failure_without_mutation() {
    let fixture = FixtureDirectory::new();
    let before_directory = directory_snapshot(&fixture.0);
    let output = run_inspector(true, None);
    assert_eq!(output.status.code(), Some(2));
    assert_output_bound("malformed invocation", &output);
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert_unchanged(
        "malformed invocation",
        &fixture.0,
        &fixture.artifact(),
        &before_directory,
        &None,
    );
}
#[cfg(unix)]
#[test]
fn stdout_write_failure_is_operational_failure_without_mutation() {
    let fixture = FixtureDirectory::new();
    let artifact = fixture.artifact();
    create_supported(&artifact);
    let before_directory = directory_snapshot(&fixture.0);
    let before_artifact = artifact_snapshot(&artifact);
    let (stdout, peer) = UnixStream::pair().expect("stdout pipe fixture should be created");
    drop(peer);
    let output = Command::new(env!("CARGO_BIN_EXE_dwv-recovery-inspect"))
        .arg(&artifact)
        .stdout(Stdio::from(std::os::fd::OwnedFd::from(stdout)))
        .stderr(Stdio::piped())
        .spawn()
        .expect("recovery inspector should start")
        .wait_with_output()
        .expect("recovery inspector should finish");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_output_bound("stdout write failure", &output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("unable to render inspection result"));
    assert_unchanged(
        "stdout write failure",
        &fixture.0,
        &artifact,
        &before_directory,
        &before_artifact,
    );
}

#[test]
fn oversized_sparse_artifact_is_refused_before_sqlite_without_mutation() {
    let fixture = FixtureDirectory::new();
    let artifact = fixture.artifact();
    let sparse = fs::File::create(&artifact).expect("sparse fixture should be created");
    sparse
        .set_len(MAX_RECOVERY_ARTIFACT_BYTES + 1)
        .expect("sparse fixture should exceed artifact bound");
    let before_directory = directory_snapshot(&fixture.0);
    let before_artifact = artifact_snapshot(&artifact);

    for json_mode in [false, true] {
        let mode = if json_mode { "JSON" } else { "human" };
        let output = run_inspector(json_mode, Some(&artifact));
        assert_eq!(output.status.code(), Some(0), "sparse {mode} status");
        assert_output_bound("sparse artifact", &output);
        let semantics = if json_mode {
            let value: Value =
                serde_json::from_slice(&output.stdout).expect("sparse JSON result should parse");
            json_semantics(&value)
        } else {
            human_semantics(&output)
        };
        assert_eq!(semantics["classification"], "corrupt-or-unreadable");
        assert_eq!(semantics["format"], json!({"layer": null, "version": null}));
        assert_eq!(semantics["manifest"], Value::Null);
        assert_unchanged(
            "sparse artifact",
            &fixture.0,
            &artifact,
            &before_directory,
            &before_artifact,
        );
    }
}

#[test]
fn oversized_manifest_is_conservatively_refused_without_mutation() {
    let fixture = FixtureDirectory::new();
    let artifact = fixture.artifact();
    let _manifest = create_supported(&artifact);
    let oversized = "x".repeat(MAX_MANIFEST_JSON_BYTES + 1);
    run_sql(
        &artifact,
        &format!("UPDATE recovery_state SET manifest_json = '{oversized}';\n"),
    );
    let before_directory = directory_snapshot(&fixture.0);
    let before_artifact = artifact_snapshot(&artifact);

    for json_mode in [false, true] {
        let mode = if json_mode { "JSON" } else { "human" };
        let output = run_inspector(json_mode, Some(&artifact));
        assert_eq!(output.status.code(), Some(0), "oversized {mode} status");
        assert_output_bound("oversized manifest", &output);
        let semantics = if json_mode {
            let value: Value =
                serde_json::from_slice(&output.stdout).expect("oversized JSON result should parse");
            json_semantics(&value)
        } else {
            human_semantics(&output)
        };
        assert_eq!(
            semantics["classification"], "corrupt-or-unreadable",
            "oversized input should remain conservatively classified"
        );
        assert_eq!(semantics["manifest"], Value::Null);
        assert_eq!(
            semantics["format"],
            json!({
                "layer": "storage-and-semantic",
                "storage_version": CURRENT_RECOVERY_SQLITE_SCHEMA,
                "semantic_version": u64::from(CURRENT_RECOVERY_SCHEMA.0),
            })
        );
        assert_unchanged(
            "oversized manifest",
            &fixture.0,
            &artifact,
            &before_directory,
            &before_artifact,
        );
    }
}
