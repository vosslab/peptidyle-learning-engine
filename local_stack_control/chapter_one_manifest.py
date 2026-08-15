"""Strict private Chapter One publication-manifest boundary."""

import json
import re
import uuid

import local_stack_control.models


MAX_MANIFEST_BYTES = 64 * 1024
QUESTION_ID_PATTERN = re.compile(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}\Z")
EXPECTED_CHAPTERS = (
	(
		"genetics-chapter-1",
		(
			"genetics-disorders-webwork-mc",
			"genetics-disorders-webwork-matching",
			"genetics-disorders-flat-mc",
			"genetics-disorders-flat-matching",
		),
	),
	(
		"biochemistry-chapter-1",
		(
			"biochemistry-functional-groups-webwork-mc",
			"biochemistry-functional-groups-webwork-matching",
			"biochemistry-functional-groups-flat-mc",
			"biochemistry-functional-groups-flat-matching",
		),
	),
)


#============================================
def unique_json_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
	"""Build one JSON object while refusing ambiguous duplicate declarations."""
	result: dict[str, object] = {}
	for name, value in pairs:
		if name in result:
			raise ValueError(f"duplicate JSON field {name}")
		result[name] = value
	return result


#============================================
def require_uuid(value: object, label: str) -> str:
	"""Require one schema UUID without returning or reporting its value."""
	if not isinstance(value, str):
		raise local_stack_control.models.ControllerError(
			f"Chapter One manifest {label} is not a UUID string"
		)
	try:
		uuid.UUID(value)
	except ValueError as error:
		raise local_stack_control.models.ControllerError(
			f"Chapter One manifest {label} is not a UUID"
		) from error
	return value


#============================================
def require_keys(value: dict[str, object], expected: tuple[str, ...], label: str) -> None:
	"""Require the closed field set for one non-secret manifest object."""
	if set(value) != set(expected):
		raise local_stack_control.models.ControllerError(
			f"Chapter One manifest {label} has an unexpected field set"
		)


#============================================
def parse_manifest_bytes(manifest_bytes: bytes) -> dict[str, object]:
	"""Parse and validate the complete answer-free Chapter One manifest schema."""
	if not manifest_bytes or len(manifest_bytes) > MAX_MANIFEST_BYTES:
		raise local_stack_control.models.ControllerError(
			"Chapter One manifest is empty or exceeds the private size limit"
		)
	try:
		manifest = json.loads(manifest_bytes, object_pairs_hook=unique_json_object)
	except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
		raise local_stack_control.models.ControllerError(
			"Chapter One publisher emitted an invalid manifest"
		) from error
	if not isinstance(manifest, dict):
		raise local_stack_control.models.ControllerError(
			"Chapter One manifest is not a JSON object"
		)
	require_keys(manifest, ("chapters",), "root")
	chapters = manifest["chapters"]
	if not isinstance(chapters, list) or len(chapters) != len(EXPECTED_CHAPTERS):
		raise local_stack_control.models.ControllerError(
			"Chapter One manifest does not contain the complete chapter collection"
		)
	enrollment_ids: set[str] = set()
	display_ids: set[str] = set()
	problem_versions: set[tuple[str, str]] = set()
	for chapter_index, (expected, chapter) in enumerate(
		zip(EXPECTED_CHAPTERS, chapters, strict=True),
		start=1,
	):
		expected_slug, expected_questions = expected
		if not isinstance(chapter, dict):
			raise local_stack_control.models.ControllerError(
				f"Chapter One manifest chapter {chapter_index} is not an object"
			)
		require_keys(
			chapter,
			("slug", "courseId", "assignmentId", "enrollmentId", "questions"),
			f"chapter {chapter_index}",
		)
		if chapter["slug"] != expected_slug:
			raise local_stack_control.models.ControllerError(
				f"Chapter One manifest chapter {chapter_index} has an unexpected identity"
			)
		for name in ("courseId", "assignmentId"):
			require_uuid(chapter[name], f"{expected_slug}.{name}")
		enrollment_id = require_uuid(chapter["enrollmentId"], f"{expected_slug}.enrollmentId")
		if enrollment_id in enrollment_ids:
			raise local_stack_control.models.ControllerError(
				"Chapter One manifest repeats an enrollment identity"
			)
		enrollment_ids.add(enrollment_id)
		questions = chapter["questions"]
		if not isinstance(questions, list) or len(questions) != len(expected_questions):
			raise local_stack_control.models.ControllerError(
				f"Chapter One manifest {expected_slug} has an incomplete question collection"
			)
		for question_index, (expected_question, question) in enumerate(
			zip(expected_questions, questions, strict=True),
			start=1,
		):
			if not isinstance(question, dict):
				raise local_stack_control.models.ControllerError(
					f"Chapter One manifest {expected_slug} question {question_index} is not an object"
				)
			require_keys(
				question,
				("slug", "displayId", "problemId", "versionId"),
				f"{expected_slug} question {question_index}",
			)
			if question["slug"] != expected_question:
				raise local_stack_control.models.ControllerError(
					f"Chapter One manifest {expected_slug} question {question_index} has an unexpected identity"
				)
			display_id = question["displayId"]
			if not isinstance(display_id, str) or not QUESTION_ID_PATTERN.fullmatch(display_id):
				raise local_stack_control.models.ControllerError(
					f"Chapter One manifest {expected_question} has an invalid Question ID"
				)
			problem_id = require_uuid(question["problemId"], f"{expected_question}.problemId")
			version_id = require_uuid(question["versionId"], f"{expected_question}.versionId")
			if display_id in display_ids or (problem_id, version_id) in problem_versions:
				raise local_stack_control.models.ControllerError(
					"Chapter One manifest repeats a question publication identity"
				)
			display_ids.add(display_id)
			problem_versions.add((problem_id, version_id))
	return manifest
