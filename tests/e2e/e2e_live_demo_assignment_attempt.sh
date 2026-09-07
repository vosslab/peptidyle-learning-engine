#!/usr/bin/env bash
# Disposable M11 acceptance: Student Assignment Access and answer-free start.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_assignment_attempt.sh [--start|--browser]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--start) mode="start" ;;
	--browser) mode="browser" ;;
	*) usage; exit 2 ;;
esac

prepared_course=""
prepared_assignment=""

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Assignment Attempt evidence requires the fixed Live Demo to be running" >&2
		exit 2
	fi
}

service_id() {
	local service="$1" identifiers
	identifiers="$(podman ps -q --filter "label=com.docker.compose.project=$project_name" --filter "label=com.docker.compose.service=$service")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Assignment Attempt evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Assignment Attempt evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1" cookie="${2:-}" method="${3:-GET}" body="${4:-}" if_match="${5:-}"
	local gateway port
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	local -a args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}' --header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json'); fi
	if [ -n "$cookie" ]; then args+=(--header "Cookie: $cookie"); fi
	if [ -n "$if_match" ]; then args+=(--header "If-Match: \"$if_match\""); fi
	if [ -n "$body" ]; then args+=(--data "$body"); fi
	podman exec "$gateway" curl "${args[@]}" "https://localhost:8080$path"
}

persona_cookie() {
	local persona="$1" gateway port headers cookie
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 --dump-header - --output /dev/null --header "Host: localhost:$port" --header "Origin: https://localhost:$port" --header 'Content-Type: application/json' --request POST --data "{\"persona\":\"$persona\"}" 'https://localhost:8080/api/auth/live-demo/accounts')"
	cookie="$(printf '%s\n' "$headers" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"
	if [ -z "$cookie" ]; then echo "seeded demo did not issue an Authenticated Session" >&2; exit 1; fi
	printf '%s\n' "$cookie"
}

response_status() { printf '%s' "${1##*$'\n'}"; }
response_body() { printf '%s' "${1%$'\n'*}"; }

assert_concealed() {
	if [ "$(response_status "$1")" != "404" ]; then echo "Student Assignment Access was not concealed" >&2; exit 1; fi
}

assert_access() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1])
if value != {"startDecision":sys.argv[2]}:
    raise SystemExit("Assignment Access projection did not report the server decision")
' "$1" "$2"
}

assert_started() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); expected_resumed=sys.argv[2] == "true"; expected_number=int(sys.argv[3])
required={"assignment","attemptNumber","resumed","title","instructions","questions"}
if set(value) != required or not re.fullmatch(r"A-[1-9][0-9]{0,9}", value["assignment"]):
    raise SystemExit("Assignment start projection is not closed")
if value["resumed"] is not expected_resumed or value["attemptNumber"] != expected_number:
    raise SystemExit("Assignment start did not preserve the exact Attempt lifecycle")
if not isinstance(value["title"],str) or not isinstance(value["instructions"],str) or not isinstance(value["questions"],list) or len(value["questions"]) != 1:
    raise SystemExit("Assignment start omitted its released answer-free presentation")
for question in value["questions"]:
    if not isinstance(question,dict) or set(question) != {"questionRevision","question_seed","presentationNonce","questionTitle","prompt","response"}:
        raise SystemExit("Issued Question presentation is not closed")
    revision=question["questionRevision"]
    if (not isinstance(revision,dict) or set(revision) != {"questionId","revisionNumber"}
        or not isinstance(revision["questionId"],str) or not re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}",revision["questionId"])
        or not isinstance(revision["revisionNumber"],int) or revision["revisionNumber"] < 1
        or not isinstance(question["question_seed"],int) or question["question_seed"] < 0
        or not isinstance(question["presentationNonce"],str) or not re.fullmatch(r"[0-9a-f]{32}",question["presentationNonce"])
        or not isinstance(question["questionTitle"],str) or not isinstance(question["prompt"],list)
        or not isinstance(question["response"],dict) or not isinstance(question["response"].get("kind"),str)):
        raise SystemExit("Issued Question presentation is malformed")
forbidden={"answer","answerKey","answers","correct","correctness","submission","grade","grading","score","student","studentRecord","assignmentAttemptId","questionAttemptId","binding","checksum","repro"}
def scan(item):
    if isinstance(item,dict):
        if forbidden.intersection(item): raise SystemExit("Assignment start exposed later Student Work state")
        for child in item.values(): scan(child)
    elif isinstance(item,list):
        for child in item: scan(child)
scan(value)
' "$1" "$2" "$3"
}

assert_same_presentation() {
	python3 -c '
import json, sys
first=json.loads(sys.argv[1]); resumed=json.loads(sys.argv[2])
if first.get("questions") != resumed.get("questions"):
    raise SystemExit("Assignment resume reminted the public Question Presentation")
' "$1" "$2"
}

released_references() {
	local postgres="$1" output
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "SELECT '\''C-'\'' || course.reference_number || '\'' A-'\'' || assignment.reference_number FROM ple_data.course_instance AS course JOIN ple_data.assignment AS assignment ON assignment.course_id=course.course_id WHERE assignment.assignment_status='\''released'\'' ORDER BY course.reference_number DESC, assignment.reference_number DESC LIMIT 1"')"
	if ! printf '%s\n' "$output" | rg -q '^C-[1-9][0-9]* A-[1-9][0-9]*$'; then echo "Released Assignment prerequisite lacks public references" >&2; exit 1; fi
	printf '%s\n' "$output"
}

claim_student_record() {
	local course="$1" instructor_cookie="$2" student_cookie="$3" imported claimed
	imported="$(request "/api/course-instances/$course/roster" "$instructor_cookie" POST '{"entries":[{"email":"mary.student@live-demo.invalid","rosterId":"m11-student"}]}')"
	if [ "$(response_status "$imported")" != "201" ]; then echo "Instructor could not import the M11 Student roster row" >&2; exit 1; fi
	claimed="$(request "/api/course-instances/$course/roster/claim" "$student_cookie" POST '{}')"
	if [ "$(response_status "$claimed")" != "200" ] || [ "$(response_body "$claimed")" != '{"activeStudentMembership":true}' ]; then echo "Student could not claim the exact Course Invitation" >&2; exit 1; fi
}

picker_question_id() {
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1])
if not isinstance(items,list) or not items or not isinstance(items[0],dict) or set(items[0])!={"questionId","description"} or not isinstance(items[0]["questionId"],str) or not re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}",items[0]["questionId"]):
    raise SystemExit("Assignment Question Picker is malformed")
print(items[0]["questionId"])
' "$1"
}

assignment_reference_and_edit() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1])
if not isinstance(value,dict) or not re.fullmatch(r"A-[1-9][0-9]{0,9}",value.get("reference","")) or not isinstance(value.get("editNumber"),str) or not value["editNumber"].isdigit():
    raise SystemExit("Assignment Workspace receipt is malformed")
print(value["reference"],value["editNumber"])
' "$1"
}

create_reject_late_assignment() {
	local course="$1" instructor_cookie="$2" picker question_id created assignment edit saved released
	picker="$(request "/api/course-instances/$course/assignment-question-picker" "$instructor_cookie")"
	if [ "$(response_status "$picker")" != "200" ]; then echo "Instructor could not load the late-work Question Picker" >&2; exit 1; fi
	question_id="$(picker_question_id "$(response_body "$picker")")"
	created="$(request "/api/course-instances/$course/assignments" "$instructor_cookie" POST '{"title":"M11 late assignment","instructions":"Complete the selected published question."}')"
	if [ "$(response_status "$created")" != "201" ]; then echo "Instructor could not create the late-work Assignment" >&2; exit 1; fi
	read -r assignment edit < <(assignment_reference_and_edit "$(response_body "$created")")
	saved="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor_cookie" PUT "{\"title\":\"M11 late assignment\",\"instructions\":\"Complete the selected published question.\",\"questionIds\":[\"$question_id\"],\"dueAt\":\"2026-09-01T00:00:00.000\",\"lateWorkRule\":\"reject\"}" "$edit")"
	if [ "$(response_status "$saved")" != "200" ]; then echo "Instructor could not save the late-work Assignment policy" >&2; exit 1; fi
	edit="$(assignment_reference_and_edit "$(response_body "$saved")" | awk '{print $2}')"
	released="$(request "/api/course-instances/$course/assignments/$assignment/release" "$instructor_cookie" POST '{}' "$edit")"
	if [ "$(response_status "$released")" != "201" ]; then echo "Instructor could not release the late-work Assignment" >&2; exit 1; fi
	printf '%s\n' "$assignment"
}

assert_no_started_attempt() {
	local course="$1" assignment="$2" postgres sql output
	postgres="$(service_id postgres)"
	sql="SELECT CASE WHEN EXISTS (SELECT 1 FROM ple_private.assignment_attempt AS attempt JOIN ple_data.assignment AS assignment ON assignment.assignment_id=attempt.assignment_id JOIN ple_data.course_instance AS course ON course.course_id=assignment.course_id WHERE course.reference_number=${course#C-} AND assignment.reference_number=${assignment#A-}) THEN 'unexpected_attempt' ELSE 'no_started_attempt' END;"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^no_started_attempt$/p')" != "no_started_attempt" ]; then echo "Late-work refusal issued an Assignment Attempt" >&2; exit 1; fi
}

assert_m11_rls_catalog() {
	local postgres sql output
	postgres="$(service_id postgres)"
	sql="DO \$\$
DECLARE v_api_owner oid := (SELECT oid FROM pg_catalog.pg_roles WHERE rolname='ple_api_owner');
BEGIN
 IF v_api_owner IS NULL
    OR (SELECT count(*) FROM pg_catalog.pg_class AS relation JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=relation.relnamespace WHERE namespace.nspname='ple_data' AND relation.relname IN ('assignment_revision_entry','assignment_revision_fixed_question') AND relation.relrowsecurity AND relation.relforcerowsecurity) <> 2
    OR (SELECT count(*) FROM pg_catalog.pg_policy AS policy JOIN pg_catalog.pg_class AS relation ON relation.oid=policy.polrelid JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=relation.relnamespace WHERE namespace.nspname='ple_data' AND ((relation.relname='assignment_revision_entry' AND policy.polname='assignment_revision_entry_api_owner_live_demo_m11_read') OR (relation.relname='assignment_revision_fixed_question' AND policy.polname='assignment_revision_fixed_question_api_owner_live_demo_m11_read')) AND policy.polcmd='r' AND policy.polroles=ARRAY[v_api_owner]) <> 2
    OR EXISTS (SELECT 1 FROM pg_catalog.pg_policy AS policy JOIN pg_catalog.pg_class AS relation ON relation.oid=policy.polrelid JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=relation.relnamespace WHERE namespace.nspname='ple_data' AND policy.polname IN ('assignment_revision_entry_api_owner_live_demo_m11_read','assignment_revision_fixed_question_api_owner_live_demo_m11_read') AND (policy.polcmd <> 'r' OR policy.polroles <> ARRAY[v_api_owner] OR (policy.polname='assignment_revision_entry_api_owner_live_demo_m11_read' AND relation.relname <> 'assignment_revision_entry') OR (policy.polname='assignment_revision_fixed_question_api_owner_live_demo_m11_read' AND relation.relname <> 'assignment_revision_fixed_question')))
    OR EXISTS (SELECT 1 FROM pg_catalog.pg_class AS relation JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=relation.relnamespace CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(relation.relacl, pg_catalog.acldefault('r', relation.relowner))) AS grant_item WHERE namespace.nspname='ple_data' AND relation.relname IN ('assignment_revision_entry','assignment_revision_fixed_question') AND grant_item.privilege_type='SELECT' AND grant_item.grantee=0)
    OR EXISTS (SELECT 1 FROM pg_catalog.pg_class AS relation JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=relation.relnamespace WHERE namespace.nspname='ple_data' AND relation.relname IN ('assignment_revision_entry','assignment_revision_fixed_question') AND pg_catalog.has_table_privilege('ple_app', relation.oid, 'SELECT'))
 THEN RAISE EXCEPTION USING ERRCODE='42501', MESSAGE='M11 RLS catalog evidence is incomplete'; END IF;
END \$\$;
SELECT 'm11_rls_catalog_authority';"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^m11_rls_catalog_authority$/p')" != "m11_rls_catalog_authority" ]; then echo "M11 RLS catalog evidence was not recorded" >&2; exit 1; fi
}

assert_database_evidence() {
	local course="$1" assignment="$2" postgres sql output
	postgres="$(service_id postgres)"
	sql="DO \$\$
DECLARE v_course_id uuid; v_assignment_id uuid; v_revision_id uuid; v_student_record_id uuid;
BEGIN
 SELECT course_id INTO v_course_id FROM ple_data.course_instance WHERE reference_number=${course#C-};
 SELECT assignment_id,released_assignment_revision_id INTO v_assignment_id,v_revision_id FROM ple_data.assignment WHERE course_id=v_course_id AND reference_number=${assignment#A-} AND assignment_status='released';
 SELECT student_record_id INTO v_student_record_id FROM ple_data.student_record WHERE course_id=v_course_id AND student_account_id=(SELECT account_id FROM ple_private.account_authentication_email WHERE normalized_email='mary.student@live-demo.invalid');
 IF v_course_id IS NULL OR v_assignment_id IS NULL OR v_revision_id IS NULL OR v_student_record_id IS NULL
    OR (SELECT count(*) FROM ple_private.assignment_attempt WHERE assignment_id=v_assignment_id AND student_record_id=v_student_record_id) <> 1
    OR (SELECT count(*) FROM ple_private.issued_question AS issued JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=issued.assignment_attempt_id WHERE attempt.assignment_id=v_assignment_id AND attempt.student_record_id=v_student_record_id) <> 1
    OR (SELECT count(*) FROM ple_private.question_attempt AS question_attempt JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=issued.assignment_attempt_id WHERE attempt.assignment_id=v_assignment_id AND attempt.student_record_id=v_student_record_id) <> 1
    OR (SELECT count(*) FROM ple_private.question_attempt_presentation_binding AS binding JOIN ple_private.question_attempt AS question_attempt ON question_attempt.question_attempt_id=binding.question_attempt_id JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=issued.assignment_attempt_id WHERE attempt.assignment_id=v_assignment_id AND attempt.student_record_id=v_student_record_id) <> 1
    OR NOT EXISTS (SELECT 1 FROM ple_private.question_attempt AS question_attempt JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=issued.assignment_attempt_id WHERE attempt.assignment_id=v_assignment_id AND attempt.student_record_id=v_student_record_id AND question_attempt.question_seed >= 0 AND question_attempt.generated_parameter_sha256 ~ '^[0-9a-f]{64}$' AND jsonb_typeof(question_attempt.reproduction_details) = 'object')
    OR EXISTS (SELECT 1 FROM ple_private.question_submission AS submission JOIN ple_private.question_attempt AS question_attempt ON question_attempt.question_attempt_id=submission.question_attempt_id JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=issued.assignment_attempt_id WHERE attempt.assignment_id=v_assignment_id AND attempt.student_record_id=v_student_record_id)
    OR EXISTS (SELECT 1 FROM ple_private.assignment_submission AS submission JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=submission.assignment_attempt_id WHERE attempt.assignment_id=v_assignment_id AND attempt.student_record_id=v_student_record_id)
    OR NOT EXISTS (SELECT 1 FROM ple_private.issued_question AS issued JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=issued.assignment_attempt_id JOIN ple_data.assignment_revision_fixed_question AS fixed_question ON fixed_question.assignment_revision_id=v_revision_id AND fixed_question.question_id=issued.question_id AND fixed_question.revision_number=issued.revision_number WHERE attempt.assignment_id=v_assignment_id AND attempt.student_record_id=v_student_record_id)
 THEN RAISE EXCEPTION USING ERRCODE='42501', MESSAGE='Assignment Attempt atomic evidence is incomplete'; END IF;
END \$\$;
SELECT 'assignment_attempt_authority';"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^assignment_attempt_authority$/p')" != "assignment_attempt_authority" ]; then echo "Assignment Attempt persistence evidence was not recorded" >&2; exit 1; fi
	assert_m11_rls_catalog
}

prove_start() {
	local instructor_cookie student_cookie sysadmin_cookie postgres references course assignment access started resumed expired_assignment rejected
	# M10 is the exact public authoring/release predecessor; it creates only one released fixed Question Assignment.
	bash "$repository_root/tests/e2e/e2e_live_demo_assignment_release.sh" --service >/dev/null
	instructor_cookie="$(persona_cookie elenaInstructor)"; student_cookie="$(persona_cookie maryStudent)"; sysadmin_cookie="$(persona_cookie morganSysadmin)"
	postgres="$(service_id postgres)"; references="$(released_references "$postgres")"; read -r course assignment <<<"$references"
	prepared_course="$course"
	prepared_assignment="$assignment"
	assert_concealed "$(request "/api/course-instances/$course/assignments/$assignment/access")"
	assert_concealed "$(request "/api/course-instances/$course/assignments/$assignment/access" "$instructor_cookie")"
	assert_concealed "$(request "/api/course-instances/$course/assignments/$assignment/start" "$sysadmin_cookie" POST '{}')"
	assert_concealed "$(request "/api/course-instances/$course/assignments/$assignment/access" "$student_cookie")"
	claim_student_record "$course" "$instructor_cookie" "$student_cookie"
	access="$(request "/api/course-instances/$course/assignments/$assignment/access" "$student_cookie")"
	if [ "$(response_status "$access")" != "200" ]; then echo "Student could not load current Assignment Access" >&2; exit 1; fi
	assert_access "$(response_body "$access")" may_start
	started="$(request "/api/course-instances/$course/assignments/$assignment/start" "$student_cookie" POST '{}')"
	if [ "$(response_status "$started")" != "201" ]; then echo "Student could not start the released Assignment" >&2; exit 1; fi
	assert_started "$(response_body "$started")" false 1
	access="$(request "/api/course-instances/$course/assignments/$assignment/access" "$student_cookie")"
	if [ "$(response_status "$access")" != "200" ]; then echo "Student could not read the current resumable Assignment Access" >&2; exit 1; fi
	assert_access "$(response_body "$access")" may_start
	resumed="$(request "/api/course-instances/$course/assignments/$assignment/start" "$student_cookie" POST '{}')"
	if [ "$(response_status "$resumed")" != "201" ]; then echo "Student could not resume the started Assignment" >&2; exit 1; fi
	assert_started "$(response_body "$resumed")" true 1
	assert_same_presentation "$(response_body "$started")" "$(response_body "$resumed")"
	assert_database_evidence "$course" "$assignment"
	# A second Assignment sets its policy through M10 before immutable release.
	expired_assignment="$(create_reject_late_assignment "$course" "$instructor_cookie")"
	access="$(request "/api/course-instances/$course/assignments/$expired_assignment/access" "$student_cookie")"
	if [ "$(response_status "$access")" != "200" ]; then echo "Student could not read the due Assignment Access" >&2; exit 1; fi
	assert_access "$(response_body "$access")" late_work_refused
	rejected="$(request "/api/course-instances/$course/assignments/$expired_assignment/start" "$student_cookie" POST '{}')"
	assert_concealed "$rejected"
	assert_no_started_attempt "$course" "$expired_assignment"
	echo "Assignment Attempt authority: Student-only current access, atomic start/resume, released fixed-question pinning, and persisted answer-free Question Presentation complete"
}

run_browser() {
	local port
	if [ -z "$prepared_course" ] || [ -z "$prepared_assignment" ]; then
		echo "Assignment Attempt browser prerequisite did not produce public references" >&2
		exit 1
	fi
	port="$(gateway_port)"
	node tests/playwright/e2e_live_demo_assignment_attempt_browser.mjs "$port" "$prepared_course" "$prepared_assignment"
	echo "Assignment Attempt browser: visible Student start and answer-free presentation complete"
}

prove_browser() {
	prepare_browser_prerequisite
	run_browser
}

prepare_browser_prerequisite() {
	local instructor_cookie student_cookie postgres references
	# Keep this fresh release available and unstarted for the visible Student flow.
	bash "$repository_root/tests/e2e/e2e_live_demo_assignment_release.sh" --service >/dev/null
	instructor_cookie="$(persona_cookie elenaInstructor)"
	student_cookie="$(persona_cookie maryStudent)"
	postgres="$(service_id postgres)"
	references="$(released_references "$postgres")"
	read -r prepared_course prepared_assignment <<<"$references"
	claim_student_record "$prepared_course" "$instructor_cookie" "$student_cookie"
}

require_live_demo
case "$mode" in
	start) prove_start ;;
	browser) prove_browser ;;
	all) prove_start; prove_browser ;;
esac
echo "Live Demo Assignment Attempt: PASS"
