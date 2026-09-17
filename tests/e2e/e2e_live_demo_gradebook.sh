#!/usr/bin/env bash
# Disposable acceptance: Instructor Gradebook evidence projection.

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

postgres_scalar() {
    local postgres sql="$1"
    postgres="$(service_id postgres)"
    podman exec "$postgres" sh -lc \
        'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' \
        sh "$sql"
}

course_reference() {
    python3 -c 'import json,re,sys
items=json.loads(sys.argv[1]).get("items",[])
values=[x.get("reference") for x in items if isinstance(x,dict)]
valid=[x for x in values if isinstance(x,str) and re.fullmatch(r"CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}",x)]
if not valid: raise SystemExit("Gradebook prerequisite lacks a Course Instance")
print(valid[0])' "$1"
}

concealed() {
    [ "$(status "$1")" = 404 ] || { echo "Gradebook access was not concealed" >&2; exit 1; }
}

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
concealed "$(request '/api/course-instances/CI8H4N6P/gradebook' "$instructor_cookie")"

received="$(request "$path" "$instructor_cookie")"
[ "$(status "$received")" = 200 ] || { echo "Current Course Instructor could not read Gradebook" >&2; exit 1; }
python3 -c 'import json,math,re,sys
value=json.loads(sys.argv[1]); course=sys.argv[2]
if set(value)!={"courseReference","studentWork"} or value["courseReference"]!=course:
    raise SystemExit("Gradebook projection is not closed to its requested Course")
rows=value["studentWork"]
if not isinstance(rows,list) or not rows:
    raise SystemExit("Gradebook projection lacks active Student Work")
for row in rows:
    if set(row)!={"rosterId","rosterName","assessmentReference","assessmentTitle","assessmentAttemptCompletion","expiredSubmitting","score"}:
        raise SystemExit("Gradebook projection exposed an unapproved field")
    if not isinstance(row["rosterId"],str) or not re.fullmatch(r"[A-Za-z0-9._-]{1,64}",row["rosterId"]):
        raise SystemExit("Gradebook roster projection is invalid")
    if not isinstance(row["assessmentReference"],str) or not re.fullmatch(r"A[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}",row["assessmentReference"]):
        raise SystemExit("Gradebook Assessment projection is invalid")
    if not isinstance(row["assessmentTitle"],str) or not row["assessmentTitle"].strip() or len(row["assessmentTitle"])>200:
        raise SystemExit("Gradebook Coursework title projection is invalid")
    if row["assessmentAttemptCompletion"] not in (None,"inProgress","completed"):
        raise SystemExit("Gradebook Assessment Attempt completion is invalid")
    if not isinstance(row["expiredSubmitting"],bool):
        raise SystemExit("Gradebook expiry projection is invalid")
    score=row["score"]
    if score is not None:
        if not isinstance(score,dict) or set(score)!={"pointsEarned","pointsPossible"} or not all(isinstance(score[k],(int,float)) and not isinstance(score[k],bool) and math.isfinite(score[k]) and score[k]>=0 for k in ("pointsEarned","pointsPossible")):
            raise SystemExit("Gradebook points projection is invalid")
    if row["assessmentAttemptCompletion"] is None and (score is not None or row["expiredSubmitting"]):
        raise SystemExit("Gradebook not-started row exposes work totals")
    if row["expiredSubmitting"] and (row["assessmentAttemptCompletion"] != "inProgress" or score is not None):
        raise SystemExit("Gradebook expired submission projection is invalid")
serialized=json.dumps(value).lower()
if any(word in serialized for word in ("studentresponse","answerkey","sourceobject","checksum","grader")):
    raise SystemExit("Gradebook projection exposed private grading evidence")' "$(body "$received")" "$course"

postgres="$(service_id postgres)"
gateway="$(service_id gateway)"
port="$(gateway_port)"
headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 \
    --dump-header - --output /dev/null --header "Host: localhost:$port" \
    --header "Cookie: $instructor_cookie" "https://localhost:8080$path")"
printf '%s\n' "$headers" | tr -d '\r' | rg -qi '^cache-control: no-store$' || {
    echo "Gradebook response was cacheable" >&2
    exit 1
}
podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "DO \$\$ BEGIN IF has_table_privilege('\''ple_app'\'', '\''ple_private.grading_result'\'', '\''SELECT'\'') OR has_table_privilege('\''ple_app'\'', '\''ple_private.question_response'\'', '\''SELECT'\'') THEN RAISE EXCEPTION '\''direct Gradebook evidence access widened'\''; END IF; END \$\$; SELECT '\''gradebook_catalog_authority'\'';"' | rg -qx 'gradebook_catalog_authority' || { echo "Gradebook least-privilege evidence failed" >&2; exit 1; }

echo "Gradebook authority: current Course Instructor receives answer-free immutable grading evidence with concealed foreign access"
if [ "$mode" = "browser" ]; then
    node tests/playwright/e2e_live_demo_gradebook_browser.mjs "$(gateway_port)" "$course"
    echo "Gradebook browser: visible current-Instructor answer-free grading evidence with concealed foreign access complete"
fi
echo "Live Demo Gradebook: PASS"
