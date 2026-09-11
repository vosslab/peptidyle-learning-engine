#!/usr/bin/env bash
# Disposable acceptance: one native PLE Assignment Attempt submission.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"
readonly manifest_path="local_stack_state/live_demo_browser/workspace/disposable.manifest"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_submission_recovery.sh [--submit|--fault]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--submit) mode="submit" ;;
	--fault) mode="fault" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Question Submission evidence requires the fixed Live Demo to be running" >&2
		exit 2
	fi
}

service_id() {
	local service="$1" identifiers
	identifiers="$(podman ps -q --filter "label=com.docker.compose.project=$project_name" --filter "label=com.docker.compose.service=$service")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Question Submission evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Question Submission evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1" cookie="${2:-}" method="${3:-GET}" body="${4:-}"
	local gateway port
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	local -a args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}' --header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json'); fi
	if [ -n "$cookie" ]; then args+=(--header "Cookie: $cookie"); fi
	if [ -n "$body" ]; then args+=(--data "$body"); fi
	podman exec "$gateway" curl "${args[@]}" "https://localhost:8080$path"
}

persona_cookie() {
	local persona="$1" gateway port headers cookie
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 --dump-header - --output /dev/null --header "Host: localhost:$port" --header "Origin: https://localhost:$port" --header 'Content-Type: application/json' --request POST --data "{\"persona\":\"$persona\"}" 'https://localhost:8080/api/auth/live-demo/accounts')"
	cookie="$(printf '%s\n' "$headers" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"
	if [ -z "$cookie" ]; then echo "seeded demo did not issue an Authenticated Session" >&2; exit 1; fi
	printf '%s\n' "$cookie"
}

response_status() { printf '%s' "${1##*$'\n'}"; }
response_body() { printf '%s' "${1%$'\n'*}"; }

current_native_ple_assignment() {
	local postgres output
	postgres="$(service_id postgres)"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "SELECT 'C-' || course.reference_number || ' A-' || assignment.reference_number FROM ple_private.question_attempt AS question_attempt JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN ple_private.assignment_attempt AS assignment_attempt ON assignment_attempt.assignment_attempt_id=issued.assignment_attempt_id JOIN ple_data.assignment AS assignment ON assignment.assignment_id=assignment_attempt.assignment_id JOIN ple_data.course_instance AS course ON course.course_id=assignment.course_id JOIN ple_data.student_record AS student_record ON student_record.student_record_id=assignment_attempt.student_record_id JOIN ple_private.question_revision_source_binding AS source_binding ON source_binding.question_id=issued.question_id AND source_binding.revision_number=issued.revision_number JOIN ple_private.account_authentication_email AS email ON email.account_id=student_record.student_account_id WHERE email.normalized_email='mary.okafor@live-demo.invalid' AND source_binding.backend='ple' AND source_binding.question_format='pleQuestionJson' AND question_attempt.question_attempt_state='open' AND NOT EXISTS (SELECT 1 FROM ple_private.question_submission AS submission WHERE submission.question_attempt_id=question_attempt.question_attempt_id) ORDER BY assignment_attempt.started_at DESC LIMIT 1;")"
	if ! printf '%s\n' "$output" | rg -q '^C-[1-9][0-9]* A-[1-9][0-9]*$'; then echo "Question Submission prerequisite lacks one open native PLE presentation" >&2; exit 1; fi
	printf '%s\n' "$output"
}

assignment_attempt_reference() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1])
attempt=value.get("assignmentAttempt")
if not isinstance(attempt,str) or not re.fullmatch(r"R-[1-9][0-9]*",attempt):
    raise SystemExit("Assignment start did not return an Assignment Attempt reference")
print(attempt)
' "$1"
}

selected_response_body() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1])
if set(value) != {"position","presentation","savedResponse"} or value["position"] != 1:
    raise SystemExit("Student Question selection is not the closed first-position projection")
presentation=value["presentation"]
if not isinstance(presentation,dict) or set(presentation) != {"prompt","response"}:
    raise SystemExit("Student Question selection did not return an answer-free presentation")
response_format=presentation["response"]
# This saves the ordinary seeded single-choice PLE question. It derives a
# format-valid choice from the public descriptor without reading source or an answer.
if response_format.get("kind") != "singleChoice" or set(response_format) != {"kind","choices"}:
    raise SystemExit("Student Question selection did not return the fixed single-choice PLE format")
choices=response_format["choices"]
if not isinstance(choices,list) or not choices or not isinstance(choices[0],dict) or set(choices[0]) != {"id","body"} or not isinstance(choices[0]["id"],str):
    raise SystemExit("Student Question response choices are malformed")
print(json.dumps({"response":{"kind":"multipleChoice","selected":[choices[0]["id"]]}},separators=(",",":")))
' "$1"
}

assert_saved_response_receipt() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1]); expected={"assignmentAttempt":sys.argv[2],"position":1,"responseState":"saved"}
if value != expected:
    raise SystemExit("Student Response save receipt was not the closed saved projection")
' "$1" "$2"
}

assert_assignment_submission_receipt() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1]); expected={"assignmentAttempt":sys.argv[2],"submissionState":"submitted"}
if value != expected:
    raise SystemExit("Assignment Attempt submission receipt was not the closed submitted projection")
' "$1" "$2"
}

assert_submission_evidence() {
	local assignment_attempt="$1" postgres sql output
	postgres="$(service_id postgres)"
	if ! [[ "$assignment_attempt" =~ ^R-[1-9][0-9]*$ ]]; then echo "Assignment Attempt reference is malformed" >&2; exit 1; fi
	sql="WITH target AS (SELECT attempt.assignment_attempt_id FROM ple_private.assignment_attempt AS attempt WHERE attempt.reference_number=${assignment_attempt#R-}), accepted AS (SELECT submission.submission_id FROM ple_private.question_submission AS submission JOIN ple_private.question_attempt AS question_attempt ON question_attempt.question_attempt_id=submission.question_attempt_id JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN target ON target.assignment_attempt_id=issued.assignment_attempt_id) SELECT CASE WHEN (SELECT count(*) FROM target)=1 AND (SELECT count(*) FROM ple_private.assignment_submission AS submission JOIN target ON target.assignment_attempt_id=submission.assignment_attempt_id)=1 AND (SELECT count(*) FROM accepted)=1 AND (SELECT count(*) FROM ple_private.job AS job JOIN accepted ON accepted.submission_id=job.question_submission_id WHERE job.job_kind='grade_accepted_submission' AND job.job_target_kind='question_submission' AND job.state IN ('ready','leased'))=1 AND (SELECT count(*) FROM ple_private.question_submission_grading AS grading JOIN accepted ON accepted.submission_id=grading.submission_id WHERE grading.grading_state='pending')=1 THEN 'native_ple_submission_persisted' ELSE 'native_ple_submission_invalid' END;"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^native_ple_submission_persisted$/p')" != "native_ple_submission_persisted" ]; then echo "Assignment Attempt submission persistence evidence was not recorded" >&2; exit 1; fi
}

last_assignment_attempt=""

prove_submit() {
	local student_cookie references course assignment started assignment_attempt selected response saved finalized repeated
	# Assignment access creates and authorizes the answer-free native PLE
	# Question Presentation through the public C-/A- routes. The selected
	# Question is saved through the Assignment Attempt before whole-Attempt finalization.
	bash "$repository_root/tests/e2e/e2e_live_demo_assignment_attempt.sh" --start >/dev/null
	student_cookie="$(persona_cookie maryStudent)"
	references="$(current_native_ple_assignment)"; read -r course assignment <<<"$references"
	started="$(request "/api/course-instances/$course/assignments/$assignment/start" "$student_cookie" POST '{}')"
	if [ "$(response_status "$started")" != "201" ]; then echo "Student could not resume the native PLE Assignment" >&2; exit 1; fi
	assignment_attempt="$(assignment_attempt_reference "$(response_body "$started")")"
	selected="$(request "/api/assignment-attempts/$assignment_attempt/student-question?position=1" "$student_cookie")"
	if [ "$(response_status "$selected")" != "200" ]; then echo "Student could not select the native PLE Question" >&2; exit 1; fi
	response="$(selected_response_body "$(response_body "$selected")")"
	saved="$(request "/api/assignment-attempts/$assignment_attempt/responses/1" "$student_cookie" PUT "$response")"
	if [ "$(response_status "$saved")" != "200" ]; then echo "Student Response was not saved" >&2; exit 1; fi
	assert_saved_response_receipt "$(response_body "$saved")" "$assignment_attempt"
	finalized="$(request "/api/assignment-attempts/$assignment_attempt/submission" "$student_cookie" POST '{}')"
	if [ "$(response_status "$finalized")" != "200" ]; then echo "Assignment Attempt was not submitted" >&2; exit 1; fi
	assert_assignment_submission_receipt "$(response_body "$finalized")" "$assignment_attempt"
	repeated="$(request "/api/assignment-attempts/$assignment_attempt/submission" "$student_cookie" POST '{}')"
	if [ "$(response_status "$repeated")" != "200" ]; then echo "Assignment Attempt submission was not idempotent" >&2; exit 1; fi
	assert_assignment_submission_receipt "$(response_body "$repeated")" "$assignment_attempt"
	assert_submission_evidence "$assignment_attempt"
	last_assignment_attempt="$assignment_attempt"
	echo "Assignment Attempt authority: one format-valid Student Response is saved and finalization records one pending native PLE grading job"
}

wait_for_leased_submission() {
	local assignment_attempt="$1" postgres output sql
	postgres="$(service_id postgres)"
	sql="WITH target AS (SELECT attempt.assignment_attempt_id FROM ple_private.assignment_attempt AS attempt WHERE attempt.reference_number=${assignment_attempt#R-}) SELECT CASE WHEN (SELECT count(*) FROM ple_private.job AS job JOIN ple_private.question_submission AS submission ON submission.submission_id=job.question_submission_id JOIN ple_private.question_attempt AS question_attempt ON question_attempt.question_attempt_id=submission.question_attempt_id JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN target ON target.assignment_attempt_id=issued.assignment_attempt_id WHERE job.job_kind='grade_accepted_submission' AND job.state='leased')=1 THEN 'native_ple_submission_leased' ELSE 'native_ple_submission_waiting' END;"
	for attempt in $(seq 1 30); do
		output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
		if [ "$output" = "native_ple_submission_leased" ]; then return; fi
		sleep 0.1
	done
	echo "Question Submission recovery did not observe the native PLE lease fault point" >&2
	exit 1
}

assert_recovery_evidence() {
	local assignment_attempt="$1" postgres output sql
	postgres="$(service_id postgres)"
	sql="WITH target AS (SELECT attempt.assignment_attempt_id FROM ple_private.assignment_attempt AS attempt WHERE attempt.reference_number=${assignment_attempt#R-}), accepted AS (SELECT submission.submission_id FROM ple_private.question_submission AS submission JOIN ple_private.question_attempt AS question_attempt ON question_attempt.question_attempt_id=submission.question_attempt_id JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN target ON target.assignment_attempt_id=issued.assignment_attempt_id) SELECT CASE WHEN (SELECT count(*) FROM target)=1 AND (SELECT count(*) FROM ple_private.assignment_submission AS submission JOIN target ON target.assignment_attempt_id=submission.assignment_attempt_id)=1 AND (SELECT count(*) FROM accepted)=1 AND (SELECT count(*) FROM ple_private.job AS job JOIN accepted ON accepted.submission_id=job.question_submission_id WHERE job.job_kind='grade_accepted_submission' AND job.job_target_kind='question_submission' AND job.state='completed')=1 AND (SELECT count(*) FROM ple_private.question_submission_grading AS grading JOIN accepted ON accepted.submission_id=grading.submission_id WHERE grading.grading_state='graded')=1 AND (SELECT count(*) FROM ple_private.grading_result AS result JOIN accepted ON accepted.submission_id=result.submission_id)=1 AND (SELECT count(*) FROM ple_audit.automated_grading_receipt AS receipt JOIN ple_private.grading_result AS result ON result.grading_result_id=receipt.grading_result_id JOIN accepted ON accepted.submission_id=result.submission_id)=1 THEN 'native_ple_recovery_committed' ELSE 'native_ple_recovery_waiting' END;"
	for attempt in $(seq 1 30); do
		output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
		if [ "$output" = "native_ple_recovery_committed" ]; then return; fi
		sleep 1
	done
	echo "Question Submission recovery did not commit one terminal native PLE result" >&2
	exit 1
}

prove_fault_recovery() {
	if [ ! -f "$manifest_path" ]; then
		echo "Question Submission recovery requires the controller-owned Live Demo manifest" >&2
		exit 2
	fi
	prove_submit
	wait_for_leased_submission "$last_assignment_attempt"
	# The dedicated controller action interrupts the worker after its durable
	# lease but before evaluation/commit; no generic Compose authority is used.
	python3 -m local_stack_control.disposable_stack_command stop-native-ple-worker --manifest "$manifest_path" >/dev/null
	python3 -m local_stack_control.disposable_stack_command replace-native-ple-worker --manifest "$manifest_path" >/dev/null
	assert_recovery_evidence "$last_assignment_attempt"
	echo "Assignment Attempt recovery: interrupted leased native PLE evaluation recovered one terminal result and receipt"
}

case "$mode" in
	fault)
		require_live_demo
		prove_fault_recovery
		;;
	submit)
		require_live_demo
		prove_submit
		;;
	all)
		require_live_demo
		prove_fault_recovery
		;;
esac
echo "Live Demo Question Submission: PASS"
