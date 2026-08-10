use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use super::{
    App, AppError, atomic_write, digest_text, markdown_headings, normalize_markdown, read_toml,
    safe_join, write_json,
};

const OBJECTS_SCHEMA: &str = "dwv.knowledge.objects.v1";
const CONTEXT_SCHEMA: &str = "dwv.knowledge.context.v1";
const AFFECTED_SCHEMA: &str = "dwv.knowledge.affected.v1";
const OBJECTS_PATH: &str = "target/dwv-docs/knowledge/objects.json";
const REVIEWED_PATH: &str = "docs/reviewed-requirements.toml";
const READINESS_SCHEMA: &str = "dwv.knowledge.readiness.v1";
const REVIEWED_SCHEMA: &str = "dwv.knowledge.reviewed-links.v1";
const OUTCOMES: [&str; 4] = ["reviewed", "reference-only", "deferred", "superseded"];

const ACTIVE_ROADMAP_MARKER: &str = "<!-- dwv:active-architecture-roadmap -->";
const MAX_AUTHORITY_FILE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub(super) struct RequirementObject {
    pub semantic_id: String,
    pub sphinx_id: String,
    pub title: String,
    pub capability: String,
    pub source_path: String,
    pub heading_path: Vec<String>,
    pub normalized_fingerprint: String,
    pub body: String,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct Reference {
    kind: String,
    id: String,
    path: String,
    relation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ReviewedState {
    schema: String,
    #[serde(default)]
    requirements: BTreeMap<String, String>,
    #[serde(default)]
    outcomes: BTreeMap<String, String>,
    #[serde(default)]
    reasons: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct ScanRef {
    id: String,
    kind: String,
    path: String,
    line: usize,
}

pub(super) fn objects(app: &App) -> Result<Vec<RequirementObject>, AppError> {
    let root = app.root.join("openspec/specs");
    let mut dirs = entries(&root, "spec_discovery_failed")?
        .into_iter()
        .filter(|entry| {
            entry
                .file_type()
                .map(|t| t.is_dir() && !t.is_symlink())
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    dirs.sort_by_key(|entry| entry.file_name());
    let mut objects = Vec::new();
    let mut ids = BTreeSet::new();
    for dir in dirs {
        let capability = dir.file_name().to_string_lossy().into_owned();
        let path = dir.path().join("spec.md");
        if !path.is_file() {
            continue;
        }
        let relative = format!("openspec/specs/{capability}/spec.md");
        let text = read_bounded(&path, &relative, app)?;
        let headings = markdown_headings(&text)?;
        for (index, heading) in headings
            .iter()
            .enumerate()
            .filter(|(_, h)| h.title.starts_with("Requirement: "))
        {
            let id = colocated_id(&text, heading, &capability)?;
            validate_semantic_id(&id)?;
            if !ids.insert(id.clone()) {
                return Err(AppError::new("duplicate_requirement_id", id));
            }
            let end = headings
                .iter()
                .skip(index + 1)
                .find(|next| next.level <= heading.level)
                .map(|next| next.start)
                .unwrap_or(text.len());
            let body = normalize_markdown(&text[heading.start..end])?;
            if body.len() > app.bounds.max_source_bytes {
                return Err(AppError::new("unit_bound_exceeded", id));
            }
            objects.push(RequirementObject {
                semantic_id: id.clone(),
                sphinx_id: sphinx_id(&id),
                title: heading.title.trim_start_matches("Requirement: ").to_owned(),
                capability: capability.clone(),
                source_path: relative.clone(),
                heading_path: heading.path.clone(),
                normalized_fingerprint: digest_text(&body),
                body,
            });
        }
    }
    if objects.len() > app.bounds.max_units {
        return Err(AppError::new(
            "unit_bound_exceeded",
            objects.len().to_string(),
        ));
    }
    objects.sort_by(|a, b| a.semantic_id.cmp(&b.semantic_id));
    check_mapping_collisions(
        objects
            .iter()
            .map(|o| (o.semantic_id.as_str(), o.sphinx_id.as_str())),
    )?;
    Ok(objects)
}

pub(super) fn export(app: &App) -> Result<Value, AppError> {
    let records = objects(app)?;
    let value = json!({"schema": OBJECTS_SCHEMA, "objects": records});
    write_json(&app.root, OBJECTS_PATH, &value)?;
    Ok(
        json!({"schema": OBJECTS_SCHEMA, "objects": value["objects"].as_array().map_or(0, Vec::len), "path": OBJECTS_PATH}),
    )
}

pub(super) fn inspect(app: &App, id: String) -> Result<Value, AppError> {
    objects(app)?
        .into_iter()
        .find(|object| object.semantic_id == id)
        .map(|object| serde_json::to_value(object).expect("object serializes"))
        .ok_or_else(|| AppError::new("unknown_requirement", id))
}

pub(super) fn context(app: &App, id: String) -> Result<Value, AppError> {
    let requirement = inspect(app, id.clone())?;
    let mut refs = scan_references(app)?;
    refs.retain(|reference| reference.id == id);
    refs.sort_by(|a, b| (&a.kind, &a.path, a.line).cmp(&(&b.kind, &b.path, b.line)));
    let mut inbound = Vec::new();
    let mut outbound = Vec::new();
    for reference in refs {
        let relation = match reference.kind.as_str() {
            "rust" => "implements",
            "verification" => "verifies",
            _ => "explained_by",
        };
        let value = Reference {
            kind: reference.kind,
            id: id.clone(),
            path: reference.path,
            relation: relation.to_owned(),
            line: Some(reference.line),
        };
        if relation == "implements" {
            inbound.push(value);
        } else {
            outbound.push(value);
        }
    }
    let inbound_total = inbound.len();
    let outbound_total = outbound.len();
    inbound.truncate(app.bounds.max_units);
    outbound.truncate(app.bounds.max_units);
    let value = json!({
        "schema": CONTEXT_SCHEMA,
        "requirement": requirement,
        "inbound": inbound,
        "outbound": outbound,
        "bounds": {
            "max_references_per_direction": app.bounds.max_units,
            "max_context_bytes": app.bounds.max_context_bytes
        },
        "omitted": {
            "inbound": inbound_total.saturating_sub(app.bounds.max_units),
            "outbound": outbound_total.saturating_sub(app.bounds.max_units)
        }
    });
    if serde_json::to_vec(&value)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX)
        > app.bounds.max_context_bytes
    {
        return Err(AppError::new("context_bound_exceeded", id));
    }
    Ok(value)
}

pub(super) fn affected(
    app: &App,
    explicit_ids: Vec<String>,
    changed_paths: Vec<String>,
) -> Result<Value, AppError> {
    if explicit_ids.is_empty() && changed_paths.is_empty() {
        return Err(AppError::new(
            "usage",
            "affected requires a requirement ID or --path <repo-relative-path>",
        ));
    }
    let requirements = objects(app)?;
    let current = requirements
        .iter()
        .map(|requirement| (requirement.semantic_id.as_str(), requirement))
        .collect::<BTreeMap<_, _>>();
    let references = scan_references(app)?;
    let reviewed: ReviewedState = read_toml(&app.root, REVIEWED_PATH)?;
    let mut selected = BTreeMap::<String, (BTreeSet<String>, bool)>::new();
    let mut path_impacts = Vec::new();

    for id in explicit_ids {
        if !current.contains_key(id.as_str()) {
            return Err(AppError::new("unknown_requirement", id));
        }
        let impact = selected.entry(id).or_default();
        impact.0.insert("explicit requirement ID".to_owned());
        impact.1 = true;
    }

    for relative in changed_paths {
        validate_affected_path(app, &relative)?;
        let canonical = requirements
            .iter()
            .filter(|requirement| requirement.source_path == relative)
            .collect::<Vec<_>>();
        if !canonical.is_empty() {
            let changed = canonical
                .into_iter()
                .filter(|requirement| {
                    reviewed.requirements.get(&requirement.semantic_id)
                        != Some(&requirement.normalized_fingerprint)
                })
                .map(|requirement| requirement.semantic_id.clone())
                .collect::<BTreeSet<_>>();
            for id in &changed {
                let impact = selected.entry(id.clone()).or_default();
                impact
                    .0
                    .insert(format!("canonical semantics changed in {relative}"));
                impact.1 = true;
            }
            path_impacts.push(json!({
                "path": relative,
                "classification": if changed.is_empty() { "canonical_fingerprints_unchanged" } else { "canonical_semantics_changed" },
                "review_required": !changed.is_empty(),
                "current_ids": changed,
                "baseline": "reviewed requirement fingerprints"
            }));
            continue;
        }

        let current_ids = references
            .iter()
            .filter(|reference| reference.path == relative)
            .map(|reference| reference.id.clone())
            .collect::<BTreeSet<_>>();
        let baseline = baseline_ids(app, &relative)?;
        let old_ids = baseline.ids.clone();
        let added = current_ids
            .difference(&old_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let removed = old_ids
            .difference(&current_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let review_required = baseline.available && !removed.is_empty();
        let classification = if review_required {
            "requirement_relationship_removed_or_reassigned"
        } else if baseline.available && !added.is_empty() {
            "requirement_relationship_added_context_only"
        } else if baseline.available {
            "implementation_or_evidence_changed_relationships_unchanged"
        } else {
            "current_context_only_baseline_unavailable"
        };
        for id in current_ids.union(&old_ids) {
            if !current.contains_key(id.as_str()) {
                return Err(AppError::new(
                    "unknown_requirement",
                    format!("{relative}: {id}"),
                ));
            }
            let impact = selected.entry(id.clone()).or_default();
            impact.0.insert(format!("{classification} in {relative}"));
            impact.1 |= review_required;
        }
        path_impacts.push(json!({
            "path": relative,
            "classification": classification,
            "review_required": review_required,
            "current_ids": current_ids,
            "previous_ids": old_ids,
            "added_ids": added,
            "removed_ids": removed,
            "baseline": baseline.source,
            "next_action": if review_required {
                "review only the listed dependent pages"
            } else {
                "use the returned context for the code task; review prose only if the task changes documented behavior"
            }
        }));
    }

    if selected.len() > app.bounds.max_units {
        return Err(AppError::new(
            "affected_unit_bound_exceeded",
            selected.len().to_string(),
        ));
    }
    let mut impacts = Vec::new();
    for (id, (reasons, review_required)) in selected {
        let packet = context(app, id.clone())?;
        let pages = packet["outbound"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|reference| reference["kind"] == "markdown")
            .map(|reference| {
                json!({
                    "path": reference["path"],
                    "line": reference["line"],
                    "action": if review_required {
                        "review whether this page still accurately explains the changed semantics or relationship"
                    } else {
                        "context only; review this page only if the implementation task changes its documented claim"
                    }
                })
            })
            .collect::<Vec<_>>();
        impacts.push(json!({
            "semantic_id": id,
            "review_required": review_required,
            "reasons": reasons,
            "pages": pages,
            "context": packet
        }));
    }
    let value = json!({
        "schema": AFFECTED_SCHEMA,
        "review_required": path_impacts.iter().any(|impact| impact["review_required"] == true)
            || impacts.iter().any(|impact| impact["review_required"] == true),
        "paths": path_impacts,
        "impacts": impacts,
        "limitations": [
            "revision-baseline recovery is unavailable for files absent from revision control or outside a supported repository",
            "unchanged relationships cannot prove that arbitrary implementation edits preserve documented behavior",
            "the command identifies review scope; it does not infer whether prose is semantically correct"
        ]
    });
    enforce_context_bound(app, "affected_context_bound_exceeded", &value)?;
    Ok(value)
}

pub(super) fn doctor(app: &App, changed_paths: Vec<String>) -> Result<Value, AppError> {
    if changed_paths.is_empty() {
        return Err(AppError::new(
            "usage",
            "knowledge doctor requires --path <repo-relative-path>",
        ));
    }
    let requirements = objects(app)?;
    let current = requirements
        .iter()
        .map(|requirement| requirement.semantic_id.as_str())
        .collect::<BTreeSet<_>>();
    let references = scan_references(app)?;
    let mut reports = Vec::new();
    for relative in changed_paths {
        validate_affected_path(app, &relative)?;
        let current_ids = references
            .iter()
            .filter(|reference| reference.path == relative)
            .map(|reference| reference.id.clone())
            .collect::<BTreeSet<_>>();
        let baseline = baseline_ids(app, &relative)?;
        let recovered_ids = baseline
            .ids
            .difference(&current_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let unresolved_ids = recovered_ids
            .iter()
            .filter(|id| !current.contains(id.as_str()))
            .cloned()
            .collect::<BTreeSet<_>>();
        reports.push(json!({
            "path": relative,
            "baseline": baseline.source,
            "baseline_available": baseline.available,
            "current_ids": current_ids,
            "recovered_removed_ids": recovered_ids,
            "unresolved_noncanonical_ids": unresolved_ids,
            "next_action": if !baseline.available {
                "recover the old ID from review history or supply it explicitly; no relationship registry is retained"
            } else if recovered_ids.is_empty() {
                "no requirement relationship was lost relative to the parent revision"
            } else {
                "run knowledge affected --id for each recovered ID and review only its listed pages"
            }
        }));
    }
    let value = json!({
        "schema": "dwv.knowledge.doctor.v1",
        "reports": reports,
        "authority": "parent-revision content is recovery evidence, not semantic authority"
    });
    enforce_context_bound(app, "doctor_context_bound_exceeded", &value)?;
    Ok(value)
}

struct BaselineIds {
    available: bool,
    source: String,
    ids: BTreeSet<String>,
}

#[derive(Clone, Copy)]
enum RevisionControl {
    Jujutsu,
    Git,
}

impl RevisionControl {
    fn detect(root: &Path) -> Option<Self> {
        [
            (Self::Jujutsu, "jj", &["root"][..]),
            (Self::Git, "git", &["rev-parse", "--show-toplevel"][..]),
        ]
        .into_iter()
        .find_map(|(provider, program, args)| {
            Command::new(program)
                .args(args)
                .current_dir(root)
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|_| provider)
        })
    }

    fn name(self) -> &'static str {
        match self {
            Self::Jujutsu => "jujutsu",
            Self::Git => "git",
        }
    }

    fn default_base(self) -> &'static str {
        match self {
            Self::Jujutsu => "@-",
            Self::Git => "HEAD",
        }
    }

    fn read(
        self,
        root: &Path,
        base: &str,
        relative: &str,
    ) -> std::io::Result<std::process::Output> {
        let mut command = match self {
            Self::Jujutsu => {
                let mut command = Command::new("jj");
                command.args(["file", "show", "-r", base, relative]);
                command
            }
            Self::Git => {
                let mut command = Command::new("git");
                command.args(["show", &format!("{base}:{relative}")]);
                command
            }
        };
        command.current_dir(root).output()
    }

    fn changed_paths(self, root: &Path, base: &str) -> Result<BTreeSet<String>, AppError> {
        let output = match self {
            Self::Jujutsu => Command::new("jj")
                .args(["diff", "--summary", "--from", base, "--to", "@"])
                .current_dir(root)
                .output(),
            Self::Git => Command::new("git")
                .args(["diff", "--name-status", base, "--"])
                .current_dir(root)
                .output(),
        }
        .map_err(|error| AppError::new("change_baseline_unavailable", error.to_string()))?;
        if !output.status.success() {
            return Err(AppError::new(
                "change_baseline_invalid",
                String::from_utf8_lossy(&output.stderr).trim(),
            ));
        }
        let text = String::from_utf8(output.stdout)
            .map_err(|error| AppError::new("change_list_not_utf8", error.to_string()))?;
        let mut paths: BTreeSet<String> = match self {
            Self::Jujutsu => text
                .lines()
                .filter_map(|line| line.get(2..))
                .flat_map(expand_rename)
                .collect(),
            Self::Git => text
                .lines()
                .flat_map(|line| line.split('\t').skip(1).map(str::to_owned))
                .collect(),
        };
        if let Self::Git = self {
            let untracked = Command::new("git")
                .args(["ls-files", "--others", "--exclude-standard"])
                .current_dir(root)
                .output()
                .map_err(|error| AppError::new("change_baseline_unavailable", error.to_string()))?;
            if !untracked.status.success() {
                return Err(AppError::new(
                    "change_baseline_invalid",
                    String::from_utf8_lossy(&untracked.stderr).trim(),
                ));
            }
            paths.extend(
                String::from_utf8(untracked.stdout)
                    .map_err(|error| AppError::new("change_list_not_utf8", error.to_string()))?
                    .lines()
                    .map(str::to_owned),
            );
        }
        Ok(paths)
    }
}

fn expand_rename(path: &str) -> Vec<String> {
    let path = path.trim();
    let Some(open) = path.find('{') else {
        return vec![path.to_owned()];
    };
    let Some(close_offset) = path[open + 1..].find('}') else {
        return vec![path.to_owned()];
    };
    let close = open + close_offset + 1;
    let Some((old, new)) = path[open + 1..close].split_once(" => ") else {
        return vec![path.to_owned()];
    };
    let prefix = &path[..open];
    let suffix = &path[close + 1..];
    vec![
        format!("{prefix}{old}{suffix}"),
        format!("{prefix}{new}{suffix}"),
    ]
}

fn baseline_ids_at(
    app: &App,
    provider: RevisionControl,
    base: &str,
    relative: &str,
) -> Result<BaselineIds, AppError> {
    let output = provider.read(&app.root, base, relative);
    match output {
        Ok(output) if output.status.success() => {
            if output.stdout.len() > app.bounds.max_source_bytes {
                return Err(AppError::new("source_bound_exceeded", relative));
            }
            let text = String::from_utf8(output.stdout)
                .map_err(|error| AppError::new("baseline_not_utf8", error.to_string()))?;
            Ok(BaselineIds {
                available: true,
                source: format!("{} revision {base}", provider.name()),
                ids: relationship_ids(relative, &text),
            })
        }
        Ok(output) if revision_path_absent(&output.stderr) => Ok(BaselineIds {
            available: true,
            source: format!("path absent from {} revision {base}", provider.name()),
            ids: BTreeSet::new(),
        }),
        Ok(_) | Err(_) => Ok(BaselineIds {
            available: false,
            source: format!("{} revision {base} unavailable", provider.name()),
            ids: BTreeSet::new(),
        }),
    }
}

impl BaselineIds {
    fn unavailable() -> Self {
        Self {
            available: false,
            source: "revision-control baseline unavailable".to_owned(),
            ids: BTreeSet::new(),
        }
    }
}

fn is_impact_candidate(path: &str) -> bool {
    !excluded(path)
        && ((path.starts_with("crates/") && path.ends_with(".rs"))
            || (path.starts_with("xtask/") && path.ends_with(".rs"))
            || (path.starts_with("openspec/specs/") && path.ends_with("/spec.md"))
            || path == "verification/manifest.toml"
            || path == "docs/curriculum.toml"
            || (path.starts_with("docs/sphinx/") && path.ends_with(".md")))
}

fn baseline_ids(app: &App, relative: &str) -> Result<BaselineIds, AppError> {
    let Some(provider) = RevisionControl::detect(&app.root) else {
        return Ok(BaselineIds::unavailable());
    };
    baseline_ids_at(app, provider, provider.default_base(), relative)
}

fn revision_path_absent(stderr: &[u8]) -> bool {
    let stderr = String::from_utf8_lossy(stderr);
    [
        "No such path",
        "does not exist",
        "exists on disk, but not in",
    ]
    .iter()
    .any(|message| stderr.contains(message))
}

fn relationship_ids(relative: &str, text: &str) -> BTreeSet<String> {
    if relative.ends_with(".rs") {
        text.lines()
            .filter(|line| line.contains("dwv:req"))
            .flat_map(extract_ids)
            .collect()
    } else {
        extract_ids(text)
    }
}

fn validate_affected_path(app: &App, relative: &str) -> Result<(), AppError> {
    if excluded(relative) {
        return Err(AppError::new("affected_path_excluded", relative));
    }
    let path = safe_join(&app.root, relative)?;
    if !path.is_file() {
        return Err(AppError::new("affected_path_missing", relative));
    }
    let metadata = fs::metadata(path)
        .map_err(|error| AppError::new("affected_read_failed", error.to_string()))?;
    if metadata.len() > app.bounds.max_source_bytes as u64 {
        return Err(AppError::new("source_bound_exceeded", relative));
    }
    Ok(())
}

fn enforce_context_bound(app: &App, code: &str, value: &Value) -> Result<(), AppError> {
    if serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX)
        > app.bounds.max_context_bytes
    {
        return Err(AppError::new(
            code,
            "narrow the changed paths or requirement IDs",
        ));
    }
    Ok(())
}
/// dwv:req req.documentation-knowledge-architecture.change-boundary-impact-is-deterministic-and-bounded
pub(super) fn change_impact(app: &App, requested_base: Option<&str>) -> Result<Value, AppError> {
    let Some(provider) = RevisionControl::detect(&app.root) else {
        return Ok(json!({
            "schema": "dwv.knowledge.change-impact.v1",
            "baseline_available": false,
            "review_required": false,
            "diagnostics": ["revision-control baseline unavailable; semantic readiness still ran, but relationship deltas were not compared"]
        }));
    };
    let base = requested_base.unwrap_or_else(|| provider.default_base());
    let paths = provider.changed_paths(&app.root, base)?;
    let candidates = paths
        .into_iter()
        .filter(|path| is_impact_candidate(path))
        .collect::<Vec<_>>();
    if candidates.len() > app.bounds.max_units {
        return Err(AppError::new(
            "change_impact_unit_bound_exceeded",
            candidates.len().to_string(),
        ));
    }

    let requirements = objects(app)?;
    let requirement_ids = requirements
        .iter()
        .map(|requirement| requirement.semantic_id.as_str())
        .collect::<BTreeSet<_>>();
    let references = scan_references(app)?;
    let reviewed: ReviewedState = read_toml(&app.root, REVIEWED_PATH)?;
    let mut entries = Vec::new();
    let mut review_ids = BTreeSet::new();
    let mut context_ids = BTreeSet::new();
    let mut diagnostics = Vec::new();
    let mut next_actions = BTreeSet::new();
    let mut added_relationships = 0usize;
    let mut unchanged_relationship_files = 0usize;

    for relative in candidates {
        let canonical = requirements
            .iter()
            .filter(|requirement| requirement.source_path == relative)
            .collect::<Vec<_>>();
        if !canonical.is_empty() {
            let ids = canonical
                .into_iter()
                .filter(|requirement| {
                    reviewed.requirements.get(&requirement.semantic_id)
                        != Some(&requirement.normalized_fingerprint)
                })
                .map(|requirement| requirement.semantic_id.clone())
                .collect::<BTreeSet<_>>();
            if !ids.is_empty() {
                review_ids.extend(ids.iter().cloned());
                next_actions.insert(format!(
                    "cargo xtask docs knowledge affected --path {relative}"
                ));
                entries.push(json!({
                    "path": relative,
                    "classification": "canonical_semantics_changed",
                    "review_required": true,
                    "requirement_ids": ids
                }));
            }
            continue;
        }

        let current_ids = references
            .iter()
            .filter(|reference| reference.path == relative)
            .map(|reference| reference.id.clone())
            .collect::<BTreeSet<_>>();
        let baseline = baseline_ids_at(app, provider, base, &relative)?;
        let added = current_ids
            .difference(&baseline.ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let removed = baseline
            .ids
            .difference(&current_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let all_ids = current_ids
            .union(&baseline.ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        if all_ids.is_empty() {
            continue;
        }
        let unknown = all_ids
            .iter()
            .filter(|id| !requirement_ids.contains(id.as_str()))
            .cloned()
            .collect::<BTreeSet<_>>();
        if !unknown.is_empty() {
            diagnostics.push(json!({
                "path": relative,
                "kind": "unknown_or_removed_canonical_requirement",
                "requirement_ids": unknown
            }));
        }
        added_relationships += added.len();
        let review_required = baseline.available && !removed.is_empty();
        if review_required {
            review_ids.extend(all_ids.iter().cloned());
            next_actions.insert(format!(
                "cargo xtask docs knowledge affected --path {relative}"
            ));
            entries.push(json!({
                "path": relative,
                "classification": "requirement_relationship_removed_or_reassigned",
                "review_required": true,
                "current_ids": current_ids,
                "added_ids": added,
                "removed_ids": removed,
                "baseline": baseline.source
            }));
        } else if baseline.available && added.is_empty() {
            unchanged_relationship_files += 1;
            context_ids.extend(current_ids);
        }
    }
    let value = json!({
        "schema": "dwv.knowledge.change-impact.v1",
        "baseline_available": true,
        "provider": provider.name(),
        "base": base,
        "review_required": !review_ids.is_empty(),
        "review_requirement_ids": review_ids,
        "context_only_requirement_ids": context_ids,
        "summary": {
            "added_relationships": added_relationships,
            "unchanged_relationship_files": unchanged_relationship_files
        },
        "entries": entries,
        "diagnostics": diagnostics,
        "next_actions": next_actions
    });
    enforce_context_bound(app, "change_impact_context_bound_exceeded", &value)?;
    Ok(value)
}

fn validate_curriculum(app: &App) -> Result<(), AppError> {
    let path = app.root.join("docs/curriculum.toml");
    if !path.is_file() {
        return Ok(());
    }
    let curriculum: toml::Value = read_toml(&app.root, "docs/curriculum.toml")?;
    if curriculum.get("schema").and_then(toml::Value::as_str) != Some("dwv.docs.curriculum.v1") {
        return Err(AppError::new(
            "curriculum_schema_mismatch",
            "docs/curriculum.toml",
        ));
    }
    let entries = curriculum
        .get("entries")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| AppError::new("curriculum_invalid", "entries must be an array"))?;
    for entry in entries {
        let table = entry
            .as_table()
            .ok_or_else(|| AppError::new("curriculum_invalid", "entry must be a table"))?;
        if table.get("projection").and_then(toml::Value::as_str) != Some("human_guide") {
            continue;
        }
        let id = table
            .get("id")
            .and_then(toml::Value::as_str)
            .unwrap_or("(unknown guide entry)");
        for field in ["audience", "question"] {
            if table
                .get(field)
                .and_then(toml::Value::as_str)
                .is_none_or(str::is_empty)
            {
                return Err(AppError::new(
                    "curriculum_invalid",
                    format!("{id}.{field} must be a non-empty string"),
                ));
            }
        }
        for field in ["misconceptions", "requirements", "sections"] {
            if table
                .get(field)
                .and_then(toml::Value::as_array)
                .is_none_or(Vec::is_empty)
            {
                return Err(AppError::new(
                    "curriculum_invalid",
                    format!("{id}.{field} must be a non-empty array"),
                ));
            }
        }
        let entry_requirements = table["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(toml::Value::as_str)
            .collect::<BTreeSet<_>>();
        let entry_scenarios = table
            .get("scenarios")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(toml::Value::as_str)
            .collect::<BTreeSet<_>>();
        for (index, section) in table["sections"].as_array().unwrap().iter().enumerate() {
            let section = section.as_table().ok_or_else(|| {
                AppError::new(
                    "curriculum_invalid",
                    format!("{id}.sections[{index}] must be a table"),
                )
            })?;
            for field in ["heading", "focus"] {
                if section
                    .get(field)
                    .and_then(toml::Value::as_str)
                    .is_none_or(str::is_empty)
                {
                    return Err(AppError::new(
                        "curriculum_invalid",
                        format!("{id}.sections[{index}].{field} must be non-empty"),
                    ));
                }
            }
            for field in ["must_answer", "requirements", "non_claims"] {
                if section
                    .get(field)
                    .and_then(toml::Value::as_array)
                    .is_none_or(Vec::is_empty)
                {
                    return Err(AppError::new(
                        "curriculum_invalid",
                        format!("{id}.sections[{index}].{field} must be non-empty"),
                    ));
                }
            }
            if section
                .get("scenarios")
                .and_then(toml::Value::as_array)
                .is_none()
            {
                return Err(AppError::new(
                    "curriculum_invalid",
                    format!("{id}.sections[{index}].scenarios must be an array"),
                ));
            }
            for (field, allowed) in [
                ("requirements", &entry_requirements),
                ("scenarios", &entry_scenarios),
            ] {
                for source in section[field].as_array().unwrap() {
                    let source = source.as_str().ok_or_else(|| {
                        AppError::new(
                            "curriculum_invalid",
                            format!("{id}.sections[{index}].{field} values must be strings"),
                        )
                    })?;
                    if !allowed.contains(source) {
                        return Err(AppError::new(
                            "curriculum_invalid",
                            format!(
                                "{id}.sections[{index}].{field} source {source} is not declared by the entry"
                            ),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn collect_markdown_files(
    root: &Path,
    path: &Path,
    files: &mut Vec<std::path::PathBuf>,
) -> Result<(), AppError> {
    let mut children = entries(path, "roadmap_scan_failed")?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let kind = child
            .file_type()
            .map_err(|error| AppError::new("roadmap_scan_failed", error.to_string()))?;
        if kind.is_symlink() {
            return Err(AppError::new(
                "roadmap_scan_symlink",
                rel(root, &child.path()),
            ));
        }
        if kind.is_dir() {
            collect_markdown_files(root, &child.path(), files)?;
        } else if kind.is_file()
            && child
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("md")
        {
            files.push(child.path());
        }
    }
    Ok(())
}

fn read_authority_file(root: &Path, path: &Path) -> Result<String, AppError> {
    let relative = rel(root, path);
    let bytes = fs::read(path)
        .map_err(|error| AppError::new("roadmap_read_failed", format!("{relative}: {error}")))?;
    if bytes.len() > MAX_AUTHORITY_FILE_BYTES {
        return Err(AppError::new("roadmap_file_bound_exceeded", relative));
    }
    String::from_utf8(bytes)
        .map_err(|error| AppError::new("roadmap_invalid_utf8", format!("{relative}: {error}")))
}

fn roadmap_table_node(line: &str) -> Option<String> {
    let cell = line.split('|').nth(1)?.trim();
    let node = cell.strip_prefix("**")?.strip_suffix("**")?;
    let digits = node.strip_prefix("OS-")?;
    (digits.len() == 3 && digits.bytes().all(|byte| byte.is_ascii_digit())).then(|| node.to_owned())
}

fn numeric_change_node(change_id: &str) -> Option<String> {
    let suffix = change_id.strip_prefix("os-")?;
    let (digits, _) = suffix.split_once('-')?;
    (digits.len() == 3 && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| format!("OS-{digits}"))
}

fn archived_change_id(directory: &str) -> &str {
    directory.splitn(4, '-').nth(3).unwrap_or(directory)
}

fn append_change_directories(
    root: &Path,
    path: &Path,
    archived: bool,
    changes: &mut Vec<(String, String, bool)>,
) -> Result<(), AppError> {
    if !path.is_dir() {
        return Ok(());
    }
    let mut children = entries(path, "roadmap_change_scan_failed")?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let kind = child
            .file_type()
            .map_err(|error| AppError::new("roadmap_change_scan_failed", error.to_string()))?;
        if kind.is_symlink() {
            return Err(AppError::new(
                "roadmap_change_symlink",
                rel(root, &child.path()),
            ));
        }
        if !kind.is_dir() || (!archived && child.file_name() == "archive") {
            continue;
        }
        let directory = child.file_name().to_string_lossy().into_owned();
        let change_id = if archived {
            archived_change_id(&directory).to_owned()
        } else {
            directory
        };
        changes.push((rel(root, &child.path()), change_id, !archived));
    }
    Ok(())
}

fn proposal_roadmap_nodes(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("<!-- dwv:roadmap-node ")
                .and_then(|value| value.strip_suffix(" -->"))
                .map(str::to_owned)
        })
        .collect()
}

fn validate_roadmap_identifiers(app: &App) -> Result<(), AppError> {
    let docs = app.root.join("docs");
    let mut markdown = Vec::new();
    if docs.is_dir() {
        collect_markdown_files(&app.root, &docs, &mut markdown)?;
    }
    if markdown.len() > app.bounds.max_units {
        return Err(AppError::new(
            "roadmap_document_bound_exceeded",
            markdown.len().to_string(),
        ));
    }

    let mut marked = Vec::new();
    let mut marker_count = 0;
    let mut marked_text = None;
    for path in markdown {
        let text = read_authority_file(&app.root, &path)?;
        let occurrences = text.matches(ACTIVE_ROADMAP_MARKER).count();
        if occurrences > 0 {
            marker_count += occurrences;
            marked.push(rel(&app.root, &path));
            if marked_text.is_none() {
                marked_text = Some(text);
            }
        }
    }
    if marker_count != 1 {
        return Err(
            AppError::new("active_roadmap_marker_count", marker_count.to_string())
                .details(json!({"marker_count": marker_count, "paths": marked})),
        );
    }

    let roadmap = marked_text.expect("one marked roadmap has text");
    let roadmap_nodes = roadmap
        .lines()
        .filter_map(roadmap_table_node)
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();
    append_change_directories(
        &app.root,
        &app.root.join("openspec/changes"),
        false,
        &mut changes,
    )?;
    append_change_directories(
        &app.root,
        &app.root.join("openspec/changes/archive"),
        true,
        &mut changes,
    )?;

    let mut claims = BTreeMap::<String, Vec<String>>::new();
    let mut diagnostics = Vec::new();
    for (path, change_id, active) in changes {
        let Some(node) = numeric_change_node(&change_id) else {
            continue;
        };
        claims.entry(node.clone()).or_default().push(path.clone());
        if active && !roadmap_nodes.contains(&node) {
            diagnostics.push(json!({
                "gate": "numeric-change-not-in-roadmap",
                "change": path,
                "roadmap_node": node,
            }));
        }
        if active {
            let proposal_path = app.root.join(&path).join("proposal.md");
            let declarations = if proposal_path.is_file() {
                proposal_roadmap_nodes(&read_authority_file(&app.root, &proposal_path)?)
            } else {
                Vec::new()
            };
            if declarations.as_slice() != [node.as_str()] {
                diagnostics.push(json!({
                    "gate": "numeric-proposal-roadmap-declaration",
                    "change": path,
                    "expected": node,
                    "declared": declarations,
                }));
            }
        }
    }
    for (node, paths) in claims {
        if paths.len() > 1 {
            diagnostics.push(json!({
                "gate": "duplicate-numeric-change-identity",
                "roadmap_node": node,
                "changes": paths,
            }));
        }
    }
    if !diagnostics.is_empty() {
        return Err(AppError::new(
            "roadmap_identifier_invalid",
            "OpenSpec roadmap identifiers are invalid",
        )
        .details(json!({"active_roadmap": marked[0], "diagnostics": diagnostics})));
    }
    Ok(())
}

/// dwv:req req.documentation-knowledge-architecture.human-curriculum-is-pedagogical-intent-not-semantic-authority
pub(super) fn readiness(app: &App) -> Result<Value, AppError> {
    validate_roadmap_identifiers(app)?;
    validate_curriculum(app)?;
    let state: ReviewedState = read_toml(&app.root, REVIEWED_PATH)?;
    if state.schema != REVIEWED_SCHEMA {
        return Err(AppError::new(
            "reviewed_state_schema_mismatch",
            state.schema,
        ));
    }
    let records = objects(app)?;
    let current = records
        .iter()
        .map(|record| (record.semantic_id.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let refs = scan_references(app)?;
    let current_ids = current.keys().copied().collect::<BTreeSet<_>>();
    let mut uncovered = current_ids
        .iter()
        .filter(|id| !state.requirements.contains_key(**id))
        .map(|id| (*id).to_owned())
        .collect::<Vec<_>>();
    uncovered.sort();
    uncovered.dedup();
    let suspect = current
        .iter()
        .filter_map(|(id, record)| {
            state
                .requirements
                .get(*id)
                .filter(|fingerprint| *fingerprint != &record.normalized_fingerprint)
                .map(|_| (*id).to_owned())
        })
        .collect::<Vec<_>>();
    let mut orphaned = BTreeSet::new();
    for id in state
        .requirements
        .keys()
        .chain(state.outcomes.keys())
        .chain(state.reasons.keys())
    {
        let outcome = state.outcomes.get(id).map(String::as_str);
        if (!current_ids.contains(id.as_str()) && outcome != Some("superseded"))
            || outcome.is_some_and(|value| !OUTCOMES.contains(&value))
            || (outcome.is_some()
                && state
                    .reasons
                    .get(id)
                    .is_none_or(|reason| reason.trim().is_empty()))
        {
            orphaned.insert(id.clone());
        }
    }
    let unknown = refs.iter().filter(|reference| !current_ids.contains(reference.id.as_str())).map(|reference| json!({"semantic_id": reference.id, "kind": reference.kind, "path": reference.path, "line": reference.line})).collect::<Vec<_>>();
    let counts = json!({"uncovered": uncovered.len(), "fingerprint-suspect": suspect.len(), "orphaned-state": orphaned.len(), "unknown-reference": unknown.len(), "historical-reference": 0, "mapping-collision": 0});
    let mut diagnostics = Vec::new();
    for id in uncovered {
        diagnostics.push(json!({"gate": "uncovered", "semantic_id": id, "next_action": format!("cargo xtask docs knowledge resolve {id} --outcome reviewed --reason <text>")}));
    }
    for id in suspect {
        diagnostics.push(json!({"gate": "fingerprint-suspect", "semantic_id": id, "next_action": format!("cargo xtask docs knowledge resolve {id} --outcome reviewed --reason <text>")}));
    }
    for id in orphaned {
        diagnostics.push(json!({"gate": "orphaned-state", "semantic_id": id, "next_action": "remove orphaned state or resolve as superseded"}));
    }
    for reference in unknown {
        diagnostics.push(json!({"gate": "unknown-reference", "semantic_id": reference["semantic_id"], "path": reference["path"], "next_action": "remove the unknown req.* reference or add its canonical requirement"}));
    }
    let next_actions = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic["next_action"].as_str())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let result = json!({"schema": READINESS_SCHEMA, "ready": diagnostics.is_empty(), "requirements": current.len(), "gate_counts": counts, "diagnostics": diagnostics, "next_actions": next_actions});
    if result["ready"] == false {
        return Err(AppError::new(
            "knowledge_not_ready",
            "documentation knowledge is not ready",
        )
        .details(result));
    }
    Ok(result)
}

pub(super) fn resolve(app: &App, id: &str, outcome: &str, reason: &str) -> Result<Value, AppError> {
    if !OUTCOMES.contains(&outcome) {
        return Err(AppError::new("invalid_outcome", outcome));
    }
    if reason.trim().is_empty() {
        return Err(AppError::new("reason_required", id));
    }
    let records = objects(app)?;
    let current = records.iter().find(|record| record.semantic_id == id);
    let path = safe_join(&app.root, REVIEWED_PATH)?;
    let old = fs::read(&path)
        .map_err(|e| AppError::new("read_failed", format!("{REVIEWED_PATH}: {e}")))?;
    let mut state: ReviewedState = read_toml(&app.root, REVIEWED_PATH)?;
    if state.schema != REVIEWED_SCHEMA {
        return Err(AppError::new(
            "reviewed_state_schema_mismatch",
            state.schema,
        ));
    }
    if current.is_none() && (outcome != "superseded" || !state.requirements.contains_key(id)) {
        return Err(AppError::new("unknown_requirement", id));
    }
    if let Some(record) = current.filter(|_| outcome != "superseded") {
        state
            .requirements
            .insert(id.to_owned(), record.normalized_fingerprint.clone());
    }
    state.outcomes.insert(id.to_owned(), outcome.to_owned());
    state
        .reasons
        .insert(id.to_owned(), reason.trim().to_owned());
    let mut rendered = toml::to_string(&state)
        .map_err(|e| AppError::new("serialization_failed", e.to_string()))?
        .into_bytes();
    rendered.push(b'\n');
    let changed = rendered != old;
    if changed {
        atomic_write(&path, &rendered)?;
    }
    Ok(
        json!({"schema": READINESS_SCHEMA, "semantic_id": id, "outcome": outcome, "changed": changed, "path": REVIEWED_PATH}),
    )
}

fn scan_references(app: &App) -> Result<Vec<ScanRef>, AppError> {
    let mut refs = Vec::new();
    for (directory, rust) in [("docs", false), ("crates", true), ("xtask", true)] {
        let path = app.root.join(directory);
        if path.exists() {
            scan_tree(&app.root, &path, rust, app, &mut refs)?;
        }
    }
    for relative in ["docs/curriculum.toml", "verification/manifest.toml"] {
        let path = app.root.join(relative);
        if path.is_file() {
            scan_file(&app.root, &path, false, app, &mut refs)?;
        }
    }
    refs.sort_by(|a, b| (&a.id, &a.kind, &a.path, a.line).cmp(&(&b.id, &b.kind, &b.path, b.line)));
    Ok(refs)
}

fn scan_tree(
    root: &Path,
    path: &Path,
    rust: bool,
    app: &App,
    refs: &mut Vec<ScanRef>,
) -> Result<(), AppError> {
    let relative = rel(root, path);
    if excluded(&relative) {
        return Ok(());
    }
    let mut entries = entries(path, "reference_scan_failed")?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let kind = entry
            .file_type()
            .map_err(|e| AppError::new("reference_scan_failed", e.to_string()))?;
        if kind.is_symlink() {
            return Err(AppError::new("reference_symlink", rel(root, &entry.path())));
        }
        if kind.is_dir() {
            scan_tree(root, &entry.path(), rust, app, refs)?;
        } else if kind.is_file()
            && entry.path().extension().and_then(|x| x.to_str())
                == Some(if rust { "rs" } else { "md" })
        {
            scan_file(root, &entry.path(), rust, app, refs)?;
        }
    }
    Ok(())
}

fn scan_file(
    root: &Path,
    path: &Path,
    rust: bool,
    app: &App,
    refs: &mut Vec<ScanRef>,
) -> Result<(), AppError> {
    let relative = rel(root, path);
    if excluded(&relative) {
        return Ok(());
    }
    let text = read_bounded(path, &relative, app)?;
    for (line, content) in text.lines().enumerate() {
        if rust && !content.contains("/// dwv:req ") {
            continue;
        }
        let kind = if rust {
            "rust"
        } else if relative == "verification/manifest.toml" {
            "verification"
        } else if relative == "docs/curriculum.toml" {
            "curriculum"
        } else {
            "markdown"
        };
        for id in extract_ids(content) {
            refs.push(ScanRef {
                id,
                kind: kind.to_owned(),
                path: relative.clone(),
                line: line + 1,
            });
        }
    }
    Ok(())
}

fn read_bounded(path: &Path, relative: &str, app: &App) -> Result<String, AppError> {
    let bytes = fs::read(path)
        .map_err(|e| AppError::new("source_read_failed", format!("{relative}: {e}")))?;
    if bytes.len() > app.bounds.max_source_bytes {
        return Err(AppError::new("source_bound_exceeded", relative));
    }
    String::from_utf8(bytes)
        .map_err(|e| AppError::new("source_invalid_utf8", format!("{relative}: {e}")))
}

fn entries(path: &Path, code: &str) -> Result<Vec<fs::DirEntry>, AppError> {
    fs::read_dir(path)
        .map_err(|error| AppError::new(code, format!("{}: {error}", path.display())))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::new(code, format!("{}: {error}", path.display())))
}
fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
fn excluded(path: &str) -> bool {
    (path == "docs/milestones" || path.starts_with("docs/milestones/"))
        || path.split('/').any(|part| {
            part.is_empty()
                || part.starts_with('.')
                || matches!(
                    part,
                    "handoffs"
                        | "archive"
                        | "archived"
                        | "generated"
                        | "target"
                        | "tmp"
                        | "private"
                        | "book"
                )
        })
}

fn colocated_id(
    text: &str,
    heading: &super::Heading,
    capability: &str,
) -> Result<String, AppError> {
    let marker = text[heading.start..]
        .lines()
        .skip(1)
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| {
            AppError::new(
                "missing_requirement_id",
                format!("{capability}: {}", heading.title),
            )
        })?
        .trim();
    let id = marker
        .strip_prefix("<!-- dwv:req ")
        .and_then(|value| value.strip_suffix(" -->"))
        .ok_or_else(|| {
            AppError::new(
                "missing_requirement_id",
                format!("{capability}: {}", heading.title),
            )
        })?;
    let expected = format!("req.{capability}.");
    if !id.starts_with(&expected) {
        return Err(AppError::new("invalid_requirement_id", id));
    }
    Ok(id.to_owned())
}

fn validate_semantic_id(id: &str) -> Result<(), AppError> {
    let parts = id.split('.').collect::<Vec<_>>();
    let valid = |part: &&str| {
        !part.is_empty()
            && !part.starts_with('-')
            && !part.ends_with('-')
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    if parts.len() != 3 || parts[0] != "req" || !valid(&parts[1]) || !valid(&parts[2]) {
        return Err(AppError::new("malformed_requirement_id", id));
    }
    Ok(())
}

fn extract_ids(text: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let mut offset = 0;
    while let Some(found) = text[offset..].find("req.") {
        let start = offset + found;
        let mut end = start;
        for (index, character) in text[start..].char_indices() {
            if character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '-'
                || character == '.'
            {
                end = start + index + character.len_utf8();
            } else {
                break;
            }
        }
        let candidate = text[start..end].trim_end_matches('.');
        if validate_semantic_id(candidate).is_ok() {
            ids.insert(candidate.to_owned());
        }
        offset = end.max(start + 4);
    }
    ids
}

fn sphinx_id(id: &str) -> String {
    format!(
        "R_{}",
        blake3::hash(id.as_bytes()).to_hex().to_string()[..20].to_ascii_uppercase()
    )
}
fn check_mapping_collisions<'a>(
    mappings: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<(), AppError> {
    let mut seen = BTreeMap::new();
    for (semantic, sphinx) in mappings {
        if let Some(previous) = seen.insert(sphinx, semantic)
            && previous != semantic
        {
            return Err(AppError::new(
                "sphinx_id_collision",
                format!("{sphinx}: {previous} and {semantic}"),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture(name: &str) -> (PathBuf, App) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dwv-knowledge-{name}-{nonce}"));
        fs::create_dir_all(root.join("openspec/specs/cap")).unwrap();
        let app = App::new(root.clone());
        (root, app)
    }

    fn requirement(text: &str) -> String {
        format!(
            "# cap Specification\n\n## Requirements\n\n### Requirement: One\n<!-- dwv:req req.cap.one -->\n\n{text}\n"
        )
    }

    #[test]
    fn malformed_missing_duplicate_ids_rejected() {
        assert!(validate_semantic_id("").is_err());
        assert!(validate_semantic_id("req.cap.Bad").is_err());
        let mut ids = BTreeSet::new();
        assert!(ids.insert("req.a.one"));
        assert!(!ids.insert("req.a.one"));
    }
    #[test]
    fn mapping_collision_rejected() {
        assert!(check_mapping_collisions([("req.a.one", "R_A"), ("req.b.two", "R_A")]).is_err());
    }
    #[test]
    fn mapping_is_stable() {
        assert_eq!(sphinx_id("req.a.one"), sphinx_id("req.a.one"));
        assert_eq!(sphinx_id("req.a.one").len(), 22);
    }
    #[test]
    fn all_resolution_outcomes_are_explicit() {
        for outcome in ["reviewed", "reference-only", "deferred", "superseded"] {
            assert!(OUTCOMES.contains(&outcome));
        }
    }
    #[test]
    fn unknown_references_are_not_current() {
        let current = BTreeSet::from(["req.a.one".to_owned()]);
        let ids = extract_ids("req.a.one req.unknown.two");
        assert!(ids.difference(&current).any(|id| id == "req.unknown.two"));
    }
    #[test]
    fn references_exclude_history_hidden_and_milestones() {
        assert!(excluded("docs/handoffs/old.md"));
        assert!(excluded("target/generated.md"));
        assert!(excluded("docs/milestones/OS-001.md"));
        assert!(!excluded("docs/verification/current.md"));
        assert!(!is_impact_candidate("docs/milestones/OS-001.md"));
    }
    #[test]
    fn milestone_markdown_cannot_enter_reference_context() {
        let (root, app) = fixture("milestone-context");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        fs::create_dir_all(root.join("docs/milestones")).unwrap();
        let milestone = [
            "<!-- dwv:req req.cap.one -->\n/// dwv:",
            "req req.cap.one\nverification req.cap.one\n",
        ]
        .concat();
        fs::write(root.join("docs/milestones/OS-001.md"), milestone).unwrap();

        let references = scan_references(&app).unwrap();
        assert!(
            references
                .iter()
                .all(|reference| !reference.path.starts_with("docs/milestones/"))
        );
        let packet = context(&app, "req.cap.one".to_owned()).unwrap();
        assert!(packet["inbound"].as_array().unwrap().is_empty());
        assert!(packet["outbound"].as_array().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn milestone_paths_are_rejected_as_explicit_affected_inputs() {
        let (root, app) = fixture("milestone-affected");
        fs::create_dir_all(root.join("docs/milestones")).unwrap();
        fs::write(root.join("docs/milestones/OS-001.md"), "req.cap.one\n").unwrap();
        fs::write(
            root.join(REVIEWED_PATH),
            toml::to_string(&ReviewedState {
                schema: REVIEWED_SCHEMA.to_owned(),
                requirements: BTreeMap::new(),
                outcomes: BTreeMap::new(),
                reasons: BTreeMap::new(),
            })
            .unwrap(),
        )
        .unwrap();

        let error = affected(
            &app,
            Vec::new(),
            vec!["docs/milestones/OS-001.md".to_owned()],
        )
        .unwrap_err();
        assert_eq!(error.code, "affected_path_excluded");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn ids_are_bounded_to_valid_shape() {
        let ids = extract_ids("req.a.one req.a.two.");
        assert_eq!(ids.len(), 2);
    }
    #[test]
    fn rust_relationship_recovery_reads_only_markers() {
        let marker = [
            "let sample = \"req.a.fake\";\n/// dwv:",
            "req req.a.real\nfn owner() {}\n",
        ]
        .concat();
        let ids = relationship_ids("crates/example.rs", &marker);
        assert_eq!(ids, BTreeSet::from(["req.a.real".to_owned()]));
    }

    #[test]
    fn jujutsu_rename_summary_expands_both_paths() {
        assert_eq!(
            expand_rename("crates/{old.rs => new.rs}"),
            ["crates/old.rs", "crates/new.rs"]
        );
    }

    #[test]
    fn git_change_boundary_includes_tracked_and_untracked_files() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let (root, _) = fixture("git-impact");
        fs::write(root.join("tracked.rs"), "fn before() {}\n").unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-q"])
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("git")
                .args(["add", "tracked.rs"])
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("git")
                .args([
                    "-c",
                    "user.name=DiskWeave Test",
                    "-c",
                    "user.email=test@diskweave.invalid",
                    "commit",
                    "-qm",
                    "baseline",
                ])
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
        fs::write(root.join("tracked.rs"), "fn after() {}\n").unwrap();
        fs::write(root.join("untracked.rs"), "fn added() {}\n").unwrap();

        let paths = RevisionControl::Git.changed_paths(&root, "HEAD").unwrap();
        assert_eq!(
            paths,
            BTreeSet::from(["tracked.rs".to_owned(), "untracked.rs".to_owned()])
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn change_impact_reports_unavailable_baseline_outside_revision_control() {
        let (root, app) = fixture("impact-no-baseline");
        let result = change_impact(&app, None).unwrap();

        assert_eq!(result["baseline_available"], false);
        assert_eq!(result["review_required"], false);
        let diagnostics = result["diagnostics"].as_array().unwrap();
        assert!(!diagnostics.is_empty());
        assert!(diagnostics[0].as_str().is_some_and(|diagnostic| {
            !diagnostic.is_empty() && diagnostic.contains("baseline unavailable")
        }));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn human_guide_curriculum_requires_section_briefs() {
        let (root, app) = fixture("curriculum-sections");
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(
            root.join("docs/curriculum.toml"),
            "schema = 'dwv.docs.curriculum.v1'\n[[entries]]\nid = 'guide.one'\nprojection = 'human_guide'\naudience = 'newcomer'\nquestion = 'Why?'\nmisconceptions = ['none']\nrequirements = ['req.cap.one']\n",
        )
        .unwrap();
        assert_eq!(
            validate_curriculum(&app).unwrap_err().code,
            "curriculum_invalid"
        );

        fs::write(
            root.join("docs/curriculum.toml"),
            "schema = 'dwv.docs.curriculum.v1'\n[[entries]]\nid = 'guide.one'\nprojection = 'human_guide'\naudience = 'newcomer'\nquestion = 'Why?'\nmisconceptions = ['none']\nrequirements = ['req.cap.one']\n[[entries.sections]]\nheading = 'Start here'\nfocus = 'Explain one event.'\nmust_answer = ['What happened?']\nrequirements = ['req.cap.one']\nscenarios = []\nnon_claims = ['No durability claim.']\n",
        )
        .unwrap();
        validate_curriculum(&app).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn no_op_rendering_is_stable() {
        let state: ReviewedState = toml::from_str(
            "schema = 'dwv.knowledge.reviewed-links.v1'\n[requirements]\n\"req.a.one\" = \"abc\"\n",
        )
        .unwrap();
        assert_eq!(
            state,
            toml::from_str(&toml::to_string(&state).unwrap()).unwrap()
        );
    }

    #[test]
    fn affected_path_selects_changed_requirement_and_reports_omissions() {
        let (root, mut app) = fixture("affected");
        let spec = root.join("openspec/specs/cap/spec.md");
        fs::write(&spec, requirement("The system SHALL remain stable.")).unwrap();
        let object = objects(&app).unwrap().remove(0);
        let state = ReviewedState {
            schema: REVIEWED_SCHEMA.to_owned(),
            requirements: BTreeMap::from([(
                object.semantic_id.clone(),
                object.normalized_fingerprint,
            )]),
            outcomes: BTreeMap::new(),
            reasons: BTreeMap::new(),
        };
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/one.md"), "First req.cap.one explanation.\n").unwrap();
        fs::write(
            root.join("docs/two.md"),
            "Second req.cap.one explanation.\n",
        )
        .unwrap();
        fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();
        fs::write(&spec, requirement("The system SHALL remain durable.")).unwrap();
        app.bounds.max_units = 1;

        let result = affected(
            &app,
            Vec::new(),
            vec!["openspec/specs/cap/spec.md".to_owned()],
        )
        .unwrap();
        assert_eq!(result["impacts"].as_array().unwrap().len(), 1);
        assert_eq!(result["impacts"][0]["semantic_id"], "req.cap.one");
        assert_eq!(result["impacts"][0]["context"]["omitted"]["outbound"], 1);
        assert_eq!(result["impacts"][0]["pages"].as_array().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reviewed_lifecycle_distinguishes_formatting_and_semantic_changes() {
        let (root, app) = fixture("lifecycle");
        let spec = root.join("openspec/specs/cap/spec.md");
        fs::write(&spec, requirement("The system SHALL remain stable.")).unwrap();
        let object = objects(&app).unwrap().remove(0);
        let state = ReviewedState {
            schema: REVIEWED_SCHEMA.to_owned(),
            requirements: BTreeMap::from([(
                object.semantic_id.clone(),
                object.normalized_fingerprint,
            )]),
            outcomes: BTreeMap::new(),
            reasons: BTreeMap::new(),
        };
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("docs/handoffs")).unwrap();
        fs::write(
            root.join("docs/handoffs/architecture.md"),
            "# Architecture\n<!-- dwv:active-architecture-roadmap -->\n",
        )
        .unwrap();
        let affected = root.join("docs/affected.md");
        let unaffected = root.join("docs/unaffected.md");
        fs::write(&affected, "Explanation for req.cap.one.\n").unwrap();
        fs::write(&unaffected, "Independent explanation.\n").unwrap();
        let unaffected_before = fs::read(&unaffected).unwrap();
        fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();
        assert!(readiness(&app).is_ok());

        fs::write(&spec, requirement("The system SHALL remain\nstable.")).unwrap();
        assert!(readiness(&app).is_ok());

        fs::write(&spec, requirement("The system SHALL remain durable.")).unwrap();
        let error = readiness(&app).unwrap_err();
        assert_eq!(error.code, "knowledge_not_ready");
        assert_eq!(
            error.details.unwrap()["gate_counts"]["fingerprint-suspect"],
            1
        );
        let packet = context(&app, "req.cap.one".to_owned()).unwrap();
        assert_eq!(packet["outbound"].as_array().unwrap().len(), 1);
        assert_eq!(packet["outbound"][0]["path"], "docs/affected.md");
        fs::write(
            &affected,
            "Updated durability explanation for req.cap.one.\n",
        )
        .unwrap();
        assert_eq!(fs::read(&unaffected).unwrap(), unaffected_before);

        assert!(resolve(&app, "req.cap.one", "reviewed", "semantic review").is_ok());
        let first = fs::read(root.join(REVIEWED_PATH)).unwrap();
        let result = resolve(&app, "req.cap.one", "reviewed", "semantic review").unwrap();
        assert_eq!(result["changed"], false);
        assert_eq!(fs::read(root.join(REVIEWED_PATH)).unwrap(), first);
        assert!(readiness(&app).is_ok());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn reference_scan_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let (root, app) = fixture("symlink");
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/real.md"), "req.cap.one").unwrap();
        symlink(root.join("docs/real.md"), root.join("docs/link.md")).unwrap();
        assert_eq!(scan_references(&app).unwrap_err().code, "reference_symlink");
        fs::remove_dir_all(root).unwrap();
    }
    fn roadmap_fixture(name: &str) -> (PathBuf, App) {
        let (root, app) = fixture(name);
        fs::create_dir_all(root.join("docs/handoffs")).unwrap();
        fs::create_dir_all(root.join("openspec/changes/archive")).unwrap();
        fs::write(
            root.join("docs/handoffs/architecture.md"),
            "# Architecture\n<!-- dwv:active-architecture-roadmap -->\n\nNarrative mentions OS-029 only.\n\n| OpenSpec | Result |\n|---|---|\n| **OS-030** | ublk conformance |\n| **OS-031** | Linux executor |\n",
        )
        .unwrap();
        (root, app)
    }

    #[test]
    fn roadmap_validator_accepts_matching_numeric_and_descriptive_changes() {
        let (root, app) = roadmap_fixture("roadmap-valid");
        fs::create_dir_all(root.join("openspec/changes/os-030-ublk-conformance")).unwrap();
        fs::write(
            root.join("openspec/changes/os-030-ublk-conformance/proposal.md"),
            "<!-- dwv:roadmap-node OS-030 -->\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("openspec/changes/request-fix")).unwrap();
        fs::write(
            root.join("openspec/changes/request-fix/proposal.md"),
            "Descriptive changes need no roadmap node.\n",
        )
        .unwrap();

        validate_roadmap_identifiers(&app).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn roadmap_validator_accepts_archived_change_missing_from_roadmap() {
        let (root, app) = roadmap_fixture("roadmap-archived-missing-node");
        fs::create_dir_all(root.join("openspec/changes/archive/2026-08-09-os-029-unrelated-work"))
            .unwrap();

        validate_roadmap_identifiers(&app).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn roadmap_validator_reports_every_marked_document() {
        let (root, app) = roadmap_fixture("roadmap-marker-count");
        fs::write(
            root.join("docs/other.md"),
            "<!-- dwv:active-architecture-roadmap -->\n",
        )
        .unwrap();

        let error = validate_roadmap_identifiers(&app).unwrap_err();
        assert_eq!(error.code, "active_roadmap_marker_count");
        assert_eq!(error.details.unwrap()["paths"].as_array().unwrap().len(), 2);

        fs::remove_file(root.join("docs/other.md")).unwrap();
        fs::write(
            root.join("docs/handoffs/architecture.md"),
            "<!-- dwv:active-architecture-roadmap -->\n<!-- dwv:active-architecture-roadmap -->\n",
        )
        .unwrap();
        let error = validate_roadmap_identifiers(&app).unwrap_err();
        assert_eq!(error.details.as_ref().unwrap()["marker_count"], 2);
        assert_eq!(error.details.unwrap()["paths"].as_array().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn roadmap_validator_rejects_unknown_mismatch_and_duplicate_claims() {
        let (root, app) = roadmap_fixture("roadmap-invalid");
        fs::create_dir_all(root.join("openspec/changes/os-030-wrong-declaration")).unwrap();
        fs::write(
            root.join("openspec/changes/os-030-wrong-declaration/proposal.md"),
            "<!-- dwv:roadmap-node OS-031 -->\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("openspec/changes/archive/2026-08-09-os-030-older-claim"))
            .unwrap();
        fs::create_dir_all(root.join("openspec/changes/os-029-unrelated-work")).unwrap();

        let error = validate_roadmap_identifiers(&app).unwrap_err();
        assert_eq!(error.code, "roadmap_identifier_invalid");
        let details = error.details.unwrap();
        let gates = details["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|diagnostic| diagnostic["gate"].as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            gates,
            BTreeSet::from([
                "duplicate-numeric-change-identity",
                "numeric-change-not-in-roadmap",
                "numeric-proposal-roadmap-declaration",
            ])
        );
        fs::remove_dir_all(root).unwrap();
    }
}
