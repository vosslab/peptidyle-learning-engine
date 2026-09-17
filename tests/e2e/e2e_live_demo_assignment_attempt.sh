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

# Choose an ordinary published Question through the Instructor API.  The
# acceptance journey deliberately selects a backend, not a source file or a
# question-control type: WeBWorK remains opaque to PLE and to this shell test.
published_question_reference() {
	local instructor="$1" backend="$2" listed
	listed="$(request "/api/questions/search?backends=$backend&authorship=any&page_size=50" "$instructor")"
	if [ "$(response_status "$listed")" != 200 ]; then
		echo "Instructor $backend Question Library search returned HTTP $(response_status "$listed"), expected 200" >&2
		return 1
	fi
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); backend=sys.argv[2]
items=value.get("items")
if not isinstance(items,list) or not items: raise SystemExit("Question Library has no selected backend Question")
summary=items[0].get("summary") if isinstance(items[0],dict) else None
reference=summary.get("latestQuestionRevision") if isinstance(summary,dict) else None
if not isinstance(summary,dict) or summary.get("backend") != backend or not isinstance(reference,dict):
    raise SystemExit("Question Library did not return the selected backend")
if (set(reference)!={"questionId","revisionNumber"} or not isinstance(reference["questionId"],str)
    or re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}",reference["questionId"]) is None
    or not isinstance(reference["revisionNumber"],int) or reference["revisionNumber"] < 1):
    raise SystemExit("Question Library backend reference is malformed")
print(json.dumps(reference,separators=(",",":")))
' "$(response_body "$listed")" "$backend"
}

canonical_native_question_reference() {
	local instructor="$1" listed
	listed="$(request '/api/questions/search?backends=ple&authorship=any&page_size=20' "$instructor")"
	if [ "$(response_status "$listed")" != 200 ]; then
		echo "Canonical native Question Library search returned HTTP $(response_status "$listed"), expected 200" >&2
		return 1
	fi
	python3 -c '
import json,re,sys
items=json.loads(sys.argv[1]).get("items",[])
for item in items:
    summary=item.get("summary") if isinstance(item,dict) else None
    metadata=summary.get("metadata") if isinstance(summary,dict) else None
    reference=summary.get("latestQuestionRevision") if isinstance(summary,dict) else None
    if (isinstance(metadata,dict) and metadata.get("questionTitle")=="Genetics Chapter 1: Phenylalanine metabolism"
        and summary.get("backend")=="ple" and isinstance(reference,dict)
        and set(reference)=={"questionId","revisionNumber"}
        and re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}",reference["questionId"])
        and isinstance(reference["revisionNumber"],int) and reference["revisionNumber"] > 0):
        print(json.dumps(reference,separators=(",",":"))); break
else: raise SystemExit("canonical native acceptance Question is unavailable")
' "$(response_body "$listed")"
}

create_released_backend_assignment() {
	local course="$1" instructor="$2" backend="$3" title="$4" duration="${5:-300}" reference="${6:-}" source_choices source created assignment edit saved saved_edit released payload
	source_choices="$(request "/api/course-instances/$course/assignment-source-choices" "$instructor")"
	require_status "Instructor Assignment source choices" "$source_choices" 200
	source="$(source_choice_reference "$(response_body "$source_choices")")"
	created="$(request "/api/course-instances/$course/assignments" "$instructor" POST "{\"blueprintAssignmentReference\":\"$source\",\"title\":\"$title\",\"instructions\":\"Complete the selected Question.\"}")"
	require_status "Backend Assignment creation" "$created" 201
	read -r assignment edit < <(workspace_reference_and_edit "$(response_body "$created")")
	[ -n "$reference" ] || reference="$(published_question_reference "$instructor" "$backend")"
	payload="$(save_payload "$(response_body "$created")" "$reference" "$title")"
	payload="$(python3 -c 'import json,sys; value=json.loads(sys.argv[1]); value["assignmentAttemptTimeLimitSeconds"]=int(sys.argv[2]); print(json.dumps(value,separators=(",",":")))' "$payload" "$duration")"
	saved="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor" PUT "$payload" "$edit")"
	require_status "Backend Assignment save" "$saved" 200
	read -r _ saved_edit < <(workspace_reference_and_edit "$(response_body "$saved")")
	released="$(request "/api/course-instances/$course/assignments/$assignment/release" "$instructor" POST '' "$saved_edit")"
	require_status "Backend Assignment release" "$released" 200
	printf '%s\n' "$assignment"
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
	claim_student_record "$course" "$instructor" "$mary" mary.okafor@biology.roosevelt.edu current-model-mary Mary
	claim_student_record "$course" "$instructor" "$jack" jack.nguyen@biology.roosevelt.edu current-model-jack Jack
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

assert_saved_backend_response() {
	python3 -c '
import json,sys
value=json.loads(sys.argv[1])
if set(value)!={"position","presentation","savedResponse"} or value["position"] != 1:
    raise SystemExit("selected Question projection is not closed")
response=value["savedResponse"]
if not isinstance(response,dict) or set(response)!={"kind","payload"} or response["kind"] != "backendOwned":
    raise SystemExit("renderer-owned saved response was not retained")
' "$(response_body "$1")"
}

assert_score() {
	python3 -c '
import json,sys
value=json.loads(sys.argv[1]); attempt=sys.argv[2]
if set(value)!={"assignmentAttempt","submissionState","score"} or value["assignmentAttempt"] != attempt or value["submissionState"] != "submitted":
    raise SystemExit("submission did not return its closed receipt")
score=value["score"]
if not isinstance(score,dict) or set(score)!={"pointsEarned","pointsPossible"}:
    raise SystemExit("submission did not return an immediate score")
if not all(isinstance(score[key],(int,float)) and not isinstance(score[key],bool) for key in score) or score["pointsPossible"] <= 0 or not 0 <= score["pointsEarned"] <= score["pointsPossible"]:
    raise SystemExit("submission returned an invalid immutable-fraction score")
print(json.dumps(score,separators=(",",":")))
' "$(response_body "$1")" "$2"
}

assert_renderer_fraction_matches_submission() {
	local attempt="$1" score="$2" postgres observed
	postgres="$(service_id postgres)"
	observed="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "SELECT json_build_object('pointsEarned', result.normalized_credit * entry.points_possible, 'pointsPossible', entry.points_possible)::text FROM ple_private.grading_result AS result JOIN ple_private.question_attempt AS question_attempt ON question_attempt.question_attempt_id=result.question_attempt_id JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN ple_private.assignment_attempt AS assignment_attempt ON assignment_attempt.assignment_attempt_id=issued.assignment_attempt_id JOIN ple_data.assignment_entry AS entry ON entry.assignment_entry_id=issued.assignment_entry_id WHERE assignment_attempt.reference_number=${attempt#R-}")"
	python3 -c '
import json,sys
expected=json.loads(sys.argv[1]); actual=json.loads(sys.argv[2])
if expected != actual:
    raise SystemExit("PLE submission score does not equal the renderer-derived stored credit fraction")
' "$observed" "$score"
}

native_response_payload() {
	python3 -c '
import json,sys
value=json.loads(sys.argv[1]); presentation=value.get("presentation",{}); response=presentation.get("response",{})
if response.get("kind") != "singleChoice" or not isinstance(response.get("choices"),list) or not response["choices"]:
    raise SystemExit("canonical native acceptance requires its single-choice PLE Question")
def text(value):
    if isinstance(value,dict): return " ".join(text(item) for item in value.values())
    if isinstance(value,list): return " ".join(text(item) for item in value)
    return value if isinstance(value,str) else ""
matches=[choice for choice in response["choices"] if isinstance(choice,dict) and isinstance(choice.get("id"),str) and "Phenylketonuria (PKU)" in text(choice.get("body"))]
if len(matches) != 1: raise SystemExit("issued native presentation did not expose its canonical correct choice exactly once")
choice=matches[0]
print(json.dumps({"response":{"kind":"multipleChoice","selected":[choice["id"]]}},separators=(",",":")))
' "$(response_body "$1")"
}

repoint_payload() {
	python3 -c '
import json,sys
workspace=json.loads(sys.argv[1]); required={"title","instructions","dueAt","availableAt","closesAt","lateWorkRule","assignmentAttemptTimeLimitSeconds","attemptLimit","activityRules","studentFeedbackReleaseRule","entries"}
if not required.issubset(workspace) or len(workspace["entries"]) != 1: raise SystemExit("Assignment workspace is not a one-Question current-point edit")
payload={key:workspace[key] for key in required}; payload["entries"][0]["pointsPossible"]="2"
print(json.dumps(payload,separators=(",",":")))
' "$1"
}

prove_native_current_points() {
	local instructor mary course assignment reference started attempt selected saved submitted initial_score workspace edit updated history gradebook
	instructor="$(persona_cookie elenaInstructor)"; mary="$(persona_cookie maryStudent)"
	course="$(new_course_reference "$instructor")"
	claim_student_record "$course" "$instructor" "$mary" mary.okafor@biology.roosevelt.edu current-points-mary Mary
	reference="$(canonical_native_question_reference "$instructor")"
	assignment="$(create_released_backend_assignment "$course" "$instructor" ple "Native Current Points" 300 "$reference")"
	started="$(request "/api/course-instances/$course/assignments/$assignment/start" "$mary" POST '{}')"
	require_status "Native Assignment start" "$started" 201
	attempt="$(python3 -c 'import json,re,sys; value=json.loads(sys.argv[1]); attempt=value.get("assignmentAttempt"); assert isinstance(attempt,str) and re.fullmatch(r"R-[1-9][0-9]*",attempt); print(attempt)' "$(response_body "$started")")"
	selected="$(request "/api/assignment-attempts/$attempt/student-question?position=1" "$mary")"
	require_status "Native Question selection" "$selected" 200
	saved="$(request "/api/assignment-attempts/$attempt/responses/1" "$mary" PUT "$(native_response_payload "$selected")")"
	require_status "Native response save" "$saved" 200
	submitted="$(request "/api/assignment-attempts/$attempt/submission" "$mary" POST '{}')"
	require_status "Native submission" "$submitted" 200
	initial_score="$(assert_score "$submitted" "$attempt")"
	python3 -c 'import json,sys; score=json.loads(sys.argv[1]); raise SystemExit(0 if score["pointsEarned"] > 0 else "native current-point acceptance needs nonzero retained credit")' "$initial_score"
	workspace="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor")"
	require_status "Native current-points workspace" "$workspace" 200
	read -r _ edit < <(workspace_reference_and_edit "$(response_body "$workspace")")
	updated="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor" PUT "$(repoint_payload "$(response_body "$workspace")")" "$edit")"
	require_status "Native current-points edit" "$updated" 200
	history="$(request "/api/assignment-attempts/$attempt/history" "$mary")"
	require_status "Native submitted history" "$history" 200
	gradebook="$(request "/api/course-instances/$course/gradebook" "$instructor")"
	require_status "Native current-points Gradebook" "$gradebook" 200
	python3 -c '
import json,math,sys
initial=json.loads(sys.argv[1]); history=json.loads(sys.argv[2]); gradebook=json.loads(sys.argv[3]); assignment=sys.argv[4]
expected={"pointsEarned":initial["pointsEarned"]*2,"pointsPossible":2}
if history.get("score") is None or not all(math.isclose(history["score"][key],expected[key]) for key in expected):
    raise SystemExit("Student read did not derive current points from immutable native credit")
rows=gradebook.get("studentWork",[])
if not any(row.get("assignmentReference")==assignment and row.get("score") is not None and all(math.isclose(row["score"][key],expected[key]) for key in expected) for row in rows if isinstance(row,dict)):
    raise SystemExit("Gradebook read did not derive current points from immutable native credit")
' "$initial_score" "$(response_body "$history")" "$(response_body "$gradebook")" "$assignment"
	echo "Native Assignment Attempt: saved response submits immediately and current points reread from immutable credit passed"
}

prove_background_expiry() {
	local instructor mary course assignment reference started attempt selected saved history completed=0 postgres evidence
	instructor="$(persona_cookie elenaInstructor)"; mary="$(persona_cookie maryStudent)"
	course="$(new_course_reference "$instructor")"
	claim_student_record "$course" "$instructor" "$mary" mary.okafor@biology.roosevelt.edu expiry-worker-mary Mary
	reference="$(canonical_native_question_reference "$instructor")"
	assignment="$(create_released_backend_assignment "$course" "$instructor" ple "Background Expiry" 300 "$reference")"
	started="$(request "/api/course-instances/$course/assignments/$assignment/start" "$mary" POST '{}')"
	require_status "Expiry Assignment start" "$started" 201
	attempt="$(python3 -c 'import json,re,sys; value=json.loads(sys.argv[1]); attempt=value.get("assignmentAttempt"); assert isinstance(attempt,str) and re.fullmatch(r"R-[1-9][0-9]*",attempt); print(attempt)' "$(response_body "$started")")"
	selected="$(request "/api/assignment-attempts/$attempt/student-question?position=1" "$mary")"
	require_status "Expiry native Question selection" "$selected" 200
	saved="$(request "/api/assignment-attempts/$attempt/responses/1" "$mary" PUT "$(native_response_payload "$selected")")"
	require_status "Expiry response save" "$saved" 200
	postgres="$(service_id postgres)"
	evidence="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "BEGIN; SET LOCAL ROLE ple_private_owner; ALTER TABLE ple_private.assignment_attempt DISABLE TRIGGER assignment_attempt_retains_evidence; UPDATE ple_private.assignment_attempt SET expires_at = clock_timestamp() + interval '2 seconds' WHERE reference_number=${attempt#R-} AND completed_at IS NULL; ALTER TABLE ple_private.assignment_attempt ENABLE TRIGGER assignment_attempt_retains_evidence; COMMIT;")"
	printf '%s\n' "$evidence" | rg -qx 'UPDATE 1' || { echo "expiry test seam did not arm exactly one saved Attempt" >&2; exit 1; }
	# Do not make another Student interaction after the timer expires.  The
	# fixed generic worker owns abandoned Attempt finalization; wait only on
	# its private immutable evidence, then make one public result read.
	for _ in $(seq 1 18); do
		sleep 5
		evidence="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "SELECT CASE WHEN EXISTS (SELECT 1 FROM ple_private.assignment_submission AS submission JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=submission.assignment_attempt_id WHERE attempt.reference_number=${attempt#R-}) AND EXISTS (SELECT 1 FROM ple_private.grading_result AS result JOIN ple_private.question_attempt AS question_attempt ON question_attempt.question_attempt_id=result.question_attempt_id JOIN ple_private.issued_question AS issued ON issued.issued_question_id=question_attempt.issued_question_id JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id=issued.assignment_attempt_id WHERE attempt.reference_number=${attempt#R-}) THEN 'worker_finalized' ELSE 'waiting' END")"
		if [ "$evidence" = worker_finalized ]; then
			completed=1
			break
		fi
	done
	[ "$completed" = 1 ] || { echo "generic worker did not finalize the expired saved Assignment Attempt" >&2; exit 1; }
	history="$(request "/api/assignment-attempts/$attempt/history" "$mary")"
	require_status "Expired Attempt result read" "$history" 200
	python3 -c 'import json,sys; value=json.loads(sys.argv[1]); raise SystemExit(0 if value.get("state")=="submitted" and value.get("score") is not None else "expired worker result is not publicly readable")' "$(response_body "$history")"
	echo "Expired Assignment Attempt: generic worker finalized saved work with no open browser passed"
}

prove_webwork_submission() {
	local instructor mary course assignment started attempt port selected manifest failed submitted score gradebook
	instructor="$(persona_cookie elenaInstructor)"; mary="$(persona_cookie maryStudent)"
	# The existing start journey creates the ordinary Course Instance and claims
	# Mary.  This adds one ordinary released Assignment whose source is selected
	# by backend through the public Question Library rather than a test fixture.
	bash "$repository_root/tests/e2e/e2e_live_demo_assignment_attempt.sh" --start >/dev/null
	course="$(new_course_reference "$instructor")"
	claim_student_record "$course" "$instructor" "$mary" mary.okafor@biology.roosevelt.edu submission-journey-mary Mary
	assignment="$(create_released_backend_assignment "$course" "$instructor" webwork "WeBWorK Submission")"
	started="$(request "/api/course-instances/$course/assignments/$assignment/start" "$mary" POST '{}')"
	require_status "WeBWorK Assignment start" "$started" 201
	attempt="$(python3 -c 'import json,re,sys; value=json.loads(sys.argv[1]); attempt=value.get("assignmentAttempt"); assert isinstance(attempt,str) and re.fullmatch(r"R-[1-9][0-9]*",attempt); print(attempt)' "$(response_body "$started")")"
	port="$(gateway_port)"
	NODE_EXTRA_CA_CERTS="$repository_root/local_stack_state/live_demo_browser/workspace/gateway-root.crt" \
		node tests/playwright/e2e_live_demo_webwork_submission_browser.mjs "$port" "$attempt"
	selected="$(request "/api/assignment-attempts/$attempt/student-question?position=1" "$mary")"
	require_status "WeBWorK saved-response reload" "$selected" 200
	assert_saved_backend_response "$selected"

	manifest="local_stack_state/live_demo_browser/workspace/disposable.manifest"
	[ -f "$manifest" ] || { echo "WeBWorK outage acceptance requires the controller-owned manifest" >&2; exit 2; }
	python3 -m local_stack_control.disposable_stack_command stop-webwork-renderer --manifest "$manifest" >/dev/null
	failed="$(request "/api/assignment-attempts/$attempt/submission" "$mary" POST '{}')"
	[ "$(response_status "$failed")" = 503 ] || { echo "renderer outage did not fail Submit plainly" >&2; exit 1; }
	printf '%s' "$(response_body "$failed")" | rg -qi 'try again|unavailable' || { echo "renderer outage did not explain retry" >&2; exit 1; }
	selected="$(request "/api/assignment-attempts/$attempt/student-question?position=1" "$mary")"
	require_status "WeBWorK saved-response after outage" "$selected" 200
	assert_saved_backend_response "$selected"
	python3 -m local_stack_control.disposable_stack_command replace-webwork-renderer --manifest "$manifest" >/dev/null
	submitted="$(request "/api/assignment-attempts/$attempt/submission" "$mary" POST '{}')"
	require_status "WeBWorK Submission" "$submitted" 200
	score="$(assert_score "$submitted" "$attempt")"
	assert_renderer_fraction_matches_submission "$attempt" "$score"
	gradebook="$(request "/api/course-instances/$course/gradebook" "$instructor")"
	require_status "Instructor Gradebook scored read" "$gradebook" 200
	python3 -c '
import json,sys
gradebook=json.loads(sys.argv[1]); assignment=sys.argv[2]; score=json.loads(sys.argv[3])
rows=gradebook.get("studentWork",[])
if not any(row.get("assignmentReference")==assignment and row.get("score")==score for row in rows if isinstance(row,dict)):
    raise SystemExit("Instructor Gradebook did not read the submitted score")
' "$(response_body "$gradebook")" "$assignment" "$score"
	echo "WeBWorK Assignment Attempt: browser-owned opaque save/reload, outage retry, immediate score, and Gradebook score passed"
}

require_live_demo
case "$mode" in
	start) prove_start ;;
	all) prove_start; prove_native_current_points; prove_background_expiry; prove_webwork_submission ;;
esac
echo "Live Demo Assignment Attempt: PASS"
