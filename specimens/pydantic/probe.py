#!/usr/bin/env python3
"""Observe pinned Pydantic behavior through the generic Python runtime boundary."""

from __future__ import annotations

import argparse
import json
import platform
import re
import sys
from pathlib import Path

import pydantic
import pydantic_core

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from specimens.pydantic.models import Rectangle, ValidatorModel
from specimens.python_runtime import RuntimeProbe

EXPECTED_PYDANTIC_VERSION = "2.13.5"
EXPECTED_CORE_VERSION = "2.46.5"
SHA1 = re.compile(r"^[0-9a-f]{40}$")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def canonical(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def exact_runtime_revision(upstream_revision: str) -> str:
    return (
        f"pydantic@{upstream_revision};"
        f"pydantic={pydantic.__version__};"
        f"pydantic-core={pydantic_core.__version__};"
        f"python={platform.python_version()}"
    )


def observe_pydantic_runtime(upstream_revision: str) -> dict[str, object]:
    require(
        SHA1.fullmatch(upstream_revision) is not None,
        "upstream revision must be a full lowercase Git SHA",
    )
    require(
        pydantic.__version__ == EXPECTED_PYDANTIC_VERSION,
        f"expected Pydantic {EXPECTED_PYDANTIC_VERSION}, observed {pydantic.__version__}",
    )
    require(
        pydantic_core.__version__ == EXPECTED_CORE_VERSION,
        f"expected pydantic-core {EXPECTED_CORE_VERSION}, observed {pydantic_core.__version__}",
    )

    revision = exact_runtime_revision(upstream_revision)
    probe = RuntimeProbe(
        source_id="pydantic.runtime",
        locator=f"https://github.com/pydantic/pydantic@{upstream_revision}",
        revision=revision,
    )
    probe.observe(
        observation_id="obs.pydantic.runtime.revision",
        locator="runtime package identity",
        fact=f"exact runtime identity is {revision}",
    )

    validator_validation_schema = ValidatorModel.model_json_schema(mode="validation")
    validator_serialization_schema = ValidatorModel.model_json_schema(mode="serialization")
    validation_value_schema = validator_validation_schema["properties"]["value"]
    serialization_value_schema = validator_serialization_schema["properties"]["value"]

    accepted_types = {
        branch.get("type")
        for branch in validation_value_schema.get("anyOf", [])
        if isinstance(branch, dict)
    }
    if "type" in validation_value_schema:
        accepted_types.add(validation_value_schema["type"])
    require(
        accepted_types == {"integer", "string"},
        f"unexpected validation perspective for value: {validation_value_schema!r}",
    )
    require(
        serialization_value_schema.get("type") == "string",
        f"unexpected serialization perspective for value: {serialization_value_schema!r}",
    )

    from_integer = ValidatorModel.model_validate({"value": 1})
    from_string = ValidatorModel.model_validate({"value": "a"})
    require(from_integer.value == "1", "integer input did not normalize to string")
    require(from_string.value == "a", "string input did not remain a string")

    probe.observe(
        observation_id="obs.pydantic.validator.runtime.integer",
        locator="ValidatorModel.model_validate({'value': 1})",
        fact="integer input 1 is accepted and normalized to string '1'",
    )
    probe.observe(
        observation_id="obs.pydantic.validator.runtime.string",
        locator="ValidatorModel.model_validate({'value': 'a'})",
        fact="string input 'a' is accepted and remains string 'a'",
    )
    probe.observe(
        observation_id="obs.pydantic.validator.validation-schema",
        locator="ValidatorModel.model_json_schema(mode=validation).properties.value",
        fact=f"validation JSON Schema is {canonical(validation_value_schema)}",
    )
    probe.observe(
        observation_id="obs.pydantic.validator.serialization-schema",
        locator="ValidatorModel.model_json_schema(mode=serialization).properties.value",
        fact=f"serialization JSON Schema is {canonical(serialization_value_schema)}",
    )

    rectangle_validation_schema = Rectangle.model_json_schema(mode="validation")
    rectangle_serialization_schema = Rectangle.model_json_schema(mode="serialization")
    require(
        "area" not in rectangle_validation_schema["properties"],
        "computed field unexpectedly appeared in validation schema",
    )
    require(
        rectangle_serialization_schema["properties"].get("area", {}).get("type")
        == "integer",
        "computed field missing from serialization schema",
    )
    rectangle = Rectangle.model_validate({"width": 2, "height": 3})
    rectangle_dump = rectangle.model_dump()
    require(
        rectangle_dump.get("area") == 6,
        "computed field missing from serialized runtime output",
    )

    probe.observe(
        observation_id="obs.pydantic.computed.validation-schema",
        locator="Rectangle.model_json_schema(mode=validation).properties",
        fact=f"validation properties are {canonical(rectangle_validation_schema['properties'])}",
    )
    probe.observe(
        observation_id="obs.pydantic.computed.serialization-schema",
        locator="Rectangle.model_json_schema(mode=serialization).properties",
        fact=f"serialization properties are {canonical(rectangle_serialization_schema['properties'])}",
    )
    probe.observe(
        observation_id="obs.pydantic.computed.runtime-output",
        locator="Rectangle(width=2,height=3).model_dump()",
        fact=f"serialized runtime output is {canonical(rectangle_dump)}",
    )

    return probe.document()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Observe pinned Pydantic runtime perspectives")
    parser.add_argument("--upstream-revision", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        document = observe_pydantic_runtime(args.upstream_revision)
    except (KeyError, RuntimeError, TypeError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    json.dump(document, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
