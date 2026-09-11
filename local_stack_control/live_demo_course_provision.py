"""Convergent product-HTTP provisioning for the Live Demo teaching baseline."""

# Standard Library
import dataclasses
import json
import os
import pathlib
import re
import tempfile

# local repo modules
import local_stack_control.live_demo_course_activity
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
ProductResponse = local_stack_control.live_demo_course_activity.ProductResponse
ResolvedCourseState = local_stack_control.live_demo_course_activity.ResolvedCourseState
_split_http_result = local_stack_control.live_demo_course_activity.split_http_result
_request = local_stack_control.live_demo_course_activity.request
_require_status = local_stack_control.live_demo_course_activity.require_status


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
def _resolve_course_reference(
	items: list[dict],
	stored_reference: str | None,
	short_name: str,
	long_name: str,
	reference_pattern: re.Pattern[str],
) -> dict | None:
	"""Resolve a prior Course reference first, then its exact name pair."""
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
	matches = [
		item
		for item in valid
		if item.get("shortName") == short_name and item.get("longName") == long_name
	]
	if len(matches) > 1:
		raise local_stack_control.models.ControllerError(
			"live-demo Course Instance identity is ambiguous"
		)
	return matches[0] if matches else None


#============================================
def _graded_question_counts(
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
def _saved_response_counts(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jars: dict[str, pathlib.Path],
	course_reference: str | None,
	assignment_reference: str | None,
	attempt_completions: dict[str, str],
) -> dict[str, int]:
	"""Read open Student work counts from the answer-free Attempt projections."""
	seed = local_stack_control.live_demo_course_seed
	counts = {entry.persona: 0 for entry in seed.SEEDED_ROSTER_ENTRIES}
	if course_reference is None or assignment_reference is None:
		return counts
	path = (
		f"/api/course-instances/{course_reference}/assignments/"
		f"{assignment_reference}/access"
	)
	for persona, completion in attempt_completions.items():
		if completion != "inProgress":
			continue
		body = _require_status(
			_request(
				runner, repository_root, url, path, jars[persona],
				seed.Stage.WORK, "saved-work",
			),
			200, seed.Stage.WORK, "saved-work",
		)
		if not isinstance(body, dict):
			raise local_stack_control.models.ControllerError(
				"live-demo Student Assignment Access projection is invalid"
			)
		reference = local_stack_control.live_demo_course_activity.assignment_attempt_reference(
			body.get("activeAssignmentAttempt")
		)
		if reference is None:
			raise local_stack_control.models.ControllerError(
				"live-demo Student Assignment Attempt is unavailable"
			)
		progress = local_stack_control.live_demo_course_activity.progress(
			runner, repository_root, url, jars[persona], reference,
			seed.Stage.WORK, "saved-work",
		)
		counts[persona] = local_stack_control.live_demo_course_activity.saved_count(progress)
	return counts


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
	course = _resolve_course_reference(
		course_items,
		stored.get("course_reference"),
		seed.SEEDED_COURSE_SHORT_NAME,
		seed.SEEDED_COURSE_LONG_NAME,
		REFERENCE_PATTERNS["course_reference"],
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
		and workspace_value.get("assignmentAttemptTimeLimitSeconds")
		== seed.SEEDED_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS
		and workspace_value.get("activityRules")
		== seed.assignment_activity_rules(
			assignment_question_display_rule="oneQuestionAtATime",
			assignment_question_order_rule="authoredOrder",
		)
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
	saved_response_counts = _saved_response_counts(
		runner, repository_root, url, jars, course_reference, assignment_reference,
		attempt_completions,
	)
	graded_question_counts, gradebook_points = _graded_question_counts(
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
		mary_attempt_completed=attempt_completions.get("maryStudent") == "completed",
		mary_graded_question_count=graded_question_counts["maryStudent"],
		jack_graded_question_count=graded_question_counts["jackStudent"],
		avery_graded_question_count=graded_question_counts["averyStudent"],
		jack_saved_response_count=saved_response_counts["jackStudent"],
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
		graded_question_counts=graded_question_counts,
		saved_response_counts=saved_response_counts,
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
		graded_question_count = state.graded_question_counts.get(entry.persona, 0)
		saved_count = state.saved_response_counts.get(entry.persona, 0)
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
			"assignment_submission_count": int(
				state.attempt_completions.get(entry.persona) == "completed"
			),
			"graded_question_count": graded_question_count,
			"saved_response_count": saved_count,
			"grading_state": {
				"graded_question_count": graded_question_count,
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
	"""Retain the established private facade for the activity owner."""
	return local_stack_control.live_demo_course_activity.assignment_path(state)


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
	"""Retain the established private facade for the activity owner."""
	return local_stack_control.live_demo_course_activity.start_attempt(
		runner, repository_root, url, jar, state, stage, require_access
	)


#============================================
def _presented_response(
	presentation: dict,
	recipe: local_stack_control.live_demo_course_seed.SeededPresentedResponse,
) -> dict:
	"""Retain the established private facade for the activity owner."""
	return local_stack_control.live_demo_course_activity.presented_response(
		presentation, recipe
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
	"""Retain the established private facade for the activity owner."""
	return local_stack_control.live_demo_course_activity.submission_status(
		runner, repository_root, url, jar, path, nonce
	)


#============================================
def _wait_for_graded_submission(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jar: pathlib.Path,
	path: str,
	nonce: str,
) -> None:
	"""Retain the established private facade for the activity owner."""
	local_stack_control.live_demo_course_activity.wait_for_graded_submission(
		runner, repository_root, url, jar, path, nonce
	)


#============================================
def _apply_attempts(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jars: dict[str, pathlib.Path],
	state: ResolvedCourseState,
) -> dict[str, int]:
	"""Retain the established private facade for the activity owner."""
	return local_stack_control.live_demo_course_activity.apply_attempts(
		runner, repository_root, url, jars, state
	)


#============================================
def _apply_work(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
	url: str,
	jars: dict[str, pathlib.Path],
	state: ResolvedCourseState,
) -> None:
	"""Retain the established private facade for the activity owner."""
	local_stack_control.live_demo_course_activity.apply_work(
		runner, repository_root, url, jars, state
	)


#============================================
def _with_attempt_checkpoint(
	state: ResolvedCourseState,
	attempts: dict[str, int],
) -> ResolvedCourseState:
	"""Retain the established private facade for the activity owner."""
	return local_stack_control.live_demo_course_activity.with_attempt_checkpoint(
		state, attempts
	)


#============================================
def _require_pinned_mary_grade(state: ResolvedCourseState) -> None:
	"""Retain the established private facade for the activity owner."""
	local_stack_control.live_demo_course_activity.require_pinned_mary_grade(state)


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
