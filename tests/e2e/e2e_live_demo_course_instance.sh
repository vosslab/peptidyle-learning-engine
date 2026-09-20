#!/usr/bin/env bash
# Disposable acceptance: exact Blueprint source, Assigned Instructor, and teaching-team browser entry.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"
readonly course_short_name="BIOL 351/451-20"
readonly course_long_name="Genetics"
readonly second_course_short_name="BIOL 480"
readonly second_course_long_name="Biotechnology"
readonly current_course_short_name="BIOL 318/418-20"
readonly current_course_long_name="Biostatistics"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_course_instance.sh [--authority|--browser]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--authority) mode="authority" ;;
	--browser) mode="browser" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"
readonly run_id="$(date +%s%N)"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Course Instance evidence requires the fixed Live Demo to be running" >&2
		exit 2
	fi
}

service_id() {
	local service="$1"
	local identifiers
	identifiers="$(podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=$service")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Course Instance evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Course Instance evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1"
	local cookie="${2:-}"
	local method="${3:-GET}"
	local body="${4:-}"
	local if_match="${5:-}"
	local idempotency_key="${6:-}"
	local gateway port
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	local -a curl_args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}'
		--header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then
		curl_args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
	fi
	if [ -n "$cookie" ]; then curl_args+=(--header "Cookie: $cookie"); fi
	if [ -n "$if_match" ]; then curl_args+=(--header "If-Match: $if_match"); fi
	if [ -n "$idempotency_key" ]; then curl_args+=(--header "Idempotency-Key: $idempotency_key"); fi
	if [ -n "$body" ]; then curl_args+=(--data "$body"); fi
	podman exec "$gateway" curl "${curl_args[@]}" "https://localhost:8080$path"
}

persona_cookie() {
	local persona="$1"
	local gateway port headers cookie
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	if [ "$persona" = morganSysadmin ]; then
		local setup_file="${PLE_LOCAL_DEMO_TOTP_SETUP_FILE:-$repository_root/local_stack_state/live_demo_browser/workspace/morgan-totp-setup-uri}"
		(
			set -e
			ca_file="$(mktemp "${TMPDIR:-/tmp}/ple-morgan-ca.XXXXXX")"
			trap 'rm -f -- "$ca_file"' EXIT
			# Public CA only; no global trust changes or credential output.
			podman exec "$gateway" cat /data/caddy/pki/authorities/local/root.crt > "$ca_file"
			python3 tests/e2e/e2e_live_demo_session.py "$port" "$setup_file" --ca-file "$ca_file"
		)
		return
	fi
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
		echo "Course Instance access was not concealed" >&2
		exit 1
	fi
}

first_published_question_id() {
	python3 -c '
import json, re, sys
items = json.loads(sys.argv[1]).get("items")
if not isinstance(items, list) or not items:
    raise SystemExit("Question Library did not return a published Question")
question_id = items[0].get("summary", {}).get("questionId")
if not isinstance(question_id, str) or not re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}", question_id):
    raise SystemExit("Question Library did not return an opaque Question ID")
revision = items[0].get("summary", {}).get("questionRevisionTuple")
if (not isinstance(revision, dict) or set(revision) != {"questionId", "revisionNumber"}
    or revision["questionId"] != question_id
    or not isinstance(revision["revisionNumber"], int)
    or isinstance(revision["revisionNumber"], bool) or revision["revisionNumber"] < 1):
    raise SystemExit("Question Library did not return an exact published Question Revision")
print(json.dumps(revision, separators=(",",":")))
' "$1"
}

blueprint_payload() {
	python3 -c '
import json, sys
published_question = json.loads(sys.argv[1])
classification = json.loads(sys.argv[2])
assignment = {
  "assessment_type": "regular_assignment",
  "title": "Course Instance source assignment",
  "instructions": "Use the published Question in reusable course structure.",
  "entries": [{"kind":"fixed","published_question":published_question,"points_possible":"1","scoring_rule":"normal","question_attempt_limit":{"maxAttempts":2},"question_attempt_time_limit":{"kind":"unlimited"}}],
  "defaults": {"assessment_attempt_time_limit_seconds":None,"assessment_attempt_limit":2,"late_work_rule":"accept","activity_rules":{"questionVariationRule":"newVariation","assessmentQuestionOrderRule":"authoredOrder"},"student_feedback_release_rule":{"score":"after_submit","submitted_response":"after_submit","per_item_correctness":"after_submit","question_answer":"never","question_answer_explanation":"never","class_statistics":"never"}},
}
print(json.dumps({"classification":classification,"short_name":"M8 source","long_name":"M8 exact Blueprint source","modules":[{"label":"M8 module","assessments":[assignment]}]}, separators=(",",":")))
' "$1" "$2"
}

replacement_payload() {
	python3 -c '
import json, sys
course = json.loads(sys.argv[1])
modules = course.get("modules")
if not isinstance(modules, list) or len(modules) != 1:
    raise SystemExit("Blueprint Revision 1 did not return one reusable module")
module = modules[0]
assignments = module.get("assessments") if isinstance(module, dict) else None
if not isinstance(assignments, list) or len(assignments) != 1:
    raise SystemExit("Blueprint Revision 1 did not return one reusable assignment")
assignment = assignments[0]
blueprint_module_id = module.get("blueprint_module_id")
blueprint_assessment_id = assignment.get("blueprint_assessment_id") if isinstance(assignment, dict) else None
content = assignment.get("content") if isinstance(assignment, dict) else None
if not isinstance(blueprint_module_id, str) or not isinstance(blueprint_assessment_id, str) or not isinstance(content, dict):
    raise SystemExit("Blueprint Revision 1 did not return stable reusable identities")
entry = content.get("entries", [None])[0]
question_revision = entry.get("question", {}).get("question_revision") if isinstance(entry, dict) else None
if not isinstance(question_revision, dict) or set(question_revision) != {"questionId", "revisionNumber"}:
    raise SystemExit("Blueprint Revision 1 did not return its reusable Question")
replacement = {"modules":[{"choice":{"kind":"retained","blueprint_module_id":blueprint_module_id},"label":module.get("label"),"assessments":[{"choice":{"kind":"retained","blueprint_assessment_id":blueprint_assessment_id},"content":{"assessment_type":content.get("assessment_type"),"title":content.get("title") + " revised","instructions":content.get("instructions"),"entries":[{"kind":"fixed","published_question":question_revision,"points_possible":entry.get("points_possible"),"scoring_rule":entry.get("scoring_rule"),"question_attempt_limit":entry.get("question_attempt_limit"),"question_attempt_time_limit":entry.get("question_attempt_time_limit")}],"defaults":content.get("defaults")}}]}]}
print(json.dumps(replacement, separators=(",",":")))
' "$1"
}

course_payload() {
	python3 -c '
import json, sys
blueprint, revision, assigned, classification, short_name, long_name = sys.argv[1:]
print(json.dumps({"classification":json.loads(classification),"source":{"kind":"adopted","blueprintCourse":blueprint,"blueprintRevisionNumber":revision},"shortName":short_name,"longName":long_name,"term":{"startDate":"2026-09-01","endDate":"2026-12-18"},"assignedInstructor":assigned}, separators=(",",":")))
' "$1" "$2" "$3" "$4" "$5" "$6"
}

assert_course_receipt() {
	python3 -c '
import json, sys
value = json.loads(sys.argv[1])
if set(value) != {"course"}:
    raise SystemExit("Course Instance creation receipt was not closed")
course = value["course"]
themes = {"tundra", "forest", "desert", "grass", "arctic", "ocean", "tropical", "coral-reef", "swamp", "underground", "salt-marsh", "wetland", "sea-floor", "magma", "beach"}
if (set(course) != {"classification", "lifecycleState", "courseEditNumber", "id", "shortName", "longName", "term", "theme"}
    or not isinstance(course["id"], str) or not course["id"]
    or course["classification"] != json.loads(sys.argv[4])
    or not isinstance(course["courseEditNumber"], str) or not course["courseEditNumber"]
    or course["theme"] not in themes):
    raise SystemExit("Course Instance creation receipt did not return a public Course Instance identity")
if course["shortName"] != sys.argv[2] or course["longName"] != sys.argv[3]:
    raise SystemExit("Sysadmin Course Instance creation did not preserve its Course identity receipt")
print(course["id"])
' "$1" "$2" "$3" "$4"
}

assert_instructor_view() {
	python3 -c '
import json, sys
value = json.loads(sys.argv[1])
if set(value) != {"course", "activeInstructorCount", "blueprintOrigin"}:
    raise SystemExit("Course Instance teaching-team view was not closed")
course = value["course"]
themes = {"tundra", "forest", "desert", "grass", "arctic", "ocean", "tropical", "coral-reef", "swamp", "underground", "salt-marsh", "wetland", "sea-floor", "magma", "beach"}
if (set(course) != {"classification", "lifecycleState", "courseEditNumber", "id", "shortName", "longName", "term", "theme"}
    or not isinstance(course["id"], str) or not course["id"]
    or course["id"] != sys.argv[2]
    or course["classification"] != json.loads(sys.argv[5])
    or not isinstance(course["courseEditNumber"], str) or not course["courseEditNumber"]
    or course["theme"] not in themes):
    raise SystemExit("Course Instance teaching-team view identity differs")
if course["shortName"] != sys.argv[3] or course["longName"] != sys.argv[4]:
    raise SystemExit("Course Instance teaching-team view did not retain both names")
if not isinstance(value["activeInstructorCount"], int) or value["activeInstructorCount"] < 1:
    raise SystemExit("Course Instance teaching-team view lacks active Instructor evidence")
origin = value["blueprintOrigin"]
if origin is not None and (
    not isinstance(origin, dict)
    or set(origin) != {"id", "adoptedRevisionNumber", "currentRevisionNumber"}
    or not isinstance(origin["id"], str) or not origin["id"]
    or not isinstance(origin["adoptedRevisionNumber"], str)
    or not isinstance(origin["currentRevisionNumber"], str)
    or not origin["adoptedRevisionNumber"].isdigit()
    or not origin["currentRevisionNumber"].isdigit()
    or not 0 < int(origin["adoptedRevisionNumber"]) <= int(origin["currentRevisionNumber"])
):
    raise SystemExit("Course Instance Blueprint origin was not a closed ordered provenance")
forbidden = {"id", "accountId", "student", "studentRecord", "assignment", "sourceObject", "answerKey"}
if forbidden.intersection(value) or forbidden.intersection(course):
    raise SystemExit("Course Instance teaching-team view exposed future or private state")
' "$1" "$2" "$3" "$4" "$5"
}

assert_course_list() {
	python3 -c '
import json, sys
value = json.loads(sys.argv[1])
items = value.get("items")
if not isinstance(items, list):
    raise SystemExit("Course Instance list was malformed")
matches = [item for item in items if isinstance(item, dict) and item.get("id") == sys.argv[2]]
if len(matches) != 1:
    raise SystemExit("Assigned Instructor Course Instance list did not retain the created identity")
course = matches[0]
if course.get("shortName") != sys.argv[3] or course.get("longName") != sys.argv[4]:
    raise SystemExit("Assigned Instructor Course Instance list did not retain both names")
if course.get("classification") != json.loads(sys.argv[5]):
    raise SystemExit("Assigned Instructor Course Instance list did not retain its classification")
' "$1" "$2" "$3" "$4" "$5"
}

classification_uuid() {
	local path="$1" key="$2" name="$3" cookie="$4" response
	response="$(request "$path" "$cookie")"
	if [ "$(response_status "$response")" != "200" ]; then
		echo "Course Instance classification selector $key failed (HTTP $(response_status "$response"))" >&2
		return 1
	fi
	python3 -c '
import json, sys, uuid
payload = json.loads(sys.argv[1])
key, name = sys.argv[2:4]
items = payload.get(key)
if not isinstance(items, list) or any(not isinstance(item, dict) for item in items):
    raise SystemExit(f"Course Instance classification selector {key} did not return a list")
matches = [item for item in items if item.get("name") == name]
if len(matches) != 1:
    raise SystemExit(f"Course Instance requires exactly one installed {key} fixture named {name}; found {len(matches)}")
value = matches[0].get("uuid")
if not isinstance(value, str) or str(uuid.UUID(value)) != value:
    raise SystemExit(f"Course Instance {name} fixture did not return a canonical UUID")
print(value)
' "$(response_body "$response")" "$key" "$name"
}

course_classification() {
	local cookie="$1" discipline_uuid subject_uuid
	# ASVS 2.2.1, 16.5.3: failed selectors cannot become empty required UUIDs.
	discipline_uuid="$(classification_uuid '/api/content-classification/disciplines' disciplines Biology "$cookie")" || return 1
	subject_uuid="$(classification_uuid "/api/content-classification/subjects?disciplineUuid=$discipline_uuid" subjects Biochemistry "$cookie")" || return 1
	python3 -c '
import json, sys
print(json.dumps({"disciplineUuid": sys.argv[1], "subjectUuid": sys.argv[2], "topicUuid": None, "subtopicUuid": None, "tags": []}, separators=(",", ":")))
' "$discipline_uuid" "$subject_uuid"
}

instructor_account_id() {
	local response
	response="$(request '/api/auth/session' "$1")"
	[ "$(response_status "$response")" = 200 ] || return 1
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1])
account_id=value.get("account",{}).get("id")
if value.get("authenticated") is not True or value.get("account",{}).get("productRole") != "instructor":
    raise SystemExit("Course fixture requires an authenticated Instructor")
if not isinstance(account_id,str) or re.fullmatch(r"U[0-9A-HJKMNP-TV-Z]{8}", account_id) is None:
    raise SystemExit("Course fixture Instructor Account ID is invalid")
print(account_id)
' "$(response_body "$response")"
}

assert_database_evidence() {
	local course_instance_id="$1"
	local blueprint_course_id="$2"
	local blueprint_revision="$3"
	local instructor_account_id="$4"
	local postgres output sql
	postgres="$(service_id postgres)"
	sql="SELECT 'course_instance_authority'
      FROM ple_data.course_instance AS course
      JOIN ple_data.blueprint_course AS blueprint
        ON blueprint.blueprint_course_id = course.blueprint_course_id
      JOIN ple_private.account AS instructor
        ON instructor.account_id = :'instructor_account_id'
     WHERE course.course_instance_id = :'course_instance_id'
       AND blueprint.blueprint_course_id = :'blueprint_course_id'
       AND course.blueprint_revision_number = :'blueprint_revision'::bigint
       AND instructor.account_id = :'instructor_account_id'
       AND EXISTS (SELECT 1 FROM ple_data.course_origin AS origin
                    WHERE origin.course_instance_id = course.course_instance_id
                      AND origin.source_course_instance_id IS NULL
                      AND origin.blueprint_course_id = course.blueprint_course_id
                      AND origin.blueprint_revision_number = course.blueprint_revision_number)
       AND EXISTS (SELECT 1 FROM ple_data.course_membership AS membership
                    WHERE membership.course_instance_id = course.course_instance_id
                      AND membership.account_id = instructor.account_id
                      AND membership.role = 'instructor'
                      AND ple_data.course_membership_is_active(membership.course_membership_id))
       AND EXISTS (SELECT 1 FROM ple_audit.course_instance_creation_event AS event
                    WHERE event.course_instance_id = course.course_instance_id
                      AND event.assigned_instructor_account_id = instructor.account_id
                      AND event.created_by_account_id <> instructor.account_id)
       AND NOT EXISTS (SELECT 1 FROM ple_data.student_record WHERE course_instance_id = course.course_instance_id)
       AND (SELECT count(*) FROM ple_data.assessment WHERE course_instance_id = course.course_instance_id) = 1
       AND EXISTS (
           SELECT 1 FROM ple_data.assessment AS assessment
           JOIN ple_data.blueprint_course_revision AS source
             ON source.blueprint_course_id = course.blueprint_course_id
            AND source.blueprint_revision_number = course.blueprint_revision_number
           CROSS JOIN LATERAL ple_data.blueprint_content_assessments(source.content) AS member
           CROSS JOIN LATERAL ple_data.blueprint_content_question_pins(source.content) AS pin
           JOIN ple_data.assessment_entry AS entry ON entry.assessment_id = assessment.assessment_id
          WHERE assessment.course_instance_id = course.course_instance_id
            AND assessment.origin_kind = 'adopted'
            AND assessment.source_blueprint_course_id = course.blueprint_course_id
            AND assessment.source_blueprint_revision_number = course.blueprint_revision_number
            AND assessment.source_blueprint_assessment_id = member.blueprint_assessment_id
            AND assessment.assessment_status = 'unreleased'
            AND entry.entry_kind = 'fixed_question'
            AND EXISTS (
                SELECT 1 FROM ple_data.assessment_entry_question AS question
                 WHERE question.assessment_entry_id = entry.assessment_entry_id
                   AND question.published_question_id = pin.published_question_id
                   AND question.question_revision_number = pin.question_revision_number
                   AND question.points_possible = 1
            )
            AND (SELECT count(*) FROM ple_data.assessment_entry
                  WHERE assessment_id = assessment.assessment_id) = 1);"
	output="$(printf '%s\n' "$sql" | podman exec -i "$postgres" sh -lc 'exec psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At "$@"' sh -v course_instance_id="$course_instance_id" -v blueprint_course_id="$blueprint_course_id" -v blueprint_revision="$blueprint_revision" -v instructor_account_id="$instructor_account_id")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^course_instance_authority$/p')" != "course_instance_authority" ]; then
		echo "Course Instance atomic persistence evidence was not recorded" >&2
		exit 1
	fi
}

prove_authority() {
	local instructor_cookie sysadmin_cookie student_cookie library question_id created blueprint revision_one <retired-term-replace-me> private_adoption published replacement saved revision_two candidates assigned classification course_created course_instance_id second_course_created second_course_instance_id course_list course_view stale_created newer_created newer_course_instance_id
	instructor_cookie="$(persona_cookie elenaInstructor)"
	sysadmin_cookie="$(persona_cookie morganSysadmin)"
	student_cookie="$(persona_cookie maryStudent)"
	assert_concealed "$(request '/api/course-instances')"
	assert_concealed "$(request '/api/course-instances' "$student_cookie")"
	library="$(request '/api/questions/search?page_size=50' "$instructor_cookie")"
	if [ "$(response_status "$library")" != "200" ]; then
		echo "Instructor could not select a Published Question for an exact Blueprint source" >&2
		exit 1
	fi
	question_id="$(first_published_question_id "$(response_body "$library")")"
	classification="$(course_classification "$instructor_cookie")"
	created="$(request '/api/course-blueprints' "$instructor_cookie" POST "$(blueprint_payload "$question_id" "$classification")" '' "m8-blueprint-create-$run_id")"
	if [ "$(response_status "$created")" != "201" ]; then
		echo "Instructor could not create the exact Blueprint source (HTTP $(response_status "$created"): $(response_body "$created"))" >&2
		exit 1
	fi
read -r blueprint revision_one <retired-term-replace-me> < <(python3 -c '
import json, sys
value=json.loads(sys.argv[1]); blueprint_course_id=value.get("id"); revision=value.get("current_revision_tuple"); <retired-term-replace-me>=value.get("blueprint_edit_number")
if (not isinstance(blueprint_course_id,str) or not blueprint_course_id
    or revision != {"blueprintCourseId": blueprint_course_id, "revisionNumber": "1"}
    or not isinstance(<retired-term-replace-me>, str) or not <retired-term-replace-me>):
    raise SystemExit("Blueprint creation did not return available exact Revision 1")
print(blueprint_course_id, revision["revisionNumber"], <retired-term-replace-me>)
' "$(response_body "$created")")
	candidates="$(request '/api/course-instance-creation/instructors' "$sysadmin_cookie")"
	if [ "$(response_status "$candidates")" != "200" ]; then
		echo "Sysadmin could not obtain the bounded Assigned Instructor selection (HTTP $(response_status "$candidates"))" >&2
		exit 1
	fi
	assigned="$(instructor_account_id "$instructor_cookie")"
	assigned="$(python3 -c '
import json, sys
items=json.loads(sys.argv[1]).get("items")
if not isinstance(items,list) or not items or any(not isinstance(item,dict) for item in items):
    raise SystemExit("Assigned Instructor selection is empty or malformed")
account_ids=[item.get("id") for item in items]
if any(not isinstance(account_id,str) or not account_id for account_id in account_ids):
    raise SystemExit("Assigned Instructor selection lacks a canonical Account ID")
if len(set(account_ids)) != len(account_ids):
    raise SystemExit("Assigned Instructor selection duplicates a canonical Account ID")
if account_ids.count(sys.argv[2]) != 1:
    raise SystemExit("Authenticated demo Instructor is absent from Assigned Instructor selection")
print(sys.argv[2])
' "$(response_body "$candidates")" "$assigned")"
	private_adoption="$(request '/api/course-instances' "$sysadmin_cookie" POST "$(course_payload "$blueprint" "$revision_one" "$assigned" "$classification" "$course_short_name" "$course_long_name")")"
	# Morgan is not the Private Blueprint owner; this read must remain concealed.
	if [ "$(response_status "$private_adoption")" != "404" ]; then
		echo "Non-owner Sysadmin Private Blueprint refusal differed (HTTP $(response_status "$private_adoption"): $(response_body "$private_adoption"))" >&2
		exit 1
	fi
	published="$(request "/api/course-blueprints/$blueprint/publish" "$instructor_cookie" POST '' "\"$<retired-term-replace-me>\"")"
	if [ "$(response_status "$published")" != "200" ]; then
		echo "Instructor could not publish the Blueprint source for Course Instance adoption" >&2
		exit 1
	fi
	course_created="$(request '/api/course-instances' "$sysadmin_cookie" POST "$(course_payload "$blueprint" "$revision_one" "$assigned" "$classification" "$course_short_name" "$course_long_name")")"
	if [ "$(response_status "$course_created")" != "201" ]; then
		echo "Sysadmin could not create the Course Instance for the selected Instructor (HTTP $(response_status "$course_created"): $(response_body "$course_created"))" >&2
		exit 1
	fi
	course_instance_id="$(assert_course_receipt "$(response_body "$course_created")" "$course_short_name" "$course_long_name" "$classification")"
	second_course_created="$(request '/api/course-instances' "$sysadmin_cookie" POST "$(course_payload "$blueprint" "$revision_one" "$assigned" "$classification" "$second_course_short_name" "$second_course_long_name")")"
	if [ "$(response_status "$second_course_created")" != "201" ]; then
		echo "Sysadmin could not create the second Revision 1 Course Instance (HTTP $(response_status "$second_course_created"): $(response_body "$second_course_created"))" >&2
		exit 1
	fi
	second_course_instance_id="$(assert_course_receipt "$(response_body "$second_course_created")" "$second_course_short_name" "$second_course_long_name" "$classification")"
	assert_database_evidence "$course_instance_id" "$blueprint" "$revision_one" "$assigned"
	assert_database_evidence "$second_course_instance_id" "$blueprint" "$revision_one" "$assigned"
	assert_concealed "$(request '/api/course-instances' "$sysadmin_cookie")"
	assert_concealed "$(request "/api/course-instances/$course_instance_id" "$sysadmin_cookie")"
	course_list="$(request '/api/course-instances' "$instructor_cookie")"
	if [ "$(response_status "$course_list")" != "200" ]; then
		echo "Assigned Instructor could not enter the new Course Instance list" >&2
		exit 1
	fi
	assert_course_list "$(response_body "$course_list")" "$course_instance_id" "$course_short_name" "$course_long_name" "$classification"
	course_view="$(request "/api/course-instances/$course_instance_id" "$instructor_cookie")"
	if [ "$(response_status "$course_view")" != "200" ]; then
		echo "Assigned Instructor could not open the new Course Instance" >&2
		exit 1
	fi
	assert_instructor_view "$(response_body "$course_view")" "$course_instance_id" "$course_short_name" "$course_long_name" "$classification"
	replacement="$(replacement_payload "$(response_body "$created")")"
	saved="$(request "/api/course-blueprints/$blueprint" "$instructor_cookie" PUT "$replacement" '"1"' "m8-blueprint-save-$run_id")"
	if [ "$(response_status "$saved")" != "200" ]; then
		echo "Instructor could not Save the next Blueprint Revision" >&2
		exit 1
	fi
	revision_two="$(python3 -c '
import json, sys
value=json.loads(sys.argv[1]); course=value.get("blueprintCourse")
if (value.get("changed") is not True or not isinstance(course,dict)
        or course.get("current_revision_tuple") != {"blueprintCourseId":sys.argv[2],"revisionNumber":"2"}):
    raise SystemExit("changed Blueprint Save did not create exact Revision 2")
print("2")
' "$(response_body "$saved")" "$blueprint")"
	assert_database_evidence "$course_instance_id" "$blueprint" "$revision_one" "$assigned"
	assert_database_evidence "$second_course_instance_id" "$blueprint" "$revision_one" "$assigned"
	stale_created="$(request '/api/course-instances' "$sysadmin_cookie" POST "$(course_payload "$blueprint" "$revision_one" "$assigned" "$classification" "BIOL stale" "Molecular Biology stale Revision Pin")")"
	if [ "$(response_status "$stale_created")" != "412" ]; then
		echo "new Course Instance accepted a superseded Blueprint Revision" >&2
		exit 1
	fi
	newer_created="$(request '/api/course-instances' "$sysadmin_cookie" POST "$(course_payload "$blueprint" "$revision_two" "$assigned" "$classification" "$current_course_short_name" "$current_course_long_name")")"
	if [ "$(response_status "$newer_created")" != "201" ]; then
		echo "Sysadmin could not create a Course Instance from the current Blueprint Revision" >&2
		exit 1
	fi
	newer_course_instance_id="$(assert_course_receipt "$(response_body "$newer_created")" "$current_course_short_name" "$current_course_long_name" "$classification")"
	assert_database_evidence "$newer_course_instance_id" "$blueprint" "$revision_two" "$assigned"
	echo "Course Instance support fixture: $newer_course_instance_id"
	echo "Course Instance authority: current source selection, historical pin preservation, and no ambient Sysadmin access complete"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	NODE_EXTRA_CA_CERTS="$repository_root/local_stack_state/live_demo_browser/workspace/gateway-root.crt" \
		node tests/playwright/e2e_live_demo_course_instance_browser.mjs "$port"
	echo "Course Instance browser: visible Instructor creation and Teaching Team entry complete"
}

require_live_demo
case "$mode" in
	authority) prove_authority ;;
	browser) prove_browser ;;
	all) prove_authority; prove_browser ;;
esac

echo "Live Demo Course Instance: PASS"
