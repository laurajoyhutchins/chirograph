#!/usr/bin/env python3
"""Check Chirograph release metadata without third-party Python packages."""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPECTED_LICENSE = "Apache-2.0"
PROVENANCE_SCHEMA = "chirograph-benchmark-provenance-v1"
SHA256 = re.compile(r"^[0-9a-f]{64}$")
ALLOWED_MATERIALIZATION = {"bundled", "reference-only", "generated"}


def fail(message: str) -> None:
    print(f"release-metadata: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def check_license_file() -> None:
    path = ROOT / "LICENSE"
    if not path.is_file():
        fail("LICENSE is missing")
    text = path.read_text(encoding="utf-8")
    if "Apache License" not in text or "Version 2.0, January 2004" not in text:
        fail("LICENSE does not look like the Apache License 2.0 text")


def check_cargo_metadata() -> None:
    root = load_toml(ROOT / "Cargo.toml")
    package = root.get("workspace", {}).get("package", {})
    if package.get("license") != EXPECTED_LICENSE:
        fail("workspace.package.license must be Apache-2.0")

    for manifest in sorted((ROOT / "crates").glob("*/Cargo.toml")):
        data = load_toml(manifest)
        package = data.get("package")
        if not isinstance(package, dict):
            continue
        license_value = package.get("license")
        inherited = isinstance(license_value, dict) and license_value.get("workspace") is True
        if license_value != EXPECTED_LICENSE and not inherited:
            fail(f"{manifest.relative_to(ROOT)} must declare license.workspace = true or Apache-2.0")


def require_text(value: object, field: str, path: Path) -> str:
    if not isinstance(value, str) or not value.strip():
        fail(f"{path.relative_to(ROOT)}: {field} must be a non-empty string")
    return value.strip()


def check_provenance(path: Path) -> None:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"{path.relative_to(ROOT)}: invalid JSON: {error}")

    if not isinstance(data, dict):
        fail(f"{path.relative_to(ROOT)}: top level must be an object")

    required = {"schema", "specimen", "case", "phenomenon", "materialization", "source", "files"}
    missing = sorted(required - data.keys())
    unknown = sorted(data.keys() - required)
    if missing:
        fail(f"{path.relative_to(ROOT)}: missing fields: {', '.join(missing)}")
    if unknown:
        fail(f"{path.relative_to(ROOT)}: unknown fields: {', '.join(unknown)}")
    if data["schema"] != PROVENANCE_SCHEMA:
        fail(f"{path.relative_to(ROOT)}: unsupported schema {data['schema']!r}")

    require_text(data["specimen"], "specimen", path)
    require_text(data["case"], "case", path)
    require_text(data["phenomenon"], "phenomenon", path)
    if data["materialization"] not in ALLOWED_MATERIALIZATION:
        fail(f"{path.relative_to(ROOT)}: invalid materialization {data['materialization']!r}")

    source = data["source"]
    if not isinstance(source, dict):
        fail(f"{path.relative_to(ROOT)}: source must be an object")
    source_required = {"origin", "revision", "license"}
    if set(source) != source_required:
        fail(f"{path.relative_to(ROOT)}: source fields must be origin, revision, license")
    for field in sorted(source_required):
        require_text(source[field], f"source.{field}", path)

    files = data["files"]
    if not isinstance(files, list):
        fail(f"{path.relative_to(ROOT)}: files must be an array")
    for index, entry in enumerate(files):
        if not isinstance(entry, dict):
            fail(f"{path.relative_to(ROOT)}: files[{index}] must be an object")
        allowed = {"source_path", "fixture_path", "sha256", "license"}
        if not {"source_path", "sha256"}.issubset(entry):
            fail(f"{path.relative_to(ROOT)}: files[{index}] requires source_path and sha256")
        if set(entry) - allowed:
            fail(f"{path.relative_to(ROOT)}: files[{index}] has unsupported fields")
        require_text(entry["source_path"], f"files[{index}].source_path", path)
        digest = require_text(entry["sha256"], f"files[{index}].sha256", path)
        if not SHA256.fullmatch(digest):
            fail(f"{path.relative_to(ROOT)}: files[{index}].sha256 must be lowercase SHA-256")
        if "fixture_path" in entry and entry["fixture_path"] is not None:
            require_text(entry["fixture_path"], f"files[{index}].fixture_path", path)
        if "license" in entry:
            require_text(entry["license"], f"files[{index}].license", path)


def check_benchmark_cases() -> None:
    benchmark_root = ROOT / "benchmarks"
    if not benchmark_root.is_dir():
        fail("benchmarks directory is missing")

    provenance_paths = sorted(benchmark_root.glob("**/provenance.json"))
    for path in provenance_paths:
        check_provenance(path)

    case_markers = set()
    for pattern in ("**/expected.json", "**/fixture"):
        for marker in benchmark_root.glob(pattern):
            case_markers.add(marker.parent)
    for case_dir in sorted(case_markers):
        if not (case_dir / "provenance.json").is_file():
            fail(f"{case_dir.relative_to(ROOT)} contains benchmark case data without provenance.json")


def main() -> int:
    check_license_file()
    check_cargo_metadata()
    check_benchmark_cases()
    print("release-metadata: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
