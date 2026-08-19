use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::Path;
use std::process::Command;

use super::{
    App, AppError, atomic_write, digest_text, markdown_headings, normalize_markdown, read_toml,
    safe_join, write_json,
};

const OBJECTS_SCHEMA: &str = "dwv.knowledge.objects.v3";
const INSPECT_SCHEMA: &str = "dwv.knowledge.inspect.v2";
const CONTEXT_SCHEMA: &str = "dwv.knowledge.context.v4";
const OWNERSHIP_SCHEMA: &str = "dwv.knowledge.ownership.v3";
const AFFECTED_SCHEMA: &str = "dwv.knowledge.affected.v3";
const AUDIT_CONTEXT_SCHEMA: &str = "dwv.knowledge.audit-context.v2";
const ARCHITECTURE_CANDIDATE_SCHEMA: &str = "dwv.knowledge.architecture-candidate.v1";
const ARCHITECTURE_HISTORY_SCHEMA: &str = "dwv.knowledge.architecture-history.v1";
const OBJECTS_PATH: &str = "target/dwv-docs/knowledge/objects.json";
const REVIEWED_PATH: &str = "docs/reviewed-requirements.toml";
const READINESS_SCHEMA: &str = "dwv.knowledge.readiness.v3";
const REVIEWED_SCHEMA: &str = "dwv.knowledge.reviewed-links.v2";
const OUTCOMES: [&str; 4] = ["reviewed", "reference-only", "deferred", "superseded"];

const ACTIVE_ROADMAP_MARKER: &str = "<!-- dwv:active-architecture-roadmap -->";
const MAX_AUTHORITY_FILE_BYTES: usize = 1024 * 1024;
const RETIRED_PATH: &str = "docs/milestones";

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub(super) struct RequirementObject {
    pub semantic_id: String,
    pub sphinx_id: String,
    pub title: String,
    pub capability: String,
    pub source_path: String,
    pub heading_path: Vec<String>,
    #[serde(skip)]
    pub source_start_line: usize,
    #[serde(skip)]
    pub source_end_line: usize,
    pub local_semantic_fingerprint: String,
    pub effective_semantic_fingerprint: String,
    pub requires: Vec<String>,
    pub refines: Vec<String>,
    pub required_by: Vec<String>,
    pub refined_by: Vec<String>,
    pub constrained_by: Vec<String>,
    pub body: String,
}

impl RequirementObject {
    fn source_locator(&self) -> String {
        format!(
            "{}:{}-{}",
            self.source_path, self.source_start_line, self.source_end_line
        )
    }
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct ArchitectureDocument {
    id: String,
    series: String,
    revision: String,
    kind: String,
    status: String,
    scope: String,
    source_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    supersedes: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct ArchitectureInvariant {
    semantic_id: String,
    title: String,
    document_id: String,
    lifecycle: String,
    source_path: String,
    heading_path: Vec<String>,
    #[serde(skip)]
    source_start_line: usize,
    #[serde(skip)]
    source_end_line: usize,
    local_semantic_fingerprint: String,
    constrains: Vec<String>,
    supersedes: Vec<String>,
    superseded_by: Vec<String>,
    body: String,
}

impl ArchitectureInvariant {
    fn source_locator(&self) -> String {
        format!(
            "{}:{}-{}",
            self.source_path, self.source_start_line, self.source_end_line
        )
    }
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ArchitectureConstraint {
    source: String,
    target: String,
}

#[derive(Debug, Clone)]
struct ArchitectureCatalog {
    documents: Vec<ArchitectureDocument>,
    invariants: Vec<ArchitectureInvariant>,
    active_document_id: String,
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
    local_fingerprints: BTreeMap<String, String>,
    #[serde(default)]
    effective_fingerprints: BTreeMap<String, String>,
    #[serde(default)]
    outcomes: BTreeMap<String, String>,
    #[serde(default)]
    reasons: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScanRef {
    id: String,
    kind: String,
    path: String,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticRelationship {
    source: String,
    kind: String,
    target: String,
    source_path: String,
    line: usize,
}

#[derive(Debug)]
struct KnowledgeModel {
    objects: Vec<RequirementObject>,
    relationships: Vec<SemanticRelationship>,
    architecture: ArchitectureCatalog,
    constraints: Vec<ArchitectureConstraint>,
}

pub(super) fn objects(app: &App) -> Result<Vec<RequirementObject>, AppError> {
    Ok(knowledge_model(app)?.objects)
}

fn knowledge_model(app: &App) -> Result<KnowledgeModel, AppError> {
    retired_path_preflight(app)?;
    let root = app.root.join("openspec/specs");
    let mut dirs = entries(&root, "spec_discovery_failed")?
        .into_iter()
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_dir() && !kind.is_symlink())
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    dirs.sort_by_key(|entry| entry.file_name());

    let mut objects = Vec::new();
    let mut relationships = Vec::new();
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
            .filter(|(_, heading)| heading.title.starts_with("Requirement: "))
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
            let source_start_line = text[..heading.start].lines().count() + 1;
            let source_end_line = text[..end].lines().count().max(source_start_line);
            if body.len() > app.bounds.max_source_bytes {
                return Err(AppError::new("unit_bound_exceeded", id));
            }
            relationships.extend(extract_relationships(&text, heading, end, &id, &relative)?);
            objects.push(RequirementObject {
                semantic_id: id.clone(),
                sphinx_id: sphinx_id(&id),
                title: heading.title.trim_start_matches("Requirement: ").to_owned(),
                capability: capability.clone(),
                source_path: relative.clone(),
                heading_path: heading.path.clone(),
                source_start_line,
                source_end_line,
                local_semantic_fingerprint: digest_text(&body),
                effective_semantic_fingerprint: String::new(),
                requires: Vec::new(),
                refines: Vec::new(),
                required_by: Vec::new(),
                refined_by: Vec::new(),
                constrained_by: Vec::new(),
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
    objects.sort_by(|left, right| left.semantic_id.cmp(&right.semantic_id));
    relationships.sort();
    check_mapping_collisions(
        objects
            .iter()
            .map(|object| (object.semantic_id.as_str(), object.sphinx_id.as_str())),
    )?;
    validate_relationships(&objects, &relationships)?;

    let indexes = objects
        .iter()
        .enumerate()
        .map(|(index, object)| (object.semantic_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<String, Vec<SemanticRelationship>>::new();
    for relationship in &relationships {
        outgoing
            .entry(relationship.source.clone())
            .or_default()
            .push(relationship.clone());
        let source = indexes[&relationship.source];
        let target = indexes[&relationship.target];
        match relationship.kind.as_str() {
            "requires" => {
                objects[source].requires.push(relationship.target.clone());
                objects[target]
                    .required_by
                    .push(relationship.source.clone());
            }
            "refines" => {
                objects[source].refines.push(relationship.target.clone());
                objects[target].refined_by.push(relationship.source.clone());
            }
            _ => unreachable!("relationship kinds are validated"),
        }
    }
    for object in &mut objects {
        object.requires.sort();
        object.refines.sort();
        object.required_by.sort();
        object.refined_by.sort();
    }
    let locals = objects
        .iter()
        .map(|object| {
            (
                object.semantic_id.clone(),
                object.local_semantic_fingerprint.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let architecture = architecture_catalog(app)?;
    let constraints = validate_architecture_semantics(&mut objects, &architecture)?;
    let active_inputs = active_constraint_inputs(&architecture);
    let mut effective = BTreeMap::new();
    for id in indexes.keys() {
        effective_fingerprint(id, &locals, &outgoing, &active_inputs, &mut effective);
    }
    for object in &mut objects {
        object.effective_semantic_fingerprint = effective[&object.semantic_id].clone();
    }
    Ok(KnowledgeModel {
        objects,
        relationships,
        architecture,
        constraints,
    })
}

fn architecture_catalog(app: &App) -> Result<ArchitectureCatalog, AppError> {
    retired_path_preflight(app)?;
    let root = app.root.join("docs/architecture");
    let mut files = Vec::new();
    collect_markdown_files(&app.root, &root, &mut files)?;
    files.sort();
    let mut documents = Vec::new();
    let mut invariants = Vec::new();
    let mut markers = Vec::new();
    for path in files {
        let relative = rel(&app.root, &path);
        let text = read_authority_file(&app.root, &path)?;
        let Some(metadata) = architecture_metadata(&text, &relative)? else {
            continue;
        };
        let marker_count = text
            .lines()
            .filter(|line| line.trim() == ACTIVE_ROADMAP_MARKER)
            .count();
        if marker_count > 0 {
            markers.extend(std::iter::repeat_n(relative.clone(), marker_count));
        }
        let document = ArchitectureDocument {
            id: required_metadata(&metadata, "id", &relative)?.to_owned(),
            series: required_metadata(&metadata, "series", &relative)?.to_owned(),
            revision: required_metadata(&metadata, "revision", &relative)?.to_owned(),
            kind: required_metadata(&metadata, "kind", &relative)?.to_owned(),
            status: required_metadata(&metadata, "status", &relative)?.to_owned(),
            scope: required_metadata(&metadata, "scope", &relative)?.to_owned(),
            source_path: relative.clone(),
            supersedes: metadata.get("supersedes").cloned(),
        };
        validate_architecture_document(&document)?;
        invariants.extend(extract_architecture_invariants(app, &text, &document)?);
        documents.push(document);
    }
    populate_superseded_by(&mut invariants);
    documents.sort_by(|left, right| left.id.cmp(&right.id));
    invariants.sort_by(|left, right| {
        (&left.document_id, &left.semantic_id).cmp(&(&right.document_id, &right.semantic_id))
    });
    validate_architecture_catalog(&documents, &invariants, &markers)?;
    let active_document_id = documents
        .iter()
        .find(|document| document.status == "active")
        .expect("catalog validation requires one active document")
        .id
        .clone();
    Ok(ArchitectureCatalog {
        documents,
        invariants,
        active_document_id,
    })
}

fn architecture_metadata(
    text: &str,
    source_path: &str,
) -> Result<Option<BTreeMap<String, String>>, AppError> {
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Ok(None);
    }
    let mut metadata = BTreeMap::new();
    let mut closed = false;
    for line in lines {
        if line == "---" {
            closed = true;
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(architecture_error(
                "architecture_metadata_invalid",
                source_path,
                format!("metadata line is not a scalar key-value pair: {line}"),
            ));
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() || value.contains(['[', ']', '{', '}', '\n']) {
            return Err(architecture_error(
                "architecture_metadata_invalid",
                source_path,
                format!("metadata field {key:?} must have one non-empty scalar value"),
            ));
        }
        if metadata.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(architecture_error(
                "architecture_metadata_duplicate",
                source_path,
                format!("metadata field {key} occurs more than once"),
            ));
        }
    }
    if !closed {
        return Err(architecture_error(
            "architecture_metadata_invalid",
            source_path,
            "leading metadata block is not closed",
        ));
    }
    if metadata.get("kind").map(String::as_str) == Some("architecture-roadmap") {
        Ok(Some(metadata))
    } else {
        Ok(None)
    }
}

fn required_metadata<'a>(
    metadata: &'a BTreeMap<String, String>,
    key: &str,
    source_path: &str,
) -> Result<&'a str, AppError> {
    metadata.get(key).map(String::as_str).ok_or_else(|| {
        architecture_error(
            "architecture_metadata_missing",
            source_path,
            format!("required metadata field {key} is absent"),
        )
    })
}

fn validate_architecture_document(document: &ArchitectureDocument) -> Result<(), AppError> {
    validate_architecture_id(&document.id).map_err(|_| {
        architecture_error(
            "architecture_document_id_invalid",
            &document.source_path,
            &document.id,
        )
    })?;
    validate_architecture_id(&document.series).map_err(|_| {
        architecture_error(
            "architecture_series_invalid",
            &document.source_path,
            &document.series,
        )
    })?;
    if document.kind != "architecture-roadmap" {
        return Err(architecture_error(
            "architecture_kind_invalid",
            &document.source_path,
            &document.kind,
        ));
    }
    if document.scope != "whole-system" {
        return Err(architecture_error(
            "architecture_scope_invalid",
            &document.source_path,
            &document.scope,
        ));
    }
    if !matches!(
        document.status.as_str(),
        "active" | "candidate" | "superseded"
    ) {
        return Err(architecture_error(
            "architecture_status_invalid",
            &document.source_path,
            &document.status,
        ));
    }
    if document.revision.trim().is_empty() {
        return Err(architecture_error(
            "architecture_revision_invalid",
            &document.source_path,
            "revision is empty",
        ));
    }
    if let Some(predecessor) = &document.supersedes {
        validate_architecture_id(predecessor).map_err(|_| {
            architecture_error(
                "architecture_supersedes_invalid",
                &document.source_path,
                predecessor,
            )
        })?;
        if predecessor == &document.id {
            return Err(architecture_error(
                "architecture_supersedes_self",
                &document.source_path,
                predecessor,
            ));
        }
    }
    Ok(())
}

fn validate_architecture_catalog(
    documents: &[ArchitectureDocument],
    invariants: &[ArchitectureInvariant],
    markers: &[String],
) -> Result<(), AppError> {
    if documents.is_empty() {
        return Err(AppError::new(
            "architecture_document_missing",
            "no whole-system architecture-roadmap document found",
        ));
    }
    let mut ids = BTreeMap::new();
    let mut series = BTreeSet::new();
    for document in documents {
        if let Some(previous) = ids.insert(document.id.as_str(), document) {
            return Err(
                AppError::new("architecture_document_duplicate", document.id.clone())
                    .details(json!({"paths": [previous.source_path, document.source_path]})),
            );
        }
        series.insert(document.series.as_str());
    }
    if series.len() != 1 {
        return Err(AppError::new(
            "architecture_series_ambiguous",
            "whole-system architecture documents must share one series",
        )
        .details(json!({"series": series})));
    }
    let active = documents
        .iter()
        .filter(|document| document.status == "active")
        .collect::<Vec<_>>();
    if active.len() != 1 || markers.len() != 1 || markers[0] != active[0].source_path {
        return Err(AppError::new(
            "active_architecture_selection_invalid",
            "exactly one active document must carry the sole active marker",
        )
        .details(json!({
            "active_documents": active.iter().map(|document| &document.source_path).collect::<Vec<_>>(),
            "marker_paths": markers,
        })));
    }
    for document in documents {
        if let Some(predecessor) = &document.supersedes
            && let Some(target) = ids.get(predecessor.as_str())
            && target.series != document.series
        {
            return Err(architecture_error(
                "architecture_supersedes_cross_series",
                &document.source_path,
                predecessor,
            ));
        }
    }
    if let Some(cycle) = document_supersession_cycle(documents) {
        return Err(AppError::new(
            "architecture_supersession_cycle",
            "document supersession contains a cycle",
        )
        .details(json!({"cycle": cycle})));
    }
    let mut active_ids = BTreeSet::new();
    let mut per_document = BTreeSet::new();
    let document_ids = ids.keys().copied().collect::<BTreeSet<_>>();
    for invariant in invariants {
        if document_ids.contains(invariant.semantic_id.as_str()) {
            return Err(architecture_error(
                "architecture_invariant_id_collision",
                &invariant.source_path,
                &invariant.semantic_id,
            ));
        }
        if !per_document.insert((
            invariant.document_id.as_str(),
            invariant.semantic_id.as_str(),
        )) {
            return Err(architecture_error(
                "architecture_invariant_duplicate",
                &invariant.source_path,
                &invariant.semantic_id,
            ));
        }
        if invariant.lifecycle == "active" && !active_ids.insert(invariant.semantic_id.as_str()) {
            return Err(architecture_error(
                "active_architecture_invariant_duplicate",
                &invariant.source_path,
                &invariant.semantic_id,
            ));
        }
    }
    Ok(())
}

fn document_supersession_cycle(documents: &[ArchitectureDocument]) -> Option<Vec<String>> {
    let present = documents
        .iter()
        .map(|document| document.id.as_str())
        .collect::<BTreeSet<_>>();
    let edges = documents
        .iter()
        .filter_map(|document| {
            document
                .supersedes
                .as_ref()
                .filter(|target| present.contains(target.as_str()))
                .map(|target| SemanticRelationship {
                    source: document.id.clone(),
                    kind: "supersedes".to_owned(),
                    target: target.clone(),
                    source_path: document.source_path.clone(),
                    line: 1,
                })
        })
        .collect::<Vec<_>>();
    smallest_cycle(&edges)
}

fn extract_architecture_invariants(
    app: &App,
    text: &str,
    document: &ArchitectureDocument,
) -> Result<Vec<ArchitectureInvariant>, AppError> {
    let headings = markdown_headings(text)?;
    let mut invariants = Vec::new();
    for (index, heading) in headings
        .iter()
        .enumerate()
        .filter(|(_, heading)| heading.title.starts_with("Invariant:"))
    {
        let end = headings
            .iter()
            .skip(index + 1)
            .find(|next| next.level <= heading.level)
            .map(|next| next.start)
            .unwrap_or(text.len());
        let section = &text[heading.start..end];
        let lines = section.lines().collect::<Vec<_>>();
        let Some((identity_line, marker)) = lines
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, line)| !line.trim().is_empty())
        else {
            continue;
        };
        let Some(id) = parse_marker(marker, "dwv:arch-invariant") else {
            if marker.trim().contains("dwv:arch-invariant") {
                return Err(architecture_error(
                    "architecture_invariant_marker_invalid",
                    &document.source_path,
                    marker.trim(),
                ));
            }
            continue;
        };
        validate_architecture_id(id).map_err(|_| {
            architecture_error(
                "architecture_invariant_id_invalid",
                &document.source_path,
                id,
            )
        })?;
        let mut constrains = Vec::new();
        let mut supersedes = Vec::new();
        let mut cursor = identity_line + 1;
        while cursor < lines.len() {
            if lines[cursor].trim().is_empty() {
                cursor += 1;
                continue;
            }
            if let Some(target) = parse_marker(lines[cursor], "dwv:constrains") {
                constrains.push(target.to_owned());
            } else if let Some(target) = parse_marker(lines[cursor], "dwv:arch-supersedes") {
                supersedes.push(target.to_owned());
            } else {
                break;
            }
            cursor += 1;
        }
        let normalized = normalize_markdown(section)?;
        let body = normalize_invariant_heading(&normalized);
        if body.len() > app.bounds.max_source_bytes {
            return Err(AppError::new("unit_bound_exceeded", id));
        }
        let source_start_line = text[..heading.start].lines().count() + 1;
        let source_end_line = text[..end].lines().count().max(source_start_line);
        invariants.push(ArchitectureInvariant {
            semantic_id: id.to_owned(),
            title: heading
                .title
                .trim_start_matches("Invariant:")
                .trim()
                .to_owned(),
            document_id: document.id.clone(),
            lifecycle: document.status.clone(),
            source_path: document.source_path.clone(),
            heading_path: heading.path.clone(),
            source_start_line,
            source_end_line,
            local_semantic_fingerprint: digest_text(&body),
            constrains,
            supersedes,
            superseded_by: Vec::new(),
            body,
        });
    }
    Ok(invariants)
}

fn normalize_invariant_heading(normalized: &str) -> String {
    let Some((heading, rest)) = normalized.split_once('\n') else {
        return normalized.to_owned();
    };
    if heading.starts_with('H') && heading.contains(":Invariant:") {
        format!("H:{}", heading.split_once(':').unwrap().1)
            + if rest.is_empty() { "" } else { "\n" }
            + rest
    } else {
        normalized.to_owned()
    }
}

fn parse_marker<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    line.trim()
        .strip_prefix("<!-- ")
        .and_then(|line| line.strip_suffix(" -->"))
        .and_then(|line| line.strip_prefix(name))
        .and_then(|line| line.strip_prefix(' '))
        .filter(|value| !value.is_empty() && !value.contains(char::is_whitespace))
}

fn populate_superseded_by(invariants: &mut [ArchitectureInvariant]) {
    let edges = invariants
        .iter()
        .flat_map(|invariant| {
            invariant
                .supersedes
                .iter()
                .map(|predecessor| (predecessor.clone(), invariant.semantic_id.clone()))
        })
        .collect::<Vec<_>>();
    for invariant in invariants {
        invariant.superseded_by = edges
            .iter()
            .filter(|(predecessor, _)| predecessor == &invariant.semantic_id)
            .map(|(_, successor)| successor.clone())
            .collect();
        invariant.superseded_by.sort();
        invariant.superseded_by.dedup();
    }
}

fn validate_architecture_id(id: &str) -> Result<(), AppError> {
    let parts = id.split('.').collect::<Vec<_>>();
    let valid = |part: &&str| {
        !part.is_empty()
            && !part.starts_with('-')
            && !part.ends_with('-')
            && part.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
            })
    };
    if parts.len() < 2 || parts[0] != "arch" || !parts[1..].iter().all(valid) {
        return Err(AppError::new("malformed_architecture_id", id));
    }
    Ok(())
}

fn architecture_error(code: &str, source_path: &str, message: impl Into<String>) -> AppError {
    AppError::new(code, message).details(json!({"source_path": source_path}))
}
fn validate_architecture_semantics(
    objects: &mut [RequirementObject],
    catalog: &ArchitectureCatalog,
) -> Result<Vec<ArchitectureConstraint>, AppError> {
    let indexes = objects
        .iter()
        .enumerate()
        .map(|(index, object)| (object.semantic_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let locations = catalog
        .invariants
        .iter()
        .enumerate()
        .map(|(index, invariant)| {
            (
                (
                    invariant.document_id.as_str(),
                    invariant.semantic_id.as_str(),
                ),
                index,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let identities = catalog.invariants.iter().fold(
        BTreeMap::<&str, Vec<usize>>::new(),
        |mut result, invariant| {
            result
                .entry(invariant.semantic_id.as_str())
                .or_default()
                .push(
                    locations[&(
                        invariant.document_id.as_str(),
                        invariant.semantic_id.as_str(),
                    )],
                );
            result
        },
    );
    let mut constraints = BTreeSet::new();
    let mut lineage = Vec::new();
    for invariant in &catalog.invariants {
        let mut targets = BTreeSet::new();
        for target in &invariant.constrains {
            validate_semantic_id(target).map_err(|_| {
                architecture_semantic_error(
                    "architecture_constraint_target_invalid",
                    invariant,
                    target,
                )
            })?;
            if !targets.insert(target.as_str()) {
                return Err(architecture_semantic_error(
                    "architecture_constraint_duplicate",
                    invariant,
                    target,
                ));
            }
            if invariant.lifecycle != "superseded" && !indexes.contains_key(target) {
                return Err(architecture_semantic_error(
                    "architecture_constraint_dangling",
                    invariant,
                    target,
                ));
            }
            if invariant.lifecycle == "active" {
                constraints.insert(ArchitectureConstraint {
                    source: invariant.semantic_id.clone(),
                    target: target.clone(),
                });
            }
        }
        let mut predecessors = BTreeSet::new();
        for predecessor in &invariant.supersedes {
            validate_architecture_id(predecessor).map_err(|_| {
                architecture_semantic_error(
                    "architecture_lineage_target_invalid",
                    invariant,
                    predecessor,
                )
            })?;
            if predecessor == &invariant.semantic_id {
                return Err(architecture_semantic_error(
                    "architecture_lineage_self",
                    invariant,
                    predecessor,
                ));
            }
            if !predecessors.insert(predecessor.as_str()) {
                return Err(architecture_semantic_error(
                    "architecture_lineage_duplicate",
                    invariant,
                    predecessor,
                ));
            }
            let Some(candidates) = identities.get(predecessor.as_str()) else {
                return Err(architecture_semantic_error(
                    "architecture_lineage_dangling",
                    invariant,
                    predecessor,
                ));
            };
            let valid = candidates
                .iter()
                .any(|index| catalog.invariants[*index].document_id != invariant.document_id);
            if !valid {
                return Err(architecture_semantic_error(
                    "architecture_lineage_same_revision",
                    invariant,
                    predecessor,
                ));
            }
            lineage.push((invariant.semantic_id.clone(), predecessor.clone()));
        }
    }
    let relationships = lineage
        .iter()
        .map(|(source, target)| SemanticRelationship {
            source: source.clone(),
            kind: "supersedes".to_owned(),
            target: target.clone(),
            source_path: String::new(),
            line: 0,
        })
        .collect::<Vec<_>>();
    if let Some(cycle) = smallest_cycle(&relationships) {
        return Err(AppError::new(
            "architecture_lineage_cycle",
            "invariant supersession contains a cycle",
        )
        .details(json!({"cycle": cycle})));
    }
    for constraint in &constraints {
        let target = indexes[&constraint.target];
        objects[target]
            .constrained_by
            .push(constraint.source.clone());
    }
    for object in objects {
        object.constrained_by.sort();
    }
    Ok(constraints.into_iter().collect())
}

fn active_constraint_inputs(
    catalog: &ArchitectureCatalog,
) -> BTreeMap<String, Vec<(String, String)>> {
    let mut inputs = BTreeMap::<String, Vec<(String, String)>>::new();
    for invariant in catalog
        .invariants
        .iter()
        .filter(|invariant| invariant.lifecycle == "active")
    {
        for target in &invariant.constrains {
            inputs.entry(target.clone()).or_default().push((
                invariant.semantic_id.clone(),
                invariant.local_semantic_fingerprint.clone(),
            ));
        }
    }
    for values in inputs.values_mut() {
        values.sort();
    }
    inputs
}

fn architecture_semantic_error(
    code: &str,
    invariant: &ArchitectureInvariant,
    target: &str,
) -> AppError {
    AppError::new(code, target).details(json!({
        "invariant_id": invariant.semantic_id,
        "target_id": target,
        "document_id": invariant.document_id,
        "source": invariant.source_locator(),
    }))
}

fn extract_relationships(
    text: &str,
    heading: &super::Heading,
    end: usize,
    source: &str,
    source_path: &str,
) -> Result<Vec<SemanticRelationship>, AppError> {
    let section = &text[heading.start..end];
    let lines = section.lines().collect::<Vec<_>>();
    let identity = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| !line.trim().is_empty())
        .map(|(index, _)| index)
        .ok_or_else(|| AppError::new("missing_requirement_id", source))?;
    let first_line = text[..heading.start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let mut relationships = Vec::new();
    let mut cursor = identity + 1;
    while cursor < lines.len() {
        let Some((kind, target)) = parse_relationship_marker(lines[cursor]) else {
            break;
        };
        let relationship = SemanticRelationship {
            source: source.to_owned(),
            kind: kind.to_owned(),
            target: target.to_owned(),
            source_path: source_path.to_owned(),
            line: first_line + cursor,
        };
        if validate_semantic_id(target).is_err() {
            return Err(relationship_error(
                "invalid_relationship_target",
                "relationship target is not a valid requirement ID",
                &relationship,
                None,
            ));
        }
        relationships.push(relationship);
        cursor += 1;
    }
    for (offset, line) in lines.iter().enumerate().skip(cursor) {
        if line.contains("<!-- dwv:requires") || line.contains("<!-- dwv:refines") {
            let relationship = parse_relationship_marker(line);
            let (kind, target) = relationship.unwrap_or(("unknown", "unknown"));
            let (code, message) = if offset == cursor && relationship.is_none() {
                (
                    "invalid_relationship_marker",
                    "relationship marker syntax is invalid",
                )
            } else {
                (
                    "misplaced_relationship_marker",
                    "relationship markers must be contiguous immediately after the intrinsic identity",
                )
            };
            let relationship = SemanticRelationship {
                source: source.to_owned(),
                kind: kind.to_owned(),
                target: target.to_owned(),
                source_path: source_path.to_owned(),
                line: first_line + offset,
            };
            return Err(relationship_error(code, message, &relationship, None));
        }
    }
    Ok(relationships)
}

fn parse_relationship_marker(line: &str) -> Option<(&str, &str)> {
    for kind in ["requires", "refines"] {
        if let Some(target) = line
            .strip_prefix(&format!("<!-- dwv:{kind} "))
            .and_then(|value| value.strip_suffix(" -->"))
        {
            return Some((kind, target));
        }
    }
    None
}

fn validate_relationships(
    objects: &[RequirementObject],
    relationships: &[SemanticRelationship],
) -> Result<(), AppError> {
    let ids = objects
        .iter()
        .map(|object| object.semantic_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut pairs = BTreeMap::<(&str, &str), &SemanticRelationship>::new();
    for relationship in relationships {
        if relationship.source == relationship.target {
            return Err(relationship_error(
                "self_relationship",
                "a requirement cannot relate to itself",
                relationship,
                None,
            ));
        }
        if !ids.contains(relationship.target.as_str()) {
            return Err(relationship_error(
                "unknown_relationship_target",
                "relationship target is not a current canonical requirement",
                relationship,
                None,
            ));
        }
        if let Some(previous) =
            pairs.insert((&relationship.source, &relationship.target), relationship)
        {
            let (code, message) = if previous.kind == relationship.kind {
                ("duplicate_relationship", "duplicate relationship edge")
            } else {
                (
                    "mixed_relationship_kind",
                    "a source-target pair cannot use both relationship kinds",
                )
            };
            return Err(relationship_error(code, message, relationship, None));
        }
    }
    if let Some(cycle) = smallest_cycle(relationships) {
        let relationship = relationships
            .iter()
            .find(|relationship| relationship.source == cycle[0] && relationship.target == cycle[1])
            .expect("cycle edges come from relationships");
        return Err(relationship_error(
            "relationship_cycle",
            "combined requires/refines graph contains a cycle",
            relationship,
            Some(&cycle),
        ));
    }
    Ok(())
}

fn relationship_error(
    code: &str,
    message: &str,
    relationship: &SemanticRelationship,
    cycle: Option<&[String]>,
) -> AppError {
    AppError::new(code, message).details(json!({
        "source_path": relationship.source_path,
        "line": relationship.line,
        "source_id": relationship.source,
        "relation_kind": relationship.kind,
        "target_id": relationship.target,
        "cycle": cycle.unwrap_or_default(),
    }))
}

fn smallest_cycle(relationships: &[SemanticRelationship]) -> Option<Vec<String>> {
    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    let mut nodes = BTreeSet::new();
    for relationship in relationships {
        adjacency
            .entry(relationship.source.clone())
            .or_default()
            .push(relationship.target.clone());
        nodes.insert(relationship.source.clone());
        nodes.insert(relationship.target.clone());
    }
    for targets in adjacency.values_mut() {
        targets.sort();
        targets.dedup();
    }
    let mut best: Option<Vec<String>> = None;
    for start in nodes {
        let mut queue = VecDeque::from([start.clone()]);
        let mut parents = BTreeMap::<String, String>::new();
        let mut visited = BTreeSet::from([start.clone()]);
        while let Some(node) = queue.pop_front() {
            for target in adjacency.get(&node).into_iter().flatten() {
                if target == &start {
                    let mut cursor = node.clone();
                    let mut cycle = vec![cursor.clone()];
                    while cursor != start {
                        cursor = parents[&cursor].clone();
                        cycle.push(cursor.clone());
                    }
                    cycle.reverse();
                    cycle.push(start.clone());
                    let cycle = normalize_cycle(cycle);
                    if best.as_ref().is_none_or(|candidate| {
                        (cycle.len(), &cycle) < (candidate.len(), candidate)
                    }) {
                        best = Some(cycle);
                    }
                } else if visited.insert(target.clone()) {
                    parents.insert(target.clone(), node.clone());
                    queue.push_back(target.clone());
                }
            }
        }
    }
    best
}

fn normalize_cycle(cycle: Vec<String>) -> Vec<String> {
    let nodes = &cycle[..cycle.len() - 1];
    let mut rotations = (0..nodes.len())
        .map(|offset| {
            let mut rotation = nodes[offset..].to_vec();
            rotation.extend_from_slice(&nodes[..offset]);
            rotation.push(rotation[0].clone());
            rotation
        })
        .collect::<Vec<_>>();
    rotations.sort();
    rotations.remove(0)
}

fn effective_fingerprint(
    id: &str,
    locals: &BTreeMap<String, String>,
    outgoing: &BTreeMap<String, Vec<SemanticRelationship>>,
    active_inputs: &BTreeMap<String, Vec<(String, String)>>,
    memo: &mut BTreeMap<String, String>,
) -> String {
    if let Some(fingerprint) = memo.get(id) {
        return fingerprint.clone();
    }
    let imported = outgoing
        .get(id)
        .into_iter()
        .flatten()
        .map(|relationship| {
            (
                relationship.kind.clone(),
                relationship.target.clone(),
                effective_fingerprint(&relationship.target, locals, outgoing, active_inputs, memo),
            )
        })
        .collect::<Vec<_>>();
    let architecture = active_inputs.get(id).cloned().unwrap_or_default();
    let encoded = if architecture.is_empty() {
        serde_json::to_string(&(locals[id].as_str(), imported))
    } else {
        serde_json::to_string(&(locals[id].as_str(), imported, architecture))
    }
    .expect("fingerprint input serializes");
    let fingerprint = digest_text(&encoded);
    memo.insert(id.to_owned(), fingerprint.clone());
    fingerprint
}

pub(super) fn export(app: &App) -> Result<Value, AppError> {
    let model = knowledge_model(app)?;
    let capabilities = capability_aggregation(&model.objects);
    let reading_order = topological_order(
        &model,
        &model
            .objects
            .iter()
            .map(|object| object.semantic_id.clone())
            .collect(),
    );
    let architecture_invariants = model
        .architecture
        .invariants
        .iter()
        .filter(|invariant| invariant.lifecycle == "active")
        .collect::<Vec<_>>();
    let value = json!({
        "schema": OBJECTS_SCHEMA,
        "objects": model.objects,
        "architecture": {
            "active_document_id": model.architecture.active_document_id,
            "invariants": architecture_invariants,
            "constraints": model.constraints,
        },
        "capabilities": capabilities,
        "reading_order": reading_order,
    });
    write_json(&app.root, OBJECTS_PATH, &value)?;
    Ok(json!({
        "schema": OBJECTS_SCHEMA,
        "objects": value["objects"].as_array().map_or(0, Vec::len),
        "relationships": model.relationships.len(),
        "capabilities": value["capabilities"].as_object().map_or(0, serde_json::Map::len),
        "path": OBJECTS_PATH,
    }))
}

fn capability_aggregation(objects: &[RequirementObject]) -> Value {
    let mut requirements = BTreeMap::<String, BTreeSet<String>>::new();
    let mut requires = BTreeMap::<String, BTreeSet<String>>::new();
    let mut refines = BTreeMap::<String, BTreeSet<String>>::new();
    let mut required_by = BTreeMap::<String, BTreeSet<String>>::new();
    let mut refined_by = BTreeMap::<String, BTreeSet<String>>::new();
    for object in objects {
        requirements
            .entry(object.capability.clone())
            .or_default()
            .insert(object.semantic_id.clone());
        for (targets, forward, reverse) in [
            (&object.requires, &mut requires, &mut required_by),
            (&object.refines, &mut refines, &mut refined_by),
        ] {
            for target in targets {
                let target_capability = target.split('.').nth(1).unwrap_or_default().to_owned();
                forward
                    .entry(object.capability.clone())
                    .or_default()
                    .insert(target_capability.clone());
                reverse
                    .entry(target_capability)
                    .or_default()
                    .insert(object.capability.clone());
            }
        }
    }
    let mut result = serde_json::Map::new();
    for capability in requirements.keys() {
        result.insert(
            capability.clone(),
            json!({
                "requirements": requirements.get(capability).into_iter().flatten().collect::<Vec<_>>(),
                "requires": requires.get(capability).into_iter().flatten().collect::<Vec<_>>(),
                "refines": refines.get(capability).into_iter().flatten().collect::<Vec<_>>(),
                "required_by": required_by.get(capability).into_iter().flatten().collect::<Vec<_>>(),
                "refined_by": refined_by.get(capability).into_iter().flatten().collect::<Vec<_>>(),
            }),
        );
    }
    Value::Object(result)
}

/// dwv:req req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible
pub(super) fn inspect(app: &App, id: String) -> Result<Value, AppError> {
    let model = knowledge_model(app)?;
    let requirement = requirement_by_id(&model, &id)?;
    let mut unit = requirement_identity(requirement);
    unit["relationships"] = direct_relationships(requirement);
    unit["body"] = json!(&requirement.body);
    Ok(json!({
        "schema": INSPECT_SCHEMA,
        "requirement": unit,
    }))
}

pub(super) fn context(app: &App, ids: Vec<String>) -> Result<Value, AppError> {
    context_packet(app, ids, false)
}

pub(super) fn audit_context(app: &App, id: String) -> Result<Value, AppError> {
    context_packet(app, vec![id], true)
}

pub(super) fn ownership(app: &App, id: String) -> Result<Value, AppError> {
    let model = knowledge_model(app)?;
    let requirement = requirement_by_id(&model, &id)?;
    let reviewed = load_reviewed(app)?;
    let references = scan_references(app)?;
    let value = ownership_result(app, requirement, &reviewed, &references);
    enforce_selected_packet_bound(
        app,
        "ownership_bound_exceeded",
        "ownership",
        &BTreeSet::from([id]),
        &value,
        Vec::new(),
    )?;
    Ok(value)
}

fn requirement_by_id<'a>(
    model: &'a KnowledgeModel,
    id: &str,
) -> Result<&'a RequirementObject, AppError> {
    model
        .objects
        .iter()
        .find(|object| object.semantic_id == id)
        .ok_or_else(|| AppError::new("unknown_requirement", id))
}

pub(super) fn architecture_candidate(app: &App, id: String) -> Result<Value, AppError> {
    let model = knowledge_model(app)?;
    let candidate = architecture_document(&model.architecture, &id, "candidate")?;
    let active = architecture_document(
        &model.architecture,
        &model.architecture.active_document_id,
        "active",
    )?;
    let candidate_invariants = document_invariants(&model.architecture, &candidate.id);
    let active_invariants = document_invariants(&model.architecture, &active.id);
    let candidate_by_id = candidate_invariants
        .iter()
        .map(|invariant| (invariant.semantic_id.as_str(), *invariant))
        .collect::<BTreeMap<_, _>>();
    let active_by_id = active_invariants
        .iter()
        .map(|invariant| (invariant.semantic_id.as_str(), *invariant))
        .collect::<BTreeMap<_, _>>();
    let identities = candidate_by_id
        .keys()
        .chain(active_by_id.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let mut direct = BTreeSet::new();
    let comparisons = identities
        .into_iter()
        .map(|identity| {
            let candidate = candidate_by_id.get(identity).copied();
            let active = active_by_id.get(identity).copied();
            let candidate_targets = candidate
                .map(|invariant| {
                    invariant.constrains.iter().cloned().collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            let active_targets = active
                .map(|invariant| {
                    invariant.constrains.iter().cloned().collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            let added_constraints = candidate_targets
                .difference(&active_targets)
                .cloned()
                .collect::<Vec<_>>();
            let removed_constraints = active_targets
                .difference(&candidate_targets)
                .cloned()
                .collect::<Vec<_>>();
            direct.extend(added_constraints.iter().cloned());
            direct.extend(removed_constraints.iter().cloned());
            let classification = match (active, candidate) {
                (None, Some(_)) => "added",
                (Some(_), None) => "removed",
                (Some(active), Some(candidate))
                    if active.local_semantic_fingerprint
                        == candidate.local_semantic_fingerprint
                        && added_constraints.is_empty()
                        && removed_constraints.is_empty() =>
                {
                    "unchanged"
                }
                (Some(_), Some(_)) => "changed",
                (None, None) => unreachable!("identity comes from one document"),
            };
            if classification == "changed" {
                direct.extend(active_targets.iter().cloned());
                direct.extend(candidate_targets.iter().cloned());
            }
            let supersedes = candidate
                .map(|invariant| invariant.supersedes.clone())
                .unwrap_or_default();
            let superseded_by = candidate
                .map(|invariant| invariant.superseded_by.clone())
                .unwrap_or_default();
            json!({
                "semantic_id": identity,
                "classification": classification,
                "active_fingerprint": active.map(|invariant| &invariant.local_semantic_fingerprint),
                "candidate_fingerprint": candidate.map(|invariant| &invariant.local_semantic_fingerprint),
                "added_constraints": added_constraints,
                "removed_constraints": removed_constraints,
                "supersedes": supersedes,
                "superseded_by": superseded_by,
            })
        })
        .collect::<Vec<_>>();
    let dependent_closure = current_dependent_closure(&model, &direct);
    let value = json!({
        "schema": ARCHITECTURE_CANDIDATE_SCHEMA,
        "authority": "non-authoritative-candidate",
        "document": candidate,
        "active_document_id": active.id,
        "invariants": candidate_invariants,
        "comparisons": comparisons,
        "directly_affected_requirements": direct,
        "dependent_closure": dependent_closure,
        "bounds": {"max_context_bytes": app.bounds.max_context_bytes},
    });
    enforce_context_bound(app, "architecture_candidate_bound_exceeded", &value)?;
    Ok(value)
}

pub(super) fn architecture_history(app: &App, id: String) -> Result<Value, AppError> {
    let model = knowledge_model(app)?;
    let document = architecture_document(&model.architecture, &id, "superseded")?;
    let invariants = document_invariants(&model.architecture, &document.id);
    let constraints = invariants
        .iter()
        .flat_map(|invariant| {
            invariant
                .constrains
                .iter()
                .map(|target| ArchitectureConstraint {
                    source: invariant.semantic_id.clone(),
                    target: target.clone(),
                })
        })
        .collect::<Vec<_>>();
    let value = json!({
        "schema": ARCHITECTURE_HISTORY_SCHEMA,
        "authority": "historical",
        "document": document,
        "invariants": invariants,
        "constraints": constraints,
        "bounds": {"max_context_bytes": app.bounds.max_context_bytes},
    });
    enforce_context_bound(app, "architecture_history_bound_exceeded", &value)?;
    Ok(value)
}

fn architecture_document<'a>(
    catalog: &'a ArchitectureCatalog,
    id: &str,
    expected_status: &str,
) -> Result<&'a ArchitectureDocument, AppError> {
    let document = catalog
        .documents
        .iter()
        .find(|document| document.id == id)
        .ok_or_else(|| AppError::new("unknown_architecture_document", id))?;
    if document.status != expected_status {
        return Err(AppError::new(
            "architecture_document_lifecycle_mismatch",
            format!("{id} is {}, expected {expected_status}", document.status),
        ));
    }
    Ok(document)
}

fn document_invariants<'a>(
    catalog: &'a ArchitectureCatalog,
    document_id: &str,
) -> Vec<&'a ArchitectureInvariant> {
    catalog
        .invariants
        .iter()
        .filter(|invariant| invariant.document_id == document_id)
        .collect()
}

fn current_dependent_closure(
    model: &KnowledgeModel,
    direct: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut closure = direct.clone();
    let mut queue = VecDeque::from_iter(direct.iter().cloned());
    while let Some(id) = queue.pop_front() {
        if let Ok(requirement) = requirement_by_id(model, &id) {
            for dependent in requirement
                .required_by
                .iter()
                .chain(&requirement.refined_by)
            {
                if closure.insert(dependent.clone()) {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }
    closure
}

fn requirement_identity(requirement: &RequirementObject) -> Value {
    json!({
        "semantic_id": requirement.semantic_id,
        "title": requirement.title,
        "capability": requirement.capability,
        "source": requirement.source_locator(),
        "local_semantic_fingerprint": requirement.local_semantic_fingerprint,
        "effective_semantic_fingerprint": requirement.effective_semantic_fingerprint,
    })
}

fn direct_relationships(requirement: &RequirementObject) -> Value {
    json!({
        "requires": requirement.requires,
        "refines": requirement.refines,
        "required_by": requirement.required_by,
        "constrained_by": requirement.constrained_by,
        "refined_by": requirement.refined_by,
    })
}

fn load_reviewed(app: &App) -> Result<ReviewedState, AppError> {
    let state: ReviewedState = read_toml(&app.root, REVIEWED_PATH)?;
    if state.schema != REVIEWED_SCHEMA {
        return Err(AppError::new(
            "reviewed_state_schema_mismatch",
            state.schema,
        ));
    }
    Ok(state)
}

fn reviewed_fact(state: &ReviewedState, id: &str) -> Value {
    json!({
        "local_fingerprint": state.local_fingerprints.get(id),
        "effective_fingerprint": state.effective_fingerprints.get(id),
        "outcome": state.outcomes.get(id),
        "reason": state.reasons.get(id),
    })
}

fn packet_references(
    references: &[ScanRef],
    selected: &BTreeSet<String>,
    max_items: usize,
) -> (Value, Value) {
    let mut implementation = Vec::new();
    let mut evidence = Vec::new();
    let mut curriculum = Vec::new();
    let mut documentation = Vec::new();
    for reference in references
        .iter()
        .filter(|reference| selected.contains(&reference.id))
    {
        let (relation, destination) = if reference.kind == "rust" {
            ("implements", &mut implementation)
        } else if reference.kind == "delegated" {
            ("delegates", &mut implementation)
        } else if reference.kind == "verification" {
            ("verifies", &mut evidence)
        } else if reference.path == "docs/curriculum.toml" {
            ("curriculum", &mut curriculum)
        } else {
            ("explained_by", &mut documentation)
        };
        destination.push(Reference {
            kind: reference.kind.clone(),
            id: reference.id.clone(),
            path: reference.path.clone(),
            relation: relation.to_owned(),
            line: Some(reference.line),
        });
    }
    let implementation_omitted = bound_reference_category(&mut implementation, max_items);
    let evidence_omitted = bound_reference_category(&mut evidence, max_items);
    let curriculum_omitted = bound_reference_category(&mut curriculum, max_items);
    let documentation_omitted = bound_reference_category(&mut documentation, max_items);
    let omitted = selected
        .iter()
        .map(|id| {
            (
                id.clone(),
                json!({
                    "implementation": implementation_omitted.get(id).copied().unwrap_or(0),
                    "evidence": evidence_omitted.get(id).copied().unwrap_or(0),
                    "curriculum": curriculum_omitted.get(id).copied().unwrap_or(0),
                    "documentation": documentation_omitted.get(id).copied().unwrap_or(0),
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    (
        json!({
            "implementation": implementation,
            "evidence": evidence,
            "curriculum": curriculum,
            "documentation": documentation,
        }),
        Value::Object(omitted),
    )
}

fn bound_reference_category(
    references: &mut Vec<Reference>,
    max_items: usize,
) -> BTreeMap<String, usize> {
    references.sort_by(|left, right| {
        (&left.kind, &left.id, &left.path, &left.relation, left.line).cmp(&(
            &right.kind,
            &right.id,
            &right.path,
            &right.relation,
            right.line,
        ))
    });
    references.dedup();
    let mut omitted = BTreeMap::<String, usize>::new();
    for reference in references.iter().skip(max_items) {
        *omitted.entry(reference.id.clone()).or_default() += 1;
    }
    references.truncate(max_items);
    omitted
}

fn ownership_result(
    app: &App,
    requirement: &RequirementObject,
    reviewed: &ReviewedState,
    all_references: &[ScanRef],
) -> Value {
    let selected = BTreeSet::from([requirement.semantic_id.clone()]);
    let (references, omitted) = packet_references(all_references, &selected, app.bounds.max_units);
    json!({
        "schema": OWNERSHIP_SCHEMA,
        "requirement": requirement_identity(requirement),
        "relationships": direct_relationships(requirement),
        "reviewed": reviewed_fact(reviewed, &requirement.semantic_id),
        "references": references,
        "bounds": {
            "max_items_per_category": app.bounds.max_units,
            "max_context_bytes": app.bounds.max_context_bytes,
        },
        "omitted": omitted,
    })
}

fn context_packet(app: &App, ids: Vec<String>, audit: bool) -> Result<Value, AppError> {
    if ids.is_empty() {
        return Err(AppError::new(
            "usage",
            if audit {
                "audit-context requires at least one ID"
            } else {
                "context requires at least one ID"
            },
        ));
    }
    let selected = ids.into_iter().collect::<BTreeSet<_>>();
    if selected.len() > app.bounds.max_units {
        return Err(AppError::new(
            "context_unit_bound_exceeded",
            selected.len().to_string(),
        ));
    }
    let model = knowledge_model(app)?;
    for id in &selected {
        requirement_by_id(&model, id)?;
    }
    let reviewed = load_reviewed(app)?;
    let all_references = scan_references(app)?;
    let value = context_packet_value(app, &model, &reviewed, &all_references, &selected, audit);
    let suggested_requests = if !audit && !packet_fits(app, &value) {
        context_split_requests(app, &model, &reviewed, &all_references, &selected)
    } else {
        Vec::new()
    };
    enforce_selected_packet_bound(
        app,
        if audit {
            "audit_context_bound_exceeded"
        } else {
            "context_bound_exceeded"
        },
        if audit { "audit-context" } else { "context" },
        &selected,
        &value,
        suggested_requests,
    )?;
    Ok(value)
}

fn context_packet_value(
    app: &App,
    model: &KnowledgeModel,
    reviewed: &ReviewedState,
    all_references: &[ScanRef],
    selected: &BTreeSet<String>,
    audit: bool,
) -> Value {
    let component = if audit {
        connected_component(model, selected)
    } else {
        ordinary_context_component(model, selected)
    };
    let reading_order = topological_order(model, &component);
    let requirements = reading_order
        .iter()
        .map(|id| {
            let requirement =
                requirement_by_id(model, id).expect("context component IDs are current");
            let mut unit = requirement_identity(requirement);
            unit["body"] = json!(&requirement.body);
            unit
        })
        .collect::<Vec<_>>();
    let relationships = model
        .relationships
        .iter()
        .filter(|relationship| {
            component.contains(&relationship.source) && component.contains(&relationship.target)
        })
        .map(|relationship| {
            json!({
                "source": relationship.source,
                "kind": relationship.kind,
                "target": relationship.target,
            })
        })
        .collect::<Vec<_>>();
    let architecture_invariants = model
        .architecture
        .invariants
        .iter()
        .filter(|invariant| {
            invariant.lifecycle == "active"
                && invariant
                    .constrains
                    .iter()
                    .any(|target| component.contains(target))
        })
        .collect::<Vec<_>>();
    let architecture_ids = architecture_invariants
        .iter()
        .map(|invariant| invariant.semantic_id.as_str())
        .collect::<BTreeSet<_>>();
    let architecture_constraints = model
        .constraints
        .iter()
        .filter(|constraint| {
            architecture_ids.contains(constraint.source.as_str())
                && component.contains(&constraint.target)
        })
        .collect::<Vec<_>>();
    let review_facts = selected
        .iter()
        .map(|id| (id.clone(), reviewed_fact(reviewed, id)))
        .collect::<BTreeMap<_, _>>();
    let (references, omitted) = packet_references(all_references, selected, app.bounds.max_units);
    json!({
        "schema": if audit { AUDIT_CONTEXT_SCHEMA } else { CONTEXT_SCHEMA },
        "requirements": requirements,
        "relationships": relationships,
        "architecture_invariants": architecture_invariants,
        "architecture_constraints": architecture_constraints,
        "reviewed": review_facts,
        "references": references,
        "reading_order": reading_order,
        "bounds": {
            "max_items_per_category": app.bounds.max_units,
            "max_context_bytes": app.bounds.max_context_bytes,
        },
        "omitted": omitted,
    })
}

fn context_split_requests(
    app: &App,
    model: &KnowledgeModel,
    reviewed: &ReviewedState,
    all_references: &[ScanRef],
    selected: &BTreeSet<String>,
) -> Vec<Vec<String>> {
    if selected.len() < 2 {
        return Vec::new();
    }
    let requests = selected
        .iter()
        .map(|id| vec![id.clone()])
        .collect::<Vec<_>>();
    if requests.iter().all(|request| {
        let candidate = request.iter().cloned().collect::<BTreeSet<_>>();
        packet_fits(
            app,
            &context_packet_value(app, model, reviewed, all_references, &candidate, false),
        )
    }) {
        requests
    } else {
        Vec::new()
    }
}

fn ordinary_context_component(
    model: &KnowledgeModel,
    selected: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut component = selected.clone();
    let mut queue = VecDeque::from_iter(selected.iter().cloned());
    while let Some(id) = queue.pop_front() {
        for relationship in model
            .relationships
            .iter()
            .filter(|relationship| relationship.source == id)
        {
            if component.insert(relationship.target.clone()) {
                queue.push_back(relationship.target.clone());
            }
        }
    }
    for relationship in &model.relationships {
        if selected.contains(&relationship.target) {
            component.insert(relationship.source.clone());
        }
    }
    component
}

fn connected_component(model: &KnowledgeModel, selected: &BTreeSet<String>) -> BTreeSet<String> {
    let mut neighbors = BTreeMap::<String, BTreeSet<String>>::new();
    for relationship in &model.relationships {
        neighbors
            .entry(relationship.source.clone())
            .or_default()
            .insert(relationship.target.clone());
        neighbors
            .entry(relationship.target.clone())
            .or_default()
            .insert(relationship.source.clone());
    }
    let mut component = selected.clone();
    let mut queue = VecDeque::from_iter(selected.iter().cloned());
    while let Some(id) = queue.pop_front() {
        for neighbor in neighbors.get(&id).into_iter().flatten() {
            if component.insert(neighbor.clone()) {
                queue.push_back(neighbor.clone());
            }
        }
    }
    component
}

fn packet_fits(app: &App, value: &Value) -> bool {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX)
        <= app.bounds.max_context_bytes
}

fn enforce_selected_packet_bound(
    app: &App,
    code: &str,
    command: &str,
    selected: &BTreeSet<String>,
    value: &Value,
    suggested_requests: Vec<Vec<String>>,
) -> Result<(), AppError> {
    if packet_fits(app, value) {
        return Ok(());
    }
    Err(AppError::new(
        code,
        if suggested_requests.is_empty() {
            "the complete packet cannot be split without omission; increase max_context_bytes"
        } else {
            "retry the deterministic complete-packet split in details"
        },
    )
    .details(json!({
        "command": command,
        "selected_ids": selected,
        "suggested_requests": suggested_requests,
        "max_context_bytes": app.bounds.max_context_bytes,
    })))
}

fn topological_order(model: &KnowledgeModel, component: &BTreeSet<String>) -> Vec<String> {
    let mut dependents = BTreeMap::<String, BTreeSet<String>>::new();
    let mut prerequisites = component
        .iter()
        .map(|id| (id.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    for relationship in &model.relationships {
        if component.contains(&relationship.source) && component.contains(&relationship.target) {
            *prerequisites
                .get_mut(&relationship.source)
                .expect("component contains source") += 1;
            dependents
                .entry(relationship.target.clone())
                .or_default()
                .insert(relationship.source.clone());
        }
    }
    let mut ready = prerequisites
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(component.len());
    while let Some(id) = ready.pop_first() {
        order.push(id.clone());
        for dependent in dependents.get(&id).into_iter().flatten() {
            let count = prerequisites
                .get_mut(dependent)
                .expect("component contains dependent");
            *count -= 1;
            if *count == 0 {
                ready.insert(dependent.clone());
            }
        }
    }
    order
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
    let model = knowledge_model(app)?;
    let requirements = &model.objects;
    let current = requirements
        .iter()
        .map(|requirement| (requirement.semantic_id.as_str(), requirement))
        .collect::<BTreeMap<_, _>>();
    let references = scan_references(app)?;
    let reviewed = load_reviewed(app)?;
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
                    reviewed
                        .effective_fingerprints
                        .get(&requirement.semantic_id)
                        != Some(&requirement.effective_semantic_fingerprint)
                })
                .map(|requirement| requirement.semantic_id.clone())
                .collect::<BTreeSet<_>>();
            for id in &changed {
                let impact = selected.entry(id.clone()).or_default();
                impact.0.insert(format!(
                    "canonical effective semantics changed in {relative}"
                ));
                impact.1 = true;
            }
            path_impacts.push(json!({
                "path": relative,
                "classification": if changed.is_empty() { "canonical_effective_fingerprints_unchanged" } else { "canonical_effective_semantics_changed" },
                "review_required": !changed.is_empty(),
                "current_ids": changed,
                "baseline": "reviewed effective semantic fingerprints"
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
        let quint_source = is_quint_source_path(&relative);
        let review_required = (quint_source && !current_ids.is_empty())
            || (baseline.available && !removed.is_empty());
        let classification = if baseline.available && !removed.is_empty() {
            "current_requirement_relationship_removed_or_reassigned"
        } else if quint_source && !current_ids.is_empty() {
            "delegated_canonical_source_changed"
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

    let seeds = selected
        .iter()
        .filter(|(_, (_, review_required))| *review_required)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for seed in seeds {
        for dependent in dependent_closure(&model, &seed) {
            if dependent == seed {
                continue;
            }
            let impact = selected.entry(dependent).or_default();
            impact
                .0
                .insert(format!("semantic-prerequisite-changed through {seed}"));
            impact.1 = true;
        }
    }

    if selected.len() > app.bounds.max_units {
        return Err(AppError::new(
            "affected_unit_bound_exceeded",
            selected.len().to_string(),
        ));
    }
    let mut impacts = Vec::new();
    for (id, (reasons, review_required)) in selected {
        let packet = ownership_result(app, current[id.as_str()], &reviewed, &references);
        let pages = packet["references"]["documentation"]
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
            "ownership": packet
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

fn dependent_closure(model: &KnowledgeModel, source: &str) -> BTreeSet<String> {
    let mut closure = BTreeSet::from([source.to_owned()]);
    let mut queue = VecDeque::from([source.to_owned()]);
    while let Some(id) = queue.pop_front() {
        for object in &model.objects {
            if (object.requires.contains(&id) || object.refines.contains(&id))
                && closure.insert(object.semantic_id.clone())
            {
                queue.push_back(object.semantic_id.clone());
            }
        }
    }
    closure
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

    fn tracked_paths(self, root: &Path) -> Result<Vec<String>, AppError> {
        let output = match self {
            Self::Jujutsu => Command::new("jj")
                .args(["file", "list"])
                .current_dir(root)
                .output(),
            Self::Git => Command::new("git")
                .args(["ls-files", "--cached", "--others", "--exclude-standard"])
                .current_dir(root)
                .output(),
        }
        .map_err(|error| AppError::new("planning_scan_unavailable", error.to_string()))?;
        if !output.status.success() {
            return Err(AppError::new(
                "planning_scan_failed",
                String::from_utf8_lossy(&output.stderr).trim(),
            ));
        }
        let text = String::from_utf8(output.stdout)
            .map_err(|error| AppError::new("planning_scan_paths_not_utf8", error.to_string()))?;
        Ok(text.lines().map(str::to_owned).collect())
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

fn is_quint_source_path(path: &str) -> bool {
    (path.starts_with("models/quint/") || path.starts_with("verification/quint/"))
        && path.ends_with(".qnt")
}

fn is_impact_candidate(path: &str) -> bool {
    !excluded(path)
        && ((path.starts_with("crates/") && path.ends_with(".rs"))
            || (path.starts_with("xtask/") && path.ends_with(".rs"))
            || (path.starts_with("openspec/specs/") && path.ends_with("/spec.md"))
            || path == "verification/manifest.toml"
            || path == "docs/curriculum.toml"
            || (path.starts_with("docs/sphinx/") && path.ends_with(".md"))
            || is_quint_source_path(path))
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

fn relationship_marker(relative: &str, line: &str) -> bool {
    if relative.ends_with(".rs") {
        line.contains("/// dwv:req ")
    } else if relative.ends_with(".qnt") {
        line.contains("// dwv:req ")
    } else {
        true
    }
}

fn relationship_ids(relative: &str, text: &str) -> BTreeSet<String> {
    if relative.ends_with(".rs") || relative.ends_with(".qnt") {
        text.lines()
            .filter(|line| relationship_marker(relative, line))
            .flat_map(extract_ids)
            .collect()
    } else {
        extract_ids(text)
    }
}

fn validate_affected_path(app: &App, relative: &str) -> Result<(), AppError> {
    reject_retired_path(relative)?;
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
    retired_path_preflight(app)?;
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

    let model = knowledge_model(app)?;
    let requirements = &model.objects;
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
                    reviewed
                        .effective_fingerprints
                        .get(&requirement.semantic_id)
                        != Some(&requirement.effective_semantic_fingerprint)
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
                    "classification": "canonical_effective_semantics_changed",
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
        let unknown = current_ids
            .iter()
            .filter(|id| !requirement_ids.contains(id.as_str()))
            .cloned()
            .collect::<BTreeSet<_>>();
        if !unknown.is_empty() {
            diagnostics.push(json!({
                "path": relative,
                "kind": "unknown_canonical_requirement",
                "requirement_ids": unknown
            }));
        }
        let removed_current = removed
            .iter()
            .filter(|id| requirement_ids.contains(id.as_str()))
            .cloned()
            .collect::<BTreeSet<_>>();
        let removed_noncurrent = removed
            .difference(&removed_current)
            .cloned()
            .collect::<BTreeSet<_>>();
        let delegated_ids = all_ids
            .iter()
            .filter(|id| requirement_ids.contains(id.as_str()))
            .cloned()
            .collect::<BTreeSet<_>>();
        added_relationships += added.len();
        let quint_source = is_quint_source_path(&relative);
        if baseline.available && !removed_current.is_empty() {
            review_ids.extend(delegated_ids.iter().cloned());
            next_actions.insert(format!(
                "cargo xtask docs knowledge affected --path {relative}"
            ));
            entries.push(json!({
                "path": relative,
                "classification": "current_requirement_relationship_removed_or_reassigned",
                "review_required": true,
                "requirement_ids": delegated_ids,
                "current_ids": current_ids,
                "previous_ids": baseline.ids,
                "added_ids": added,
                "removed_current_ids": removed_current,
                "removed_noncurrent_ids": removed_noncurrent,
                "baseline": baseline.source
            }));
        } else if quint_source && !current_ids.is_empty() {
            review_ids.extend(delegated_ids.iter().cloned());
            next_actions.insert(format!(
                "cargo xtask docs knowledge affected --path {relative}"
            ));
            entries.push(json!({
                "path": relative,
                "classification": "delegated_canonical_source_changed",
                "review_required": true,
                "requirement_ids": delegated_ids,
                "current_ids": current_ids,
                "previous_ids": baseline.ids,
                "added_ids": added,
                "removed_current_ids": removed_current,
                "removed_noncurrent_ids": removed_noncurrent,
                "baseline": baseline.source
            }));
        } else if baseline.available && !removed_noncurrent.is_empty() {
            entries.push(json!({
                "path": relative,
                "classification": "noncurrent_reference_removed",
                "review_required": false,
                "removed_ids": removed_noncurrent,
                "baseline": baseline.source
            }));
        } else if baseline.available && !added.is_empty() {
            context_ids.extend(added);
        } else if baseline.available {
            unchanged_relationship_files += 1;
            context_ids.extend(current_ids);
        }
    }
    for id in review_ids.clone() {
        review_ids.extend(dependent_closure(&model, &id));
    }
    context_ids.retain(|id| !review_ids.contains(id));

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

fn is_retired_path(path: &str) -> bool {
    path == RETIRED_PATH || path.starts_with("docs/milestones/")
}

fn retired_path_diagnostic(relative: &str) -> Value {
    let fragment = if relative == RETIRED_PATH {
        format!("{RETIRED_PATH}/**")
    } else {
        relative.to_owned()
    };
    json!({
        "gate": "retired-planning-identifier",
        "kind": "retired-planning-path",
        "path": relative,
        "source": "path",
        "line": 0,
        "column": 1,
        "fragment": fragment,
        "next_action": "remove docs/milestones/**; use Beads or current product terminology",
    })
}

fn retired_path_error(relative: &str) -> AppError {
    AppError::new(
        "retired_planning_identifier",
        "docs/milestones/** is a retired path and is not a current knowledge source",
    )
    .details(json!({
        "schema": PLANNING_SCAN_SCHEMA,
        "forbidden_project_planning_uses": 1,
        "ordinary_goal_language_allowed": true,
        "diagnostics": [retired_path_diagnostic(relative)],
    }))
}

fn reject_retired_path(relative: &str) -> Result<(), AppError> {
    if is_retired_path(relative) {
        Err(retired_path_error(relative))
    } else {
        Ok(())
    }
}

pub(super) fn retired_path_preflight(app: &App) -> Result<(), AppError> {
    match fs::symlink_metadata(app.root.join(RETIRED_PATH)) {
        Ok(_) => Err(retired_path_error(RETIRED_PATH)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::new(
            "retired_path_scan_failed",
            format!("{RETIRED_PATH}: {error}"),
        )),
    }
}

fn collect_markdown_files(
    root: &Path,
    path: &Path,
    files: &mut Vec<std::path::PathBuf>,
) -> Result<(), AppError> {
    reject_retired_path(&rel(root, path))?;
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

/// dwv:req req.documentation-knowledge-architecture.active-roadmap-is-explicit-and-non-authoritative
fn validate_roadmap_identifiers(app: &App) -> Result<(), AppError> {
    retired_path_preflight(app)?;
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

const PLANNING_SCAN_SCHEMA: &str = "dwv.docs.planning-nomenclature.v1";

fn planning_scan_paths(app: &App) -> Result<Vec<String>, AppError> {
    let mut paths = if let Some(provider) = RevisionControl::detect(&app.root) {
        provider.tracked_paths(&app.root)?
    } else {
        let mut paths = Vec::new();
        collect_planning_scan_paths(&app.root, &app.root, &mut paths)?;
        paths
    };
    paths.sort();
    paths.dedup();
    for relative in &paths {
        reject_retired_path(relative)?;
    }
    if paths.len() > app.bounds.max_planning_scan_paths {
        return Err(AppError::new(
            "planning_scan_path_bound_exceeded",
            format!(
                "{} (max {})",
                paths.len(),
                app.bounds.max_planning_scan_paths
            ),
        ));
    }
    Ok(paths)
}

fn collect_planning_scan_paths(
    root: &Path,
    path: &Path,
    paths: &mut Vec<String>,
) -> Result<(), AppError> {
    reject_retired_path(&rel(root, path))?;
    let mut children = entries(path, "planning_scan_failed")?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let kind = child
            .file_type()
            .map_err(|error| AppError::new("planning_scan_failed", error.to_string()))?;
        let name = child.file_name();
        let name = name.to_string_lossy();
        if kind.is_symlink() {
            return Err(AppError::new(
                "planning_scan_symlink",
                rel(root, &child.path()),
            ));
        }
        if kind.is_dir() {
            if name.starts_with('.') || matches!(name.as_ref(), "target" | "node_modules") {
                continue;
            }
            collect_planning_scan_paths(root, &child.path(), paths)?;
        } else if kind.is_file() {
            paths.push(rel(root, &child.path()));
        }
    }
    Ok(())
}

fn find_ascii_case_insensitive(text: &[u8], needle: &[u8]) -> Option<usize> {
    text.windows(needle.len())
        .position(|candidate| candidate.eq_ignore_ascii_case(needle))
}

fn retired_planning_match(text: &str) -> Option<(&'static str, usize, usize)> {
    let bytes = text.as_bytes();
    for (needle, kind) in [
        (
            concat!("close-", "go", "al-v7-acceptance-gaps").as_bytes(),
            "retired-acceptance-archive",
        ),
        (
            concat!("dwv-", "go", "al-v7").as_bytes(),
            "retired-linux-runtime-name",
        ),
        (
            concat!("docs/handoffs/", "go", "al-v").as_bytes(),
            "retired-planning-path",
        ),
        (
            concat!("docs/handoffs/", "go", "al.md").as_bytes(),
            "retired-planning-path",
        ),
    ] {
        if let Some(start) = find_ascii_case_insensitive(bytes, needle) {
            return Some((kind, start, start + needle.len()));
        }
    }

    for start in 0..bytes.len().saturating_sub(3) {
        if !bytes[start..start + 4].eq_ignore_ascii_case(b"goal")
            || start > 0 && bytes[start - 1].is_ascii_alphanumeric()
        {
            continue;
        }
        let mut cursor = start + 4;
        let separator_start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| matches!(byte, b'-' | b'_' | b' '))
        {
            cursor += 1;
        }
        if cursor == separator_start {
            continue;
        }
        if bytes
            .get(cursor)
            .is_some_and(|byte| byte.eq_ignore_ascii_case(&b'v'))
        {
            cursor += 1;
            while bytes
                .get(cursor)
                .is_some_and(|byte| matches!(byte, b'-' | b'_' | b' '))
            {
                cursor += 1;
            }
        }
        if !bytes
            .get(cursor)
            .is_some_and(|byte| matches!(byte, b'1'..=b'8'))
        {
            continue;
        }
        let end = cursor + 1;
        if bytes
            .get(end)
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        {
            continue;
        }
        return Some(("retired-numbered-planning-identifier", start, end));
    }
    None
}

fn retired_planning_diagnostics(app: &App) -> Result<Vec<Value>, AppError> {
    retired_path_preflight(app)?;
    let mut diagnostics = Vec::new();
    for relative in planning_scan_paths(app)? {
        if let Some((kind, start, end)) = retired_planning_match(&relative) {
            diagnostics.push(json!({
                "gate": "retired-planning-identifier",
                "kind": kind,
                "path": relative,
                "source": "path",
                "line": 0,
                "column": start + 1,
                "fragment": &relative[start..end],
                "next_action": "remove docs/milestones/**; use Beads or current product terminology",
            }));
        }
        let path = safe_join(&app.root, &relative)?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| AppError::new("planning_scan_failed", error.to_string()))?;
        if metadata.file_type().is_symlink() {
            return Err(AppError::new("planning_scan_symlink", relative));
        }
        if !metadata.is_file() {
            continue;
        }
        if metadata.len() > MAX_AUTHORITY_FILE_BYTES as u64 {
            return Err(AppError::new("planning_scan_file_bound_exceeded", relative));
        }
        let bytes = fs::read(&path)
            .map_err(|error| AppError::new("planning_scan_failed", error.to_string()))?;
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        for (line, content) in text.lines().enumerate() {
            if let Some((kind, start, end)) = retired_planning_match(content) {
                diagnostics.push(json!({
                    "gate": "retired-planning-identifier",
                    "kind": kind,
                    "path": relative,
                    "source": "content",
                    "line": line + 1,
                    "column": start + 1,
                    "fragment": &content[start..end],
                    "next_action": "remove docs/milestones/**; use Beads or current product terminology",
                }));
            }
        }
    }
    if diagnostics.len() > app.bounds.max_units {
        return Err(AppError::new(
            "planning_scan_diagnostic_bound_exceeded",
            diagnostics.len().to_string(),
        ));
    }
    Ok(diagnostics)
}

pub(super) fn planning_nomenclature(app: &App) -> Result<Value, AppError> {
    retired_path_preflight(app)?;
    let diagnostics = retired_planning_diagnostics(app)?;
    let result = json!({
        "schema": PLANNING_SCAN_SCHEMA,
        "forbidden_project_planning_uses": diagnostics.len(),
        "ordinary_goal_language_allowed": true,
        "diagnostics": diagnostics,
    });
    if result["forbidden_project_planning_uses"] != 0 {
        return Err(AppError::new(
            "retired_planning_identifier",
            "retired project-planning identifiers remain",
        )
        .details(result));
    }
    Ok(result)
}

/// dwv:req req.documentation-knowledge-architecture.human-curriculum-is-pedagogical-intent-not-semantic-authority
pub(super) fn readiness(app: &App) -> Result<Value, AppError> {
    retired_path_preflight(app)?;
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
    let planning_diagnostics = retired_planning_diagnostics(app)?;
    let current_ids = current.keys().copied().collect::<BTreeSet<_>>();
    let mut uncovered = current_ids
        .iter()
        .filter(|id| {
            !state.local_fingerprints.contains_key(**id)
                || !state.effective_fingerprints.contains_key(**id)
        })
        .map(|id| (*id).to_owned())
        .collect::<Vec<_>>();
    uncovered.sort();
    uncovered.dedup();
    let local_suspect = current
        .iter()
        .filter(|(id, record)| {
            state
                .local_fingerprints
                .get(**id)
                .is_some_and(|fingerprint| fingerprint != &record.local_semantic_fingerprint)
        })
        .map(|(id, _)| (*id).to_owned())
        .collect::<Vec<_>>();
    let prerequisite_suspect = current
        .iter()
        .filter(|(id, record)| {
            state
                .local_fingerprints
                .get(**id)
                .is_some_and(|fingerprint| fingerprint == &record.local_semantic_fingerprint)
                && state
                    .effective_fingerprints
                    .get(**id)
                    .is_some_and(|fingerprint| {
                        fingerprint != &record.effective_semantic_fingerprint
                    })
        })
        .map(|(id, _)| (*id).to_owned())
        .collect::<Vec<_>>();
    let mut orphaned = BTreeSet::new();
    for id in state
        .local_fingerprints
        .keys()
        .chain(state.effective_fingerprints.keys())
        .chain(state.outcomes.keys())
        .chain(state.reasons.keys())
    {
        let outcome = state.outcomes.get(id).map(String::as_str);
        if !current_ids.contains(id.as_str())
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
    let counts = json!({
        "uncovered": uncovered.len(),
        "local-fingerprint-suspect": local_suspect.len(),
        "semantic-prerequisite-changed": prerequisite_suspect.len(),
        "orphaned-state": orphaned.len(),
        "unknown-reference": unknown.len(),
        "retired-planning-identifier": planning_diagnostics.len(),
        "historical-reference": 0,
        "mapping-collision": 0,
    });
    let mut diagnostics = Vec::new();
    for id in uncovered {
        diagnostics.push(json!({"gate": "uncovered", "semantic_id": id, "next_action": format!("cargo xtask docs knowledge resolve {id} --outcome reviewed --reason <text>")}));
    }
    for id in local_suspect {
        diagnostics.push(json!({"gate": "local-fingerprint-suspect", "semantic_id": id, "next_action": format!("cargo xtask docs knowledge resolve {id} --outcome reviewed --reason <text>")}));
    }
    for id in prerequisite_suspect {
        diagnostics.push(json!({"gate": "semantic-prerequisite-changed", "semantic_id": id, "next_action": format!("cargo xtask docs knowledge resolve {id} --outcome reviewed --reason <text>")}));
    }
    for id in orphaned {
        diagnostics.push(json!({"gate": "orphaned-state", "semantic_id": id, "next_action": "remove orphaned state or resolve as superseded"}));
    }
    for reference in unknown {
        diagnostics.push(json!({"gate": "unknown-reference", "semantic_id": reference["semantic_id"], "path": reference["path"], "next_action": "remove the unknown req.* reference or add its canonical requirement"}));
    }
    diagnostics.extend(planning_diagnostics);
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
    let Some(record) = current else {
        return Err(AppError::new("unknown_requirement", id));
    };
    state
        .local_fingerprints
        .insert(id.to_owned(), record.local_semantic_fingerprint.clone());
    state
        .effective_fingerprints
        .insert(id.to_owned(), record.effective_semantic_fingerprint.clone());
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
    retired_path_preflight(app)?;
    let mut refs = Vec::new();
    let architecture_paths = architecture_catalog(app)?
        .documents
        .into_iter()
        .map(|document| document.source_path)
        .collect::<BTreeSet<_>>();
    for (directory, rust, extension) in [
        ("docs", false, "md"),
        ("crates", true, "rs"),
        ("xtask", true, "rs"),
        ("models/quint", false, "qnt"),
        ("verification/quint", false, "qnt"),
    ] {
        let path = app.root.join(directory);
        if path.exists() {
            scan_tree(
                &app.root,
                &path,
                rust,
                extension,
                app,
                &architecture_paths,
                &mut refs,
            )?;
        }
    }
    for relative in ["docs/curriculum.toml", "verification/manifest.toml"] {
        let path = app.root.join(relative);
        if path.is_file() {
            scan_file(&app.root, &path, false, app, &architecture_paths, &mut refs)?;
        }
    }
    refs.sort_by(|a, b| (&a.id, &a.kind, &a.path, a.line).cmp(&(&b.id, &b.kind, &b.path, b.line)));
    Ok(refs)
}

fn scan_tree(
    root: &Path,
    path: &Path,
    rust: bool,
    extension: &str,
    app: &App,
    architecture_paths: &BTreeSet<String>,
    refs: &mut Vec<ScanRef>,
) -> Result<(), AppError> {
    let relative = rel(root, path);
    reject_retired_path(&relative)?;
    if excluded(&relative) || architecture_paths.contains(&relative) {
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
            scan_tree(
                root,
                &entry.path(),
                rust,
                extension,
                app,
                architecture_paths,
                refs,
            )?;
        } else if kind.is_file()
            && entry.path().extension().and_then(|x| x.to_str()) == Some(extension)
        {
            scan_file(root, &entry.path(), rust, app, architecture_paths, refs)?;
        }
    }
    Ok(())
}

fn scan_file(
    root: &Path,
    path: &Path,
    rust: bool,
    app: &App,
    architecture_paths: &BTreeSet<String>,
    refs: &mut Vec<ScanRef>,
) -> Result<(), AppError> {
    let relative = rel(root, path);
    reject_retired_path(&relative)?;
    if excluded(&relative) || architecture_paths.contains(&relative) {
        return Ok(());
    }
    let text = read_bounded(path, &relative, app)?;
    for (line, content) in text.lines().enumerate() {
        if !relationship_marker(&relative, content) {
            continue;
        }
        let kind = if rust {
            "rust"
        } else if is_quint_source_path(&relative) {
            "delegated"
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
    (path.starts_with("docs/architecture/diskweave-architecture-roadmap-v")
        && path.ends_with(".md"))
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
        write_roadmap(&root);
        let app = App::new(root.clone());
        (root, app)
    }

    fn requirement(text: &str) -> String {
        format!(
            "# cap Specification\n\n## Requirements\n\n### Requirement: One\n<!-- dwv:req req.cap.one -->\n\n{text}\n"
        )
    }

    fn reviewed_state(objects: &[RequirementObject]) -> ReviewedState {
        ReviewedState {
            schema: REVIEWED_SCHEMA.to_owned(),
            local_fingerprints: objects
                .iter()
                .map(|object| {
                    (
                        object.semantic_id.clone(),
                        object.local_semantic_fingerprint.clone(),
                    )
                })
                .collect(),
            effective_fingerprints: objects
                .iter()
                .map(|object| {
                    (
                        object.semantic_id.clone(),
                        object.effective_semantic_fingerprint.clone(),
                    )
                })
                .collect(),
            outcomes: BTreeMap::new(),
            reasons: BTreeMap::new(),
        }
    }

    fn write_reviewed(root: &Path, objects: &[RequirementObject]) {
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(
            root.join(REVIEWED_PATH),
            toml::to_string(&reviewed_state(objects)).unwrap(),
        )
        .unwrap();
    }

    type SpecEntry<'a> = (&'a str, &'a [(&'a str, &'a str)], &'a str);

    fn specification(entries: &[SpecEntry<'_>]) -> String {
        let mut text = "# cap Specification\n\n## Requirements\n".to_owned();
        for (id, relationships, body) in entries {
            text.push_str(&format!("\n### Requirement: {id}\n<!-- dwv:req {id} -->\n"));
            for (kind, target) in *relationships {
                text.push_str(&format!("<!-- dwv:{kind} {target} -->\n"));
            }
            text.push_str(&format!("\n{body}\n"));
        }
        text
    }

    fn write_roadmap(root: &Path) {
        write_architecture(
            root,
            "diskweave-architecture-roadmap-v0.8.md",
            "arch.test.v0.8",
            "v0.8",
            "active",
            None,
            "",
        );
    }

    fn write_architecture(
        root: &Path,
        relative: &str,
        id: &str,
        revision: &str,
        status: &str,
        supersedes: Option<&str>,
        body: &str,
    ) {
        let path = root.join("docs/architecture").join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let predecessor = supersedes
            .map(|value| format!("supersedes: {value}\n"))
            .unwrap_or_default();
        let marker = if status == "active" {
            format!("{ACTIVE_ROADMAP_MARKER}\n")
        } else {
            String::new()
        };
        fs::write(
            path,
            format!(
                "---\nid: {id}\nseries: arch.test.series\nkind: architecture-roadmap\nrevision: {revision}\nstatus: {status}\nscope: whole-system\n{predecessor}---\n{marker}# Architecture\n\n{body}\n"
            ),
        )
        .unwrap();
    }

    fn replace_architectures(
        root: &Path,
        documents: &[(&str, &str, &str, &str, Option<&str>, &str)],
    ) {
        fs::remove_dir_all(root.join("docs/architecture")).unwrap();
        for (path, id, revision, status, supersedes, body) in documents {
            write_architecture(root, path, id, revision, status, *supersedes, body);
        }
    }

    fn semantic_fingerprints(app: &App) -> BTreeMap<String, (String, String)> {
        objects(app)
            .unwrap()
            .into_iter()
            .map(|object| {
                (
                    object.semantic_id,
                    (
                        object.local_semantic_fingerprint,
                        object.effective_semantic_fingerprint,
                    ),
                )
            })
            .collect()
    }
    fn assert_retired_path_error<T>(result: Result<T, AppError>) {
        let error = match result {
            Ok(_) => panic!("expected retired-path error"),
            Err(error) => error,
        };
        assert_eq!(error.code, "retired_planning_identifier");
        let details = error.details.expect("retired-path details");
        let diagnostics = details["diagnostics"]
            .as_array()
            .expect("retired-path diagnostics");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0]["kind"], "retired-planning-path");
        assert_eq!(
            diagnostics[0]["next_action"],
            "remove docs/milestones/**; use Beads or current product terminology"
        );
    }

    #[test]
    fn real_retained_architecture_catalog_is_valid_and_history_is_empty() {
        let app = App::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .to_owned(),
        );
        let catalog = architecture_catalog(&app).unwrap();
        assert_eq!(catalog.active_document_id, "arch.diskweave.v0.8");
        assert_eq!(
            catalog
                .documents
                .iter()
                .map(|document| (
                    document.id.as_str(),
                    document.status.as_str(),
                    document.supersedes.as_deref()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("arch.diskweave.v0.6", "superseded", None),
                (
                    "arch.diskweave.v0.7",
                    "superseded",
                    Some("arch.diskweave.v0.6")
                ),
                ("arch.diskweave.v0.8", "active", Some("arch.diskweave.v0.7")),
                (
                    "arch.diskweave.v0.9",
                    "candidate",
                    Some("arch.diskweave.v0.9-beta3")
                ),
                (
                    "arch.diskweave.v0.9-beta2",
                    "superseded",
                    Some("arch.diskweave.v0.8")
                ),
                (
                    "arch.diskweave.v0.9-beta3",
                    "superseded",
                    Some("arch.diskweave.v0.9-beta2")
                ),
            ]
        );
        assert!(catalog
            .invariants
            .iter()
            .any(|invariant| invariant.document_id == "arch.diskweave.v0.9"));
        let history = architecture_history(&app, "arch.diskweave.v0.7".to_owned()).unwrap();
        assert_eq!(history["authority"], "historical");
        assert_eq!(history["document"]["status"], "superseded");
        assert_eq!(history["invariants"], json!([]));
        assert_eq!(history["constraints"], json!([]));
    }

    #[test]
    fn unmarked_predecessor_changes_do_not_change_current_semantics() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let copy = std::env::temp_dir().join(format!("dwv-history-isolation-{nonce}"));
        fs::create_dir_all(copy.join("docs/architecture/archive")).unwrap();
        fs::create_dir_all(copy.join("docs")).unwrap();
        fs::create_dir_all(copy.join("openspec/specs/cap")).unwrap();
        fs::write(
            copy.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        write_architecture(
            &copy,
            "active.md",
            "arch.test.v2",
            "v2",
            "active",
            Some("arch.test.v1"),
            "",
        );
        write_architecture(
            &copy,
            "archive/v1.md",
            "arch.test.v1",
            "v1",
            "superseded",
            None,
            "Historical prose without invariant markers.",
        );
        let copy_app = App::new(copy.clone());
        let copy_before = semantic_fingerprints(&copy_app);
        fs::write(
            copy.join("docs/architecture/archive/v1.md"),
            "---\nid: arch.test.v1\nseries: arch.test.series\nkind: architecture-roadmap\nrevision: v1\nstatus: superseded\nscope: whole-system\n---\n# Architecture\n\nChanged historical prose without invariant markers.\n",
        )
        .unwrap();
        assert_eq!(copy_before, semantic_fingerprints(&copy_app));
        fs::remove_dir_all(copy).unwrap();
    }

    #[test]
    fn active_invariant_context_inverse_and_review_propagation_are_exact() {
        let (root, app) = fixture("active-architecture");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            specification(&[
                ("req.cap.a", &[], "A SHALL own the policy."),
                ("req.cap.b", &[("requires", "req.cap.a")], "B SHALL use A."),
                ("req.cap.c", &[], "C SHALL remain unrelated."),
            ]),
        )
        .unwrap();
        let invariant = "### Invariant: Boundary\n<!-- dwv:arch-invariant arch.test.boundary -->\n<!-- dwv:constrains req.cap.a -->\n\nThe architecture SHALL preserve boundary one.";
        replace_architectures(
            &root,
            &[("active.md", "arch.test.v1", "v1", "active", None, invariant)],
        );
        let before = objects(&app).unwrap();
        write_reviewed(&root, &before);
        let a = before
            .iter()
            .find(|object| object.semantic_id == "req.cap.a")
            .unwrap();
        let b = before
            .iter()
            .find(|object| object.semantic_id == "req.cap.b")
            .unwrap();
        let c = before
            .iter()
            .find(|object| object.semantic_id == "req.cap.c")
            .unwrap();
        assert_eq!(a.constrained_by, vec!["arch.test.boundary"]);
        let packet = context(&app, vec!["req.cap.a".to_owned()]).unwrap();
        assert_eq!(
            packet["architecture_invariants"][0]["semantic_id"],
            "arch.test.boundary"
        );
        assert_eq!(packet["architecture_constraints"][0]["target"], "req.cap.a");
        let before_fingerprints = BTreeMap::from([
            ("a", a.effective_semantic_fingerprint.clone()),
            ("b", b.effective_semantic_fingerprint.clone()),
            ("c", c.effective_semantic_fingerprint.clone()),
        ]);
        let changed = invariant.replace("boundary one", "boundary two");
        replace_architectures(
            &root,
            &[("active.md", "arch.test.v1", "v1", "active", None, &changed)],
        );
        let after = objects(&app).unwrap();
        let after_a = after
            .iter()
            .find(|object| object.semantic_id == "req.cap.a")
            .unwrap();
        let after_b = after
            .iter()
            .find(|object| object.semantic_id == "req.cap.b")
            .unwrap();
        let after_c = after
            .iter()
            .find(|object| object.semantic_id == "req.cap.c")
            .unwrap();
        assert_ne!(
            before_fingerprints["a"],
            after_a.effective_semantic_fingerprint
        );
        assert_ne!(
            before_fingerprints["b"],
            after_b.effective_semantic_fingerprint
        );
        assert_eq!(
            before_fingerprints["c"],
            after_c.effective_semantic_fingerprint
        );
        let error = readiness(&app).unwrap_err();
        assert_eq!(error.code, "knowledge_not_ready");
        let details = error.details.unwrap();
        let stale = details["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|diagnostic| diagnostic["gate"] == "semantic-prerequisite-changed")
            .map(|diagnostic| diagnostic["semantic_id"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(stale, BTreeSet::from(["req.cap.a", "req.cap.b"]));
    }

    #[test]
    fn marked_active_invariant_survives_clean_room_input_reconstruction() {
        let (root, app) = fixture("active-clean-room-source");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL preserve the current boundary."),
        )
        .unwrap();
        let unconstrained_effective = objects(&app).unwrap()[0]
            .effective_semantic_fingerprint
            .clone();
        replace_architectures(
            &root,
            &[
                (
                    "active.md",
                    "arch.test.v2",
                    "v2",
                    "active",
                    Some("arch.test.v1"),
                    "### Invariant: Current boundary\n<!-- dwv:arch-invariant arch.test.boundary -->\n<!-- dwv:constrains req.cap.one -->\n\nThe architecture SHALL preserve the active boundary.",
                ),
                (
                    "candidate.md",
                    "arch.test.v3",
                    "v3",
                    "candidate",
                    Some("arch.test.v2"),
                    "### Invariant: Candidate boundary\n<!-- dwv:arch-invariant arch.test.boundary -->\n<!-- dwv:constrains req.cap.one -->\n\nThe candidate architecture SHALL preserve a proposed boundary.",
                ),
                (
                    "archive/v1.md",
                    "arch.test.v1",
                    "v1",
                    "superseded",
                    None,
                    "### Invariant: Historical boundary\n<!-- dwv:arch-invariant arch.test.old-boundary -->\n<!-- dwv:constrains req.cap.one -->\n\nThe historical architecture SHALL preserve the former boundary.",
                ),
            ],
        );
        let source_objects = objects(&app).unwrap();
        let source_requirement = source_objects
            .iter()
            .find(|object| object.semantic_id == "req.cap.one")
            .unwrap();
        assert_ne!(
            source_requirement.effective_semantic_fingerprint,
            unconstrained_effective
        );
        assert_eq!(source_requirement.constrained_by, ["arch.test.boundary"]);
        write_reviewed(&root, &source_objects);
        let source_context = context(&app, vec!["req.cap.one".to_owned()]).unwrap();
        let source_readiness = readiness(&app).unwrap();
        assert_eq!(
            source_context["architecture_invariants"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            source_context["architecture_invariants"][0]["semantic_id"],
            "arch.test.boundary"
        );
        assert!(
            source_context["architecture_invariants"][0]["local_semantic_fingerprint"]
                .as_str()
                .is_some_and(|fingerprint| !fingerprint.is_empty())
        );
        assert_eq!(
            source_context["architecture_constraints"],
            json!([{"source": "arch.test.boundary", "target": "req.cap.one"}])
        );
        assert_eq!(source_context["references"]["documentation"], json!([]));
        assert_eq!(source_readiness["ready"], true);

        let reconstructed = root.with_extension("reconstructed");
        fs::create_dir_all(&reconstructed).unwrap();
        crate::copy_clean_room_inputs(&root, &reconstructed).unwrap();
        assert!(reconstructed.join("docs/architecture/active.md").is_file());
        assert!(
            !reconstructed
                .join("docs/architecture/candidate.md")
                .exists()
        );
        assert!(
            !reconstructed
                .join("docs/architecture/archive/v1.md")
                .exists()
        );

        let reconstructed_app = App::new(reconstructed.clone());
        assert_eq!(objects(&reconstructed_app).unwrap(), source_objects);
        assert_eq!(
            context(&reconstructed_app, vec!["req.cap.one".to_owned()]).unwrap(),
            source_context
        );
        assert_eq!(readiness(&reconstructed_app).unwrap(), source_readiness);
        let reconstructed_catalog = architecture_catalog(&reconstructed_app).unwrap();
        assert_eq!(reconstructed_catalog.documents.len(), 1);
        assert_eq!(reconstructed_catalog.active_document_id, "arch.test.v2");
        assert_eq!(
            architecture_candidate(&reconstructed_app, "arch.test.v3".to_owned())
                .unwrap_err()
                .code,
            "unknown_architecture_document"
        );
        assert_eq!(
            architecture_history(&reconstructed_app, "arch.test.v1".to_owned())
                .unwrap_err()
                .code,
            "unknown_architecture_document"
        );

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(reconstructed).unwrap();
    }

    #[test]
    fn active_context_obeys_packet_bound_without_partial_units() {
        let (root, mut app) = fixture("active-context-bound");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        replace_architectures(
            &root,
            &[(
                "active.md",
                "arch.test.v1",
                "v1",
                "active",
                None,
                "### Invariant: Boundary\n<!-- dwv:arch-invariant arch.test.boundary -->\n<!-- dwv:constrains req.cap.one -->\n\nThe architecture SHALL preserve a deliberately long boundary body.",
            )],
        );
        write_reviewed(&root, &objects(&app).unwrap());
        app.bounds.max_context_bytes = 100;
        assert_eq!(
            context(&app, vec!["req.cap.one".to_owned()])
                .unwrap_err()
                .code,
            "context_bound_exceeded"
        );
    }

    #[test]
    fn candidate_preview_is_non_authoritative_and_current_output_is_stable() {
        let (root, app) = fixture("candidate-preview");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            specification(&[
                ("req.cap.a", &[], "A SHALL own policy."),
                (
                    "req.cap.b",
                    &[("requires", "req.cap.a")],
                    "B SHALL depend on A.",
                ),
            ]),
        )
        .unwrap();
        let active = "### Invariant: Boundary\n<!-- dwv:arch-invariant arch.test.boundary -->\n<!-- dwv:constrains req.cap.a -->\n\nThe architecture SHALL preserve one boundary.";
        replace_architectures(
            &root,
            &[("active.md", "arch.test.v1", "v1", "active", None, active)],
        );
        let current_before = semantic_fingerprints(&app);
        write_reviewed(&root, &objects(&app).unwrap());
        let candidate = "### Invariant: Boundary\n<!-- dwv:arch-invariant arch.test.boundary -->\n<!-- dwv:constrains req.cap.b -->\n\nThe architecture SHALL preserve a changed boundary.\n\n### Invariant: Unlinked\n<!-- dwv:arch-invariant arch.test.unlinked -->\n\nThe architecture SHALL preserve an unlinked boundary.";
        write_architecture(
            &root,
            "candidate.md",
            "arch.test.v2",
            "v2",
            "candidate",
            Some("arch.test.v1"),
            candidate,
        );
        let readiness_before = readiness(&app).unwrap();
        assert_eq!(readiness_before["ready"], true);
        let current_with_candidate = semantic_fingerprints(&app);
        let packet = context(&app, vec!["req.cap.a".to_owned()]).unwrap();
        assert!(
            packet["architecture_invariants"][0]["body"]
                .as_str()
                .unwrap()
                .contains("preserve one boundary")
        );
        assert_eq!(readiness(&app).unwrap(), readiness_before);
        assert_eq!(current_before, current_with_candidate);
        let preview = architecture_candidate(&app, "arch.test.v2".to_owned()).unwrap();
        assert_eq!(preview["authority"], "non-authoritative-candidate");
        assert_eq!(
            preview["comparisons"]
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| (
                    entry["semantic_id"].as_str().unwrap(),
                    entry["classification"].as_str().unwrap()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("arch.test.boundary", "changed"),
                ("arch.test.unlinked", "added"),
            ]
        );
        assert_eq!(
            preview["dependent_closure"],
            json!(["req.cap.a", "req.cap.b"])
        );
        fs::remove_file(root.join("docs/architecture/candidate.md")).unwrap();
        assert_eq!(current_before, semantic_fingerprints(&app));
    }

    #[test]
    fn invariant_fingerprint_ignores_format_location_and_lifecycle() {
        let (root, app) = fixture("invariant-fingerprint");
        let first = "### Invariant: Boundary\n<!-- dwv:arch-invariant arch.test.boundary -->\n\nThe architecture SHALL preserve one boundary.";
        replace_architectures(
            &root,
            &[("active.md", "arch.test.v1", "v1", "active", None, first)],
        );
        let initial = architecture_catalog(&app).unwrap().invariants[0]
            .local_semantic_fingerprint
            .clone();
        let moved = "## Context\n\n#### Invariant: Boundary\n<!-- dwv:arch-invariant arch.test.boundary -->\n\nThe architecture SHALL preserve one\nboundary.";
        replace_architectures(
            &root,
            &[
                (
                    "archive/v1.md",
                    "arch.test.v1",
                    "v1",
                    "superseded",
                    None,
                    first,
                ),
                (
                    "candidate.md",
                    "arch.test.v2",
                    "v2",
                    "candidate",
                    Some("arch.test.v1"),
                    moved,
                ),
                (
                    "active.md",
                    "arch.test.v3",
                    "v3",
                    "active",
                    Some("arch.test.v2"),
                    "",
                ),
            ],
        );
        let catalog = architecture_catalog(&app).unwrap();
        let moved_fingerprint = catalog
            .invariants
            .iter()
            .find(|invariant| invariant.document_id == "arch.test.v2")
            .unwrap()
            .local_semantic_fingerprint
            .clone();
        assert_eq!(initial, moved_fingerprint);
        let changed = moved.replace("one", "another");
        replace_architectures(
            &root,
            &[("active.md", "arch.test.v2", "v2", "active", None, &changed)],
        );
        assert_ne!(
            initial,
            architecture_catalog(&app).unwrap().invariants[0].local_semantic_fingerprint
        );
    }

    #[test]
    fn candidate_unchanged_classification_includes_zero_edge_identity() {
        let (root, app) = fixture("candidate-unchanged");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        let invariant = "### Invariant: Unlinked\n<!-- dwv:arch-invariant arch.test.unlinked -->\n\nThe architecture SHALL preserve an unlinked boundary.";
        replace_architectures(
            &root,
            &[("active.md", "arch.test.v1", "v1", "active", None, invariant)],
        );
        write_architecture(
            &root,
            "candidate.md",
            "arch.test.v2",
            "v2",
            "candidate",
            Some("arch.test.v1"),
            invariant,
        );
        let preview = architecture_candidate(&app, "arch.test.v2".to_owned()).unwrap();
        assert_eq!(preview["comparisons"][0]["classification"], "unchanged");
        assert_eq!(preview["comparisons"][0]["added_constraints"], json!([]));
        assert_eq!(preview["comparisons"][0]["removed_constraints"], json!([]));
        assert_eq!(preview["directly_affected_requirements"], json!([]));
    }

    #[test]
    fn candidate_preview_obeys_packet_bound_without_partial_units() {
        let (root, mut app) = fixture("candidate-bound");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        write_architecture(
            &root,
            "candidate.md",
            "arch.test.v2",
            "v2",
            "candidate",
            Some("arch.test.v0.8"),
            "### Invariant: Candidate\n<!-- dwv:arch-invariant arch.test.candidate -->\n\nCandidate policy.",
        );
        app.bounds.max_context_bytes = 100;
        assert_eq!(
            architecture_candidate(&app, "arch.test.v2".to_owned())
                .unwrap_err()
                .code,
            "architecture_candidate_bound_exceeded"
        );
    }

    #[test]
    fn lineage_preserves_replacement_split_and_merge_edges() {
        let (root, app) = fixture("lineage");
        replace_architectures(
            &root,
            &[
                (
                    "archive/v1.md",
                    "arch.test.v1",
                    "v1",
                    "superseded",
                    None,
                    "### Invariant: Old\n<!-- dwv:arch-invariant arch.test.old -->\n\nOld policy.",
                ),
                (
                    "archive/v2.md",
                    "arch.test.v2",
                    "v2",
                    "superseded",
                    Some("arch.test.v1"),
                    "### Invariant: Left\n<!-- dwv:arch-invariant arch.test.left -->\n<!-- dwv:arch-supersedes arch.test.old -->\n\nLeft policy.\n\n### Invariant: Right\n<!-- dwv:arch-invariant arch.test.right -->\n<!-- dwv:arch-supersedes arch.test.old -->\n\nRight policy.",
                ),
                (
                    "candidate.md",
                    "arch.test.v3",
                    "v3",
                    "candidate",
                    Some("arch.test.v2"),
                    "### Invariant: Merged\n<!-- dwv:arch-invariant arch.test.merged -->\n<!-- dwv:arch-supersedes arch.test.left -->\n<!-- dwv:arch-supersedes arch.test.right -->\n\nMerged policy.",
                ),
                (
                    "active.md",
                    "arch.test.v4",
                    "v4",
                    "active",
                    Some("arch.test.v3"),
                    "",
                ),
            ],
        );
        let catalog = architecture_catalog(&app).unwrap();
        validate_architecture_semantics(&mut objects(&app).unwrap(), &catalog).unwrap();
        let old = catalog
            .invariants
            .iter()
            .find(|invariant| invariant.semantic_id == "arch.test.old")
            .unwrap();
        assert_eq!(old.superseded_by, vec!["arch.test.left", "arch.test.right"]);
        let merged = catalog
            .invariants
            .iter()
            .find(|invariant| invariant.semantic_id == "arch.test.merged")
            .unwrap();
        assert_eq!(merged.supersedes, vec!["arch.test.left", "arch.test.right"]);
        let history = architecture_history(&app, "arch.test.v2".to_owned()).unwrap();
        assert_eq!(history["authority"], "historical");
        assert_eq!(history["document"]["status"], "superseded");
        assert_eq!(
            history["invariants"]
                .as_array()
                .unwrap()
                .iter()
                .map(|invariant| invariant["semantic_id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["arch.test.left", "arch.test.right"]
        );
        assert_eq!(
            history["invariants"][0]["superseded_by"],
            json!(["arch.test.merged"])
        );
    }

    #[test]
    fn malformed_lifecycle_duplicate_and_dangling_constraints_fail() {
        let (root, app) = fixture("architecture-invalid");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        write_architecture(
            &root,
            "candidate.md",
            "arch.test.v2",
            "v2",
            "active",
            None,
            "",
        );
        assert_eq!(
            objects(&app).unwrap_err().code,
            "active_architecture_selection_invalid"
        );
        fs::remove_file(root.join("docs/architecture/candidate.md")).unwrap();
        replace_architectures(
            &root,
            &[(
                "active.md",
                "arch.test.v1",
                "v1",
                "active",
                None,
                "### Invariant: One\n<!-- dwv:arch-invariant arch.test.same -->\n\nOne.\n\n### Invariant: Two\n<!-- dwv:arch-invariant arch.test.same -->\n\nTwo.",
            )],
        );
        assert_eq!(
            objects(&app).unwrap_err().code,
            "architecture_invariant_duplicate"
        );
        replace_architectures(
            &root,
            &[(
                "active.md",
                "arch.test.v1",
                "v1",
                "active",
                None,
                "### Invariant: One\n<!-- dwv:arch-invariant arch.test.one -->\n<!-- dwv:constrains req.cap.unknown -->\n\nOne.",
            )],
        );
        assert_eq!(
            objects(&app).unwrap_err().code,
            "architecture_constraint_dangling"
        );
        replace_architectures(
            &root,
            &[(
                "active.md",
                "arch.test.v1",
                "v1",
                "active",
                None,
                "### Invariant: One\n<!-- dwv:arch-invariant arch.test.one -->\n<!-- dwv:constrains req.cap.one -->\n<!-- dwv:constrains req.cap.one -->\n\nOne.",
            )],
        );
        assert_eq!(
            objects(&app).unwrap_err().code,
            "architecture_constraint_duplicate"
        );
    }

    #[test]
    fn production_discovery_ignores_architecture_text_outside_its_root() {
        let (root, app) = fixture("architecture-root-isolation");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        fs::write(
            root.join("fixture.md"),
            "---\nid: arch.test.fake\nseries: arch.test.series\nkind: architecture-roadmap\nrevision: fake\nstatus: active\nscope: whole-system\n---\n<!-- dwv:active-architecture-roadmap -->\n### Invariant: Fake\n<!-- dwv:arch-invariant arch.test.fake-invariant -->",
        )
        .unwrap();
        let catalog = architecture_catalog(&app).unwrap();
        assert_eq!(catalog.documents.len(), 1);
        assert!(catalog.invariants.is_empty());
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
    fn planning_scanner_rejects_retired_project_identifiers() {
        for number in 1..=8 {
            assert!(retired_planning_match(&format!("{}-v{number}", "goal")).is_some());
            assert!(retired_planning_match(&format!("{} v{number}", "Goal")).is_some());
        }
        for sample in [
            format!("docs/handoffs/{}.md", "goal"),
            format!("docs/handoffs/{}-v2.md", "goal"),
            format!("close-{}-v7-acceptance-gaps", "goal"),
            format!("dwv-{}-v7", "goal"),
        ] {
            assert!(retired_planning_match(&sample).is_some());
        }
    }

    #[test]
    fn planning_scanner_allows_ordinary_goal_language() {
        for sample in [
            "## Goals",
            "## Non-Goals",
            "The product goal is safe storage.",
            "This check preserves goal-oriented ordinary prose.",
        ] {
            assert_eq!(retired_planning_match(sample), None);
        }
    }

    #[test]
    fn planning_scanner_reports_exact_content_location() {
        let (root, app) = fixture("planning-scan-location");
        fs::write(
            root.join("notes.txt"),
            format!("ordinary goal\n{}-v7\n", "goal"),
        )
        .unwrap();

        let error = planning_nomenclature(&app).unwrap_err();
        assert_eq!(error.code, "retired_planning_identifier");
        let diagnostics = error.details.unwrap()["diagnostics"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0]["path"], "notes.txt");
        assert_eq!(diagnostics[0]["source"], "content");
        assert_eq!(diagnostics[0]["line"], 2);
        assert_eq!(diagnostics[0]["column"], 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn planning_scanner_uses_independent_path_capacity() {
        let (root, mut app) = fixture("planning-scan-bound");
        fs::write(root.join("notes.txt"), "ordinary goal language\n").unwrap();

        app.bounds.max_units = 1;
        app.bounds.max_planning_scan_paths = 2;
        assert!(planning_nomenclature(&app).is_ok());

        app.bounds.max_planning_scan_paths = 1;
        let error = planning_nomenclature(&app).unwrap_err();
        assert_eq!(error.code, "planning_scan_path_bound_exceeded");
        fs::remove_dir_all(root).unwrap();
    }
    fn commit_git_fixture(root: &Path) {
        for args in [
            vec!["init", "-q"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=DiskWeave Test",
                "-c",
                "user.email=test@diskweave.invalid",
                "commit",
                "-qm",
                "baseline",
            ],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(root)
                    .status()
                    .unwrap()
                    .success()
            );
        }
    }
    #[test]
    fn references_exclude_history_and_hidden_sources() {
        assert!(excluded("docs/handoffs/old.md"));
        assert!(excluded(
            "docs/architecture/diskweave-architecture-roadmap-v0.8.md"
        ));
        assert!(excluded(
            "docs/architecture/archive/diskweave-architecture-roadmap-v0.7.md"
        ));
        assert!(!excluded(
            "docs/architecture/derived-documentation-system.md"
        ));
        assert!(excluded("target/generated.md"));
        assert!(!excluded("docs/milestones/OS-001.md"));
        assert!(!is_impact_candidate("docs/milestones/OS-001.md"));
        assert!(!excluded("docs/verification/current.md"));
    }

    #[test]
    fn recreated_retired_path_fails_closed_before_knowledge_consumers() {
        let (root, app) = fixture("retired-path-boundary");
        let path = root.join(RETIRED_PATH).join("m11.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, [0xff, 0xfe]).unwrap();

        assert_retired_path_error(retired_path_preflight(&app));
        assert_retired_path_error(objects(&app));
        assert_retired_path_error(export(&app));
        assert_retired_path_error(inspect(&app, "req.cap.one".to_owned()));
        assert_retired_path_error(context(&app, vec!["req.cap.one".to_owned()]));
        assert_retired_path_error(audit_context(&app, "req.cap.one".to_owned()));
        assert_retired_path_error(ownership(&app, "req.cap.one".to_owned()));
        assert_retired_path_error(affected(
            &app,
            Vec::new(),
            vec!["docs/milestones/m11.md".to_owned()],
        ));
        assert_retired_path_error(doctor(
            &app,
            vec!["docs/milestones/m11.md".to_owned()],
        ));
        assert_retired_path_error(change_impact(&app, None));
        assert_retired_path_error(validate_roadmap_identifiers(&app));
        assert_retired_path_error(architecture_catalog(&app));
        assert_retired_path_error(scan_references(&app));
        assert_retired_path_error(readiness(&app));
        assert_retired_path_error(planning_nomenclature(&app));
        assert_retired_path_error(crate::clean_room(&app));

        let mut markdown = Vec::new();
        assert_retired_path_error(collect_markdown_files(
            &root,
            &root.join(RETIRED_PATH),
            &mut markdown,
        ));
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
            "let sample = \"req.a.fake\";\nlet fixture = \"<!-- dwv:req req.a.fixture -->\";\n/// dwv:",
            "req req.a.real\nfn owner() {}\n",
        ]
        .concat();
        let ids = relationship_ids("crates/example.rs", &marker);
        assert_eq!(ids, BTreeSet::from(["req.a.real".to_owned()]));
    }

    #[test]
    fn quint_relationships_scan_as_delegated_sources() {
        let (root, app) = fixture("quint-reference-scan");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        let path = root.join("verification/quint/RecoveryProtocol.qnt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "const Fake: str = \"req.cap.fake\"\n// dwv:req req.cap.one\n",
        )
        .unwrap();

        assert!(is_impact_candidate(
            "verification/quint/RecoveryProtocol.qnt"
        ));
        assert_eq!(
            relationship_ids(
                "verification/quint/RecoveryProtocol.qnt",
                &fs::read_to_string(&path).unwrap()
            ),
            BTreeSet::from(["req.cap.one".to_owned()])
        );
        let references = scan_references(&app).unwrap();
        let delegated = references
            .iter()
            .filter(|reference| reference.path == "verification/quint/RecoveryProtocol.qnt")
            .collect::<Vec<_>>();
        assert_eq!(delegated.len(), 1);
        assert_eq!(delegated[0].kind, "delegated");
        assert_eq!(delegated[0].id, "req.cap.one");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn change_impact_requires_review_for_delegated_model_changes() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let (root, app) = fixture("quint-impact");
        let spec = root.join("openspec/specs/cap/spec.md");
        fs::write(&spec, requirement("The system SHALL remain stable.")).unwrap();
        write_reviewed(&root, &objects(&app).unwrap());
        let path = root.join("models/quint/RecoveryProtocol.qnt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "// dwv:req req.cap.one\nval modelRevision: int = 1\n",
        )
        .unwrap();
        commit_git_fixture(&root);
        fs::write(
            &path,
            "// dwv:req req.cap.one\nval modelRevision: int = 2\n",
        )
        .unwrap();

        let result = change_impact(&app, Some("HEAD")).unwrap();
        assert_eq!(result["review_required"], true);
        assert_eq!(result["review_requirement_ids"], json!(["req.cap.one"]));
        let entry = result["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["path"] == "models/quint/RecoveryProtocol.qnt")
            .unwrap();
        assert_eq!(
            entry["classification"],
            "delegated_canonical_source_changed"
        );
        assert_eq!(entry["review_required"], true);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unmarked_quint_path_has_no_owner() {
        let (root, app) = fixture("unmarked-quint");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        let path = root.join("models/quint/Unmarked.qnt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "val modelRevision: int = 1\nreq.cap.one\n").unwrap();

        assert!(is_impact_candidate("models/quint/Unmarked.qnt"));
        assert!(
            relationship_ids(
                "models/quint/Unmarked.qnt",
                &fs::read_to_string(&path).unwrap()
            )
            .is_empty()
        );
        assert!(
            !scan_references(&app)
                .unwrap()
                .iter()
                .any(|reference| reference.path == "models/quint/Unmarked.qnt")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn quint_marker_removal_is_a_relationship_reassignment() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let (root, app) = fixture("quint-marker-removal");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        write_reviewed(&root, &objects(&app).unwrap());
        let path = root.join("verification/quint/RecoveryProtocol.qnt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "// dwv:req req.cap.one\nval modelRevision: int = 1\n",
        )
        .unwrap();
        commit_git_fixture(&root);
        fs::write(&path, "val modelRevision: int = 2\n").unwrap();

        let result = change_impact(&app, Some("HEAD")).unwrap();
        let entry = result["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["path"] == "verification/quint/RecoveryProtocol.qnt")
            .unwrap();
        assert_eq!(
            entry["classification"],
            "current_requirement_relationship_removed_or_reassigned"
        );
        assert_eq!(entry["review_required"], true);
        assert_eq!(entry["requirement_ids"], json!(["req.cap.one"]));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn marked_quint_change_propagates_to_dependents() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let (root, app) = fixture("quint-dependent-impact");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            specification(&[
                (
                    "req.cap.one",
                    &[] as &[(&str, &str)],
                    "The owner SHALL remain stable.",
                ),
                (
                    "req.cap.two",
                    &[("requires", "req.cap.one")],
                    "The dependent SHALL preserve the owner boundary.",
                ),
            ]),
        )
        .unwrap();
        write_reviewed(&root, &objects(&app).unwrap());
        let path = root.join("models/quint/RecoveryProtocol.qnt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "// dwv:req req.cap.one\nval modelRevision: int = 1\n",
        )
        .unwrap();
        commit_git_fixture(&root);
        fs::write(
            &path,
            "// dwv:req req.cap.one\nval modelRevision: int = 2\n",
        )
        .unwrap();

        let result = change_impact(&app, Some("HEAD")).unwrap();
        assert_eq!(
            result["review_requirement_ids"],
            json!(["req.cap.one", "req.cap.two"])
        );
        fs::remove_dir_all(root).unwrap();
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
        commit_git_fixture(&root);
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
    fn change_impact_does_not_broaden_a_retired_reference_removal() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let (root, app) = fixture("retired-reference-impact");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            requirement("The system SHALL remain stable."),
        )
        .unwrap();
        write_reviewed(&root, &objects(&app).unwrap());
        fs::create_dir_all(root.join("verification")).unwrap();
        let manifest = root.join("verification/manifest.toml");
        fs::write(&manifest, "req.cap.one\nreq.cap.retired\n").unwrap();
        commit_git_fixture(&root);
        fs::write(&manifest, "req.cap.one\n").unwrap();

        let result = change_impact(&app, Some("HEAD")).unwrap();
        assert_eq!(result["review_required"], false);
        assert!(
            result["review_requirement_ids"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(result["diagnostics"].as_array().unwrap().is_empty());
        assert_eq!(
            result["entries"][0]["classification"],
            "noncurrent_reference_removed"
        );
        assert_eq!(
            result["entries"][0]["removed_ids"],
            json!(["req.cap.retired"])
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
            "schema = 'dwv.knowledge.reviewed-links.v2'\n[local_fingerprints]\n\"req.a.one\" = \"abc\"\n[effective_fingerprints]\n\"req.a.one\" = \"def\"\n",
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
        let state = reviewed_state(std::slice::from_ref(&object));
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
        assert_eq!(
            result["impacts"][0]["ownership"]["omitted"]["req.cap.one"]["documentation"],
            1
        );
        assert_eq!(result["impacts"][0]["pages"].as_array().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reviewed_lifecycle_distinguishes_formatting_and_semantic_changes() {
        let (root, app) = fixture("lifecycle");
        let spec = root.join("openspec/specs/cap/spec.md");
        fs::write(&spec, requirement("The system SHALL remain stable.")).unwrap();
        let object = objects(&app).unwrap().remove(0);
        let state = reviewed_state(std::slice::from_ref(&object));
        write_roadmap(&root);
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
            error.details.unwrap()["gate_counts"]["local-fingerprint-suspect"],
            1
        );
        let packet = context(&app, vec!["req.cap.one".to_owned()]).unwrap();
        assert_eq!(
            packet["references"]["documentation"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            packet["references"]["documentation"][0]["path"],
            "docs/affected.md"
        );
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

    #[test]
    fn relationships_derive_backlinks_aggregation_and_owner_first_order() {
        let (root, app) = fixture("relationship-graph");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            specification(&[
                ("req.cap.a", &[("requires", "req.cap.b")], "A SHALL use B."),
                (
                    "req.cap.b",
                    &[("refines", "req.cap.c")],
                    "B SHALL specialize C.",
                ),
                ("req.cap.c", &[], "C SHALL own the policy."),
            ]),
        )
        .unwrap();
        let model = knowledge_model(&app).unwrap();
        let a = model
            .objects
            .iter()
            .find(|object| object.semantic_id == "req.cap.a")
            .unwrap();
        let b = model
            .objects
            .iter()
            .find(|object| object.semantic_id == "req.cap.b")
            .unwrap();
        assert_eq!(a.requires, ["req.cap.b"]);
        assert_eq!(b.required_by, ["req.cap.a"]);
        assert_eq!(b.refines, ["req.cap.c"]);
        let component =
            ordinary_context_component(&model, &BTreeSet::from(["req.cap.a".to_owned()]));
        assert_eq!(
            topological_order(&model, &component),
            ["req.cap.c", "req.cap.b", "req.cap.a"]
        );
        let aggregation = capability_aggregation(&model.objects);
        assert_eq!(
            aggregation["cap"]["requirements"].as_array().unwrap().len(),
            3
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inspect_uses_exact_source_locator_without_line_sensitive_identity() {
        let (root, mut app) = fixture("inspect-source-locator");
        let spec = root.join("openspec/specs/cap/spec.md");
        fs::write(&spec, requirement("The system SHALL remain stable.")).unwrap();
        app.bounds.max_context_bytes = 1;

        let before = inspect(&app, "req.cap.one".to_owned()).unwrap();
        assert_eq!(before["schema"], INSPECT_SCHEMA);
        assert_eq!(
            before["requirement"]["source"],
            "openspec/specs/cap/spec.md:5-8"
        );
        assert!(before["requirement"].get("source_path").is_none());
        assert!(before["requirement"].get("heading_path").is_none());

        fs::write(
            &spec,
            format!("\n\n{}", requirement("The system SHALL remain stable.")),
        )
        .unwrap();
        let after = inspect(&app, "req.cap.one".to_owned()).unwrap();
        assert_eq!(
            after["requirement"]["source"],
            "openspec/specs/cap/spec.md:7-10"
        );
        for field in [
            "semantic_id",
            "title",
            "capability",
            "local_semantic_fingerprint",
            "effective_semantic_fingerprint",
            "body",
        ] {
            assert_eq!(before["requirement"][field], after["requirement"][field]);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn context_batches_units_and_keeps_whole_component_for_explicit_audit() {
        let (root, mut app) = fixture("bounded-context");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            specification(&[
                (
                    "req.cap.a",
                    &[("requires", "req.cap.b"), ("requires", "req.cap.x")],
                    "A SHALL combine B and X.",
                ),
                ("req.cap.b", &[("requires", "req.cap.c")], "B SHALL use C."),
                ("req.cap.c", &[], "C SHALL own the policy."),
                ("req.cap.d", &[("requires", "req.cap.a")], "D SHALL use A."),
                ("req.cap.x", &[], "X SHALL own an unrelated input."),
                ("req.cap.y", &[], "Y SHALL own an independent policy."),
            ]),
        )
        .unwrap();
        assert_eq!(
            context(&app, vec!["req.cap.unknown".to_owned()])
                .unwrap_err()
                .code,
            "unknown_requirement"
        );
        write_reviewed(&root, &objects(&app).unwrap());

        let packet = context(
            &app,
            vec![
                "req.cap.b".to_owned(),
                "req.cap.c".to_owned(),
                "req.cap.b".to_owned(),
            ],
        )
        .unwrap();
        assert_eq!(packet["schema"], CONTEXT_SCHEMA);
        let requirement_ids = packet["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|unit| unit["semantic_id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(requirement_ids, ["req.cap.c", "req.cap.b", "req.cap.a"]);
        assert!(
            packet["requirements"]
                .as_array()
                .unwrap()
                .iter()
                .all(|unit| unit["body"].as_str().is_some())
        );
        assert_eq!(
            packet["reading_order"],
            json!(["req.cap.c", "req.cap.b", "req.cap.a"])
        );
        assert_eq!(packet["relationships"].as_array().unwrap().len(), 2);
        assert!(!packet.to_string().contains("req.cap.x"));

        let overlapping = [
            inspect(&app, "req.cap.b".to_owned()).unwrap(),
            ownership(&app, "req.cap.b".to_owned()).unwrap(),
            context(&app, vec!["req.cap.b".to_owned()]).unwrap(),
            inspect(&app, "req.cap.c".to_owned()).unwrap(),
            ownership(&app, "req.cap.c".to_owned()).unwrap(),
            context(&app, vec!["req.cap.c".to_owned()]).unwrap(),
        ];
        let overlapping_bytes = overlapping
            .iter()
            .map(|value| serde_json::to_vec(value).unwrap().len())
            .sum::<usize>();
        assert!(serde_json::to_vec(&packet).unwrap().len() < overlapping_bytes);

        let audit = audit_context(&app, "req.cap.b".to_owned()).unwrap();
        assert_eq!(audit["schema"], AUDIT_CONTEXT_SCHEMA);
        assert_eq!(
            audit["reading_order"],
            json!([
                "req.cap.c",
                "req.cap.b",
                "req.cap.x",
                "req.cap.a",
                "req.cap.d"
            ])
        );
        assert_eq!(
            audit["requirements"]
                .as_array()
                .unwrap()
                .iter()
                .map(|unit| unit["semantic_id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            audit["reading_order"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_str().unwrap())
                .collect::<Vec<_>>()
        );

        let b = context(&app, vec!["req.cap.b".to_owned()]).unwrap();
        let y = context(&app, vec!["req.cap.y".to_owned()]).unwrap();
        let a = context(&app, vec!["req.cap.a".to_owned()]).unwrap();
        let d = context(&app, vec!["req.cap.d".to_owned()]).unwrap();
        app.bounds.max_context_bytes = serde_json::to_vec(&b)
            .unwrap()
            .len()
            .max(serde_json::to_vec(&y).unwrap().len());
        let split =
            context(&app, vec!["req.cap.b".to_owned(), "req.cap.y".to_owned()]).unwrap_err();
        assert_eq!(
            split.details.unwrap()["suggested_requests"],
            json!([["req.cap.b"], ["req.cap.y"]])
        );

        app.bounds.max_context_bytes = serde_json::to_vec(&a)
            .unwrap()
            .len()
            .max(serde_json::to_vec(&d).unwrap().len());
        let shared =
            context(&app, vec!["req.cap.a".to_owned(), "req.cap.d".to_owned()]).unwrap_err();
        assert_eq!(
            shared.details.unwrap()["suggested_requests"],
            json!([["req.cap.a"], ["req.cap.d"]])
        );

        app.bounds.max_context_bytes = 1;
        let same_scope =
            context(&app, vec!["req.cap.c".to_owned(), "req.cap.b".to_owned()]).unwrap_err();
        assert_eq!(same_scope.code, "context_bound_exceeded");
        assert_eq!(same_scope.details.unwrap()["suggested_requests"], json!([]));
        let audit_error = audit_context(&app, "req.cap.b".to_owned()).unwrap_err();
        assert_eq!(audit_error.code, "audit_context_bound_exceeded");
        assert_eq!(
            audit_error.details.unwrap()["suggested_requests"],
            json!([])
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn batched_reference_omissions_are_attributable() {
        let (root, mut app) = fixture("reference-omissions");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            specification(&[
                ("req.cap.a", &[], "A SHALL own one policy."),
                ("req.cap.b", &[], "B SHALL own another policy."),
            ]),
        )
        .unwrap();
        write_reviewed(&root, &objects(&app).unwrap());
        for id in ["a", "b"] {
            for index in 0..3 {
                fs::write(
                    root.join(format!("docs/{id}-{index}.md")),
                    format!("Explanation for req.cap.{id}.\n"),
                )
                .unwrap();
            }
        }
        app.bounds.max_units = 2;

        let packet = context(&app, vec!["req.cap.a".to_owned(), "req.cap.b".to_owned()]).unwrap();
        assert_eq!(
            packet["omitted"],
            json!({
                "req.cap.a": {
                    "implementation": 0,
                    "evidence": 0,
                    "curriculum": 0,
                    "documentation": 1,
                },
                "req.cap.b": {
                    "implementation": 0,
                    "evidence": 0,
                    "curriculum": 0,
                    "documentation": 3,
                },
            })
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_relationship_edges_fail_with_structured_diagnostics() {
        let cases = [
            (
                "self",
                specification(&[("req.cap.a", &[("requires", "req.cap.a")], "A SHALL exist.")]),
                "self_relationship",
            ),
            (
                "duplicate",
                specification(&[
                    (
                        "req.cap.a",
                        &[("requires", "req.cap.b"), ("requires", "req.cap.b")],
                        "A SHALL use B.",
                    ),
                    ("req.cap.b", &[], "B SHALL exist."),
                ]),
                "duplicate_relationship",
            ),
            (
                "mixed",
                specification(&[
                    (
                        "req.cap.a",
                        &[("requires", "req.cap.b"), ("refines", "req.cap.b")],
                        "A SHALL use B.",
                    ),
                    ("req.cap.b", &[], "B SHALL exist."),
                ]),
                "mixed_relationship_kind",
            ),
            (
                "retired",
                specification(&[(
                    "req.cap.a",
                    &[("requires", "req.cap.retired")],
                    "A SHALL use the retired owner.",
                )]),
                "unknown_relationship_target",
            ),
        ];
        for (name, spec, code) in cases {
            let (root, app) = fixture(name);
            fs::write(root.join("openspec/specs/cap/spec.md"), spec).unwrap();
            if name == "retired" {
                fs::create_dir_all(root.join("openspec/changes/archive/old/specs/cap")).unwrap();
                fs::write(
                    root.join("openspec/changes/archive/old/specs/cap/spec.md"),
                    specification(&[("req.cap.retired", &[], "Historical only.")]),
                )
                .unwrap();
            }
            let error = objects(&app).unwrap_err();
            assert_eq!(error.code, code);
            let details = error.details.unwrap();
            assert_eq!(details["source_id"], "req.cap.a");
            assert!(
                details["source_path"]
                    .as_str()
                    .unwrap()
                    .ends_with("spec.md")
            );
            assert!(details["line"].as_u64().unwrap() > 0);
            fs::remove_dir_all(root).unwrap();
        }

        let (root, app) = fixture("misplaced");
        fs::write(
            root.join("openspec/specs/cap/spec.md"),
            "# cap Specification\n\n## Requirements\n\n### Requirement: A\n<!-- dwv:req req.cap.a -->\n\n<!-- dwv:requires req.cap.b -->\n\nA SHALL use B.\n\n### Requirement: B\n<!-- dwv:req req.cap.b -->\n\nB SHALL exist.\n",
        )
        .unwrap();
        assert_eq!(
            objects(&app).unwrap_err().code,
            "misplaced_relationship_marker"
        );
        fs::remove_dir_all(root).unwrap();

        let (root, app) = fixture("missing-edge-boundary");
        let spec = root.join("openspec/specs/cap/spec.md");
        fs::write(&spec, requirement("A SHALL stand alone.")).unwrap();
        assert_eq!(objects(&app).unwrap().len(), 1);
        fs::write(
            &spec,
            "# cap Specification\n\n## Requirements\n\n### Requirement: A\n<!-- dwv:req req.cap.a -->\n<!-- dwv:requires -->\n\nA SHALL use an owner.\n",
        )
        .unwrap();
        assert_eq!(
            objects(&app).unwrap_err().code,
            "invalid_relationship_marker"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn direct_and_multi_node_cycles_report_the_smallest_stable_cycle() {
        let cases = [
            (
                "direct-cycle",
                specification(&[
                    ("req.cap.a", &[("requires", "req.cap.b")], "A SHALL use B."),
                    (
                        "req.cap.b",
                        &[("refines", "req.cap.a")],
                        "B SHALL specialize A.",
                    ),
                ]),
                vec!["req.cap.a", "req.cap.b", "req.cap.a"],
            ),
            (
                "multi-cycle",
                specification(&[
                    ("req.cap.a", &[("requires", "req.cap.b")], "A SHALL use B."),
                    ("req.cap.b", &[("requires", "req.cap.c")], "B SHALL use C."),
                    (
                        "req.cap.c",
                        &[("refines", "req.cap.a")],
                        "C SHALL specialize A.",
                    ),
                ]),
                vec!["req.cap.a", "req.cap.b", "req.cap.c", "req.cap.a"],
            ),
        ];
        for (name, spec, expected) in cases {
            let (root, app) = fixture(name);
            fs::write(root.join("openspec/specs/cap/spec.md"), spec).unwrap();
            let error = objects(&app).unwrap_err();
            assert_eq!(error.code, "relationship_cycle");
            assert_eq!(
                error.details.unwrap()["cycle"],
                serde_json::to_value(expected).unwrap()
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn effective_fingerprints_invalidate_dependency_closure_only() {
        let (root, app) = fixture("effective-fingerprint");
        let spec = root.join("openspec/specs/cap/spec.md");
        let chain = |c_body: &str| {
            specification(&[
                ("req.cap.a", &[("requires", "req.cap.b")], "A SHALL use B."),
                ("req.cap.b", &[("requires", "req.cap.c")], "B SHALL use C."),
                ("req.cap.c", &[], c_body),
                ("req.cap.d", &[], "D SHALL remain unrelated."),
            ])
        };
        fs::write(&spec, chain("C SHALL own the policy.")).unwrap();
        let before = objects(&app).unwrap();
        write_reviewed(&root, &before);
        write_roadmap(&root);
        fs::write(&spec, chain("C SHALL own the durable policy.")).unwrap();

        let error = readiness(&app).unwrap_err();
        let diagnostics = error.details.unwrap()["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic["semantic_id"].as_str().unwrap().to_owned(),
                    diagnostic["gate"].as_str().unwrap().to_owned(),
                )
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            diagnostics,
            BTreeSet::from([
                (
                    "req.cap.a".to_owned(),
                    "semantic-prerequisite-changed".to_owned(),
                ),
                (
                    "req.cap.b".to_owned(),
                    "semantic-prerequisite-changed".to_owned(),
                ),
                (
                    "req.cap.c".to_owned(),
                    "local-fingerprint-suspect".to_owned(),
                ),
            ])
        );
        let after = objects(&app).unwrap();
        let c_before = before
            .iter()
            .find(|object| object.semantic_id == "req.cap.c")
            .unwrap();
        let c_after = after
            .iter()
            .find(|object| object.semantic_id == "req.cap.c")
            .unwrap();
        assert_ne!(
            c_before.local_semantic_fingerprint,
            c_after.local_semantic_fingerprint
        );
        assert_ne!(
            c_before.effective_semantic_fingerprint,
            c_after.effective_semantic_fingerprint
        );
        let d_before = before
            .iter()
            .find(|object| object.semantic_id == "req.cap.d")
            .unwrap();
        let d_after = after
            .iter()
            .find(|object| object.semantic_id == "req.cap.d")
            .unwrap();
        assert_eq!(
            d_before.effective_semantic_fingerprint,
            d_after.effective_semantic_fingerprint
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relationship_only_change_is_semantic_and_ownership_output_is_bounded() {
        let (root, mut app) = fixture("relationship-only");
        let spec = root.join("openspec/specs/cap/spec.md");
        fs::write(
            &spec,
            specification(&[
                ("req.cap.a", &[], "A SHALL remain stable."),
                ("req.cap.b", &[], "B SHALL own the policy."),
            ]),
        )
        .unwrap();
        let before = objects(&app).unwrap();
        write_reviewed(&root, &before);
        write_roadmap(&root);
        fs::write(
            &spec,
            specification(&[
                (
                    "req.cap.a",
                    &[("requires", "req.cap.b")],
                    "A SHALL remain stable.",
                ),
                ("req.cap.b", &[], "B SHALL own the policy."),
            ]),
        )
        .unwrap();
        let after = objects(&app).unwrap();
        let a_before = before
            .iter()
            .find(|object| object.semantic_id == "req.cap.a")
            .unwrap();
        let a_after = after
            .iter()
            .find(|object| object.semantic_id == "req.cap.a")
            .unwrap();
        assert_eq!(
            a_before.local_semantic_fingerprint,
            a_after.local_semantic_fingerprint
        );
        assert_ne!(
            a_before.effective_semantic_fingerprint,
            a_after.effective_semantic_fingerprint
        );
        fs::write(
            &spec,
            specification(&[
                ("req.cap.a", &[], "A SHALL remain stable."),
                ("req.cap.b", &[], "B SHALL own the policy."),
            ]),
        )
        .unwrap();
        let reverted = objects(&app).unwrap();
        let a_reverted = reverted
            .iter()
            .find(|object| object.semantic_id == "req.cap.a")
            .unwrap();
        assert_eq!(
            a_before.local_semantic_fingerprint,
            a_reverted.local_semantic_fingerprint
        );
        assert_eq!(
            a_before.effective_semantic_fingerprint,
            a_reverted.effective_semantic_fingerprint
        );

        fs::write(
            &spec,
            specification(&[
                (
                    "req.cap.a",
                    &[("requires", "req.cap.b")],
                    "A SHALL remain stable.",
                ),
                ("req.cap.b", &[], "B SHALL own the policy."),
            ]),
        )
        .unwrap();
        let error = readiness(&app).unwrap_err();
        assert_eq!(
            error.details.unwrap()["diagnostics"][0]["gate"],
            "semantic-prerequisite-changed"
        );

        write_reviewed(&root, &after);
        for index in 0..3 {
            fs::write(
                root.join(format!("docs/reference-{index}.md")),
                "Explanation for req.cap.a.\n",
            )
            .unwrap();
        }
        app.bounds.max_units = 2;
        let packet = ownership(&app, "req.cap.a".to_owned()).unwrap();
        assert_eq!(packet["schema"], OWNERSHIP_SCHEMA);
        assert!(packet.get("reading_order").is_none());
        assert_eq!(
            packet["references"]["documentation"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(packet["omitted"]["req.cap.a"]["documentation"], 1);
        assert!(packet.get("verdict").is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relationship_target_kind_and_order_change_only_effective_fingerprints() {
        let (root, app) = fixture("relationship-shape");
        let spec = root.join("openspec/specs/cap/spec.md");
        let write = |relationships: &[(&str, &str)]| {
            fs::write(
                &spec,
                specification(&[
                    ("req.cap.a", relationships, "A SHALL remain stable."),
                    ("req.cap.b", &[], "B SHALL own one policy."),
                    ("req.cap.c", &[], "C SHALL own another policy."),
                ]),
            )
            .unwrap();
            objects(&app)
                .unwrap()
                .into_iter()
                .find(|object| object.semantic_id == "req.cap.a")
                .unwrap()
        };

        let requires_b = write(&[("requires", "req.cap.b")]);
        let requires_c = write(&[("requires", "req.cap.c")]);
        let refines_c = write(&[("refines", "req.cap.c")]);
        for changed in [&requires_c, &refines_c] {
            assert_eq!(
                requires_b.local_semantic_fingerprint,
                changed.local_semantic_fingerprint
            );
            assert_ne!(
                requires_b.effective_semantic_fingerprint,
                changed.effective_semantic_fingerprint
            );
        }

        let first_order = write(&[("requires", "req.cap.b"), ("refines", "req.cap.c")]);
        let second_order = write(&[("refines", "req.cap.c"), ("requires", "req.cap.b")]);
        assert_eq!(
            first_order.local_semantic_fingerprint,
            second_order.local_semantic_fingerprint
        );
        assert_eq!(
            first_order.effective_semantic_fingerprint,
            second_order.effective_semantic_fingerprint
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn formatting_and_review_metadata_leave_semantic_fingerprints_stable() {
        let (root, app) = fixture("non-semantic-fingerprint");
        let spec = root.join("openspec/specs/cap/spec.md");
        fs::write(
            &spec,
            requirement("The system SHALL preserve this\nsemantic contract."),
        )
        .unwrap();
        let before = objects(&app).unwrap();
        write_reviewed(&root, &before);

        fs::write(
            &spec,
            requirement("The system SHALL preserve this semantic contract."),
        )
        .unwrap();
        let mut state: ReviewedState = read_toml(&root, REVIEWED_PATH).unwrap();
        state
            .outcomes
            .insert("req.cap.one".to_owned(), "reviewed".to_owned());
        state.reasons.insert(
            "req.cap.one".to_owned(),
            "review metadata changed".to_owned(),
        );
        fs::write(root.join(REVIEWED_PATH), toml::to_string(&state).unwrap()).unwrap();

        let after = objects(&app).unwrap();
        assert_eq!(
            before[0].local_semantic_fingerprint,
            after[0].local_semantic_fingerprint
        );
        assert_eq!(
            before[0].effective_semantic_fingerprint,
            after[0].effective_semantic_fingerprint
        );
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
        fs::create_dir_all(root.join("docs/architecture")).unwrap();
        fs::create_dir_all(root.join("openspec/changes/archive")).unwrap();
        fs::write(
            root.join("docs/architecture/diskweave-architecture-roadmap-v0.8.md"),
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
            root.join("docs/architecture/diskweave-architecture-roadmap-v0.8.md"),
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
