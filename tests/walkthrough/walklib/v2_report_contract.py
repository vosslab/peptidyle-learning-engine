"""Strict public schema-v2 report boundary for the UI walkthrough runner."""

import json


MAX_JOURNEY_ELAPSED_MS = 30 * 60 * 1000
MAX_VISIBLE_OUTCOME_OUTPUT_BYTES = 4096
UINT32_MAX = 4_294_967_295
EXPECTED_JOURNEYS = (
	("J11", ["visible_course_created", "visible_course_opened"]),
	("J12", ["visible_local_student_active"]),
	(
		"J13",
		[
			"visible_assignment_created",
			"visible_catalog_problem_selected",
			"visible_four_question_chapter_one_selection",
			"visible_mastery_policy",
		],
	),
	("J1", ["visible_feedback", "visible_response", "visible_retry", "visible_submit"]),
	("J2", ["visible_completion", "visible_feedback", "visible_fresh_practice", "visible_submit"]),
	("J3", ["visible_controls_cleared", "visible_leave", "visible_resume", "visible_start"]),
	(
		"J4",
		["visible_back_action", "visible_completion", "visible_controls_cleared", "visible_submit"],
	),
	("J5", ["visible_gradebook", "visible_score_summary", "visible_two_run_history"]),
	(
		"J8",
		[
			"visible_instructor_gradebook",
			"visible_learner_completion",
			"visible_shared_assignment",
		],
	),
)


def parse_public_v2_report(stdout: str, master_seed: int) -> dict[str, object]:
	"""Return the canonical v2 report or raise ValueError for any private or malformed output."""
	try:
		encoded = stdout.encode("ascii")
		payload = json.loads(stdout)
	except (UnicodeEncodeError, json.JSONDecodeError) as error:
		raise ValueError("invalid output") from error
	if (
		len(encoded) > MAX_VISIBLE_OUTCOME_OUTPUT_BYTES
		or not stdout.endswith("\n")
		or stdout.count("\n") != 1
		or "\r" in stdout
		or stdout[0].isspace()
		or not isinstance(payload, dict)
		or set(payload)
		!= {"schemaVersion", "status", "masterSeed", "stage", "elapsedMs", "arrangements", "journeys"}
		or payload["schemaVersion"] != 2
		or payload["status"] != "PASS"
		or not isinstance(payload["masterSeed"], int)
		or isinstance(payload["masterSeed"], bool)
		or payload["masterSeed"] < 0
		or payload["masterSeed"] > UINT32_MAX
		or payload["masterSeed"] != master_seed
		or payload["stage"] != "complete"
		or not isinstance(payload["elapsedMs"], int)
		or isinstance(payload["elapsedMs"], bool)
		or payload["elapsedMs"] < 0
		or payload["elapsedMs"] > len(EXPECTED_JOURNEYS) * MAX_JOURNEY_ELAPSED_MS
		or payload["arrangements"] != [{"label": "launcher-chapter-one-genetics"}]
		or not isinstance(payload["journeys"], list)
		or len(payload["journeys"]) != len(EXPECTED_JOURNEYS)
	):
		raise ValueError("invalid output")
	for journey, (expected_journey, expected_codes) in zip(
		payload["journeys"], EXPECTED_JOURNEYS, strict=True
	):
		if (
			not isinstance(journey, dict)
			or set(journey)
			!= {"journey", "status", "elapsedMs", "visibleOutcomeCodes", "diagnostics"}
			or journey["journey"] != expected_journey
			or journey["status"] != "PASS"
			or not isinstance(journey["elapsedMs"], int)
			or isinstance(journey["elapsedMs"], bool)
			or journey["elapsedMs"] < 0
			or journey["elapsedMs"] > MAX_JOURNEY_ELAPSED_MS
			or journey["visibleOutcomeCodes"] != expected_codes
			or journey["diagnostics"] != []
		):
			raise ValueError("invalid output")
	if payload["elapsedMs"] != sum(row["elapsedMs"] for row in payload["journeys"]):
		raise ValueError("invalid output")
	if stdout != json.dumps(payload, separators=(",", ":")) + "\n":
		raise ValueError("invalid output")
	return payload
