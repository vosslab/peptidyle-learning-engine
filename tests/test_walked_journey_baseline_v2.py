"""Keep the corrected no-email pilot baseline closed and duplicate-safe."""

import copy
import json
import pathlib
from collections.abc import Callable

import pytest


REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[1]
BASELINE_PATH = REPOSITORY_ROOT / "tests" / "fixtures" / "walked_journey_baseline_v2.json"
EXPECTED_JOURNEYS = ["J11", "J12", "J13", "J1", "J2", "J3", "J4", "J5", "J8"]


def reject_duplicate_json_members(pairs: list[tuple[str, object]]) -> dict[str, object]:
	"""Reject every duplicate before JSON's last-member-wins behavior can occur."""
	result: dict[str, object] = {}
	for key, value in pairs:
		if key in result:
			raise ValueError(f"duplicate JSON member: {key}")
		result[key] = value
	return result


def load_baseline_text(text: str) -> dict[str, object]:
	"""Decode exactly one object with duplicate-member rejection at every depth."""
	parsed = json.loads(text, object_pairs_hook=reject_duplicate_json_members)
	if not isinstance(parsed, dict):
		raise ValueError("baseline must be an object")
	return parsed


def load_baseline() -> dict[str, object]:
	"""Read the committed, non-live corrected baseline."""
	return load_baseline_text(BASELINE_PATH.read_text(encoding="ascii"))


def validate_baseline(baseline: object) -> None:
	"""Reject anything that expands scope or overstates a no-email pilot PASS."""
	if not isinstance(baseline, dict):
		raise ValueError("baseline must be an object")
	if list(baseline) != ["schemaVersion", "recordType", "masterSeed", "arrangementLabels", "journeys"]:
		raise ValueError("baseline keys are not closed")
	if baseline["schemaVersion"] != 2 or baseline["recordType"] != "prospective-walked-journey-fixture":
		raise ValueError("fixture is not the corrected prospective record")
	if baseline["masterSeed"] != 42 or baseline["arrangementLabels"] != ["api-retry-corpus-publication"]:
		raise ValueError("baseline seed or arrangements drifted")
	journeys = baseline["journeys"]
	expected_rows = [{"id": journey, "outcome": "PASS"} for journey in EXPECTED_JOURNEYS]
	if journeys != expected_rows:
		raise ValueError("baseline journeys must be the corrected ordered PASS charter")


def test_walked_journey_baseline_v2_is_ascii_closed_and_current() -> None:
	"""The successor baseline contains only the corrected, no-email pilot journeys."""
	assert BASELINE_PATH.read_bytes().isascii()
	validate_baseline(load_baseline())


@pytest.mark.parametrize(
	("original", "replacement"),
	[
		('"schemaVersion": 2,', '"schemaVersion": 2,\n  "schemaVersion": 1,'),
		('"id": "J5",', '"id": "J5", "id": "J6",'),
		('"outcome": "PASS"', '"outcome": "PASS", "outcome": "BLOCKED"'),
	],
)
def test_walked_journey_baseline_v2_rejects_duplicate_members(
	original: str,
	replacement: str,
) -> None:
	"""No duplicate member can smuggle a different no-email acceptance result."""
	hostile = BASELINE_PATH.read_text(encoding="ascii").replace(original, replacement, 1)
	with pytest.raises(ValueError, match="duplicate JSON member"):
		load_baseline_text(hostile)


@pytest.mark.parametrize(
	("mutation", "message"),
	[
		(lambda value: value.update({"email": "forbidden"}), "keys"),
		(lambda value: value.update({"arrangementLabels": ["local-identity-availability"]}), "arrangements"),
		(lambda value: value["journeys"].append({"id": "J6", "outcome": "BLOCKED"}), "journeys"),
		(lambda value: value["journeys"].pop(), "journeys"),
	],
)
def test_walked_journey_baseline_v2_rejects_scope_drift(
	mutation: Callable[[dict[str, object]], None],
	message: str,
) -> None:
	"""Email, non-corpus arrangements, and changed PASS rows are rejected."""
	baseline = copy.deepcopy(load_baseline())
	mutation(baseline)
	with pytest.raises(ValueError, match=message):
		validate_baseline(baseline)
