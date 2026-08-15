use serde_json::Value;
use std::collections::BTreeMap;
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
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn assert_human_projection_preserves_consequential_fields(human: &str, json: &Value) {
    let mut lines = human.lines();
    let mut fields = BTreeMap::new();
    let (schema_key, schema_value) = lines
        .next()
        .expect("human operator result should start with schema")
        .split_once(": ")
        .expect("human schema line should contain a value");
    assert_eq!(schema_key, "schema");
    fields.insert(schema_key, schema_value);

    let (command, reason) = lines
        .next()
        .expect("human operator result should contain command and reason")
        .split_once(": ")
        .expect("human command line should contain a reason");
    assert_eq!(command, json["command"].as_str().unwrap());
    assert_eq!(reason, json["reason"].as_str().unwrap());

    for (key, value) in lines.filter_map(|line| line.split_once(": ")) {
        fields.insert(key, value);
    }
    for (human_key, json_key) in [
        ("schema", "schema"),
        ("kind", "kind"),
        ("reason-code", "reason_code"),
        ("state", "lifecycle"),
        ("access", "access"),
        ("start", "start"),
        ("parity", "parity"),
        ("damage", "damage"),
        ("redundancy", "redundancy"),
        ("next", "next_action"),
        ("authority-blocker", "blocker"),
    ] {
        let expected = if json_key == "blocker" {
            json["authority"][json_key].as_str().unwrap()
        } else {
            json[json_key].as_str().unwrap()
        };
        assert_eq!(fields[human_key], expected, "{human_key} changed");
    }

    let expected_outcome = match json["outcome"].as_str().unwrap() {
        "success" => "Success",
        "usage" => "Usage",
        "refused" => "Refused",
        "blocked" => "Blocked",
        "not-supported" => "NotSupported",
        "operation-failed" => "OperationFailed",
        "reconciliation-required" => "ReconciliationRequired",
        outcome => panic!("unrecognized operator outcome {outcome}"),
    };
    assert_eq!(fields["outcome"], expected_outcome);
    assert_eq!(
        fields["authorization"],
        json["authority"]["authorization"].as_str().unwrap()
    );
    let display_label = |value: &str| match value {
        "accepted" => "Accepted",
        "ambiguous" => "Ambiguous",
        "unproved" => "Unproved",
        "continuity-proved" => "ContinuityProved",
        "gap-observed" => "GapObserved",
        "continuity-unproved" => "ContinuityUnproved",
        value => panic!("unrecognized authority disposition {value}"),
    };
    assert!(
        fields["lineage"].starts_with(display_label(
            json["authority"]["lineage"]["disposition"]
                .as_str()
                .unwrap()
        ))
    );
    assert!(
        fields["custody"].starts_with(display_label(
            json["authority"]["custody"]["disposition"]
                .as_str()
                .unwrap()
        ))
    );
    for (human_key, json_key) in [
        ("checksums", "checksum"),
        ("publication", "publication"),
        ("recovery", "recovery"),
    ] {
        assert_eq!(
            serde_json::from_str::<Value>(fields[human_key]).unwrap(),
            json[json_key],
            "{human_key} changed"
        );
    }
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
    assert_eq!(status["reason_code"], "array-observed");
    assert_eq!(status["parity"], "not-authorized");
    assert_eq!(status["damage"], "not-assessed");
    assert_eq!(status["start"], "authority-assessment-required");
    assert_eq!(status["recovery"]["classification"], "supported");
    assert_eq!(status["checksum"]["status"], "not-required");
    assert_eq!(status["publication"]["status"], "not-published");
    assert_eq!(status["authority"]["lineage"]["disposition"], "accepted");
    assert_eq!(
        status["authority"]["custody"]["disposition"],
        "continuity-unproved"
    );
    assert_eq!(
        status["authority"]["protection_basis"][0]["basis"],
        "not-yet-interpretable"
    );
    assert_eq!(
        status["authority"]["protection_basis"][0]["bytes"],
        status["topology"]["protected_length"]
    );
    assert!(
        status["authority"]["authorization"]
            .as_str()
            .unwrap()
            .contains("authorizes no publication")
    );
    assert!(status["members"].is_array());
    assert!(status["topology"].is_object());
    let human = run_human(&root, "status", &[]);
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    for expected in [
        "schema: dwv.operator.v2",
        "kind: array-operator-result",
        "reason-code: array-observed",
        "outcome: Success",
        "state: stopped",
        "access: none",
        "start: authority-assessment-required",
        "lineage: Accepted",
        "custody: ContinuityUnproved",
        "\"classification\":\"supported\"",
        "\"status\":\"not-required\"",
        "\"status\":\"not-published\"",
        "basis-range: role=parity basis=NotYetInterpretable offset=0 length=16384",
    ] {
        assert!(
            human.contains(expected),
            "missing human classification: {expected}"
        );
    }

    for command in ["members", "scrub", "damage", "recover"] {
        success(run(&root, command, &[]));
    }
    let scrub_human = run_human(&root, "scrub", &[]);
    assert!(scrub_human.status.success());
    let scrub_human = String::from_utf8(scrub_human.stdout).unwrap();
    for expected in [
        "schema: dwv.operator.v2",
        "verification: mode=exhaustive complete=true matching=1/1",
        "verification-region: offset=0 length=16384 disposition=match-without-current-evidence",
    ] {
        assert!(
            scrub_human.contains(expected),
            "missing human verification detail: {expected}"
        );
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
fn human_and_structured_results_preserve_consequential_fields() {
    let root = fixture();

    let observed = success(run(&root, "status", &[]));
    let observed_human = run_human(&root, "status", &[]);
    assert!(observed_human.status.success());
    assert_human_projection_preserves_consequential_fields(
        &String::from_utf8(observed_human.stdout).unwrap(),
        &observed,
    );

    let refused = failure(run(&root, "start", &[]), 5);
    let refused_human = run_human(&root, "start", &[]);
    assert_eq!(refused_human.status.code(), Some(5));
    assert_human_projection_preserves_consequential_fields(
        &String::from_utf8(refused_human.stdout).unwrap(),
        &refused,
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn semantic_failures_use_the_shared_operator_result_contract() {
    let root = fixture();
    let unsupported = failure(run(&root, "start", &[]), 5);
    assert_eq!(unsupported["schema"], "dwv.operator.v2");
    assert_eq!(unsupported["kind"], "array-operator-result");
    assert_eq!(unsupported["command"], "start");
    assert_eq!(unsupported["outcome"], "not-supported");
    assert_eq!(unsupported["reason_code"], "capability-not-supported");
    assert!(
        unsupported["reason"]
            .as_str()
            .unwrap()
            .contains("selects no frontend")
    );
    let human = run_human(&root, "start", &[]);
    assert_eq!(human.status.code(), Some(5));
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("kind: array-operator-result"));
    assert!(human.contains("outcome: NotSupported"));
    assert!(human.contains("reason-code: capability-not-supported"));

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
    assert_eq!(unsupported_profile["outcome"], "not-supported");
    assert!(
        unsupported_profile["reason"]
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
        "unresolved-conflict"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn policy_and_parity_cannot_manufacture_recovery_topology_authority() {
    for corrupt in [false, true] {
        let root = fixture();
        let recovery = root.join("recovery.sqlite3");
        if corrupt {
            fs::write(&recovery, b"untrusted recovery bytes").unwrap();
        } else {
            fs::remove_file(&recovery).unwrap();
        }
        let payload_before = [
            fs::read(root.join("data0.raw")).unwrap(),
            fs::read(root.join("data1.raw")).unwrap(),
            fs::read(root.join("parity.raw")).unwrap(),
        ];

        let preview = success(run(&root, "recover", &[]));
        assert_eq!(preview["recovery_plan"]["executable"], false);
        assert_eq!(
            preview["recovery_plan"]["case"],
            "all-metadata-lost-all-data-present"
        );
        assert_eq!(
            preview["recovery_plan"]["action"],
            "refuse-authority-unavailable"
        );
        assert_eq!(
            preview["recovery_plan"]["payload_write_policy"],
            "no-automatic-write"
        );
        let plan_id = preview["recovery_plan"]["plan_id"].as_str().unwrap();
        let refused = failure(run(&root, "recover", &["--apply", plan_id]), 4);
        assert_eq!(refused["schema"], "dwv.operator.v2");
        assert_eq!(refused["outcome"], "refused");
        assert_eq!(refused["reason_code"], "semantic-refusal");
        assert!(
            refused["reason"]
                .as_str()
                .unwrap()
                .contains("recovery is not executable")
        );
        let scrub = failure(run(&root, "scrub", &[]), 3);
        assert_eq!(scrub["outcome"], "blocked");
        assert_eq!(scrub["reason_code"], "recovery-authority-not-current");
        assert_eq!(fs::read(root.join("data0.raw")).unwrap(), payload_before[0]);
        assert_eq!(fs::read(root.join("data1.raw")).unwrap(), payload_before[1]);
        assert_eq!(
            fs::read(root.join("parity.raw")).unwrap(),
            payload_before[2]
        );
        if corrupt {
            assert_eq!(fs::read(&recovery).unwrap(), b"untrusted recovery bytes");
        } else {
            assert!(!recovery.exists());
        }

        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn declarative_member_order_is_not_topology_semantics() {
    let root = fixture();
    let policy_path = root.join("array.json");
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["members"].as_array_mut().unwrap().reverse();
    fs::write(&policy_path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();

    let status = success(run(&root, "status", &[]));
    assert_eq!(status["start"], "authority-assessment-required");
    assert_eq!(status["recovery"]["classification"], "supported");
    assert!(
        status["members"]
            .as_array()
            .unwrap()
            .iter()
            .all(|member| member["status"] == "recognized")
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn uncertain_recovery_commit_remains_reconciliation_required() {
    let root = fixture();
    let recovery = root.join("recovery.sqlite3");
    let mut marker = recovery.as_os_str().to_owned();
    marker.push(".commit-intent");
    let marker = PathBuf::from(marker);
    fs::write(&marker, b"uninterpreted uncertain commit").unwrap();
    let recovery_before = fs::read(&recovery).unwrap();

    let status = failure(run(&root, "status", &[]), 6);
    assert_eq!(status["outcome"], "reconciliation-required");
    assert_eq!(status["reason_code"], "reconciliation-required");
    assert_eq!(
        status["recovery"]["classification"],
        "reconciliation-required"
    );
    assert!(status["topology"].is_null());
    assert_eq!(fs::read(&recovery).unwrap(), recovery_before);
    assert_eq!(
        fs::read(&marker).unwrap(),
        b"uninterpreted uncertain commit"
    );

    let recover = failure(run(&root, "recover", &[]), 6);
    assert_eq!(recover["outcome"], "reconciliation-required");
    assert_eq!(recover["reason_code"], "reconciliation-required");
    assert!(recover["recovery_plan"].is_null());
    assert_eq!(fs::read(&recovery).unwrap(), recovery_before);
    assert_eq!(
        fs::read(&marker).unwrap(),
        b"uninterpreted uncertain commit"
    );

    fs::remove_dir_all(root).unwrap();
}
