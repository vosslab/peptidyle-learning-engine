"""Keep the current walked-journey baseline closed, public-only, and non-live."""

import copy
import json
import pathlib
from collections.abc import Callable

import pytest


REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[1]
BASELINE_PATH = REPOSITORY_ROOT / "docs" / "active_plans" / "walked_journey_baseline.json"
ARRANGEMENT_LABELS = [
	"launcher-seeded-enrollment",
	"launcher-baseline-assignment",
	"api-retry-corpus-publication",
	"api-mastery-assignment",
	"api-exam-assignment",
]
JOURNEY_DEPENDENCIES = [
	("J1", "PASS", None),
	("J2", "PASS", None),
	("J3", "PASS", None),
	("J4", "PASS", None),
	("J5", "PASS", None),
	("J6", "BLOCKED", "RELEASE_READINESS_PREREQUISITE"),
	("J7", "BLOCKED", "RELEASE_READINESS_PREREQUISITE"),
	("J8", "PASS", None),
	("J9", "BLOCKED", "CANONICAL_ONBOARDING_PREREQUISITE"),
	("J10", "BLOCKED", "CANONICAL_ONBOARDING_PREREQUISITE"),
	("ALL_FAMILY", "BLOCKED", "ALL_FAMILY_AND_SECURE_PAYLOAD_RELEASE_GATES"),
	("MULTI_LEARNER", "BLOCKED", "CANONICAL_ONBOARDING_AND_ALL_FAMILY_RELEASE_GATES"),
]


def load_baseline() -> dict[str, object]:
	"""Read the one committed baseline without treating it as a live report."""
	return load_baseline_text(BASELINE_PATH.read_text(encoding="ascii"))


def reject_duplicate_json_members(pairs: list[tuple[str, object]]) -> dict[str, object]:
	"""Reject duplicate JSON names at every object level before any validation."""
	result: dict[str, object] = {}
	for key, value in pairs:
		if key in result:
			raise ValueError(f"duplicate JSON member: {key}")
		result[key] = value
	return result


def load_baseline_text(text: str) -> dict[str, object]:
	"""Decode one baseline record without JSON's unsafe last-member-wins rule."""
	parsed = json.loads(text, object_pairs_hook=reject_duplicate_json_members)
	if not isinstance(parsed, dict):
		raise ValueError("baseline must be an object")
	return parsed


def validate_baseline(baseline: object) -> None:
	"""Reject drift that could overstate terminal walkthrough coverage."""
	if not isinstance(baseline, dict):
		raise ValueError("baseline must be an object")
	if list(baseline) != [
		"schemaVersion",
		"recordType",
		"masterSeed",
		"arrangementLabels",
		"journeys",
	]:
		raise ValueError("baseline keys are not closed")
	if baseline["schemaVersion"] != 1 or baseline["recordType"] != "walked-journey-baseline":
		raise ValueError("baseline is not the fixed non-live record")
	if baseline["masterSeed"] != 42 or baseline["arrangementLabels"] != ARRANGEMENT_LABELS:
		raise ValueError("baseline seed or arrangements drifted")
	journeys = baseline["journeys"]
	if not isinstance(journeys, list):
		raise ValueError("baseline journeys must be a list")
	expected_rows: list[dict[str, str]] = []
	for journey_id, outcome, reason_code in JOURNEY_DEPENDENCIES:
		row = {"id": journey_id, "outcome": outcome}
		if reason_code is not None:
			row["reasonCode"] = reason_code
		expected_rows.append(row)
	if journeys != expected_rows:
		raise ValueError("baseline journeys or dependency reasons drifted")


def test_walked_journey_baseline_is_ascii_closed_and_current() -> None:
	"""The committed record has the exact terminal, public-only baseline shape."""
	baseline_bytes = BASELINE_PATH.read_bytes()
	assert baseline_bytes.isascii()
	validate_baseline(load_baseline())


@pytest.mark.parametrize(
	("original", "replacement"),
	[
		(
			'"recordType": "walked-journey-baseline",',
			'"recordType": "walked-journey-baseline",\n'
			'  "recordType": "visible-outcome-report",',
		),
		(
			'"masterSeed": 42,',
			'"masterSeed": 42,\n'
			'  "courseId": "public-id-is-not-baseline-evidence",\n'
			'  "courseId": "duplicate",',
		),
		(
			'"id": "J3",',
			'"id": "J3",\n'
			'      "id": "J3",',
		),
		(
			'"outcome": "BLOCKED",',
			'"outcome": "BLOCKED",\n'
			'      "outcome": "PASS",',
		),
		(
			'"reasonCode": "RELEASE_READINESS_PREREQUISITE"',
			'"reasonCode": "RELEASE_READINESS_PREREQUISITE",\n'
			'      "reasonCode": "CANONICAL_ONBOARDING_PREREQUISITE"',
		),
	],
)
def test_walked_journey_baseline_rejects_duplicate_raw_json_members(
	original: str,
	replacement: str,
) -> None:
	"""Duplicate top-level and journey members cannot smuggle last-wins values."""
	raw_baseline = BASELINE_PATH.read_text(encoding="ascii")
	hostile = raw_baseline.replace(original, replacement, 1)
	with pytest.raises(ValueError, match="duplicate JSON member"):
		load_baseline_text(hostile)


@pytest.mark.parametrize(
	("mutation", "message"),
	[
		(lambda baseline: baseline["journeys"][5].update({"outcome": "PASS"}), "dependency reasons"),
		(
			lambda baseline: baseline["journeys"][5].update(
				{"reasonCode": "CANONICAL_ONBOARDING_PREREQUISITE"}
			),
			"dependency reasons",
		),
		(lambda baseline: baseline.update({"courseId": "public-id-is-not-baseline-evidence"}), "keys"),
		(lambda baseline: baseline.update({"recordType": "visible-outcome-report"}), "non-live"),
	],
)
def test_walked_journey_baseline_rejects_hostile_drift(
	mutation: Callable[[dict[str, object]], None],
	message: str,
) -> None:
	"""Answer-like run detail, false PASS, and report-shape drift fail closed."""
	baseline = copy.deepcopy(load_baseline())
	mutation(baseline)
	with pytest.raises(ValueError, match=message):
		validate_baseline(baseline)
