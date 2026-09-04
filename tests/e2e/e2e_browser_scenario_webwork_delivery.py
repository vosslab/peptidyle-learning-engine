"""Private hand-offs and receipt policy for the real WebWork UI journey."""

import dataclasses
import json
import os
import pathlib
import re


SCENARIO_ID = "webwork_delivery"
PUBLISHED_QUESTION_TITLE = "Biochemistry: Identify hydrophobic compounds from formulas"
PUBLISHED_QUESTION_FIXTURE_SCHEMA_VERSION = 1
ISSUANCE_ACKNOWLEDGEMENT_SCHEMA_VERSION = 1
ISSUANCE_ACKNOWLEDGEMENT_EVENT = "visible_question_issued"
RENDERER_EVENT_TYPE = "renderer_call"
MAXIMUM_PRIVATE_INPUT_BYTES = 1_024
MAXIMUM_OBSERVATION_WINDOW_SECONDS = 600
QUESTION_ID_PATTERN = re.compile(r"^[A-Z0-9]{3}-[A-Z0-9]{4}$")
NAMESPACE_PATTERN = re.compile(r"^bs1-[0-9a-f]{12}-webwork_delivery$")
RENDERER_CALL_PATTERN = re.compile(
	r'ple\.webwork\.cache.*\bevent="renderer_call"', re.MULTILINE
)


class WebworkDeliveryEvidenceError(ValueError):
	"""A WebWork browser-scenario hand-off or receipt is malformed."""


@dataclasses.dataclass(frozen=True)
class PublishedQuestionFixture:
	"""One public Published Question selected for this disposable browser scenario."""

	question_id: str
	question_title: str


@dataclasses.dataclass(frozen=True)
class RendererCallWitness:
	"""Non-sensitive confirmation of one renderer call after visible issuance."""

	scenario_id: str
	event_type: str
	event_count: int
	observation_window_seconds: int

	def as_value(self) -> dict[str, int | str]:
		"""Return the deliberately small public receipt projection."""
		return {
			"scenario": self.scenario_id,
			"eventType": self.event_type,
			"eventCount": self.event_count,
			"observationWindowSeconds": self.observation_window_seconds,
		}


#============================================
def decode_published_question_fixture_receipt(contents: str) -> PublishedQuestionFixture:
	"""Decode one public Published Question receipt without provider internals."""
	value = _decode_canonical_object(contents, {"questionId", "questionTitle"})
	question_id = value["questionId"]
	question_title = value["questionTitle"]
	if (
		not isinstance(question_id, str)
		or QUESTION_ID_PATTERN.fullmatch(question_id) is None
		or question_title != PUBLISHED_QUESTION_TITLE
	):
		raise WebworkDeliveryEvidenceError("WebWork Published Question fixture receipt is invalid")
	return PublishedQuestionFixture(question_id, question_title)


#============================================
def write_published_question_fixture_input(
	path: pathlib.Path, fixture: PublishedQuestionFixture
) -> None:
	"""Write the closed browser hand-off from one already validated public receipt."""
	if not isinstance(fixture, PublishedQuestionFixture):
		raise WebworkDeliveryEvidenceError("WebWork Published Question fixture is invalid")
	if QUESTION_ID_PATTERN.fullmatch(fixture.question_id) is None or fixture.question_title != PUBLISHED_QUESTION_TITLE:
		raise WebworkDeliveryEvidenceError("WebWork Published Question fixture is invalid")
	value = {
		"questionId": fixture.question_id,
		"scenarioId": SCENARIO_ID,
		"schemaVersion": PUBLISHED_QUESTION_FIXTURE_SCHEMA_VERSION,
		"questionTitle": fixture.question_title,
	}
	contents = json.dumps(value, separators=(",", ":"), ensure_ascii=True)
	_write_private_file(path, contents)


#============================================
def validate_published_question_fixture_input(path: pathlib.Path) -> PublishedQuestionFixture:
	"""Require one canonical private browser input before Chromium can read it."""
	contents = _read_private_file(path, "WebWork Published Question fixture input")
	value = _decode_canonical_object(
		contents, {"questionId", "questionTitle", "scenarioId", "schemaVersion"}
	)
	if value.get("schemaVersion") != PUBLISHED_QUESTION_FIXTURE_SCHEMA_VERSION or value.get("scenarioId") != SCENARIO_ID:
		raise WebworkDeliveryEvidenceError("WebWork Published Question fixture input is invalid")
	fixture = decode_published_question_fixture_receipt(
		json.dumps(
			{"questionId": value.get("questionId"), "questionTitle": value.get("questionTitle")},
			separators=(",", ":"),
			ensure_ascii=True,
		)
	)
	if contents != json.dumps(value, separators=(",", ":"), ensure_ascii=True):
		raise WebworkDeliveryEvidenceError("WebWork Published Question fixture input must use canonical ASCII JSON")
	return fixture


#============================================
def validate_visible_issuance_acknowledgement(path: pathlib.Path, namespace: str) -> None:
	"""Bind the renderer witness window to the spec's visible issued-question state."""
	if NAMESPACE_PATTERN.fullmatch(namespace) is None:
		raise WebworkDeliveryEvidenceError("WebWork scenario namespace is invalid")
	contents = _read_private_file(path, "WebWork visible issuance acknowledgement")
	value = _decode_canonical_object(
		contents, {"event", "namespace", "scenarioId", "schemaVersion"}
	)
	expected = {
		"event": ISSUANCE_ACKNOWLEDGEMENT_EVENT,
		"namespace": namespace,
		"scenarioId": SCENARIO_ID,
		"schemaVersion": ISSUANCE_ACKNOWLEDGEMENT_SCHEMA_VERSION,
	}
	if value != expected or contents != json.dumps(expected, separators=(",", ":"), ensure_ascii=True):
		raise WebworkDeliveryEvidenceError("WebWork visible issuance acknowledgement is invalid")


#============================================
def renderer_call_count(evidence_logs: str) -> int:
	"""Count the redacted server's intentionally content-free renderer witness."""
	if not isinstance(evidence_logs, str):
		raise WebworkDeliveryEvidenceError("WebWork evidence logs are invalid")
	return len(RENDERER_CALL_PATTERN.findall(evidence_logs))


#============================================
def renderer_call_witness(
	before_logs: str,
	after_logs: str,
	observation_window_seconds: int,
) -> RendererCallWitness:
	"""Require exactly one new safe renderer event after the UI issuance acknowledgement."""
	if (
		not isinstance(observation_window_seconds, int)
		or isinstance(observation_window_seconds, bool)
		or not 0 < observation_window_seconds <= MAXIMUM_OBSERVATION_WINDOW_SECONDS
	):
		raise WebworkDeliveryEvidenceError("WebWork renderer observation window is invalid")
	delta = renderer_call_count(after_logs) - renderer_call_count(before_logs)
	if delta != 1:
		raise WebworkDeliveryEvidenceError("WebWork visible issuance did not produce one renderer call")
	return RendererCallWitness(
		SCENARIO_ID,
		RENDERER_EVENT_TYPE,
		delta,
		observation_window_seconds,
	)


#============================================
def _decode_canonical_object(contents: str, expected_keys: set[str]) -> dict[str, object]:
	"""Decode a small ASCII JSON object with an exact field boundary."""
	if not isinstance(contents, str) or not contents.isascii() or len(contents.encode("ascii")) > MAXIMUM_PRIVATE_INPUT_BYTES:
		raise WebworkDeliveryEvidenceError("WebWork private input is invalid")
	try:
		value = json.loads(contents)
	except json.JSONDecodeError as error:
		raise WebworkDeliveryEvidenceError("WebWork private input is not valid JSON") from error
	if not isinstance(value, dict) or set(value) != expected_keys:
		raise WebworkDeliveryEvidenceError("WebWork private input has an invalid shape")
	return value


#============================================
def _read_private_file(path: pathlib.Path, label: str) -> str:
	"""Read one owner-managed regular file with the exact private mode."""
	if not path.is_file() or path.is_symlink() or path.stat().st_mode & 0o777 != 0o600:
		raise WebworkDeliveryEvidenceError(label + " is invalid")
	try:
		return path.read_text(encoding="ascii")
	except UnicodeDecodeError as error:
		raise WebworkDeliveryEvidenceError(label + " is invalid") from error


#============================================
def _write_private_file(path: pathlib.Path, contents: str) -> None:
	"""Create one private regular file without a permissive creation window."""
	if path.exists() or path.is_symlink() or not path.parent.is_dir() or not contents.isascii():
		raise WebworkDeliveryEvidenceError("WebWork Published Question fixture input path is invalid")
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(file_descriptor, "w", encoding="ascii") as output:
		output.write(contents)
