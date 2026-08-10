use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

fn fixture() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "dwv-operator-{}-{nonce}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    assert!(
        Command::new(env!("CARGO_BIN_EXE_dwv"))
            .args(["demo", "file", "init", "--root"])
            .arg(&root)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new(env!("CARGO_BIN_EXE_dwv"))
            .args(["demo", "file", "policy", "--root"])
            .arg(&root)
            .status()
            .unwrap()
            .success()
    );
    root
}

fn run(root: &Path, command: &str, extra: &[&str]) -> Output {
    let mut args = vec![OsString::from(command), OsString::from("--array")];
    args.push(root.join("array.json").into_os_string());
    args.push(OsString::from("--json"));
    args.extend(extra.iter().map(OsString::from));
    Command::new(env!("CARGO_BIN_EXE_dwv"))
        .args(args)
        .output()
        .unwrap()
}
fn run_human(root: &Path, command: &str, extra: &[&str]) -> Output {
    let mut args = vec![OsString::from(command), OsString::from("--array")];
    args.push(root.join("array.json").into_os_string());
    args.extend(extra.iter().map(OsString::from));
    Command::new(env!("CARGO_BIN_EXE_dwv"))
        .args(args)
        .output()
        .unwrap()
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn failure(output: Output, code: i32) -> Value {
    assert_eq!(output.status.code(), Some(code));
    serde_json::from_slice(&output.stderr).unwrap()
}

#[test]
fn observation_commands_are_read_only_and_keep_state_dimensions_separate() {
    let root = fixture();
    let tracked = ["data0.raw", "data1.raw", "parity.raw", "recovery.sqlite3"];
    let before: Vec<_> = tracked
        .iter()
        .map(|name| fs::read(root.join(name)).unwrap())
        .collect();

    let status = success(run(&root, "status", &[]));
    assert_eq!(status["lifecycle"], "stopped");
    assert_eq!(status["access"], "none");
    assert_eq!(status["kind"], "array-operator-result");
    assert_eq!(status["reason_code"], "current-array-observed");
    assert_eq!(status["parity"], "not-verified");
    assert_eq!(status["damage"], "not-assessed");
    assert_eq!(status["start"], "read-write-available");
    assert_eq!(status["recovery"]["classification"], "supported");
    assert_eq!(status["checksum"]["status"], "not-required");
    assert_eq!(status["publication"]["status"], "not-published");
    assert!(status["members"].is_array());
    assert!(status["topology"].is_object());
    let human = run_human(&root, "status", &[]);
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    for expected in [
        "kind: array-operator-result",
        "reason-code: current-array-observed",
        "outcome: Success",
        "state: stopped",
        "access: none",
        "start: read-write-available",
        "\"classification\":\"supported\"",
        "\"status\":\"not-required\"",
        "\"status\":\"not-published\"",
    ] {
        assert!(
            human.contains(expected),
            "missing human classification: {expected}"
        );
    }

    for command in ["members", "scrub", "damage", "recover"] {
        success(run(&root, command, &[]));
    }
    for (name, bytes) in tracked.iter().zip(before) {
        assert_eq!(fs::read(root.join(name)).unwrap(), bytes, "{name} changed");
    }

    let parity = root.join("parity.raw");
    let unavailable = root.join("parity.unavailable");
    fs::rename(&parity, &unavailable).unwrap();
    let output = run(&root, "scrub", &[]);
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["outcome"], "blocked");
    assert!(
        report["members"]
            .as_array()
            .unwrap()
            .iter()
            .any(|member| member["status"] == "unreadable")
    );
    assert!(
        report["verification"]["regions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|region| region["disposition"] == "unreadable-member")
    );
    fs::rename(unavailable, parity).unwrap();

    let policy_path = root.join("array.json");
    let original_policy = fs::read(&policy_path).unwrap();
    let mut cloned_policy: Value = serde_json::from_slice(&original_policy).unwrap();
    cloned_policy["members"][1]["path"] = cloned_policy["members"][0]["path"].clone();
    cloned_policy["members"][1]["expected_identity"] =
        cloned_policy["members"][0]["expected_identity"].clone();
    fs::write(
        &policy_path,
        serde_json::to_vec_pretty(&cloned_policy).unwrap(),
    )
    .unwrap();
    let ambiguous = success(run(&root, "status", &[]));
    assert_eq!(ambiguous["start"], "member-resolution-required");
    assert_eq!(
        ambiguous["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|member| member["status"] == "ambiguous-clone")
            .count(),
        2
    );
    fs::write(policy_path, original_policy).unwrap();

    let recovery = root.join("recovery.sqlite3");
    let recovery_bytes = fs::read(&recovery).unwrap();
    fs::remove_file(&recovery).unwrap();
    let missing = success(run(&root, "status", &[]));
    assert_eq!(missing["outcome"], "success");
    assert_eq!(missing["recovery"]["classification"], "absent");
    assert_eq!(missing["start"], "recovery-required");
    assert!(!recovery.exists());
    fs::write(recovery, recovery_bytes).unwrap();

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recovery_apply_revalidates_then_requires_a_resumable_baseline_before_start() {
    let root = fixture();
    let recovery = root.join("recovery.sqlite3");
    fs::remove_file(&recovery).unwrap();
    let payload_before = [
        fs::read(root.join("data0.raw")).unwrap(),
        fs::read(root.join("data1.raw")).unwrap(),
        fs::read(root.join("parity.raw")).unwrap(),
    ];

    let preview = success(run(&root, "recover", &[]));
    assert_eq!(preview["recovery"]["classification"], "absent");
    assert_eq!(preview["recovery_plan"]["executable"], true);
    let plan_id = preview["recovery_plan"]["plan_id"].as_str().unwrap();
    let applied = success(run(&root, "recover", &["--apply", plan_id]));
    assert_eq!(applied["checksum"]["status"], "required");
    assert_eq!(applied["start"], "baseline-required");
    assert_eq!(fs::read(root.join("data0.raw")).unwrap(), payload_before[0]);
    assert_eq!(fs::read(root.join("data1.raw")).unwrap(), payload_before[1]);
    assert_eq!(
        fs::read(root.join("parity.raw")).unwrap(),
        payload_before[2]
    );

    let partial = success(run(&root, "baseline", &["--max-extents", "1"]));
    assert_eq!(partial["checksum"]["status"], "partial");
    let reopened = success(run(&root, "status", &[]));
    assert_eq!(reopened["checksum"]["status"], "partial");
    assert_eq!(reopened["start"], "baseline-required");
    assert_eq!(
        failure(run(&root, "start", &[]), 4)["error"]["class"],
        "refused"
    );

    let complete = success(run(&root, "baseline", &[]));
    assert_eq!(complete["checksum"]["status"], "complete");
    assert_eq!(complete["start"], "read-write-available");
    let unsupported = failure(run(&root, "start", &[]), 5);
    assert_eq!(unsupported["error"]["class"], "not-supported");
    assert!(
        unsupported["error"]["message"]
            .as_str()
            .unwrap()
            .contains("selects no frontend")
    );

    let policy_path = root.join("array.json");
    let policy_bytes = fs::read(&policy_path).unwrap();
    let mut linux_policy: Value = serde_json::from_slice(&policy_bytes).unwrap();
    linux_policy["frontend"] = Value::String("linux-ublk".into());
    fs::write(
        &policy_path,
        serde_json::to_vec_pretty(&linux_policy).unwrap(),
    )
    .unwrap();
    let unsupported_profile = failure(run(&root, "start", &[]), 5);
    assert_eq!(unsupported_profile["error"]["class"], "not-supported");
    assert!(
        unsupported_profile["error"]["message"]
            .as_str()
            .unwrap()
            .contains("requires exactly one data and one parity member")
    );
    fs::write(policy_path, policy_bytes).unwrap();

    let data_path = root.join("data0.raw");
    let mut data = fs::read(&data_path).unwrap();
    data[0] ^= 0xff;
    fs::write(&data_path, data).unwrap();
    let damage = success(run(&root, "damage", &[]));
    assert_eq!(damage["verification"]["complete"], true);
    assert_eq!(
        damage["verification"]["regions"][0]["disposition"],
        "known-data-mismatch:0"
    );
    assert_eq!(
        damage["verification"]["regions"][0]["data_evidence"][0],
        "CurrentMismatch"
    );
    assert_eq!(
        damage["verification"]["regions"][0]["parity_evidence"],
        "CurrentMatch"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrupt_recovery_is_previewed_without_mutation_and_preserved_on_apply() {
    let root = fixture();
    let recovery = root.join("recovery.sqlite3");
    let corrupt = b"untrusted recovery bytes".to_vec();
    fs::write(&recovery, &corrupt).unwrap();
    let payload_before = [
        fs::read(root.join("data0.raw")).unwrap(),
        fs::read(root.join("data1.raw")).unwrap(),
        fs::read(root.join("parity.raw")).unwrap(),
    ];

    let preview = success(run(&root, "recover", &[]));
    assert_eq!(
        preview["recovery"]["classification"],
        "corrupt-or-unreadable"
    );
    assert_eq!(preview["recovery_plan"]["executable"], true);
    assert_eq!(fs::read(&recovery).unwrap(), corrupt);
    let plan_id = preview["recovery_plan"]["plan_id"].as_str().unwrap();
    let applied = success(run(&root, "recover", &["--apply", plan_id]));
    assert_eq!(applied["recovery"]["classification"], "supported");
    assert_eq!(fs::read(root.join("data0.raw")).unwrap(), payload_before[0]);
    assert_eq!(fs::read(root.join("data1.raw")).unwrap(), payload_before[1]);
    assert_eq!(
        fs::read(root.join("parity.raw")).unwrap(),
        payload_before[2]
    );

    let preserved: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("recovery.sqlite3.dwv-untrusted-")
        })
        .collect();
    assert_eq!(preserved.len(), 1);
    assert_eq!(fs::read(&preserved[0]).unwrap(), corrupt);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_recovery_preview_refuses_without_publishing_recovery_state() {
    let root = fixture();
    let recovery = root.join("recovery.sqlite3");
    fs::remove_file(&recovery).unwrap();
    let preview = success(run(&root, "recover", &[]));
    let plan_id = preview["recovery_plan"]["plan_id"].as_str().unwrap();

    let data_path = root.join("data0.raw");
    let mut data = fs::read(&data_path).unwrap();
    data[0] ^= 0xff;
    fs::write(&data_path, &data).unwrap();
    let refused = failure(run(&root, "recover", &["--apply", plan_id]), 4);
    assert_eq!(refused["error"]["class"], "refused");
    assert_eq!(refused["error"]["reason_code"], "semantic-refusal");
    assert!(!recovery.exists());
    assert_eq!(fs::read(data_path).unwrap(), data);

    fs::remove_dir_all(root).unwrap();
}
