#!/usr/bin/env python3
"""Generate and build the repository-local offline Sphinx projection."""

from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path
 
ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "docs" / "sphinx"
STATE = ROOT / "target" / "dwv-docs" / "sphinx"
SOURCE = STATE / "source"
RUST_SOURCE = STATE / "rust-source"
HTML = STATE / "html"
OBJECTS = ROOT / "target" / "dwv-docs" / "knowledge" / "objects.json"
MANIFEST = ROOT / "verification" / "manifest.toml"
SCHEMA = "dwv.knowledge.objects.v4"
VERIFICATION_SCHEMA = "dwv.verification.manifest.v2"
MAX_FIXTURE_BYTES = 1_000_000
RUST_MARKER_RE = re.compile(r"(?m)^(?P<prefix>\s*///\s*)dwv:req\s+(?P<id>req\.[a-z0-9.-]+)\s*$")


def fail(message: str) -> None:
    raise SystemExit(f"dwv-sphinx: {message}")


def load_knowledge() -> tuple[
    list[dict[str, object]], dict[str, dict[str, list[str]]], list[str]
]:
    if not OBJECTS.is_file():
        fail(f"missing {OBJECTS}; run cargo xtask docs knowledge export first")
    try:
        payload = json.loads(OBJECTS.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {OBJECTS}: {error}")
    if not isinstance(payload, dict) or payload.get("schema") != SCHEMA:
        actual = payload.get("schema") if isinstance(payload, dict) else type(payload).__name__
        fail(f"expected schema {SCHEMA}, got {actual!r}")
    objects = payload.get("objects")
    capabilities = payload.get("capabilities")
    reading_order = payload.get("reading_order")
    if not isinstance(objects, list):
        fail("knowledge export has no objects array")
    if not isinstance(capabilities, dict):
        fail("knowledge export has no capability aggregation")
    if not isinstance(reading_order, list) or not all(
        isinstance(item, str) for item in reading_order
    ):
        fail("knowledge export has no owner-before-dependent reading order")
    required = {
        "semantic_id",
        "sphinx_id",
        "title",
        "capability",
        "source_path",
        "heading_path",
        "local_semantic_fingerprint",
        "effective_semantic_fingerprint",
        "requires",
        "refines",
        "required_by",
        "refined_by",
        "constrained_by",
        "body",
    }
    result = []
    seen: set[str] = set()
    for item in objects:
        if not isinstance(item, dict) or not required <= item.keys():
            fail("each knowledge object must contain the shared contract fields")
        semantic_id = item["semantic_id"]
        sphinx_id = item["sphinx_id"]
        if not isinstance(semantic_id, str) or semantic_id in seen:
            fail(f"duplicate or invalid semantic_id: {semantic_id!r}")
        if not isinstance(sphinx_id, str):
            fail(f"invalid sphinx_id for {semantic_id}")
        if not all(
            isinstance(item.get(field), list)
            and all(isinstance(target, str) for target in item[field])
            for field in ("requires", "refines", "required_by", "refined_by", "constrained_by")
        ):
            fail(f"invalid relationships for {semantic_id}")
        seen.add(semantic_id)
        result.append(item)
    if set(reading_order) != seen or len(reading_order) != len(seen):
        fail("reading order must contain every current requirement exactly once")
    return (
        sorted(result, key=lambda item: str(item["semantic_id"])),
        capabilities,
        reading_order,
    )


def canonical_body(item: dict[str, object]) -> str:
    source = (ROOT / str(item["source_path"])).resolve()
    try:
        source.relative_to(ROOT)
    except ValueError:
        fail(f"source escapes repository: {item['source_path']}")
    try:
        text = source.read_text(encoding="utf-8")
    except OSError as error:
        fail(f"cannot read {source}: {error}")
    marker = f"<!-- dwv:req {item['semantic_id']} -->"
    if text.count(marker) != 1:
        fail(f"expected one canonical marker for {item['semantic_id']}")
    body = text.split(marker, 1)[1].lstrip("\r\n")
    next_requirement = body.find("\n### Requirement:")
    if next_requirement >= 0:
        body = body[:next_requirement]
    return body.strip()


def write_requirements(objects: list[dict[str, object]]) -> None:
    mapping = {str(item["semantic_id"]): str(item["sphinx_id"]) for item in objects}
    lines = [
        "# Current requirements",
        "",
        "These are non-authoritative Sphinx-Needs projections of the exported knowledge objects.",
        "",
    ]
    for item in objects:
        lines.extend(
            [
                f"```{{req}} {item['title']}",
                f":id: {item['sphinx_id']}",
                f":semantic_id: {item['semantic_id']}",
                f":capability: {item['capability']}",
                f":source_path: {item['source_path']}",
                f":heading_path: {item['heading_path']}",
                f":local_fingerprint: {item['local_semantic_fingerprint']}",
                f":effective_fingerprint: {item['effective_semantic_fingerprint']}",
            ]
        )
        for kind in ("requires", "refines"):
            targets = [mapping[str(target)] for target in item[kind]]
            if targets:
                lines.append(f":{kind}: {', '.join(targets)}")
        lines.append("")
        body = canonical_body(item)
        if body:
            for line in body.splitlines():
                lines.append(f"**{line[5:]}**" if line.startswith("#### ") else line)
        lines.extend(["```", "", ""])
    (SOURCE / "requirements.md").write_text("\n".join(lines), encoding="utf-8")


def write_ownership(
    objects: list[dict[str, object]],
    capabilities: dict[str, dict[str, list[str]]],
    reading_order: list[str],
) -> None:
    lines = [
        "# Semantic ownership",
        "",
        "This view is derived from current canonical forward markers. It reports graph facts, not a semantic-coherence verdict.",
        "",
        "## Owner-before-dependent reading order",
        "",
    ]
    lines.extend(f"{index}. `{semantic_id}`" for index, semantic_id in enumerate(reading_order, 1))
    lines.extend(["", "## Capability aggregation", ""])
    for capability, relationships in sorted(capabilities.items()):
        lines.extend([f"### `{capability}`", ""])
        for field in ("requirements", "requires", "refines", "required_by", "refined_by"):
            write_values(lines, field.replace("_", " "), relationships.get(field, []))
        lines.append("")
    lines.extend(["## Requirement relationships", ""])
    for item in objects:
        lines.extend(
            [
                f"### `{item['semantic_id']}`",
                "",
                f"- local fingerprint: `{item['local_semantic_fingerprint']}`",
                f"- effective fingerprint: `{item['effective_semantic_fingerprint']}`",
            ]
        )
        for field in ("requires", "refines", "required_by", "refined_by"):
            write_values(lines, field.replace("_", " "), item[field])
        lines.append("")
    (SOURCE / "ownership-generated.md").write_text("\n".join(lines), encoding="utf-8")


def load_manifest() -> dict[str, list[dict[str, object]]]:
    if not MANIFEST.is_file():
        fail(f"missing {MANIFEST}")
    try:
        payload = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot read {MANIFEST}: {error}")
    if payload.get("schema") != VERIFICATION_SCHEMA:
        fail(f"expected schema {VERIFICATION_SCHEMA}, got {payload.get('schema')!r}")
    result: dict[str, list[dict[str, object]]] = {}
    for key in ("evidence", "scenarios"):
        records = payload.get(key, [])
        if not isinstance(records, list) or not all(isinstance(record, dict) for record in records):
            fail(f"manifest {key} must be an array of tables")
        result[key] = sorted(records, key=lambda record: str(record.get("id", "")))
    return result

def bounded_path(relative: object, *, prefix: str | None = None) -> Path:
    if not isinstance(relative, str) or not relative or Path(relative).is_absolute():
        fail(f"invalid repository path: {relative!r}")
    path = (ROOT / relative).resolve()
    try:
        path.relative_to(ROOT)
    except ValueError:
        fail(f"path escapes repository: {relative!r}")
    if prefix and not relative.startswith(prefix):
        fail(f"path is outside {prefix}: {relative!r}")
    return path


def fixture_facts(relative: object) -> dict[str, object]:
    path = bounded_path(relative, prefix="verification/corpus/")
    try:
        raw = path.read_bytes()
    except OSError as error:
        fail(f"cannot read fixture {relative}: {error}")
    if len(raw) > MAX_FIXTURE_BYTES:
        fail(f"fixture exceeds {MAX_FIXTURE_BYTES} bytes: {relative}")
    facts: dict[str, object] = {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}
    if path.name.endswith(".trace.json"):
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError as error:
            fail(f"invalid JSON fixture {relative}: {error}")
        if not isinstance(payload, dict):
            fail(f"JSON fixture must contain an object: {relative}")
        facts["format"] = "json"
        if isinstance(payload.get("schema"), (str, int)):
            facts["schema"] = payload["schema"]
        fixture = payload.get("fixture")
        if isinstance(fixture, dict):
            facts["fixture"] = [
                f"{key}={fixture[key]}"
                for key in sorted(fixture)
                if isinstance(fixture[key], (str, int, float, bool))
            ]
        events = payload.get("events")
        if not isinstance(events, list):
            fail(f"JSON fixture has no events array: {relative}")
        event_kinds: list[str] = []
        for event in events:
            if not isinstance(event, dict):
                continue
            kind = event.get("kind")
            if isinstance(kind, dict) and kind:
                name = next(iter(kind))
                sequence = event.get("sequence")
                event_kinds.append(f"{sequence}:{name}" if isinstance(sequence, int) else str(name))
        facts["event_count"] = len(events)
        facts["event_kinds"] = event_kinds
    else:
        lines = [line.strip() for line in raw.decode("utf-8").splitlines() if line.strip()]
        if not lines:
            fail(f"schedule fixture is empty: {relative}")
        facts["format"] = "schedule"
        facts["header"] = lines[0]
        operations = [line.split("|", 1)[0] for line in lines[1:] if "|" in line]
        facts["operation_count"] = len(operations)
        facts["operation_codes"] = operations
    return facts


def list_field(record: dict[str, object], name: str) -> list[str]:
    value = record.get(name, [])
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        fail(f"manifest field {name!r} must contain strings in {record.get('id')!r}")
    return [str(item) for item in value]


def write_values(lines: list[str], label: str, values: list[str]) -> None:
    lines.append(f"- {label}:")
    lines.extend((f"  - `{value}`" for value in values) or ["  - (none listed)"])


def write_scenarios(manifest: dict[str, list[dict[str, object]]]) -> None:
    evidence_by_id = {str(item["id"]): item for item in manifest["evidence"] if "id" in item}
    lines = [
        "# Scenarios",
        "",
        "This non-authoritative view is generated from `verification/manifest.toml` and the "
        "referenced fixture bytes. Fixture facts describe structure only; this page does not "
        "invent execution results.",
        "",
    ]
    for scenario in manifest["scenarios"]:
        scenario_id = str(scenario.get("id", ""))
        fixture = scenario.get("fixture")
        if not scenario_id or not fixture:
            fail("every scenario requires id and fixture")
        facts = fixture_facts(fixture)
        lines.extend([f"## {scenario.get('title', scenario_id)}", "", f"- `id`: `{scenario_id}`"])
        lines.append(f"- `fixture`: `{fixture}`")
        lines.append(f"- `runner`: `{scenario['runner']}`" if "runner" in scenario else "- `runner`: (not specified)")
        if "fixture_size" in scenario:
            lines.append(f"- `manifest.fixture_size`: `{scenario['fixture_size']}`")
        lines.extend([f"- `fixture.bytes`: `{facts['bytes']}`", f"- `fixture.sha256`: `{facts['sha256']}`"])
        for key in sorted(facts):
            if key in {"bytes", "sha256"}:
                continue
            value = facts[key]
            if isinstance(value, list):
                rendered = ", ".join(f"`{item}`" for item in value) or "(none)"
            else:
                rendered = f"`{value}`"
            lines.append(f"- `fixture.{key}`: {rendered}")
        write_values(lines, "requirements", sorted(list_field(scenario, "requirements")))
        write_values(lines, "required events", list_field(scenario, "required_events"))
        evidence_ids = sorted(list_field(scenario, "evidence"))
        write_values(lines, "evidence IDs", evidence_ids)
        lines.extend(["", "### Manifest claims", ""])
        write_values(lines, "claims", list_field(scenario, "claims"))
        lines.extend(["", "### Evidence boundaries", ""])
        for evidence_id in evidence_ids:
            evidence = evidence_by_id.get(evidence_id)
            if evidence is None:
                fail(f"scenario {scenario_id} references unknown evidence {evidence_id}")
            lines.extend(
                [
                    f"#### `{evidence_id}`",
                    "",
                    f"- claim: {evidence.get('claim', '(not listed)')}",
                    f"- evidence tier: `{evidence.get('tier', '(not listed)')}`",
                ]
            )
            write_values(lines, "fault models", sorted(list_field(evidence, "fault_model")))
            write_values(lines, "scopes", sorted(list_field(evidence, "scope")))
            write_values(lines, "non-claims", list_field(evidence, "non_claims"))
            lines.append("")
        if not evidence_ids:
            lines.append("- evidence tier: (no evidence record listed; no assurance tier is inferred)")
            lines.append("")
        write_values(lines, "scenario non-claims", list_field(scenario, "non_claims"))
        lines.append("")
    (SOURCE / "scenarios.md").write_text("\n".join(lines), encoding="utf-8")


def write_assurance(objects: list[dict[str, object]], manifest: dict[str, list[dict[str, object]]]) -> None:
    evidence_by_requirement: dict[str, list[dict[str, object]]] = {}
    for evidence in manifest["evidence"]:
        for requirement in list_field(evidence, "requirements"):
            evidence_by_requirement.setdefault(requirement, []).append(evidence)
    lines = [
        "# Assurance mapping",
        "",
        "This non-authoritative projection maps current requirement semantic IDs to evidence "
        "records in `verification/manifest.toml`. Tiers, fault models, scopes, and non-claims "
        "are copied from those records; missing evidence is not treated as a guarantee.",
        "",
    ]
    for requirement in objects:
        semantic_id = str(requirement["semantic_id"])
        records = sorted(evidence_by_requirement.get(semantic_id, []), key=lambda item: str(item["id"]))
        labels = ", ".join(f"`{item['id']}`" for item in records) or "(none listed)"
        tiers = ", ".join(f"`{item.get('tier', '(not listed)')}`" for item in records) or "(none listed)"
        lines.extend(
            [
                f"## `{semantic_id}`",
                "",
                f"- capability: `{requirement['capability']}`",
                f"- source_path: `{requirement['source_path']}`",
                f"- heading_path: `{' / '.join(str(part) for part in requirement['heading_path'])}`",
                f"- local fingerprint: `{requirement['local_semantic_fingerprint']}`",
                f"- effective fingerprint: `{requirement['effective_semantic_fingerprint']}`",
                f"- evidence IDs: {labels}",
                f"- evidence tiers: {tiers}",
            ]
        )
        write_values(lines, "fault models", sorted({fault for item in records for fault in list_field(item, "fault_model")}))
        write_values(lines, "scopes", sorted({scope for item in records for scope in list_field(item, "scope")}))
        lines.extend(["", "### Evidence records", ""])
        if not records:
            lines.append("- No manifest evidence record is linked; no assurance claim is made.")
            lines.append("")
        for evidence in records:
            lines.extend(
                [
                    f"#### `{evidence['id']}`",
                    "",
                    f"- claim: {evidence.get('claim', '(not listed)')}",
                    f"- evidence tier: `{evidence.get('tier', '(not listed)')}`",
                ]
            )
            write_values(lines, "fault models", sorted(list_field(evidence, "fault_model")))
            write_values(lines, "scopes", sorted(list_field(evidence, "scope")))
            write_values(lines, "non-claims", list_field(evidence, "non_claims"))
            lines.append("")
    (SOURCE / "assurance-generated.md").write_text("\n".join(lines), encoding="utf-8")
 
def display_package_id(value: object) -> str:
    text = str(value)
    return text.replace(f"path+{ROOT.as_uri()}", "path+file://./")


def relative_path(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def cargo_metadata(metadata_file: Path | None) -> dict[str, object]:
    if metadata_file is not None:
        try:
            payload = json.loads(metadata_file.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            fail(f"cannot read Cargo metadata {metadata_file}: {error}")
    else:
        try:
            result = subprocess.run(
                ["cargo", "metadata", "--format-version", "1", "--no-deps"],
                cwd=ROOT, check=True, capture_output=True, text=True
            )
        except (OSError, subprocess.CalledProcessError) as error:
            fail(f"cannot capture Cargo metadata: {getattr(error, 'stderr', str(error)).strip()}")
        try:
            payload = json.loads(result.stdout)
        except json.JSONDecodeError as error:
            fail(f"Cargo metadata is invalid JSON: {error}")
    if not isinstance(payload, dict) or not isinstance(payload.get("packages"), list):
        fail("Cargo metadata must contain a packages array")
    return payload




def write_contributors(metadata_file: Path | None) -> None:
    metadata = cargo_metadata(metadata_file)
    packages = [item for item in metadata["packages"] if isinstance(item, dict)]
    packages.sort(key=lambda item: (str(item.get("name", "")), str(item.get("id", ""))))
    lines = [
        "# Contributors and package boundaries",
        "",
        "This non-authoritative view is generated from `cargo metadata --format-version 1 "
        "--no-deps`. Package structure is not a semantic requirement registry; semantic "
        "ownership and source association remain on the Rust API and source-trace pages.",
        "",
        "- evidence tier: `repository Cargo metadata`",
        "- non-claims: this view does not measure runtime behavior, semantic ownership, "
        "authorship, execution results, platform support, or hardware durability.",
        "",
    ]
    for package in packages:
        name = str(package.get("name", "(unnamed)"))
        lines.extend([f"## `{name}`", "", f"- package ID: `{display_package_id(package.get('id', '(not listed)'))}`"])
        if isinstance(package.get("manifest_path"), str):
            lines.append(f"- manifest: `{relative_path(Path(str(package['manifest_path'])))}`")
        lines.extend(["", "### Targets", ""])
        targets = [target for target in package.get("targets", []) if isinstance(target, dict)]
        targets.sort(key=lambda item: str(item.get("name", "")))
        for target in targets:
            kinds = ", ".join(str(kind) for kind in target.get("kind", []))
            src = relative_path(Path(str(target["src_path"]))) if isinstance(target.get("src_path"), str) else "(not listed)"
            lines.append(f"- `{target.get('name', '(unnamed)')}` ({kinds or 'kind not listed'}): `{src}`")
        if not targets:
            lines.append("- (none listed)")
        lines.extend(["", "### Dependencies", ""])
        dependencies = [dependency for dependency in package.get("dependencies", []) if isinstance(dependency, dict)]
        dependencies.sort(key=lambda item: (str(item.get("name", "")), str(item.get("kind", ""))))
        for dependency in dependencies:
            detail = str(dependency.get("req", ""))
            if dependency.get("kind"):
                detail = f"{detail}; kind={dependency['kind']}"
            lines.append(f"- `{dependency.get('name', '(unnamed)')}`: `{detail}`")
        if not dependencies:
            lines.append("- (none listed)")
        lines.append("")
    (SOURCE / "contributors-generated.md").write_text("\n".join(lines), encoding="utf-8")


def write_codelinks_source(objects: list[dict[str, object]]) -> None:
    mapping = {str(item["semantic_id"]): str(item["sphinx_id"]) for item in objects}
    if RUST_SOURCE.exists():
        shutil.rmtree(RUST_SOURCE)
    marker_count = 0
    roots = [(ROOT / "crates", Path()), (ROOT / "xtask", Path("xtask"))]
    paths = sorted(
        (path, prefix / path.relative_to(root))
        for root, prefix in roots
        for path in root.rglob("*.rs")
    )
    for path, relative in paths:
        if path.is_symlink():
            fail(f"Rust source is a symlink: {relative_path(path)}")
        text = path.read_text(encoding="utf-8")

        def replace(match: re.Match[str]) -> str:
            nonlocal marker_count
            semantic_id = match.group("id")
            sphinx_id = mapping.get(semantic_id)
            if sphinx_id is None:
                fail(f"Rust marker has no canonical requirement: {semantic_id}")
            line = text.count("\n", 0, match.start()) + 1
            marker_id = "ST_" + hashlib.sha256(
                f"{relative}:{line}:{semantic_id}".encode()
            ).hexdigest()[:20].upper()
            marker_count += 1
            return (
                f"{match.group('prefix')}@dwv Rust marker {semantic_id}, "
                f"{marker_id}, srctrace, [{sphinx_id}]"
            )

        rendered = RUST_MARKER_RE.sub(replace, text)
        destination = RUST_SOURCE / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(rendered, encoding="utf-8")
    if marker_count == 0:
        fail("CodeLinks source projection found no dwv:req markers")


def write_rust_link() -> None:
    (SOURCE / "rust-api.md").write_text(
        "# Rust API\n\n"
        "The API pages are generated by Cargo/rustdoc.\n\n"
        "[Open rustdoc](../../../doc/dwv/index.html)\n",
        encoding="utf-8",
    )


def generate(metadata_file: Path | None = None) -> list[dict[str, object]]:
    objects, capabilities, reading_order = load_knowledge()
    manifest = load_manifest()
    if SOURCE.exists():
        shutil.rmtree(SOURCE)
    SOURCE.mkdir(parents=True)
    for path in CONFIG.rglob("*"):
        if path.is_file():
            destination = SOURCE / path.relative_to(CONFIG)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, destination)
    write_codelinks_source(objects)
    write_requirements(objects)
    write_ownership(objects, capabilities, reading_order)
    write_scenarios(manifest)
    write_assurance(objects, manifest)
    write_contributors(metadata_file)
    write_rust_link()
    return objects


def run(command: list[str]) -> None:
    try:
        subprocess.run(command, cwd=ROOT, check=True)
    except OSError as error:
        fail(f"cannot run {' '.join(command)}: {error}")
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.returncode) from error


def build(metadata_file: Path | None = None) -> None:
    objects = generate(metadata_file)
    run(["mise", "exec", "--", "cargo", "doc", "--workspace", "--no-deps", "--offline"])
    if not (ROOT / "target" / "doc" / "dwv" / "index.html").is_file():
        fail("cargo doc completed without target/doc/dwv/index.html")
    if HTML.exists():
        shutil.rmtree(HTML)
    run(["mise", "exec", "--", "sphinx-build", "-W", "--keep-going", "-b", "html", str(SOURCE), str(HTML)])
    print(json.dumps({"objects": len(objects), "source": str(SOURCE), "output": str(HTML)}, sort_keys=True))


def command_metadata(arguments: list[str]) -> Path | None:
    if not arguments:
        return None
    if len(arguments) == 1 and arguments[0] != "--metadata":
        return Path(arguments[0]).expanduser()
    if len(arguments) == 2 and arguments[0] == "--metadata":
        return Path(arguments[1]).expanduser()
    fail("usage: tools/dwv-sphinx.py [generate|build] [metadata.json|--metadata metadata.json]")


def main() -> None:
    command = sys.argv[1] if len(sys.argv) > 1 else "build"
    metadata_file = command_metadata(sys.argv[2:])
    if command == "generate":
        print(json.dumps({"objects": len(generate(metadata_file)), "source": str(SOURCE)}, sort_keys=True))
    elif command == "build":
        build(metadata_file)
    else:
        fail("usage: tools/dwv-sphinx.py [generate|build] [metadata.json|--metadata metadata.json]")


if __name__ == "__main__":
    main()
