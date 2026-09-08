"""Product HTTP and Student-activity operations for Live Demo Course convergence."""

# Standard Library
import dataclasses
import json
import pathlib
import re
import time

# local repo modules
import local_stack_control.live_demo_course_seed
import local_stack_control.live_demo_gateway
import local_stack_control.models
import local_stack_control.process


GRADING_POLL_ATTEMPTS = 40
GRADING_POLL_SECONDS = 0.25


@dataclasses.dataclass(frozen=True)
class ProductResponse:
	"""One bounded JSON product response and its HTTP status."""

	status: int
	body: object | None


@dataclasses.dataclass(frozen=True)
class ResolvedCourseState:
	"""Product facts resolved during one convergence observation."""

	observed: local_stack_control.live_demo_course_seed.ObservedState
	blueprint_reference: str | None
	blueprint_revision: str | None
	course_reference: str | None
	assignment_reference: str | None
	assignment_edit_number: str | None
	roster_states: dict[str, str]
	attempt_numbers: dict[str, int]
	attempt_completions: dict[str, str]
	terminal_submission_counts: dict[str, int]
	gradebook_points: dict[str, tuple[float, float]]


#============================================
def split_http_result(
	result: local_stack_control.models.CommandResult,
	stage: local_stack_control.live_demo_course_seed.Stage,
	operation: str,
) -> ProductResponse:
	"""Decode curl's bounded body/status output without echoing private content."""
	if not result.ok() or "\n" not in result.stdout:
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} {operation} did not complete"
		)
	body_text, status_text = result.stdout.rsplit("\n", 1)
	if re.fullmatch(r"[1-5][0-9]{2}", status_text) is None:
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} {operation} returned an invalid status"
		)
	status = int(status_text)
	body: object | None = None
	if body_text != "" and 200 <= status < 300:
		try:
			body = json.loads(body_text)
		except json.JSONDecodeError as error:
			raise local_stack_control.models.ControllerError(
				f"live-demo provisioning {stage.value} {operation} returned invalid JSON"
			) from error
	return ProductResponse(status=status, body=body)


#============================================
def request(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	path: str,
	jar: pathlib.Path,
	stage: local_stack_control.live_demo_course_seed.Stage,
	operation: str,
	method: str = "GET",
	body: dict | None = None,
	if_match: str | None = None,
) -> ProductResponse:
	"""Run one fixed-origin product request through the shared safe builder."""
	argv = local_stack_control.live_demo_gateway.demo_request_argv(
		url, path, jar, method, body, if_match
	)
	result = runner.run(argv, cwd=repository_root)
	return split_http_result(result, stage, operation)


#============================================
def require_status(
	response: ProductResponse,
	expected: int,
	stage: local_stack_control.live_demo_course_seed.Stage,
	operation: str,
) -> object | None:
	"""Require one exact product response status without exposing its body."""
	if response.status != expected:
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} {operation} returned HTTP {response.status}"
		)
	return response.body


#============================================
def assignment_path(state: ResolvedCourseState) -> str:
	"""Return the exact public Course/Assignment path after prerequisite checks."""
	if state.course_reference is None or state.assignment_reference is None:
		raise local_stack_control.models.ControllerError(
			"live-demo Student work prerequisite is unavailable"
		)
	return (
		f"/api/course-instances/{state.course_reference}/assignments/"
		f"{state.assignment_reference}"
	)


#============================================
def start_attempt(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	state: ResolvedCourseState,
	stage: local_stack_control.live_demo_course_seed.Stage,
	require_access: bool,
) -> dict:
	"""Start or resume one real Assignment Attempt and validate its presentation."""
	path = assignment_path(state)
	if require_access:
		access = request(
			runner, repository_root, url, path + "/access", jar,
			stage, "access",
		)
		if require_status(access, 200, stage, "access") != {
			"startDecision": "may_start"
		}:
			raise local_stack_control.models.ControllerError(
				f"live-demo provisioning {stage.value} is not startable"
			)
	started = request(
		runner, repository_root, url, path + "/start", jar,
		stage, "apply", "POST", {},
	)
	body = require_status(started, 201, stage, "apply")
	if not isinstance(body, dict) or body.get("assignment") != state.assignment_reference:
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} returned an invalid Assignment Attempt"
		)
	attempt_number = body.get("attemptNumber")
	questions = body.get("questions")
	if (
		not isinstance(attempt_number, int)
		or isinstance(attempt_number, bool)
		or attempt_number < 1
		or not isinstance(body.get("resumed"), bool)
		or not isinstance(questions, list)
		or len(questions) != len(local_stack_control.live_demo_course_seed.SEEDED_QUESTION_IDS)
		or not all(isinstance(question, dict) for question in questions)
	):
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} returned an invalid Assignment Attempt"
		)
	question_ids = tuple(
		question.get("questionRevision", {}).get("questionId")
		if isinstance(question.get("questionRevision"), dict)
		else None
		for question in questions
	)
	if question_ids != local_stack_control.live_demo_course_seed.SEEDED_QUESTION_IDS:
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} returned the wrong issued Questions"
		)
	return body


#============================================
def presented_response(
	presentation: dict,
	recipe: local_stack_control.live_demo_course_seed.SeededPresentedResponse,
) -> dict:
	"""Resolve one pinned response recipe only from an answer-free presentation."""
	revision = presentation.get("questionRevision")
	response_format = presentation.get("response")
	if (
		not isinstance(revision, dict)
		or revision.get("questionId") != recipe.question_id
		or not isinstance(response_format, dict)
		or response_format.get("kind") != recipe.presented_kind
	):
		raise local_stack_control.models.ControllerError(
			"live-demo presented response recipe does not match its Question"
		)
	if recipe.presented_kind == "singleChoice":
		choices = response_format.get("choices")
		index = recipe.selection_index
		if not isinstance(choices, list) or not isinstance(index, int) or not 0 <= index < len(choices):
			raise local_stack_control.models.ControllerError(
				"live-demo single-choice presentation is invalid"
			)
		choice = choices[index]
		if not isinstance(choice, dict) or not isinstance(choice.get("id"), str):
			raise local_stack_control.models.ControllerError(
				"live-demo single-choice presentation is invalid"
			)
		return {"kind": "multipleChoice", "selected": [choice["id"]]}
	if recipe.presented_kind == "numerical":
		if not isinstance(recipe.numeric_value, (int, float)):
			raise local_stack_control.models.ControllerError(
				"live-demo numerical response recipe is invalid"
			)
		return {"kind": "numeric", "value": recipe.numeric_value}
	if recipe.presented_kind == "hotspot":
		surface = response_format.get("surface")
		regions = surface.get("regions") if isinstance(surface, dict) else None
		index = recipe.selection_index
		if not isinstance(regions, list) or not isinstance(index, int) or not 0 <= index < len(regions):
			raise local_stack_control.models.ControllerError(
				"live-demo hotspot presentation is invalid"
			)
		region = regions[index]
		if not isinstance(region, dict) or not isinstance(region.get("id"), str):
			raise local_stack_control.models.ControllerError(
				"live-demo hotspot presentation is invalid"
			)
		return {"kind": "hotspot", "selections": [{"region": region["id"]}]}
	raise local_stack_control.models.ControllerError(
		"live-demo response recipe uses an unsupported presentation kind"
	)


#============================================
def submission_status(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	path: str,
	nonce: str,
) -> str | None:
	"""Read one nonce-bound closed grading state, or absence before submission."""
	stage = local_stack_control.live_demo_course_seed.Stage.WORK
	response = request(
		runner, repository_root, url,
		f"{path}/presentations/{nonce}/submissions", jar,
		stage, "status",
	)
	if response.status == 404:
		return None
	body = require_status(response, 200, stage, "status")
	if (
		not isinstance(body, dict)
		or set(body) != {"presentationNonce", "gradingState"}
		or body.get("presentationNonce") != nonce
		or body.get("gradingState") not in ("pending", "graded", "instructorAttention")
	):
		raise local_stack_control.models.ControllerError(
			"live-demo Question Submission status is invalid"
		)
	return body["gradingState"]


#============================================
def wait_for_graded_submission(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	path: str,
	nonce: str,
) -> None:
	"""Wait for one accepted response to reach its real terminal Grading Result."""
	for _ in range(GRADING_POLL_ATTEMPTS):
		state = submission_status(
			runner, repository_root, url, jar, path, nonce
		)
		if state == "graded":
			return
		if state == "instructorAttention":
			raise local_stack_control.models.ControllerError(
				"live-demo Question Submission requires Instructor attention"
			)
		time.sleep(GRADING_POLL_SECONDS)
	raise local_stack_control.models.ControllerError(
		"live-demo Question Submission grading timed out"
	)


#============================================
def apply_attempts(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jars: dict[str, pathlib.Path],
	state: ResolvedCourseState,
) -> dict[str, int]:
	"""Start or resume Mary and Jack while leaving Avery unstarted."""
	seed = local_stack_control.live_demo_course_seed
	if "averyStudent" in state.attempt_numbers:
		raise local_stack_control.models.ControllerError(
			"live-demo Avery Assignment Attempt cannot be converged"
		)
	attempts: dict[str, int] = {}
	for persona in ("maryStudent", "jackStudent"):
		started = start_attempt(
			runner, repository_root, url, jars[persona], state,
			seed.Stage.ATTEMPTS, True,
		)
		attempts[persona] = started["attemptNumber"]
	access = request(
		runner, repository_root, url, assignment_path(state) + "/access",
		jars["averyStudent"], seed.Stage.ATTEMPTS, "Avery access",
	)
	if require_status(access, 200, seed.Stage.ATTEMPTS, "Avery access") != {
		"startDecision": "may_start"
	}:
		raise local_stack_control.models.ControllerError(
			"live-demo Avery Assignment is not startable"
		)
	return attempts


#============================================
def apply_work(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jars: dict[str, pathlib.Path],
	state: ResolvedCourseState,
) -> None:
	"""Submit and genuinely grade only the declared Mary and Jack responses."""
	seed = local_stack_control.live_demo_course_seed
	response_sets = {item.persona: item.responses for item in seed.SEEDED_STUDENT_RESPONSES}
	work_counts = {item.persona: item.answered_question_count for item in seed.SEEDED_STUDENT_WORK}
	path = assignment_path(state)
	for persona in ("maryStudent", "jackStudent"):
		if persona not in state.attempt_numbers:
			raise local_stack_control.models.ControllerError(
				"live-demo Student work Assignment Attempt is unavailable"
			)
		started = start_attempt(
			runner, repository_root, url, jars[persona], state,
			seed.Stage.WORK, False,
		)
		if started.get("resumed") is not True:
			raise local_stack_control.models.ControllerError(
				"live-demo Student work did not resume its Assignment Attempt"
			)
		questions = started["questions"]
		by_question_id = {
			question["questionRevision"]["questionId"]: question
			for question in questions
		}
		recipes = response_sets[persona]
		if len(recipes) != work_counts[persona]:
			raise local_stack_control.models.ControllerError(
				"live-demo Student response declaration is inconsistent"
			)
		for recipe in recipes:
			presentation = by_question_id.get(recipe.question_id)
			if not isinstance(presentation, dict):
				raise local_stack_control.models.ControllerError(
					"live-demo Student response Question is unavailable"
				)
			nonce = presentation.get("presentationNonce")
			if not isinstance(nonce, str) or re.fullmatch(r"[0-9a-f]{32}", nonce) is None:
				raise local_stack_control.models.ControllerError(
					"live-demo Question Presentation nonce is invalid"
				)
			grading_state = submission_status(
				runner, repository_root, url, jars[persona], path, nonce
			)
			if grading_state is None:
				response = presented_response(presentation, recipe)
				submitted = request(
					runner, repository_root, url,
					f"{path}/presentations/{nonce}/submissions", jars[persona],
					seed.Stage.WORK, "submit", "POST", {"response": response},
				)
				receipt = require_status(submitted, 201, seed.Stage.WORK, "submit")
				if receipt != {"presentationNonce": nonce, "gradingState": "pending"}:
					raise local_stack_control.models.ControllerError(
						"live-demo Question Submission receipt is invalid"
					)
			wait_for_graded_submission(
				runner, repository_root, url, jars[persona], path, nonce
			)


#============================================
def with_attempt_checkpoint(
	state: ResolvedCourseState,
	attempts: dict[str, int],
) -> ResolvedCourseState:
	"""Build the report checkpoint that makes partial Student work resumable."""
	merged = dict(state.attempt_numbers)
	merged.update(attempts)
	completions = dict(state.attempt_completions)
	completions.update({persona: "inProgress" for persona in attempts})
	observed = dataclasses.replace(
		state.observed,
		mary_attempt_exists="maryStudent" in merged,
		jack_attempt_exists="jackStudent" in merged,
		avery_attempt_exists="averyStudent" in merged,
	)
	return dataclasses.replace(
		state,
		observed=observed,
		attempt_numbers=merged,
		attempt_completions=completions,
	)


#============================================
def require_pinned_mary_grade(state: ResolvedCourseState) -> None:
	"""Require the evidence-selected deterministic mixed grade once work is complete."""
	if not state.observed.work_complete:
		return
	seed = local_stack_control.live_demo_course_seed
	if state.gradebook_points.get("maryStudent") != (
		seed.SEEDED_MARY_POINTS_EARNED,
		seed.SEEDED_MARY_POINTS_POSSIBLE,
	):
		raise local_stack_control.models.ControllerError(
			"live-demo Mary grading outcome differs from the pinned baseline"
		)
