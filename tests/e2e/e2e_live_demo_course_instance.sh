#!/usr/bin/env bash
# Disposable M8 acceptance: exact Blueprint source, Assigned Instructor, and teaching-team browser entry.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_course_instance.sh [--authority|--browser]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--authority) mode="authority" ;;
	--browser) mode="browser" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Course Instance evidence requires the fixed Live Demo to be running" >&2
		exit 2
	fi
}

service_id() {
	local service="$1"
	local identifiers
	identifiers="$(podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=$service")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Course Instance evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Course Instance evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1"
	local cookie="${2:-}"
	local method="${3:-GET}"
	local body="${4:-}"
	local gateway port
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	local -a curl_args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}'
		--header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then
		curl_args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
	fi
	if [ -n "$cookie" ]; then curl_args+=(--header "Cookie: $cookie"); fi
	if [ -n "$body" ]; then curl_args+=(--data "$body"); fi
	podman exec "$gateway" curl "${curl_args[@]}" "https://localhost:8080$path"
}

persona_cookie() {
	local persona="$1"
	local gateway port headers cookie
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 \
		--dump-header - --output /dev/null --header "Host: localhost:$port" \
		--header "Origin: https://localhost:$port" --header 'Content-Type: application/json' \
		--request POST --data "{\"persona\":\"$persona\"}" \
		'https://localhost:8080/api/auth/live-demo/accounts')"
	cookie="$(printf '%s\n' "$headers" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"
	if [ -z "$cookie" ]; then
		echo "seeded demo did not issue an Authenticated Session" >&2
		exit 1
	fi
	printf '%s\n' "$cookie"
}

response_status() { printf '%s' "${1##*$'\n'}"; }
response_body() { printf '%s' "${1%$'\n'*}"; }

assert_concealed() {
	if [ "$(response_status "$1")" != "404" ]; then
		echo "Course Instance access was not concealed" >&2
		exit 1
	fi
}

first_published_question_id() {
	python3 -c '
import json, re, sys
items = json.loads(sys.argv[1]).get("items")
if not isinstance(items, list) or not items:
    raise SystemExit("Question Library did not return a published Question")
question_id = items[0].get("summary", {}).get("questionId")
if not isinstance(question_id, str) or not re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}", question_id):
    raise SystemExit("Question Library did not return an opaque Question ID")
print(question_id)
' "$1"
}

blueprint_payload() {
	python3 -c '
import json, sys
question_id = sys.argv[1]
assignment = {
  "title": "Course Instance source assignment",
  "instructions": "Use the published Question in reusable course structure.",
  "entries": [{"kind":"fixed","question_id":question_id,"points_possible":"1","scoring_rule":"normal","question_attempt_limit":{"maxAttempts":None},"question_attempt_time_limit":{"kind":"unlimited"}}],
  "defaults": {"assignment_attempt_time_limit_seconds":None,"attempt_limit":2,"late_work_rule":"accept","assignment_deadline_rule":"auto_submit","activity_rules":{"assignmentCompletionRule":{"kind":"answerAll"},"assignmentAttemptGradeRule":"highest","assignmentAttemptContinuationRule":{"kind":"unlimited"},"questionPoolReuseRule":"reuseSelection","questionVariationRule":"newVariation","assignmentAttemptResumeRule":"resumable","assignmentQuestionDisplayRule":"allQuestions","assignmentNavigationRule":"freeNavigation","assignmentQuestionOrderRule":"authoredOrder"},"student_feedback_release_rule":{"score":"after_submit","per_item_correctness":"after_submit","question_feedback":"after_submit","question_answer":"never","question_answer_explanation":"never","class_statistics":"never"}},
  "schedule":{"available_at":None,"due_at":None,"closes_at":None},
}
print(json.dumps({"title":"M8 exact Blueprint source","modules":[{"label":"M8 module","assignments":[assignment]}]}, separators=(",",":")))
' "$1"
}

course_payload() {
	python3 -c '
import json, sys
blueprint, revision, assigned = sys.argv[1:]
print(json.dumps({"blueprintCourse":blueprint,"blueprintRevision":revision,"title":"M8 live Course Instance","term":{"startDate":"2026-09-01","endDate":"2026-12-18","timeZone":"America/Chicago"},"assignedInstructor":assigned}, separators=(",",":")))
' "$1" "$2" "$3"
}

assert_course_receipt() {
	python3 -c '
import json, re, sys
value = json.loads(sys.argv[1])
if set(value) != {"course", "creatorIsAssignedInstructor"}:
    raise SystemExit("Course Instance creation receipt was not closed")
course = value["course"]
if set(course) != {"reference", "title", "term"} or not re.fullmatch(r"C-[1-9][0-9]{0,9}", course["reference"]):
    raise SystemExit("Course Instance creation receipt did not return a public Course Instance identity")
if course["title"] != "M8 live Course Instance" or value["creatorIsAssignedInstructor"] is not False:
    raise SystemExit("Sysadmin Course Instance creation did not preserve its no-ambient-access receipt")
print(course["reference"])
' "$1"
}

assert_instructor_view() {
	python3 -c '
import json, sys
value = json.loads(sys.argv[1])
if set(value) != {"course", "isAssignedInstructor", "activeInstructorCount"}:
    raise SystemExit("Course Instance teaching-team view was not closed")
course = value["course"]
if set(course) != {"reference", "title", "term"} or course["reference"] != sys.argv[2]:
    raise SystemExit("Course Instance teaching-team view identity differs")
if value["isAssignedInstructor"] is not True or value["activeInstructorCount"] != 1:
    raise SystemExit("Assigned Instructor did not receive exactly initial teaching authority")
forbidden = {"id", "accountId", "student", "studentRecord", "assignment", "sourceObject", "answerKey"}
if forbidden.intersection(value) or forbidden.intersection(course):
    raise SystemExit("Course Instance teaching-team view exposed future or private state")
' "$1" "$2"
}

assert_database_evidence() {
	local course_reference="$1"
	local blueprint_reference="$2"
	local instructor_reference="$3"
	local postgres output sql
	postgres="$(service_id postgres)"
	sql="DO \$\$
DECLARE
    v_course_id uuid;
    v_assigned_instructor_id uuid;
BEGIN
    SELECT course.course_id, course.assigned_instructor_account_id
      INTO v_course_id, v_assigned_instructor_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = ${course_reference#C-}
       AND course.blueprint_course_reference_number = ${blueprint_reference#BP-}
       AND course.blueprint_revision_number = 1;
    IF v_course_id IS NULL
       OR NOT EXISTS (SELECT 1 FROM ple_data.course_origin AS origin
                      WHERE origin.course_id = v_course_id
                        AND origin.source_course_id IS NULL
                        AND origin.blueprint_course_reference_number = ${blueprint_reference#BP-}
                        AND origin.blueprint_revision_number = 1)
       OR NOT EXISTS (SELECT 1 FROM ple_data.course_membership AS membership
                      WHERE membership.course_id = v_course_id
                        AND membership.account_id = v_assigned_instructor_id
                        AND membership.role = 'instructor'
                        AND ple_data.course_membership_is_active(membership.membership_id))
       OR NOT EXISTS (SELECT 1 FROM ple_audit.course_instance_creation_event AS event
                      WHERE event.course_id = v_course_id
                        AND event.assigned_instructor_account_id = v_assigned_instructor_id
                        AND event.created_by_account_id <> v_assigned_instructor_id)
       OR EXISTS (SELECT 1 FROM ple_data.student_record WHERE course_id = v_course_id)
       OR EXISTS (SELECT 1 FROM ple_data.assignment WHERE course_id = v_course_id)
       OR NOT EXISTS (SELECT 1 FROM ple_private.account
                      WHERE account_id = v_assigned_instructor_id
                        AND reference_number = ${instructor_reference#U-})
    THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Instance atomic evidence is incomplete';
    END IF;
END
\$\$;
SELECT 'course_instance_authority';"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^course_instance_authority$/p')" != "course_instance_authority" ]; then
		echo "Course Instance atomic persistence evidence was not recorded" >&2
		exit 1
	fi
}

prove_authority() {
	local instructor_cookie sysadmin_cookie student_cookie library question_id created blueprint reference candidates assigned course_created course_reference course_list course_view
	instructor_cookie="$(persona_cookie elenaInstructor)"
	sysadmin_cookie="$(persona_cookie morganSysadmin)"
	student_cookie="$(persona_cookie maryStudent)"
	assert_concealed "$(request '/api/course-instances')"
	assert_concealed "$(request '/api/course-instances' "$student_cookie")"
	library="$(request '/api/questions/search?page_size=50' "$instructor_cookie")"
	if [ "$(response_status "$library")" != "200" ]; then
		echo "Instructor could not select a Published Question for an exact Blueprint source" >&2
		exit 1
	fi
	question_id="$(first_published_question_id "$(response_body "$library")")"
	created="$(request '/api/course-blueprints' "$instructor_cookie" POST "$(blueprint_payload "$question_id")")"
	if [ "$(response_status "$created")" != "201" ]; then
		echo "Instructor could not create the exact Blueprint source" >&2
		exit 1
	fi
	read -r blueprint reference < <(python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); reference=value.get("reference"); revision=value.get("revision")
if not isinstance(reference,str) or not re.fullmatch(r"BP-[1-9][0-9]{0,9}",reference) or revision != "1":
    raise SystemExit("exact Blueprint source creation did not return its first immutable revision")
print(reference, revision)
' "$(response_body "$created")")
	candidates="$(request '/api/course-instance-creation/instructors' "$sysadmin_cookie")"
	if [ "$(response_status "$candidates")" != "200" ]; then
		echo "Sysadmin could not obtain the bounded Assigned Instructor selection" >&2
		exit 1
	fi
	assigned="$(python3 -c '
import json, re, sys
items=json.loads(sys.argv[1]).get("items")
if not isinstance(items,list) or len(items) != 1 or not isinstance(items[0],dict):
    raise SystemExit("Assigned Instructor selection is not bounded")
reference=items[0].get("reference")
if not isinstance(reference,str) or not re.fullmatch(r"U-[1-9][0-9]{0,9}",reference):
    raise SystemExit("Assigned Instructor selection lacks a public Account Reference")
print(reference)
' "$(response_body "$candidates")")"
	course_created="$(request '/api/course-instances' "$sysadmin_cookie" POST "$(course_payload "$blueprint" "$reference" "$assigned")")"
	if [ "$(response_status "$course_created")" != "201" ]; then
		echo "Sysadmin could not create the Course Instance for the selected Instructor" >&2
		exit 1
	fi
	course_reference="$(assert_course_receipt "$(response_body "$course_created")")"
	assert_concealed "$(request '/api/course-instances' "$sysadmin_cookie")"
	assert_concealed "$(request "/api/course-instances/$course_reference" "$sysadmin_cookie")"
	course_list="$(request '/api/course-instances' "$instructor_cookie")"
	if [ "$(response_status "$course_list")" != "200" ]; then
		echo "Assigned Instructor could not enter the new Course Instance list" >&2
		exit 1
	fi
	course_view="$(request "/api/course-instances/$course_reference" "$instructor_cookie")"
	if [ "$(response_status "$course_view")" != "200" ]; then
		echo "Assigned Instructor could not open the new Course Instance" >&2
		exit 1
	fi
	assert_instructor_view "$(response_body "$course_view")" "$course_reference"
	assert_database_evidence "$course_reference" "$blueprint" "$assigned"
	echo "Course Instance authority: exact source, Assigned Instructor, and no ambient Sysadmin access complete"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	node tests/playwright/e2e_live_demo_course_instance_browser.mjs "$port"
	echo "Course Instance browser: visible Instructor creation and Teaching Team entry complete"
}

require_live_demo
case "$mode" in
	authority) prove_authority ;;
	browser) prove_browser ;;
	all) prove_authority; prove_browser ;;
esac

echo "Live Demo Course Instance: PASS"
