#!/usr/bin/env bash
# Prove private Draft Question authoring and immutable Question Publication on the live stack.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_authoring.sh [--draft|--publish]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--draft) mode="draft" ;;
	--publish) mode="publish" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Authoring evidence requires the fixed Live Demo to be running" >&2
		exit 2
	fi
	python3 local_stack.py status --project "$project_name" >/dev/null
}

gateway_id() {
	local identifiers
	identifiers="$(podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=gateway")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Authoring evidence requires one running gateway" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Authoring evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1"
	local cookie="${2:-}"
	local method="${3:-GET}"
	local body="${4:-}"
	local content_type="${5:-}"
	local if_match="${6:-}"
	local gateway port
	gateway="$(gateway_id)"
	port="$(gateway_port)"
	local -a curl_args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}'
		--header "Host: localhost:$port" --request "$method")
	if [ -n "$cookie" ]; then curl_args+=(--header "Cookie: $cookie"); fi
	if [ -n "$content_type" ]; then curl_args+=(--header "Content-Type: $content_type"); fi
	if [ -n "$if_match" ]; then curl_args+=(--header "If-Match: $if_match"); fi
	if [ -n "$body" ]; then curl_args+=(--data "$body"); fi
	podman exec "$gateway" curl "${curl_args[@]}" "https://localhost:8080$path"
}

persona_cookie() {
	local persona="$1"
	local gateway port headers cookie
	gateway="$(gateway_id)"
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
		echo "Draft Question access was not concealed" >&2
		exit 1
	fi
}

source_payload() {
	python3 -c '
import json
print(json.dumps({
    "format": "pleQuestionJson", "version": 3,
    "questionTitle": "Live authoring boundary", 
    "questionDescription": "Private authoring verification question.",
    "prompt": "Which choice demonstrates the authoring boundary?",
    "response": {"kind": "singleChoice", "choices": [
        {"id": "a", "text": "The private workspace", "feedback": None},
        {"id": "b", "text": "The global library", "feedback": None}
    ], "correctChoice": "a"},
    "questionHint": None, "feedback": {"correct": None, "incorrect": None},
    "tags": ["live-demo"], "questionLicense": "CC-BY-4.0",
    "questionCitation": None, "language": "en-US"
}, separators=(",", ":")))'
}

draft_reference=""
draft_edit=""
instructor_cookie=""

prove_draft() {
	local anonymous student_cookie student instructor created body source anonymous_source student_source saved stale list
	anonymous="$(request '/api/authoring/drafts')"
	assert_concealed "$anonymous"
	student_cookie="$(persona_cookie maryStudent)"
	student="$(request '/api/authoring/drafts' "$student_cookie")"
	assert_concealed "$student"
	if [ "$(response_body "$anonymous")" != "$(response_body "$student")" ]; then
		echo "anonymous and Student Draft Question concealment differs" >&2
		exit 1
	fi
	instructor_cookie="$(persona_cookie elenaInstructor)"
	created="$(request '/api/authoring/drafts' "$instructor_cookie" POST "$(source_payload)" 'application/vnd.peptidyle.question+json')"
	if [ "$(response_status "$created")" != "201" ]; then
		echo "Instructor could not create a private Draft Question" >&2
		exit 1
	fi
	body="$(response_body "$created")"
	read -r draft_reference draft_edit < <(python3 -c '
import json, re, sys
value = json.loads(sys.argv[1])
reference = value.get("draftQuestion")
edit = value.get("editNumber")
if not isinstance(reference, str) or not re.fullmatch(r"D-[1-9][0-9]{0,9}", reference):
    raise SystemExit("Draft Question creation did not return an opaque Draft Question Reference")
if not isinstance(edit, int) or edit <= 0:
    raise SystemExit("Draft Question creation did not return a positive Edit Number")
if any(key in value for key in ("draftQuestionUuid", "workspaceId", "objectAddress", "sourceObject")):
    raise SystemExit("Draft Question creation exposed a server-only identity")
print(reference, edit)
' "$body")
	source="$(request "/api/authoring/drafts/$draft_reference/source" "$instructor_cookie")"
	if [ "$(response_status "$source")" != "200" ]; then
		echo "Instructor could not load the private Draft Question source" >&2
		exit 1
	fi
	anonymous_source="$(request "/api/authoring/drafts/$draft_reference/source")"
	student_source="$(request "/api/authoring/drafts/$draft_reference/source" "$student_cookie")"
	assert_concealed "$anonymous_source"
	assert_concealed "$student_source"
	if [ "$(response_body "$anonymous_source")" != "$(response_body "$student_source")" ]; then
		echo "private Draft Question source concealment differs" >&2
		exit 1
	fi
	saved="$(request "/api/authoring/drafts/$draft_reference/source" "$instructor_cookie" PUT "$(source_payload)" 'application/vnd.peptidyle.question+json' "\"$draft_edit\"")"
	if [ "$(response_status "$saved")" != "204" ]; then
		echo "Instructor could not save the private Draft Question" >&2
		exit 1
	fi
	stale="$(request "/api/authoring/drafts/$draft_reference/source" "$instructor_cookie" PUT "$(source_payload)" 'application/vnd.peptidyle.question+json' "\"$draft_edit\"")"
	if [ "$(response_status "$stale")" != "409" ]; then
		echo "stale Draft Question Edit Number was accepted" >&2
		exit 1
	fi
	draft_edit="$((draft_edit + 1))"
	list="$(request '/api/authoring/drafts' "$instructor_cookie")"
	if [ "$(response_status "$list")" != "200" ]; then
		echo "Instructor could not list My Question Drafts" >&2
		exit 1
	fi
	python3 -c '
import json, sys
payload = json.loads(sys.argv[1])
rendered = json.dumps(payload, sort_keys=True)
if not isinstance(payload.get("items"), list):
    raise SystemExit("My Question Drafts did not return a list")
for forbidden in ("draftQuestionUuid", "workspaceId", "objectAddress", "sourceObject", "sourceChecksum"):
    if forbidden in rendered:
        raise SystemExit("My Question Drafts exposed a server-only source fact")
' "$(response_body "$list")"
	echo "Draft Question API: workspace authority, Edit Number conflict, and private source boundary complete"
}

prove_publish() {
	local published question_id library
	if [ -z "$draft_reference" ]; then prove_draft; fi
	published="$(request "/api/authoring/drafts/$draft_reference/publish" "$instructor_cookie" POST '{"authors":["Live Demo Instructor"]}' 'application/json' "\"$draft_edit\"")"
	if [ "$(response_status "$published")" != "200" ]; then
		echo "Instructor could not publish the saved Draft Question" >&2
		exit 1
	fi
	question_id="$(python3 -c '
import json, re, sys
value = json.loads(sys.argv[1])
if set(value) != {"questionId"} or not isinstance(value["questionId"], str):
    raise SystemExit("Question Publication returned a non-publication DTO")
if not re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}", value["questionId"]):
    raise SystemExit("Question Publication did not mint a Question ID")
print(value["questionId"])
' "$(response_body "$published")")"
	library="$(request '/api/questions/search?page_size=100' "$instructor_cookie")"
	if [ "$(response_status "$library")" != "200" ]; then
		echo "Question Library did not expose the published Question" >&2
		exit 1
	fi
	python3 -c '
import json, sys
payload = json.loads(sys.argv[1])
question_id = sys.argv[2]
items = payload.get("items")
if not isinstance(items, list) or question_id not in {
    item.get("summary", {}).get("questionId") for item in items
}:
    raise SystemExit("Question Library did not expose the immutable published Question")
rendered = json.dumps(payload, sort_keys=True)
for forbidden in ("draftQuestion", "draftQuestionUuid", "workspaceId", "objectAddress", "sourceObject", "sourceChecksum"):
    if forbidden in rendered:
        raise SystemExit("Question Library publication view exposed a private Draft Question fact")
' "$(response_body "$library")" "$question_id"
	echo "Question Publication API: immutable Question Revision exposed through Question Library"
}

require_live_demo
case "$mode" in
	draft) prove_draft ;;
	publish) prove_publish ;;
	all)
		prove_draft
		prove_publish
		port="$(gateway_port)"
		node tests/e2e/e2e_live_demo_authoring_browser.mjs "$port"
		echo "Authoring browser: My Question Drafts, private editing, and publication complete"
		;;
esac

echo "Live Demo authoring: PASS"
