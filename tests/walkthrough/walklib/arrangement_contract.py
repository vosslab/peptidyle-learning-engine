"""Strict public arrangement-output boundary for the UI walkthrough runner."""

import json
import re


MAX_ARRANGEMENT_OUTPUT_BYTES = 2048
UUID_TEXT = re.compile(
	"^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
	re.IGNORECASE,
)
DISPLAY_ID = re.compile(r"^P-[1-9][0-9]*-v[1-9][0-9]*$")


def parse_arrangement_output(stdout: str) -> tuple[list[dict[str, object]], list[dict[str, str]] | None]:
	"""Return strict public arrangement references and private instructor question locators."""
	try:
		encoded = stdout.encode("ascii")
		payload = json.loads(stdout)
	except (UnicodeEncodeError, json.JSONDecodeError) as error:
		raise ValueError("invalid output") from error
	if (
		len(encoded) > MAX_ARRANGEMENT_OUTPUT_BYTES
		or not stdout.endswith("\n")
		or stdout.count("\n") != 1
		or "\r" in stdout
		or stdout[0].isspace()
		or not isinstance(payload, dict)
		or set(payload) != {"arrangements"}
		or stdout != json.dumps(payload, separators=(",", ":")) + "\n"
	):
		raise ValueError("invalid output")
	arrangements = payload["arrangements"]
	if not isinstance(arrangements, list) or len(arrangements) not in {1, 5}:
		raise ValueError("invalid output")
	if len(arrangements) == 1:
		value = arrangements[0]
		if (
			not isinstance(value, dict)
			or set(value) != {"label", "questions"}
			or value.get("label") != "launcher-chapter-one-genetics"
			or not isinstance(value["questions"], list)
			or len(value["questions"]) != 4
		):
			raise ValueError("invalid output")
		questions: list[dict[str, str]] = []
		for question in value["questions"]:
			if (
				not isinstance(question, dict)
				or set(question) != {"displayId", "problemId", "versionId"}
				or not all(isinstance(question[key], str) for key in question)
				or not DISPLAY_ID.fullmatch(question["displayId"])
				or not UUID_TEXT.fullmatch(question["problemId"])
				or not UUID_TEXT.fullmatch(question["versionId"])
			):
				raise ValueError("invalid output")
			questions.append(question)
		if len({question["displayId"] for question in questions}) != 4:
			raise ValueError("invalid output")
		return (
			[{"label": value["label"]}],
			questions,
		)
	allowed = (
		({"label"}, "launcher-seeded-enrollment"),
		({"label", "baselineAssignmentId"}, "launcher-baseline-assignment"),
		({"label", "problemId", "versionId"}, "api-retry-corpus-publication"),
		({"label", "courseId", "masteryAssignmentId"}, "api-mastery-assignment"),
		({"label", "courseId", "examAssignmentId"}, "api-exam-assignment"),
	)
	validated: list[dict[str, object]] = []
	for value, (keys, label) in zip(arrangements, allowed, strict=True):
		if not isinstance(value, dict) or set(value) != keys or value.get("label") != label:
			raise ValueError("invalid output")
		checked: dict[str, str] = {}
		for key, identifier in value.items():
			if not isinstance(identifier, str):
				raise ValueError("invalid output")
			if key != "label" and not UUID_TEXT.fullmatch(identifier):
				raise ValueError("invalid output")
			checked[key] = identifier
		validated.append(checked)
	return validated, None


#============================================
def parse_runner_arrangement_output(stdout: str) -> tuple[list[dict[str, object]], list[str] | None]:
	"""Return runner-ready arrangements and human-readable catalog IDs."""
	arrangements, catalog_questions = parse_arrangement_output(stdout)
	if catalog_questions is None:
		return arrangements, None
	display_ids = [question["displayId"] for question in catalog_questions]
	return arrangements, display_ids
