#!/usr/bin/env bash
# Disposable acceptance: ordinary Instructor Assignment listing and concealment.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

if [ ! -f "$runtime_environment_path" ]; then
	echo "Assignment list evidence requires the fixed Live Demo to be running" >&2
	exit 2
fi

service_id() {
	local service="$1" identifiers
	identifiers="$(podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=$service")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Assignment list evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Assignment list evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1" cookie="${2:-}" method="${3:-GET}" body="${4:-}"
	local gateway port
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	local -a curl_args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}'
		--header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then
		curl_args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
	fi
	if [ -n "$cookie" ]; then curl_args+=(--header "Cookie: $cookie"); fi
	if [ -n "$body" ]; then curl_args+=(--data "$body"); fi
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
		echo "Course Assignment access was not concealed" >&2
		exit 1
	fi
}

latest_course_reference() {
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1]).get("items")
references=[item.get("reference") for item in items if isinstance(item,dict)] if isinstance(items,list) else []
if not references or any(not isinstance(value,str) or not re.fullmatch(r"C-[1-9][0-9]{0,9}",value) for value in references):
    raise SystemExit("Assignment list prerequisite lacks a Course Instance")
print(max(references,key=lambda value:int(value[2:])))
' "$1"
}

assert_assignment_list() {
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1])
expected={"M10 live assignment":"released","M3 unreleased assignment":"unreleased"}
if not isinstance(items,list):
    raise SystemExit("Course Assignment list is not an array")
observed={}
for item in items:
    if not isinstance(item,dict) or set(item)!={"reference","title","dueAt","displayTimeZone","status","editNumber"}:
        raise SystemExit("Course Assignment list is not a closed projection")
    if not isinstance(item["reference"],str) or not re.fullmatch(r"A-[1-9][0-9]{0,9}",item["reference"]):
        raise SystemExit("Course Assignment list lacks a public Assignment Reference")
    if item["status"] not in {"unreleased","released","closed","archived"}:
        raise SystemExit("Course Assignment list contains an invalid Assignment Status")
    if not isinstance(item["editNumber"],str) or not item["editNumber"].isdigit():
        raise SystemExit("Course Assignment list contains an invalid Assignment Edit Number")
    if item["dueAt"] is not None and (not isinstance(item["dueAt"],str) or not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{3}",item["dueAt"])):
        raise SystemExit("Course Assignment list contains an invalid local Due at")
    if not isinstance(item["displayTimeZone"],str) or not item["displayTimeZone"]:
        raise SystemExit("Course Assignment list lacks the governing Instructor time zone")
    observed[item["title"]]=item["status"]
if any(observed.get(title) != status for title,status in expected.items()):
    raise SystemExit("Course Assignment list did not retain released and unreleased Assignments")
serialized=json.dumps(items).lower()
if any(word in serialized for word in ("student", "response", "answer", "grading", "points")):
    raise SystemExit("Course Assignment list exposed Student or grading state")
' "$1"
}

blueprint_reference_and_revision() {
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1]).get("items")
if not isinstance(items,list) or not items:
    raise SystemExit("Foreign-Course setup lacks an available Blueprint Course")
item=max(items,key=lambda value:int(value.get("reference","BP-0")[3:]) if isinstance(value,dict) and re.fullmatch(r"BP-[1-9][0-9]{0,9}",value.get("reference","")) else 0)
reference=item.get("reference"); revision=item.get("revision")
if not isinstance(reference,str) or not re.fullmatch(r"BP-[1-9][0-9]{0,9}",reference) or not isinstance(revision,str) or not revision.isdigit():
    raise SystemExit("Foreign-Course setup Blueprint source is malformed")
print(reference,revision)
' "$1"
}

foreign_instructor_reference() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); reference=value.get("reference")
if set(value)!={"reference","state","lastSuccessfulSignIn"} or not isinstance(reference,str) or not re.fullmatch(r"U-[1-9][0-9]{0,9}",reference) or value.get("state")!="active":
    raise SystemExit("Foreign Instructor creation receipt is malformed")
print(reference)
' "$1"
}

foreign_course_reference() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); course=value.get("course",{}); reference=course.get("reference")
if set(value)!={"course","creatorIsAssignedInstructor"} or value.get("creatorIsAssignedInstructor") is not False or not isinstance(reference,str) or not re.fullmatch(r"C-[1-9][0-9]{0,9}",reference):
    raise SystemExit("Foreign Course Instance creation receipt is malformed")
print(reference)
' "$1"
}

# The release acceptance creates an exact Instructor-owned Course with one
# Released Assignment through ordinary product HTTP operations.
bash "$repository_root/tests/e2e/e2e_live_demo_assignment_release.sh" --service >/dev/null

instructor_cookie="$(persona_cookie elenaInstructor)"
student_cookie="$(persona_cookie maryStudent)"
sysadmin_cookie="$(persona_cookie morganSysadmin)"
courses="$(request '/api/course-instances' "$instructor_cookie")"
if [ "$(response_status "$courses")" != "200" ]; then
	echo "Instructor could not list the Assignment prerequisite Course" >&2
	exit 1
fi
course_reference="$(latest_course_reference "$(response_body "$courses")")"
path="/api/course-instances/$course_reference/assignments"

assert_concealed "$(request "$path")"
assert_concealed "$(request "$path" "$student_cookie")"
assert_concealed "$(request "$path" "$sysadmin_cookie")"

created="$(request "$path" "$instructor_cookie" POST '{"title":"M3 unreleased assignment","instructions":"Keep this Assignment unreleased for list evidence."}')"
if [ "$(response_status "$created")" != "201" ]; then
	echo "Instructor could not create the Unreleased Assignment list fixture" >&2
	exit 1
fi

listed="$(request "$path" "$instructor_cookie")"
if [ "$(response_status "$listed")" != "200" ]; then
	echo "Current Course Instructor could not list Assignments" >&2
	exit 1
fi
assert_assignment_list "$(response_body "$listed")"

created_instructor="$(request '/api/instructor-accounts' "$sysadmin_cookie" POST \
	'{"normalizedEmail":"m3-foreign-instructor@live-demo.invalid"}')"
if [ "$(response_status "$created_instructor")" != "201" ]; then
	echo "Foreign Instructor setup could not create an Instructor Account" >&2
	exit 1
fi
foreign_instructor="$(foreign_instructor_reference "$(response_body "$created_instructor")")"
blueprints="$(request '/api/course-blueprints?pageSize=100' "$instructor_cookie")"
if [ "$(response_status "$blueprints")" != "200" ]; then
	echo "Foreign-Course setup could not list an available Blueprint Course" >&2
	exit 1
fi
read -r blueprint_reference blueprint_revision < <(blueprint_reference_and_revision "$(response_body "$blueprints")")
foreign_payload="$(python3 -c '
import json, sys
print(json.dumps({"blueprintCourse":sys.argv[1],"blueprintRevision":sys.argv[2],"title":"M3 foreign Instructor Course","term":{"startDate":"2026-08-24","endDate":"2026-12-11"},"assignedInstructor":sys.argv[3]},separators=(",",":")))
' "$blueprint_reference" "$blueprint_revision" "$foreign_instructor")"
created_course="$(request '/api/course-instances' "$sysadmin_cookie" POST "$foreign_payload")"
if [ "$(response_status "$created_course")" != "201" ]; then
	echo "Foreign-Course setup could not create an exact Course Instance" >&2
	exit 1
fi
foreign_course="$(foreign_course_reference "$(response_body "$created_course")")"
assert_concealed "$(request "/api/course-instances/$foreign_course/assignments" "$instructor_cookie")"

echo "Course Assignment list: direct Instructor released/unreleased projection and exact 404 concealment complete"
echo "Live Demo Course Assignment List: PASS"
