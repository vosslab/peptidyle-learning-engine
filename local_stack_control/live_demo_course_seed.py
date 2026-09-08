"""Declarative Course-domain baseline and convergence plan for the Live Demo."""

# Standard Library
import re
import enum
import dataclasses

# local repo modules
import local_stack_control.live_demo_seed


SEEDED_BLUEPRINT_TITLE = "Biochemistry 301: Proteins and Peptides"
SEEDED_COURSE_TITLE = "Biochemistry 301: Proteins and Peptides"
SEEDED_ASSIGNMENT_TITLE = "Peptide Structure Practice"
SEEDED_ASSIGNMENT_INSTRUCTIONS = (
	"Complete the four practice questions on peptide structure and properties."
)
SEEDED_BLUEPRINT_MODULE_LABEL = "Peptide structure and properties"


@dataclasses.dataclass(frozen=True)
class SeededCourseTerm:
	"""One fictional Course Term used by the complete Live Demo baseline."""

	start_date: str
	end_date: str
	time_zone: str


@dataclasses.dataclass(frozen=True)
class SeededRosterEntry:
	"""One declared roster import row and its closed seeded persona."""

	persona: str
	roster_id: str
	email: str


@dataclasses.dataclass(frozen=True)
class SeededStudentWork:
	"""The number of issued Questions one seeded Student answers."""

	persona: str
	answered_question_count: int


@dataclasses.dataclass(frozen=True)
class SeededPresentedResponse:
	"""One fixed response recipe resolved only against a public presentation."""

	question_id: str
	presented_kind: str
	selection_index: int | None = None
	numeric_value: float | None = None


@dataclasses.dataclass(frozen=True)
class SeededStudentResponses:
	"""One Student's ordered response recipes for the declared work count."""

	persona: str
	responses: tuple[SeededPresentedResponse, ...]


class Stage(enum.StrEnum):
	"""One ordered, independently detectable provisioning operation."""

	BLUEPRINT = "blueprint"
	COURSE = "course"
	ROSTER = "roster"
	CLAIMS = "claims"
	ASSIGNMENT = "assignment"
	SELECTION = "selection"
	RELEASE = "release"
	ATTEMPTS = "attempts"
	WORK = "work"


@dataclasses.dataclass(frozen=True)
class ObservedState:
	"""Product facts observed before planning one convergent provisioning pass."""

	blueprint_complete: bool = False
	course_complete: bool = False
	roster_complete: bool = False
	claims_complete: bool = False
	assignment_complete: bool = False
	selection_complete: bool = False
	release_complete: bool = False
	mary_attempt_exists: bool = False
	jack_attempt_exists: bool = False
	avery_attempt_exists: bool = False
	mary_terminal_submission_count: int = 0
	jack_terminal_submission_count: int = 0
	avery_terminal_submission_count: int = 0

	@property
	def attempts_complete(self) -> bool:
		"""Return whether only Mary and Jack hold the declared Assignment Attempts."""
		complete = (
			self.mary_attempt_exists
			and self.jack_attempt_exists
			and not self.avery_attempt_exists
		)
		return complete

	@property
	def work_complete(self) -> bool:
		"""Return whether terminal submissions match the declared Student work."""
		complete = (
			self.mary_terminal_submission_count == 4
			and self.jack_terminal_submission_count == 2
			and self.avery_terminal_submission_count == 0
		)
		return complete


SEEDED_COURSE_TERM = SeededCourseTerm(
	start_date="2026-08-24",
	end_date="2026-12-11",
	time_zone="America/Chicago",
)

_ACCOUNT_IDS_BY_SETTING = {
	account.setting: account.account_id
	for account in local_stack_control.live_demo_seed.SEEDED_ACCOUNTS
}
_STUDENT_EMAILS_BY_ACCOUNT_ID = dict(
	local_stack_control.live_demo_seed.SEEDED_STUDENT_AUTHENTICATION_EMAILS
)

SEEDED_ROSTER_ENTRIES = (
	SeededRosterEntry(
		persona="maryStudent",
		roster_id="BIO301-MARY",
		email=_STUDENT_EMAILS_BY_ACCOUNT_ID[
			_ACCOUNT_IDS_BY_SETTING["PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID"]
		],
	),
	SeededRosterEntry(
		persona="jackStudent",
		roster_id="BIO301-JACK",
		email=_STUDENT_EMAILS_BY_ACCOUNT_ID[
			_ACCOUNT_IDS_BY_SETTING["PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID"]
		],
	),
	SeededRosterEntry(
		persona="averyStudent",
		roster_id="BIO301-AVERY",
		email=_STUDENT_EMAILS_BY_ACCOUNT_ID[
			_ACCOUNT_IDS_BY_SETTING["PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID"]
		],
	),
)

SEEDED_STUDENT_WORK = (
	SeededStudentWork(persona="maryStudent", answered_question_count=4),
	SeededStudentWork(persona="jackStudent", answered_question_count=2),
	SeededStudentWork(persona="averyStudent", answered_question_count=0),
)

SEEDED_QUESTION_IDS = tuple(
	question.question_id
	for question in local_stack_control.live_demo_seed.SEEDED_PUBLISHED_QUESTIONS
)

# These recipes use only public Question Presentation shapes. The provisioner
# resolves opaque choice and region references from the presented order; it
# never reads a Question Source or Answer Key.
SEEDED_STUDENT_RESPONSES = (
	SeededStudentResponses(
		persona="maryStudent",
		responses=(
			SeededPresentedResponse("PNE-0001", "singleChoice", selection_index=0),
			SeededPresentedResponse("PNE-0002", "singleChoice", selection_index=1),
			SeededPresentedResponse("PNE-0003", "numerical", numeric_value=1),
			SeededPresentedResponse("PNE-0004", "hotspot", selection_index=1),
		),
	),
	SeededStudentResponses(
		persona="jackStudent",
		responses=(
			SeededPresentedResponse("PNE-0001", "singleChoice", selection_index=1),
			SeededPresentedResponse("PNE-0002", "singleChoice", selection_index=0),
		),
	),
	SeededStudentResponses(persona="averyStudent", responses=()),
)

# The fixed public-presentation recipes above intentionally yield a mixed real
# grade for Mary: PNE-0001 and PNE-0003 correct, the other two incorrect.
SEEDED_MARY_POINTS_EARNED = 2.0
SEEDED_MARY_POINTS_POSSIBLE = 4.0


#============================================
def fixed_question_entry(question_id: str) -> dict:
	"""Return one fixed reusable Question entry for the Blueprint Course."""
	entry = {
		"kind": "fixed",
		"question_id": question_id,
		"points_possible": "1",
		"scoring_rule": "normal",
		"question_attempt_limit": {"maxAttempts": None},
		"question_attempt_time_limit": {"kind": "unlimited"},
	}
	return entry


#============================================
def assignment_activity_rules() -> dict:
	"""Return the complete reusable Assignment activity rules."""
	rules = {
		"assignmentCompletionRule": {"kind": "answerAll"},
		"assignmentAttemptGradeRule": "highest",
		"assignmentAttemptContinuationRule": {"kind": "unlimited"},
		"questionPoolReuseRule": "reuseSelection",
		"questionVariationRule": "newVariation",
		"assignmentAttemptResumeRule": "resumable",
		"assignmentQuestionDisplayRule": "allQuestions",
		"assignmentNavigationRule": "freeNavigation",
		"assignmentQuestionOrderRule": "authoredOrder",
	}
	return rules


#============================================
def student_feedback_release_rule() -> dict:
	"""Return the baseline's explicit Student Feedback Release Rule."""
	rule = {
		"score": "after_submit",
		"per_item_correctness": "after_submit",
		"question_feedback": "after_submit",
		"question_answer": "never",
		"question_answer_explanation": "never",
		"class_statistics": "never",
	}
	return rule


#============================================
def blueprint_payload() -> dict:
	"""Return the complete reusable Blueprint Course creation input."""
	assignment = {
		"title": SEEDED_ASSIGNMENT_TITLE,
		"instructions": SEEDED_ASSIGNMENT_INSTRUCTIONS,
		"entries": [fixed_question_entry(question_id) for question_id in SEEDED_QUESTION_IDS],
		"defaults": {
			"assignment_attempt_time_limit_seconds": None,
			"attempt_limit": None,
			"late_work_rule": "accept",
			"assignment_deadline_rule": "auto_submit",
			"activity_rules": assignment_activity_rules(),
			"student_feedback_release_rule": student_feedback_release_rule(),
		},
		"schedule": {"available_at": None, "due_at": None, "closes_at": None},
	}
	payload = {
		"title": SEEDED_BLUEPRINT_TITLE,
		"modules": [{"label": SEEDED_BLUEPRINT_MODULE_LABEL, "assignments": [assignment]}],
	}
	return payload


#============================================
def course_payload(blueprint_reference: str, blueprint_revision: str = "1") -> dict:
	"""Return Elena's self-assigned Course Instance creation input."""
	if re.fullmatch(r"BP-[1-9][0-9]{0,9}", blueprint_reference) is None:
		raise ValueError("Blueprint Course Reference must be canonical")
	if re.fullmatch(r"[1-9][0-9]*", blueprint_revision) is None:
		raise ValueError("Blueprint Revision Number must be positive")
	payload = {
		"blueprintCourse": blueprint_reference,
		"blueprintRevision": blueprint_revision,
		"title": SEEDED_COURSE_TITLE,
		"term": {
			"startDate": SEEDED_COURSE_TERM.start_date,
			"endDate": SEEDED_COURSE_TERM.end_date,
			"timeZone": SEEDED_COURSE_TERM.time_zone,
		},
	}
	return payload


#============================================
def roster_payload() -> dict:
	"""Return the three-row Course Roster Import request."""
	entries = [
		{"email": entry.email, "rosterId": entry.roster_id}
		for entry in SEEDED_ROSTER_ENTRIES
	]
	payload = {"entries": entries}
	return payload


#============================================
def assignment_create_payload() -> dict:
	"""Return the initial Unreleased Assignment creation input."""
	payload = {
		"title": SEEDED_ASSIGNMENT_TITLE,
		"instructions": SEEDED_ASSIGNMENT_INSTRUCTIONS,
	}
	return payload


#============================================
def assignment_save_payload(question_ids: tuple[str, ...]) -> dict:
	"""Return the complete baseline Assignment Workspace save input."""
	if question_ids != SEEDED_QUESTION_IDS:
		raise ValueError("Assignment selection must match the seeded Published Questions")
	payload = {
		"title": SEEDED_ASSIGNMENT_TITLE,
		"instructions": SEEDED_ASSIGNMENT_INSTRUCTIONS,
		"dueAt": None,
		"lateWorkRule": "accept",
		"questionIds": list(question_ids),
	}
	return payload


#============================================
def plan_stages(observed: ObservedState) -> tuple[Stage, ...]:
	"""Return each incomplete provisioning stage in dependency order."""
	planned: list[Stage] = []
	if not observed.blueprint_complete:
		planned.append(Stage.BLUEPRINT)
	if not observed.course_complete:
		planned.append(Stage.COURSE)
	if not observed.roster_complete:
		planned.append(Stage.ROSTER)
	if not observed.claims_complete:
		planned.append(Stage.CLAIMS)
	if not observed.assignment_complete:
		planned.append(Stage.ASSIGNMENT)
	if not observed.selection_complete:
		planned.append(Stage.SELECTION)
	if not observed.release_complete:
		planned.append(Stage.RELEASE)
	if not observed.attempts_complete:
		planned.append(Stage.ATTEMPTS)
	if not observed.work_complete:
		planned.append(Stage.WORK)
	stages = tuple(planned)
	return stages
