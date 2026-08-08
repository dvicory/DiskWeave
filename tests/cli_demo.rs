use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dwv"))
        .args(["demo"])
        .args(args)
        .args(["--root", root.to_str().unwrap()])
        .output()
        .unwrap()
}

fn json_stdout(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn json_stderr(output: &Output) -> Value {
    assert!(!output.status.success());
    serde_json::from_slice(&output.stderr).unwrap()
}

#[test]
fn demo_process_boundary_covers_lifecycle_confirmation_and_alias_refusal() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("dwv-cli-{}-{nonce}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let usage = run(&root, &["nope"]);
    assert_eq!(usage.status.code(), Some(2));
    assert_eq!(json_stderr(&usage)["error"]["class"], "usage");

    let initialized = json_stdout(&run(&root, &["init"]));
    assert_eq!(initialized["command"], "demo.init");
    assert_eq!(initialized["fixture"], "initialized");
    let recovery = root.join("recovery.sqlite3");
    let missing_recovery = root.join("recovery.missing");
    fs::rename(&recovery, &missing_recovery).unwrap();
    let missing = run(&root, &["status"]);
    assert_eq!(missing.status.code(), Some(3));
    assert_eq!(json_stderr(&missing)["error"]["class"], "blocked");
    fs::rename(missing_recovery, &recovery).unwrap();
    let recovery_bytes = fs::read(&recovery).unwrap();
    fs::write(&recovery, b"corrupt recovery state").unwrap();
    let corrupt = run(&root, &["status"]);
    assert_eq!(corrupt.status.code(), Some(5));
    assert_eq!(json_stderr(&corrupt)["error"]["class"], "operation-failed");
    fs::write(&recovery, recovery_bytes).unwrap();

    assert_eq!(json_stdout(&run(&root, &["verify"]))["parity"], "clean");
    assert_eq!(
        json_stdout(&run(&root, &["capabilities"]))["members"]
            .as_array()
            .unwrap()
            .len(),
        4
    );

    let plan = json_stdout(&run(&root, &["plan"]));
    let confirmation = plan["confirmation"].as_str().unwrap();
    let refused = run(&root, &["rebuild", "--confirm", "wrong"]);
    assert_eq!(refused.status.code(), Some(4));
    assert_eq!(json_stderr(&refused)["error"]["class"], "refused");

    let checkpoint = json_stdout(&run(
        &root,
        &["rebuild", "--confirm", confirmation, "--stop-after", "2"],
    ));
    let data0 = root.join("data0.raw");
    let original = root.join("data0.original");
    let bytes = fs::read(&data0).unwrap();
    fs::rename(&data0, &original).unwrap();
    fs::write(&data0, vec![0; bytes.len()]).unwrap();
    let identity_changed = run(&root, &["status"]);
    assert_eq!(identity_changed.status.code(), Some(4));
    assert_eq!(json_stderr(&identity_changed)["error"]["class"], "refused");
    fs::remove_file(&data0).unwrap();
    fs::rename(original, data0).unwrap();

    assert_eq!(checkpoint["workflow"]["checkpoint"], "durable");
    assert_eq!(checkpoint["workflow"]["resumable"], true);

    let rebuilt = json_stdout(&run(&root, &["rebuild", "--confirm", confirmation]));
    assert_eq!(rebuilt["workflow"]["final_verification"], "passed");
    assert_eq!(rebuilt["workflow"]["source_parity_preserved"], true);
    assert_eq!(json_stdout(&run(&root, &["status"]))["rebuilds"], 1);

    let outside = root
        .parent()
        .unwrap()
        .join(format!("dwv-cli-outside-{nonce}.raw"));
    fs::write(&outside, b"outside").unwrap();
    let manifest_path = root.join("fixture.json");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(
        &manifest_path,
        manifest.replacen("data0.raw", &format!("../dwv-cli-outside-{nonce}.raw"), 1),
    )
    .unwrap();
    let escaped = run(&root, &["status"]);
    assert_eq!(escaped.status.code(), Some(4));
    assert_eq!(json_stderr(&escaped)["error"]["class"], "refused");

    let _ = fs::remove_file(outside);
    let _ = fs::remove_dir_all(root);
}
