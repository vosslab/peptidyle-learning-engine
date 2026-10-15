"""Small editable starting Blueprints assembled from BPW topic order."""

# Standard Library
import re
import copy

COURSE_THEMES = {"genetics": "ocean", "biochemistry": "tundra",
	"biotechnology": "magma", "biostatistics": "grass"}
DEFAULTS = {
	"assessment_attempt_time_limit_seconds": None, "assessment_attempt_limit": None,
	"late_work_rule": "accept", "activity_rules": {"partialCreditEnabled": True,
		"questionVariationRule": "newVariation", "assessmentQuestionOrderRule": "authoredOrder"},
	"student_feedback_release_rule": {"question_answer": "after_submit",
		"question_answer_explanation": "after_submit", "class_statistics": "never",
		"hints": "during_attempt", "worked_solutions": "after_submit"},
}


#============================================
def native_points(document: dict) -> str:
	"""Read QPM's ready metadata for editable per-choice starting points."""
	response = document["response"]
	kind = response["kind"]
	points = 1.0
	if kind == "matching":
		points = 0.5 * len(response["choices"])
	elif kind == "ordering":
		points = 0.5 * len(response["items"])
	elif kind == "multiFillIn":
		points = float(len(response["blanks"]))
	return str(points)


#============================================
def entry(reference: dict, pool: bool, points: str, bonus: bool) -> dict:
	"""Create one Fixed or random single-item Pool entry with editable defaults."""
	result = {"kind": "pool" if pool else "fixed",
		"scoring_rule": "extraCredit" if bonus else "normal",
		"question_attempt_limit": {"maxAttempts": None},
		"question_attempt_time_limit": {"kind": "unlimited"}}
	if pool:
		result.update({"question_pool_id": reference["questionPoolId"],
			"selection_count": 1, "points_per_item": points})
	else:
		result.update({"published_question_revision_tuple": reference, "points_possible": points})
	return result


#============================================
def is_bonus(title: str) -> bool:
	"""Preserve existing explicit Hard or Bonus distinctions as Bonus content."""
	bonus = re.search(r"\b(?:hard|bonus)\b", title, re.IGNORECASE) is not None
	return bonus


#============================================
def module(topic: dict, entries: list[dict]) -> dict:
	"""Keep related source order and deliver fewer than ten Questions per Assessment."""
	assessments = []
	for bonus in (False, True):
		selected = [row for row in entries if (row["scoring_rule"] == "extraCredit") == bonus]
		chunks = [selected[index:index + 9] for index in range(0, len(selected), 9)]
		for index, chunk in enumerate(chunks):
			title = topic["title"] + (" Bonus" if bonus else "")
			if len(chunks) > 1:
				# There are fewer than 26 parts in the current catalog.
				title += " " + chr(ord("A") + index)
			assessment_type = "bonus_assignment" if bonus else "regular_assignment"
			assessments.append({"assessment_type": assessment_type, "title": title,
				"instructions": topic["description"], "entries": chunk,
				"defaults": copy.deepcopy(DEFAULTS)})
	result = {"label": topic["title"], "assessments": assessments}
	return result
