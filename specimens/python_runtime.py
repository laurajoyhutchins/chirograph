"""Framework-agnostic runtime observation helper for Python specimens."""

from __future__ import annotations


class RuntimeProbe:
    """Collect exact-revision observations from one executable Python source."""

    def __init__(self, *, source_id: str, locator: str, revision: str) -> None:
        self._source_id = _required_text(source_id, "source_id")
        self._locator = _required_text(locator, "locator")
        self._revision = _required_text(revision, "revision")
        self._observations: list[dict[str, object]] = []

    def observe(self, *, observation_id: str, locator: str, fact: str) -> None:
        self._observations.append(
            {
                "id": _required_text(observation_id, "observation_id"),
                "source": self._source_id,
                "revision": {"kind": "exact", "value": self._revision},
                "locator": _required_text(locator, "locator"),
                "fact": _required_text(fact, "fact"),
            }
        )

    def document(self) -> dict[str, object]:
        return {
            "schema": "python-runtime-observations-v1",
            "sources": [
                {
                    "id": self._source_id,
                    "kind": "executable",
                    "locator": self._locator,
                }
            ],
            "observations": list(self._observations),
        }


def _required_text(value: str, field: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value:
        raise ValueError(f"{field} must be non-empty and trimmed")
    return value
