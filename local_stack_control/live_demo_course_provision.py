"""Convergent product-HTTP provisioning for the Live Demo teaching baseline."""

# Standard Library
import dataclasses
import json
import os
import pathlib
import re
import tempfile
import time

# local repo modules
import local_stack_control.live_demo_course_seed
import local_stack_control.live_demo_gateway
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process


REPORT_NAME = "live_demo_course_report.json"
MAX_REPORT_BYTES = 32_768
REFERENCE_PATTERNS = {
	"blueprint_reference": re.compile(r"BP-[1-9][0-9]{0,9}"),
	"course_reference": re.compile(r"C-[1-9][0-9]{0,9}"),
	"assignment_reference": re.compile(r"A-[1-9][0-9]{0,9}"),
}
SUPPORTED_STAGES = (
	local_stack_control.live_demo_course_seed.Stage.BLUEPRINT,
	local_stack_control.live_demo_course_seed.Stage.COURSE,
	local_stack_control.live_demo_course_seed.Stage.ROSTER,
	local_stack_control.live_demo_course_seed.Stage.CLAIMS,
	local_stack_control.live_demo_course_seed.Stage.ASSIGNMENT,
	local_stack_control.live_demo_course_seed.Stage.SELECTION,
	local_stack_control.live_demo_course_seed.Stage.RELEASE,
	local_stack_control.live_demo_course_seed.Stage.ATTEMPTS,
	local_stack_control.live_demo_course_seed.Stage.WORK,
)
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


@dataclasses.dataclass(frozen=True)
class ProvisionResult:
	"""One safe provisioning result for the command or lifecycle caller."""

	state: ResolvedCourseState
	planned_stages: tuple[local_stack_control.live_demo_course_seed.Stage, ...]
	report_path: pathlib.Path


#============================================
def report_path(workspace: pathlib.Path) -> pathlib.Path:
	"""Return the fixed private baseline report path."""
	return workspace / REPORT_NAME


#============================================
def _split_http_result(
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
def _request(
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
	return _split_http_result(result, stage, operation)


#============================================
def _require_status(
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
def _create_session_jars(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	workspace: pathlib.Path,
	url: str,
) -> dict[str, pathlib.Path]:
	"""Issue ordinary sessions into transient mode-0600 persona cookie jars."""
	jars: dict[str, pathlib.Path] = {}
	try:
		for persona in local_stack_control.live_demo_gateway.SEEDED_DEMO_PERSONAS:
			file_descriptor, name = tempfile.mkstemp(
				prefix=f".live-demo-{persona}-", suffix=".cookies", dir=workspace
			)
			try:
				os.fchmod(file_descriptor, 0o600)
			finally:
				os.close(file_descriptor)
			path = pathlib.Path(name)
			jars[persona] = path
			argv = local_stack_control.live_demo_gateway.persona_session_argv(
				url, persona, path
			)
			result = runner.run(argv, cwd=repository_root)
			response = _split_http_result(
				result,
				local_stack_control.live_demo_course_seed.Stage.BLUEPRINT,
				"session",
			)
			_require_status(
				response,
				200,
				local_stack_control.live_demo_course_seed.Stage.BLUEPRINT,
				"session",
			)
			os.chmod(path, 0o600)
		return jars
	except BaseException:
		_remove_session_jars(jars)
		raise


#============================================
def _remove_session_jars(jars: dict[str, pathlib.Path]) -> None:
	"""Remove only transient cookie jars allocated by this invocation."""
	for path in jars.values():
		try:
			path.unlink()
		except FileNotFoundError:
			pass


#============================================
def _stored_report(workspace: pathlib.Path) -> dict:
	"""Read the prior bounded private baseline report when it exists."""
	path = report_path(workspace)
	if not path.exists():
		return {}
	content = local_stack_control.private_files.read_current_user_private_file(
		path, MAX_REPORT_BYTES
	)
	try:
		value = json.loads(content)
	except (UnicodeDecodeError, json.JSONDecodeError) as error:
		raise local_stack_control.models.ControllerError(
			"live-demo course report is invalid"
		) from error
	if not isinstance(value, dict):
		raise local_stack_control.models.ControllerError("live-demo course report is invalid")
	return value


#============================================
def _stored_references(value: dict) -> dict[str, str]:
	"""Read valid machine references from one prior baseline report."""
	resolved: dict[str, str] = {}
	for name, pattern in REFERENCE_PATTERNS.items():
		reference = value.get(name)
		if reference is None:
			continue
		if not isinstance(reference, str) or pattern.fullmatch(reference) is None:
			raise local_stack_control.models.ControllerError(
				"live-demo course report contains an invalid reference"
			)
		resolved[name] = reference
	return resolved


#============================================
def _items(value: object | None, label: str, wrapped: bool = True) -> list[dict]:
	"""Require one closed list-bearing response at the controller boundary."""
	items_value = value.get("items") if wrapped and isinstance(value, dict) else value
	if not isinstance(items_value, list) or not all(
		isinstance(item, dict) for item in items_value
	):
		raise local_stack_control.models.ControllerError(
			f"live-demo {label} projection is invalid"
		)
	return items_value


#============================================
def _resolve_named_reference(
	items: list[dict],
	stored_reference: str | None,
	title: str,
	reference_pattern: re.Pattern[str],
	label: str,
) -> dict | None:
	"""Resolve a prior machine reference first, then one unambiguous exact title."""
	valid = [
		item
		for item in items
		if isinstance(item.get("reference"), str)
		and reference_pattern.fullmatch(item["reference"]) is not None
	]
	if stored_reference is not None:
		for item in valid:
			if item["reference"] == stored_reference:
				return item
	matches = [item for item in valid if item.get("title") == title]
	if len(matches) > 1:
		raise local_stack_control.models.ControllerError(
			f"live-demo {label} identity is ambiguous"
		)
	return matches[0] if matches else None


#============================================
def _gradebook_counts(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	course_reference: str | None,
	assignment_reference: str | None,
) -> tuple[dict[str, int], dict[str, tuple[float, float]]]:
	"""Read actual terminal Grading Result facts from the Instructor Gradebook."""
	seed = local_stack_control.live_demo_course_seed
	counts = {entry.persona: 0 for entry in seed.SEEDED_ROSTER_ENTRIES}
	points = {entry.persona: (0.0, 0.0) for entry in seed.SEEDED_ROSTER_ENTRIES}
	if course_reference is None or assignment_reference is None:
		return counts, points
	response = _request(
		runner, repository_root, url,
		f"/api/course-instances/{course_reference}/gradebook", jar,
		seed.Stage.WORK, "detect",
	)
	body = _require_status(response, 200, seed.Stage.WORK, "detect")
	if not isinstance(body, dict) or body.get("courseReference") != course_reference:
		raise local_stack_control.models.ControllerError(
			"live-demo Gradebook projection is invalid"
		)
	rows = body.get("studentWork")
	if not isinstance(rows, list) or not all(isinstance(row, dict) for row in rows):
		raise local_stack_control.models.ControllerError(
			"live-demo Gradebook projection is invalid"
		)
	persona_by_roster_id = {
		entry.roster_id: entry.persona for entry in seed.SEEDED_ROSTER_ENTRIES
	}
	seen: set[str] = set()
	for row in rows:
		if row.get("assignmentReference") != assignment_reference:
			continue
		persona = persona_by_roster_id.get(row.get("rosterId"))
		count = row.get("gradedQuestionCount")
		earned = row.get("pointsEarned")
		possible = row.get("pointsPossible")
		if persona is None:
			continue
		if (
			persona in seen
			or not isinstance(count, int)
			or isinstance(count, bool)
			or not 0 <= count <= len(seed.SEEDED_QUESTION_IDS)
			or not isinstance(earned, (int, float))
			or isinstance(earned, bool)
			or not isinstance(possible, (int, float))
			or isinstance(possible, bool)
			or not 0 <= earned <= possible
		):
			raise local_stack_control.models.ControllerError(
				"live-demo Gradebook Student work projection is invalid"
			)
		seen.add(persona)
		counts[persona] = count
		points[persona] = (float(earned), float(possible))
	return counts, points


#============================================
def _assignment_attempts(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jars: dict[str, pathlib.Path],
	course_reference: str | None,
	assignment_reference: str | None,
) -> tuple[dict[str, int], dict[str, str]]:
	"""Read each seeded Student's own Assignment Attempt progress."""
	if course_reference is None or assignment_reference is None:
		return {}, {}
	seed = local_stack_control.live_demo_course_seed
	numbers: dict[str, int] = {}
	completions: dict[str, str] = {}
	path = f"/api/course-instances/{course_reference}/assignment-landing"
	for entry in seed.SEEDED_ROSTER_ENTRIES:
		response = _request(
			runner, repository_root, url, path, jars[entry.persona],
			seed.Stage.ATTEMPTS, "detect",
		)
		body = _require_status(response, 200, seed.Stage.ATTEMPTS, "detect")
		assignments = body.get("assignments") if isinstance(body, dict) else None
		if not isinstance(assignments, list):
			raise local_stack_control.models.ControllerError(
				"live-demo Student Assignment landing projection is invalid"
			)
		matches = [
			item for item in assignments
			if isinstance(item, dict)
			and item.get("reference") == assignment_reference
		]
		if not matches:
			continue
		if len(matches) != 1:
			raise local_stack_control.models.ControllerError(
				"live-demo Student Assignment landing identity is ambiguous"
			)
		attempt_number = matches[0].get("assignmentAttemptNumber")
		completion = matches[0].get("assignmentAttemptCompletion")
		if completion is None and attempt_number is None:
			continue
		if (
			not isinstance(attempt_number, int)
			or isinstance(attempt_number, bool)
			or attempt_number < 1
			or completion not in ("inProgress", "completed")
		):
			raise local_stack_control.models.ControllerError(
				"live-demo Student Assignment Attempt projection is invalid"
			)
		numbers[entry.persona] = attempt_number
		completions[entry.persona] = completion
	return numbers, completions


#============================================
def _observe(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	workspace: pathlib.Path,
	url: str,
	jars: dict[str, pathlib.Path],
) -> ResolvedCourseState:
	"""Read all M4 stage facts through ordinary product HTTP contracts."""
	seed = local_stack_control.live_demo_course_seed
	stored_report = _stored_report(workspace)
	stored = _stored_references(stored_report)
	elena = jars["elenaInstructor"]

	blueprint_response = _request(
		runner, repository_root, url, "/api/course-blueprints?pageSize=100", elena,
		seed.Stage.BLUEPRINT, "detect",
	)
	blueprint_items = _items(
		_require_status(blueprint_response, 200, seed.Stage.BLUEPRINT, "detect"),
		"Blueprint Course",
	)
	blueprint = _resolve_named_reference(
		blueprint_items,
		stored.get("blueprint_reference"),
		seed.SEEDED_BLUEPRINT_TITLE,
		REFERENCE_PATTERNS["blueprint_reference"],
		"Blueprint Course",
	)
	blueprint_reference = blueprint["reference"] if blueprint is not None else None
	blueprint_revision = blueprint.get("revision") if blueprint is not None else None
	if blueprint_revision is not None and (
		not isinstance(blueprint_revision, str) or not blueprint_revision.isdecimal()
	):
		raise local_stack_control.models.ControllerError(
			"live-demo Blueprint Revision projection is invalid"
		)

	course_response = _request(
		runner, repository_root, url, "/api/course-instances", elena,
		seed.Stage.COURSE, "detect",
	)
	course_items = _items(
		_require_status(course_response, 200, seed.Stage.COURSE, "detect"),
		"Course Instance",
	)
	course = _resolve_named_reference(
		course_items,
		stored.get("course_reference"),
		seed.SEEDED_COURSE_TITLE,
		REFERENCE_PATTERNS["course_reference"],
		"Course Instance",
	)
	course_reference = course["reference"] if course is not None else None

	roster_states: dict[str, str] = {}
	assignment: dict | None = None
	workspace_value: dict | None = None
	if course_reference is not None:
		roster_response = _request(
			runner, repository_root, url,
			f"/api/course-instances/{course_reference}/roster", elena,
			seed.Stage.ROSTER, "detect",
		)
		roster_items = _items(
			_require_status(roster_response, 200, seed.Stage.ROSTER, "detect"),
			"Course Roster", wrapped=False,
		)
		for item in roster_items:
			roster_id = item.get("rosterId")
			state = item.get("state")
			if isinstance(roster_id, str) and state in (
				"invitationPending", "activeStudent"
			):
				roster_states[roster_id] = state

		assignment_response = _request(
			runner, repository_root, url,
			f"/api/course-instances/{course_reference}/assignments", elena,
			seed.Stage.ASSIGNMENT, "detect",
		)
		assignment_items = _items(
			_require_status(
				assignment_response, 200, seed.Stage.ASSIGNMENT, "detect"
			),
			"Course Assignment", wrapped=False,
		)
		assignment = _resolve_named_reference(
			assignment_items,
			stored.get("assignment_reference"),
			seed.SEEDED_ASSIGNMENT_TITLE,
			REFERENCE_PATTERNS["assignment_reference"],
			"Course Assignment",
		)

	assignment_reference = assignment["reference"] if assignment is not None else None
	if course_reference is not None and assignment_reference is not None:
		workspace_response = _request(
			runner, repository_root, url,
			f"/api/course-instances/{course_reference}/assignments/{assignment_reference}",
			elena, seed.Stage.SELECTION, "detect",
		)
		workspace_body = _require_status(
			workspace_response, 200, seed.Stage.SELECTION, "detect"
		)
		if not isinstance(workspace_body, dict):
			raise local_stack_control.models.ControllerError(
				"live-demo Assignment Workspace projection is invalid"
			)
		workspace_value = workspace_body

	expected_roster_ids = {entry.roster_id for entry in seed.SEEDED_ROSTER_ENTRIES}
	questions = workspace_value.get("questions") if workspace_value is not None else None
	question_ids = (
		tuple(item.get("questionId") for item in questions if isinstance(item, dict))
		if isinstance(questions, list)
		else ()
	)
	selection_complete = workspace_value is not None and (
		workspace_value.get("title") == seed.SEEDED_ASSIGNMENT_TITLE
		and workspace_value.get("instructions") == seed.SEEDED_ASSIGNMENT_INSTRUCTIONS
		and workspace_value.get("dueAt") is None
		and workspace_value.get("lateWorkRule") == "accept"
		and question_ids == seed.SEEDED_QUESTION_IDS
	)
	assignment_edit_number = (
		workspace_value.get("editNumber") if workspace_value is not None else None
	)
	if assignment_edit_number is not None and (
		not isinstance(assignment_edit_number, str)
		or not assignment_edit_number.isdecimal()
		or int(assignment_edit_number) < 1
	):
		raise local_stack_control.models.ControllerError(
			"live-demo Assignment Edit Number projection is invalid"
		)
	assignment_status = assignment.get("status") if assignment is not None else None
	attempt_numbers, attempt_completions = _assignment_attempts(
		runner, repository_root, url, jars,
		course_reference, assignment_reference,
	)
	terminal_counts, gradebook_points = _gradebook_counts(
		runner, repository_root, url, elena, course_reference, assignment_reference
	)
	observed = seed.ObservedState(
		blueprint_complete=blueprint_reference is not None,
		course_complete=course_reference is not None,
		roster_complete=expected_roster_ids.issubset(roster_states),
		claims_complete=all(
			roster_states.get(roster_id) == "activeStudent"
			for roster_id in expected_roster_ids
		),
		assignment_complete=assignment_reference is not None,
		selection_complete=selection_complete,
		release_complete=assignment_status == "released",
		mary_attempt_exists="maryStudent" in attempt_numbers,
		jack_attempt_exists="jackStudent" in attempt_numbers,
		avery_attempt_exists="averyStudent" in attempt_numbers,
		mary_terminal_submission_count=terminal_counts["maryStudent"],
		jack_terminal_submission_count=terminal_counts["jackStudent"],
		avery_terminal_submission_count=terminal_counts["averyStudent"],
	)
	return ResolvedCourseState(
		observed=observed,
		blueprint_reference=blueprint_reference,
		blueprint_revision=blueprint_revision,
		course_reference=course_reference,
		assignment_reference=assignment_reference,
		assignment_edit_number=assignment_edit_number,
		roster_states=roster_states,
		attempt_numbers=attempt_numbers,
		attempt_completions=attempt_completions,
		terminal_submission_counts=terminal_counts,
		gradebook_points=gradebook_points,
	)


#============================================
def _report_value(state: ResolvedCourseState) -> dict:
	"""Return only resolved product facts and outstanding baseline stages."""
	seed = local_stack_control.live_demo_course_seed
	students = []
	for entry in seed.SEEDED_ROSTER_ENTRIES:
		membership = state.roster_states.get(entry.roster_id, "absent")
		attempt_number = state.attempt_numbers.get(entry.persona)
		terminal_count = state.terminal_submission_counts.get(entry.persona, 0)
		points_earned, points_possible = state.gradebook_points.get(
			entry.persona, (0.0, 0.0)
		)
		students.append({
			"persona": entry.persona,
			"roster_id": entry.roster_id,
			"membership": membership,
			"assignment_attempt": (
				{
					"attempt_number": attempt_number,
					"state": (
						"completed"
						if state.attempt_completions.get(entry.persona) == "completed"
						else "open"
					),
				}
				if attempt_number is not None
				else None
			),
			"submission_count": terminal_count,
			"grading_state": {
				"graded": terminal_count,
				"pending": 0,
				"instructorAttention": 0,
			},
			"points_earned": points_earned,
			"points_possible": points_possible,
		})
	return {
		"blueprint_reference": state.blueprint_reference,
		"course_reference": state.course_reference,
		"assignment_reference": state.assignment_reference,
		"students": students,
		"stages_still_outstanding": [
			stage.value for stage in seed.plan_stages(state.observed)
		],
	}


#============================================
def _write_report(workspace: pathlib.Path, state: ResolvedCourseState) -> pathlib.Path:
	"""Atomically replace the private machine-readable baseline report."""
	# ASVS 14.2.6 and 8.3.4: the mode-0600 report is bounded to minimum
	# product facts and intentionally contains no cookie, email, or response.
	content = json.dumps(
		_report_value(state), sort_keys=True, separators=(",", ":"), ensure_ascii=True
	).encode("ascii") + b"\n"
	path = report_path(workspace)
	local_stack_control.private_files.write_atomic_file(path, content, 0o600)
	return path


#============================================
def _supported_plan(
	observed: local_stack_control.live_demo_course_seed.ObservedState,
) -> tuple[local_stack_control.live_demo_course_seed.Stage, ...]:
	"""Return the implemented Course and Student-work baseline stages."""
	full = local_stack_control.live_demo_course_seed.plan_stages(observed)
	return tuple(stage for stage in full if stage in SUPPORTED_STAGES)


#============================================
def _assignment_path(state: ResolvedCourseState) -> str:
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
def _start_attempt(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	state: ResolvedCourseState,
	stage: local_stack_control.live_demo_course_seed.Stage,
	require_access: bool,
) -> dict:
	"""Start or resume one real Assignment Attempt and validate its presentation."""
	path = _assignment_path(state)
	if require_access:
		access = _request(
			runner, repository_root, url, path + "/access", jar,
			stage, "access",
		)
		if _require_status(access, 200, stage, "access") != {
			"startDecision": "may_start"
		}:
			raise local_stack_control.models.ControllerError(
				f"live-demo provisioning {stage.value} is not startable"
			)
	started = _request(
		runner, repository_root, url, path + "/start", jar,
		stage, "apply", "POST", {},
	)
	body = _require_status(started, 201, stage, "apply")
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
def _presented_response(
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
def _submission_status(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	path: str,
	nonce: str,
) -> str | None:
	"""Read one nonce-bound closed grading state, or absence before submission."""
	stage = local_stack_control.live_demo_course_seed.Stage.WORK
	response = _request(
		runner, repository_root, url,
		f"{path}/presentations/{nonce}/submissions", jar,
		stage, "status",
	)
	if response.status == 404:
		return None
	body = _require_status(response, 200, stage, "status")
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
def _wait_for_graded_submission(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	path: str,
	nonce: str,
) -> None:
	"""Wait for one accepted response to reach its real terminal Grading Result."""
	for _ in range(GRADING_POLL_ATTEMPTS):
		state = _submission_status(
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
def _apply_attempts(
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
		started = _start_attempt(
			runner, repository_root, url, jars[persona], state,
			seed.Stage.ATTEMPTS, True,
		)
		attempts[persona] = started["attemptNumber"]
	access = _request(
		runner, repository_root, url, _assignment_path(state) + "/access",
		jars["averyStudent"], seed.Stage.ATTEMPTS, "Avery access",
	)
	if _require_status(access, 200, seed.Stage.ATTEMPTS, "Avery access") != {
		"startDecision": "may_start"
	}:
		raise local_stack_control.models.ControllerError(
			"live-demo Avery Assignment is not startable"
		)
	return attempts


#============================================
def _apply_work(
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
	path = _assignment_path(state)
	for persona in ("maryStudent", "jackStudent"):
		if persona not in state.attempt_numbers:
			raise local_stack_control.models.ControllerError(
				"live-demo Student work Assignment Attempt is unavailable"
			)
		started = _start_attempt(
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
			grading_state = _submission_status(
				runner, repository_root, url, jars[persona], path, nonce
			)
			if grading_state is None:
				response = _presented_response(presentation, recipe)
				submitted = _request(
					runner, repository_root, url,
					f"{path}/presentations/{nonce}/submissions", jars[persona],
					seed.Stage.WORK, "submit", "POST", {"response": response},
				)
				receipt = _require_status(submitted, 201, seed.Stage.WORK, "submit")
				if receipt != {"presentationNonce": nonce, "gradingState": "pending"}:
					raise local_stack_control.models.ControllerError(
						"live-demo Question Submission receipt is invalid"
					)
			_wait_for_graded_submission(
				runner, repository_root, url, jars[persona], path, nonce
			)


#============================================
def _with_attempt_checkpoint(
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
def _require_pinned_mary_grade(state: ResolvedCourseState) -> None:
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


#============================================
def _apply_stage(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jars: dict[str, pathlib.Path],
	stage: local_stack_control.live_demo_course_seed.Stage,
	state: ResolvedCourseState,
) -> dict[str, int] | None:
	"""Apply one planned baseline stage through ordinary product HTTP contracts."""
	seed = local_stack_control.live_demo_course_seed
	elena = jars["elenaInstructor"]
	if stage is seed.Stage.BLUEPRINT:
		response = _request(
			runner, repository_root, url, "/api/course-blueprints", elena,
			stage, "apply", "POST", seed.blueprint_payload(),
		)
		_require_status(response, 201, stage, "apply")
		return None
	if stage is seed.Stage.COURSE:
		if state.blueprint_reference is None or state.blueprint_revision is None:
			raise local_stack_control.models.ControllerError(
				"live-demo provisioning course prerequisite is unavailable"
			)
		response = _request(
			runner, repository_root, url, "/api/course-instances", elena,
			stage, "apply", "POST",
			seed.course_payload(state.blueprint_reference, state.blueprint_revision),
		)
		_require_status(response, 201, stage, "apply")
		return None
	if state.course_reference is None:
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} Course prerequisite is unavailable"
		)
	if stage is seed.Stage.ROSTER:
		response = _request(
			runner, repository_root, url,
			f"/api/course-instances/{state.course_reference}/roster", elena,
			stage, "apply", "POST", seed.roster_payload(),
		)
		_require_status(response, 201, stage, "apply")
		return None
	if stage is seed.Stage.CLAIMS:
		for entry in seed.SEEDED_ROSTER_ENTRIES:
			if state.roster_states.get(entry.roster_id) == "activeStudent":
				continue
			response = _request(
				runner, repository_root, url,
				f"/api/course-instances/{state.course_reference}/roster/claim",
				jars[entry.persona], stage, "apply", "POST", {},
			)
			body = _require_status(response, 200, stage, "apply")
			if body != {"activeStudentMembership": True}:
				raise local_stack_control.models.ControllerError(
					"live-demo provisioning claims returned an invalid receipt"
				)
		return None
	if stage is seed.Stage.ASSIGNMENT:
		response = _request(
			runner, repository_root, url,
			f"/api/course-instances/{state.course_reference}/assignments", elena,
			stage, "apply", "POST", seed.assignment_create_payload(),
		)
		_require_status(response, 201, stage, "apply")
		return None
	if state.assignment_reference is None or state.assignment_edit_number is None:
		raise local_stack_control.models.ControllerError(
			f"live-demo provisioning {stage.value} Assignment prerequisite is unavailable"
		)
	assignment_path = (
		f"/api/course-instances/{state.course_reference}/assignments/"
		f"{state.assignment_reference}"
	)
	if stage is seed.Stage.SELECTION:
		response = _request(
			runner, repository_root, url, assignment_path, elena,
			stage, "apply", "PUT", seed.assignment_save_payload(seed.SEEDED_QUESTION_IDS),
			state.assignment_edit_number,
		)
		_require_status(response, 200, stage, "apply")
		return None
	if stage is seed.Stage.RELEASE:
		validation = _request(
			runner, repository_root, url, assignment_path + "/release-validation",
			elena, stage, "validate",
		)
		validation_body = _require_status(validation, 200, stage, "validate")
		if validation_body != {"canRelease": True, "issues": []}:
			raise local_stack_control.models.ControllerError(
				"live-demo provisioning release validation did not pass"
			)
		released = _request(
			runner, repository_root, url, assignment_path + "/release", elena,
			stage, "apply", "POST", {}, state.assignment_edit_number,
		)
		receipt = _require_status(released, 201, stage, "apply")
		if receipt != {
			"reference": state.assignment_reference,
			"revisionNumber": 1,
		}:
			raise local_stack_control.models.ControllerError(
				"live-demo provisioning release returned an invalid receipt"
			)
		return None
	if stage is seed.Stage.ATTEMPTS:
		return _apply_attempts(runner, repository_root, url, jars, state)
	if stage is seed.Stage.WORK:
		_apply_work(runner, repository_root, url, jars, state)
		return None
	raise local_stack_control.models.ControllerError(
		f"live-demo provisioning stage {stage.value} is unsupported"
	)


#============================================
def _stage_complete(
	stage: local_stack_control.live_demo_course_seed.Stage,
	observed: local_stack_control.live_demo_course_seed.ObservedState,
) -> bool:
	"""Return the exact observation predicate owned by one baseline stage."""
	checks = {
		local_stack_control.live_demo_course_seed.Stage.BLUEPRINT: observed.blueprint_complete,
		local_stack_control.live_demo_course_seed.Stage.COURSE: observed.course_complete,
		local_stack_control.live_demo_course_seed.Stage.ROSTER: observed.roster_complete,
		local_stack_control.live_demo_course_seed.Stage.CLAIMS: observed.claims_complete,
		local_stack_control.live_demo_course_seed.Stage.ASSIGNMENT: observed.assignment_complete,
		local_stack_control.live_demo_course_seed.Stage.SELECTION: observed.selection_complete,
		local_stack_control.live_demo_course_seed.Stage.RELEASE: observed.release_complete,
		local_stack_control.live_demo_course_seed.Stage.ATTEMPTS: observed.attempts_complete,
		local_stack_control.live_demo_course_seed.Stage.WORK: observed.work_complete,
	}
	return checks.get(stage, False)


#============================================
def provision_live_demo_course(
	runner: local_stack_control.process.CommandRunner,
	target: local_stack_control.models.DisposableComposeTarget,
	workspace: pathlib.Path,
	stop_after: local_stack_control.live_demo_course_seed.Stage | None = None,
	report_only: bool = False,
) -> ProvisionResult:
	"""Observe and converge the complete Live Demo teaching baseline."""
	if not local_stack_control.live_demo_gateway.is_tls_target(target.target):
		raise local_stack_control.models.ControllerError(
			"course provisioning requires the fixed TLS Live Demo"
		)
	local_stack_control.private_files.require_private_directory(workspace)
	url = local_stack_control.live_demo_gateway.gateway_url(target.target)
	jars = _create_session_jars(runner, target.target.repo_root, workspace, url)
	try:
		state = _observe(runner, target.target.repo_root, workspace, url, jars)
		_require_pinned_mary_grade(state)
		initial_plan = _supported_plan(state.observed)
		path = report_path(workspace)
		if report_only:
			return ProvisionResult(state, initial_plan, path)
		for stage in initial_plan:
			attempt_checkpoint = _apply_stage(
				runner, target.target.repo_root, url, jars, stage, state
			)
			if attempt_checkpoint is not None:
				state = _with_attempt_checkpoint(state, attempt_checkpoint)
				path = _write_report(workspace, state)
			state = _observe(runner, target.target.repo_root, workspace, url, jars)
			_require_pinned_mary_grade(state)
			if not _stage_complete(stage, state.observed):
				raise local_stack_control.models.ControllerError(
					f"live-demo provisioning {stage.value} did not converge"
				)
			path = _write_report(workspace, state)
			if stop_after is stage:
				return ProvisionResult(state, _supported_plan(state.observed), path)
		path = _write_report(workspace, state)
		return ProvisionResult(state, _supported_plan(state.observed), path)
	finally:
		_remove_session_jars(jars)
