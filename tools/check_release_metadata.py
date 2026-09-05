#!/usr/bin/env python3
"""Check Chirograph release metadata and canonical benchmark layout."""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPECTED_LICENSE = "Apache-2.0"


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


def check_benchmark_layout() -> None:
    legacy_root = ROOT / "benchmarks"
    if legacy_root.exists():
        fail("legacy benchmarks/ directory must not exist; use benchmark/")

    benchmark_root = ROOT / "benchmark"
    if not benchmark_root.is_dir():
        fail("benchmark directory is missing")
    for required in ("README.md", "baseline.json"):
        if not (benchmark_root / required).is_file():
            fail(f"benchmark/{required} is missing")

    specimen_paths = sorted(benchmark_root.glob("*/*/*/specimen.yaml"))
    if not specimen_paths:
        fail("benchmark corpus contains no specimen.yaml cases")
    for specimen in specimen_paths:
        case_dir = specimen.parent
        for required in ("golden.yaml", "fixture"):
            if not (case_dir / required).exists():
                fail(f"{case_dir.relative_to(ROOT)} is missing {required}")


def main() -> int:
    check_license_file()
    check_cargo_metadata()
    check_benchmark_layout()
    print("release-metadata: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())