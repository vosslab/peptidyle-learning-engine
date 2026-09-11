"""Public-presentation response projection for Live Demo Student work."""

# Standard Library
import pathlib

# third-party modules
import pytest

# local repo modules
import local_stack_control.live_demo_course_activity
import local_stack_control.live_demo_course_provision
import local_stack_control.live_demo_course_seed


#============================================
def test_pinned_recipes_resolve_only_presented_response_references() -> None:
	"""Choice and hotspot recipes become exact public Student Response shapes."""
	recipes = {
		response.question_id: response
		for response in local_stack_control.live_demo_course_seed.SEEDED_STUDENT_RESPONSES[0].responses
	}
	single_choice = {
		"questionRevision": {"questionId": "PNE-0001"},
		"response": {
			"kind": "singleChoice",
			"choices": [{"id": "a001"}, {"id": "a002"}],
		},
	}
	numerical = {
		"questionRevision": {"questionId": "PNE-0003"},
		"response": {"kind": "numerical"},
	}
	hotspot = {
		"questionRevision": {"questionId": "PNE-0004"},
		"response": {
			"kind": "hotspot",
			"surface": {"regions": [{"id": "b001"}, {"id": "b002"}]},
		},
	}

	assert local_stack_control.live_demo_course_provision._presented_response(
		single_choice, recipes["PNE-0001"]
	) == {"kind": "multipleChoice", "selected": ["a001"]}
	assert local_stack_control.live_demo_course_provision._presented_response(
		numerical, recipes["PNE-0003"]
	) == {"kind": "numeric", "value": 1}
	assert local_stack_control.live_demo_course_provision._presented_response(
		hotspot, recipes["PNE-0004"]
	) == {"kind": "hotspot", "selections": [{"region": "b002"}]}


#============================================
def test_saved_response_counts_accept_expanded_access_projection(

	monkeypatch: pytest.MonkeyPatch,
	tmp_path: pathlib.Path,
) -> None:
	"""Open work reads its validated Attempt despite additional access facts."""
	provision = local_stack_control.live_demo_course_provision
	activity = local_stack_control.live_demo_course_activity
	access = activity.ProductResponse(
		status=200,
		body={
			"startDecision": "may_resume",
			"activeAssignmentAttempt": "R-1",
			"assignment": {"reference": "A-1"},
			"attemptHistory": [{"reference": "R-1"}],
		},
		failure_detail=None,
	)
	monkeypatch.setattr(provision, "_request", lambda *args: access)
	monkeypatch.setattr(
		activity,
		"progress",
		lambda *args: {"positions": [{"responseState": "saved"}]},
	)

	counts = provision._saved_response_counts(
		object(), tmp_path, "https://demo.invalid", {"jackStudent": tmp_path},
		"C-1", "A-1", {"jackStudent": "inProgress"},
	)

	assert counts["jackStudent"] == 1


#============================================
def test_seed_report_separates_attempt_submission_from_graded_questions() -> None:
	"""The report keeps Mary's one finalization distinct from four graded Questions."""
	seed = local_stack_control.live_demo_course_seed
	state = local_stack_control.live_demo_course_provision.ResolvedCourseState(
		observed=seed.ObservedState(),
		blueprint_reference="BP-1",
		blueprint_revision="1",
		course_reference="C-1",
		assignment_reference="A-1",
		assignment_edit_number="1",
		roster_states={entry.roster_id: "activeStudent" for entry in seed.SEEDED_ROSTER_ENTRIES},
		attempt_numbers={"maryStudent": 1, "jackStudent": 1},
		attempt_completions={"maryStudent": "completed", "jackStudent": "inProgress"},
		graded_question_counts={"maryStudent": 4, "jackStudent": 0, "averyStudent": 0},
		saved_response_counts={"maryStudent": 0, "jackStudent": 2, "averyStudent": 0},
		gradebook_points={"maryStudent": (2.0, 4.0)},
	)

	students = local_stack_control.live_demo_course_provision._report_value(state)["students"]
	mary = next(row for row in students if row["persona"] == "maryStudent")

	assert mary["assignment_submission_count"] == 1
	assert mary["graded_question_count"] == 4
	assert mary["grading_state"]["graded_question_count"] == 4
	assert "submission_count" not in mary
