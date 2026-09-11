"""Bounded failure diagnostics for Live Demo product provisioning."""

# Standard Library
import json
import pathlib

# third-party modules
import pytest

import local_stack_control.live_demo_course_activity
import local_stack_control.live_demo_course_seed
import local_stack_control.models


#============================================
def result(stdout: str) -> local_stack_control.models.CommandResult:
	"""Build one completed curl-shaped product result."""
	return local_stack_control.models.CommandResult(
		argv=("curl",), returncode=0, stdout=stdout, stderr=""
	)


#============================================
def test_non_success_response_emits_bounded_redacted_error_detail() -> None:
	"""A provisioning failure retains its safe product cause without credentials."""
	password = "correct-horse-battery-staple"
	cookie = "ple_session=private-cookie-value"
	token = "token private-token-value"
	body = json.dumps({
		"error": (
			"Assignment Workspace unavailable; "
			f"password={password}; cookie={cookie}; {token}"
		),
	})
	response = local_stack_control.live_demo_course_activity.split_http_result(
		result(body + "\n503"),
		local_stack_control.live_demo_course_seed.Stage.ASSIGNMENT,
		"apply",
	)

	with pytest.raises(local_stack_control.models.ControllerError) as raised:
		local_stack_control.live_demo_course_activity.require_status(
			response,
			201,
			local_stack_control.live_demo_course_seed.Stage.ASSIGNMENT,
			"apply",
		)

	message = str(raised.value)
	assert "HTTP 503" in message
	assert "Assignment Workspace unavailable" in message
	assert "[redacted]" in message
	assert response.failure_detail is not None
	assert len(response.failure_detail) <= (
		local_stack_control.live_demo_course_activity.MAX_FAILURE_DETAIL_CHARS
	)
	assert password not in message
	assert cookie not in message
	assert token not in message


#============================================
def test_success_response_keeps_its_existing_json_contract() -> None:
	"""Successful provisioning keeps parsing and output unchanged."""
	payload = '{"assignment":"A-1","title":"Protein Structure"}\n201'

	response = local_stack_control.live_demo_course_activity.split_http_result(
		result(payload),
		local_stack_control.live_demo_course_seed.Stage.ASSIGNMENT,
		"apply",
	)

	assert response.status == 201
	assert response.body == {"assignment": "A-1", "title": "Protein Structure"}
	assert response.failure_detail is None
	assert local_stack_control.live_demo_course_activity.require_status(
		response,
		201,
		local_stack_control.live_demo_course_seed.Stage.ASSIGNMENT,
		"apply",
	) == {"assignment": "A-1", "title": "Protein Structure"}


#============================================
def test_startable_access_requires_no_active_attempt() -> None:
	"""Only required new-Attempt facts permit a seeded start."""
	activity = local_stack_control.live_demo_course_activity

	assert activity.require_startable_access({
		"startDecision": "may_start",
		"activeAssignmentAttempt": None,
		"title": "Peptide Structure Practice",
		"questionCount": 4,
		"pointsPossible": 8,
		"timeLimitSeconds": 900,
		"previousAttempts": [],
	})
	assert activity.require_startable_access({
		"startDecision": "may_start",
		"activeAssignmentAttempt": "R-1",
	}) is False
	assert activity.require_startable_access({
		"startDecision": "closed",
		"activeAssignmentAttempt": None,
	}) is False
	assert activity.require_startable_access({"activeAssignmentAttempt": None}) is False
	assert activity.require_startable_access({"startDecision": "may_start"}) is False


#============================================
def test_saved_count_reads_only_answer_free_saved_positions() -> None:
	"""The seed report distinguishes Jack's saved work from a submission."""
	progress = {
		"positions": [
			{"position": 1, "responseState": "saved"},
			{"position": 2, "responseState": "unanswered"},
			{"position": 3, "responseState": "submitted"},
		]
	}

	assert local_stack_control.live_demo_course_activity.saved_count(progress) == 1


#============================================
def test_mary_grade_wait_covers_the_declared_whole_attempt() -> None:
	"""The bounded seed wait accommodates four serialized native-PLE grades."""
	activity = local_stack_control.live_demo_course_activity
	assert (
		activity.GRADING_POLL_ATTEMPTS * activity.GRADING_POLL_SECONDS
		>= len(local_stack_control.live_demo_course_seed.SEEDED_QUESTION_IDS) * 3
	)


#============================================
def test_completed_mary_pending_grade_replay_waits_without_restarting(

	monkeypatch: pytest.MonkeyPatch,
	tmp_path: pathlib.Path,
) -> None:
	"""A completed Mary waits for grading during replay without another start request."""
	activity = local_stack_control.live_demo_course_activity
	state = activity.ResolvedCourseState(
		observed=local_stack_control.live_demo_course_seed.ObservedState(
			mary_attempt_exists=True,
			jack_attempt_exists=True,
			mary_attempt_completed=True,
		),
		blueprint_reference=None,
		blueprint_revision=None,
		course_reference="C-1",
		assignment_reference="A-1",
		assignment_edit_number=None,
		roster_states={},
		attempt_numbers={"maryStudent": 1, "jackStudent": 1},
		attempt_completions={"maryStudent": "completed", "jackStudent": "completed"},
		graded_question_counts={"maryStudent": 0, "jackStudent": 0, "averyStudent": 0},
		saved_response_counts={"maryStudent": 0, "jackStudent": 2, "averyStudent": 0},
		gradebook_points={},
	)
	calls: list[object] = []
	monkeypatch.setattr(activity, "wait_for_mary_grade", lambda *args: calls.append(args))

	activity.apply_work(
		object(),
		tmp_path,
		"https://demo.invalid",
		{
			"elenaInstructor": tmp_path,
			"maryStudent": tmp_path,
			"jackStudent": tmp_path,
		},
		state,
	)

	assert len(calls) == 1
