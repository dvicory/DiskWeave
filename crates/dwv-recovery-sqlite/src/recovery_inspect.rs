use dwv_recovery::{RecoveryFormatLayer, RecoveryInspection, RecoveryManifest};
use dwv_recovery_sqlite::{
    CURRENT_RECOVERY_SQLITE_SCHEMA, MAX_MANIFEST_JSON_BYTES, SqliteRecoveryStore,
};
use serde_json::{Value, json};
use std::env;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const COMMAND: &str = "dwv-recovery-inspect";
const RESULT_SCHEMA: &str = "dwv.recovery-inspection.v1";
const FORMAT_CLAIM: &str = "experimental";
const NON_AUTHORIZATION: &str = "inspection authorizes no writable format interpretation, publication, payload or recovery mutation, repair, migration, parity or integrity claim, lineage or custody claim, historical recovery claim, or stable-format claim";
const USAGE_EXIT_CODE: u8 = 2;
const OPERATIONAL_EXIT_CODE: u8 = 1;
const MAX_RESULT_JSON_BYTES: usize = MAX_MANIFEST_JSON_BYTES + 64 * 1024;

#[derive(Clone, Copy, Debug)]
enum FormatFacts {
    Unknown,
    Supported {
        storage_version: u64,
        semantic_version: u64,
    },
    KnownVersions {
        storage_version: Option<u64>,
        semantic_version: Option<u64>,
    },
    Known {
        layer: &'static str,
        version: Option<u64>,
        storage_version: Option<u64>,
    },
    Migration {
        layer: &'static str,
        from: u64,
        to: u64,
        storage_version: Option<u64>,
    },
}

impl FormatFacts {
    fn as_json(self) -> Value {
        match self {
            Self::Unknown => json!({
                "layer": null,
                "version": null,
            }),
            Self::Supported {
                storage_version,
                semantic_version,
            } => json!({
                "layer": "storage-and-semantic",
                "storage_version": storage_version,
                "semantic_version": semantic_version,
            }),
            Self::KnownVersions {
                storage_version,
                semantic_version,
            } => {
                let Some(layer) = (match (storage_version, semantic_version) {
                    (Some(_), Some(_)) => Some("storage-and-semantic"),
                    (Some(_), None) => Some("storage"),
                    (None, Some(_)) => Some("semantic"),
                    (None, None) => None,
                }) else {
                    return json!({
                        "layer": null,
                        "version": null,
                    });
                };
                json!({
                    "layer": layer,
                    "storage_version": storage_version,
                    "semantic_version": semantic_version,
                })
            }
            Self::Known {
                layer,
                version,
                storage_version,
            } => match storage_version {
                Some(storage_version) => json!({
                    "layer": layer,
                    "version": version,
                    "storage_version": storage_version,
                }),
                None => json!({
                    "layer": layer,
                    "version": version,
                }),
            },
            Self::Migration {
                layer,
                from,
                to,
                storage_version,
            } => match storage_version {
                Some(storage_version) => json!({
                    "layer": layer,
                    "from": from,
                    "to": to,
                    "storage_version": storage_version,
                }),
                None => json!({
                    "layer": layer,
                    "from": from,
                    "to": to,
                }),
            },
        }
    }
}

struct InspectionResult {
    classification: &'static str,
    format: FormatFacts,
    manifest: Option<RecoveryManifest>,
    next_action: &'static str,
}

impl InspectionResult {
    fn from_inspection(inspection: RecoveryInspection) -> Self {
        match inspection {
            RecoveryInspection::Absent => Self {
                classification: "absent",
                format: FormatFacts::Unknown,
                manifest: None,
                next_action: "provide-artifact",
            },
            RecoveryInspection::Supported(manifest) => {
                let semantic_version = u64::from(manifest.schema.0);
                Self {
                    classification: "supported",
                    format: FormatFacts::Supported {
                        storage_version: CURRENT_RECOVERY_SQLITE_SCHEMA,
                        semantic_version,
                    },
                    manifest: Some(*manifest),
                    next_action: "none",
                }
            }
            RecoveryInspection::CorruptOrUnreadable {
                storage_version,
                semantic_version,
            } => Self {
                classification: "corrupt-or-unreadable",
                format: FormatFacts::KnownVersions {
                    storage_version,
                    semantic_version,
                },
                manifest: None,
                next_action: "inspect-recovery-options",
            },
            RecoveryInspection::Unsupported {
                layer,
                version,
                storage_version,
            } => Self {
                classification: "unsupported",
                format: FormatFacts::Known {
                    layer: format_layer(layer),
                    version,
                    storage_version,
                },
                manifest: None,
                next_action: "use-compatible-inspector",
            },
            RecoveryInspection::MigrationRequired {
                layer,
                from,
                to,
                storage_version,
            } => Self {
                classification: "migration-required",
                format: FormatFacts::Migration {
                    layer: format_layer(layer),
                    from,
                    to,
                    storage_version,
                },
                manifest: None,
                next_action: "review-migration",
            },
            RecoveryInspection::ReconciliationRequired => Self {
                classification: "reconciliation-required",
                format: FormatFacts::Unknown,
                manifest: None,
                next_action: "reconcile-before-use",
            },
        }
    }

    fn as_json(&self) -> Value {
        json!({
            "schema": RESULT_SCHEMA,
            "command": COMMAND,
            "outcome": "observed",
            "classification": self.classification,
            "format_claim": FORMAT_CLAIM,
            "experimental": true,
            "format": self.format.as_json(),
            "manifest": &self.manifest,
            "non_authorization": NON_AUTHORIZATION,
            "next_action": self.next_action,
        })
    }
}

fn format_layer(layer: RecoveryFormatLayer) -> &'static str {
    match layer {
        RecoveryFormatLayer::Storage => "storage",
        RecoveryFormatLayer::Semantic => "semantic",
    }
}

fn parse_artifact() -> Result<(bool, PathBuf), &'static str> {
    let mut arguments = env::args_os().skip(1);
    let first = arguments
        .next()
        .ok_or("usage: dwv-recovery-inspect [--json] <artifact>")?;
    let (json_mode, artifact) = if first == OsStr::new("--json") {
        let artifact = arguments
            .next()
            .ok_or("usage: dwv-recovery-inspect [--json] <artifact>")?;
        (true, artifact)
    } else {
        (false, first)
    };
    if arguments.next().is_some() {
        return Err("usage: dwv-recovery-inspect [--json] <artifact>");
    }
    let artifact = PathBuf::from(artifact);
    if artifact.as_os_str().is_empty() {
        return Err("artifact path must not be empty");
    }
    Ok((json_mode, artifact))
}

fn write_json(result: &InspectionResult) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(&result.as_json())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if bytes.len() > MAX_RESULT_JSON_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "inspection result exceeds output bound",
        ));
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(&bytes)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

fn write_human(result: &InspectionResult) -> io::Result<()> {
    let value = result.as_json();
    let format = serde_json::to_string(
        value
            .get("format")
            .expect("inspection result always contains format facts"),
    )
    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let manifest = serde_json::to_string(
        value
            .get("manifest")
            .expect("inspection result always contains manifest field"),
    )
    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let output = format!(
        "schema: {RESULT_SCHEMA}\ncommand: {COMMAND}\noutcome: observed\nclassification: {}\nformat_claim: {FORMAT_CLAIM}\nexperimental: true\nformat: {format}\nmanifest: {manifest}\nnon_authorization: {NON_AUTHORIZATION}\nnext_action: {}\n",
        result.classification, result.next_action
    );
    if output.len() > MAX_RESULT_JSON_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "inspection result exceeds output bound",
        ));
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(output.as_bytes())?;
    stdout.flush()
}

/// dwv:req req.independent-recovery-inspection.independent-recovery-state-inspection-is-bounded-and-non-authorizing
fn run() -> ExitCode {
    let (json_mode, artifact) = match parse_artifact() {
        Ok(arguments) => arguments,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(USAGE_EXIT_CODE);
        }
    };
    let result = InspectionResult::from_inspection(SqliteRecoveryStore::inspect(artifact));
    let rendered = if json_mode {
        write_json(&result)
    } else {
        write_human(&result)
    };
    match rendered {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{COMMAND}: unable to render inspection result: {error}");
            ExitCode::from(OPERATIONAL_EXIT_CODE)
        }
    }
}

fn main() -> ExitCode {
    run()
}
