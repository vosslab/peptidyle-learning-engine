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


# The native PLE worker deliberately holds a three-second post-lease handoff
# window.  Mary's fixed baseline has four independently graded Questions, so
# its one whole-Attempt finalization can take just over twelve seconds even
# when the worker is healthy.  Keep the bounded controller wait long enough
# to observe that declared baseline rather than tearing down a healthy stack.
GRADING_POLL_ATTEMPTS = 64
GRADING_POLL_SECONDS = 0.25
MAX_FAILURE_DETAIL_CHARS = 240
SAFE_FAILURE_DETAIL = re.compile(r"[A-Za-z0-9][A-Za-z0-9 .,:;()/_\-\[\]]*")
SENSITIVE_FAILURE_VALUE = re.compile(
	r"(?i)\b(authorization|bearer|cookie|credential|password|secret|session(?:[_-]?id)?|token)"
	r"\b(?:\s*[:=]\s*|\s+)[^\s,;\]}\"']+"
)


@dataclasses.dataclass(frozen=True)
class ProductResponse:
	"""One bounded JSON product response and its HTTP status."""

	status: int
	body: object | None
	failure_detail: str | None


#============================================
def safe_failure_detail(body_text: str) -> str | None:
	"""Return a short redacted product-error detail suitable for controller output."""
	value: object = body_text
	try:
		parsed = json.loads(body_text)
	except json.JSONDecodeError:
		pass
	else:
		if not isinstance(parsed, dict):
			return None
		for field in ("error", "code", "message"):
			candidate = parsed.get(field)
			if isinstance(candidate, str):
				value = candidate
				break
		else:
			return None
	if not isinstance(value, str):
		return None
	# Preserve one operator-useful phrase while removing credential-shaped values.
	redacted = SENSITIVE_FAILURE_VALUE.sub(r"\1 [redacted]", value)
	normalized = " ".join(redacted.split())
	if SAFE_FAILURE_DETAIL.fullmatch(normalized) is None:
		return None
	result = normalized[:MAX_FAILURE_DETAIL_CHARS]
	return result


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
	graded_question_counts: dict[str, int]
	saved_response_counts: dict[str, int]
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
	failure_detail: str | None = None
	if body_text != "" and 200 <= status < 300:
		try:
			body = json.loads(body_text)
		except json.JSONDecodeError as error:
			raise local_stack_control.models.ControllerError(
				f"live-demo provisioning {stage.value} {operation} returned invalid JSON"
			) from error
	if body_text != "" and not 200 <= status < 300:
		failure_detail = safe_failure_detail(body_text)
	return ProductResponse(status=status, body=body, failure_detail=failure_detail)


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
		detail = ""
		if response.failure_detail is not None:
			detail = f" ({response.failure_detail})"
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} {operation} returned HTTP {response.status}{detail}"
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
	"""Start or resume one real Assignment Attempt through its public route."""
	path = assignment_path(state)
	if require_access:
		access = request(
			runner, repository_root, url, path + "/access", jar,
			stage, "access",
		)
		if require_startable_access(require_status(access, 200, stage, "access")) is False:
			raise local_stack_control.models.ControllerError(
				f"live-demo provisioning {stage.value} is not startable"
			)
	started = request(
		runner, repository_root, url, path + "/start", jar,
		stage, "apply", "POST", {},
	)
	body = require_status(started, 201, stage, "apply")
	if (
		not isinstance(body, dict)
		or body.get("assignment") != state.assignment_reference
		or assignment_attempt_reference(body.get("assignmentAttempt")) is None
	):
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} returned an invalid Assignment Attempt"
		)
	attempt_number = body.get("attemptNumber")
	if (
		not isinstance(attempt_number, int)
		or isinstance(attempt_number, bool)
		or attempt_number < 1
		or not isinstance(body.get("resumed"), bool)
	):
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} returned an invalid Assignment Attempt"
		)
	return body


#============================================
def assignment_attempt_reference(value: object) -> str | None:
	"""Return one canonical public Assignment Attempt reference, if present."""
	if isinstance(value, str) and re.fullmatch(r"R-[1-9][0-9]{0,9}", value):
		return value
	return None


#============================================
def require_startable_access(value: object) -> bool:
	"""Recognize the exact answer-free access projection for a new Attempt."""
	return value == {
		"startDecision": "may_start",
		"activeAssignmentAttempt": None,
	}


#============================================
def attempt_path(reference: str) -> str:
	"""Return the fixed public route base for one validated Assignment Attempt."""
	if assignment_attempt_reference(reference) is None:
		raise local_stack_control.models.ControllerError(
			"live-demo Assignment Attempt reference is invalid"
		)
	path = f"/api/assignment-attempts/{reference}"
	return path


#============================================
def progress(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	reference: str,
	stage: local_stack_control.live_demo_course_seed.Stage,
	operation: str,
) -> dict:
	"""Read and validate the answer-free position state for one owned Attempt."""
	path = attempt_path(reference) + "/student-progress"
	body = require_status(
		request(runner, repository_root, url, path, jar, stage, operation),
		200, stage, operation,
	)
	if not isinstance(body, dict) or set(body) != {
		"assignmentAttempt", "questionCount", "recommendedPosition", "positions",
	} or body.get("assignmentAttempt") != reference:
		raise local_stack_control.models.ControllerError(
			"live-demo Student Attempt progress is invalid"
		)
	count = body.get("questionCount")
	positions = body.get("positions")
	recommended = body.get("recommendedPosition")
	if (
		not isinstance(count, int)
		or isinstance(count, bool)
		or count != len(local_stack_control.live_demo_course_seed.SEEDED_QUESTION_IDS)
		or not isinstance(positions, list)
		or len(positions) != count
		or (
			recommended is not None
			and (
				not isinstance(recommended, int)
				or isinstance(recommended, bool)
				or not 1 <= recommended <= count
			)
		)
	):
		raise local_stack_control.models.ControllerError(
			"live-demo Student Attempt progress is invalid"
		)
	for expected_position, position in enumerate(positions, start=1):
		if not isinstance(position, dict) or position != {
			"position": expected_position,
			"responseState": position.get("responseState"),
		} or position["responseState"] not in ("unanswered", "saved", "submitted", "closed"):
			raise local_stack_control.models.ControllerError(
				"live-demo Student Attempt progress is invalid"
			)
	return body


#============================================
def selected_presentation(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	reference: str,
	position: int,
) -> dict:
	"""Read one authorized answer-free presentation by its fixed position."""
	stage = local_stack_control.live_demo_course_seed.Stage.WORK
	body = require_status(
		request(
			runner, repository_root, url,
			f"{attempt_path(reference)}/student-question?position={position}", jar,
			stage, "presentation",
		),
		200, stage, "presentation",
	)
	if (
		not isinstance(body, dict)
		or set(body) != {"position", "presentation", "savedResponse"}
		or body.get("position") != position
		or not isinstance(body.get("presentation"), dict)
		or body.get("savedResponse") is not None and not isinstance(body.get("savedResponse"), dict)
	):
		raise local_stack_control.models.ControllerError(
			"live-demo Student Attempt presentation is invalid"
		)
	return body


#============================================
def presented_response(
	presentation: dict,
	recipe: local_stack_control.live_demo_course_seed.SeededPresentedResponse,
) -> dict:
	"""Resolve one pinned response recipe only from an answer-free presentation."""
	response_format = presentation.get("response")
	if (
		not isinstance(response_format, dict)
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
	if require_startable_access(
		require_status(access, 200, seed.Stage.ATTEMPTS, "Avery access")
	) is False:
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
	"""Save declared work, finalize Mary once, and retain Jack's open work."""
	seed = local_stack_control.live_demo_course_seed
	response_sets = {item.persona: item.responses for item in seed.SEEDED_STUDENT_RESPONSES}
	work_counts = {item.persona: item.answered_question_count for item in seed.SEEDED_STUDENT_WORK}
	for persona in ("maryStudent", "jackStudent"):
		if persona not in state.attempt_numbers:
			raise local_stack_control.models.ControllerError(
				"live-demo Student work Assignment Attempt is unavailable"
			)
		if state.attempt_completions.get(persona) == "completed":
			if (
				persona == "maryStudent"
				and state.observed.mary_graded_question_count < len(seed.SEEDED_QUESTION_IDS)
			):
				wait_for_mary_grade(
					runner, repository_root, url, jars["elenaInstructor"], state
				)
			continue
		started = start_attempt(
			runner, repository_root, url, jars[persona], state,
			seed.Stage.WORK, False,
		)
		if started.get("resumed") is not True:
			raise local_stack_control.models.ControllerError(
				"live-demo Student work did not resume its Assignment Attempt"
			)
		attempt = assignment_attempt_reference(started.get("assignmentAttempt"))
		if attempt is None:
			raise local_stack_control.models.ControllerError(
				"live-demo Student work Assignment Attempt is invalid"
			)
		attempt_progress = progress(
			runner, repository_root, url, jars[persona], attempt,
			seed.Stage.WORK, "progress",
		)
		recipes = response_sets[persona]
		if len(recipes) != work_counts[persona]:
			raise local_stack_control.models.ControllerError(
				"live-demo Student response declaration is inconsistent"
			)
		for position, recipe in enumerate(recipes, start=1):
			selected = selected_presentation(
				runner, repository_root, url, jars[persona], attempt, position
			)
			response = presented_response(selected["presentation"], recipe)
			saved = request(
				runner, repository_root, url,
				f"{attempt_path(attempt)}/responses/{position}", jars[persona],
				seed.Stage.WORK, "save", "PUT", {"response": response},
			)
			receipt = require_status(saved, 200, seed.Stage.WORK, "save")
			if receipt != {
				"assignmentAttempt": attempt,
				"position": position,
				"responseState": "saved",
			}:
				raise local_stack_control.models.ControllerError(
					"live-demo Student response save receipt is invalid"
				)
		if persona == "maryStudent":
			finalized = request(
				runner, repository_root, url, f"{attempt_path(attempt)}/submission",
				jars[persona], seed.Stage.WORK, "finalize", "POST", {},
			)
			receipt = require_status(finalized, 200, seed.Stage.WORK, "finalize")
			if receipt != {"assignmentAttempt": attempt, "submissionState": "submitted"}:
				raise local_stack_control.models.ControllerError(
					"live-demo Assignment Attempt submission receipt is invalid"
				)
			wait_for_mary_grade(runner, repository_root, url, jars["elenaInstructor"], state)
		elif saved_count(attempt_progress) > len(recipes):
				raise local_stack_control.models.ControllerError(
					"live-demo Student work has unexpected saved responses"
				)


#============================================
def saved_count(value: dict) -> int:
	"""Count only the truthful saved positions in one validated progress response."""
	positions = value["positions"]
	return sum(position["responseState"] == "saved" for position in positions)


#============================================
def wait_for_mary_grade(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	state: ResolvedCourseState,
) -> None:
	"""Wait for Mary's real whole-Attempt grading result through Gradebook HTTP."""
	if state.course_reference is None or state.assignment_reference is None:
		raise local_stack_control.models.ControllerError(
			"live-demo Gradebook prerequisite is unavailable"
		)
	seed = local_stack_control.live_demo_course_seed
	for _ in range(GRADING_POLL_ATTEMPTS):
		body = require_status(
			request(
				runner, repository_root, url,
				f"/api/course-instances/{state.course_reference}/gradebook", jar,
				seed.Stage.WORK, "gradebook",
			),
			200, seed.Stage.WORK, "gradebook",
		)
		rows = body.get("studentWork") if isinstance(body, dict) else None
		if not isinstance(rows, list):
			raise local_stack_control.models.ControllerError(
				"live-demo Gradebook projection is invalid"
			)
		matches = [
			row for row in rows
			if isinstance(row, dict)
			and row.get("rosterId") == "BIO301-MARY"
			and row.get("assignmentReference") == state.assignment_reference
		]
		if len(matches) != 1:
			raise local_stack_control.models.ControllerError(
				"live-demo Mary Gradebook projection is invalid"
			)
		row = matches[0]
		if (
			row.get("assignmentAttemptCompletion") == "completed"
			and row.get("gradedQuestionCount") == len(seed.SEEDED_QUESTION_IDS)
			and row.get("pointsEarned") == seed.SEEDED_MARY_POINTS_EARNED
			and row.get("pointsPossible") == seed.SEEDED_MARY_POINTS_POSSIBLE
		):
			return
		time.sleep(GRADING_POLL_SECONDS)
	raise local_stack_control.models.ControllerError(
		"live-demo Mary Assignment Attempt grading timed out"
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
