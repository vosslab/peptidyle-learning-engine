#!/usr/bin/env bash
# Shared helpers for current-model Live Demo Assignment journeys.

set -euo pipefail

if [ -z "${repository_root:-}" ]; then
	repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
fi
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

require_live_demo() {
	[ -f "$runtime_environment_path" ] || {
		echo "Live Demo Assignment acceptance requires a running fixed Live Demo" >&2
		exit 2
	}
}

service_id() {
	local service="$1" identifiers
	identifiers="$(podman ps -q --filter "label=com.docker.compose.project=$project_name" --filter "label=com.docker.compose.service=$service")"
	[ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = "1" ] || {
		echo "expected one running $service service" >&2
		exit 1
	}
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entry
	entry="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	[ "$(printf '%s\n' "$entry" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = "1" ] || {
		echo "expected one gateway port" >&2
		exit 1
	}
	printf '%s\n' "${entry#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1" cookie="${2:-}" method="${3:-GET}" body="${4:-}" if_match="${5:-}"
	local gateway port
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	local -a arguments=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}' --header "Host: localhost:$port" --request "$method")
	[ "$method" = GET ] || arguments+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
	[ -z "$cookie" ] || arguments+=(--header "Cookie: $cookie")
	[ -z "$if_match" ] || arguments+=(--header "If-Match: \"$if_match\"")
	[ -z "$body" ] || arguments+=(--data "$body")
	podman exec "$gateway" curl "${arguments[@]}" "https://localhost:8080$path"
}

response_status() { printf '%s' "${1##*$'\n'}"; }
response_body() { printf '%s' "${1%$'\n'*}"; }

require_status() {
	local label="$1" response="$2" expected="$3"
	[ "$(response_status "$response")" = "$expected" ] || {
		echo "$label returned HTTP $(response_status "$response"), expected $expected" >&2
		exit 1
	}
}

assert_concealed() { require_status "Concealed response" "$1" 404; }

persona_cookie() {
	local persona="$1" gateway port headers cookie
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 --dump-header - --output /dev/null --header "Host: localhost:$port" --header "Origin: https://localhost:$port" --header 'Content-Type: application/json' --request POST --data "{\"persona\":\"$persona\"}" 'https://localhost:8080/api/auth/live-demo/accounts')"
	cookie="$(printf '%s\n' "$headers" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"
	[ -n "$cookie" ] || { echo "Live Demo did not issue an authenticated session" >&2; exit 1; }
	printf '%s\n' "$cookie"
}

new_course_reference() {
	local instructor_cookie="$1" listed
	bash "$repository_root/tests/e2e/e2e_live_demo_course_instance.sh" --authority >/dev/null
	listed="$(request '/api/course-instances' "$instructor_cookie")"
	require_status "Instructor Course list" "$listed" 200
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1]).get("items", [])
refs=[item.get("reference") for item in items if isinstance(item, dict)]
if not refs or any(not isinstance(value, str) or re.fullmatch(r"C-[1-9][0-9]{0,9}", value) is None for value in refs):
    raise SystemExit("Course list lacks canonical public references")
print(max(refs, key=lambda value: int(value[2:])))
' "$(response_body "$listed")"
}

source_choice_reference() {
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1])
if not isinstance(items, list) or not items: raise SystemExit("Course has no Blueprint Assignment source")
source=items[0].get("source") if isinstance(items[0], dict) else None
if not isinstance(source, dict) or set(source) != {"blueprint_revision", "blueprint_assignment_reference"}:
    raise SystemExit("Assignment source choice is malformed")
value=source["blueprint_assignment_reference"]
if not isinstance(value, str) or re.fullmatch(r"[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}", value) is None:
    raise SystemExit("Assignment source reference is malformed")
print(value)
' "$1"
}

picker_reference() {
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1])
if not isinstance(items, list) or not items: raise SystemExit("Question picker is empty")
item=items[0]
if not isinstance(item, dict) or set(item) != {"reference", "description"}: raise SystemExit("Question picker is malformed")
reference=item["reference"]
if (not isinstance(reference, dict) or set(reference) != {"questionId", "revisionNumber"}
    or not isinstance(reference["questionId"], str)
    or re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}", reference["questionId"]) is None
    or not isinstance(reference["revisionNumber"], int) or reference["revisionNumber"] < 1):
    raise SystemExit("Question picker lacks an exact Question Revision")
print(json.dumps(reference, separators=(",", ":")))
' "$1"
}

workspace_reference_and_edit() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1])
if (not isinstance(value, dict) or not re.fullmatch(r"A-[1-9][0-9]{0,9}", value.get("reference", ""))
    or not isinstance(value.get("editNumber"), str) or not value["editNumber"].isdigit()):
    raise SystemExit("Assignment workspace lacks a reference and Edit Number")
print(value["reference"], value["editNumber"])
' "$1"
}

save_payload() {
	local workspace="$1" question_reference="$2" title="$3"
	python3 -c '
import json, sys, uuid
workspace=json.loads(sys.argv[1]); reference=json.loads(sys.argv[2]); title=sys.argv[3]
required={"title","instructions","dueAt","availableAt","closesAt","lateWorkRule","assignmentAttemptTimeLimitSeconds","attemptLimit","activityRules","studentFeedbackReleaseRule"}
if not required.issubset(workspace): raise SystemExit("workspace is missing current editable state")
payload={key: workspace[key] for key in required}
payload["title"]=title
payload["assignmentAttemptTimeLimitSeconds"]=300
payload["entries"]=[{
  "kind":"fixedQuestion", "id":str(uuid.uuid4()), "reference":reference,
  "pointsPossible":"1", "availability":"available", "scoringRule":"normal",
  "questionAttemptLimit":{"maxAttempts":None}, "questionAttemptTimeLimit":{"kind":"unlimited"},
}]
print(json.dumps(payload, separators=(",", ":")))
' "$workspace" "$question_reference" "$title"
}

retitle_payload() {
	local workspace="$1" title="$2"
	python3 -c '
import json, sys
workspace=json.loads(sys.argv[1]); title=sys.argv[2]
required={"title","instructions","dueAt","availableAt","closesAt","lateWorkRule","assignmentAttemptTimeLimitSeconds","attemptLimit","activityRules","studentFeedbackReleaseRule","entries"}
if not required.issubset(workspace): raise SystemExit("workspace is missing current editable state")
payload={key: workspace[key] for key in required}
payload["title"]=title
print(json.dumps(payload, separators=(",", ":")))
' "$workspace" "$title"
}

claim_student_record() {
	local course="$1" instructor_cookie="$2" student_cookie="$3" email="$4" roster_id="$5" imported claimed
	imported="$(request "/api/course-instances/$course/roster" "$instructor_cookie" POST "{\"entries\":[{\"email\":\"$email\",\"rosterId\":\"$roster_id\"}]}")"
	require_status "Instructor roster import" "$imported" 201
	claimed="$(request "/api/course-instances/$course/roster/claim" "$student_cookie" POST '{}')"
	require_status "Student Course Invitation claim" "$claimed" 200
}

assert_started() {
	local response="$1" resumed="$2" title="$3" exact_reference="$4"
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); expected_resumed=sys.argv[2] == "true"; title=sys.argv[3]; reference=json.loads(sys.argv[4])
required={"assignmentAttempt","assignment","attemptNumber","resumed","title","instructions","questions"}
if set(value) != required or value["resumed"] is not expected_resumed or value["title"] != title:
    raise SystemExit("Assignment start did not preserve its retained evidence")
if not re.fullmatch(r"R-[1-9][0-9]{0,9}", value["assignmentAttempt"]): raise SystemExit("Attempt identity is malformed")
if not isinstance(value["questions"], list) or len(value["questions"]) != 1: raise SystemExit("Attempt lacks one issued Question")
question=value["questions"][0]
if question.get("questionRevision") != reference: raise SystemExit("Issued Question lost its exact Question Revision pin")
forbidden={"answer","answerKey","studentRecord","assignmentAttemptId","questionAttemptId","checksum","reproduction"}
def scan(item):
    if isinstance(item, dict):
        if forbidden.intersection(item): raise SystemExit("start response exposed private evidence")
        for child in item.values(): scan(child)
    elif isinstance(item, list):
        for child in item: scan(child)
scan(value)
' "$(response_body "$response")" "$resumed" "$title" "$exact_reference"
}
