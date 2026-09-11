#!/usr/bin/env bash
# Disposable proof: private WeBWorK render issue and replay.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"
readonly webwork_question_id="7K3-M9QX"
readonly webwork_object_id="00000000-0000-0000-0000-000000001401"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_webwork.sh [--render|--grade]" >&2
}

mode="render"
case "${1:-}" in
	"") ;;
	--render) ;;
	--grade) mode="grade" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "WeBWorK render evidence requires the fixed Live Demo to be running" >&2
		exit 2
	fi
	python3 local_stack.py status --project "$project_name" >/dev/null
}

service_id() {
	local service="$1" identifiers
	identifiers="$(podman ps -q --filter "label=com.docker.compose.project=$project_name" --filter "label=com.docker.compose.service=$service")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "WeBWorK render evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "WeBWorK render evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1" cookie="${2:-}" method="${3:-GET}" body="${4:-}" if_match="${5:-}"
	local gateway port
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	local -a args=(--silent --show-error --insecure --max-time 15 --write-out $'\n%{http_code}' --header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json'); fi
	if [ -n "$cookie" ]; then args+=(--header "Cookie: $cookie"); fi
	if [ -n "$if_match" ]; then args+=(--header "If-Match: \"$if_match\""); fi
	if [ -n "$body" ]; then args+=(--data "$body"); fi
	podman exec "$gateway" curl "${args[@]}" "https://localhost:8080$path"
}

persona_cookie() {
	local persona="$1" gateway port headers cookie
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 15 --dump-header - --output /dev/null --header "Host: localhost:$port" --header "Origin: https://localhost:$port" --header 'Content-Type: application/json' --request POST --data "{\"persona\":\"$persona\"}" 'https://localhost:8080/api/auth/live-demo/accounts')"
	cookie="$(printf '%s\n' "$headers" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"
	if [ -z "$cookie" ]; then echo "seeded demo did not issue an Authenticated Session" >&2; exit 1; fi
	printf '%s\n' "$cookie"
}

response_status() { printf '%s' "${1##*$'\n'}"; }
response_body() { printf '%s' "${1%$'\n'*}"; }

private_webwork_source() {
	# This fixed, one-time source is piped only into the private bucket.  It is
	# deliberately never echoed, included in a browser request, or logged.
	printf '%s\n' \
		'DOCUMENT();' \
		'loadMacros("PGstandard.pl", "PGML.pl", "parserRadioButtons.pl");' \
		'$choice = RadioButtons(["hydrophobic", "polar", "charged"], "hydrophobic");' \
		'BEGIN_PGML' \
		'Choose the amino-acid side-chain category described by the prompt.' \
		'[_]{$choice}' \
		'END_PGML' \
		'ENDDOCUMENT();'
}

install_private_webwork_source() {
	local scratch source_file checksum size_bytes metadata object_path minio postgres
	scratch="$(mktemp -d)"
	trap 'rm -rf "$scratch"' RETURN
	source_file="$scratch/source.pg"
	private_webwork_source >"$source_file"
	chmod 600 "$source_file"
	checksum="$(shasum -a 256 "$source_file" | awk '{print $1}')"
	size_bytes="$(wc -c <"$source_file" | tr -d '[:space:]')"
	metadata="$(python3 -c '
import base64, json, sys
question_id, object_id, checksum, size = sys.argv[1:]
record = {"id": object_id, "storageArea": "private-content", "dataClass": "question-source", "address": {"kind": "questionSource", "questionRevision": {"questionId": question_id, "revisionNumber": 1}, "object": object_id}, "sha256": checksum, "sizeBytes": int(size), "mediaType": "text/x-weBWorK", "questionRevision": {"questionId": question_id, "revisionNumber": 1}, "createdAt": 1788652800000}
print(base64.urlsafe_b64encode(json.dumps(record, separators=(",", ":")).encode("ascii")).decode("ascii").rstrip("="))
' "$webwork_question_id" "$webwork_object_id" "$checksum" "$size_bytes")"
	object_path="questions/$webwork_question_id/versions/1/source/$webwork_object_id"
	minio="$(service_id minio)"
	# The bucket receives the immutable bytes and attested record before the
	# bootstrap-owned DB facts.  Neither operation is a browser shortcut.
	podman exec -i "$minio" sh -ec '
	mc alias set seeded http://127.0.0.1:9000 "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD" >/dev/null
	file=$(mktemp /tmp/m14-webwork-source.XXXXXX)
	trap "rm -f \"$file\"" EXIT
	cat >"$file"
	mc cp --disable-multipart --custom-header "Content-Type: $1" --attr "ple-record-v1=$2" "$file" "$3" >/dev/null
' sh 'text/x-weBWorK' "$metadata" "seeded/private-content/$object_path" <"$source_file"
	postgres="$(service_id postgres)"
	podman exec -i "$postgres" sh -ec 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB"' <<SQL >/dev/null
BEGIN;
INSERT INTO ple_data.published_question (question_id, created_at)
VALUES ('$webwork_question_id', '2026-09-06T00:00:00Z') ON CONFLICT (question_id) DO NOTHING;
INSERT INTO ple_data.question_revision (question_id, revision_number, backend, published_at)
VALUES ('$webwork_question_id', 1, 'webwork', '2026-09-06T00:00:00Z') ON CONFLICT (question_id, revision_number) DO NOTHING;
INSERT INTO ple_data.published_question_metadata (question_id, question_title, question_description, created_at, updated_at)
VALUES ('$webwork_question_id', 'WeBWorK rendered category', 'A renderer-backed Published Question for disposable delivery evidence.', '2026-09-06T00:00:00Z', '2026-09-06T00:00:00Z') ON CONFLICT (question_id) DO NOTHING;
INSERT INTO ple_private.object_record (object_id, object_address, object_storage_area, object_data_class, sha256, size_bytes, media_type, created_at)
VALUES ('$webwork_object_id', jsonb_build_object('kind','questionSource','questionRevision',jsonb_build_object('questionId','$webwork_question_id','revisionNumber',1),'object','$webwork_object_id'::uuid), 'private-content', 'question-source', decode('$checksum','hex'), $size_bytes, 'text/x-weBWorK', '2026-09-06T00:00:00Z') ON CONFLICT (object_id) DO NOTHING;
INSERT INTO ple_private.question_revision_source_binding (question_id, revision_number, backend, question_format, source_object_id, source_object_checksum, webwork_pg_path, created_at)
VALUES ('$webwork_question_id', 1, 'webwork', 'webworkPg', '$webwork_object_id', '$checksum', 'private/live-demo-render.pg', '2026-09-06T00:00:00Z') ON CONFLICT (question_id, revision_number) DO NOTHING;
INSERT INTO ple_data.question_revision_acceptance (question_id, revision_number, parent_revision_number, editor_account_id, accepted_by_account_id, accepted_at, reason_for_edit)
VALUES ('$webwork_question_id', 1, NULL, '00000000-0000-0000-0000-000000000101', '00000000-0000-0000-0000-000000000101', '2026-09-06T00:00:00Z', 'Disposable renderer evidence') ON CONFLICT (question_id, revision_number) DO NOTHING;
INSERT INTO ple_data.question_revision_authorship (question_id, revision_number, author_position, author_display_name, author_account_id)
VALUES ('$webwork_question_id', 1, 1, 'Live Demo Instructor', '00000000-0000-0000-0000-000000000101') ON CONFLICT (question_id, revision_number, author_position) DO NOTHING;
INSERT INTO ple_data.question_revision_license (question_id, revision_number, spdx_expression)
VALUES ('$webwork_question_id', 1, 'CC-BY-SA-4.0') ON CONFLICT (question_id, revision_number) DO NOTHING;
INSERT INTO ple_data.question_ownership_event (question_ownership_event_id, question_id, owner_account_id, recorded_by_account_id, event_kind, occurred_at)
VALUES ('00000000-0000-0000-0000-000000002401', '$webwork_question_id', '00000000-0000-0000-0000-000000000101', '00000000-0000-0000-0000-000000000101', 'initial', '2026-09-06T00:00:00Z') ON CONFLICT (question_ownership_event_id) DO NOTHING;
INSERT INTO ple_data.question_publication_event (event_id, question_id, revision_number, published_at)
VALUES ('00000000-0000-0000-0000-000000003401', '$webwork_question_id', 1, '2026-09-06T00:00:00Z') ON CONFLICT (event_id) DO NOTHING;
INSERT INTO ple_data.question_revision_availability_event (event_id, question_id, revision_number, availability, reason, occurred_at)
VALUES ('00000000-0000-0000-0000-000000004401', '$webwork_question_id', 1, 'available', NULL, '2026-09-06T00:00:00Z') ON CONFLICT (event_id) DO NOTHING;
COMMIT;
SQL
	trap - RETURN
	rm -rf "$scratch"
}

released_course_assignment() {
	local postgres output
	postgres="$(service_id postgres)"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "SELECT '\''C-'\'' || course.reference_number || '\'' A-'\'' || assignment.reference_number FROM ple_data.course_instance course JOIN ple_data.assignment assignment ON assignment.course_id = course.course_id WHERE assignment.assignment_status = '\''released'\'' ORDER BY course.reference_number DESC, assignment.reference_number DESC LIMIT 1"')"
	if ! printf '%s\n' "$output" | rg -q '^C-[1-9][0-9]* A-[1-9][0-9]*$'; then echo "WeBWorK prerequisite lacks public Course and Assignment references" >&2; exit 1; fi
	printf '%s\n' "$output"
}

assignment_reference_and_edit() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1])
if not re.fullmatch(r"A-[1-9][0-9]{0,9}", value.get("reference", "")) or not isinstance(value.get("editNumber"), str) or not value["editNumber"].isdigit():
    raise SystemExit("WeBWorK Assignment Workspace receipt is malformed")
print(value["reference"], value["editNumber"])
' "$1"
}

claim_student_record() {
	local course="$1" instructor_cookie="$2" student_cookie="$3" imported claimed
	imported="$(request "/api/course-instances/$course/roster" "$instructor_cookie" POST '{"entries":[{"email":"mary.okafor@live-demo.invalid","rosterId":"m14-student"}]}')"
	if [ "$(response_status "$imported")" != "201" ]; then echo "Instructor could not import the M14 Student roster row" >&2; exit 1; fi
	claimed="$(request "/api/course-instances/$course/roster/claim" "$student_cookie" POST '{}')"
	if [ "$(response_status "$claimed")" != "200" ] || [ "$(response_body "$claimed")" != '{"activeStudentMembership":true}' ]; then echo "Student could not claim the exact Course Invitation" >&2; exit 1; fi
}

release_webwork_assignment() {
	local course="$1" instructor_cookie="$2" created assignment edit saved released
	created="$(request "/api/course-instances/$course/assignments" "$instructor_cookie" POST '{"title":"M14 WeBWorK render","instructions":"Complete the rendered question."}')"
	if [ "$(response_status "$created")" != "201" ]; then echo "Instructor could not create the WeBWorK Assignment" >&2; exit 1; fi
	read -r assignment edit < <(assignment_reference_and_edit "$(response_body "$created")")
	saved="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor_cookie" PUT "{\"title\":\"M14 WeBWorK render\",\"instructions\":\"Complete the rendered question.\",\"questionIds\":[\"$webwork_question_id\"],\"dueAt\":null,\"assignmentAttemptTimeLimitSeconds\":3600,\"lateWorkRule\":\"accept\"}" "$edit")"
	if [ "$(response_status "$saved")" != "200" ]; then echo "Instructor could not select the private renderer-backed Published Question" >&2; exit 1; fi
	edit="$(assignment_reference_and_edit "$(response_body "$saved")" | awk '{print $2}')"
	released="$(request "/api/course-instances/$course/assignments/$assignment/release" "$instructor_cookie" POST '{}' "$edit")"
	if [ "$(response_status "$released")" != "201" ]; then echo "Instructor could not release the all-WeBWorK Assignment" >&2; exit 1; fi
	printf '%s\n' "$assignment"
}

release_mixed_assignment() {
	local course="$1" instructor_cookie="$2" created assignment edit saved released
	created="$(request "/api/course-instances/$course/assignments" "$instructor_cookie" POST '{"title":"M14 mixed render","instructions":"Complete the released questions."}')"
	if [ "$(response_status "$created")" != "201" ]; then echo "Instructor could not create the mixed M14 Assignment" >&2; exit 1; fi
	read -r assignment edit < <(assignment_reference_and_edit "$(response_body "$created")")
	# The released order makes aggregation concrete: a native PLE Question and
	# the renderer-backed Question share one Assignment Attempt contract.
	saved="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor_cookie" PUT "{\"title\":\"M14 mixed render\",\"instructions\":\"Complete the released questions.\",\"questionIds\":[\"PNE-0001\",\"$webwork_question_id\"],\"dueAt\":null,\"assignmentAttemptTimeLimitSeconds\":3600,\"lateWorkRule\":\"accept\"}" "$edit")"
	if [ "$(response_status "$saved")" != "200" ]; then echo "Instructor could not select the mixed Published Questions" >&2; exit 1; fi
	edit="$(assignment_reference_and_edit "$(response_body "$saved")" | awk '{print $2}')"
	released="$(request "/api/course-instances/$course/assignments/$assignment/release" "$instructor_cookie" POST '{}' "$edit")"
	if [ "$(response_status "$released")" != "201" ]; then echo "Instructor could not release the mixed M14 Assignment" >&2; exit 1; fi
	printf '%s\n' "$assignment"
}

assert_webwork_presentation() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1])
if set(value) != {"assignment","assignmentAttempt","attemptNumber","resumed","title","instructions","questions"} or value["resumed"] is not (sys.argv[2] == "true"):
    raise SystemExit("WeBWorK Assignment start projection is invalid")
if not isinstance(value["assignmentAttempt"], str) or not re.fullmatch(r"R-[1-9][0-9]{0,9}", value["assignmentAttempt"]):
    raise SystemExit("WeBWorK Assignment start projection has an invalid public Assignment Attempt reference")
questions=value["questions"]
if not isinstance(questions,list) or len(questions) != 1:
    raise SystemExit("Assignment did not issue exactly one WeBWorK Question Presentation")
question=questions[0]
if not isinstance(question,dict) or question.get("questionRevision",{}).get("questionId") != "7K3-M9QX" or question.get("response",{}).get("kind") != "singleChoice":
    raise SystemExit("Assignment did not issue the expected rendered WeBWorK presentation")
rendered=json.dumps(value, sort_keys=True).lower()
for forbidden in ("answer", "correct", "solution", "grading", "score", "replay", "source", "webworkpgpath", "questionattemptid", "binding", "checksum"):
    if forbidden in rendered:
        raise SystemExit("WeBWorK browser projection exposed protected renderer evidence")
' "$1" "$2"
}

assert_same_presentation() {
	python3 -c '
import json, sys
if json.loads(sys.argv[1]).get("questions") != json.loads(sys.argv[2]).get("questions"):
    raise SystemExit("WeBWorK Assignment resume did not replay the exact public presentation")
' "$1" "$2"
}

assert_mixed_presentation() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1])
questions=value.get("questions")
if not isinstance(questions,list) or len(questions) != 2:
    raise SystemExit("Mixed Assignment did not issue two Question Presentations")
ids={question.get("questionRevision",{}).get("questionId") for question in questions if isinstance(question,dict)}
if ids != {"PNE-0001", "7K3-M9QX"}:
    raise SystemExit("Mixed Assignment did not retain its exact released Question revisions")
rendered=json.dumps(value, sort_keys=True).lower()
if any(word in rendered for word in ("answer", "correct", "solution", "grading", "score", "replay", "source", "webworkpgpath", "questionattemptid", "binding", "checksum")):
    raise SystemExit("Mixed Assignment projection exposed protected Question Backend evidence")
' "$1"
}

assert_private_replay() {
	local postgres course="$1" assignment="$2" output
	postgres="$(service_id postgres)"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "SELECT CASE WHEN (SELECT count(*) FROM ple_private.question_attempt_webwork_replay replay JOIN ple_private.question_attempt attempt ON attempt.question_attempt_id = replay.question_attempt_id JOIN ple_private.issued_question issued ON issued.issued_question_id = attempt.issued_question_id JOIN ple_private.assignment_attempt assignment_attempt ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id JOIN ple_data.assignment assignment ON assignment.assignment_id = assignment_attempt.assignment_id JOIN ple_data.course_instance course ON course.course_id = assignment.course_id WHERE course.reference_number = ${course#C-} AND assignment.reference_number = ${assignment#A-}) = 1 THEN 'private_replay_present' ELSE 'private_replay_missing' END;")"
	if [ "$output" != "private_replay_present" ]; then echo "WeBWorK issue did not retain one private replay record" >&2; exit 1; fi
}

assert_mixed_private_replay() {
	local postgres course="$1" assignment="$2" output
	postgres="$(service_id postgres)"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "WITH issued_attempts AS (SELECT attempt.question_attempt_id, issued.question_id FROM ple_private.question_attempt attempt JOIN ple_private.issued_question issued ON issued.issued_question_id = attempt.issued_question_id JOIN ple_private.assignment_attempt assignment_attempt ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id JOIN ple_data.assignment assignment ON assignment.assignment_id = assignment_attempt.assignment_id JOIN ple_data.course_instance course ON course.course_id = assignment.course_id WHERE course.reference_number = ${course#C-} AND assignment.reference_number = ${assignment#A-}) SELECT CASE WHEN (SELECT count(*) FROM issued_attempts) = 2 AND (SELECT count(*) FROM ple_private.question_attempt_webwork_replay replay JOIN issued_attempts issued_attempt ON issued_attempt.question_attempt_id = replay.question_attempt_id) = 1 AND (SELECT count(*) FROM ple_private.question_attempt_webwork_replay replay JOIN issued_attempts issued_attempt ON issued_attempt.question_attempt_id = replay.question_attempt_id WHERE issued_attempt.question_id = '$webwork_question_id') = 1 THEN 'mixed_private_replay_present' ELSE 'mixed_private_replay_invalid' END;")"
	if [ "$output" != "mixed_private_replay_present" ]; then echo "Mixed Assignment did not retain one private WeBWorK replay record across two Question Attempts" >&2; exit 1; fi
}

prove_render() {
	local instructor_cookie student_cookie refs course assignment mixed_assignment started resumed mixed_started port
	# The authoring journey supplies the legitimate Course and Instructor
	# authority. This runner adds one private, immutable renderer source only.
	bash "$repository_root/tests/e2e/e2e_live_demo_assignment_release.sh" --service >/dev/null
	install_private_webwork_source
	instructor_cookie="$(persona_cookie elenaInstructor)"
	student_cookie="$(persona_cookie maryStudent)"
	refs="$(released_course_assignment)"; read -r course _ <<<"$refs"
	assignment="$(release_webwork_assignment "$course" "$instructor_cookie")"
	claim_student_record "$course" "$instructor_cookie" "$student_cookie"
	started="$(request "/api/course-instances/$course/assignments/$assignment/start" "$student_cookie" POST '{}')"
	if [ "$(response_status "$started")" != "201" ]; then echo "Student could not start the all-WeBWorK Assignment" >&2; exit 1; fi
	assert_webwork_presentation "$(response_body "$started")" false
	resumed="$(request "/api/course-instances/$course/assignments/$assignment/start" "$student_cookie" POST '{}')"
	if [ "$(response_status "$resumed")" != "201" ]; then echo "Student could not resume the rendered Assignment" >&2; exit 1; fi
	assert_webwork_presentation "$(response_body "$resumed")" true
	assert_same_presentation "$(response_body "$started")" "$(response_body "$resumed")"
	assert_private_replay "$course" "$assignment"
	mixed_assignment="$(release_mixed_assignment "$course" "$instructor_cookie")"
	mixed_started="$(request "/api/course-instances/$course/assignments/$mixed_assignment/start" "$student_cookie" POST '{}')"
	if [ "$(response_status "$mixed_started")" != "201" ]; then echo "Student could not start the mixed M14 Assignment" >&2; exit 1; fi
	assert_mixed_presentation "$(response_body "$mixed_started")"
	assert_mixed_private_replay "$course" "$mixed_assignment"
	port="$(gateway_port)"
	node tests/playwright/e2e_live_demo_webwork_browser.mjs "$port" "$course" "$assignment"
	echo "WeBWorK render authority: private renderer issue, answer-free public presentation, and exact replay complete"
	echo "WeBWorK render browser: visible Student start and rendered answer-free Question Presentation complete"
}

webwork_response() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1]); question=value["questions"][0]
choices=question.get("response",{}).get("choices")
if not isinstance(choices,list) or not choices or not isinstance(choices[0].get("id"),str):
    raise SystemExit("WeBWorK presentation has no submit-ready single choice")
print(json.dumps({"response":{"kind":"multipleChoice","selected":[choices[0]["id"]]}},separators=(",",":")))
' "$1"
}

assignment_attempt_reference() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1])
attempt=value.get("activeAssignmentAttempt")
if set(value) != {"startDecision", "activeAssignmentAttempt"} or not isinstance(attempt, str) or not re.fullmatch(r"R-[1-9][0-9]{0,9}", attempt):
    raise SystemExit("Assignment Access did not expose the active public Assignment Attempt")
print(attempt)
' "$1"
}

assert_saved_response() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1])
expected={"assignmentAttempt":sys.argv[2],"position":1,"responseState":"saved"}
if value != expected:
    raise SystemExit("WeBWorK response save acknowledgement is malformed")
' "$1" "$2"
}

assert_assignment_submitted() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1])
expected={"assignmentAttempt":sys.argv[2],"submissionState":"submitted"}
if value != expected:
    raise SystemExit("WeBWorK Assignment Attempt finalization acknowledgement is malformed")
' "$1" "$2"
}

assert_webwork_job_state() {
	local course="$1" assignment="$2" expected="$3" postgres output
	postgres="$(service_id postgres)"
	for _ in $(seq 1 30); do
		output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "SELECT job.state || '|' || grading.grading_state FROM ple_private.job AS job JOIN ple_private.question_submission_grading AS grading ON grading.job_id = job.job_id JOIN ple_private.question_submission AS submission ON submission.submission_id = job.question_submission_id JOIN ple_private.question_attempt AS attempt ON attempt.question_attempt_id = submission.question_attempt_id JOIN ple_private.issued_question AS issued ON issued.issued_question_id = attempt.issued_question_id JOIN ple_private.assignment_attempt AS assignment_attempt ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id JOIN ple_data.assignment AS assignment ON assignment.assignment_id = assignment_attempt.assignment_id JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id WHERE course.reference_number = ${course#C-} AND assignment.reference_number = ${assignment#A-} ORDER BY job.created_at DESC LIMIT 1")"
		if [ "$output" = "$expected" ]; then return; fi
		sleep 1
	done
	echo "WeBWorK grading Job did not reach $expected" >&2; exit 1
}

prove_grade() {
	local instructor_cookie student_cookie refs course assignment started access attempt response saved finalized manifest_path outage_assignment outage_started outage_access outage_attempt outage_response outage_saved outage_finalized
	bash "$repository_root/tests/e2e/e2e_live_demo_assignment_release.sh" --service >/dev/null
	install_private_webwork_source
	instructor_cookie="$(persona_cookie elenaInstructor)"; student_cookie="$(persona_cookie maryStudent)"
	refs="$(released_course_assignment)"; read -r course _ <<<"$refs"
	claim_student_record "$course" "$instructor_cookie" "$student_cookie"
	assignment="$(release_webwork_assignment "$course" "$instructor_cookie")"
	started="$(request "/api/course-instances/$course/assignments/$assignment/start" "$student_cookie" POST '{}')"
	if [ "$(response_status "$started")" != "201" ]; then echo "Student could not start the WeBWorK grading Assignment" >&2; exit 1; fi
	access="$(request "/api/course-instances/$course/assignments/$assignment/access" "$student_cookie")"
	if [ "$(response_status "$access")" != "200" ]; then echo "Student could not read the active WeBWorK Assignment Attempt" >&2; exit 1; fi
	attempt="$(assignment_attempt_reference "$(response_body "$access")")"
	response="$(webwork_response "$(response_body "$started")")"
	saved="$(request "/api/assignment-attempts/$attempt/responses/1" "$student_cookie" PUT "$response")"
	if [ "$(response_status "$saved")" != "200" ]; then echo "Student WeBWorK Response was not saved" >&2; exit 1; fi
	assert_saved_response "$(response_body "$saved")" "$attempt"
	finalized="$(request "/api/assignment-attempts/$attempt/submission" "$student_cookie" POST '{}')"
	if [ "$(response_status "$finalized")" != "200" ]; then echo "Student could not submit the WeBWorK Assignment Attempt" >&2; exit 1; fi
	assert_assignment_submitted "$(response_body "$finalized")" "$attempt"
	assert_webwork_job_state "$course" "$assignment" 'completed|graded'
	outage_assignment="$(release_webwork_assignment "$course" "$instructor_cookie")"
	outage_started="$(request "/api/course-instances/$course/assignments/$outage_assignment/start" "$student_cookie" POST '{}')"
	if [ "$(response_status "$outage_started")" != "201" ]; then echo "Student could not start the renderer-outage Assignment" >&2; exit 1; fi
	outage_access="$(request "/api/course-instances/$course/assignments/$outage_assignment/access" "$student_cookie")"
	if [ "$(response_status "$outage_access")" != "200" ]; then echo "Student could not read the active renderer-outage Assignment Attempt" >&2; exit 1; fi
	outage_attempt="$(assignment_attempt_reference "$(response_body "$outage_access")")"
	outage_response="$(webwork_response "$(response_body "$outage_started")")"
	outage_saved="$(request "/api/assignment-attempts/$outage_attempt/responses/1" "$student_cookie" PUT "$outage_response")"
	if [ "$(response_status "$outage_saved")" != "200" ]; then echo "Renderer-outage Student Response was not saved before grading" >&2; exit 1; fi
	assert_saved_response "$(response_body "$outage_saved")" "$outage_attempt"
	manifest_path="local_stack_state/live_demo_browser/workspace/disposable.manifest"
	python3 -m local_stack_control.disposable_stack_command stop-webwork-renderer --manifest "$manifest_path" >/dev/null
	outage_finalized="$(request "/api/assignment-attempts/$outage_attempt/submission" "$student_cookie" POST '{}')"
	if [ "$(response_status "$outage_finalized")" != "200" ]; then echo "Renderer-outage Assignment Attempt was not submitted" >&2; exit 1; fi
	assert_assignment_submitted "$(response_body "$outage_finalized")" "$outage_attempt"
	assert_webwork_job_state "$course" "$outage_assignment" 'failed|instructor_attention'
	python3 -m local_stack_control.disposable_stack_command replace-webwork-renderer --manifest "$manifest_path" >/dev/null
	echo "WeBWorK grade authority: deterministic renderer grade commit and bounded renderer failure complete"
}

if [ "$mode" = "grade" ]; then
	require_live_demo
	prove_grade
	echo "Live Demo WeBWorK grade: PASS"
	exit 0
fi
require_live_demo
prove_render
echo "Live Demo WeBWorK render: PASS"
