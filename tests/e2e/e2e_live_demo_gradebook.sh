#!/usr/bin/env bash
# Disposable M15 WP-M15-1 acceptance: Instructor Gradebook evidence projection.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

mode="all"
case "${1:-}" in
    "") ;;
    --api) mode="api" ;;
    --browser) mode="browser" ;;
    *) echo "Usage: bash $0 [--api|--browser]" >&2; exit 2 ;;
esac
source "$repository_root/source_me.sh"
cd "$repository_root"

if [ ! -f "$runtime_environment_path" ]; then
    echo "Gradebook evidence requires the fixed Live Demo to be running" >&2
    exit 2
fi

service_id() {
    local ids
    ids="$(podman ps -q --filter "label=com.docker.compose.project=$project_name" --filter "label=com.docker.compose.service=$1")"
    [ "$(printf '%s\n' "$ids" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = 1 ] || {
        echo "Gradebook evidence requires one running $1 service" >&2
        exit 1
    }
    printf '%s\n' "$ids"
}

gateway_port() {
    local value
    value="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
    [ "$(printf '%s\n' "$value" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = 1 ] || {
        echo "Gradebook evidence requires one gateway port" >&2
        exit 1
    }
    printf '%s\n' "${value#PLE_GATEWAY_HOST_PORT=}"
}

request() {
    local gateway port
    gateway="$(service_id gateway)"
    port="$(gateway_port)"
    local -a args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}' --header "Host: localhost:$port" --request GET)
    [ -z "${2:-}" ] || args+=(--header "Cookie: $2")
    podman exec "$gateway" curl "${args[@]}" "https://localhost:8080$1"
}

persona_cookie() {
    local gateway port headers cookie
    gateway="$(service_id gateway)"
    port="$(gateway_port)"
    headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 --dump-header - --output /dev/null --header "Host: localhost:$port" --header "Origin: https://localhost:$port" --header 'Content-Type: application/json' --request POST --data "{\"persona\":\"$1\"}" 'https://localhost:8080/api/auth/live-demo/accounts')"
    cookie="$(printf '%s\n' "$headers" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"
    [ -n "$cookie" ] || { echo "seeded demo did not issue an Authenticated Session" >&2; exit 1; }
    printf '%s\n' "$cookie"
}

status() { printf '%s' "${1##*$'\n'}"; }
body() { printf '%s' "${1%$'\n'*}"; }

course_reference() {
    python3 -c 'import json,re,sys
items=json.loads(sys.argv[1]).get("items",[])
values=[x.get("reference") for x in items if isinstance(x,dict)]
valid=[x for x in values if isinstance(x,str) and re.fullmatch(r"C-[1-9][0-9]{0,9}",x)]
if not valid: raise SystemExit("Gradebook prerequisite lacks a Course Instance")
print(max(valid,key=lambda x:int(x[2:])))' "$1"
}

concealed() {
    [ "$(status "$1")" = 404 ] || { echo "Gradebook access was not concealed" >&2; exit 1; }
}

# M13 is the producer of the immutable Grading Result evidence that this
# read-only projection consumes. It remains a separate disposable acceptance.
bash "$repository_root/tests/e2e/e2e_live_demo_submission_recovery.sh" --fault >/dev/null
instructor_cookie="$(persona_cookie elenaInstructor)"
student_cookie="$(persona_cookie maryStudent)"
sysadmin_cookie="$(persona_cookie morganSysadmin)"
courses="$(request '/api/course-instances' "$instructor_cookie")"
[ "$(status "$courses")" = 200 ] || { echo "Instructor could not load Course Instances" >&2; exit 1; }
course="$(course_reference "$(body "$courses")")"
path="/api/course-instances/$course/gradebook"

concealed "$(request "$path")"
concealed "$(request "$path" "$student_cookie")"
concealed "$(request "$path" "$sysadmin_cookie")"
concealed "$(request '/api/course-instances/C-2147483647/gradebook' "$instructor_cookie")"

received="$(request "$path" "$instructor_cookie")"
[ "$(status "$received")" = 200 ] || { echo "Current Course Instructor could not read Gradebook" >&2; exit 1; }
python3 -c 'import json,re,sys
value=json.loads(sys.argv[1]); course=sys.argv[2]
if set(value)!={"courseReference","gradedStudentWork"} or value["courseReference"]!=course:
    raise SystemExit("Gradebook projection is not closed to its requested Course")
rows=value["gradedStudentWork"]
if not isinstance(rows,list) or not rows:
    raise SystemExit("Gradebook projection lacks immutable graded Student Work")
for row in rows:
    if set(row)!={"rosterId","assignmentReference","gradedQuestionCount","pointsEarned","pointsPossible"}:
        raise SystemExit("Gradebook projection exposed an unapproved field")
    if not isinstance(row["rosterId"],str) or not re.fullmatch(r"[A-Za-z0-9._-]{1,64}",row["rosterId"]):
        raise SystemExit("Gradebook roster projection is invalid")
    if not isinstance(row["assignmentReference"],str) or not re.fullmatch(r"A-[1-9][0-9]{0,9}",row["assignmentReference"]):
        raise SystemExit("Gradebook Assignment projection is invalid")
    if not isinstance(row["gradedQuestionCount"],int) or row["gradedQuestionCount"] < 1:
        raise SystemExit("Gradebook graded Question count is invalid")
    if not all(isinstance(row[k],(int,float)) and not isinstance(row[k],bool) for k in ("pointsEarned","pointsPossible")) or not 0 <= row["pointsEarned"] <= row["pointsPossible"]:
        raise SystemExit("Gradebook points projection is invalid")
serialized=json.dumps(value).lower()
if any(word in serialized for word in ("studentresponse","answerkey","sourceobject","checksum","grader")):
    raise SystemExit("Gradebook projection exposed private grading evidence")' "$(body "$received")" "$course"

postgres="$(service_id postgres)"
podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "DO \$\$ BEGIN IF has_table_privilege('\''ple_app'\'', '\''ple_private.grading_result'\'', '\''SELECT'\'') OR has_table_privilege('\''ple_app'\'', '\''ple_private.question_submission'\'', '\''SELECT'\'') THEN RAISE EXCEPTION '\''direct Gradebook evidence access widened'\''; END IF; END \$\$; SELECT '\''gradebook_catalog_authority'\'';"' | rg -qx 'gradebook_catalog_authority' || { echo "Gradebook least-privilege evidence failed" >&2; exit 1; }

echo "Gradebook authority: current Course Instructor receives answer-free immutable grading evidence with concealed foreign access"
if [ "$mode" = "browser" ]; then
    node tests/playwright/e2e_live_demo_gradebook_browser.mjs "$(gateway_port)" "$course"
    echo "Gradebook browser: visible current-Instructor answer-free grading evidence with concealed foreign access complete"
fi
echo "Live Demo Gradebook: PASS"
