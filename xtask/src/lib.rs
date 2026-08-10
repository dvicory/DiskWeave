mod knowledge;

use blake3::Hash;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::env;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

const CLI_SCHEMA: &str = "dwv.docs.cli.v1";
const INVENTORY_SCHEMA: &str = "dwv.docs.source-inventory.v1";

#[derive(Debug)]
pub struct AppError {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) details: Option<Value>,
}

impl AppError {
    pub(crate) fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub(crate) fn details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Bounds {
    pub(crate) max_source_bytes: usize,
    pub(crate) max_context_bytes: usize,
    pub(crate) max_units: usize,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            max_source_bytes: 262_144,
            max_context_bytes: 65_536,
            max_units: 512,
        }
    }
}

pub(crate) struct App {
    pub(crate) root: PathBuf,
    pub(crate) bounds: Bounds,
}

impl App {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            bounds: Bounds::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Heading {
    pub(crate) level: u8,
    pub(crate) title: String,
    pub(crate) path: Vec<String>,
    pub(crate) start: usize,
}

pub fn run_cli(args: Vec<String>) -> ExitCode {
    let command = args.get(1).cloned().unwrap_or_else(|| "help".to_owned());
    match dispatch(&args) {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema": CLI_SCHEMA, "command": command, "ok": true,
                    "outcome": "success", "diagnostics": [], "result": result,
                }))
                .expect("CLI result serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema": CLI_SCHEMA, "command": command, "ok": false,
                    "outcome": "error", "error": {"code": error.code, "message": error.message},
                    "details": error.details,
                }))
                .expect("CLI error serializes")
            );
            ExitCode::FAILURE
        }
    }
}

fn dispatch(args: &[String]) -> Result<Value, AppError> {
    if args.first().map(String::as_str) != Some("docs") {
        return Err(AppError::new("usage", "cargo xtask docs <operation>"));
    }
    let operation = args.get(1).map(String::as_str).unwrap_or("help");
    if operation == "help" {
        return Ok(help_value());
    }
    if operation == "schema" {
        return Ok(schema_value());
    }

    let app = App::new(workspace_root());
    match operation {
        "knowledge" => dispatch_knowledge(&app, &args[2..]),
        "inspect" | "context" | "trace" | "why" | "ownership" | "affected" | "readiness" => {
            dispatch_knowledge(&app, args.get(1..).unwrap_or_default())
        }
        "doctor" => {
            let objects = knowledge::objects(&app)?;
            Ok(json!({"config": "valid", "requirements": objects.len(), "network": "not used"}))
        }
        "extract" => knowledge::export(&app),
        "check" => {
            let impact = knowledge::change_impact(&app, option_value(args, "--base").as_deref())?;
            match knowledge::readiness(&app) {
                Ok(mut readiness) => {
                    readiness["change_impact"] = impact;
                    Ok(readiness)
                }
                Err(mut error) => {
                    let mut details = error.details.take().unwrap_or_else(|| json!({}));
                    details["change_impact"] = impact;
                    error.details = Some(details);
                    Err(error)
                }
            }
        }
        "build" => {
            knowledge::export(&app)?;
            run_sphinx(&app.root, "build")
        }
        "serve" => {
            knowledge::export(&app)?;
            run_sphinx(&app.root, "generate")?;
            let output = Command::new("mise")
                .args([
                    "exec",
                    "--",
                    "uv",
                    "run",
                    "--offline",
                    "--no-project",
                    "--python",
                    "3.12.13",
                    "sphinx-autobuild",
                    "target/dwv-docs/sphinx/source",
                    "target/dwv-docs/sphinx/html",
                ])
                .current_dir(&app.root)
                .status()
                .map_err(|e| AppError::new("sphinx_serve_unavailable", e.to_string()))?;
            if !output.success() {
                return Err(AppError::new("sphinx_serve_failed", output.to_string()));
            }
            Ok(json!({"served": true}))
        }
        "clean-room" => clean_room(&app),
        _ => Err(AppError::new(
            "usage",
            format!("unknown docs operation {operation}"),
        )),
    }
}

fn dispatch_knowledge(app: &App, args: &[String]) -> Result<Value, AppError> {
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "help" => Ok(help_value()),
        "export" | "extract" | "export-sphinx" => knowledge::export(app),
        "inspect" => knowledge::inspect(app, required_id(args, "inspect")?),
        "context" | "trace" | "why" => knowledge::context(app, required_id(args, command)?),
        "ownership" => knowledge::ownership(app, required_id(args, "ownership")?),
        "affected" => {
            let mut ids = option_values(args, "--id");
            if let Some(id) = args.get(1).filter(|value| !value.starts_with("--")) {
                ids.push(id.clone());
            }
            knowledge::affected(app, ids, option_values(args, "--path"))
        }
        "doctor" => knowledge::doctor(app, option_values(args, "--path")),
        "readiness" | "check" => knowledge::readiness(app),
        "resolve" => {
            let id = required_id(args, "resolve")?;
            let outcome = option_value(args, "--outcome")
                .ok_or_else(|| AppError::new("usage", "resolve requires --outcome"))?;
            let reason = option_value(args, "--reason")
                .ok_or_else(|| AppError::new("usage", "resolve requires --reason"))?;
            knowledge::resolve(app, &id, &outcome, &reason)
        }
        _ => Err(AppError::new(
            "usage",
            format!("unknown knowledge operation {command}"),
        )),
    }
}

fn required_id(args: &[String], operation: &str) -> Result<String, AppError> {
    args.get(1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .ok_or_else(|| AppError::new("usage", format!("{operation} requires an ID")))
}

fn workspace_root() -> PathBuf {
    env::var_os("DWV_DOCS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("xtask is a workspace child")
                .to_path_buf()
        })
}
fn clean_room(app: &App) -> Result<Value, AppError> {
    knowledge::export(app)?;
    let expected = fs::read(app.root.join("target/dwv-docs/knowledge/objects.json"))
        .map_err(|error| AppError::new("clean_room_read_failed", error.to_string()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::new("clean_room_clock_failed", error.to_string()))?
        .as_nanos();
    let root = env::temp_dir().join(format!("diskweave-docs-{}-{nonce}", std::process::id()));
    fs::create_dir(&root)
        .map_err(|error| AppError::new("clean_room_create_failed", error.to_string()))?;
    let result = (|| {
        for relative in [
            "Cargo.toml",
            "Cargo.lock",
            ".cargo",
            "mise.toml",
            "src",
            "tests",
            "crates",
            "xtask",
            "tools/dwv-sphinx.py",
            "openspec/specs",
            "verification",
            "docs/sphinx",
            "docs/curriculum.toml",
            "docs/reviewed-requirements.toml",
            ".agents/skills/diskweave-knowledge",
        ] {
            copy_permanent_input(&app.root, &root, relative)?;
        }
        let output = Command::new("cargo")
            .args([
                "run",
                "--offline",
                "-q",
                "-p",
                "xtask",
                "--",
                "docs",
                "build",
            ])
            .current_dir(&root)
            .output()
            .map_err(|error| AppError::new("clean_room_build_unavailable", error.to_string()))?;
        if !output.status.success() {
            return Err(AppError::new(
                "clean_room_build_failed",
                format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }
        let actual = fs::read(root.join("target/dwv-docs/knowledge/objects.json"))
            .map_err(|error| AppError::new("clean_room_read_failed", error.to_string()))?;
        if actual != expected {
            return Err(AppError::new(
                "clean_room_mismatch",
                "reconstructed knowledge objects differ",
            ));
        }
        Ok(json!({
            "schema": "dwv.docs.clean-room.v1",
            "equivalent": true,
            "objects_digest": digest_bytes(&actual),
            "excluded": ["handoffs", "archived changes", "milestones", "generated output", "global Python packages"]
        }))
    })();
    let cleanup = fs::remove_dir_all(&root);
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(AppError::new(
            "clean_room_cleanup_failed",
            error.to_string(),
        )),
    }
}

fn copy_permanent_input(
    source_root: &Path,
    target_root: &Path,
    relative: &str,
) -> Result<(), AppError> {
    let source = safe_join(source_root, relative)?;
    if !source.exists() {
        return Ok(());
    }
    let target = target_root.join(relative);
    let metadata = fs::symlink_metadata(&source)
        .map_err(|error| AppError::new("clean_room_copy_failed", error.to_string()))?;
    if metadata.file_type().is_symlink() {
        return Err(AppError::new("clean_room_symlink", relative));
    }
    if metadata.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| AppError::new("clean_room_copy_failed", error.to_string()))?;
        }
        fs::copy(&source, &target)
            .map_err(|error| AppError::new("clean_room_copy_failed", error.to_string()))?;
        return Ok(());
    }
    fs::create_dir_all(&target)
        .map_err(|error| AppError::new("clean_room_copy_failed", error.to_string()))?;
    let mut entries = fs::read_dir(&source)
        .map_err(|error| AppError::new("clean_room_copy_failed", error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::new("clean_room_copy_failed", error.to_string()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let child = format!("{relative}/{}", entry.file_name().to_string_lossy());
        copy_permanent_input(source_root, target_root, &child)?;
    }
    Ok(())
}

fn run_sphinx(root: &Path, command: &str) -> Result<Value, AppError> {
    let output = Command::new("mise")
        .args([
            "exec",
            "--",
            "uv",
            "run",
            "--offline",
            "--no-project",
            "--python",
            "3.12.13",
            "tools/dwv-sphinx.py",
            command,
        ])
        .current_dir(root)
        .output()
        .map_err(|e| AppError::new("sphinx_build_unavailable", e.to_string()))?;
    if !output.status.success() {
        return Err(AppError::new(
            "sphinx_build_failed",
            format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next_back()
        .ok_or_else(|| AppError::new("sphinx_result_missing", "Sphinx produced no result"))
        .and_then(|line| {
            serde_json::from_str(line)
                .map_err(|e| AppError::new("sphinx_result_invalid", e.to_string()))
        })
}

fn help_value() -> Value {
    json!({"schema": CLI_SCHEMA, "usage": "cargo xtask docs <operation>", "operations": [
        "help", "schema", "doctor", "check [--base <revision>]", "build", "serve", "clean-room",
        "knowledge export|extract|export-sphinx", "inspect <id>",
        "context|trace|why <id>", "ownership <id>",
        "affected <id>|--id <id>|--path <repo-relative-path>",
        "knowledge doctor --path <repo-relative-path>", "readiness|check",
        "resolve <id> --outcome <reviewed|reference-only|deferred|superseded> --reason <text>"
    ], "guarantees": ["offline deterministic scans", "bounded relation context", "atomic reviewed state"]})
}

fn schema_value() -> Value {
    json!({"schema": CLI_SCHEMA, "commands": {
        "knowledge_objects": "dwv.knowledge.objects.v2", "knowledge_context": "dwv.knowledge.context.v2",
        "knowledge_ownership": "dwv.knowledge.ownership.v1", "knowledge_readiness": "dwv.knowledge.readiness.v2",
        "reviewed_state": "dwv.knowledge.reviewed-links.v2",
        "source_inventory": INVENTORY_SCHEMA
    }})
}

pub(crate) fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
}

pub(crate) fn markdown_headings(text: &str) -> Result<Vec<Heading>, AppError> {
    let mut open = None;
    let mut raw = Vec::new();
    for (event, range) in Parser::new_ext(text, markdown_options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                open = Some((heading_level(level), range.start, String::new()))
            }
            Event::Text(value) | Event::Code(value) => {
                if let Some((_, _, title)) = &mut open {
                    title.push_str(&value);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, start, title)) = open.take() {
                    raw.push((level, start, title.trim().to_owned()));
                }
            }
            _ => {}
        }
    }
    let mut stack: Vec<(u8, String)> = Vec::new();
    Ok(raw
        .into_iter()
        .map(|(level, start, title)| {
            while stack.last().is_some_and(|previous| previous.0 >= level) {
                stack.pop();
            }
            stack.push((level, title.clone()));
            Heading {
                level,
                title,
                path: stack.iter().map(|(_, title)| title.clone()).collect(),
                start,
            }
        })
        .collect())
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub(crate) fn normalize_markdown(text: &str) -> Result<String, AppError> {
    let mut records = Vec::new();
    let mut kind = "TEXT".to_owned();
    let mut buffer = String::new();
    let mut links = Vec::new();
    let mut code = false;
    for event in Parser::new_ext(text, markdown_options()) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_record(&mut records, &mut kind, &mut buffer, code);
                kind = format!("H{}", heading_level(level));
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_record(&mut records, &mut kind, &mut buffer, code)
            }
            Event::Start(Tag::Paragraph) => {
                flush_record(&mut records, &mut kind, &mut buffer, code);
                kind = "P".to_owned();
            }
            Event::End(TagEnd::Paragraph) => {
                flush_record(&mut records, &mut kind, &mut buffer, code)
            }
            Event::Start(Tag::Item) => {
                flush_record(&mut records, &mut kind, &mut buffer, code);
                kind = "LI".to_owned();
            }
            Event::End(TagEnd::Item) => flush_record(&mut records, &mut kind, &mut buffer, code),
            Event::Start(Tag::CodeBlock(_)) => {
                flush_record(&mut records, &mut kind, &mut buffer, code);
                kind = "CODE".to_owned();
                code = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                flush_record(&mut records, &mut kind, &mut buffer, code);
                code = false;
            }
            Event::Start(Tag::BlockQuote(_)) => {
                flush_record(&mut records, &mut kind, &mut buffer, code);
                kind = "QUOTE".to_owned();
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                flush_record(&mut records, &mut kind, &mut buffer, code)
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                buffer.push('[');
                links.push(dest_url.to_string());
            }
            Event::End(TagEnd::Link) => {
                if let Some(url) = links.pop() {
                    buffer.push_str("](");
                    buffer.push_str(&url);
                    buffer.push(')');
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                buffer.push_str("[image:");
                links.push(dest_url.to_string());
            }
            Event::End(TagEnd::Image) => {
                if let Some(url) = links.pop() {
                    buffer.push_str("](");
                    buffer.push_str(&url);
                    buffer.push(')');
                }
            }
            Event::Text(value) | Event::Code(value) => buffer.push_str(&value),
            Event::InlineHtml(value) | Event::Html(value)
                if !value.trim_start().starts_with("<!--") =>
            {
                return Err(AppError::new("unsafe_markdown", value.to_string()));
            }
            Event::SoftBreak => buffer.push(if code { '\n' } else { ' ' }),
            Event::HardBreak => buffer.push('\n'),
            Event::TaskListMarker(checked) => {
                buffer.push_str(if checked { "[x] " } else { "[ ] " })
            }
            Event::Start(Tag::TableCell) => buffer.push('\t'),
            Event::End(TagEnd::TableRow) => {
                flush_record(&mut records, &mut kind, &mut buffer, code)
            }
            Event::End(TagEnd::List(_)) => flush_record(&mut records, &mut kind, &mut buffer, code),
            _ => {}
        }
    }
    flush_record(&mut records, &mut kind, &mut buffer, code);
    Ok(records.join("\n"))
}

fn flush_record(records: &mut Vec<String>, kind: &mut String, buffer: &mut String, code: bool) {
    if buffer.is_empty() {
        return;
    }
    let value = if code {
        buffer.replace("\r\n", "\n")
    } else {
        buffer.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    if !value.is_empty() {
        records.push(format!("{kind}:{value}"));
    }
    buffer.clear();
    if !code {
        *kind = "TEXT".to_owned();
    }
}

pub(crate) fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, AppError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(AppError::new("invalid_path", relative));
    }
    Ok(root.join(path))
}

pub(crate) fn read_toml<T: DeserializeOwned>(root: &Path, relative: &str) -> Result<T, AppError> {
    let path = safe_join(root, relative)?;
    let text = fs::read_to_string(&path)
        .map_err(|e| AppError::new("read_failed", format!("{relative}: {e}")))?;
    toml::from_str(&text).map_err(|e| AppError::new("parse_failed", format!("{relative}: {e}")))
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::new("write_failed", path.display().to_string()))?;
    fs::create_dir_all(parent).map_err(|e| AppError::new("write_failed", e.to_string()))?;
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&temporary, bytes).map_err(|e| AppError::new("write_failed", e.to_string()))?;
    fs::rename(&temporary, path).map_err(|e| AppError::new("write_failed", e.to_string()))
}

pub(crate) fn write_json<T: Serialize>(
    root: &Path,
    relative: &str,
    value: &T,
) -> Result<(), AppError> {
    let text = serde_json::to_vec_pretty(value)
        .map_err(|e| AppError::new("serialization_failed", e.to_string()))?;
    atomic_write(
        &safe_join(root, relative)?,
        &[text, b"\n".to_vec()].concat(),
    )
}

pub(crate) fn digest_text(text: &str) -> String {
    digest_bytes(text.as_bytes())
}
pub(crate) fn digest_bytes(bytes: &[u8]) -> String {
    hash_hex(blake3::hash(bytes))
}
fn hash_hex(hash: Hash) -> String {
    hash.to_hex().to_string()
}

fn option_value(args: &[String], option: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == option)
        .map(|pair| pair[1].clone())
        .or_else(|| {
            args.iter()
                .find_map(|arg| arg.strip_prefix(&format!("{option}=")).map(str::to_owned))
        })
}

fn option_values(args: &[String], option: &str) -> Vec<String> {
    let mut values = args
        .windows(2)
        .filter(|pair| pair[0] == option)
        .map(|pair| pair[1].clone())
        .collect::<Vec<_>>();
    values.extend(
        args.iter()
            .filter_map(|arg| arg.strip_prefix(&format!("{option}=")).map(str::to_owned)),
    );
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markdown_fingerprint_is_format_stable_and_semantic_sensitive() {
        let a = normalize_markdown("# Heading\n\nwrapped\nwords").unwrap();
        let b = normalize_markdown("# Heading\n\nwrapped words").unwrap();
        assert_eq!(digest_text(&a), digest_text(&b));
        assert_ne!(
            digest_text(&a),
            digest_text(&normalize_markdown("# Heading\n\nchanged").unwrap())
        );
    }
    #[test]
    fn path_join_rejects_escape() {
        assert!(safe_join(Path::new("/tmp"), "../x").is_err());
    }
}
