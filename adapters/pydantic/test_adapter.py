import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ADAPTER = ROOT / "adapters" / "pydantic" / "extract.py"
STALE_VALIDATION_SCHEMA = ROOT / "adapters" / "pydantic" / "fixtures" / "stale-validation-schema.json"
UPSTREAM_REVISION = "27f473c24ed63a475903d8289c84fb81987f04e9"


def extract_and_inspect(*extra_args: str) -> tuple[dict, str]:
    command = [
        sys.executable,
        str(ADAPTER),
        "--upstream-revision",
        UPSTREAM_REVISION,
        *extra_args,
    ]
    extracted = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, check=False)
    if extracted.returncode != 0:
        raise AssertionError(
            f"Pydantic adapter failed with {extracted.returncode}:\n{extracted.stderr or extracted.stdout}"
        )

    evidence = json.loads(extracted.stdout)
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as handle:
        json.dump(evidence, handle)
        evidence_path = Path(handle.name)

    try:
        inspected = subprocess.run(
            [
                "cargo",
                "run",
                "--quiet",
                "-p",
                "chirograph-cli",
                "--",
                "inspect",
                str(evidence_path),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
    finally:
        evidence_path.unlink(missing_ok=True)

    if inspected.returncode != 0:
        raise AssertionError(
            f"Chirograph inspect failed with {inspected.returncode}:\n{inspected.stderr or inspected.stdout}"
        )
    return evidence, inspected.stdout


class PydanticPerspectiveExperiment(unittest.TestCase):
    def test_validation_and_serialization_perspectives_are_legitimate(self) -> None:
        evidence, output = extract_and_inspect()

        self.assertEqual(evidence["schema"], "chirograph-evidence-v1")
        self.assertIn(UPSTREAM_REVISION, json.dumps(evidence))
        self.assertNotIn("CONTESTED", output)
        self.assertIn(
            "executable clause pydantic.validator.value.accepted-input [requirement] CONSISTENT",
            output,
        )
        self.assertIn(
            "semantic clause pydantic.validator.value.normalized-state [guarantee] CONSISTENT",
            output,
        )
        self.assertIn(
            "structural clause pydantic.computed.area.validation-input [guarantee] CONSISTENT",
            output,
        )
        self.assertIn(
            "structural clause pydantic.computed.area.serialization-output [guarantee] CONSISTENT",
            output,
        )

    def test_same_perspective_stale_schema_is_contested(self) -> None:
        _, output = extract_and_inspect(
            "--claimed-validation-schema",
            str(STALE_VALIDATION_SCHEMA),
        )

        self.assertIn(
            "executable clause pydantic.validator.value.accepted-input [requirement] CONTESTED",
            output,
        )
        self.assertIn("contradicts: pydantic.validator.claimed-validation-schema", output)


if __name__ == "__main__":
    unittest.main()
