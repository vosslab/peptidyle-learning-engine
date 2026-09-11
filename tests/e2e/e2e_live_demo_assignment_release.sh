#!/usr/bin/env bash
# Disposable service acceptance: Assignment Workspace and immutable release.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_assignment_release.sh [--service|--browser]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--service) mode="service" ;;
	--browser) mode="browser" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Assignment Release evidence requires the fixed Live Demo to be running" >&2
		exit 2
	fi
}

service_id() {
	local service="$1" identifiers
	identifiers="$(podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=$service")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Assignment Release evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Assignment Release evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1" cookie="${2:-}" method="${3:-GET}" body="${4:-}" if_match="${5:-}"
	local gateway port
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	local -a curl_args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}'
		--header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then
		curl_args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
	fi
	if [ -n "$cookie" ]; then curl_args+=(--header "Cookie: $cookie"); fi
	if [ -n "$if_match" ]; then curl_args+=(--header "If-Match: \"$if_match\""); fi
	if [ -n "$body" ]; then curl_args+=(--data "$body"); fi
	podman exec "$gateway" curl "${curl_args[@]}" "https://localhost:8080$path"
}

persona_cookie() {
	local persona="$1" gateway port headers cookie
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
		echo "Assignment Workspace access was not concealed" >&2
		exit 1
	fi
}

new_course_reference() {
	local instructor_cookie="$1" listed
	bash "$repository_root/tests/e2e/e2e_live_demo_course_instance.sh" --authority >/dev/null
	listed="$(request '/api/course-instances' "$instructor_cookie")"
	if [ "$(response_status "$listed")" != "200" ]; then
		echo "Instructor could not list the exact prerequisite Course Instance" >&2
		exit 1
	fi
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1]).get("items")
references=[item.get("reference") for item in items if isinstance(item,dict)] if isinstance(items,list) else []
if not references or any(not isinstance(v,str) or not re.fullmatch(r"C-[1-9][0-9]{0,9}",v) for v in references):
    raise SystemExit("Course Instance list lacks public identities")
print(max(references,key=lambda v:int(v[2:])))
' "$(response_body "$listed")"
}

picker_question_id() {
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1])
if not isinstance(items,list) or not items:
    raise SystemExit("Assignment Question Picker is not bounded Available Published Questions")
for item in items:
    if not isinstance(item,dict) or set(item)!={"questionId","description"}:
        raise SystemExit("Assignment Question Picker projection is not closed")
    if not isinstance(item["questionId"],str) or not re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}",item["questionId"]):
        raise SystemExit("Assignment Question Picker lacks opaque Published Question identities")
    if not isinstance(item["description"],str): raise SystemExit("Assignment Question Picker is malformed")
print(items[0]["questionId"])
' "$1"
}

workspace_reference_and_edit() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1])
required={"reference","editNumber","status","title","instructions","dueAt","lateWorkRule","assignmentAttemptTimeLimitSeconds","attemptLimit","activityRules","studentFeedbackReleaseRule","displayTimeZone","questions"}
if set(value)!=required or not re.fullmatch(r"A-[1-9][0-9]{0,9}",value["reference"]):
    raise SystemExit("Assignment Workspace projection is not closed")
if value["status"] != sys.argv[2] or not isinstance(value["editNumber"],str) or not value["editNumber"].isdigit():
    raise SystemExit("Assignment Workspace lifecycle or Edit Number is malformed")
if not isinstance(value["questions"],list): raise SystemExit("Assignment Workspace Question selection is malformed")
print(value["reference"], value["editNumber"])
' "$1" "$2"
}

workspace_save_payload() {
	python3 -c '
import json, sys
workspace=json.loads(sys.argv[1])
expected={"reference","editNumber","status","title","instructions","dueAt","lateWorkRule","assignmentAttemptTimeLimitSeconds","attemptLimit","activityRules","studentFeedbackReleaseRule","displayTimeZone","questions"}
if set(workspace) != expected:
    raise SystemExit("Assignment Workspace projection is not closed")
payload={
    "title": sys.argv[2],
    "instructions": sys.argv[3],
    "questionIds": [sys.argv[4]],
    "dueAt": workspace["dueAt"],
    "lateWorkRule": workspace["lateWorkRule"],
    "assignmentAttemptTimeLimitSeconds": int(sys.argv[5]),
    "attemptLimit": workspace["attemptLimit"],
    "activityRules": workspace["activityRules"],
    "studentFeedbackReleaseRule": workspace["studentFeedbackReleaseRule"],
}
print(json.dumps(payload, separators=(",", ":")))
' "$1" "$2" "$3" "$4" "$5"
}

assert_validation() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1])
expected={"canRelease":sys.argv[2]=="true","issues":([] if sys.argv[3]=="" else sys.argv[3].split(","))}
if value != expected: raise SystemExit("Assignment Release Validation did not report the exact current boundary")
' "$1" "$2" "$3"
}

assert_saved_workspace() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1]); question_id=sys.argv[2]; edit=sys.argv[3]; time_limit_seconds=int(sys.argv[4])
required={"reference","editNumber","status","title","instructions","dueAt","lateWorkRule","assignmentAttemptTimeLimitSeconds","attemptLimit","activityRules","studentFeedbackReleaseRule","displayTimeZone","questions"}
if (set(value)!=required or value.get("status")!="unreleased" or value.get("editNumber")!=edit
    or value.get("assignmentAttemptTimeLimitSeconds")!=time_limit_seconds
    or not isinstance(value.get("questions"),list) or len(value["questions"])!=1
    or set(value["questions"][0])!={"questionId","description"}
    or value["questions"][0].get("questionId")!=question_id):
    raise SystemExit("Assignment save did not retain the accepted fixed Question selection")
' "$1" "$2" "$3" "$4"
}

assert_preview() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1]); forbidden={"answer","answerKey","student","studentRecord","attempt","submission","scenario","id"}
if set(value)!={"title","instructions","questions"} or not isinstance(value["questions"],list) or len(value["questions"])!=1:
    raise SystemExit("Assignment Preview is not its narrow current projection")
def scan(item):
    if isinstance(item,dict):
        if forbidden.intersection(item): raise SystemExit("Assignment Preview exposed protected delivery state")
        for value in item.values(): scan(value)
    elif isinstance(item,list):
        for value in item: scan(value)
scan(value)
' "$1"
}

assert_release_receipt() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1])
if set(value)!={"reference","revisionNumber"} or value.get("reference")!=sys.argv[2] or value.get("revisionNumber")!=1:
    raise SystemExit("Assignment Release receipt did not select its first immutable revision")
' "$1" "$2"
}

assert_database_evidence() {
	local course_reference="$1" assignment_reference="$2" postgres sql output
	postgres="$(service_id postgres)"
	sql="DO \$\$
DECLARE v_course_id uuid; v_assignment_id uuid; v_revision_id uuid;
BEGIN
    SELECT course_id INTO v_course_id FROM ple_data.course_instance WHERE reference_number=${course_reference#C-};
    SELECT assignment_id, released_assignment_revision_id INTO v_assignment_id, v_revision_id
      FROM ple_data.assignment WHERE course_id=v_course_id AND reference_number=${assignment_reference#A-} AND assignment_status='released';
    IF v_course_id IS NULL OR v_assignment_id IS NULL OR v_revision_id IS NULL
       OR NOT EXISTS (SELECT 1 FROM ple_data.assignment_revision WHERE assignment_revision_id=v_revision_id AND assignment_id=v_assignment_id AND course_id=v_course_id AND revision_number=1)
       OR (SELECT count(*) FROM ple_data.assignment_revision_fixed_question WHERE assignment_revision_id=v_revision_id) <> 1
       OR EXISTS (SELECT 1 FROM ple_data.student_record WHERE course_id=v_course_id)
       OR EXISTS (SELECT 1 FROM ple_private.assignment_attempt WHERE assignment_id=v_assignment_id)
       OR EXISTS (SELECT 1 FROM ple_private.issued_question AS issued JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=issued.assignment_attempt_id WHERE attempt.assignment_id=v_assignment_id)
       OR EXISTS (SELECT 1 FROM ple_private.question_attempt AS question_attempt JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=issued.assignment_attempt_id WHERE attempt.assignment_id=v_assignment_id)
       OR EXISTS (SELECT 1 FROM ple_private.question_submission AS submission JOIN ple_private.question_attempt AS question_attempt ON question_attempt.question_attempt_id=submission.question_attempt_id JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=issued.assignment_attempt_id WHERE attempt.assignment_id=v_assignment_id)
       OR EXISTS (SELECT 1 FROM ple_private.assignment_submission AS submission JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=submission.assignment_attempt_id WHERE attempt.assignment_id=v_assignment_id)
    THEN RAISE EXCEPTION USING ERRCODE='42501', MESSAGE='Assignment Release atomic evidence is incomplete'; END IF;
END
\$\$;
SELECT 'assignment_release_authority';"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^assignment_release_authority$/p')" != "assignment_release_authority" ]; then
		echo "Assignment Release immutable persistence evidence was not recorded" >&2
		exit 1
	fi
}

prove_service() {
	local instructor_cookie student_cookie sysadmin_cookie course_reference picker question_id created assignment_reference initial_edit save_payload validation saved saved_edit stale retained preview released time_limit_seconds=1800
	instructor_cookie="$(persona_cookie elenaInstructor)"
	student_cookie="$(persona_cookie maryStudent)"
	sysadmin_cookie="$(persona_cookie morganSysadmin)"
	course_reference="$(new_course_reference "$instructor_cookie")"
	assert_concealed "$(request "/api/course-instances/$course_reference/assignment-question-picker")"
	assert_concealed "$(request "/api/course-instances/$course_reference/assignment-question-picker" "$student_cookie")"
	assert_concealed "$(request "/api/course-instances/$course_reference/assignment-question-picker" "$sysadmin_cookie")"
	picker="$(request "/api/course-instances/$course_reference/assignment-question-picker" "$instructor_cookie")"
	if [ "$(response_status "$picker")" != "200" ]; then echo "Instructor could not load the Assignment Question Picker" >&2; exit 1; fi
	question_id="$(picker_question_id "$(response_body "$picker")")"
	created="$(request "/api/course-instances/$course_reference/assignments" "$instructor_cookie" POST '{"title":"M10 live assignment","instructions":"Complete the selected published question."}')"
	if [ "$(response_status "$created")" != "201" ]; then echo "Instructor could not create an Unreleased Assignment" >&2; exit 1; fi
	read -r assignment_reference initial_edit < <(workspace_reference_and_edit "$(response_body "$created")" unreleased)
	save_payload="$(workspace_save_payload "$(response_body "$created")" "M10 live assignment" "Complete the selected published question." "$question_id" "$time_limit_seconds")"
	validation="$(request "/api/course-instances/$course_reference/assignments/$assignment_reference/release-validation" "$instructor_cookie")"
	if [ "$(response_status "$validation")" != "200" ]; then echo "Instructor could not validate an Unreleased Assignment" >&2; exit 1; fi
	assert_validation "$(response_body "$validation")" false timeLimitRequired,noPublishedQuestions
	saved="$(request "/api/course-instances/$course_reference/assignments/$assignment_reference" "$instructor_cookie" PUT "$save_payload" "$initial_edit")"
	if [ "$(response_status "$saved")" != "200" ]; then echo "Instructor could not save the fixed Question selection" >&2; exit 1; fi
	saved_edit="$(workspace_reference_and_edit "$(response_body "$saved")" unreleased | awk '{print $2}')"
	if [ "$saved_edit" = "$initial_edit" ]; then echo "Assignment save did not issue a new Edit Number" >&2; exit 1; fi
	assert_saved_workspace "$(response_body "$saved")" "$question_id" "$saved_edit" "$time_limit_seconds"
	stale="$(request "/api/course-instances/$course_reference/assignments/$assignment_reference" "$instructor_cookie" PUT "$save_payload" "$initial_edit")"
	if [ "$(response_status "$stale")" != "412" ]; then echo "Stale Assignment save did not fail its Edit Number precondition" >&2; exit 1; fi
	retained="$(request "/api/course-instances/$course_reference/assignments/$assignment_reference" "$instructor_cookie")"
	if [ "$(response_status "$retained")" != "200" ]; then echo "Instructor could not reload the accepted Assignment state" >&2; exit 1; fi
	assert_saved_workspace "$(response_body "$retained")" "$question_id" "$saved_edit" "$time_limit_seconds"
	validation="$(request "/api/course-instances/$course_reference/assignments/$assignment_reference/release-validation" "$instructor_cookie")"
	if [ "$(response_status "$validation")" != "200" ]; then echo "Instructor could not validate the selected Assignment" >&2; exit 1; fi
	assert_validation "$(response_body "$validation")" true ''
	preview="$(request "/api/course-instances/$course_reference/assignments/$assignment_reference/preview" "$instructor_cookie")"
	if [ "$(response_status "$preview")" != "200" ]; then echo "Instructor could not load the Assignment Preview" >&2; exit 1; fi
	assert_preview "$(response_body "$preview")"
	released="$(request "/api/course-instances/$course_reference/assignments/$assignment_reference/release" "$instructor_cookie" POST '{}' "$saved_edit")"
	if [ "$(response_status "$released")" != "201" ]; then echo "Instructor could not release the validated Assignment" >&2; exit 1; fi
	assert_release_receipt "$(response_body "$released")" "$assignment_reference"
	assert_database_evidence "$course_reference" "$assignment_reference"
	echo "Assignment Release authority: direct Instructor workspace and immutable fixed-question release complete"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	node tests/playwright/e2e_live_demo_assignment_release_browser.mjs "$port"
	echo "Assignment Release browser: visible Instructor authoring, preview, and release complete"
}

require_live_demo
case "$mode" in
	service) prove_service ;;
	browser) prove_browser ;;
	all) prove_service; prove_browser ;;
esac
echo "Live Demo Assignment Release: PASS"
