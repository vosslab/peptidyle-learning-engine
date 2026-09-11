#!/usr/bin/env bash
# Disposable acceptance: complete used-Course baseline, boundaries, and recovery.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly workspace="local_stack_state/live_demo_browser/workspace"
readonly runtime_environment_path="$workspace/env.local"
readonly manifest_path="$workspace/disposable.manifest"
readonly report_path="$workspace/live_demo_course_report.json"

usage() {
	echo "Usage: bash $0 [--state|--authorization|--converge|--recover]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--state) mode="state" ;;
	--authorization) mode="authorization" ;;
	--converge) mode="converge" ;;
	--recover) mode="recover" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

fail() {
	echo "Live Demo Course seed: $*" >&2
	exit 1
}

require_live_demo() {
	[ -f "$runtime_environment_path" ] || fail "fixed Live Demo environment is unavailable"
	[ -f "$manifest_path" ] || fail "fixed Live Demo manifest is unavailable"
	[ -f "$report_path" ] || fail "Course baseline report is unavailable"
}

service_id() {
	local service="$1" identifiers
	identifiers="$(podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=$service")"
	[ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = "1" ] ||
		fail "expected one running $service service"
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' \
		"$runtime_environment_path" || true)"
	[ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = "1" ] ||
		fail "expected one validated gateway port"
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1" cookie="${2:-}" method="${3:-GET}" body="${4:-}"
	local gateway port
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	local -a curl_args=(
		--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}'
		--header "Host: localhost:$port" --request "$method"
	)
	if [ "$method" != "GET" ]; then
		curl_args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
	fi
	[ -z "$cookie" ] || curl_args+=(--header "Cookie: $cookie")
	[ -z "$body" ] || curl_args+=(--data "$body")
	podman exec "$gateway" curl "${curl_args[@]}" "https://localhost:8080$path"
}

persona_cookie() {
	local persona="$1" gateway port headers cookie
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 \
		--dump-header - --output /dev/null --header "Host: localhost:$port" \
		--header "Origin: https://localhost:$port" --header 'Content-Type: application/json' \
		--request POST --data "{\"persona\":\"$persona\"}" \
		'https://localhost:8080/api/auth/live-demo/accounts')"
	cookie="$(printf '%s\n' "$headers" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"
	[ -n "$cookie" ] || fail "seeded demo did not issue an Authenticated Session"
	printf '%s\n' "$cookie"
}

response_status() { printf '%s' "${1##*$'\n'}"; }
response_body() { printf '%s' "${1%$'\n'*}"; }

require_status() {
	local label="$1" response="$2" expected="$3"
	[ "$(response_status "$response")" = "$expected" ] ||
		fail "$label returned HTTP $(response_status "$response"), not $expected"
}

assert_concealed() {
	require_status "$1" "$2" 404
}

baseline_references() {
	python3 -c '
import json, re, sys
value=json.load(open(sys.argv[1],encoding="ascii"))
fields=(("blueprint_reference",r"BP-[1-9][0-9]{0,9}"),("course_reference",r"C-[1-9][0-9]{0,9}"),("assignment_reference",r"A-[1-9][0-9]{0,9}"))
references=[]
for field,pattern in fields:
    reference=value.get(field)
    if not isinstance(reference,str) or re.fullmatch(pattern,reference) is None:
        raise SystemExit("Course baseline report lacks exact public references")
    references.append(reference)
print(*references)
' "$report_path"
}

assert_complete_report() {
	python3 -c '
import json, stat, sys
path=sys.argv[1]
value=json.load(open(path,encoding="ascii"))
if stat.S_IMODE(__import__("os").stat(path).st_mode) != 0o600:
    raise SystemExit("Course baseline report is not private")
if value.get("stages_still_outstanding") != []:
    raise SystemExit("Course baseline report retains outstanding stages")
students=value.get("students")
if not isinstance(students,list) or len(students) != 3:
    raise SystemExit("Course baseline report lacks the three Students")
observed={student.get("persona"):student for student in students if isinstance(student,dict)}
expected={
 "maryStudent":("BIO301-MARY",1,4,0,4,2.0,4.0),
 "jackStudent":("BIO301-JACK",0,0,2,0,0.0,0.0),
 "averyStudent":("BIO301-AVERY",0,0,0,0,0.0,0.0),
}
if set(observed) != set(expected):
    raise SystemExit("Course baseline report contains the wrong Students")
for persona,(roster,assignment_submissions,graded_questions,saved,graded,earned,possible) in expected.items():
    row=observed[persona]
    attempt=row.get("assignment_attempt")
    expected_attempt={
        "maryStudent":{"attempt_number":1,"state":"completed"},
        "jackStudent":{"attempt_number":1,"state":"open"},
        "averyStudent":None,
    }[persona]
    if (row.get("roster_id")!=roster or row.get("membership")!="activeStudent"
        or attempt!=expected_attempt
        or row.get("assignment_submission_count")!=assignment_submissions
        or row.get("graded_question_count")!=graded_questions
        or row.get("saved_response_count")!=saved
        or row.get("grading_state")!={"graded_question_count":graded,"pending":0,"instructorAttention":0}
        or row.get("points_earned")!=earned or row.get("points_possible")!=possible):
        raise SystemExit("Course baseline report does not match the declared Student state")
' "$report_path"
}

assert_product_state() {
	local blueprint_reference="$1" course_reference="$2" assignment_reference="$3"
	local elena_cookie mary_cookie jack_cookie avery_cookie
	local courses course assignments roster gradebook mary_landing jack_landing avery_landing access
	elena_cookie="$(persona_cookie elenaInstructor)"
	mary_cookie="$(persona_cookie maryStudent)"
	jack_cookie="$(persona_cookie jackStudent)"
	avery_cookie="$(persona_cookie averyStudent)"
	courses="$(request '/api/course-instances' "$elena_cookie")"
	course="$(request "/api/course-instances/$course_reference" "$elena_cookie")"
	assignments="$(request "/api/course-instances/$course_reference/assignments" "$elena_cookie")"
	roster="$(request "/api/course-instances/$course_reference/roster" "$elena_cookie")"
	gradebook="$(request "/api/course-instances/$course_reference/gradebook" "$elena_cookie")"
	mary_landing="$(request "/api/course-instances/$course_reference/assignment-landing" "$mary_cookie")"
	jack_landing="$(request "/api/course-instances/$course_reference/assignment-landing" "$jack_cookie")"
	avery_landing="$(request "/api/course-instances/$course_reference/assignment-landing" "$avery_cookie")"
	access="$(request "/api/course-instances/$course_reference/assignments/$assignment_reference/access" \
		"$avery_cookie")"
	require_status "Elena Course list" "$courses" 200
	require_status "Elena Course workspace" "$course" 200
	require_status "Elena Assignment list" "$assignments" 200
	require_status "Elena Course Roster" "$roster" 200
	require_status "Elena Gradebook" "$gradebook" 200
	require_status "Mary Assignment landing" "$mary_landing" 200
	require_status "Jack Assignment landing" "$jack_landing" 200
	require_status "Avery Assignment landing" "$avery_landing" 200
	require_status "Avery Assignment Access" "$access" 200
	python3 -c '
import json, sys
courses,course,assignments,roster,gradebook,mary_landing,jack_landing,avery_landing,access=map(json.loads,sys.argv[1:10])
blueprint_reference,course_reference,assignment_reference=sys.argv[10:13]
short_name="BCHM 301"
long_name="Biochemistry 301: Proteins and Peptides"
matching=[item for item in courses.get("items",[]) if isinstance(item,dict) and item.get("shortName")==short_name and item.get("longName")==long_name]
if len(matching)!=1 or matching[0].get("reference")!=course_reference:
    raise SystemExit("Elena does not see exactly one fixed Course Instance")
if course.get("course")!=matching[0] or course.get("isAssignedInstructor") is not True or course.get("activeInstructorCount")!=1:
    raise SystemExit("Elena Course workspace does not retain exact direct authority")
expected_assignment={"reference":assignment_reference,"title":"Peptide Structure Practice","status":"released","editNumber":"2"}
if assignments != [expected_assignment]:
    raise SystemExit("Elena does not see the exact released Assignment")
expected_roster={
 "BIO301-MARY":("mary.okafor@live-demo.invalid","activeStudent"),
 "BIO301-JACK":("jack.nguyen@live-demo.invalid","activeStudent"),
 "BIO301-AVERY":("avery.thompson@live-demo.invalid","activeStudent"),
}
observed={item.get("rosterId"):(item.get("rosterEmail"),item.get("state")) for item in roster if isinstance(item,dict)}
if observed != expected_roster:
    raise SystemExit("Elena Course Roster does not contain the exact active Students")
expected_grades={
 ("BIO301-MARY",assignment_reference):("completed",4,4,2.0,4.0),
 ("BIO301-JACK",assignment_reference):("inProgress",0,4,0.0,0.0),
 ("BIO301-AVERY",assignment_reference):(None,0,4,0.0,0.0),
}
rows=gradebook.get("studentWork") if gradebook.get("courseReference")==course_reference else None
observed_grades={(row.get("rosterId"),row.get("assignmentReference")):(row.get("assignmentAttemptCompletion"),row.get("gradedQuestionCount"),row.get("questionCount"),row.get("pointsEarned"),row.get("pointsPossible")) for row in rows or [] if isinstance(row,dict)}
if observed_grades != expected_grades:
    raise SystemExit("Elena Gradebook does not distinguish Mary, Jack, and Avery")
expected_student_progress={
 "mary":(1,"completed",4,4,{"pointsEarned":2.0,"pointsPossible":4.0}),
 "jack":(1,"inProgress",0,4,None),
 "avery":(None,None,0,4,None),
}
for persona,landing in (("mary",mary_landing),("jack",jack_landing),("avery",avery_landing)):
    rows=landing.get("assignments") if isinstance(landing,dict) else None
    if not isinstance(rows,list) or len(rows)!=1:
        raise SystemExit(f"{persona.title()} Assignment landing is not exact")
    row=rows[0]
    progress=(row.get("assignmentAttemptNumber"),row.get("assignmentAttemptCompletion"),row.get("gradedQuestionCount"),row.get("questionCount"),row.get("score"))
    if row.get("reference")!=assignment_reference or row.get("title")!="Peptide Structure Practice" or progress!=expected_student_progress[persona]:
        raise SystemExit(f"{persona.title()} Assignment landing progress is wrong")
if access != {"startDecision":"may_start", "activeAssignmentAttempt":None}:
    raise SystemExit("Avery is not startable with no Assignment Attempt")
if blueprint_reference == "":
    raise SystemExit("baseline Blueprint Reference is absent")
	' "$(response_body "$courses")" "$(response_body "$course")" \
		"$(response_body "$assignments")" "$(response_body "$roster")" \
		"$(response_body "$gradebook")" "$(response_body "$mary_landing")" \
		"$(response_body "$jack_landing")" "$(response_body "$avery_landing")" \
		"$(response_body "$access")" \
		"$blueprint_reference" "$course_reference" "$assignment_reference"
}

assert_database_state() {
	local blueprint_reference="$1" course_reference="$2" assignment_reference="$3"
	local postgres sql output
	postgres="$(service_id postgres)"
	sql="DO \$\$
DECLARE
    v_course_id uuid;
    v_assignment_id uuid;
    v_revision_id uuid;
    v_mary_record uuid;
    v_jack_record uuid;
    v_avery_record uuid;
BEGIN
    SELECT course.course_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number=${course_reference#C-}
       AND course.course_short_name='BCHM 301'
       AND course.course_long_name='Biochemistry 301: Proteins and Peptides'
       AND course.blueprint_course_reference_number=${blueprint_reference#BP-};
    SELECT assignment.assignment_id, assignment.released_assignment_revision_id
      INTO v_assignment_id, v_revision_id
      FROM ple_data.assignment AS assignment
     WHERE assignment.course_id=v_course_id
       AND assignment.reference_number=${assignment_reference#A-}
       AND assignment.assignment_status='released';
    SELECT record.student_record_id INTO v_mary_record
      FROM ple_private.course_roster_profile AS profile
      JOIN ple_data.student_record AS record
        ON record.course_id=profile.course_id
       AND record.student_account_id=profile.student_account_id
     WHERE profile.course_id=v_course_id AND profile.roster_id='BIO301-MARY';
    SELECT record.student_record_id INTO v_jack_record
      FROM ple_private.course_roster_profile AS profile
      JOIN ple_data.student_record AS record
        ON record.course_id=profile.course_id
       AND record.student_account_id=profile.student_account_id
     WHERE profile.course_id=v_course_id AND profile.roster_id='BIO301-JACK';
    SELECT record.student_record_id INTO v_avery_record
      FROM ple_private.course_roster_profile AS profile
      JOIN ple_data.student_record AS record
        ON record.course_id=profile.course_id
       AND record.student_account_id=profile.student_account_id
     WHERE profile.course_id=v_course_id AND profile.roster_id='BIO301-AVERY';
    IF v_course_id IS NULL OR v_assignment_id IS NULL OR v_revision_id IS NULL
       OR v_mary_record IS NULL OR v_jack_record IS NULL OR v_avery_record IS NULL
       OR (SELECT count(*) FROM ple_data.course_instance
            WHERE course_short_name='BCHM 301'
              AND course_long_name='Biochemistry 301: Proteins and Peptides') <> 1
       OR (SELECT array_agg(fixed.question_id ORDER BY entry.assignment_content_entry_index)
             FROM ple_data.assignment_revision_fixed_question AS fixed
             JOIN ple_data.assignment_revision_entry AS entry
               ON entry.assignment_revision_id=fixed.assignment_revision_id
              AND entry.assignment_entry_id=fixed.assignment_entry_id
            WHERE fixed.assignment_revision_id=v_revision_id)
          IS DISTINCT FROM ARRAY['PNE-0001','PNE-0002','PNE-0003','PNE-0004']::text[]
       OR (SELECT count(*) FROM ple_data.course_membership AS membership
            WHERE membership.course_id=v_course_id AND membership.role='student'
              AND membership.student_record_id IN (v_mary_record,v_jack_record,v_avery_record)
              AND ple_data.course_membership_is_active(membership.membership_id)) <> 3
       OR (SELECT count(*) FROM ple_private.assignment_attempt AS attempt
            WHERE attempt.assignment_id=v_assignment_id
              AND attempt.student_record_id=v_mary_record
              AND attempt.completed_at IS NOT NULL) <> 1
       OR (SELECT count(*) FROM ple_private.assignment_attempt AS attempt
            WHERE attempt.assignment_id=v_assignment_id
              AND attempt.student_record_id=v_jack_record
              AND attempt.completed_at IS NULL) <> 1
       OR EXISTS (SELECT 1 FROM ple_private.assignment_attempt AS attempt
                   WHERE attempt.assignment_id=v_assignment_id
                     AND attempt.student_record_id=v_avery_record)
       OR (SELECT count(*) FROM ple_private.question_submission AS submission
             JOIN ple_private.question_attempt AS question_attempt
               ON question_attempt.question_attempt_id=submission.question_attempt_id
              AND question_attempt.question_attempt_state='submission_accepted'
             JOIN ple_private.issued_question AS issued
               ON issued.issued_question_id=question_attempt.issued_question_id
             JOIN ple_private.assignment_attempt AS attempt
               ON attempt.assignment_attempt_id=issued.assignment_attempt_id
             JOIN ple_private.question_submission_grading AS grading
               ON grading.submission_id=submission.submission_id
              AND grading.grading_state='graded' AND grading.completed_at IS NOT NULL
             JOIN ple_private.grading_result AS result
               ON result.submission_id=submission.submission_id
            WHERE attempt.assignment_id=v_assignment_id
              AND attempt.student_record_id=v_mary_record) <> 4
       OR EXISTS (SELECT 1 FROM ple_private.question_submission AS submission
             JOIN ple_private.question_attempt AS question_attempt
               ON question_attempt.question_attempt_id=submission.question_attempt_id
              AND question_attempt.question_attempt_state='submission_accepted'
             JOIN ple_private.issued_question AS issued
               ON issued.issued_question_id=question_attempt.issued_question_id
             JOIN ple_private.assignment_attempt AS attempt
               ON attempt.assignment_attempt_id=issued.assignment_attempt_id
             JOIN ple_private.question_submission_grading AS grading
               ON grading.submission_id=submission.submission_id
              AND grading.grading_state='graded' AND grading.completed_at IS NOT NULL
             JOIN ple_private.grading_result AS result
               ON result.submission_id=submission.submission_id
            WHERE attempt.assignment_id=v_assignment_id
              AND attempt.student_record_id=v_jack_record)
       OR (SELECT count(*) FROM ple_private.assignment_attempt_saved_response AS saved_response
             JOIN ple_private.question_attempt AS question_attempt
               ON question_attempt.question_attempt_id=saved_response.question_attempt_id
             JOIN ple_private.issued_question AS issued
               ON issued.issued_question_id=question_attempt.issued_question_id
             JOIN ple_private.assignment_attempt AS attempt
               ON attempt.assignment_attempt_id=issued.assignment_attempt_id
            WHERE attempt.assignment_id=v_assignment_id
              AND attempt.student_record_id=v_jack_record) <> 2
    THEN
        RAISE EXCEPTION USING ERRCODE='42501',
            MESSAGE='complete Live Demo Course seed evidence is invalid';
    END IF;
END
\$\$;
SELECT 'live_demo_course_seed_state';"
	output="$(podman exec "$postgres" sh -lc \
		'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' \
		sh "$sql")"
	[ "$(printf '%s\n' "$output" | sed -n '/^live_demo_course_seed_state$/p')" = \
		"live_demo_course_seed_state" ] || fail "database invariants were not established"
}

prove_state() {
	local blueprint_reference course_reference assignment_reference
	require_live_demo
	read -r blueprint_reference course_reference assignment_reference < <(baseline_references)
	assert_complete_report
	assert_product_state "$blueprint_reference" "$course_reference" "$assignment_reference"
	assert_database_state "$blueprint_reference" "$course_reference" "$assignment_reference"
	echo "Live Demo Course state: one released four-Question Assignment; Mary graded, Jack in progress, Avery not started"
}

active_attempt_reference() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); attempt=value.get("activeAssignmentAttempt")
if set(value)!={"startDecision","activeAssignmentAttempt"} or not isinstance(attempt,str) or not re.fullmatch(r"R-[1-9][0-9]{0,9}",attempt):
    raise SystemExit("Jack Assignment Access lacks an active public Assignment Attempt")
print(attempt)
' "$1"
}

prove_authorization() {
	local _blueprint course assignment mary_cookie jack_cookie morgan_cookie
	local roster_path assignment_list_path gradebook_path workspace_path jack_access attempt other_work
	require_live_demo
	read -r _blueprint course assignment < <(baseline_references)
	mary_cookie="$(persona_cookie maryStudent)"
	jack_cookie="$(persona_cookie jackStudent)"
	morgan_cookie="$(persona_cookie morganSysadmin)"
	roster_path="/api/course-instances/$course/roster"
	assignment_list_path="/api/course-instances/$course/assignments"
	gradebook_path="/api/course-instances/$course/gradebook"
	workspace_path="/api/course-instances/$course/assignments/$assignment"
	for path in "$roster_path" "$assignment_list_path" "$gradebook_path" "$workspace_path"; do
		assert_concealed "anonymous seeded-Course access" "$(request "$path")"
		assert_concealed "Student seeded-Course access" "$(request "$path" "$mary_cookie")"
		assert_concealed "Morgan seeded-Course access" "$(request "$path" "$morgan_cookie")"
	done
	jack_access="$(request "/api/course-instances/$course/assignments/$assignment/access" "$jack_cookie")"
	require_status "Jack Assignment Access" "$jack_access" 200
	attempt="$(active_attempt_reference "$(response_body "$jack_access")")"
	other_work="$(request "/api/assignment-attempts/$attempt/student-question?position=1" "$mary_cookie")"
	assert_concealed "cross-Student work access" "$other_work"
	echo "Live Demo Course authorization: Student, Sysadmin, cross-Student, and anonymous FERPA boundaries concealed"
}

provision_course() {
	python3 -m local_stack_control.disposable_stack_command provision-course \
		--manifest "$manifest_path" "$@"
}

prove_convergence() {
	local planned
	require_live_demo
	provision_course >/dev/null
	planned="$(provision_course --report)"
	[ "$planned" = "[]" ] || fail "converged baseline still plans $planned"
	prove_state
	echo "Live Demo Course convergence: repeat provisioning created no duplicate baseline objects"
}

assert_checkpoint() {
	local stage="$1"
	python3 -c '
import json, sys
stages=["blueprint","course","roster","claims","assignment","selection","release","attempts","work"]
value=json.load(open(sys.argv[1],encoding="ascii")); stopped=sys.argv[2]
expected=stages[stages.index(stopped)+1:]
if value.get("stages_still_outstanding") != expected:
    raise SystemExit("provisioning checkpoint did not stop at the requested stage")
' "$report_path" "$stage"
}

prove_recovery() {
	local stage
	local -a stages=(blueprint course roster claims assignment selection release attempts work)
	for stage in "${stages[@]}"; do
		echo "Live Demo Course recovery: manufacturing interruption after $stage"
		python3 local_stack.py start --headless --stop-after "$stage" >/dev/null
		require_live_demo
		assert_checkpoint "$stage"
		provision_course >/dev/null
		prove_state
	done
	echo "Live Demo Course recovery: every provisioning boundary resumed to the exact complete baseline"
}

case "$mode" in
	state) prove_state ;;
	authorization) prove_authorization ;;
	converge) prove_convergence ;;
	recover) prove_recovery ;;
	all)
		prove_state
		prove_authorization
		prove_convergence
		prove_recovery
		;;
esac

echo "Live Demo Course seed: PASS"
