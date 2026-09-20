#!/usr/bin/env bash
# Connected acceptance for one current Assessment creation, save, and release.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
# shellcheck disable=SC1091
source "$repository_root/tests/e2e/e2e_live_demo_assignment_helpers.sh"
cd "$repository_root"

usage() { echo "Usage: bash tests/e2e/e2e_live_demo_assignment_release.sh [--service|--browser]" >&2; }
mode="all"
case "${1:-}" in
	"") ;;
	--service) mode="service" ;;
	--browser) mode="browser" ;;
	*) usage; exit 2 ;;
esac

assert_workspace() {
	local response="$1" expected_status="$2" expected_question_revision_tuple="$3"
	python3 -c '
import json, sys
value=json.loads(sys.argv[1]); status=sys.argv[2]; question_revision=json.loads(sys.argv[3])
required={"id","editNumber","status","origin","assessmentType","title","instructions","dueAt","availableAt","closesAt","lateWorkRule","assessmentAttemptTimeLimitSeconds","attemptLimit","activityRules","studentFeedbackReleaseRule","displayTimeZone","entries","questions"}
if set(value) != required or value["status"] != status: raise SystemExit("workspace projection is not current and closed")
if status == "unreleased" and value["questions"] and value["questions"][0].get("questionRevisionTuple") != question_revision:
    raise SystemExit("workspace did not retain the exact Question Revision pin")
' "$(response_body "$response")" "$expected_status" "$expected_question_revision_tuple"
}

assert_unrelease_impact() {
	local response="$1" expected_title="$2" expected_edit="$3"
	python3 -c '
import json, sys
value=json.loads(sys.argv[1]); title=sys.argv[2]; edit=sys.argv[3]
required={"confirmationTitle","editNumber","attemptCount","submissionCount","gradeCount"}
if set(value) != required: raise SystemExit("Unrelease impact is not a closed aggregate projection")
if value["confirmationTitle"] != title or value["editNumber"] != edit:
    raise SystemExit("Unrelease impact did not retain the current title and Edit Number")
if any(not isinstance(value[key], int) or value[key] != 0 for key in ("attemptCount", "submissionCount", "gradeCount")):
    raise SystemExit("new Assignment unexpectedly has Student Work in its Unrelease impact")
' "$(response_body "$response")" "$expected_title" "$expected_edit"
}

assert_unreleased_response() {
	local response="$1" expected_title="$2" previous_edit="$3" expected_question_revision_tuple="$4"
	python3 -c '
import json, sys
value=json.loads(sys.argv[1]); title=sys.argv[2]; previous_edit=sys.argv[3]; question_revision=json.loads(sys.argv[4])
if set(value) != {"assessment", "deleted"}: raise SystemExit("Unrelease response is not a closed receipt")
assignment=value["assessment"]; deleted=value["deleted"]
workspace={"id","editNumber","status","origin","assessmentType","title","instructions","dueAt","availableAt","closesAt","lateWorkRule","assessmentAttemptTimeLimitSeconds","attemptLimit","activityRules","studentFeedbackReleaseRule","displayTimeZone","entries","questions"}
impact={"confirmationTitle","editNumber","attemptCount","submissionCount","gradeCount"}
if set(assignment) != workspace or set(deleted) != impact:
    raise SystemExit("Unrelease receipt contains an obsolete Assessment projection")
if assignment["status"] != "unreleased" or assignment["title"] != title:
    raise SystemExit("Unrelease did not return the current Unreleased Assessment")
if not isinstance(assignment["editNumber"], str) or int(assignment["editNumber"]) != int(previous_edit) + 1:
    raise SystemExit("Unrelease did not advance the Assessment Edit Number")
if assignment["questions"] and assignment["questions"][0].get("questionRevisionTuple") != question_revision:
    raise SystemExit("Unrelease lost the retained Question Revision pin")
if (deleted["confirmationTitle"] != title or deleted["editNumber"] != assignment["editNumber"]
    or any(not isinstance(deleted[key], int) or deleted[key] != 0 for key in ("attemptCount", "submissionCount", "gradeCount"))):
    raise SystemExit("Unrelease deletion receipt is not the expected redacted aggregate")
' "$(response_body "$response")" "$expected_title" "$previous_edit" "$expected_question_revision_tuple"
}

prove_service() {
	local instructor student sysadmin course picker question_revision_tuple created assignment initial_edit payload saved saved_edit stale validation released released_edit impact unreleased
	instructor="$(persona_cookie elenaInstructor)"
	student="$(persona_cookie maryStudent)"
	sysadmin="$(persona_cookie morganSysadmin)"
	course="$(new_course_instance_id "$instructor")"

	assert_concealed "$(request "/api/course-instances/$course/assessments")"
	assert_concealed "$(request "/api/course-instances/$course/assessments" "$student")"
	assert_concealed "$(request "/api/course-instances/$course/assessments" "$sysadmin")"
	created="$(request "/api/course-instances/$course/assessments" "$instructor" POST '{"assessmentType":"practice_question_assignment","title":"Current Assignment","instructions":"Use the selected Question."}')"
	require_status "Assessment creation" "$created" 201
	read -r assignment initial_edit < <(workspace_id_and_edit_number "$(response_body "$created")")
	assert_workspace "$created" unreleased '{}'

	picker="$(request "/api/course-instances/$course/assessment-question-picker" "$instructor")"
	require_status "Assessment Question picker" "$picker" 200
	question_revision_tuple="$(picker_question_revision_tuple "$(response_body "$picker")")"
	payload="$(save_payload "$(response_body "$created")" "$question_revision_tuple" "Current Assignment")"
	saved="$(request "/api/course-instances/$course/assessments/$assignment" "$instructor" PUT "$payload" "$initial_edit")"
	require_status "Assessment save" "$saved" 200
	read -r _ saved_edit < <(workspace_id_and_edit_number "$(response_body "$saved")")
	[ "$saved_edit" != "$initial_edit" ] || { echo "Assessment save did not advance its Edit Number" >&2; exit 1; }
	assert_workspace "$saved" unreleased "$question_revision_tuple"

	stale="$(request "/api/course-instances/$course/assessments/$assignment" "$instructor" PUT "$payload" "$initial_edit")"
	require_status "Stale Assessment save" "$stale" 412
	validation="$(request "/api/course-instances/$course/assessments/$assignment/release-validation" "$instructor")"
	require_status "Assessment release validation" "$validation" 200
	python3 -c 'import json,sys; value=json.loads(sys.argv[1]); assert value == {"canRelease": True, "issues": []}, value' "$(response_body "$validation")"
	released="$(request "/api/course-instances/$course/assessments/$assignment/release" "$instructor" POST '' "$saved_edit")"
	require_status "Assessment release" "$released" 200
	assert_workspace "$released" released "$question_revision_tuple"
	read -r _ released_edit < <(workspace_id_and_edit_number "$(response_body "$released")")
	impact="$(request "/api/course-instances/$course/assessments/$assignment/unrelease-impact" "$instructor")"
	require_status "Assessment Unrelease impact" "$impact" 200
	assert_unrelease_impact "$impact" "Current Assignment" "$released_edit"
	unreleased="$(request "/api/course-instances/$course/assessments/$assignment/unrelease" "$instructor" POST '{"confirmationTitle":"Current Assignment"}' "$released_edit")"
	require_status "Assessment Unrelease" "$unreleased" 200
	assert_unreleased_response "$unreleased" "Current Assignment" "$released_edit" "$question_revision_tuple"
	echo "Assessment release: current aggregate, exact Question Revision pin, CAS, release, and Unrelease receipt passed"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	NODE_EXTRA_CA_CERTS="$repository_root/local_stack_state/live_demo_browser/workspace/gateway-root.crt" \
		node tests/playwright/e2e_live_demo_assignment_release_browser.mjs "$port"
}

require_live_demo
case "$mode" in
	service) prove_service ;;
	browser) prove_browser ;;
	all) prove_service; prove_browser ;;
esac
echo "Live Demo Assessment Release: PASS"
