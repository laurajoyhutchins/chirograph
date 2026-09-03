import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPECIMEN = ROOT / "specimens" / "pydantic" / "evidence.py"
STALE_VALIDATION_SCHEMA = (
    ROOT / "specimens" / "pydantic" / "fixtures" / "stale-validation-schema.json"
)
UPSTREAM_REVISION = "001dea020e0809844e5b17666432c9135a976f46"


def repository_revision() -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(f"cannot resolve repository revision: {result.stderr}")
    revision = result.stdout.strip()
    if len(revision) != 40 or any(char not in "0123456789abcdef" for char in revision):
        raise AssertionError(f"git returned a non-exact revision: {revision!r}")
    return revision


def extract_and_inspect(*extra_args: str) -> tuple[dict, str]:
    if not SPECIMEN.exists():
        raise AssertionError(
            "Pydantic must be exposed from specimens/pydantic/evidence.py, not adapters/pydantic"
        )

    command = [
        sys.executable,
        str(SPECIMEN),
        "--repository-revision",
        repository_revision(),
        "--upstream-revision",
        UPSTREAM_REVISION,
        *extra_args,
    ]
    extracted = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, check=False)
    if extracted.returncode != 0:
        raise AssertionError(
            f"Pydantic specimen failed with {extracted.returncode}:\n{extracted.stderr or extracted.stdout}"
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


class PydanticSpecimenExperiment(unittest.TestCase):
    def test_static_source_comes_from_generic_python_acquisition(self) -> None:
        evidence, output = extract_and_inspect()

        self.assertEqual(evidence["schema"], "chirograph-evidence-v1")
        serialized = json.dumps(evidence)
        self.assertNotIn("adapters/pydantic", serialized)
        self.assertIn(UPSTREAM_REVISION, serialized)

        static_observations = [
            observation
            for observation in evidence["observations"]
            if observation["id"].startswith("obs.python.")
            and "value: str" in observation["fact"]
        ]
        self.assertEqual(len(static_observations), 1)
        static_observation = static_observations[0]
        self.assertEqual(
            static_observation["revision"],
            {"kind": "exact", "value": repository_revision()},
        )
        self.assertTrue(
            static_observation["locator"].startswith("specimens/pydantic/models.py:B")
        )

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
