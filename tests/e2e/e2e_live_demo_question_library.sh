#!/usr/bin/env bash
# Prove the live Instructor Question Library route and its browser task.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_question_library.sh [--api|--browser]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--api) mode="api" ;;
	--browser) mode="browser" ;;
	*)
		usage
		exit 2
		;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Question Library evidence requires the fixed Live Demo to be running" >&2
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
		echo "Question Library evidence requires one running gateway" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Question Library evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1"
	local cookie="${2:-}"
	local gateway
	local port
	gateway="$(gateway_id)"
	port="$(gateway_port)"
	if [ -n "$cookie" ]; then
		podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 \
			--write-out $'\n%{http_code}' --header "Host: localhost:$port" \
			--header "Cookie: $cookie" "https://localhost:8080$path"
	else
		podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 \
			--write-out $'\n%{http_code}' --header "Host: localhost:$port" \
			"https://localhost:8080$path"
	fi
}

persona_cookie() {
	local persona="$1"
	local gateway
	local port
	local headers
	local cookie
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

response_status() {
	printf '%s' "${1##*$'\n'}"
}

response_body() {
	printf '%s' "${1%$'\n'*}"
}

assert_concealed() {
	local response="$1"
	if [ "$(response_status "$response")" != "404" ]; then
		echo "Question Library access was not concealed" >&2
		exit 1
	fi
}

assert_search_payload() {
	local payload="$1"
	python3 -c '
import json
import sys

payload = json.loads(sys.argv[1])
items = payload.get("items")
if not isinstance(items, list) or len(items) != 4:
    raise SystemExit("Instructor Question Library did not return the fixed four-question baseline")
identifiers = {item.get("summary", {}).get("questionId") for item in items}
if identifiers != {"PNE-0001", "PNE-0002", "PNE-0003", "PNE-0004"}:
    raise SystemExit("Instructor Question Library returned an unexpected baseline")
rendered = json.dumps(payload, sort_keys=True)
for forbidden in ("correctChoice", "correctAnswer", "studentResponse", "sourceObject", "sourceChecksum", "objectAddress"):
    if forbidden in rendered:
        raise SystemExit("Question Library search exposed a protected field")
' "$payload"
}

assert_detail_payload() {
	local payload="$1"
	python3 -c '
import json
import sys

payload = json.loads(sys.argv[1])
if payload.get("summary", {}).get("questionId") != "PNE-0001":
    raise SystemExit("Question detail did not preserve the selected Question ID")
rendered = json.dumps(payload, sort_keys=True)
for forbidden in ("correctChoice", "correctAnswer", "studentResponse", "sourceObject", "sourceChecksum", "objectAddress"):
    if forbidden in rendered:
        raise SystemExit("Question detail exposed a protected field")
' "$payload"
}

prove_api() {
	local anonymous
	local student_cookie
	local student
	local instructor_cookie
	local instructor
	local detail

	anonymous="$(request '/api/questions/search?page_size=50')"
	assert_concealed "$anonymous"
	student_cookie="$(persona_cookie maryStudent)"
	student="$(request '/api/questions/search?page_size=50' "$student_cookie")"
	assert_concealed "$student"
	if [ "$(response_body "$anonymous")" != "$(response_body "$student")" ]; then
		echo "anonymous and Student Question Library concealment differs" >&2
		exit 1
	fi
	instructor_cookie="$(persona_cookie elenaInstructor)"
	instructor="$(request '/api/questions/search?page_size=50' "$instructor_cookie")"
	if [ "$(response_status "$instructor")" != "200" ]; then
		echo "Instructor Question Library search did not succeed" >&2
		exit 1
	fi
	assert_search_payload "$(response_body "$instructor")"
	detail="$(request '/api/questions/by-id/PNE-0001/detail' "$instructor_cookie")"
	if [ "$(response_status "$detail")" != "200" ]; then
		echo "Instructor Question Library detail did not succeed" >&2
		exit 1
	fi
	assert_detail_payload "$(response_body "$detail")"
	echo "Question Library API: Instructor success; Student and anonymous concealed"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	node tests/e2e/e2e_live_demo_question_library_browser.mjs "$port"
	echo "Question Library browser: navigation, search, and detail complete"
}

require_live_demo
case "$mode" in
	api) prove_api ;;
	browser) prove_browser ;;
	all)
		prove_api
		prove_browser
		;;
esac

echo "Live Demo Question Library: PASS"
