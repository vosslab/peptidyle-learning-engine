"""Strict public arrangement-output boundary for the UI walkthrough runner."""

import json
import re


MAX_ARRANGEMENT_OUTPUT_BYTES = 2048
UUID_TEXT = re.compile(
	"^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
	re.IGNORECASE,
)
CATALOG_SEARCH_TITLE = re.compile(r"^Fake amino acid question [0-9a-f]{12}$")


def parse_arrangement_output(stdout: str) -> tuple[list[dict[str, str]], str | None]:
	"""Return strict public arrangement references and an optional instructor title."""
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
			or set(value) != {"label", "problemId", "versionId", "catalogSearchTitle"}
			or value.get("label") != "api-retry-corpus-publication"
			or not isinstance(value["problemId"], str)
			or not isinstance(value["versionId"], str)
			or not isinstance(value["catalogSearchTitle"], str)
			or not UUID_TEXT.fullmatch(value["problemId"])
			or not UUID_TEXT.fullmatch(value["versionId"])
			or not CATALOG_SEARCH_TITLE.fullmatch(value["catalogSearchTitle"])
		):
			raise ValueError("invalid output")
		return (
			[
				{
					"label": value["label"],
					"problemId": value["problemId"],
					"versionId": value["versionId"],
				}
			],
			value["catalogSearchTitle"],
		)
	allowed = (
		({"label"}, "launcher-seeded-enrollment"),
		({"label", "baselineAssignmentId"}, "launcher-baseline-assignment"),
		({"label", "problemId", "versionId"}, "api-retry-corpus-publication"),
		({"label", "courseId", "masteryAssignmentId"}, "api-mastery-assignment"),
		({"label", "courseId", "examAssignmentId"}, "api-exam-assignment"),
	)
	validated: list[dict[str, str]] = []
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
