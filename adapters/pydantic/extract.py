#!/usr/bin/env python3
"""Read-only Pydantic specimen adapter for Chirograph.

The adapter observes public Pydantic behavior at a caller-supplied exact upstream
revision. It never modifies the Pydantic checkout or package.
"""

from __future__ import annotations

import argparse
import json
import platform
import re
import sys
from pathlib import Path
from typing import Any

import pydantic
import pydantic_core
from pydantic import BaseModel, computed_field, field_validator

EXPECTED_PYDANTIC_VERSION = "2.14.0b1"
EXPECTED_CORE_VERSION = "2.48.0"
SHA1 = re.compile(r"^[0-9a-f]{40}$")


class ValidatorModel(BaseModel):
    value: str

    @field_validator("value", mode="before", json_schema_input_type=int | str)
    @classmethod
    def cast_ints(cls, value: Any) -> Any:
        if isinstance(value, int):
            return str(value)
        return value


class Rectangle(BaseModel):
    width: int
    height: int

    @computed_field
    @property
    def area(self) -> int:
        return self.width * self.height


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def canonical(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def exact_revision(upstream_revision: str) -> str:
    return (
        f"pydantic@{upstream_revision};"
        f"pydantic={pydantic.__version__};"
        f"pydantic-core={pydantic_core.__version__};"
        f"python={platform.python_version()}"
    )


def revision(value: str) -> dict[str, str]:
    return {"kind": "exact", "value": value}


def observed_pydantic(upstream_revision: str, claimed_schema: Path | None) -> dict[str, object]:
    require(SHA1.fullmatch(upstream_revision) is not None, "upstream revision must be a full lowercase Git SHA")
    require(
        pydantic.__version__ == EXPECTED_PYDANTIC_VERSION,
        f"expected Pydantic {EXPECTED_PYDANTIC_VERSION}, observed {pydantic.__version__}",
    )
    require(
        pydantic_core.__version__ == EXPECTED_CORE_VERSION,
        f"expected pydantic-core {EXPECTED_CORE_VERSION}, observed {pydantic_core.__version__}",
    )

    observed_revision = exact_revision(upstream_revision)

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
    require(
        ValidatorModel.model_fields["value"].annotation is str,
        "declared validated-state annotation is not str",
    )

    rectangle_validation_schema = Rectangle.model_json_schema(mode="validation")
    rectangle_serialization_schema = Rectangle.model_json_schema(mode="serialization")
    require(
        "area" not in rectangle_validation_schema["properties"],
        "computed field unexpectedly appeared in validation schema",
    )
    require(
        rectangle_serialization_schema["properties"].get("area", {}).get("type") == "integer",
        "computed field missing from serialization schema",
    )
    rectangle = Rectangle.model_validate({"width": 2, "height": 3})
    rectangle_dump = rectangle.model_dump()
    require(rectangle_dump.get("area") == 6, "computed field missing from serialized runtime output")

    sources: list[dict[str, object]] = [
        {
            "id": "pydantic.runtime",
            "kind": "executable",
            "locator": f"https://github.com/pydantic/pydantic@{upstream_revision}",
        }
    ]
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
            "source": "pydantic.runtime",
            "kind": "type_definition",
            "locator": "ValidatorModel.value: str",
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
    observations: list[dict[str, object]] = [
        {
            "id": "obs.pydantic.validator.annotation",
            "source": "pydantic.runtime",
            "revision": revision(observed_revision),
            "locator": "ValidatorModel.model_fields[value].annotation",
            "fact": "validated-state annotation is str",
        },
        {
            "id": "obs.pydantic.validator.runtime.integer",
            "source": "pydantic.runtime",
            "revision": revision(observed_revision),
            "locator": "ValidatorModel.model_validate({'value': 1})",
            "fact": "integer input 1 is accepted and normalized to string '1'",
        },
        {
            "id": "obs.pydantic.validator.runtime.string",
            "source": "pydantic.runtime",
            "revision": revision(observed_revision),
            "locator": "ValidatorModel.model_validate({'value': 'a'})",
            "fact": "string input 'a' is accepted and remains string 'a'",
        },
        {
            "id": "obs.pydantic.validator.validation-schema",
            "source": "pydantic.runtime",
            "revision": revision(observed_revision),
            "locator": "ValidatorModel.model_json_schema(mode=validation).properties.value",
            "fact": f"validation JSON Schema is {canonical(validation_value_schema)}",
        },
        {
            "id": "obs.pydantic.validator.serialization-schema",
            "source": "pydantic.runtime",
            "revision": revision(observed_revision),
            "locator": "ValidatorModel.model_json_schema(mode=serialization).properties.value",
            "fact": f"serialization JSON Schema is {canonical(serialization_value_schema)}",
        },
        {
            "id": "obs.pydantic.computed.validation-schema",
            "source": "pydantic.runtime",
            "revision": revision(observed_revision),
            "locator": "Rectangle.model_json_schema(mode=validation).properties",
            "fact": f"validation properties are {canonical(rectangle_validation_schema['properties'])}",
        },
        {
            "id": "obs.pydantic.computed.serialization-schema",
            "source": "pydantic.runtime",
            "revision": revision(observed_revision),
            "locator": "Rectangle.model_json_schema(mode=serialization).properties",
            "fact": f"serialization properties are {canonical(rectangle_serialization_schema['properties'])}",
        },
        {
            "id": "obs.pydantic.computed.runtime-output",
            "source": "pydantic.runtime",
            "revision": revision(observed_revision),
            "locator": "Rectangle(width=2,height=3).model_dump()",
            "fact": f"serialized runtime output is {canonical(rectangle_dump)}",
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
            "evidence": ["obs.pydantic.validator.annotation"],
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
                "revision": revision(f"fixture:{canonical(claimed)}"),
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
    parser = argparse.ArgumentParser(description="Observe pinned Pydantic contract perspectives")
    parser.add_argument("--upstream-revision", required=True)
    parser.add_argument("--claimed-validation-schema", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        evidence = observed_pydantic(args.upstream_revision, args.claimed_validation_schema)
    except (KeyError, OSError, RuntimeError, TypeError, ValueError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    json.dump(evidence, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
