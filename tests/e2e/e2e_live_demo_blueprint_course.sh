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
	local idempotency_key="${6:-}"
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
	if [ -n "$idempotency_key" ]; then curl_args+=(--header "Idempotency-Key: $idempotency_key"); fi
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
            "submitted_response": "after_submit",
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
draft = course.get("draft")
if not isinstance(draft, dict):
    raise SystemExit("Blueprint Course creation did not return its private Draft")
modules = draft.get("modules")
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
    "title": "{} (saved)".format(course.get("title")),
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
	local response="$1" reference="$2" draft_edit="$3" title="$4"
	python3 -c '
import json, sys
payload = json.loads(sys.argv[1])
reference, draft_edit, title = sys.argv[2:]
expected = {"reference", "title", "availability", "availability_edit_number", "latest_published_revision", "read_access", "draft"}
if set(payload) != expected:
    raise SystemExit("Blueprint Course response did not have the current closed DTO shape")
if (payload["reference"] != reference or payload["title"] != title
        or payload["availability"] != "available" or payload["availability_edit_number"] != "1"
        or payload["latest_published_revision"] is not None or payload["read_access"] != "blueprint_course_owner"):
    raise SystemExit("Blueprint Course creation did not return an available private Draft without a Revision")
draft = payload["draft"]
if not isinstance(draft, dict) or draft.get("edit_number") != draft_edit or not isinstance(draft.get("modules"), list):
    raise SystemExit("Blueprint Course creation did not return the expected private Draft")
' "$response" "$reference" "$draft_edit" "$title"
}

assert_publication() {
	python3 -c '
import json, sys
value = json.loads(sys.argv[1])
expected_reference, expected_revision = sys.argv[2:]
if set(value) != {"blueprintRevision"}:
    raise SystemExit("Blueprint publication receipt was not closed")
revision = value["blueprintRevision"]
if revision != {"reference": expected_reference, "revision": expected_revision}:
    raise SystemExit("Blueprint publication did not return its exact immutable Revision")
' "$1" "$2" "$3"
}

assert_exact_revision() {
	python3 -c '
import json, sys
value = json.loads(sys.argv[1])
reference, revision = sys.argv[2:]
if set(value) != {"blueprintRevision", "title", "modules"}:
    raise SystemExit("exact Blueprint Revision response was not closed")
if value["blueprintRevision"] != {"reference": reference, "revision": revision}:
    raise SystemExit("exact Blueprint Revision did not resolve its immutable identity")
modules = value["modules"]
if not isinstance(modules, list) or len(modules) != 1:
    raise SystemExit("exact Blueprint Revision did not retain its module")
assignments = modules[0].get("assignments") if isinstance(modules[0], dict) else None
if not isinstance(assignments, list) or len(assignments) != 1:
    raise SystemExit("exact Blueprint Revision did not retain its assignment")
' "$1" "$2" "$3"
}

assert_availability() {
	python3 -c '
import json, sys
value = json.loads(sys.argv[1])
if value != {"availability": sys.argv[2], "editNumber": sys.argv[3]}:
    raise SystemExit("Blueprint availability transition did not return its qualified result")
' "$1" "$2" "$3"
}

assert_absent_from_listing() {
	python3 -c '
import json, sys
items = json.loads(sys.argv[1]).get("items")
if not isinstance(items, list) or any(item.get("reference") == sys.argv[2] for item in items if isinstance(item, dict)):
    raise SystemExit("archived Blueprint Course remained in ordinary browsing")
' "$1" "$2"
}

assert_present_in_listing() {
	python3 -c '
import json, sys
items = json.loads(sys.argv[1]).get("items")
if not isinstance(items, list) or not any(item.get("reference") == sys.argv[2] for item in items if isinstance(item, dict)):
    raise SystemExit("restored Blueprint Course did not return to ordinary browsing")
' "$1" "$2"
}

prove_service() {
	local anonymous student_cookie student instructor_cookie library question_id created body reference replacement saved detail publication replay deliberate exact archived restored listed
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
	created="$(request '/api/course-blueprints' "$instructor_cookie" POST "$(creation_payload "$question_id")" '' 'm7-blueprint-create')"
	if [ "$(response_status "$created")" != "201" ]; then
		echo "Instructor could not create a Blueprint Course Draft (HTTP $(response_status "$created"))" >&2
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
	assert_public_view "$body" "$reference" "1" "Live Demo Blueprint Course"
	detail="$(request "/api/course-blueprints/$reference" "$instructor_cookie")"
	if [ "$(response_status "$detail")" != "200" ]; then
		echo "Blueprint Course Owner could not read its private Draft" >&2
		exit 1
	fi
	assert_public_view "$(response_body "$detail")" "$reference" "1" "Live Demo Blueprint Course"
	replacement="$(replacement_payload "$body")"
	saved="$(request "/api/course-blueprints/$reference/draft" "$instructor_cookie" PUT "$replacement" '"1"' 'm7-blueprint-save')"
	if [ "$(response_status "$saved")" != "200" ]; then
		echo "Blueprint Course Owner could not save its Draft (HTTP $(response_status "$saved"))" >&2
		exit 1
	fi
	assert_public_view "$(response_body "$saved")" "$reference" "2" "Live Demo Blueprint Course (saved)"
	publication="$(request "/api/course-blueprints/$reference/publish" "$instructor_cookie" POST '' '"2"' 'm7-blueprint-publish-1')"
	if [ "$(response_status "$publication")" != "200" ]; then
		echo "Blueprint Course Owner could not explicitly publish its Draft" >&2
		exit 1
	fi
	assert_publication "$(response_body "$publication")" "$reference" "1"
	replay="$(request "/api/course-blueprints/$reference/publish" "$instructor_cookie" POST '' '"2"' 'm7-blueprint-publish-1')"
	if [ "$(response_status "$replay")" != "200" ]; then
		echo "accepted Blueprint publication did not converge on replay" >&2
		exit 1
	fi
	assert_publication "$(response_body "$replay")" "$reference" "1"
	deliberate="$(request "/api/course-blueprints/$reference/publish" "$instructor_cookie" POST '' '"2"' 'm7-blueprint-publish-2')"
	if [ "$(response_status "$deliberate")" != "200" ]; then
		echo "unchanged deliberate Blueprint publication was not accepted" >&2
		exit 1
	fi
	assert_publication "$(response_body "$deliberate")" "$reference" "2"
	exact="$(request "/api/course-blueprints/$reference/revisions/1" "$instructor_cookie")"
	if [ "$(response_status "$exact")" != "200" ]; then
		echo "exact published Blueprint Revision did not resolve" >&2
		exit 1
	fi
	assert_exact_revision "$(response_body "$exact")" "$reference" "1"
	archived="$(request "/api/course-blueprints/$reference/archive" "$instructor_cookie" POST '{"confirmationTitle":"Live Demo Blueprint Course (saved)"}' '"1"')"
	if [ "$(response_status "$archived")" != "200" ]; then
		echo "Blueprint Course Owner could not archive its lineage" >&2
		exit 1
	fi
	assert_availability "$(response_body "$archived")" "archived" "2"
	listed="$(request '/api/course-blueprints?pageSize=100' "$instructor_cookie")"
	if [ "$(response_status "$listed")" != "200" ]; then
		echo "Instructor could not browse Blueprint Courses after archive" >&2
		exit 1
	fi
	assert_absent_from_listing "$(response_body "$listed")" "$reference"
	exact="$(request "/api/course-blueprints/$reference/revisions/1" "$instructor_cookie")"
	if [ "$(response_status "$exact")" != "200" ]; then
		echo "archive broke an exact Blueprint Revision reference" >&2
		exit 1
	fi
	restored="$(request "/api/course-blueprints/$reference/restore" "$instructor_cookie" POST '' '"2"')"
	if [ "$(response_status "$restored")" != "200" ]; then
		echo "Blueprint Course Owner could not restore its lineage" >&2
		exit 1
	fi
	assert_availability "$(response_body "$restored")" "available" "3"
	listed="$(request '/api/course-blueprints?pageSize=100' "$instructor_cookie")"
	if [ "$(response_status "$listed")" != "200" ]; then
		echo "Instructor could not browse Blueprint Courses after restore" >&2
		exit 1
	fi
	assert_present_in_listing "$(response_body "$listed")" "$reference"
	echo "Blueprint Course service: Draft, publication replay, availability, and exact Revision behavior complete"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	node tests/playwright/e2e_live_demo_blueprint_course_browser.mjs "$port"
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
