#!/usr/bin/env python3
"""Assemble the Pydantic specimen from generic Python static and runtime evidence."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODELS = ROOT / "specimens" / "pydantic" / "models.py"
PROBE = ROOT / "specimens" / "pydantic" / "probe.py"
SHA1 = re.compile(r"^[0-9a-f]{40}$")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def canonical(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def run_json(command: list[str], label: str) -> dict[str, object]:
    result = subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"{label} failed with {result.returncode}: {result.stderr or result.stdout}"
        )
    try:
        document = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"{label} did not emit JSON: {error}") from error
    require(isinstance(document, dict), f"{label} must emit a JSON object")
    return document


def static_python_acquisition(repository_revision: str) -> dict[str, object]:
    relative_path = MODELS.relative_to(ROOT).as_posix()
    return run_json(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "chirograph-python-adapter",
            "--bin",
            "chirograph-python-observe",
            "--",
            "python.pydantic.models",
            repository_revision,
            relative_path,
        ],
        "generic Python acquisition",
    )


def runtime_observations(upstream_revision: str) -> dict[str, object]:
    return run_json(
        [
            sys.executable,
            str(PROBE),
            "--upstream-revision",
            upstream_revision,
        ],
        "generic Python runtime probe",
    )


def annotation_observation(acquisition: dict[str, object]) -> dict[str, object]:
    facts = acquisition.get("facts")
    observations = acquisition.get("observations")
    require(isinstance(facts, list), "Python acquisition facts must be an array")
    require(
        isinstance(observations, list),
        "Python acquisition observations must be an array",
    )
    require(
        len(facts) == len(observations),
        "Python acquisition facts and observations lost positional correspondence",
    )

    matches = []
    for fact, observation in zip(facts, observations, strict=True):
        if not isinstance(fact, dict) or not isinstance(observation, dict):
            continue
        if (
            fact.get("kind") == "annotated_assignment"
            and fact.get("name") == "value"
            and fact.get("annotation") == "str"
        ):
            matches.append(observation)
    require(
        len(matches) == 1,
        f"expected exactly one generic observation for ValidatorModel.value: str, found {len(matches)}",
    )
    return matches[0]


def build_evidence(
    repository_revision: str,
    upstream_revision: str,
    claimed_schema: Path | None,
) -> dict[str, object]:
    require(
        SHA1.fullmatch(repository_revision) is not None,
        "repository revision must be a full lowercase Git SHA",
    )
    require(
        SHA1.fullmatch(upstream_revision) is not None,
        "upstream revision must be a full lowercase Git SHA",
    )

    acquisition = static_python_acquisition(repository_revision)
    require(
        acquisition.get("schema") == "chirograph-python-acquisition-v1",
        "unexpected generic Python acquisition schema",
    )
    static_source = acquisition.get("source")
    require(isinstance(static_source, dict), "Python acquisition source is missing")
    static_observation = annotation_observation(acquisition)

    runtime = runtime_observations(upstream_revision)
    require(
        runtime.get("schema") == "python-runtime-observations-v1",
        "unexpected Python runtime observation schema",
    )
    runtime_sources = runtime.get("sources")
    runtime_observation_list = runtime.get("observations")
    require(isinstance(runtime_sources, list), "runtime sources must be an array")
    require(
        isinstance(runtime_observation_list, list),
        "runtime observations must be an array",
    )

    sources: list[dict[str, object]] = [dict(static_source)] + [
        dict(source) for source in runtime_sources if isinstance(source, dict)
    ]
    observations: list[dict[str, object]] = [dict(static_observation)] + [
        dict(observation)
        for observation in runtime_observation_list
        if isinstance(observation, dict)
    ]

    static_source_id = static_source.get("id")
    static_locator = static_observation.get("locator")
    static_observation_id = static_observation.get("id")
    require(isinstance(static_source_id, str), "static source id is missing")
    require(isinstance(static_locator, str), "static observation locator is missing")
    require(isinstance(static_observation_id, str), "static observation id is missing")

    contracts: list[dict[str, object]] = [
        {
            "id": "pydantic.validator.value",
            "name": "Pydantic before-validator value contract",
            "facets": ["structural", "executable", "semantic"],
        },
        {
            "id": "pydantic.computed.area",
            "name": "Pydantic computed-field area contract",
            "facets": ["structural", "semantic"],
        },
    ]
    representations: list[dict[str, object]] = [
        {
            "id": "pydantic.validator.annotation",
            "contract": "pydantic.validator.value",
            "source": static_source_id,
            "kind": "type_definition",
            "locator": static_locator,
            "facets": ["structural", "semantic"],
        },
        {
            "id": "pydantic.validator.runtime",
            "contract": "pydantic.validator.value",
            "source": "pydantic.runtime",
            "kind": "executable_surface",
            "locator": "ValidatorModel.model_validate",
            "facets": ["executable", "semantic"],
        },
        {
            "id": "pydantic.validator.validation-schema",
            "contract": "pydantic.validator.value",
            "source": "pydantic.runtime",
            "kind": "schema",
            "locator": "ValidatorModel.model_json_schema(mode=validation)",
            "facets": ["structural", "executable"],
        },
        {
            "id": "pydantic.validator.serialization-schema",
            "contract": "pydantic.validator.value",
            "source": "pydantic.runtime",
            "kind": "schema",
            "locator": "ValidatorModel.model_json_schema(mode=serialization)",
            "facets": ["structural"],
        },
        {
            "id": "pydantic.computed.validation-schema",
            "contract": "pydantic.computed.area",
            "source": "pydantic.runtime",
            "kind": "schema",
            "locator": "Rectangle.model_json_schema(mode=validation)",
            "facets": ["structural"],
        },
        {
            "id": "pydantic.computed.serialization-schema",
            "contract": "pydantic.computed.area",
            "source": "pydantic.runtime",
            "kind": "schema",
            "locator": "Rectangle.model_json_schema(mode=serialization)",
            "facets": ["structural"],
        },
        {
            "id": "pydantic.computed.runtime-output",
            "contract": "pydantic.computed.area",
            "source": "pydantic.runtime",
            "kind": "executable_surface",
            "locator": "Rectangle.model_dump",
            "facets": ["structural", "semantic"],
        },
    ]
    clauses: list[dict[str, object]] = [
        {
            "id": "pydantic.validator.value.accepted-input",
            "contract": "pydantic.validator.value",
            "facet": "executable",
            "kind": "requirement",
            "statement": "validation input for value may be an integer or string",
        },
        {
            "id": "pydantic.validator.value.normalized-state",
            "contract": "pydantic.validator.value",
            "facet": "semantic",
            "kind": "guarantee",
            "statement": "after validation, value is a string",
        },
        {
            "id": "pydantic.computed.area.validation-input",
            "contract": "pydantic.computed.area",
            "facet": "structural",
            "kind": "guarantee",
            "statement": "validation schema omits computed field area from input properties",
        },
        {
            "id": "pydantic.computed.area.serialization-output",
            "contract": "pydantic.computed.area",
            "facet": "structural",
            "kind": "guarantee",
            "statement": "serialization schema includes computed field area as an integer output",
        },
    ]
    clause_assertions: list[dict[str, object]] = [
        {
            "clause": "pydantic.validator.value.accepted-input",
            "representation": "pydantic.validator.runtime",
            "stance": "supports",
            "evidence": [
                "obs.pydantic.validator.runtime.integer",
                "obs.pydantic.validator.runtime.string",
            ],
        },
        {
            "clause": "pydantic.validator.value.accepted-input",
            "representation": "pydantic.validator.validation-schema",
            "stance": "supports",
            "evidence": ["obs.pydantic.validator.validation-schema"],
        },
        {
            "clause": "pydantic.validator.value.normalized-state",
            "representation": "pydantic.validator.runtime",
            "stance": "supports",
            "evidence": [
                "obs.pydantic.validator.runtime.integer",
                "obs.pydantic.validator.runtime.string",
            ],
        },
        {
            "clause": "pydantic.validator.value.normalized-state",
            "representation": "pydantic.validator.annotation",
            "stance": "supports",
            "evidence": [static_observation_id],
        },
        {
            "clause": "pydantic.computed.area.validation-input",
            "representation": "pydantic.computed.validation-schema",
            "stance": "supports",
            "evidence": ["obs.pydantic.computed.validation-schema"],
        },
        {
            "clause": "pydantic.computed.area.serialization-output",
            "representation": "pydantic.computed.serialization-schema",
            "stance": "supports",
            "evidence": ["obs.pydantic.computed.serialization-schema"],
        },
        {
            "clause": "pydantic.computed.area.serialization-output",
            "representation": "pydantic.computed.runtime-output",
            "stance": "supports",
            "evidence": ["obs.pydantic.computed.runtime-output"],
        },
    ]

    if claimed_schema is not None:
        claimed = json.loads(claimed_schema.read_text(encoding="utf-8"))
        claimed_value = claimed.get("properties", {}).get("value", {})
        require(
            claimed_value.get("type") == "string" and "anyOf" not in claimed_value,
            "drift fixture must claim string-only validation input",
        )
        sources.append(
            {
                "id": "pydantic.claimed-validation-schema",
                "kind": "file_system",
                "locator": str(claimed_schema),
            }
        )
        representations.append(
            {
                "id": "pydantic.validator.claimed-validation-schema",
                "contract": "pydantic.validator.value",
                "source": "pydantic.claimed-validation-schema",
                "kind": "schema",
                "locator": str(claimed_schema),
                "facets": ["structural", "executable"],
            }
        )
        observations.append(
            {
                "id": "obs.pydantic.validator.claimed-validation-schema",
                "source": "pydantic.claimed-validation-schema",
                "revision": {"kind": "exact", "value": f"fixture:{canonical(claimed)}"},
                "locator": "properties.value",
                "fact": f"claimed validation JSON Schema is {canonical(claimed_value)}",
            }
        )
        clause_assertions.append(
            {
                "clause": "pydantic.validator.value.accepted-input",
                "representation": "pydantic.validator.claimed-validation-schema",
                "stance": "contradicts",
                "evidence": ["obs.pydantic.validator.claimed-validation-schema"],
            }
        )

    return {
        "schema": "chirograph-evidence-v1",
        "sources": sources,
        "contracts": contracts,
        "representations": representations,
        "observations": observations,
        "clauses": clauses,
        "clause_assertions": clause_assertions,
        "relations": [],
        "authority_claims": [],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Assemble the Pydantic Chirograph specimen")
    parser.add_argument("--repository-revision", required=True)
    parser.add_argument("--upstream-revision", required=True)
    parser.add_argument("--claimed-validation-schema", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        evidence = build_evidence(
            args.repository_revision,
            args.upstream_revision,
            args.claimed_validation_schema,
        )
    except (OSError, RuntimeError, TypeError, ValueError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    json.dump(evidence, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
