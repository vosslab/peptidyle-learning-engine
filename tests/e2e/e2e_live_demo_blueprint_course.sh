#!/usr/bin/env bash
# Prove Blueprint Course creation, immutable revision replacement, and Instructor read authority.
#
# The optional C19 reader capability is supplied only by a self-owned isolated
# service fixture.  The fixed Developer Browser Suite deliberately has one
# Instructor persona and must never be altered to manufacture another account.

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
readonly run_id="$(date +%s%N)"

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
if not isinstance(question_id, str) or not re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}", question_id):
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
            "questionVariationRule": "newVariation",
            "assignmentAttemptResumeRule": "resumable",
            "assignmentQuestionDisplayRule": "allQuestions",
            "assignmentNavigationRule": "freeNavigation",
            "assignmentQuestionOrderRule": "authoredOrder",
        },
        "student_feedback_release_rule": {
            "score": "after_submit", "per_item_correctness": "after_submit",
            "submitted_response": "after_submit",
            "question_answer": "never",
            "question_answer_explanation": "never", "class_statistics": "never",
        },
    },
}
print(json.dumps({"short_name": "Live Blueprint", "long_name": "Live Demo Blueprint Course", "modules": [{"label": "Module 1", "assignments": [content]}]}, separators=(",", ":")))
' "$question_id"
}

replacement_payload() {
	local response="$1"
	python3 -c '
import json, sys
course = json.loads(sys.argv[1])
modules = course.get("modules")
if not isinstance(modules, list) or len(modules) != 1:
    raise SystemExit("Blueprint Course creation did not return Revision 1 reusable content")
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
    "modules": [{
        "choice": {"kind": "retained", "blueprint_module_reference": module_ref},
        "label": module.get("label"),
        "assignments": [{
            "choice": {"kind": "retained", "blueprint_assignment_reference": assignment_ref},
            "content": {
                "title": "{} (saved)".format(content.get("title")),
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
            },
        }],
    }],
}
print(json.dumps(payload, separators=(",", ":")))
' "$response"
}

assert_current_view() {
	local response="$1" reference="$2" revision="$3" short_name="$4" long_name="$5" availability="$6" read_access="$7"
	python3 -c '
import json, sys
payload = json.loads(sys.argv[1])
reference, revision, short_name, long_name, availability, read_access = sys.argv[2:]
expected = {"reference", "short_name", "long_name", "availability", "metadata_etag", "current_revision", "read_access", "modules"}
if set(payload) != expected:
    raise SystemExit("Blueprint Course response did not have the current closed DTO shape")
if (payload["reference"] != reference or payload["short_name"] != short_name or payload["long_name"] != long_name
        or payload["availability"] != availability or payload["read_access"] != read_access
        or payload["current_revision"] != {"reference": reference, "revision": revision}
        or not isinstance(payload["metadata_etag"], str) or not payload["metadata_etag"]
        or not isinstance(payload["modules"], list)):
    raise SystemExit("Blueprint Course did not return its current immutable Revision")
' "$response" "$reference" "$revision" "$short_name" "$long_name" "$availability" "$read_access"
}

assert_save() {
	python3 -c '
import json, sys
value = json.loads(sys.argv[1])
reference, revision, changed = sys.argv[2:]
if set(value) != {"blueprintCourse", "changed"}:
    raise SystemExit("Blueprint Save response was not closed")
course = value["blueprintCourse"]
if (value["changed"] != (changed == "true")
        or course.get("current_revision") != {"reference": reference, "revision": revision}):
    raise SystemExit("Blueprint Save did not return its exact current Revision and changed outcome")
' "$1" "$2" "$3" "$4"
}

assert_exact_revision() {
	python3 -c '
import json, sys
value = json.loads(sys.argv[1])
reference, revision = sys.argv[2:]
if set(value) != {"blueprintRevision", "modules"}:
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

assert_metadata() {
	python3 -c '
import json, sys
value = json.loads(sys.argv[1])
if (set(value) != {"short_name", "long_name", "availability", "metadata_etag"}
        or value["availability"] != sys.argv[2]
        or value["short_name"] != sys.argv[3]
        or value["long_name"] != sys.argv[4]
        or not isinstance(value["metadata_etag"], str) or not value["metadata_etag"]):
    raise SystemExit("Blueprint metadata transition did not return its opaque qualified result")
print(value["metadata_etag"])
' "$1" "$2" "$3" "$4"
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

adoption_payload() {
	local reference="$1" revision="$2"
	python3 -c '
import datetime, json, sys
reference, revision = sys.argv[1:]
today = datetime.date.today()
print(json.dumps({
    "source": {"kind": "adopted", "blueprintCourse": reference, "blueprintRevision": revision},
    "shortName": "Blueprint browse adoption",
    "longName": "Blueprint Course browse adoption",
    "term": {"startDate": today.isoformat(), "endDate": (today + datetime.timedelta(days=7)).isoformat()},
}, separators=(",", ":")))
' "$reference" "$revision"
}

assert_distinct_instructors() {
	local owner_cookie="$1" reader_cookie="$2" owner reader
	owner="$(request '/api/auth/session' "$owner_cookie")"
	reader="$(request '/api/auth/session' "$reader_cookie")"
	if [ "$(response_status "$owner")" != "200" ] || [ "$(response_status "$reader")" != "200" ]; then
		echo "C19 isolated fixture did not provide two authenticated Instructor Sessions" >&2
		exit 1
	fi
	python3 -c '
import json, sys
owner, reader = (json.loads(value) for value in sys.argv[1:])
for value in (owner, reader):
    account = value.get("account")
    if value.get("authenticated") is not True or not isinstance(account, dict) or account.get("productRole") != "instructor":
        raise SystemExit("C19 fixture Session is not an authenticated Instructor")
if owner["account"].get("id") == reader["account"].get("id"):
    raise SystemExit("C19 fixture must use distinct owner and reader Instructors")
' "$(response_body "$owner")" "$(response_body "$reader")"
}

prove_non_owner_browse() {
	local reader_cookie="$1" reference="$2" metadata_etag="$3" replacement="$4"
	local private public archived response next_metadata_etag adoption public_adoption

	# Regression: an accidental owner check, a Private disclosure, or an
	# Archived adoption would alter deliberate reusable-course authority.  If
	# this fails, repair the route/persistence authorization predicate or the
	# lifecycle guard; do not weaken these product-boundary assertions.
	assert_distinct_instructors "$instructor_cookie" "$reader_cookie"
	private="$(request "/api/course-blueprints/$reference" "$reader_cookie")"
	assert_concealed "$private"
	response="$(request "/api/course-blueprints/$reference" "$reader_cookie" PUT "$replacement" '"1"' "m19-reader-save-$run_id")"
	assert_concealed "$response"

	public="$(request "/api/course-blueprints/$reference/publish" "$instructor_cookie" POST '' "\"$metadata_etag\"")"
	if [ "$(response_status "$public")" != "200" ]; then
		echo "Blueprint Course Owner could not publish the Private Blueprint for C19 browse evidence" >&2
		exit 1
	fi
	next_metadata_etag="$(assert_metadata "$(response_body "$public")" public "Live Renamed Blueprint" "Live Demo Blueprint Course Renamed")"
	response="$(request "/api/course-blueprints/$reference" "$reader_cookie")"
	if [ "$(response_status "$response")" != "200" ]; then
		echo "Second vetted Instructor could not browse Public Blueprint content" >&2
		exit 1
	fi
	assert_current_view "$(response_body "$response")" "$reference" "2" "Live Renamed Blueprint" "Live Demo Blueprint Course Renamed" public active_instructor
	response="$(request "/api/course-blueprints/$reference/revisions/1" "$reader_cookie")"
	if [ "$(response_status "$response")" != "200" ]; then
		echo "Second vetted Instructor could not browse Public Blueprint Revision content" >&2
		exit 1
	fi
	assert_exact_revision "$(response_body "$response")" "$reference" "1"
	public_adoption="$(request '/api/course-instances' "$reader_cookie" POST "$(adoption_payload "$reference" 2)")"
	if [ "$(response_status "$public_adoption")" != "201" ]; then
		echo "Public Blueprint Course did not permit a new Course Instance adoption" >&2
		exit 1
	fi
	response="$(request "/api/course-blueprints/$reference/metadata" "$reader_cookie" PUT '{"short_name":"Reader edit","long_name":"Reader edit"}' "\"$next_metadata_etag\"")"
	assert_concealed "$response"

	archived="$(request "/api/course-blueprints/$reference/archive" "$instructor_cookie" POST '{"confirmationLongName":"Live Demo Blueprint Course Renamed"}' "\"$next_metadata_etag\"")"
	if [ "$(response_status "$archived")" != "200" ]; then
		echo "Blueprint Course Owner could not archive the Public Blueprint for C19 browse evidence" >&2
		exit 1
	fi
	metadata_etag="$(assert_metadata "$(response_body "$archived")" archived "Live Renamed Blueprint" "Live Demo Blueprint Course Renamed")"
	response="$(request "/api/course-blueprints/$reference" "$reader_cookie")"
	if [ "$(response_status "$response")" != "200" ]; then
		echo "Second vetted Instructor could not browse Archived Blueprint content" >&2
		exit 1
	fi
	assert_current_view "$(response_body "$response")" "$reference" "2" "Live Renamed Blueprint" "Live Demo Blueprint Course Renamed" archived active_instructor
	response="$(request "/api/course-blueprints/$reference/archive" "$reader_cookie" POST '{"confirmationLongName":"Live Demo Blueprint Course Renamed"}' "\"$metadata_etag\"")"
	assert_concealed "$response"
	adoption="$(request '/api/course-instances' "$reader_cookie" POST "$(adoption_payload "$reference" 2)")"
	if [ "$(response_status "$adoption")" != "422" ]; then
		echo "Archived Blueprint Course accepted a new Course Instance adoption" >&2
		exit 1
	fi

	# Restore the source so the fixed owner-only service walkthrough remains
	# repeatable when an isolated fixture chooses to run the added C19 branch.
	response="$(request "/api/course-blueprints/$reference/restore" "$instructor_cookie" POST '' "\"$metadata_etag\"")"
	if [ "$(response_status "$response")" != "200" ]; then
		echo "Blueprint Course Owner could not restore C19 browse fixture" >&2
		exit 1
	fi
	assert_metadata "$(response_body "$response")" public "Live Renamed Blueprint" "Live Demo Blueprint Course Renamed" >/dev/null
}

prove_service() {
	local anonymous student_cookie student instructor_cookie reader_cookie library question_id created body reference metadata_etag replacement saved stale_save no_op detail renamed published archived restored listed
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
	created="$(request '/api/course-blueprints' "$instructor_cookie" POST "$(creation_payload "$question_id")" '' "m7-blueprint-create-$run_id")"
	if [ "$(response_status "$created")" != "201" ]; then
		echo "Instructor could not create Blueprint Revision 1 (HTTP $(response_status "$created"))" >&2
		exit 1
	fi
	body="$(response_body "$created")"
	read -r reference metadata_etag < <(python3 -c '
import json, re, sys
value = json.loads(sys.argv[1])
reference = value.get("reference")
revision = value.get("current_revision")
metadata_etag = value.get("metadata_etag")
if (not isinstance(reference, str) or not re.fullmatch(r"BP-[1-9][0-9]{0,9}", reference)
        or revision != {"reference": reference, "revision": "1"}
        or not isinstance(metadata_etag, str) or not metadata_etag):
    raise SystemExit("Blueprint Course creation did not return Revision 1 and metadata identity")
print(reference, metadata_etag)
' "$body")
	assert_current_view "$body" "$reference" "1" "Live Blueprint" "Live Demo Blueprint Course" private blueprint_course_owner
	detail="$(request "/api/course-blueprints/$reference" "$instructor_cookie")"
	if [ "$(response_status "$detail")" != "200" ]; then
		echo "Blueprint Course Owner could not read the current Revision" >&2
		exit 1
	fi
	assert_current_view "$(response_body "$detail")" "$reference" "1" "Live Blueprint" "Live Demo Blueprint Course" private blueprint_course_owner
	replacement="$(replacement_payload "$body")"
	saved="$(request "/api/course-blueprints/$reference" "$instructor_cookie" PUT "$replacement" '"1"' "m7-blueprint-save-$run_id")"
	if [ "$(response_status "$saved")" != "200" ]; then
		echo "Blueprint Course Owner could not Save Revision 2 (HTTP $(response_status "$saved"))" >&2
		exit 1
	fi
	assert_save "$(response_body "$saved")" "$reference" "2" true
	stale_save="$(request "/api/course-blueprints/$reference" "$instructor_cookie" PUT "$replacement" '"1"' "m7-blueprint-stale-save-$run_id")"
	if [ "$(response_status "$stale_save")" != "412" ]; then
		echo "stale Blueprint Save did not receive a Revision conflict" >&2
		exit 1
	fi
	no_op="$(request "/api/course-blueprints/$reference" "$instructor_cookie" PUT "$replacement" '"2"' "m7-blueprint-save-no-op-$run_id")"
	if [ "$(response_status "$no_op")" != "200" ]; then
		echo "canonical no-op Blueprint Save was not accepted" >&2
		exit 1
	fi
	assert_save "$(response_body "$no_op")" "$reference" "2" false
	renamed="$(request "/api/course-blueprints/$reference/metadata" "$instructor_cookie" PUT '{"short_name":"Live Renamed Blueprint","long_name":"Live Demo Blueprint Course Renamed"}' "\"$metadata_etag\"")"
	if [ "$(response_status "$renamed")" != "200" ]; then
		echo "Blueprint Course Owner could not rename Blueprint lineage metadata" >&2
		exit 1
	fi
	metadata_etag="$(assert_metadata "$(response_body "$renamed")" private "Live Renamed Blueprint" "Live Demo Blueprint Course Renamed")"
	detail="$(request "/api/course-blueprints/$reference" "$instructor_cookie")"
	if [ "$(response_status "$detail")" != "200" ]; then
		echo "renamed Blueprint Course could not reload its current Revision" >&2
		exit 1
	fi
	assert_current_view "$(response_body "$detail")" "$reference" "2" "Live Renamed Blueprint" "Live Demo Blueprint Course Renamed" private blueprint_course_owner
	exact="$(request "/api/course-blueprints/$reference/revisions/1" "$instructor_cookie")"
	if [ "$(response_status "$exact")" != "200" ]; then
		echo "exact saved Blueprint Revision did not resolve" >&2
		exit 1
	fi
	assert_exact_revision "$(response_body "$exact")" "$reference" "1"
	reader_cookie="${PLE_BLUEPRINT_E2E_READER_COOKIE:-}"
	if [ -n "$reader_cookie" ]; then
		prove_non_owner_browse "$reader_cookie" "$reference" "$metadata_etag" "$replacement"
		# C19's archive/restore path advances the opaque metadata validator.
		detail="$(request "/api/course-blueprints/$reference" "$instructor_cookie")"
		metadata_etag="$(python3 -c 'import json, sys; value=json.loads(sys.argv[1]); print(value["metadata_etag"])' "$(response_body "$detail")")"
	else
		# The fixed Browser Suite deliberately has only one Instructor. It still
		# proves the owner lifecycle; a self-owned runtime supplies the C19 reader
		# capability above without mutating the shared suite.
		published="$(request "/api/course-blueprints/$reference/publish" "$instructor_cookie" POST '' "\"$metadata_etag\"")"
		if [ "$(response_status "$published")" != "200" ]; then
			echo "Blueprint Course Owner could not publish the Private Blueprint" >&2
			exit 1
		fi
		metadata_etag="$(assert_metadata "$(response_body "$published")" public "Live Renamed Blueprint" "Live Demo Blueprint Course Renamed")"
	fi
	archived="$(request "/api/course-blueprints/$reference/archive" "$instructor_cookie" POST '{"confirmationLongName":"Live Demo Blueprint Course Renamed"}' "\"$metadata_etag\"")"
	if [ "$(response_status "$archived")" != "200" ]; then
		echo "Blueprint Course Owner could not archive its lineage" >&2
		exit 1
	fi
	metadata_etag="$(assert_metadata "$(response_body "$archived")" archived "Live Renamed Blueprint" "Live Demo Blueprint Course Renamed")"
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
	restored="$(request "/api/course-blueprints/$reference/restore" "$instructor_cookie" POST '' "\"$metadata_etag\"")"
	if [ "$(response_status "$restored")" != "200" ]; then
		echo "Blueprint Course Owner could not restore its lineage" >&2
		exit 1
	fi
	assert_metadata "$(response_body "$restored")" public "Live Renamed Blueprint" "Live Demo Blueprint Course Renamed" >/dev/null
	listed="$(request '/api/course-blueprints?pageSize=100' "$instructor_cookie")"
	if [ "$(response_status "$listed")" != "200" ]; then
		echo "Instructor could not browse Blueprint Courses after restore" >&2
		exit 1
	fi
	assert_present_in_listing "$(response_body "$listed")" "$reference"
	echo "Blueprint Course service: Revision creation, stale/no-op Save, metadata, availability, and exact Revision behavior complete"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	node tests/playwright/e2e_live_demo_blueprint_course_browser.mjs "$port"
	echo "Blueprint Course browser: visible Revision 1 creation and explicit Save complete"
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
