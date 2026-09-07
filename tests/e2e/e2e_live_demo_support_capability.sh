#!/usr/bin/env bash
# Disposable M17 WP-M17-1 acceptance: closed exact-course support issuance.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

mode="all"
case "${1:-}" in
    "") ;;
    --issue) mode="issue" ;;
    --browser) mode="browser" ;;
    *) echo "Usage: bash $0 [--issue|--browser]" >&2; exit 2 ;;
esac
source "$repository_root/source_me.sh"
cd "$repository_root"

if [ ! -f "$runtime_environment_path" ]; then echo "Support capability evidence requires the fixed Live Demo to be running" >&2; exit 2; fi
service_id() { local ids; ids="$(podman ps -q --filter "label=com.docker.compose.project=$project_name" --filter "label=com.docker.compose.service=$1")"; [ "$(printf '%s\n' "$ids" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = 1 ] || { echo "Support capability evidence requires one running $1 service" >&2; exit 1; }; printf '%s\n' "$ids"; }
gateway_port() { local value; value="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"; [ "$(printf '%s\n' "$value" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = 1 ] || { echo "Support capability evidence requires one gateway port" >&2; exit 1; }; printf '%s\n' "${value#PLE_GATEWAY_HOST_PORT=}"; }
request() { local gateway port; gateway="$(service_id gateway)"; port="$(gateway_port)"; local -a args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}' --header "Host: localhost:$port" --request "${3:-GET}"); [ "${3:-GET}" = GET ] || args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json'); [ -z "${2:-}" ] || args+=(--header "Cookie: $2"); [ -z "${4:-}" ] || args+=(--data "$4"); podman exec "$gateway" curl "${args[@]}" "https://localhost:8080$1"; }
persona_cookie() { local gateway port headers cookie; gateway="$(service_id gateway)"; port="$(gateway_port)"; headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 --dump-header - --output /dev/null --header "Host: localhost:$port" --header "Origin: https://localhost:$port" --header 'Content-Type: application/json' --request POST --data "{\"persona\":\"$1\"}" 'https://localhost:8080/api/auth/live-demo/accounts')"; cookie="$(printf '%s\n' "$headers" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"; [ -n "$cookie" ] || { echo "seeded demo did not issue an Authenticated Session" >&2; exit 1; }; printf '%s\n' "$cookie"; }
status() { printf '%s' "${1##*$'\n'}"; }; body() { printf '%s' "${1%$'\n'*}"; }
concealed() { [ "$(status "$1")" = 404 ] || { echo "Support capability authority was not concealed" >&2; exit 1; }; }
course_reference() { python3 -c 'import json,re,sys; values=[x.get("reference") for x in json.loads(sys.argv[1]).get("items",[]) if isinstance(x,dict)]; valid=[x for x in values if isinstance(x,str) and re.fullmatch(r"C-[1-9][0-9]{0,9}",x)];
if not valid: raise SystemExit("Course Instance identity is invalid")
print(max(valid,key=lambda x:int(x[2:])))' "$1"; }
sysadmin_reference() { local postgres; postgres="$(service_id postgres)"; podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "SELECT reference_number FROM ple_private.account WHERE product_role = '\''sysadmin'\'' ORDER BY reference_number LIMIT 1"' | python3 -c 'import re,sys; value=sys.stdin.read().strip();
if not re.fullmatch(r"[1-9][0-9]*",value): raise SystemExit("Sysadmin identity is invalid")
print("U-"+value)'; }

issue_browser_capability() {
    local instructor_cookie course_response course sysadmin imported issued
    bash "$repository_root/tests/e2e/e2e_live_demo_course_instance.sh" --authority >/dev/null
    instructor_cookie="$(persona_cookie elenaInstructor)"
    course_response="$(request '/api/course-instances' "$instructor_cookie")"
    [ "$(status "$course_response")" = 200 ] || { echo "Instructor could not load Course Instances for scoped support" >&2; exit 1; }
    course="$(course_reference "$(body "$course_response")")"
    imported="$(request "/api/course-instances/$course/roster" "$instructor_cookie" POST '{"entries":[{"email":"m17.support@live-demo.invalid","rosterId":"m17-support"}]}')"
    [ "$(status "$imported")" = 201 ] || { echo "Instructor could not prepare the registered scoped-support roster projection" >&2; exit 1; }
    sysadmin="$(sysadmin_reference)"
    issued="$(request "/api/course-instances/$course/support-capabilities" "$instructor_cookie" POST "{\"sysadminReference\":\"$sysadmin\",\"purpose\":\"Investigate roster invitation state\"}")"
    [ "$(status "$issued")" = 201 ] || { echo "Current Course Instructor could not issue browser support capability" >&2; exit 1; }
    python3 -c 'import json,re,sys
x=json.loads(sys.argv[1]); capability=x.get("capabilityId")
if not isinstance(capability,str) or not re.fullmatch(r"[0-9a-f-]{36}",capability): raise SystemExit("Browser support capability receipt is invalid")
print(capability)' "$(body "$issued")"
}

prove_issue() {
instructor_cookie="$(persona_cookie elenaInstructor)"; sysadmin_cookie="$(persona_cookie morganSysadmin)"; student_cookie="$(persona_cookie maryStudent)"
bash "$repository_root/tests/e2e/e2e_live_demo_course_instance.sh" --authority >/dev/null
course_response="$(request '/api/course-instances' "$instructor_cookie")"; [ "$(status "$course_response")" = 200 ] || { echo "Instructor could not load Course Instances" >&2; exit 1; }
course="$(course_reference "$(body "$course_response")")"; sysadmin="$(sysadmin_reference)"
path="/api/course-instances/$course/support-capabilities"; request_body="{\"sysadminReference\":\"$sysadmin\",\"purpose\":\"Investigate roster invitation state\"}"
concealed "$(request "$path" '' POST "$request_body")"; concealed "$(request "$path" "$student_cookie" POST "$request_body")"; concealed "$(request "$path" "$sysadmin_cookie" POST "$request_body")"; concealed "$(request "/api/course-instances/C-0/support-capabilities" "$instructor_cookie" POST "$request_body")"
issued="$(request "$path" "$instructor_cookie" POST "$request_body")"; [ "$(status "$issued")" = 201 ] || { echo "Current Course Instructor could not issue support capability (HTTP $(status "$issued"))" >&2; exit 1; }
capability="$(python3 -c 'import json,re,sys
x=json.loads(sys.argv[1]); expected={"capabilityId","courseReference","sysadminReference","operationKind","minimumProjection","purpose","expiresAt","revokedAt"}
if set(x)!=expected: raise SystemExit("Support capability receipt field boundary is invalid")
if x["courseReference"]!=sys.argv[2] or x["sysadminReference"]!=sys.argv[3]: raise SystemExit("Support capability receipt identity is invalid")
if x["operationKind"]!="course_roster_support" or x["minimumProjection"]!="course_roster": raise SystemExit("Support capability receipt operation is invalid")
if x["purpose"]!="Investigate roster invitation state" or not isinstance(x["expiresAt"],int) or x["revokedAt"] is not None: raise SystemExit("Support capability receipt lifecycle is invalid")
if not re.fullmatch(r"[0-9a-f-]{36}",x["capabilityId"]): raise SystemExit("Support capability receipt identity token is invalid")
print(x["capabilityId"])' "$(body "$issued")" "$course" "$sysadmin")"
concealed "$(request "/api/course-instances/C-2147483647/support-capabilities/$capability/revoke" "$instructor_cookie" POST '{}')"
revoked="$(request "$path/$capability/revoke" "$instructor_cookie" POST '{}')"; [ "$(status "$revoked")" = 200 ] || { echo "Issuing Instructor could not revoke support capability" >&2; exit 1; }
python3 -c 'import json,sys; x=json.loads(sys.argv[1]); raise SystemExit(0 if isinstance(x.get("revokedAt"),int) else "Support capability revocation receipt is invalid")' "$(body "$revoked")"
concealed "$(request "$path/$capability/revoke" "$instructor_cookie" POST '{}')"
postgres="$(service_id postgres)"; podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "DO \$\$ BEGIN IF has_table_privilege('\''ple_app'\'', '\''ple_private.sysadmin_support_capability'\'', '\''SELECT'\'') OR has_table_privilege('\''ple_app'\'', '\''ple_audit.sysadmin_support_capability_event'\'', '\''SELECT'\'') THEN RAISE EXCEPTION '\''direct support capability access widened'\''; END IF; END \$\$; SELECT '\''support_capability_catalog_authority'\'';"' | rg -qx 'support_capability_catalog_authority' || { echo "Support capability least privilege evidence failed" >&2; exit 1; }
echo "Support capability authority: exact-course roster issuance, concealment, and revocation complete"
}

prove_browser() {
    local port capability
    capability="$(issue_browser_capability)"
    port="$(gateway_port)"
    node tests/e2e/e2e_live_demo_support_capability_browser.mjs "$port" "$capability"
    echo "Support capability browser: visible Sysadmin scoped roster task complete"
}

case "$mode" in
    issue) prove_issue ;;
    browser) prove_browser ;;
    all) prove_issue; prove_browser ;;
esac
echo "Live Demo Support Capability: PASS"
