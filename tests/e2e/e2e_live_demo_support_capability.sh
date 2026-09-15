#!/usr/bin/env bash
# Disposable acceptance: exact-record support repair.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

mode="all"
case "${1:-}" in
    "") ;;
    --issue) mode="issue" ;;
    *) echo "Usage: bash $0 [--issue]" >&2; exit 2 ;;
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
prove_issue() {
instructor_cookie="$(persona_cookie elenaInstructor)"; sysadmin_cookie="$(persona_cookie morganSysadmin)"; student_cookie="$(persona_cookie maryStudent)"
bash "$repository_root/tests/e2e/e2e_live_demo_course_instance.sh" --authority >/dev/null
course_response="$(request '/api/course-instances' "$instructor_cookie")"; [ "$(status "$course_response")" = 200 ] || { echo "Instructor could not load Course Instances" >&2; exit 1; }
course="$(course_reference "$(body "$course_response")")"; sysadmin="$(sysadmin_reference)"
imported="$(request "/api/course-instances/$course/roster" "$instructor_cookie" POST '{"entries":[{"email":"m17.support@biology.roosevelt.edu","rosterId":"m17-support"},{"email":"m18.support@biology.roosevelt.edu","rosterId":"m18-support"}]}')"
[ "$(status "$imported")" = 201 ] || { echo "Instructor could not prepare the named Student support record" >&2; exit 1; }
# A Sysadmin's platform authority never implies membership-based Course roster
# access; exact-record repair is the only support path.
concealed "$(request "/api/course-instances/$course/roster" "$sysadmin_cookie")"

# C25 is a request receipt, not a generic support-data endpoint.  These three
# externally visible resource classes are a deliberately stable contract.
repair_path='/api/support-repair-capabilities'
concealed "$(request "$repair_path" '' POST "{\"sysadminReference\":\"$sysadmin\",\"resourceClass\":\"course\",\"resourceReference\":\"$course\",\"purpose\":\"Correct a course configuration\"}")"
concealed "$(request "$repair_path" "$student_cookie" POST "{\"sysadminReference\":\"$sysadmin\",\"resourceClass\":\"course\",\"resourceReference\":\"$course\",\"purpose\":\"Correct a course configuration\"}")"
concealed "$(request "$repair_path" "$sysadmin_cookie" POST "{\"sysadminReference\":\"$sysadmin\",\"resourceClass\":\"course\",\"resourceReference\":\"$course\",\"purpose\":\"Correct a course configuration\"}")"
issue_repair() {
    local resource_class="$1" resource_reference="$2" purpose="$3" repair_response
    repair_response="$(request "$repair_path" "$instructor_cookie" POST "{\"sysadminReference\":\"$sysadmin\",\"resourceClass\":\"$resource_class\",\"resourceReference\":\"$resource_reference\",\"purpose\":\"$purpose\"}")"
    [ "$(status "$repair_response")" = 201 ] || { echo "Instructor could not issue $resource_class repair capability" >&2; exit 1; }
    python3 -c 'import json,re,sys,time
x=json.loads(sys.argv[1]); expected={"capabilityId","sysadminReference","resourceClass","resourceReference","purpose","expiresAt","revokedAt"}
if set(x)!=expected: raise SystemExit("Support repair receipt field boundary is invalid")
if x["sysadminReference"]!=sys.argv[2] or x["resourceClass"]!=sys.argv[3] or x["resourceReference"]!=sys.argv[4] or x["purpose"]!=sys.argv[5]: raise SystemExit("Support repair receipt contents are invalid")
remaining=x["expiresAt"]-int(time.time()*1000)
if not isinstance(x["expiresAt"],int) or not 3540000 <= remaining <= 3660000 or x["revokedAt"] is not None or not re.fullmatch(r"[0-9a-f-]{36}",x["capabilityId"]): raise SystemExit("Support repair receipt lifecycle is invalid")
print(x["capabilityId"])' "$(body "$repair_response")" "$sysadmin" "$resource_class" "$resource_reference" "$purpose"
}
course_repair="$(issue_repair course "$course" 'Correct a course configuration')"
student_reference="course-instance/$course/roster/m17-support"
student_repair="$(issue_repair student "$student_reference" 'Correct a student roster mismatch')"
content_repair="$(issue_repair content 'content-record:module-7' 'Correct a content configuration')"
repair_record_path() { printf '%s' "$repair_path/$1/course-instances/$course/roster/m17-support"; }
# C26 permits exactly the named Student record. A Course-class capability,
# ordinary Sysadmin administration, and every other role remain concealed.
concealed "$(request "$(repair_record_path "$course_repair")" "$sysadmin_cookie")"
concealed "$(request "$(repair_record_path "$student_repair")" '' )"
concealed "$(request "$(repair_record_path "$student_repair")" "$student_cookie")"
concealed "$(request "$(repair_record_path "$student_repair")" "$instructor_cookie")"
concealed "$(request "$repair_path/$student_repair/course-instances/$course/roster/m18-support" "$sysadmin_cookie")"
repair_record="$(request "$(repair_record_path "$student_repair")" "$sysadmin_cookie")"
[ "$(status "$repair_record")" = 200 ] || { echo "Exact Student repair capability could not read its named record" >&2; exit 1; }
python3 -c 'import json,sys; x=json.loads(sys.argv[1]); expected={"rosterId","state"}; raise SystemExit(0 if set(x)==expected and x["rosterId"]=="m17-support" and x["state"]=="invitationPending" else "Exact Student support projection is invalid")' "$(body "$repair_record")"
# Capability use never makes the Sysadmin a Course member or Instructor, so
# the normal Instructor-only mutation remains concealed as well.
concealed "$(request "/api/course-instances/$course/roster" "$sysadmin_cookie" POST '{"entries":[{"email":"m18.denied@biology.roosevelt.edu","rosterId":"m18-denied"}]}')"
postgres="$(service_id postgres)"
podman exec "$postgres" sh -lc "psql -X -v ON_ERROR_STOP=1 -U \"\$POSTGRES_USER\" -d \"\$POSTGRES_DB\" -At -c \"SELECT account.product_role || ':' || count(membership.membership_id) FROM ple_private.account AS account LEFT JOIN ple_data.course_membership AS membership ON membership.account_id = account.account_id AND membership.course_id = (SELECT course_id FROM ple_data.course_instance WHERE reference_number = ${course#C-}) WHERE account.reference_number = ${sysadmin#U-} GROUP BY account.product_role\"" | rg -qx 'sysadmin:0' || { echo "Support capability escalated Sysadmin role or Course membership" >&2; exit 1; }
revoked_student="$(request "$repair_path/$student_repair/revoke" "$instructor_cookie" POST '{}')"
[ "$(status "$revoked_student")" = 200 ] || { echo "Issuing Instructor could not revoke Student repair capability" >&2; exit 1; }
concealed "$(request "$(repair_record_path "$student_repair")" "$sysadmin_cookie")"
revoked_repair="$(request "$repair_path/$content_repair/revoke" "$instructor_cookie" POST '{}')"
[ "$(status "$revoked_repair")" = 200 ] || { echo "Issuing Instructor could not revoke repair capability" >&2; exit 1; }
python3 -c 'import json,sys; x=json.loads(sys.argv[1]); raise SystemExit(0 if isinstance(x.get("revokedAt"),int) else "Support repair revocation receipt is invalid")' "$(body "$revoked_repair")"
concealed "$(request "$repair_path/$content_repair/revoke" "$instructor_cookie" POST '{}')"
podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "DO \$\$ BEGIN IF has_table_privilege('\''ple_app'\'', '\''ple_private.support_repair_capability'\'', '\''SELECT'\'') OR has_table_privilege('\''ple_app'\'', '\''ple_audit.support_repair_capability_event'\'', '\''SELECT'\'') THEN RAISE EXCEPTION '\''direct support capability access widened'\''; END IF; END \$\$; SELECT '\''support_capability_catalog_authority'\'';"' | rg -qx 'support_capability_catalog_authority' || { echo "Support capability least privilege evidence failed" >&2; exit 1; }
# Audit evidence is stable product behavior: one issuance per named class and
# immutable revocation receipt, and one exact C26 Student-record use receipt.
repair_events="$(podman exec "$postgres" sh -lc "psql -X -v ON_ERROR_STOP=1 -U \"\$POSTGRES_USER\" -d \"\$POSTGRES_DB\" -At -c \"SELECT resource_class || ':' || result FROM ple_audit.support_repair_capability_event WHERE capability_id IN ('$course_repair', '$student_repair', '$content_repair') ORDER BY resource_class, result\"")"
python3 -c 'import sys; expected=["content:issued", "content:revoked", "course:issued", "student:issued", "student:revoked", "student:used"]; actual=sorted(line for line in sys.stdin.read().splitlines() if line); raise SystemExit(0 if actual == expected else "Support repair audit receipts are incomplete")' <<<"$repair_events"
echo "Support repair authority: course/student/content issuance, concealment, and revocation complete"
}

case "$mode" in
    issue) prove_issue ;;
    all) prove_issue ;;
esac
echo "Live Demo Support Repair Capability: PASS"
