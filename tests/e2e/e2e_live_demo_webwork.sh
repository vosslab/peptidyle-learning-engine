#!/usr/bin/env bash
# Connected proof that an ordinary published WeBWorK Question can be selected,
# issued, submitted, and graded through the current Assignment contract.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
# shellcheck disable=SC1091
source "$repository_root/tests/e2e/e2e_live_demo_assignment_helpers.sh"
cd "$repository_root"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_webwork.sh [--render|--grade|--all]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--render) mode="render" ;;
	--grade) mode="grade" ;;
	--all) ;;
	*) usage; exit 2 ;;
esac

compact_question_id_to_wire() {
	python3 -c '
import re, sys
value=sys.argv[1]
if re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{7}", value) is None:
    raise SystemExit("published WeBWorK Question ID is not compact Crockford Base32")
print(value[:3] + "-" + value[3:])
' "$1"
}

published_webwork_reference() {
	local postgres compact revision
	postgres="$(service_id postgres)"
	read -r compact revision < <(
		podman exec "$postgres" sh -lc \
			'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -F " " -c "SELECT revision.question_id, revision.revision_number FROM ple_data.question_revision AS revision JOIN ple_private.question_revision_source_binding AS binding ON binding.question_id = revision.question_id AND binding.revision_number = revision.revision_number JOIN ple_data.published_question AS lineage ON lineage.question_id = revision.question_id WHERE lineage.availability = '\''available'\'' AND revision.backend = '\''webwork'\'' AND binding.question_format = '\''webworkPg'\'' ORDER BY revision.published_at, revision.question_id, revision.revision_number LIMIT 1"'
	)
	[ -n "${compact:-}" ] && [ -n "${revision:-}" ] || {
		echo "Live Demo has no available, ordinarily published WeBWorK Question" >&2
		exit 1
	}
	printf '{"questionId":"%s","revisionNumber":%s}\n' \
		"$(compact_question_id_to_wire "$compact")" "$revision"
}

picker_references() {
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1])
if not isinstance(items, list):
    raise SystemExit("Assignment Question picker is not a list")
references=[]
for item in items:
    reference=item.get("reference") if isinstance(item, dict) else None
    if (not isinstance(reference, dict) or set(reference) != {"questionId", "revisionNumber"}
        or not isinstance(reference["questionId"], str)
        or re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}", reference["questionId"]) is None
        or not isinstance(reference["revisionNumber"], int) or isinstance(reference["revisionNumber"], bool)
        or reference["revisionNumber"] < 1):
        raise SystemExit("Assignment Question picker lacks an exact current Question Revision")
    references.append(reference)
if len(references) < 2:
    raise SystemExit("Live Demo must expose a Pilot Question and a WeBWorK Question")
print(json.dumps(references, separators=(",", ":")))
' "$1"
}

assert_picker_contains() {
	python3 -c '
import json, sys
references=json.loads(sys.argv[1]); required=json.loads(sys.argv[2])
if required not in references:
    raise SystemExit("published WeBWorK Question is not selectable through the ordinary Assignment picker")
if not any(reference != required for reference in references):
    raise SystemExit("Picker did not also expose a distinct runtime Pilot Question")
' "$1" "$2"
}

webwork_assignment_payload() {
	local workspace="$1" reference="$2"
	python3 -c '
import json, sys, uuid
workspace=json.loads(sys.argv[1]); reference=json.loads(sys.argv[2])
required={"title","instructions","dueAt","availableAt","closesAt","lateWorkRule","assignmentAttemptTimeLimitSeconds","attemptLimit","activityRules","studentFeedbackReleaseRule"}
if not required.issubset(workspace):
    raise SystemExit("current Assignment workspace is incomplete")
payload={key: workspace[key] for key in required}
payload.update({
  "title":"Current WeBWorK delivery",
  "instructions":"Complete the rendered question.",
  "assignmentAttemptTimeLimitSeconds":300,
  "entries":[{
    "kind":"fixedQuestion", "id":str(uuid.uuid4()), "reference":reference,
    "pointsPossible":"1", "availability":"available", "scoringRule":"normal",
    "questionAttemptLimit":{"maxAttempts":None},
    "questionAttemptTimeLimit":{"kind":"unlimited"},
  }],
})
print(json.dumps(payload, separators=(",", ":")))
' "$workspace" "$reference"
}

new_released_webwork_assignment() {
	local instructor_cookie="$1" course source_choices source_reference created assignment edit picker references webwork payload saved released
	course="$(new_course_reference "$instructor_cookie")"
	source_choices="$(request "/api/course-instances/$course/assignment-source-choices" "$instructor_cookie")"
	require_status "Assignment source choices" "$source_choices" 200
	source_reference="$(source_choice_reference "$(response_body "$source_choices")")"
	created="$(request "/api/course-instances/$course/assignments" "$instructor_cookie" POST "{\"blueprintAssignmentReference\":\"$source_reference\",\"title\":\"Current WeBWorK delivery\",\"instructions\":\"Complete the rendered question.\"}")"
	require_status "WeBWorK Assignment creation" "$created" 201
	read -r assignment edit < <(workspace_reference_and_edit "$(response_body "$created")")
	picker="$(request "/api/course-instances/$course/assignment-question-picker" "$instructor_cookie")"
	require_status "Assignment Question picker" "$picker" 200
	references="$(picker_references "$(response_body "$picker")")"
	webwork="$(published_webwork_reference)"
	assert_picker_contains "$references" "$webwork"
	payload="$(webwork_assignment_payload "$(response_body "$created")" "$webwork")"
	saved="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor_cookie" PUT "$payload" "$edit")"
	require_status "normalized WeBWorK Assignment save" "$saved" 200
	read -r _ edit < <(workspace_reference_and_edit "$(response_body "$saved")")
	released="$(request "/api/course-instances/$course/assignments/$assignment/release" "$instructor_cookie" POST '{}' "$edit")"
	require_status "WeBWorK Assignment release" "$released" 200
	printf '%s %s %s\n' "$course" "$assignment" "$webwork"
}

assert_webwork_presentation() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); expected=json.loads(sys.argv[2]); resumed=sys.argv[3] == "true"
required={"assignment","assignmentAttempt","attemptNumber","resumed","title","instructions","questions"}
if set(value) != required or value["resumed"] is not resumed:
    raise SystemExit("Assignment start projection is not current and closed")
if re.fullmatch(r"R-[1-9][0-9]{0,9}", value["assignmentAttempt"] or "") is None:
    raise SystemExit("Assignment Attempt reference is malformed")
questions=value["questions"]
if not isinstance(questions, list) or len(questions) != 1:
    raise SystemExit("Assignment did not issue exactly one selected WeBWorK Question")
question=questions[0]
if not isinstance(question, dict) or question.get("questionRevision") != expected:
    raise SystemExit("issued Question did not retain the selected exact WeBWorK Revision")
if question.get("response", {}).get("kind") != "singleChoice":
    raise SystemExit("published WeBWorK Question did not render a single-choice presentation")
private_words={"answer", "correct", "solution", "grading", "score", "replay", "source", "webworkpgpath", "questionattemptid", "binding", "checksum"}
if any(word in json.dumps(value, sort_keys=True).lower() for word in private_words):
    raise SystemExit("public WeBWorK presentation exposed private backend evidence")
' "$1" "$2" "$3"
}

assert_same_presentation() {
	python3 -c '
import json, sys
if json.loads(sys.argv[1]).get("questions") != json.loads(sys.argv[2]).get("questions"):
    raise SystemExit("Assignment resume did not replay the exact public presentation")
' "$1" "$2"
}

assignment_attempt_reference() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); attempt=value.get("activeAssignmentAttempt")
if not isinstance(value, dict) or re.fullmatch(r"R-[1-9][0-9]{0,9}", attempt or "") is None:
    raise SystemExit("Assignment Access did not expose the active public Attempt")
print(attempt)
' "$1"
}

webwork_response() {
	python3 -c '
import json, sys
questions=json.loads(sys.argv[1]).get("questions")
if not isinstance(questions, list) or len(questions) != 1:
    raise SystemExit("WeBWorK presentation has no single issued Question")
choices=questions[0].get("response", {}).get("choices")
if not isinstance(choices, list) or not choices or not isinstance(choices[0].get("id"), str):
    raise SystemExit("WeBWorK presentation has no submit-ready choice")
print(json.dumps({"response":{"kind":"multipleChoice","selected":[choices[0]["id"]]}}, separators=(",", ":")))
' "$1"
}

assert_saved_response() {
	python3 -c '
import json, sys
if json.loads(sys.argv[1]) != {"assignmentAttempt":sys.argv[2], "position":1, "responseState":"saved"}:
    raise SystemExit("WeBWorK response save acknowledgement is malformed")
' "$1" "$2"
}

assert_assignment_submitted() {
	python3 -c '
import json, sys
if json.loads(sys.argv[1]) != {"assignmentAttempt":sys.argv[2], "submissionState":"submitted"}:
    raise SystemExit("Assignment submission acknowledgement is malformed")
' "$1" "$2"
}

assert_webwork_job_state() {
	local course="$1" assignment="$2" expected="$3" postgres output
	postgres="$(service_id postgres)"
	for _ in $(seq 1 30); do
		output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "SELECT job.state || '|' || grading.grading_state FROM ple_private.job AS job JOIN ple_private.question_submission_grading AS grading ON grading.job_id = job.job_id JOIN ple_private.question_submission AS submission ON submission.submission_id = job.question_submission_id JOIN ple_private.question_attempt AS question_attempt ON question_attempt.question_attempt_id = submission.question_attempt_id JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id JOIN ple_private.assignment_attempt AS assignment_attempt ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id JOIN ple_data.assignment AS current_assignment ON current_assignment.assignment_id = assignment_attempt.assignment_id JOIN ple_data.course_instance AS course_instance ON course_instance.course_id = current_assignment.course_id WHERE course_instance.reference_number = ${course#C-} AND current_assignment.reference_number = ${assignment#A-} ORDER BY job.created_at DESC LIMIT 1")"
		[ "$output" != "$expected" ] || return
		sleep 1
	done
	echo "WeBWorK grading Job did not reach $expected" >&2
	exit 1
}

new_student_attempt() {
	local course="$1" assignment="$2" instructor_cookie="$3" student_cookie="$4" started access attempt
	claim_student_record "$course" "$instructor_cookie" "$student_cookie" "mary.okafor@live-demo.invalid" "WEBWORK-MARY-$assignment"
	started="$(request "/api/course-instances/$course/assignments/$assignment/start" "$student_cookie" POST '{}')"
	require_status "Student WeBWorK Assignment start" "$started" 201
	access="$(request "/api/course-instances/$course/assignments/$assignment/access" "$student_cookie")"
	require_status "Student WeBWorK Assignment access" "$access" 200
	attempt="$(assignment_attempt_reference "$(response_body "$access")")"
	printf '%s\n%s\n' "$attempt" "$(response_body "$started")"
}

prove_render() {
	local instructor student course assignment webwork attempt started started_data resumed port
	instructor="$(persona_cookie elenaInstructor)"
	student="$(persona_cookie maryStudent)"
	read -r course assignment webwork < <(new_released_webwork_assignment "$instructor")
	started_data="$(new_student_attempt "$course" "$assignment" "$instructor" "$student")"
	attempt="${started_data%%$'\n'*}"; started="${started_data#*$'\n'}"
	assert_webwork_presentation "$started" "$webwork" false
	resumed="$(request "/api/course-instances/$course/assignments/$assignment/start" "$student" POST '{}')"
	require_status "Student WeBWorK Assignment resume" "$resumed" 201
	assert_webwork_presentation "$(response_body "$resumed")" "$webwork" true
	assert_same_presentation "$started" "$(response_body "$resumed")"
	port="$(gateway_port)"
	node tests/playwright/e2e_live_demo_webwork_browser.mjs "$port" "$course" "$assignment"
	echo "WeBWorK render: current Assignment entry, exact runtime Revision, answer-free presentation, and resume passed"
}

prove_grade() {
	local instructor student course assignment webwork attempt started started_data response saved finalized
	instructor="$(persona_cookie elenaInstructor)"
	student="$(persona_cookie maryStudent)"
	read -r course assignment webwork < <(new_released_webwork_assignment "$instructor")
	started_data="$(new_student_attempt "$course" "$assignment" "$instructor" "$student")"
	attempt="${started_data%%$'\n'*}"; started="${started_data#*$'\n'}"
	assert_webwork_presentation "$started" "$webwork" false
	response="$(webwork_response "$started")"
	saved="$(request "/api/assignment-attempts/$attempt/responses/1" "$student" PUT "$response")"
	require_status "Student WeBWorK response save" "$saved" 200
	assert_saved_response "$(response_body "$saved")" "$attempt"
	finalized="$(request "/api/assignment-attempts/$attempt/submission" "$student" POST '{}')"
	require_status "Student WeBWorK Assignment submission" "$finalized" 200
	assert_assignment_submitted "$(response_body "$finalized")" "$attempt"
	assert_webwork_job_state "$course" "$assignment" 'completed|graded'
	echo "WeBWorK grade: ordinary response save, submission, and worker grading passed"
}

require_live_demo
case "$mode" in
	render) prove_render ;;
	grade) prove_grade ;;
	all) prove_render; prove_grade ;;
esac
echo "Live Demo WeBWorK: PASS"
