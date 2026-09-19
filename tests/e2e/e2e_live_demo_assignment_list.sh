#!/usr/bin/env bash
# Disposable acceptance: current Instructor Assessment list and concealment.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
# shellcheck disable=SC1091
source "$repository_root/tests/e2e/e2e_live_demo_assignment_helpers.sh"
cd "$repository_root"

assert_assessment_list() {
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1])
expected={"M10 released Assessment":"released","M3 unreleased Assessment":"unreleased"}
if not isinstance(items,list):
    raise SystemExit("Course Assessment list is not an array")
observed={}
for item in items:
    if not isinstance(item,dict) or set(item)!={"id","assessmentType","title","dueAt","displayTimeZone","status","editNumber"}:
        raise SystemExit("Course Assessment list is not a closed current projection")
    if not isinstance(item["id"],str) or not re.fullmatch(r"A[0-9A-HJKMNP-TV-Z]{8}",item["id"]):
        raise SystemExit("Course Assessment list lacks a canonical Assessment ID")
    if item["assessmentType"] not in {"practice_question_assignment","regular_assignment","quiz","exam"}:
        raise SystemExit("Course Assessment list contains an invalid Assessment Type")
    if item["status"] not in {"unreleased","released","closed","archived"}:
        raise SystemExit("Course Assessment list contains an invalid Assessment Status")
    if not isinstance(item["editNumber"],str) or not item["editNumber"].isdigit():
        raise SystemExit("Course Assessment list contains an invalid Assessment Edit Number")
    if item["dueAt"] is not None and (not isinstance(item["dueAt"],str) or not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{3}",item["dueAt"])):
        raise SystemExit("Course Assessment list contains an invalid local Due at")
    if not isinstance(item["displayTimeZone"],str) or not item["displayTimeZone"]:
        raise SystemExit("Course Assessment list lacks the governing Instructor time zone")
    observed[item["title"]]=(item["status"], item["assessmentType"])
if any(observed.get(title) != (status, "practice_question_assignment") for title,status in expected.items()):
    raise SystemExit("Course Assessment list did not retain released and unreleased Assessments")
serialized=json.dumps(items).lower()
if any(word in serialized for word in ("student", "response", "answer", "grading", "points")):
    raise SystemExit("Course Assessment list exposed Student or grading state")
' "$1"
}

foreign_instructor_reference() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); reference=value.get("id")
avatar=value.get("providedAvatarId")
if set(value)!={"id","state","lastSuccessfulSignIn","providedAvatarId"} or not isinstance(reference,str) or not re.fullmatch(r"U[0-9A-HJKMNP-TV-Z]{8}",reference) or value.get("state")!="active" or value.get("lastSuccessfulSignIn") is not None or (avatar is not None and (not isinstance(avatar,str) or not avatar)):
    raise SystemExit("Foreign Instructor creation receipt is malformed")
print(reference)
' "$1"
}

vetting_decision_reference() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); reference=value.get("vettingDecisionReference")
if set(value)!={"vettingDecisionReference"} or not isinstance(reference,str) or not re.fullmatch(r"[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}",reference):
    raise SystemExit("Foreign Instructor vetting receipt is malformed")
print(reference)
' "$1"
}

foreign_course_payload() {
	python3 -c '
import json, sys
course, assigned = json.loads(sys.argv[1]), sys.argv[2]
classification = course.get("course", {}).get("classification")
if not isinstance(classification, dict):
    raise SystemExit("Assessment list fixture Course lacks its classification")
payload={
    "classification": classification,
    "source": {"kind":"empty"},
    "shortName":"M3 foreign",
    "longName":"M3 Foreign Instructor Course",
    "term":{"startDate":"2026-09-01","endDate":"2026-12-18"},
    "assignedInstructor": assigned,
}
print(json.dumps(payload,separators=(",",":")))
' "$1" "$2"
}

foreign_course_reference() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); course=value.get("course",{}); reference=course.get("id")
if set(value)!={"course"} or not isinstance(reference,str) or not re.fullmatch(r"CI[0-9A-HJKMNP-TV-Z]{8}",reference):
    raise SystemExit("Foreign Course Instance creation receipt is malformed")
print(reference)
' "$1"
}

morgan_sysadmin_cookie() {
	local gateway port setup_file ca_file
	ca_file=""
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	setup_file="${PLE_LOCAL_DEMO_TOTP_SETUP_FILE:-$repository_root/local_stack_state/live_demo_browser/workspace/morgan-totp-setup-uri}"
	(
		set -e
		ca_file="$(mktemp "${TMPDIR:-/tmp}/ple-morgan-ca.XXXXXX")"
		trap '[[ -z "${ca_file:-}" ]] || rm -f -- "$ca_file"' EXIT
		# ASVS 6.3.4, 6.5.1, 6.5.5, and 7.2.4: use the normal local MFA path;
		# trust only this gateway CA and return its fresh session through the pipe.
		podman exec "$gateway" cat /data/caddy/pki/authorities/local/root.crt > "$ca_file"
		python3 tests/e2e/e2e_live_demo_session.py "$port" "$setup_file" --ca-file "$ca_file"
	)
}

require_live_demo
instructor_cookie="$(persona_cookie elenaInstructor)"
student_cookie="$(persona_cookie maryStudent)"
sysadmin_cookie="$(morgan_sysadmin_cookie)"
course="$(new_course_reference "$instructor_cookie")"
path="/api/course-instances/$course/assessments"

assert_concealed "$(request "$path")"
assert_concealed "$(request "$path" "$student_cookie")"
assert_concealed "$(request "$path" "$sysadmin_cookie")"

created="$(request "$path" "$instructor_cookie" POST '{"assessmentType":"practice_question_assignment","title":"M10 released Assessment","instructions":"Release this Assessment for list evidence."}')"
require_status "Released Assessment creation" "$created" 201
read -r assessment initial_edit < <(workspace_reference_and_edit "$(response_body "$created")")
picker="$(request "/api/course-instances/$course/assessment-question-picker" "$instructor_cookie")"
require_status "Assessment Question picker" "$picker" 200
question_reference="$(picker_reference "$(response_body "$picker")")"
payload="$(save_payload "$(response_body "$created")" "$question_reference" "M10 released Assessment")"
saved="$(request "/api/course-instances/$course/assessments/$assessment" "$instructor_cookie" PUT "$payload" "$initial_edit")"
require_status "Released Assessment save" "$saved" 200
read -r _ saved_edit < <(workspace_reference_and_edit "$(response_body "$saved")")
released="$(request "/api/course-instances/$course/assessments/$assessment/release" "$instructor_cookie" POST '' "$saved_edit")"
require_status "Released Assessment release" "$released" 200

created="$(request "$path" "$instructor_cookie" POST '{"assessmentType":"practice_question_assignment","title":"M3 unreleased Assessment","instructions":"Keep this Assessment unreleased for list evidence."}')"
require_status "Unreleased Assessment creation" "$created" 201

listed="$(request "$path" "$instructor_cookie")"
require_status "Current Course Instructor Assessment list" "$listed" 200
assert_assessment_list "$(response_body "$listed")"

run_id="$(python3 -c 'import uuid; print(uuid.uuid4().hex[:12])')"
foreign_email="m3-foreign-$run_id@live-demo.invalid"
vetted="$(request '/api/instructor-identity-vetting-decisions' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"$foreign_email\",\"verifiedInstructorDisplayName\":\"M3 Foreign Instructor\"}")"
require_status "Foreign Instructor vetting" "$vetted" 201
vetting_decision="$(vetting_decision_reference "$(response_body "$vetted")")"
created_instructor="$(request '/api/instructor-accounts' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"$foreign_email\",\"vettingDecisionReference\":\"$vetting_decision\"}")"
require_status "Foreign Instructor setup" "$created_instructor" 201
foreign_instructor="$(foreign_instructor_reference "$(response_body "$created_instructor")")"
course_view="$(request "/api/course-instances/$course" "$instructor_cookie")"
require_status "Assessment list fixture Course view" "$course_view" 200
foreign_payload="$(foreign_course_payload "$(response_body "$course_view")" "$foreign_instructor")"
created_course="$(request '/api/course-instances' "$sysadmin_cookie" POST "$foreign_payload")"
require_status "Foreign Course setup" "$created_course" 201
foreign_course="$(foreign_course_reference "$(response_body "$created_course")")"
assert_concealed "$(request "/api/course-instances/$foreign_course/assessments" "$instructor_cookie")"

echo "Course Assessment list: direct Instructor released/unreleased projection and exact 404 concealment complete"
echo "Live Demo Course Assessment List: PASS"
