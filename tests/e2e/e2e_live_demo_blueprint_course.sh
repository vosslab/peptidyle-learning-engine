#!/usr/bin/env bash
# Prove Blueprint Course creation, immutable revision replacement, and Instructor read authority.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_blueprint_course.sh [--service|--browser]" >&2
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
		echo "Blueprint Course evidence requires the fixed Live Demo to be running" >&2
		exit 2
	fi
	python3 local_stack.py status --project "$project_name" >/dev/null
}

service_id() {
	local service="$1"
	local identifiers
	identifiers="$(podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=$service")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Blueprint Course evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Blueprint Course evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1"
	local cookie="${2:-}"
	local method="${3:-GET}"
	local body="${4:-}"
	local if_match="${5:-}"
	local gateway port
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	local -a curl_args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}'
		--header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then
		curl_args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
	fi
	if [ -n "$cookie" ]; then curl_args+=(--header "Cookie: $cookie"); fi
	if [ -n "$if_match" ]; then curl_args+=(--header "If-Match: $if_match"); fi
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
		echo "Blueprint Course access was not concealed" >&2
		exit 1
	fi
}

first_published_question_id() {
	local response="$1"
	python3 -c '
import json, re, sys
payload = json.loads(sys.argv[1])
items = payload.get("items")
if not isinstance(items, list) or not items:
    raise SystemExit("Question Library did not return a published Question for Blueprint Course creation")
question_id = items[0].get("summary", {}).get("questionId")
if not isinstance(question_id, str) or not re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}", question_id):
    raise SystemExit("Question Library did not return an opaque Question ID")
print(question_id)
' "$response"
}

creation_payload() {
	local question_id="$1"
	python3 -c '
import json, sys
question_id = sys.argv[1]
content = {
    "title": "Live Demo Blueprint Assignment",
    "instructions": "Use the published Question in reusable course structure.",
    "entries": [{
        "kind": "fixed", "question_id": question_id, "points_possible": "1", "scoring_rule": "normal",
        "question_attempt_limit": {"maxAttempts": None},
        "question_attempt_time_limit": {"kind": "unlimited"},
    }],
    "defaults": {
        "assignment_attempt_time_limit_seconds": None,
        "attempt_limit": 2,
        "late_work_rule": "accept",
        "assignment_deadline_rule": "auto_submit",
        "activity_rules": {
            "assignmentCompletionRule": {"kind": "answerAll"},
            "assignmentAttemptGradeRule": "highest",
            "assignmentAttemptContinuationRule": {"kind": "unlimited"},
            "questionPoolReuseRule": "reuseSelection",
            "questionVariationRule": "newVariation",
            "assignmentAttemptResumeRule": "resumable",
            "assignmentQuestionDisplayRule": "allQuestions",
            "assignmentNavigationRule": "freeNavigation",
            "assignmentQuestionOrderRule": "authoredOrder",
        },
        "student_feedback_release_rule": {
            "score": "after_submit", "per_item_correctness": "after_submit",
            "question_feedback": "after_submit", "question_answer": "never",
            "question_answer_explanation": "never", "class_statistics": "never",
        },
    },
    "schedule": {"available_at": None, "due_at": None, "closes_at": None},
}
print(json.dumps({"title": "Live Demo Blueprint Course", "modules": [{"label": "Module 1", "assignments": [content]}]}, separators=(",", ":")))
' "$question_id"
}

replacement_payload() {
	local response="$1"
	python3 -c '
import json, sys
course = json.loads(sys.argv[1])
modules = course.get("modules")
if not isinstance(modules, list) or len(modules) != 1:
    raise SystemExit("Blueprint Course creation did not return one reusable module")
module = modules[0]
assignments = module.get("assignments")
if not isinstance(assignments, list) or len(assignments) != 1:
    raise SystemExit("Blueprint Course creation did not return one reusable assignment")
assignment = assignments[0]
module_ref = module.get("blueprint_module_reference")
assignment_ref = assignment.get("blueprint_assignment_reference")
content = assignment.get("content")
if not isinstance(module_ref, str) or not isinstance(assignment_ref, str) or not isinstance(content, dict):
    raise SystemExit("Blueprint Course creation did not return retained reusable identities")
payload = {
    "title": course.get("title"),
    "modules": [{
        "choice": {"kind": "retained", "blueprint_module_reference": module_ref},
        "label": module.get("label"),
        "assignments": [{
            "choice": {"kind": "retained", "blueprint_assignment_reference": assignment_ref},
            "content": {
                "title": content.get("title"),
                "instructions": content.get("instructions"),
                "entries": [{
                    "kind": "fixed",
                    "question_id": content["entries"][0]["question"]["question_library"]["summary"]["questionId"],
                    "points_possible": content["entries"][0]["points_possible"],
                    "scoring_rule": content["entries"][0]["scoring_rule"],
                    "question_attempt_limit": content["entries"][0]["question_attempt_limit"],
                    "question_attempt_time_limit": content["entries"][0]["question_attempt_time_limit"],
                }],
                "defaults": content.get("defaults"),
                "schedule": content.get("schedule"),
            },
        }],
    }],
}
print(json.dumps(payload, separators=(",", ":")))
' "$response"
}

assert_public_view() {
	local response="$1"
	local reference="$2"
	local revision="$3"
	local access="$4"
	python3 -c '
import json, sys
payload = json.loads(sys.argv[1])
reference, revision, access = sys.argv[2:]
if set(payload) != {"reference", "title", "revision", "read_access", "modules"}:
    raise SystemExit("Blueprint Course response did not have the closed public DTO shape")
if payload["reference"] != reference or payload["revision"] != revision or payload["read_access"] != access:
    raise SystemExit("Blueprint Course response did not preserve its expected public identity or read access")
forbidden = {"accountId", "ownerAccountId", "sourceObject", "sourceChecksum", "questionRevision", "answerKey"}
def verify(value):
    if isinstance(value, dict):
        overlap = forbidden.intersection(value)
        if overlap:
            raise SystemExit("Blueprint Course response exposed a server-only or answer-bearing field")
        for child in value.values():
            verify(child)
    elif isinstance(value, list):
        for child in value:
            verify(child)
verify(payload)
' "$response" "$reference" "$revision" "$access"
}

assert_active_instructor_read() {
	local reference="$1"
	local postgres output sql
	postgres="$(service_id postgres)"
	sql="BEGIN;
DO \$\$ BEGIN
    PERFORM set_config('ple.session_account_id', (
        SELECT account_id::text FROM ple_private.account
         WHERE product_role = 'sysadmin' ORDER BY account_id LIMIT 1
    ), true);
END \$\$;
SET LOCAL ROLE ple_app;
DO \$\$
DECLARE
    v_reader_account_id uuid;
BEGIN
    SELECT account_id INTO v_reader_account_id
      FROM ple_api.create_instructor_account(
          'm7-blueprint-reader@example.invalid',
          'm7-blueprint-reader@example.invalid'
      );
    PERFORM set_config('ple.session_account_id', v_reader_account_id::text, true);
    IF NOT ple_api.current_session_account_is_instructor()
       OR NOT EXISTS (
           SELECT 1 FROM ple_api.load_live_demo_blueprint_course(${reference#BP-}) AS course
            WHERE course.is_owner IS FALSE AND course.revision_number = 2
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Active Instructor Blueprint Course read failed';
    END IF;
END
\$\$;
SELECT 'active_instructor';
ROLLBACK;"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^active_instructor$/p')" != "active_instructor" ]; then
		echo "an Active Instructor did not receive non-owner Blueprint Course Read Access" >&2
		exit 1
	fi
}

prove_service() {
	local anonymous student_cookie student instructor_cookie library question_id created body reference replacement saved stale detail
	anonymous="$(request '/api/course-blueprints')"
	assert_concealed "$anonymous"
	student_cookie="$(persona_cookie maryStudent)"
	student="$(request '/api/course-blueprints' "$student_cookie")"
	assert_concealed "$student"
	if [ "$(response_body "$anonymous")" != "$(response_body "$student")" ]; then
		echo "anonymous and Student Blueprint Course concealment differs" >&2
		exit 1
	fi
	instructor_cookie="$(persona_cookie elenaInstructor)"
	library="$(request '/api/questions/search?page_size=50' "$instructor_cookie")"
	if [ "$(response_status "$library")" != "200" ]; then
		echo "Instructor could not browse the Question Library for Blueprint Course creation" >&2
		exit 1
	fi
	question_id="$(first_published_question_id "$(response_body "$library")")"
	created="$(request '/api/course-blueprints' "$instructor_cookie" POST "$(creation_payload "$question_id")")"
	if [ "$(response_status "$created")" != "201" ]; then
		echo "Instructor could not create and publish a Blueprint Course (HTTP $(response_status "$created"))" >&2
		exit 1
	fi
	body="$(response_body "$created")"
	read -r reference < <(python3 -c '
import json, re, sys
value = json.loads(sys.argv[1])
reference = value.get("reference")
if not isinstance(reference, str) or not re.fullmatch(r"BP-[1-9][0-9]{0,9}", reference):
    raise SystemExit("Blueprint Course creation did not return an opaque Blueprint Course Reference")
print(reference)
' "$body")
	assert_public_view "$body" "$reference" "1" "blueprint_course_owner"
	detail="$(request "/api/course-blueprints/$reference" "$instructor_cookie")"
	if [ "$(response_status "$detail")" != "200" ]; then
		echo "Blueprint Course Owner could not read the published Blueprint Course" >&2
		exit 1
	fi
	assert_public_view "$(response_body "$detail")" "$reference" "1" "blueprint_course_owner"
	replacement="$(replacement_payload "$body")"
	saved="$(request "/api/course-blueprints/$reference" "$instructor_cookie" PUT "$replacement" '"1"')"
	if [ "$(response_status "$saved")" != "200" ]; then
		echo "Blueprint Course Owner could not create its successor Blueprint Revision (HTTP $(response_status "$saved"))" >&2
		exit 1
	fi
	assert_public_view "$(response_body "$saved")" "$reference" "2" "blueprint_course_owner"
	stale="$(request "/api/course-blueprints/$reference" "$instructor_cookie" PUT "$replacement" '"1"')"
	if [ "$(response_status "$stale")" != "412" ]; then
		echo "a stale Blueprint Revision replacement was accepted" >&2
		exit 1
	fi
	assert_active_instructor_read "$reference"
	echo "Blueprint Course service: owner, Active Instructor read access, and immutable revisions complete"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	node tests/e2e/e2e_live_demo_blueprint_course_browser.mjs "$port"
	echo "Blueprint Course browser: visible creation and publication complete"
}

require_live_demo
case "$mode" in
	service) prove_service ;;
	browser) prove_browser ;;
	all)
		prove_service
		prove_browser
		;;
esac

echo "Live Demo Blueprint Course: PASS"
