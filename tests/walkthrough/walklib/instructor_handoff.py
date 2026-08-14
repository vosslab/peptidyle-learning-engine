"""Validate the public instructor-to-learner walkthrough handoff."""

import json
import os
import pathlib
import re
import stat

import walklib.models


MAX_JOURNEY_ELAPSED_MS = 30 * 60 * 1000
PUBLIC_REFERENCE_TEXT = re.compile(r"^(?P<kind>C|A)-(?P<number>[1-9][0-9]{0,9})$")
MAX_PUBLIC_REFERENCE_NUMBER = 2_147_483_647
HANDOFF_ERROR = "instructor setup public-ID handoff is unavailable"


#============================================
def read_handoff(
	journey_state_file: pathlib.Path | None,
	arrangements: list[dict[str, object]] | None,
	catalog_display_ids: list[str] | None,
) -> tuple[str, str]:
	"""Read the canonical public J11/J12/J13 result without accepting private identifiers.

	Args:
		journey_state_file: Exact private runner-owned JSON file written by browser children.
		arrangements: Validated public arranger result for the current run.
		catalog_display_ids: Human-readable catalog IDs selected by the instructor child.

	Returns:
		The shared course and current mastery-assignment route references for the learner child.

	Raises:
		walklib.models.RunnerError: The fixed public handoff is absent, replaced, or malformed.
	"""
	if (
		journey_state_file is None
		or journey_state_file.name != "journeys.json"
		or journey_state_file.is_symlink()
	):
		raise walklib.models.RunnerError(HANDOFF_ERROR)
	parent = journey_state_file.parent
	parent_descriptor = -1
	file_descriptor = -1
	try:
		parent_descriptor = os.open(
			parent,
			os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0),
		)
		parent_metadata = os.fstat(parent_descriptor)
		file_descriptor = os.open(
			"journeys.json",
			os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0),
			dir_fd=parent_descriptor,
		)
	except OSError as error:
		raise walklib.models.RunnerError(HANDOFF_ERROR) from error
	try:
		metadata = os.fstat(file_descriptor)
		current_parent = parent.lstat()
		if (
			not stat.S_ISDIR(parent_metadata.st_mode)
			or stat.S_IMODE(parent_metadata.st_mode) != 0o700
			or not stat.S_ISREG(metadata.st_mode)
			or stat.S_IMODE(metadata.st_mode) != 0o600
			or metadata.st_size > 4096
			or not stat.S_ISDIR(current_parent.st_mode)
			or stat.S_ISLNK(current_parent.st_mode)
			or stat.S_IMODE(current_parent.st_mode) != 0o700
			or current_parent.st_dev != parent_metadata.st_dev
			or current_parent.st_ino != parent_metadata.st_ino
		):
			raise walklib.models.RunnerError(HANDOFF_ERROR)
		raw = os.read(file_descriptor, metadata.st_size)
	finally:
		os.close(file_descriptor)
		os.close(parent_descriptor)
	try:
		text = raw.decode("ascii")
		value = json.loads(text)
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise walklib.models.RunnerError(HANDOFF_ERROR) from error
	if text != json.dumps(value, separators=(",", ":")) + "\n":
		raise walklib.models.RunnerError(HANDOFF_ERROR)
	if not isinstance(value, list) or len(value) != 3:
		raise walklib.models.RunnerError(HANDOFF_ERROR)
	expected = (
		(
			"J11",
			{
				"schemaVersion", "journey", "status", "elapsedMs", "courseReference",
				"visibleOutcomeCodes", "diagnostics",
			},
			["visible_course_created", "visible_course_opened"],
		),
		(
			"J12",
			{
				"schemaVersion", "journey", "status", "elapsedMs", "courseReference",
				"visibleOutcomeCodes", "diagnostics",
			},
			["visible_local_student_active"],
		),
		(
			"J13",
			{
				"schemaVersion", "journey", "status", "elapsedMs", "courseReference",
				"assignmentReference", "selectedDisplayIds", "visibleOutcomeCodes", "diagnostics",
			},
			[
				"visible_assignment_created", "visible_catalog_problem_selected",
				"visible_four_question_chapter_one_selection", "visible_mastery_policy",
			],
		),
	)
	course_reference: str | None = None
	for fragment, (journey, keys, outcome_codes) in zip(value, expected, strict=True):
		if not isinstance(fragment, dict) or set(fragment) != keys:
			raise walklib.models.RunnerError(HANDOFF_ERROR)
		if (
			fragment.get("schemaVersion") != 2
			or fragment.get("journey") != journey
			or fragment.get("status") != "PASS"
			or not isinstance(fragment.get("elapsedMs"), int)
			or isinstance(fragment.get("elapsedMs"), bool)
			or fragment["elapsedMs"] < 0
			or fragment["elapsedMs"] > MAX_JOURNEY_ELAPSED_MS
			or not isinstance(fragment.get("diagnostics"), list)
			or fragment["diagnostics"] != []
			or fragment.get("visibleOutcomeCodes") != outcome_codes
			or not isinstance(fragment.get("courseReference"), str)
			or not valid_reference(fragment["courseReference"], "C")
		):
			raise walklib.models.RunnerError(HANDOFF_ERROR)
		if course_reference is None:
			course_reference = fragment["courseReference"]
		elif fragment["courseReference"] != course_reference:
			raise walklib.models.RunnerError(HANDOFF_ERROR)
	j13 = value[2]
	if course_reference is None or not isinstance(j13, dict):
		raise walklib.models.RunnerError(HANDOFF_ERROR)
	assignment_reference = j13.get("assignmentReference")
	if not valid_reference(assignment_reference, "A"):
		raise walklib.models.RunnerError(HANDOFF_ERROR)
	if arrangements is None or len(arrangements) != 1 or catalog_display_ids is None:
		raise walklib.models.RunnerError(HANDOFF_ERROR)
	selected_display_ids = j13.get("selectedDisplayIds")
	if not isinstance(selected_display_ids, list) or selected_display_ids != catalog_display_ids:
		raise walklib.models.RunnerError(HANDOFF_ERROR)
	return course_reference, assignment_reference


#============================================
def valid_reference(value: object, expected_kind: str) -> bool:
	"""Accept one bounded human-facing C-* or A-* route reference."""
	if not isinstance(value, str):
		return False
	match = PUBLIC_REFERENCE_TEXT.fullmatch(value)
	if match is None or match["kind"] != expected_kind:
		return False
	return int(match["number"]) <= MAX_PUBLIC_REFERENCE_NUMBER
