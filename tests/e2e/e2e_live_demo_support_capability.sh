#!/usr/bin/env bash
# Disposable acceptance: confirmed and audited Sysadmin Student-data access.

set -euo pipefail
set +o braceexpand

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"
case "${1:-}" in
    ""|--issue) ;;
    *) echo "Usage: bash $0 [--issue]" >&2; exit 2 ;;
esac
source "$repository_root/source_me.sh"
cd "$repository_root"

if [ ! -f "$runtime_environment_path" ]; then
    echo "Student-data access evidence requires the fixed Live Demo to be running" >&2
    exit 2
fi
service_id() {
    local ids
    ids="$(podman ps -q --filter "label=com.docker.compose.project=$project_name" --filter "label=com.docker.compose.service=$1")"
    [ "$(printf '%s\n' "$ids" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = 1 ] || {
        echo "Student-data access evidence requires one running $1 service" >&2
        exit 1
    }
    printf '%s\n' "$ids"
}
gateway_port() {
    local value
    value="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
    [ "$(printf '%s\n' "$value" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = 1 ] || {
        echo "Student-data access evidence requires one gateway port" >&2
        exit 1
    }
    printf '%s\n' "${value#PLE_GATEWAY_HOST_PORT=}"
}
request() {
    local gateway port
    gateway="$(service_id gateway)"
    port="$(gateway_port)"
    local -a args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}' --header "Host: localhost:$port" --request "${3:-GET}")
    [ "${3:-GET}" = GET ] || args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
    [ -z "${2:-}" ] || args+=(--header "Cookie: $2")
    [ -z "${4:-}" ] || args+=(--data "$4")
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
concealed() {
    [ "$(status "$1")" = 404 ] || {
        echo "Sysadmin Student-data authority was not concealed (${2:-unnamed scope}, HTTP $(status "$1"))" >&2
        exit 1
    }
}
course_id() {
    python3 -c 'import json,re,sys; values=[x.get("id") for x in json.loads(sys.argv[1]).get("items",[]) if isinstance(x,dict)]; course_id=sys.argv[2]
if not re.fullmatch(r"CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}",course_id) or values.count(course_id)!=1: raise SystemExit("Owned Course Instance identity is invalid or absent")
print(course_id)' "$1" "$2"
}
sysadmin_cookie() {
    local gateway ca_file
    gateway="$(service_id gateway)"
    ca_file="$(mktemp "${TMPDIR:-/tmp}/ple-morgan-ca.XXXXXX")"
    trap 'rm -f -- "$ca_file"' EXIT
    podman exec "$gateway" cat /data/caddy/pki/authorities/local/root.crt > "$ca_file"
    python3 tests/e2e/e2e_live_demo_session.py "$(gateway_port)" "${PLE_LOCAL_DEMO_TOTP_SETUP_FILE:-$repository_root/local_stack_state/live_demo_browser/workspace/morgan-totp-setup-uri}" --ca-file "$ca_file"
    rm -f -- "$ca_file"
    trap - EXIT
}

instructor_cookie="$(persona_cookie elenaInstructor)"
student_cookie="$(persona_cookie maryStudent)"
admin_cookie="$(sysadmin_cookie)"
course_evidence="$(bash "$repository_root/tests/e2e/e2e_live_demo_course_instance.sh" --authority)"
owned_course="$(printf '%s\n' "$course_evidence" | sed -n 's/^Course Instance support fixture: //p')"
course_response="$(request '/api/course-instances' "$instructor_cookie")"
[ "$(status "$course_response")" = 200 ] || { echo "Instructor could not load Course Instances" >&2; exit 1; }
course="$(course_id "$(body "$course_response")" "$owned_course")"
imported="$(request "/api/course-instances/$course/roster" "$instructor_cookie" POST '{"entries":[{"email":"m17.support@biology.roosevelt.edu","rosterId":"m17-support","rosterName":"Synthetic Support Student"},{"email":"m18.support@biology.roosevelt.edu","rosterId":"m18-support","rosterName":"Synthetic Support Student"}]}')"
[ "$(status "$imported")" = 201 ] || { echo "Instructor could not prepare named Student roster records" >&2; exit 1; }

path="/api/sysadmin/course-instances/$course/roster/m17-support/student-data"
[ "$(status "$(request "$path" "$admin_cookie" GET)")" = 405 ] || { echo "Direct GET unexpectedly admitted Student-data access" >&2; exit 1; }
concealed "$(request "$path" '' POST '{"administrativeAccessConfirmed":true}')" 'anonymous read'
concealed "$(request "$path" "$student_cookie" POST '{"administrativeAccessConfirmed":true}')" 'Student read'
concealed "$(request "$path" "$instructor_cookie" POST '{"administrativeAccessConfirmed":true}')" 'Instructor read'
for invalid_confirmation in '{"administrativeAccessConfirmed":false}' '{}'; do
    response="$(request "$path" "$admin_cookie" POST "$invalid_confirmation")"
    [ "$(status "$response")" = 422 ] || { echo "Unconfirmed Student-data access was not rejected" >&2; exit 1; }
done
concealed "$(request "/api/sysadmin/course-instances/$course/roster/missing-student/student-data" "$admin_cookie" POST '{"administrativeAccessConfirmed":true}')" 'missing Student read'
concealed "$(request "/api/support-repair-capabilities" "$instructor_cookie" POST '{}')" 'retired Instructor grant endpoint'

record="$(request "$path" "$admin_cookie" POST '{"administrativeAccessConfirmed":true}')"
[ "$(status "$record")" = 200 ] || { echo "Confirmed Sysadmin Student-data read failed" >&2; exit 1; }
student_account="$(python3 -c 'import json,re,sys; x=json.loads(sys.argv[1]); expected={"courseInstanceId","studentAccountId","rosterId","rosterName","state","audit"}; audit=x.get("audit",{}); valid=set(x)==expected and x["courseInstanceId"]==sys.argv[2] and x["rosterId"]=="m17-support" and x["rosterName"]=="Synthetic Support Student" and x["state"]=="invitationPending" and set(audit)=={"eventId","occurredAt"} and re.fullmatch(r"U[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}",x["studentAccountId"]) and re.fullmatch(r"[0-9a-f-]{36}",audit["eventId"]) and isinstance(audit["occurredAt"],int)
if not valid: raise SystemExit("Confirmed Student-data projection or audit receipt is invalid")
print(x["studentAccountId"])' "$(body "$record")" "$course")"
postgres="$(service_id postgres)"
podman exec "$postgres" sh -lc "psql -X -v ON_ERROR_STOP=1 -U \"\$POSTGRES_USER\" -d \"\$POSTGRES_DB\" -At -c \"SELECT event_kind || ':' || acting_account_id || ':' || student_account_id FROM ple_audit.course_roster_event WHERE course_instance_id = '$course' AND student_account_id = '$student_account' AND event_kind = 'sysadmin_student_data_accessed'\"" | rg -q '^sysadmin_student_data_accessed:U[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}:U[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}$' || {
    echo "Confirmed Student-data read did not record the acting Sysadmin and Student" >&2
    exit 1
}
podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "DO \$\$ BEGIN IF to_regclass('\''ple_private.support_repair_capability'\'') IS NOT NULL OR to_regclass('\''ple_audit.support_repair_capability_event'\'') IS NOT NULL THEN RAISE EXCEPTION '\''retired support registry remains installed'\''; END IF; END \$\$; SELECT '\''sysadmin_student_data_schema_clean'\'';"' | rg -qx 'sysadmin_student_data_schema_clean' || {
    echo "Retired support registry remains installed" >&2
    exit 1
}
echo "Live Demo confirmed Sysadmin Student-data access: PASS"
