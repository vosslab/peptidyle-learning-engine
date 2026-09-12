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

# Returns response headers followed by the final HTTP status. Availability
# transitions need the server-issued lineage ETag, while the ordinary request
# helper intentionally keeps its browser-safe JSON body simple.
request_headers() {
	local path="$1"
	local cookie="${2:-}"
	local method="${3:-GET}"
	local body="${4:-}"
	local if_match="${5:-}"
	local gateway
	local port
	gateway="$(gateway_id)"
	port="$(gateway_port)"
	local -a curl_args=(--silent --show-error --insecure --max-time 12 --dump-header - --output /dev/null \
		--write-out $'\n%{http_code}' --header "Host: localhost:$port" --request "$method")
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
import re
import sys

payload = json.loads(sys.argv[1])
items = payload.get("items")
if not isinstance(items, list) or len(items) != 8:
    raise SystemExit("Instructor Question Library did not return the complete eight-question Pilot library")
expected_titles = {
    "Genetic disorders: Which one?",
    "Genetic disorders: Matching",
    "Genetics Chapter 1: Phenylalanine metabolism",
    "Genetics Chapter 1: Genetic disorder matching",
    "Biochemical functional groups: Which one?",
    "Biochemical functional groups: Matching",
    "Biochemistry Chapter 1: Charged functional groups",
    "Biochemistry Chapter 1: Functional group matching",
}
question_id = re.compile(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}")
seen_ids = set()
ple_item = None
for item in items:
    if not isinstance(item, dict) or set(item) != {"summary", "evidence"}:
        raise SystemExit("Question Library search returned an invalid browser-safe entry")
    summary = item.get("summary")
    if not isinstance(summary, dict):
        raise SystemExit("Question Library search omitted a Question summary")
    identifier = summary.get("questionId")
    revision = summary.get("latestQuestionRevision")
    metadata = summary.get("metadata")
    availability = summary.get("availability")
    if (not isinstance(identifier, str) or question_id.fullmatch(identifier) is None
        or identifier in seen_ids or not isinstance(revision, dict)
        or revision.get("questionId") != identifier
        or not isinstance(revision.get("revisionNumber"), int) or revision["revisionNumber"] < 1
        or not isinstance(metadata, dict) or metadata.get("questionTitle") not in expected_titles
        or not isinstance(availability, dict) or availability.get("availability") != "available"):
        raise SystemExit("Question Library search did not return the ordinary available Pilot publications")
    seen_ids.add(identifier)
    if summary.get("backend") == "ple":
        ple_item = summary
if len(seen_ids) != 8 or {item["summary"]["metadata"]["questionTitle"] for item in items} != expected_titles:
    raise SystemExit("Question Library did not preserve the reviewed Genetics and Biochemistry organization")
if ple_item is None:
    raise SystemExit("Question Library lacks a PLE-backed Pilot Question for detail acceptance")
rendered = json.dumps(payload, sort_keys=True)
for forbidden in ("correctChoice", "correctAnswer", "studentResponse", "sourceObject", "sourceChecksum", "objectAddress"):
    if forbidden in rendered:
        raise SystemExit("Question Library search exposed a protected field")
print(
    ple_item["questionId"],
    ple_item["latestQuestionRevision"]["revisionNumber"],
    ple_item["metadata"]["questionTitle"],
    sep="\t",
)
' "$payload"
}

optional_response_etag() {
	printf '%s\n' "$1" | tr -d '\r' | sed -n 's/^etag: \("[1-9][0-9]*"\)$/\1/Ip' | tail -n 1
}

response_etag() {
	local etag
	etag="$(optional_response_etag "$1")"
	if [ -z "$etag" ]; then
		echo "Question availability response did not include a strong ETag" >&2
		exit 1
	fi
	printf '%s\n' "$etag"
}

assert_available_summary() {
	local payload="$1" expected_id="$2"
	python3 -c '
import json
import sys

summary = json.loads(sys.argv[1])
expected_id = sys.argv[2]
availability = summary.get("availability") if isinstance(summary, dict) else None
if (summary.get("questionId") != expected_id
    or not isinstance(availability, dict)
    or availability.get("availability") != "available"):
    raise SystemExit("restored Question lineage was not ordinarily available")
' "$payload" "$expected_id"
}

assert_not_discoverable() {
	local payload="$1" expected_id="$2"
	python3 -c '
import json
import sys

payload = json.loads(sys.argv[1])
expected_id = sys.argv[2]
items = payload.get("items")
if not isinstance(items, list):
    raise SystemExit("Question Library search returned no item list after archive")
for item in items:
    if isinstance(item, dict) and item.get("summary", {}).get("questionId") == expected_id:
        raise SystemExit("archived Question remained available for ordinary search and new selection")
' "$payload" "$expected_id"
}

url_encode() {
	python3 -c 'import sys; from urllib.parse import quote; print(quote(sys.argv[1], safe=""))' "$1"
}

availability_archived_id=""
availability_restore_etag=""
availability_instructor_cookie=""
availability_archived_revision=""

restore_archived_question_on_exit() {
	local result="$?"
	local restored
	local revision_headers
	if [ -n "$availability_archived_id" ] && [ -n "$availability_restore_etag" ] && [ -n "$availability_instructor_cookie" ]; then
		if restored="$(request_headers "/api/questions/by-id/$availability_archived_id/restore" "$availability_instructor_cookie" POST '' "$availability_restore_etag")" \
			&& [ "$(response_status "$restored")" = "200" ]; then
			availability_archived_id=""
		else
			echo "Question availability cleanup could not restore $availability_archived_id" >&2
		fi
	elif [ -n "$availability_archived_id" ] && [ -n "$availability_archived_revision" ] && [ -n "$availability_instructor_cookie" ]; then
		# The immutable route remains available after archive and supplies the
		# current lineage ETag if a failed response prevented us from retaining it.
		if revision_headers="$(request_headers "/api/questions/by-id/$availability_archived_id/revisions/$availability_archived_revision" "$availability_instructor_cookie")" \
			&& [ "$(response_status "$revision_headers")" = "200" ]; then
			availability_restore_etag="$(optional_response_etag "$revision_headers")"
			if [ -n "$availability_restore_etag" ]; then
				restored="$(request_headers "/api/questions/by-id/$availability_archived_id/restore" "$availability_instructor_cookie" POST '' "$availability_restore_etag")" || true
				if [ "$(response_status "$restored")" = "200" ]; then
					availability_archived_id=""
				else
					echo "Question availability cleanup could not restore $availability_archived_id" >&2
				fi
			fi
		fi
	fi
	return "$result"
}

trap restore_archived_question_on_exit EXIT

assert_detail_payload() {
	local payload="$1" expected_id="$2" expected_revision="$3"
	python3 -c '
import json
import sys

payload = json.loads(sys.argv[1])
expected_id = sys.argv[2]
expected_revision = int(sys.argv[3])
summary = payload.get("summary", {})
revision = summary.get("latestQuestionRevision", {}) if isinstance(summary, dict) else {}
if (summary.get("questionId") != expected_id
    or revision.get("questionId") != expected_id
    or revision.get("revisionNumber") != expected_revision):
    raise SystemExit("Question detail did not preserve the selected exact Question Revision")
rendered = json.dumps(payload, sort_keys=True)
for forbidden in ("correctChoice", "correctAnswer", "studentResponse", "sourceObject", "sourceChecksum", "objectAddress"):
    if forbidden in rendered:
        raise SystemExit("Question detail exposed a protected field")
' "$payload" "$expected_id" "$expected_revision"
}

prove_api() {
	local anonymous
	local student_cookie
	local student
	local instructor_cookie
	local instructor
	local selected_id
	local selected_revision
	local selected_title
	local detail
	local lineage_headers
	local archive_headers
	local restore_headers
	local archived_search
	local archived_current
	local exact_revision
	local restored_current
	local restored_search
	local encoded_title

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
	IFS=$'\t' read -r selected_id selected_revision selected_title <<< "$(assert_search_payload "$(response_body "$instructor")")"
	detail="$(request "/api/questions/by-id/$selected_id/detail" "$instructor_cookie")"
	if [ "$(response_status "$detail")" != "200" ]; then
		echo "Instructor Question Library detail did not succeed" >&2
		exit 1
	fi
	assert_detail_payload "$(response_body "$detail")" "$selected_id" "$selected_revision"
	lineage_headers="$(request_headers "/api/questions/by-id/$selected_id" "$instructor_cookie")"
	if [ "$(response_status "$lineage_headers")" != "200" ]; then
		echo "Question availability journey could not load the selected lineage" >&2
		exit 1
	fi
	availability_archived_id="$selected_id"
	availability_instructor_cookie="$instructor_cookie"
	availability_archived_revision="$selected_revision"
	archive_headers="$(request_headers "/api/questions/by-id/$selected_id/archive" "$instructor_cookie" POST "{\"confirmationTitle\":$(python3 -c 'import json, sys; print(json.dumps(sys.argv[1]))' "$selected_title")}" "$(response_etag "$lineage_headers")")"
	if [ "$(response_status "$archive_headers")" != "200" ]; then
		echo "Instructor could not archive the selected Question lineage" >&2
		exit 1
	fi
	availability_restore_etag="$(response_etag "$archive_headers")"
	encoded_title="$(url_encode "$selected_title")"
	archived_search="$(request "/api/questions/search?page_size=100&text=$encoded_title" "$instructor_cookie")"
	if [ "$(response_status "$archived_search")" != "200" ]; then
		echo "Question Library search did not remain available after archive" >&2
		exit 1
	fi
	assert_not_discoverable "$(response_body "$archived_search")" "$selected_id"
	archived_current="$(request "/api/questions/by-id/$selected_id" "$instructor_cookie")"
	assert_concealed "$archived_current"
	exact_revision="$(request "/api/questions/by-id/$selected_id/revisions/$selected_revision" "$instructor_cookie")"
	if [ "$(response_status "$exact_revision")" != "200" ]; then
		echo "archiving a Question lineage broke its exact immutable revision" >&2
		exit 1
	fi
	assert_detail_payload "$(response_body "$exact_revision")" "$selected_id" "$selected_revision"
	restore_headers="$(request_headers "/api/questions/by-id/$selected_id/restore" "$instructor_cookie" POST '' "$availability_restore_etag")"
	if [ "$(response_status "$restore_headers")" != "200" ]; then
		echo "Instructor could not restore the archived Question lineage" >&2
		exit 1
	fi
	availability_archived_id=""
	availability_restore_etag=""
	restored_current="$(request "/api/questions/by-id/$selected_id" "$instructor_cookie")"
	if [ "$(response_status "$restored_current")" != "200" ]; then
		echo "restored Question lineage was not available for ordinary selection" >&2
		exit 1
	fi
	assert_available_summary "$(response_body "$restored_current")" "$selected_id"
	restored_search="$(request "/api/questions/search?page_size=100&text=$encoded_title" "$instructor_cookie")"
	if [ "$(response_status "$restored_search")" != "200" ]; then
		echo "Question Library search did not remain available after restore" >&2
		exit 1
	fi
	if ! python3 -c '
import json
import sys
print(any(item.get("summary", {}).get("questionId") == sys.argv[2] for item in json.loads(sys.argv[1]).get("items", [])))
' "$(response_body "$restored_search")" "$selected_id" | rg -qx 'True'; then
		echo "restored Question lineage did not return to ordinary search" >&2
		exit 1
	fi
	echo "Question Library API: Instructor success; Student and anonymous concealed"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	node tests/playwright/e2e_live_demo_question_library_browser.mjs "$port"
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
