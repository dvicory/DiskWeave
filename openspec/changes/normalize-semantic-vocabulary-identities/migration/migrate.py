#!/usr/bin/env python3
"""One-time vocabulary migration helpers for normalize-semantic-vocabulary-identities.

This script is intentionally narrow:
- scenario bodies and requirement semantics are owned by OpenSpec deltas;
- this script renames approved scenario headings that OpenSpec 1.8 cannot rename losslessly;
- this script rekeys reviewed outcomes/reasons for approved req.* renames;
- this script applies exact reviewed canonical explanatory-text replacements that OpenSpec deltas cannot represent;
- existing DiskWeave tooling remains responsible for semantic fingerprints, graph validation, and readiness.

Run against a disposable candidate tree before Gate #2, then against the real cutover tree only after Gate #2.
"""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import re
import sys
import tempfile
import tomllib

SCENARIO_RE = re.compile(r"^#### Scenario: (.+)$", re.MULTILINE)
REQ_RE = re.compile(r"^### Requirement: (.+)$", re.MULTILINE)
REQ_ID_RE = re.compile(r"<!-- dwv:req ([^ ]+) -->")


def fail(message: str) -> "NoReturn":
    raise SystemExit(message)


def normalized_body(body: str) -> str:
    lines = [line.rstrip() for line in body.replace("\r\n", "\n").split("\n")]
    while lines and lines[0] == "":
        lines.pop(0)
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines) + "\n"


def body_digest(body: str) -> str:
    return hashlib.sha256(normalized_body(body).encode("utf-8")).hexdigest()


def requirement_blocks(text: str):
    matches = list(REQ_RE.finditer(text))
    for index, match in enumerate(matches):
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        block = text[match.start():end]
        id_match = REQ_ID_RE.search(block)
        if id_match:
            yield id_match.group(1), block, match.start(), end


def scenario_blocks(requirement_block: str):
    matches = list(SCENARIO_RE.finditer(requirement_block))
    for index, match in enumerate(matches):
        end = matches[index + 1].start() if index + 1 < len(matches) else len(requirement_block)
        block = requirement_block[match.start():end]
        body = block.split("\n", 1)[1] if "\n" in block else ""
        yield index + 1, match.group(1), body, match.start(), end


def load_toml(path: Path):
    with path.open("rb") as handle:
        return tomllib.load(handle)


def load_scenario_map(change_dir: Path):
    data = load_toml(change_dir / "migration" / "scenario-renames.toml")
    return data.get("scenario", [])


def load_requirement_map(change_dir: Path):
    data = load_toml(change_dir / "migration" / "semantic-id-renames.toml")
    return data.get("requirement", [])


def load_text_map(change_dir: Path):
    path = change_dir / "migration" / "canonical-text-renames.toml"
    if not path.is_file():
        return []
    return load_toml(path).get("text", [])


def prepare_scenario_updates(root: Path, change_dir: Path):
    prepared: dict[Path, str] = {}
    for entry in load_scenario_map(change_dir):
        path = root / "openspec" / "specs" / entry["capability"] / "spec.md"
        if not path.is_file():
            fail(f"missing canonical spec: {path}")
        original = prepared.get(path, path.read_text())
        req_matches = [
            (block, start, end)
            for req_id, block, start, end in requirement_blocks(original)
            if req_id == entry["requirement_id"]
        ]
        if len(req_matches) != 1:
            fail(
                f"{path}: expected one requirement {entry['requirement_id']}, "
                f"found {len(req_matches)}"
            )
        req_block, req_start, req_end = req_matches[0]
        scenarios = list(scenario_blocks(req_block))
        occurrence = int(entry["occurrence"])
        if occurrence < 1 or occurrence > len(scenarios):
            fail(
                f"{path}: {entry['requirement_id']} scenario occurrence "
                f"{occurrence} is out of range"
            )
        ordinal, heading, body, scenario_start, _ = scenarios[occurrence - 1]
        if ordinal != occurrence:
            fail(f"{path}: internal scenario ordinal mismatch")
        expected_old = entry["old"]
        expected_new = entry["new"]
        if heading == expected_new:
            fail(
                f"{path}: scenario already renamed to {expected_new!r}; "
                "refusing a partially applied migration"
            )
        if heading != expected_old:
            fail(
                f"{path}: expected scenario heading {expected_old!r}, found {heading!r}"
            )
        digest = body_digest(body)
        if digest != entry["body_sha256"]:
            fail(
                f"{path}: body digest mismatch for {expected_old!r}: "
                f"expected {entry['body_sha256']}, found {digest}"
            )
        absolute_heading_start = req_start + scenario_start
        old_line = f"#### Scenario: {expected_old}"
        new_line = f"#### Scenario: {expected_new}"
        if original[absolute_heading_start:absolute_heading_start + len(old_line)] != old_line:
            fail(f"{path}: heading location changed unexpectedly")
        updated = (
            original[:absolute_heading_start]
            + new_line
            + original[absolute_heading_start + len(old_line):]
        )
        prepared[path] = updated
    return prepared



def prepare_text_updates(root: Path, change_dir: Path, prepared: dict[Path, str]):
    for entry in load_text_map(change_dir):
        path = root / entry["path"]
        if not path.is_file():
            fail(f"missing canonical text target: {path}")
        content = prepared.get(path, path.read_text())
        old = entry["old"]
        new = entry["new"]
        old_count = content.count(old)
        new_count = content.count(new)
        if new_count:
            fail(f"{path}: reviewed replacement is already present; refusing partial migration")
        if old_count != 1:
            fail(f"{path}: expected exact reviewed text once, found {old_count}")
        prepared[path] = content.replace(old, new, 1)
    return prepared

def atomic_write(path: Path, content: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_name, path)
    finally:
        if os.path.exists(temp_name):
            os.unlink(temp_name)


def rekey_reviewed(root: Path, change_dir: Path, apply: bool):
    path = root / "docs" / "reviewed-requirements.toml"
    if not path.is_file():
        fail(f"missing reviewed state: {path}")
    data = load_toml(path)
    mapping = {entry["old"]: entry["new"] for entry in load_requirement_map(change_dir)}
    sections = ["local_fingerprints", "effective_fingerprints", "outcomes", "reasons"]
    for section in sections:
        data.setdefault(section, {})

    # Fail before mutation on partial/ambiguous state. Some current requirements
    # may have fingerprints without an explicit reviewed outcome/reason; preserve
    # only decisions that actually exist rather than inventing one.
    for old, new in mapping.items():
        for section in ["outcomes", "reasons"]:
            table = data[section]
            if old in table and new in table:
                fail(
                    f"{path}: both old and new {section} entries exist for "
                    f"{old} -> {new}"
                )
        for section in ["local_fingerprints", "effective_fingerprints"]:
            table = data[section]
            if old in table and new in table:
                fail(
                    f"{path}: both old and new {section} entries exist for "
                    f"{old} -> {new}"
                )

    if not apply:
        return

    # Preserve decisions/reasons that exist; fingerprints are intentionally
    # removed and must be refreshed by existing knowledge tooling after the new
    # graph is current.
    for old, new in mapping.items():
        if old in data["outcomes"]:
            data["outcomes"][new] = data["outcomes"].pop(old)
        if old in data["reasons"]:
            data["reasons"][new] = data["reasons"].pop(old)
        data["local_fingerprints"].pop(old, None)
        data["effective_fingerprints"].pop(old, None)

    # Minimal deterministic TOML writer for this fixed schema.
    def toml_quote(value: str) -> str:
        escaped = value.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
        return f'"{escaped}"'

    lines = [f"schema = {toml_quote(data['schema'])}", ""]
    for section in sections:
        lines.append(f"[{section}]")
        for key in sorted(data[section]):
            lines.append(f"{toml_quote(key)} = {toml_quote(str(data[section][key]))}")
        lines.append("")
    atomic_write(path, "\n".join(lines))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--change-dir",
        type=Path,
        default=Path("openspec/changes/normalize-semantic-vocabulary-identities"),
    )
    parser.add_argument(
        "mode",
        choices=["check", "apply"],
        help="check validates without writing; apply performs the one-time mechanical migration",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    change_dir = args.change_dir
    if not change_dir.is_absolute():
        change_dir = root / change_dir
    if not change_dir.is_dir():
        fail(f"missing change directory: {change_dir}")

    prepared = prepare_scenario_updates(root, change_dir)
    prepared = prepare_text_updates(root, change_dir, prepared)
    rekey_reviewed(root, change_dir, apply=False)

    if args.mode == "check":
        print(
            f"validated {len(prepared)} canonical spec file(s), "
            f"{len(load_scenario_map(change_dir))} scenario rename(s), "
            f"{len(load_text_map(change_dir))} exact canonical text rename(s), and "
            f"{len(load_requirement_map(change_dir))} reviewed-state rekey(s)"
        )
        return 0

    # All validation happened before the first write.
    for path, content in prepared.items():
        atomic_write(path, content)
    rekey_reviewed(root, change_dir, apply=True)
    print(
        f"applied {len(load_scenario_map(change_dir))} scenario rename(s), "
        f"{len(load_text_map(change_dir))} exact canonical text rename(s), and "
        f"rekeyed reviewed outcomes/reasons for {len(load_requirement_map(change_dir))} requirement(s)"
    )
    print(
        "reviewed fingerprints were intentionally removed for renamed requirements; "
        "refresh them with existing knowledge tooling after graph/model equivalence passes"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
