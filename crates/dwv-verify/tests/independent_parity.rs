use dwv_recovery::{Blake3Provider, Digest, DigestProvider};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    data0: PathBuf,
    data1: PathBuf,
    parity: PathBuf,
    descriptor: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after the Unix epoch")
            .as_nanos();
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "dwv-independent-parity-{}-{nonce}-{fixture_id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let data0 = root.join("data0.raw");
        let data1 = root.join("data1.raw");
        let parity = root.join("parity.raw");
        let descriptor = root.join("descriptor.json");
        fs::write(&data0, [1_u8, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        fs::write(&data1, [8_u8, 7, 6, 5, 4, 3, 2, 1]).unwrap();
        fs::write(
            &parity,
            xor(&[1, 2, 3, 4, 5, 6, 7, 8], &[8, 7, 6, 5, 4, 3, 2, 1]),
        )
        .unwrap();
        Self {
            root,
            data0,
            data1,
            parity,
            descriptor,
        }
    }

    fn descriptor(&self) -> Value {
        json!({
            "version": "dwv.independent-parity.v1",
            "profile": "single-xor",
            "protected_length": 8,
            "whole_protected_range": true,
            "region_size": 4,
            "max_regions": 2,
            "data_payloads": [self.data0, self.data1],
            "parity_payload": self.parity,
            "output": "human"
        })
    }

    fn write_descriptor(&self, descriptor: Value) {
        fs::write(&self.descriptor, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn xor(left: &[u8], right: &[u8]) -> Vec<u8> {
    left.iter()
        .zip(right)
        .map(|(left, right)| left ^ right)
        .collect()
}

fn run(fixture: &Fixture, json: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dwv-independent-parity"));
    if json {
        command.arg("--json");
    }
    command.arg(&fixture.descriptor).output().unwrap()
}

fn result_json(output: &Output) -> Value {
    assert!(
        output.status.success() || output.status.code() == Some(2),
        "unexpected process status: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn digest(path: &Path) -> Digest {
    Blake3Provider
        .digest(
            &fs::read(path)
                .unwrap_or_else(|error| panic!("failed to hash {}: {error}", path.display())),
        )
        .unwrap()
}

fn hashes(fixture: &Fixture) -> [Digest; 4] {
    [
        digest(&fixture.data0),
        digest(&fixture.data1),
        digest(&fixture.parity),
        digest(&fixture.descriptor),
    ]
}

fn assert_unchanged(fixture: &Fixture, before: [Digest; 4]) {
    assert_eq!(hashes(fixture), before);
}

fn human_projection(output: &Output) -> Value {
    assert!(
        output.status.success() || output.status.code() == Some(2),
        "human status: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let mut fields = Map::new();
    let mut regions = Vec::new();
    let mut non_claims = Vec::new();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("region: ") {
            let mut region = Map::new();
            for field in value.splitn(4, ' ') {
                let (key, value) = field.split_once('=').unwrap();
                region.insert(
                    key.to_owned(),
                    if matches!(key, "offset" | "length") {
                        Value::from(value.parse::<u64>().unwrap())
                    } else {
                        Value::from(value)
                    },
                );
            }
            regions.push(Value::Object(region));
        } else if let Some(value) = line.strip_prefix("non_claim: ") {
            non_claims.push(Value::from(value));
        } else if let Some((key, value)) = line.split_once(": ") {
            let value = match key {
                "experimental" => Value::from(value.parse::<bool>().unwrap()),
                "exit_code" => Value::from(value.parse::<u64>().unwrap()),
                "descriptor_version" if value == "unknown" => Value::Null,
                _ => Value::from(value),
            };
            fields.insert(key.to_owned(), value);
        }
    }
    fields.insert("regions".to_owned(), Value::Array(regions));
    fields.insert("non_claims".to_owned(), Value::Array(non_claims));
    Value::Object(fields)
}

fn semantic_projection(result: &Value) -> Value {
    let object = result.as_object().unwrap();
    json!({
        "schema": object["schema"],
        "command": object["command"],
        "outcome": object["outcome"],
        "descriptor_version": object["descriptor_version"],
        "format_claim": object["format_claim"],
        "experimental": object["experimental"],
        "evidence_tier": object["evidence_tier"],
        "regions": object["regions"],
        "non_claims": object["non_claims"],
        "reason": object["reason"],
        "process_status": object["process_status"],
        "exit_code": object["exit_code"]
    })
}
/// dwv:req req.independent-parity-verification.standalone-equation-verification-is-bounded-read-only-and-non-authorizing
#[test]
fn independent_parity_reports_bounded_equations_without_mutation() {
    let fixture = Fixture::new();
    fixture.write_descriptor(fixture.descriptor());
    let before = hashes(&fixture);

    let human = run(&fixture, false);
    let structured = run(&fixture, true);
    let human_result = human_projection(&human);
    let structured_result = result_json(&structured);
    assert_eq!(
        semantic_projection(&human_result),
        semantic_projection(&structured_result)
    );
    assert_eq!(human.status.code(), Some(0));
    assert_eq!(structured.status.code(), Some(0));
    assert_eq!(structured_result["process_status"], "completed-observation");
    assert_eq!(
        structured_result["regions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|region| region["disposition"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["matched", "matched"]
    );
    assert_eq!(structured_result["experimental"], true);
    assert_eq!(structured_result["evidence_tier"], "file-backed-model");
    assert_eq!(structured_result["non_claims"].as_array().unwrap().len(), 4);
    assert_unchanged(&fixture, before);

    let first = structured.stdout.clone();
    let second = run(&fixture, true);
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(second.stdout, first);
    let first_human = human.stdout.clone();
    let second_human = run(&fixture, false);
    assert_eq!(second_human.status.code(), Some(0));
    assert_eq!(second_human.stdout, first_human);
    assert_unchanged(&fixture, before);

    fs::write(&fixture.parity, [0_u8, 5, 5, 1, 1, 5, 5, 9]).unwrap();
    fixture.write_descriptor(fixture.descriptor());
    let before = hashes(&fixture);
    let mismatch_output = run(&fixture, true);
    assert_eq!(mismatch_output.status.code(), Some(0));
    let mismatch = result_json(&mismatch_output);
    assert_eq!(mismatch["process_status"], "completed-observation");
    assert_eq!(
        mismatch["reason"],
        "one or more bounded XOR equations mismatched"
    );
    assert_eq!(
        mismatch["regions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|region| region["disposition"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["mismatched", "matched"]
    );
    assert_unchanged(&fixture, before);

    fs::write(&fixture.data0, [1_u8, 2]).unwrap();
    fs::write(&fixture.data1, [8_u8, 7, 6, 5, 4, 3, 2, 1]).unwrap();
    fs::write(&fixture.parity, [9_u8, 5, 6, 5, 4, 3, 2, 1]).unwrap();
    fixture.write_descriptor(fixture.descriptor());
    let before = hashes(&fixture);
    let zero_tail_output = run(&fixture, true);
    assert_eq!(zero_tail_output.status.code(), Some(0));
    let zero_tail = result_json(&zero_tail_output);

    assert_eq!(
        zero_tail["reason"],
        "all selected bounded XOR equations matched"
    );
    assert!(
        zero_tail["regions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|region| { region["disposition"] == "matched" })
    );
    assert_unchanged(&fixture, before);

    fs::write(&fixture.parity, [9_u8, 5, 6, 5, 4, 3]).unwrap();
    fixture.write_descriptor(fixture.descriptor());
    let before = hashes(&fixture);
    let short_output = run(&fixture, true);
    assert_eq!(short_output.status.code(), Some(0));
    let short = result_json(&short_output);
    assert_eq!(
        short["reason"],
        "one or more required payload reads were incomplete"
    );
    assert_eq!(
        short["regions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|region| region["disposition"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["matched", "incomplete"]
    );
    assert_unchanged(&fixture, before);
}

#[test]
fn independent_parity_rejects_failed_reads_aliases_and_hostile_bounds() {
    let fixture = Fixture::new();
    fixture.write_descriptor(fixture.descriptor());

    let missing = fixture.root.join("missing.raw");
    let mut descriptor = fixture.descriptor();
    descriptor["parity_payload"] = missing.to_str().unwrap().into();
    fixture.write_descriptor(descriptor);
    let before = hashes(&fixture);
    let failed_read = result_json(&run(&fixture, true));
    assert_eq!(failed_read["process_status"], "invalid-unsupported-input");
    assert_eq!(failed_read["reason"], "payload identity is unavailable");
    assert_eq!(failed_read["regions"].as_array().unwrap().len(), 0);
    assert_eq!(run(&fixture, true).status.code(), Some(2));
    assert_unchanged(&fixture, before);

    let mut descriptor = fixture.descriptor();
    descriptor["data_payloads"] = json!([fixture.data0, fixture.data0]);
    fixture.write_descriptor(descriptor);
    let before = hashes(&fixture);
    let duplicate = result_json(&run(&fixture, true));
    assert_eq!(
        duplicate["reason"],
        "payload identities are duplicate or aliased"
    );
    assert_unchanged(&fixture, before);
    fs::write(
        &fixture.descriptor,
        br#"{"version":"dwv.independent-parity.v0","version":"dwv.independent-parity.v1"}"#,
    )
    .unwrap();
    let before = hashes(&fixture);
    let duplicate_member = result_json(&run(&fixture, true));
    assert_eq!(
        duplicate_member["reason"],
        "descriptor contains duplicate members"
    );
    assert_unchanged(&fixture, before);

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let alias = fixture.root.join("data0-alias.raw");
        symlink(&fixture.data0, &alias).unwrap();
        let mut descriptor = fixture.descriptor();
        descriptor["data_payloads"] = json!([fixture.data0, alias]);
        fixture.write_descriptor(descriptor);
        let before = hashes(&fixture);
        let aliased = result_json(&run(&fixture, true));
        assert_eq!(
            aliased["reason"],
            "payload identities are duplicate or aliased"
        );
        assert_unchanged(&fixture, before);
    }

    let mut selected_descriptor = fixture.descriptor();
    selected_descriptor["selected_range"] = json!({"offset": 4, "length": 4});
    selected_descriptor
        .as_object_mut()
        .unwrap()
        .remove("whole_protected_range");
    fixture.write_descriptor(selected_descriptor);
    let before = hashes(&fixture);
    let selected_range = result_json(&run(&fixture, true));
    assert_eq!(selected_range["process_status"], "completed-observation");
    assert_eq!(
        selected_range["regions"],
        json!([{
            "offset": 4,
            "length": 4,
            "disposition": "matched",
            "reason": "the observed parity matched the explicit XOR equation"
        }])
    );
    assert_unchanged(&fixture, before);

    fs::write(&fixture.parity, [0_u8, 0, 0, 0, 1, 5, 5, 9]).unwrap();
    let before = hashes(&fixture);
    let outside_range_corruption = result_json(&run(&fixture, true));
    assert_eq!(
        outside_range_corruption["process_status"],
        "completed-observation"
    );
    assert_eq!(
        outside_range_corruption["regions"],
        selected_range["regions"]
    );
    assert_unchanged(&fixture, before);

    let mut descriptor = fixture.descriptor();
    descriptor["selected_range"] = json!({"offset": 7, "length": 2});
    descriptor
        .as_object_mut()
        .unwrap()
        .remove("whole_protected_range");
    fixture.write_descriptor(descriptor);
    let before = hashes(&fixture);
    let invalid_geometry = result_json(&run(&fixture, true));
    assert_eq!(
        invalid_geometry["reason"],
        "selected range exceeds protected geometry"
    );
    assert_unchanged(&fixture, before);

    let extra_payloads = (2..6)
        .map(|index| {
            let path = fixture.root.join(format!("extra{index}.raw"));
            fs::write(&path, []).unwrap();
            path
        })
        .collect::<Vec<_>>();
    let mut descriptor = fixture.descriptor();
    descriptor["region_size"] = (8_u64 * 1024 * 1024).into();
    descriptor["max_regions"] = 1.into();
    descriptor["data_payloads"] = json!([
        fixture.data0,
        fixture.data1,
        extra_payloads[0],
        extra_payloads[1],
        extra_payloads[2],
        extra_payloads[3]
    ]);
    fixture.write_descriptor(descriptor);
    let before = hashes(&fixture);
    let hostile_bounds = result_json(&run(&fixture, true));
    assert_eq!(
        hostile_bounds["reason"],
        "region buffers exceed their bound"
    );

    assert!(hostile_bounds.to_string().len() < 4096);
    assert_unchanged(&fixture, before);

    let mut descriptor = fixture.descriptor();
    descriptor["version"] = "dwv.independent-parity.v0".into();
    fixture.write_descriptor(descriptor);
    let before = hashes(&fixture);
    let unsupported = result_json(&run(&fixture, true));
    assert_eq!(unsupported["reason"], "descriptor version is unsupported");
    assert_eq!(unsupported["format_claim"], "experimental");
    assert_eq!(unsupported["experimental"], true);
    assert_unchanged(&fixture, before);
    let before = hashes(&fixture);
    let human_invalid = run(&fixture, false);
    let structured_invalid = run(&fixture, true);
    assert_eq!(human_invalid.status.code(), Some(2));
    assert_eq!(structured_invalid.status.code(), Some(2));
    assert_eq!(
        semantic_projection(&human_projection(&human_invalid)),
        semantic_projection(&result_json(&structured_invalid))
    );
    assert_unchanged(&fixture, before);

    fs::write(&fixture.descriptor, vec![b' '; 1024 * 1024 + 1]).unwrap();
    let oversized = result_json(&run(&fixture, true));
    assert_eq!(oversized["reason"], "descriptor exceeds its byte bound");
    assert!(oversized.to_string().len() < 4096);
}
