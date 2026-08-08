mod demo;

use serde_json::json;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.get(1).map(|name| format!("demo.{name}"));
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
            "usage: dwv demo <init|run|status|inspect|capabilities|verify|scrub|repair|plan|rebuild|trace-export|trace-render|trace-replay> --root PATH",
        ));
    }
    let command = args
        .get(1)
        .ok_or_else(|| demo::DemoError::usage("missing demo command"))?
        .as_str();
    let mut root = None;
    let mut size = None;
    let mut plan = PathBuf::from("rebuild-plan.json");
    let mut confirmation = None;
    let mut stop_after = None;
    let mut scrub_plan = PathBuf::from("scrub-plan.json");
    let mut trace = PathBuf::from("trace.json");
    let mut index = 2;
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
    result["command"] = serde_json::Value::String(format!("demo.{command}"));
    result["ok"] = serde_json::json!(true);
    result["outcome"] = serde_json::json!("success");
    result["diagnostics"] = serde_json::json!([]);
    Ok(result)
}
