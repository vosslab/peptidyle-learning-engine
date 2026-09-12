#!/usr/bin/env bash
# Connected acceptance for Student Work evidence under a mutable Assignment.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
# shellcheck disable=SC1091
source "$repository_root/tests/e2e/e2e_live_demo_assignment_helpers.sh"
cd "$repository_root"

usage() { echo "Usage: bash tests/e2e/e2e_live_demo_assignment_attempt.sh [--start]" >&2; }
mode="all"
case "${1:-}" in
	"") ;;
	--start) mode="start" ;;
	*) usage; exit 2 ;;
esac

rerelease_latest_unreleased_current_assignment() {
	local instructor="$1" postgres output course assignment workspace edit released
	postgres="$(service_id postgres)"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "SELECT '\''C-'\'' || course.reference_number || '\'' A-'\'' || assignment.reference_number FROM ple_data.course_instance AS course JOIN ple_data.assignment AS assignment ON assignment.course_id=course.course_id WHERE assignment.assignment_status='\''unreleased'\'' AND assignment.assignment_title='\''Current Assignment'\'' ORDER BY assignment.reference_number DESC LIMIT 1"')"
	printf '%s\n' "$output" | rg -q '^C-[1-9][0-9]* A-[1-9][0-9]*$' || { echo "current unreleased Assignment prerequisite is unavailable" >&2; exit 1; }
	read -r course assignment <<<"$output"
	workspace="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor")"
	require_status "Instructor current unreleased Assignment read" "$workspace" 200
	read -r _ edit < <(workspace_reference_and_edit "$(response_body "$workspace")")
	released="$(request "/api/course-instances/$course/assignments/$assignment/release" "$instructor" POST '' "$edit")"
	require_status "Current Assignment re-release" "$released" 200
	printf '%s %s\n' "$course" "$assignment"
}

assert_evidence_rows() {
	local course="$1" assignment="$2" postgres sql output
	postgres="$(service_id postgres)"
	sql="SELECT CASE WHEN
		(SELECT count(*) FROM ple_private.assignment_attempt AS attempt
		 JOIN ple_data.assignment AS assignment_row ON assignment_row.assignment_id = attempt.assignment_id
		 JOIN ple_data.course_instance AS course_row ON course_row.course_id = assignment_row.course_id
		 WHERE course_row.reference_number = ${course#C-}
		   AND assignment_row.reference_number = ${assignment#A-}) = 2
		AND (SELECT count(*) FROM ple_private.issued_question AS issued
		     JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id = issued.assignment_attempt_id
		     JOIN ple_data.assignment AS assignment_row ON assignment_row.assignment_id = attempt.assignment_id
		     JOIN ple_data.course_instance AS course_row ON course_row.course_id = assignment_row.course_id
		     WHERE course_row.reference_number = ${course#C-}
		       AND assignment_row.reference_number = ${assignment#A-}) = 2
		AND NOT EXISTS (SELECT 1 FROM ple_private.issued_question AS issued
		                JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id = issued.assignment_attempt_id
		                JOIN ple_data.assignment AS assignment_row ON assignment_row.assignment_id = attempt.assignment_id
		                WHERE assignment_row.reference_number = ${assignment#A-}
		                  AND (issued.question_id IS NULL OR issued.revision_number < 1))
	THEN 'assignment_attempt_evidence' ELSE 'incomplete' END"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	[ "$output" = assignment_attempt_evidence ] || { echo "Assignment Attempt evidence rows are incomplete" >&2; exit 1; }
}

prove_start() {
	local instructor mary jack sysadmin course assignment access started question_reference workspace assignment_edit updated resumed new_attempt
	instructor="$(persona_cookie elenaInstructor)"; mary="$(persona_cookie maryStudent)"; jack="$(persona_cookie jackStudent)"; sysadmin="$(persona_cookie morganSysadmin)"
	bash "$repository_root/tests/e2e/e2e_live_demo_assignment_release.sh" --service >/dev/null
	read -r course assignment < <(rerelease_latest_unreleased_current_assignment "$instructor")

	assert_concealed "$(request "/api/course-instances/$course/assignments/$assignment/access")"
	assert_concealed "$(request "/api/course-instances/$course/assignments/$assignment/access" "$instructor")"
	assert_concealed "$(request "/api/course-instances/$course/assignments/$assignment/start" "$sysadmin" POST '{}')"
	claim_student_record "$course" "$instructor" "$mary" mary.okafor@live-demo.invalid current-model-mary
	claim_student_record "$course" "$instructor" "$jack" jack.nguyen@live-demo.invalid current-model-jack
	access="$(request "/api/course-instances/$course/assignments/$assignment/access" "$mary")"
	require_status "Student Assignment Access" "$access" 200

	started="$(request "/api/course-instances/$course/assignments/$assignment/start" "$mary" POST '{}')"
	require_status "Student Assignment start" "$started" 201
	question_reference="$(python3 -c 'import json,sys; print(json.dumps(json.loads(sys.argv[1])["questions"][0]["questionRevision"], separators=(",", ":")))' "$(response_body "$started")")"
	assert_started "$started" false "Current Assignment" "$question_reference"

	workspace="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor")"
	require_status "Instructor current Assignment read" "$workspace" 200
	read -r _ assignment_edit < <(workspace_reference_and_edit "$(response_body "$workspace")")
	updated="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor" PUT "$(retitle_payload "$(response_body "$workspace")" "Edited for future Attempts")" "$assignment_edit")"
	require_status "Released Assignment edit" "$updated" 200

	resumed="$(request "/api/course-instances/$course/assignments/$assignment/start" "$mary" POST '{}')"
	require_status "Existing Attempt resume" "$resumed" 201
	assert_started "$resumed" true "Current Assignment" "$question_reference"
	new_attempt="$(request "/api/course-instances/$course/assignments/$assignment/start" "$jack" POST '{}')"
	require_status "New Attempt start" "$new_attempt" 201
	assert_started "$new_attempt" false "Edited for future Attempts" "$question_reference"
	assert_evidence_rows "$course" "$assignment"
	echo "Assignment Attempt: authorization, exact Question Revision evidence, and released-edit boundary passed"
}

require_live_demo
case "$mode" in
	start) prove_start ;;
	all) prove_start ;;
esac
echo "Live Demo Assignment Attempt: PASS"
