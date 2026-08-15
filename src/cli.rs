use crate::array;
use crate::demo::DemoError;
use crate::operator::{
    OperatorError, OperatorResult, Outcome, baseline, damage, members_result, observe,
    operation_error_result, recover_apply, recover_preview, scrub, start,
};
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "dwv",
    version,
    about = "DiskWeave operator and acceptance commands"
)]
pub struct Cli {
    #[command(subcommand)]
    command: TopCommand,
}

#[derive(Debug, Subcommand)]
enum TopCommand {
    /// Inspect current array state without mutation.
    Status(ArrayArgs),
    /// Report member recognition and identity state.
    Members(ArrayArgs),
    /// Exhaustively verify current parity equations without writes.
    Scrub(ArrayArgs),
    /// Report exact current damage classifications without writes.
    Damage(ArrayArgs),
    /// Preview or apply currently supported metadata-loss recovery.
    Recover(RecoverArgs),
    /// Continue the mandatory checksum baseline.
    Baseline(BaselineArgs),
    /// Admit and publish the array through the supported frontend.
    Start(StartArgs),
    /// Disposable acceptance workflows.
    Demo(DemoArgs),
}

#[derive(Debug, Args)]
struct ArrayArgs {
    /// Declarative array policy file.
    #[arg(long, value_name = "PATH")]
    array: PathBuf,
    /// Render the versioned JSON result instead of human output.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct RecoverArgs {
    #[command(flatten)]
    array: ArrayArgs,
    /// Apply the current previewed proposal.
    #[arg(long, value_name = "PLAN-ID")]
    apply: Option<String>,
}

#[derive(Debug, Args)]
struct BaselineArgs {
    #[command(flatten)]
    array: ArrayArgs,
    /// Bound work for deterministic interruption evidence.
    #[arg(long, value_name = "N")]
    max_extents: Option<usize>,
}

#[derive(Debug, Args)]
struct StartArgs {
    #[command(flatten)]
    array: ArrayArgs,
    /// Request read-only publication.
    #[arg(long)]
    read_only: bool,
    /// Request a specific ublk device ID.
    #[arg(long, value_name = "N")]
    device_id: Option<u32>,
}

#[derive(Debug, Args)]
struct DemoArgs {
    #[command(subcommand)]
    frontend: DemoFrontend,
}

#[derive(Debug, Subcommand)]
enum DemoFrontend {
    File(DemoFileCommand),
    Disk(DemoDiskCommand),
}

#[derive(Debug, Args)]
struct DemoFileCommand {
    #[command(subcommand)]
    command: DemoFileOperation,
}

#[derive(Debug, Subcommand)]
enum DemoFileOperation {
    Init(DemoInitArgs),
    Run(CommonRootArgs),
    Status(CommonRootArgs),
    Inspect(CommonRootArgs),
    Capabilities(CommonRootArgs),
    Verify(CommonRootArgs),
    Scrub(CommonRootArgs),
    Policy(CommonRootArgs),
    Repair(RepairArgs),
    Plan(PlanArgs),
    Rebuild(RebuildArgs),
    TraceExport(TraceArgs),
    TraceRender(TraceArgs),
    TraceReplay(TraceArgs),
}

#[derive(Debug, Args)]
struct CommonRootArgs {
    #[arg(long, value_name = "PATH")]
    root: PathBuf,
}

#[derive(Debug, Args)]
struct DemoInitArgs {
    #[arg(long, value_name = "PATH")]
    root: PathBuf,
    #[arg(long, value_name = "BYTES")]
    size: Option<u64>,
}

#[derive(Debug, Args)]
struct PlanArgs {
    #[arg(long, value_name = "PATH")]
    root: PathBuf,
    #[arg(long, value_name = "PATH", default_value = "rebuild-plan.json")]
    plan: PathBuf,
}

#[derive(Debug, Args)]
struct RepairArgs {
    #[arg(long, value_name = "PATH")]
    root: PathBuf,
    #[arg(long, value_name = "PATH", default_value = "scrub-plan.json")]
    scrub_plan: PathBuf,
    #[arg(long, value_name = "TOKEN")]
    confirm: String,
}

#[derive(Debug, Args)]
struct RebuildArgs {
    #[arg(long, value_name = "PATH")]
    root: PathBuf,
    #[arg(long, value_name = "PATH", default_value = "rebuild-plan.json")]
    plan: PathBuf,
    #[arg(long, value_name = "TOKEN")]
    confirm: String,
    #[arg(long, value_name = "N")]
    stop_after: Option<u64>,
}

#[derive(Debug, Args)]
struct TraceArgs {
    #[arg(long, value_name = "PATH")]
    root: PathBuf,
    #[arg(long, value_name = "PATH", default_value = "trace.json")]
    trace: PathBuf,
}

#[derive(Debug, Args)]
struct DemoDiskCommand {
    #[command(subcommand)]
    command: DemoDiskOperation,
}

#[derive(Debug, Subcommand)]
enum DemoDiskOperation {
    Probe,
    Init(DiskInitArgs),
    Serve(DiskServeArgs),
    Inspect(DiskRootArgs),
    Cleanup(DiskCleanupArgs),
    TraceReplay(DiskTraceArgs),
    Policy(DiskRootArgs),
}

#[derive(Debug, Args)]
struct DiskRootArgs {
    #[arg(long, value_name = "PATH")]
    root: PathBuf,
}

#[derive(Debug, Args)]
struct DiskInitArgs {
    #[arg(long, value_name = "PATH")]
    root: PathBuf,
    #[arg(long, value_name = "BYTES", default_value_t = 64 * 1024 * 1024)]
    size: u64,
}

#[derive(Debug, Args)]
struct DiskServeArgs {
    #[arg(long, value_name = "PATH")]
    root: PathBuf,
    #[arg(long, value_name = "N", default_value_t = -1)]
    device_id: i32,
}

#[derive(Debug, Args)]
struct DiskCleanupArgs {
    #[arg(long, value_name = "PATH")]
    root: PathBuf,
    #[arg(long, value_name = "N")]
    device_id: u32,
}

#[derive(Debug, Args)]
struct DiskTraceArgs {
    #[arg(long, value_name = "PATH", default_value = "trace.json")]
    trace: PathBuf,
}

pub fn run() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let kind = match error.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    Outcome::Success
                }
                _ => Outcome::Usage,
            };
            let output = error.to_string();
            if kind == Outcome::Success {
                print!("{output}");
            } else {
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "schema": "dwv.cli.v0",
                        "contract": "dwv.cli.v0",
                        "ok": false,
                        "kind": "cli-error",
                        "outcome": kind,
                        "diagnostics": [],
                        "error": {
                            "class": kind,
                            "reason_code": "usage-error",
                            "message": output,
                        }
                    }))
                    .expect("CLI parse-error JSON serialization cannot fail")
                );
            }
            return ExitCode::from(kind.exit_code());
        }
    };
    match execute(cli) {
        Ok(CliExit {
            json,
            result,
            already_printed,
        }) => {
            if already_printed {
                return ExitCode::from(result.outcome.exit_code());
            }
            render_result(json, &result);
            ExitCode::from(result.outcome.exit_code())
        }
        Err(error) => {
            let outcome = error.outcome();
            let exit_code = match &error {
                OperatorError::LegacyFailed { exit_code, .. } => *exit_code,
                _ => outcome.exit_code(),
            };
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema": "dwv.cli.v0",
                    "contract": "dwv.cli.v0",
                    "ok": false,
                    "kind": "cli-error",
                    "outcome": outcome,
                    "diagnostics": [],
                    "error": {
                        "class": outcome,
                        "reason_code": error.reason_code(),
                        "message": error.message(),
                    }
                }))
                .expect("CLI error JSON serialization cannot fail")
            );
            ExitCode::from(exit_code)
        }
    }
}

struct CliExit {
    json: bool,
    result: OperatorResult,
    already_printed: bool,
}

fn execute(cli: Cli) -> Result<CliExit, OperatorError> {
    match cli.command {
        TopCommand::Status(args) => {
            policy_result(args, "status", |policy| Ok(observe(policy, "status")))
        }
        TopCommand::Members(args) => {
            policy_result(args, "members", |policy| Ok(members_result(policy)))
        }
        TopCommand::Scrub(args) => policy_result(args, "scrub", scrub),
        TopCommand::Damage(args) => policy_result(args, "damage", damage),
        TopCommand::Recover(args) => {
            let json = args.array.json;
            let policy = load_policy(&args.array.array)?;
            let operation = if let Some(plan_id) = args.apply {
                recover_apply(&policy, &plan_id)
            } else {
                recover_preview(&policy)
            };
            Ok(CliExit {
                json,
                result: operation
                    .unwrap_or_else(|error| operation_error_result(&policy, "recover", error)),
                already_printed: false,
            })
        }
        TopCommand::Baseline(args) => {
            let json = args.array.json;
            let policy = load_policy(&args.array.array)?;
            Ok(CliExit {
                json,
                result: baseline(&policy, args.max_extents)
                    .unwrap_or_else(|error| operation_error_result(&policy, "baseline", error)),
                already_printed: false,
            })
        }
        TopCommand::Start(args) => {
            let json = args.array.json;
            let policy = load_policy(&args.array.array)?;
            let started = start(&policy, args.read_only, args.device_id, move |result| {
                render_result(json, &result)
            });
            let (result, published) = started
                .unwrap_or_else(|error| (operation_error_result(&policy, "start", error), false));
            Ok(CliExit {
                json,
                result,
                already_printed: published,
            })
        }
        TopCommand::Demo(args) => demo_result(args),
    }
}

fn policy_result(
    args: ArrayArgs,
    command: &'static str,
    operation: impl FnOnce(&array::ArrayPolicy) -> Result<OperatorResult, OperatorError>,
) -> Result<CliExit, OperatorError> {
    let policy = load_policy(&args.array)?;
    let result =
        operation(&policy).unwrap_or_else(|error| operation_error_result(&policy, command, error));
    Ok(CliExit {
        json: args.json,
        result,
        already_printed: false,
    })
}

fn load_policy(path: &std::path::Path) -> Result<array::ArrayPolicy, OperatorError> {
    array::load(path).map_err(|error| match error {
        array::PolicyError::Io(message) => OperatorError::Blocked(message),
        array::PolicyError::Parse(message) | array::PolicyError::Invalid(message) => {
            OperatorError::Usage(message)
        }
    })
}

fn demo_result(args: DemoArgs) -> Result<CliExit, OperatorError> {
    match dispatch_demo(args) {
        Ok((command, value)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("demo JSON serialization cannot fail")
            );
            let mut result = OperatorResult::new(
                command,
                Outcome::Success,
                "demo-command-succeeded",
                "demo command succeeded",
            );
            result.recovery = crate::operator::RecoveryAssessment::Unavailable {
                reason: "demo result was rendered through the shared CLI boundary".into(),
            };
            Ok(CliExit {
                json: false,
                result,
                already_printed: true,
            })
        }
        Err(error) => Err(match error.class {
            "usage" => OperatorError::Usage(error.message),
            "blocked" => OperatorError::Blocked(error.message),
            "unsupported" => OperatorError::Unsupported(error.message),
            "refused" => OperatorError::Invalid(error.message),
            "reconciliation-required" => OperatorError::Reconciliation(error.message),
            _ => OperatorError::LegacyFailed {
                message: error.message,
                exit_code: error.code,
            },
        }),
    }
}

fn dispatch_demo(args: DemoArgs) -> Result<(&'static str, serde_json::Value), DemoError> {
    let (command, value) = match args.frontend {
        DemoFrontend::File(command) => match command.command {
            DemoFileOperation::Init(args) => {
                ("demo.file.init", crate::demo::init(&args.root, args.size)?)
            }
            DemoFileOperation::Run(args) => ("demo.file.run", crate::demo::run(&args.root)?),
            DemoFileOperation::Status(args) => {
                ("demo.file.status", crate::demo::status(&args.root)?)
            }
            DemoFileOperation::Inspect(args) => {
                ("demo.file.inspect", crate::demo::inspect(&args.root)?)
            }
            DemoFileOperation::Capabilities(args) => (
                "demo.file.capabilities",
                crate::demo::capabilities(&args.root)?,
            ),
            DemoFileOperation::Verify(args) => {
                ("demo.file.verify", crate::demo::verify(&args.root)?)
            }
            DemoFileOperation::Scrub(args) => ("demo.file.scrub", crate::demo::scrub(&args.root)?),
            DemoFileOperation::Policy(args) => (
                "demo.file.policy",
                crate::demo::write_array_policy(&args.root)
                    .map(|()| json!({ "policy": "array.json" }))?,
            ),
            DemoFileOperation::Repair(args) => (
                "demo.file.repair",
                crate::demo::repair(&args.root, &args.scrub_plan, &args.confirm)?,
            ),
            DemoFileOperation::Plan(args) => {
                ("demo.file.plan", crate::demo::plan(&args.root, &args.plan)?)
            }
            DemoFileOperation::Rebuild(args) => (
                "demo.file.rebuild",
                crate::demo::execute(&args.root, &args.plan, &args.confirm, args.stop_after)?,
            ),
            DemoFileOperation::TraceExport(args) => (
                "demo.file.trace-export",
                crate::demo::trace_export(&args.root, &args.trace)?,
            ),
            DemoFileOperation::TraceRender(args) => (
                "demo.file.trace-render",
                crate::demo::trace_render(&args.root, &args.trace)?,
            ),
            DemoFileOperation::TraceReplay(args) => (
                "demo.file.trace-replay",
                crate::demo::trace_replay(&args.root, &args.trace)?,
            ),
        },
        DemoFrontend::Disk(command) => match command.command {
            DemoDiskOperation::Probe => (
                "demo.disk.probe",
                serde_json::to_value(dwv_frontend_ublk::probe()).map_err(|error| {
                    ublk_error(dwv_frontend_ublk::AdapterError::Io(error.to_string()))
                })?,
            ),
            DemoDiskOperation::Init(args) => (
                "demo.disk.init",
                serde_json::to_value(
                    dwv_frontend_ublk::initialize(&args.root, args.size).map_err(ublk_error)?,
                )
                .map_err(|error| {
                    ublk_error(dwv_frontend_ublk::AdapterError::Io(error.to_string()))
                })?,
            ),
            DemoDiskOperation::Serve(args) => (
                "demo.disk.serve",
                dwv_frontend_ublk::serve(&args.root, args.device_id).map_err(ublk_error)?,
            ),
            DemoDiskOperation::Inspect(args) => (
                "demo.disk.inspect",
                serde_json::to_value(dwv_frontend_ublk::inspect(&args.root).map_err(ublk_error)?)
                    .map_err(|error| {
                    ublk_error(dwv_frontend_ublk::AdapterError::Io(error.to_string()))
                })?,
            ),
            DemoDiskOperation::Cleanup(args) => (
                "demo.disk.cleanup",
                dwv_frontend_ublk::cleanup(&args.root, args.device_id).map_err(ublk_error)?,
            ),
            DemoDiskOperation::TraceReplay(args) => (
                "demo.disk.trace-replay",
                serde_json::to_value(
                    dwv_frontend_ublk::replay_trace_file(&args.trace).map_err(ublk_error)?,
                )
                .map_err(|error| {
                    ublk_error(dwv_frontend_ublk::AdapterError::Io(error.to_string()))
                })?,
            ),
            DemoDiskOperation::Policy(args) => (
                "demo.disk.policy",
                dwv_frontend_ublk::array_policy(&args.root).map_err(ublk_error)?,
            ),
        },
    };
    let mut value = value;
    value["schema"] = json!("dwv.cli.v0");
    value["contract"] = json!("dwv.cli.v0");
    value["command"] = json!(command);
    value["ok"] = json!(true);
    value["outcome"] = json!("success");
    value["diagnostics"] = json!([]);
    Ok((command, value))
}

fn ublk_error(error: dwv_frontend_ublk::AdapterError) -> DemoError {
    let (code, class) = match error.terminal() {
        dwv_frontend_ublk::TerminalResult::Invalid => (2, "usage"),
        dwv_frontend_ublk::TerminalResult::Unsupported => (3, "unsupported"),
        dwv_frontend_ublk::TerminalResult::ResourceExhausted => (4, "resource-exhausted"),
        dwv_frontend_ublk::TerminalResult::ReconciliationRequired => (5, "reconciliation-required"),
        dwv_frontend_ublk::TerminalResult::Io => (5, "operation-failed"),
        dwv_frontend_ublk::TerminalResult::Success => (0, "success"),
    };
    DemoError {
        code,
        class,
        message: error.to_string(),
    }
}

fn render_result(json: bool, result: &OperatorResult) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(result).expect("operator JSON serialization cannot fail")
        );
    } else {
        print_human(result);
    }
}

fn print_human(result: &OperatorResult) {
    println!("schema: {}", result.schema);
    println!("{}: {}", result.command, result.reason);
    println!("kind: {}", result.kind);
    println!("reason-code: {}", result.reason_code);
    println!("outcome: {:?}", result.outcome);
    println!("state: {}", result.lifecycle);
    println!("access: {}", result.access);
    println!("start: {}", result.start);
    println!("parity: {}", result.parity);
    println!("damage: {}", result.damage);
    println!("redundancy: {}", result.redundancy);
    println!("next: {}", result.next_action);
    println!(
        "lineage: {:?} ({})",
        result.authority.lineage.disposition, result.authority.lineage.reason
    );
    println!(
        "custody: {:?} ({})",
        result.authority.custody.disposition, result.authority.custody.reason
    );
    println!("authority-blocker: {}", result.authority.blocker);
    println!("authorization: {}", result.authority.authorization);
    for coverage in &result.authority.protection_basis {
        println!(
            "basis: role={} state={:?} ranges={} bytes={} detail-truncated={}",
            coverage.role,
            coverage.basis,
            coverage.range_count,
            coverage.bytes,
            coverage.detail_truncated
        );
        for range in &coverage.exact_ranges {
            println!(
                "  basis-range: role={} basis={:?} offset={} length={}",
                coverage.role, coverage.basis, range.offset, range.length
            );
        }
    }
    println!(
        "checksums: {}",
        serde_json::to_string(&result.checksum).expect("checksum JSON cannot fail")
    );
    println!(
        "publication: {}",
        serde_json::to_string(&result.publication).expect("publication JSON cannot fail")
    );
    println!(
        "recovery: {}",
        serde_json::to_string(&result.recovery).expect("recovery JSON cannot fail")
    );
    if let Some(topology) = &result.topology {
        println!(
            "topology: array={} epoch={} data={} parity={} protected={}",
            topology.array_id,
            topology.topology_epoch,
            topology.data_slots,
            topology.parity_slots,
            topology.protected_length
        );
    }
    if !result.members.is_empty() {
        println!("members:");
        for member in &result.members {
            println!(
                "  {} role={} status={} expected={} observed={}",
                member.path.display(),
                member.role,
                member.status,
                member.expected_identity,
                member.observed_identity.as_deref().unwrap_or("unreadable")
            );
        }
    }
    if let Some(verification) = &result.verification {
        println!(
            "verification: mode={} complete={} matching={}/{}",
            verification.mode,
            verification.complete,
            verification.matching_regions,
            verification.regions.len()
        );
        for region in &verification.regions {
            println!(
                "  verification-region: offset={} length={} disposition={} data-evidence={} parity-evidence={}",
                region.offset,
                region.length,
                region.disposition,
                serde_json::to_string(&region.data_evidence)
                    .expect("verification data evidence JSON cannot fail"),
                region.parity_evidence
            );
        }
    }
    if let Some(plan) = &result.recovery_plan {
        println!(
            "recovery-plan: id={} case={} action={} executable={}",
            plan.plan_id, plan.case, plan.action, plan.executable
        );
        for missing in &plan.missing {
            println!("  missing: {missing}");
        }
    }
}
