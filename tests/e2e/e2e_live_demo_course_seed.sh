#!/usr/bin/env bash
# Connected acceptance for the ordinary Live Demo Course state.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
# shellcheck disable=SC1091
source "$repository_root/tests/e2e/e2e_live_demo_assignment_helpers.sh"
cd "$repository_root"

usage() { echo "Usage: bash tests/e2e/e2e_live_demo_course_seed.sh [--state|--authorization]" >&2; }
mode="all"
case "${1:-}" in
	"") ;;
	--state) mode="state" ;;
	--authorization) mode="authorization" ;;
	*) usage; exit 2 ;;
esac

readonly live_demo_course_long_name="Biochemistry 301: Proteins and Peptides"
readonly live_demo_assignment_title="Chapter 1 Pilot Practice"

live_demo_course_reference() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); long_name=sys.argv[2]
if not isinstance(value, dict) or set(value) != {"items", "nextCursor"} or value["nextCursor"] is not None or not isinstance(value["items"], list):
    raise SystemExit("Course list is not the closed current projection")
matches=[]
for item in value["items"]:
    if not isinstance(item, dict) or set(item) != {"reference", "shortName", "longName", "term", "theme"}:
        raise SystemExit("Course list item is not the closed current projection")
    if item.get("longName") == long_name:
        matches.append(item)
if len(matches) != 1:
    raise SystemExit("Live Demo Course is absent or duplicated")
reference=matches[0].get("reference")
if not isinstance(reference, str) or re.fullmatch(r"C-[1-9][0-9]{0,9}", reference) is None:
    raise SystemExit("Live Demo Course lacks a canonical public reference")
print(reference)
' "$1" "$live_demo_course_long_name"
}

live_demo_assignment_reference() {
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1]); title=sys.argv[2]
if not isinstance(items, list):
    raise SystemExit("Course Assignment list is malformed")
matches=[item for item in items if isinstance(item, dict) and item.get("title") == title]
if len(matches) != 1:
    raise SystemExit("Live Demo Assignment is absent or duplicated")
item=matches[0]
if set(item) != {"reference","title","dueAt","displayTimeZone","status","editNumber"}:
    raise SystemExit("Course Assignment list is not its current closed projection")
reference=item["reference"]
if not isinstance(reference, str) or re.fullmatch(r"A-[1-9][0-9]{0,9}", reference) is None:
    raise SystemExit("Live Demo Assignment lacks a canonical public reference")
print(reference)
' "$1" "$live_demo_assignment_title"
}

assert_current_assignment() {
	local response="$1" assignment="$2"
	python3 -c '
import json, sys
items=json.loads(sys.argv[1]); reference=sys.argv[2]
if not isinstance(items, list): raise SystemExit("Course Assignment list is malformed")
matches=[item for item in items if isinstance(item, dict) and item.get("reference") == reference]
if len(matches) != 1: raise SystemExit("Live Demo Assignment is absent or duplicated")
item=matches[0]
if set(item) != {"reference","title","dueAt","displayTimeZone","status","editNumber"}:
    raise SystemExit("Course Assignment list is not its current closed projection")
if item["status"] != "released" or not isinstance(item["editNumber"], str) or not item["editNumber"].isdigit():
    raise SystemExit("Live Demo Assignment is not a released current Assignment")
' "$(response_body "$response")" "$assignment"
}

prove_state() {
	local course assignment instructor courses assignments
	require_live_demo
	instructor="$(persona_cookie elenaInstructor)"
	courses="$(request '/api/course-instances' "$instructor")"
	require_status "Instructor Course list" "$courses" 200
	course="$(live_demo_course_reference "$(response_body "$courses")")"
	assignments="$(request "/api/course-instances/$course/assignments" "$instructor")"
	require_status "Instructor Course Assignment list" "$assignments" 200
	assignment="$(live_demo_assignment_reference "$(response_body "$assignments")")"
	assert_current_assignment "$assignments" "$assignment"
	echo "Live Demo Course seed: ordinary released current Assignment is available"
}

prove_authorization() {
	local course assignment instructor mary sysadmin courses assignments
	require_live_demo
	instructor="$(persona_cookie elenaInstructor)"; mary="$(persona_cookie maryStudent)"; sysadmin="$(persona_cookie morganSysadmin)"
	courses="$(request '/api/course-instances' "$instructor")"
	require_status "Instructor Course list" "$courses" 200
	course="$(live_demo_course_reference "$(response_body "$courses")")"
	assignments="$(request "/api/course-instances/$course/assignments" "$instructor")"
	require_status "Instructor Course Assignment list" "$assignments" 200
	assignment="$(live_demo_assignment_reference "$(response_body "$assignments")")"
	for path in \
		"/api/course-instances/$course/assignments" \
		"/api/course-instances/$course/assignments/$assignment" \
		"/api/course-instances/$course/assignment-question-picker"; do
		assert_concealed "$(request "$path")"
		assert_concealed "$(request "$path" "$mary")"
		assert_concealed "$(request "$path" "$sysadmin")"
	done
	require_status "Instructor current Assignment workspace" "$(request "/api/course-instances/$course/assignments/$assignment" "$instructor")" 200
	echo "Live Demo Course seed: Instructor-only Assignment authoring remains concealed"
}

case "$mode" in
	state) prove_state ;;
	authorization) prove_authorization ;;
	all) prove_state; prove_authorization ;;
esac
echo "Live Demo Course seed: PASS"
