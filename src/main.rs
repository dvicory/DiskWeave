mod demo;

use serde_json::json;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.get(1).and_then(|frontend| {
        args.get(2)
            .map(|command| format!("demo.{frontend}.{command}"))
    });
    match dispatch(args) {
        Ok(value) => {
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema": "dwv.cli.v0",
                    "contract": "dwv.cli.v0",
                    "command": command,
                    "ok": false,
                    "outcome": error.class,
                    "diagnostics": [],
                    "error": {
                        "class": error.class,
                        "code": error.code,
                        "message": error.message,
                    }
                }))
                .unwrap()
            );
            ExitCode::from(error.code)
        }
    }
}

fn dispatch(args: Vec<String>) -> Result<serde_json::Value, demo::DemoError> {
    if args.first().map(String::as_str) != Some("demo") {
        return Err(demo::DemoError::usage(
            "usage: dwv demo <file|disk> <command> [options]",
        ));
    }
    match args.get(1).map(String::as_str) {
        Some("file") => dispatch_file(&args[2..]),
        Some("disk") => dispatch_disk(&args[2..]),
        Some(frontend) => Err(demo::DemoError::usage(format!(
            "unknown demo frontend: {frontend}"
        ))),
        None => Err(demo::DemoError::usage("missing demo frontend")),
    }
}

fn dispatch_file(args: &[String]) -> Result<serde_json::Value, demo::DemoError> {
    let command = args
        .first()
        .ok_or_else(|| demo::DemoError::usage("missing file demo command"))?
        .as_str();
    let mut root = None;
    let mut size = None;
    let mut plan = PathBuf::from("rebuild-plan.json");
    let mut confirmation = None;
    let mut stop_after = None;
    let mut scrub_plan = PathBuf::from("scrub-plan.json");
    let mut trace = PathBuf::from("trace.json");
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                index += 1;
                root =
                    Some(PathBuf::from(args.get(index).ok_or_else(|| {
                        demo::DemoError::usage("--root requires a path")
                    })?));
            }
            "--size" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| demo::DemoError::usage("--size requires a byte count"))?;
                size =
                    Some(value.parse().map_err(|_| {
                        demo::DemoError::usage("--size must be an unsigned integer")
                    })?);
            }
            "--plan" => {
                index += 1;
                plan = PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| demo::DemoError::usage("--plan requires a path"))?,
                );
            }
            "--scrub-plan" => {
                index += 1;
                scrub_plan = PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| demo::DemoError::usage("--scrub-plan requires a path"))?,
                );
            }
            "--trace" => {
                index += 1;
                trace = PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| demo::DemoError::usage("--trace requires a path"))?,
                );
            }
            "--confirm" => {
                index += 1;
                confirmation = Some(
                    args.get(index)
                        .ok_or_else(|| demo::DemoError::usage("--confirm requires the plan token"))?
                        .clone(),
                );
            }
            "--stop-after" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| demo::DemoError::usage("--stop-after requires a chunk count"))?;
                stop_after = Some(value.parse().map_err(|_| {
                    demo::DemoError::usage("--stop-after must be an unsigned integer")
                })?);
            }
            option if option.starts_with('-') => {
                return Err(demo::DemoError::usage(format!("unknown option: {option}")));
            }
            value => {
                return Err(demo::DemoError::usage(format!(
                    "unexpected argument: {value}"
                )));
            }
        }
        index += 1;
    }
    let root = root.ok_or_else(|| demo::DemoError::usage("--root is required"))?;
    let result = match command {
        "init" => demo::init(&root, size),
        "run" => demo::run(&root),
        "status" => demo::status(&root),
        "inspect" => demo::inspect(&root),
        "capabilities" => demo::capabilities(&root),
        "verify" => demo::verify(&root),
        "scrub" => demo::scrub(&root),
        "repair" => demo::repair(
            &root,
            &scrub_plan,
            confirmation
                .as_deref()
                .ok_or_else(|| demo::DemoError::usage("repair requires --confirm TOKEN"))?,
        ),
        "plan" => demo::plan(&root, &plan),
        "rebuild" => demo::execute(
            &root,
            &plan,
            confirmation
                .as_deref()
                .ok_or_else(|| demo::DemoError::usage("rebuild requires --confirm TOKEN"))?,
            stop_after,
        ),
        "trace-export" => demo::trace_export(&root, &trace),
        "trace-render" => demo::trace_render(&root, &trace),
        "trace-replay" => demo::trace_replay(&root, &trace),
        _ => Err(demo::DemoError::usage(format!(
            "unknown demo command: {command}"
        ))),
    }?;
    let mut result = result;
    result["schema"] = serde_json::json!("dwv.cli.v0");
    result["contract"] = serde_json::json!("dwv.cli.v0");
    result["command"] = serde_json::Value::String(format!("demo.file.{command}"));
    result["ok"] = serde_json::json!(true);
    result["outcome"] = serde_json::json!("success");
    result["diagnostics"] = serde_json::json!([]);
    Ok(result)
}

fn dispatch_disk(args: &[String]) -> Result<serde_json::Value, demo::DemoError> {
    let command = args
        .first()
        .ok_or_else(|| {
            demo::DemoError::usage(
                "usage: dwv demo disk <probe|init|serve|inspect|cleanup|trace-replay> [options]",
            )
        })?
        .as_str();
    let mut root = None;
    let mut size = 64 * 1024 * 1024_u64;
    let mut device_id = -1_i32;
    let mut trace = PathBuf::from("trace.json");
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                index += 1;
                root =
                    Some(PathBuf::from(args.get(index).ok_or_else(|| {
                        demo::DemoError::usage("--root requires a path")
                    })?));
            }
            "--size" => {
                index += 1;
                size = args
                    .get(index)
                    .ok_or_else(|| demo::DemoError::usage("--size requires a byte count"))?
                    .parse()
                    .map_err(|_| demo::DemoError::usage("--size must be an unsigned integer"))?;
            }
            "--device-id" => {
                index += 1;
                device_id = args
                    .get(index)
                    .ok_or_else(|| demo::DemoError::usage("--device-id requires an integer"))?
                    .parse()
                    .map_err(|_| demo::DemoError::usage("--device-id must be an integer"))?;
            }
            "--trace" => {
                index += 1;
                trace = PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| demo::DemoError::usage("--trace requires a path"))?,
                );
            }
            option if option.starts_with('-') => {
                return Err(demo::DemoError::usage(format!("unknown option: {option}")));
            }
            value => {
                return Err(demo::DemoError::usage(format!(
                    "unexpected argument: {value}"
                )));
            }
        }
        index += 1;
    }

    let value = match command {
        "probe" => serde_json::to_value(dwv_frontend_ublk::probe())
            .map_err(|error| ublk_error(dwv_frontend_ublk::AdapterError::Io(error.to_string())))?,
        "init" => serde_json::to_value(
            dwv_frontend_ublk::initialize(required_ublk_root(root.as_ref())?, size)
                .map_err(ublk_error)?,
        )
        .map_err(|error| ublk_error(dwv_frontend_ublk::AdapterError::Io(error.to_string())))?,
        "serve" => dwv_frontend_ublk::serve(required_ublk_root(root.as_ref())?, device_id)
            .map_err(ublk_error)?,
        "inspect" => serde_json::to_value(
            dwv_frontend_ublk::inspect(required_ublk_root(root.as_ref())?).map_err(ublk_error)?,
        )
        .map_err(|error| ublk_error(dwv_frontend_ublk::AdapterError::Io(error.to_string())))?,
        "cleanup" => {
            let device_id = u32::try_from(device_id)
                .map_err(|_| demo::DemoError::usage("cleanup requires --device-id N"))?;
            dwv_frontend_ublk::cleanup(required_ublk_root(root.as_ref())?, device_id)
                .map_err(ublk_error)?
        }
        "trace-replay" => {
            serde_json::to_value(dwv_frontend_ublk::replay_trace_file(&trace).map_err(ublk_error)?)
                .map_err(|error| {
                    ublk_error(dwv_frontend_ublk::AdapterError::Io(error.to_string()))
                })?
        }
        _ => {
            return Err(demo::DemoError::usage(format!(
                "unknown disk demo command: {command}"
            )));
        }
    };
    let mut value = value;
    value["schema"] = json!("dwv.cli.v0");
    value["contract"] = json!("dwv.cli.v0");
    value["command"] = json!(format!("demo.disk.{command}"));
    value["ok"] = json!(true);
    value["outcome"] = json!("success");
    value["diagnostics"] = json!([]);
    Ok(value)
}

fn required_ublk_root(root: Option<&PathBuf>) -> Result<&std::path::Path, demo::DemoError> {
    root.map(PathBuf::as_path)
        .ok_or_else(|| demo::DemoError::usage("--root is required"))
}

fn ublk_error(error: dwv_frontend_ublk::AdapterError) -> demo::DemoError {
    let (code, class) = match error.terminal() {
        dwv_frontend_ublk::TerminalResult::Invalid => (2, "usage"),
        dwv_frontend_ublk::TerminalResult::Unsupported => (3, "unsupported"),
        dwv_frontend_ublk::TerminalResult::ResourceExhausted => (4, "resource-exhausted"),
        dwv_frontend_ublk::TerminalResult::ReconciliationRequired => (5, "reconciliation-required"),
        dwv_frontend_ublk::TerminalResult::Io => (5, "operation-failed"),
        dwv_frontend_ublk::TerminalResult::Success => (0, "success"),
    };
    demo::DemoError {
        code,
        class,
        message: error.to_string(),
    }
}
