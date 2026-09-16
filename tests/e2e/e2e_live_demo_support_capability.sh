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
course_reference() { python3 -c 'import json,re,sys; values=[x.get("reference") for x in json.loads(sys.argv[1]).get("items",[]) if isinstance(x,dict)]; valid=[x for x in values if isinstance(x,str) and re.fullmatch(r"CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}",x)];
if not valid: raise SystemExit("Course Instance identity is invalid")
print(valid[-1])' "$1"; }
sysadmin_reference() { local postgres; postgres="$(service_id postgres)"; podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "SELECT public_reference FROM ple_private.account WHERE product_role = '\''sysadmin'\'' ORDER BY reference_number LIMIT 1"' | python3 -c 'import re,sys; value=sys.stdin.read().strip();
if not re.fullmatch(r"U[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}",value): raise SystemExit("Sysadmin identity is invalid")
print(value)'; }
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

# Only the implemented Student roster repair scope can be issued. Course and
# content repair remain future work, not authority-free receipt scaffolding.
repair_path='/api/support-repair-capabilities'
for denied_cookie in '' "$student_cookie" "$sysadmin_cookie"; do
    concealed "$(request "$repair_path" "$denied_cookie" POST "{\"sysadminReference\":\"$sysadmin\",\"resourceClass\":\"student\",\"resourceReference\":\"course-instance/$course/roster/m17-support\",\"purpose\":\"Correct a roster mismatch\"}")"
done
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
student_reference="course-instance/$course/roster/m17-support"
student_repair="$(issue_repair student "$student_reference" 'Correct a student roster mismatch')"
for unsupported in course content; do
    rejected="$(request "$repair_path" "$instructor_cookie" POST "{\"sysadminReference\":\"$sysadmin\",\"resourceClass\":\"$unsupported\",\"resourceReference\":\"$course\",\"purpose\":\"Unsupported repair\"}")"
    [ "$(status "$rejected")" = 422 ] || { echo "Unsupported support class accepted" >&2; exit 1; }
done
for invalid_scope in "$sysadmin" "course-instance/$course/roster/missing-profile" "$student_reference/extra"; do
    concealed "$(request "$repair_path" "$instructor_cookie" POST "{\"sysadminReference\":\"$sysadmin\",\"resourceClass\":\"student\",\"resourceReference\":\"$invalid_scope\",\"purpose\":\"Invalid repair scope\"}")"
done
repair_record_path() { printf '%s' "$repair_path/$1/course-instances/$course/roster/m17-support"; }
# C26 permits exactly the named Student record. A Course-class capability,
# ordinary Sysadmin administration, and every other role remain concealed.
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
support_sql() { podman exec -i "$postgres" sh -lc 'exec psql -X -q -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At "$@"' sh "$@"; }
# ASVS 8.3.2/8.3.3: a global Instructor cannot issue for an unrelated
# Course; the original issuer must retain current authority at every use.
# A separate disposable Course keeps its assigned Instructor active while
# the issuing co-Instructor departs, isolating the exact authority condition.
authority_course="$(support_sql -v capability="$student_repair" <<'SQL'
BEGIN;
SELECT gen_random_uuid() AS authority_instructor_id, gen_random_uuid() AS authority_course_id, gen_random_uuid() AS authority_profile_id \gset
INSERT INTO ple_private.account(account_id,product_role,created_at)
VALUES (:'authority_instructor_id','instructor',clock_timestamp());
INSERT INTO ple_data.course_instance(course_id,source_kind,assigned_instructor_account_id,course_short_name,course_long_name,discipline_uuid,tags,term_starts_on,term_ends_on,created_at,active_until_at,retention_starts_at)
SELECT :'authority_course_id','empty',:'authority_instructor_id','SUPPORT','Disposable support authority Course',discipline_uuid,tags,term_starts_on,term_ends_on,clock_timestamp(),active_until_at,retention_starts_at
FROM ple_data.course_instance WHERE public_reference = split_part((SELECT resource_reference FROM ple_private.support_repair_capability WHERE capability_id=:'capability'), '/', 2);
INSERT INTO ple_data.course_membership(membership_id,course_id,account_id,role,joined_at)
VALUES (gen_random_uuid(),:'authority_course_id',:'authority_instructor_id','instructor',clock_timestamp());
INSERT INTO ple_private.course_roster_profile(course_roster_profile_id,course_id,student_account_id,roster_id,created_at)
SELECT :'authority_profile_id',:'authority_course_id',student_account_id,'m17-support',clock_timestamp()
FROM ple_private.course_roster_profile WHERE course_id=(SELECT course_id FROM ple_data.course_instance WHERE public_reference=split_part((SELECT resource_reference FROM ple_private.support_repair_capability WHERE capability_id=:'capability'), '/', 2)) AND roster_id='m17-support';
INSERT INTO ple_private.course_invitation(invitation_id,course_id,target_account_id,membership_role,inviting_instructor_account_id,issued_at,expires_at)
SELECT gen_random_uuid(),course_id,student_account_id,'student',:'authority_instructor_id',clock_timestamp(),clock_timestamp()+interval '1 hour'
FROM ple_private.course_roster_profile WHERE course_roster_profile_id=:'authority_profile_id';
COMMIT;
SELECT public_reference FROM ple_data.course_instance WHERE course_id=:'authority_course_id';
SQL
)"
[[ "$authority_course" =~ ^CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$ ]] || { echo "Support authority fixture Course identity invalid" >&2; exit 1; }
authority_scope="course-instance/$authority_course/roster/m17-support"
concealed "$(request "$repair_path" "$instructor_cookie" POST "{\"sysadminReference\":\"$sysadmin\",\"resourceClass\":\"student\",\"resourceReference\":\"$authority_scope\",\"purpose\":\"Unrelated global Instructor denial\"}")"
support_sql -v capability="$student_repair" -v course="$authority_course" <<'SQL'
INSERT INTO ple_data.course_membership(membership_id,course_id,account_id,role,joined_at)
SELECT gen_random_uuid(),(SELECT course_id FROM ple_data.course_instance WHERE public_reference=:'course'),issuer_account_id,'instructor',clock_timestamp()
FROM ple_private.support_repair_capability WHERE capability_id=:'capability';
SQL
authority_repair="$(issue_repair student "$authority_scope" 'Original issuer authority regression')"
authority_path="$repair_path/$authority_repair/course-instances/$authority_course/roster/m17-support"
[ "$(status "$(request "$authority_path" "$sysadmin_cookie")")" = 200 ] || { echo "Current co-Instructor capability denied before authority change" >&2; exit 1; }
support_sql -v capability="$authority_repair" <<'SQL'
INSERT INTO ple_private.account_state_event(event_id,account_id,state,occurred_at,reason)
SELECT gen_random_uuid(),issuer_account_id,'deactivated',clock_timestamp(),'Disposable support authority regression' FROM ple_private.support_repair_capability WHERE capability_id=:'capability';
SQL
concealed "$(request "$authority_path" "$sysadmin_cookie")"
support_sql -v capability="$authority_repair" <<'SQL'
INSERT INTO ple_private.account_state_event(event_id,account_id,state,occurred_at)
SELECT gen_random_uuid(),issuer_account_id,'active',clock_timestamp() FROM ple_private.support_repair_capability WHERE capability_id=:'capability';
SQL
# Reactivation preserves the Account but cannot resurrect its revoked session.
instructor_cookie="$(persona_cookie elenaInstructor)"
[ "$(status "$(request "$authority_path" "$sysadmin_cookie")")" = 200 ] || { echo "Reactivated original issuer capability did not recover" >&2; exit 1; }
support_sql -v capability="$authority_repair" -v course="$authority_course" <<'SQL'
INSERT INTO ple_data.course_membership_event(course_membership_event_id,membership_id,event_kind,occurred_at,reason)
SELECT gen_random_uuid(),membership.membership_id,'ended',clock_timestamp(),'Disposable original issuer Course departure'
FROM ple_data.course_membership AS membership
JOIN ple_data.course_instance AS course ON course.course_id=membership.course_id
JOIN ple_private.support_repair_capability AS capability ON capability.issuer_account_id=membership.account_id
WHERE capability.capability_id=:'capability' AND course.public_reference=:'course'
AND membership.role='instructor' AND ple_data.course_membership_is_active(membership.membership_id);
SQL
concealed "$(request "$authority_path" "$sysadmin_cookie")"
authority_events="$(support_sql -v capability="$authority_repair" <<'SQL'
SELECT result FROM ple_audit.support_repair_capability_event WHERE capability_id=:'capability' ORDER BY result;
SQL
)"
[ "$authority_events" = $'issued\nused\nused' ] || { echo "Denied original issuer authority use produced a success audit" >&2; exit 1; }
podman exec "$postgres" sh -lc "psql -X -v ON_ERROR_STOP=1 -U \"\$POSTGRES_USER\" -d \"\$POSTGRES_DB\" -At -c \"SELECT account.product_role || ':' || count(membership.membership_id) FROM ple_private.account AS account LEFT JOIN ple_data.course_membership AS membership ON membership.account_id = account.account_id AND membership.course_id = (SELECT course_id FROM ple_data.course_instance WHERE public_reference = '$course') WHERE account.public_reference = '$sysadmin' GROUP BY account.product_role\"" | rg -qx 'sysadmin:0' || { echo "Support capability escalated Sysadmin role or Course membership" >&2; exit 1; }
revoked_student="$(request "$repair_path/$student_repair/revoke" "$instructor_cookie" POST '{}')"
[ "$(status "$revoked_student")" = 200 ] || { echo "Issuing Instructor could not revoke Student repair capability" >&2; exit 1; }
concealed "$(request "$(repair_record_path "$student_repair")" "$sysadmin_cookie")"
python3 -c 'import json,sys; x=json.loads(sys.argv[1]); raise SystemExit(0 if isinstance(x.get("revokedAt"),int) else "Support repair revocation receipt is invalid")' "$(body "$revoked_student")"
concealed "$(request "$repair_path/$student_repair/revoke" "$instructor_cookie" POST '{}')"
podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "DO \$\$ BEGIN IF has_table_privilege('\''ple_app'\'', '\''ple_private.support_repair_capability'\'', '\''SELECT'\'') OR has_table_privilege('\''ple_app'\'', '\''ple_audit.support_repair_capability_event'\'', '\''SELECT'\'') THEN RAISE EXCEPTION '\''direct support capability access widened'\''; END IF; END \$\$; SELECT '\''support_capability_catalog_authority'\'';"' | rg -qx 'support_capability_catalog_authority' || { echo "Support capability least privilege evidence failed" >&2; exit 1; }
# Audit evidence is stable product behavior: one issuance per named class and
# immutable revocation receipt, and one exact C26 Student-record use receipt.
repair_events="$(podman exec "$postgres" sh -lc "psql -X -v ON_ERROR_STOP=1 -U \"\$POSTGRES_USER\" -d \"\$POSTGRES_DB\" -At -c \"SELECT resource_class || ':' || result FROM ple_audit.support_repair_capability_event WHERE capability_id = '$student_repair' ORDER BY resource_class, result\"")"
python3 -c 'import sys; expected=["student:issued", "student:revoked", "student:used"]; actual=sorted(line for line in sys.stdin.read().splitlines() if line); raise SystemExit(0 if actual == expected else "Support repair audit receipts are incomplete")' <<<"$repair_events"
echo "Support repair authority: exact Student scope, unsupported-class denial, concealment, and revocation complete"
}

case "$mode" in
    issue) prove_issue ;;
    all) prove_issue ;;
esac
echo "Live Demo Support Repair Capability: PASS"
