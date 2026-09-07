#!/usr/bin/env bash
# Disposable M9 acceptance: roster import, invitation claim, Student Record, and immediate revocation.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_roster.sh [--import|--browser]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--import) mode="import" ;;
	--browser) mode="browser" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Course Roster evidence requires the fixed Live Demo to be running" >&2
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
		echo "Course Roster evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Course Roster evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1"
	local cookie="${2:-}"
	local method="${3:-GET}"
	local body="${4:-}"
	local gateway port
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	local -a curl_args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}'
		--header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then curl_args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json'); fi
	if [ -n "$cookie" ]; then curl_args+=(--header "Cookie: $cookie"); fi
	if [ -n "$body" ]; then curl_args+=(--data "$body"); fi
	podman exec "$gateway" curl "${curl_args[@]}" "https://localhost:8080$path"
}

persona_cookie() {
	local persona="$1"
	local gateway port headers cookie
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
		echo "Course Roster access was not concealed" >&2
		exit 1
	fi
}

new_course_reference() {
	local instructor_cookie list
	instructor_cookie="$1"
	bash "$repository_root/tests/e2e/e2e_live_demo_course_instance.sh" --authority >/dev/null
	list="$(request '/api/course-instances' "$instructor_cookie")"
	if [ "$(response_status "$list")" != "200" ]; then
		echo "Instructor could not list the prerequisite Course Instance" >&2
		exit 1
	fi
	python3 -c '
import json, re, sys
items=json.loads(sys.argv[1]).get("items")
if not isinstance(items,list): raise SystemExit("Course Instance list was malformed")
references=[item.get("reference") for item in items if isinstance(item,dict)]
if not references or any(not isinstance(value,str) or not re.fullmatch(r"C-[1-9][0-9]{0,9}",value) for value in references):
    raise SystemExit("Course Instance list lacks public identities")
print(max(references,key=lambda value:int(value[2:])))
' "$(response_body "$list")"
}

assert_import_projection() {
	python3 -c '
import json, sys
items=json.loads(sys.argv[1])
if not isinstance(items,list) or len(items) != 2:
    raise SystemExit("Course Roster Import did not return its two reviewed rows")
for item in items:
    if not isinstance(item,dict) or set(item)!={"rosterId","rosterEmail","state"}:
        raise SystemExit("Course Roster Import response is not a closed roster projection")
    if item["state"] != "invitationPending":
        raise SystemExit("Course Roster Import did not retain pending Course Invitations")
    if not isinstance(item["rosterId"],str) or not isinstance(item["rosterEmail"],str):
        raise SystemExit("Course Roster Import projection is malformed")
' "$1"
}

assert_active_projection() {
	python3 -c '
import json, sys
items=json.loads(sys.argv[1])
if not isinstance(items,list) or len(items) != 2:
    raise SystemExit("Course Roster projection lost an imported row")
states={item.get("rosterId"):item.get("state") for item in items if isinstance(item,dict)}
if states.get("m9-seeded") != "activeStudent" or states.get("m9-created") != "invitationPending":
    raise SystemExit("Course Invitation claim did not create only the exact active Student membership")
' "$1"
}

assert_database_evidence() {
	local course_reference="$1"
	local postgres output sql
	postgres="$(service_id postgres)"
	sql="DO \$\$
DECLARE
    v_course_id uuid;
    v_student_account_id uuid;
BEGIN
    SELECT course_id INTO v_course_id FROM ple_data.course_instance
     WHERE reference_number = ${course_reference#C-};
    SELECT profile.student_account_id INTO v_student_account_id
      FROM ple_private.course_roster_profile AS profile
     WHERE profile.course_id = v_course_id AND profile.roster_id = 'm9-seeded';
    IF v_course_id IS NULL OR v_student_account_id IS NULL
       OR NOT EXISTS (SELECT 1 FROM ple_data.student_record AS record
                      WHERE record.course_id = v_course_id AND record.student_account_id = v_student_account_id)
       OR EXISTS (SELECT 1 FROM ple_data.course_membership AS membership
                  WHERE membership.course_id = v_course_id AND membership.account_id = v_student_account_id
                    AND membership.role = 'student' AND ple_data.course_membership_is_active(membership.membership_id))
       OR (SELECT count(*) FROM ple_private.account_authentication_email
           WHERE normalized_email = 'm9-created@live-demo.invalid') <> 1
       OR NOT EXISTS (SELECT 1 FROM ple_audit.course_roster_event AS event
                      WHERE event.course_id = v_course_id AND event.student_account_id = v_student_account_id
                        AND event.event_kind = 'invitation_claimed')
       OR NOT EXISTS (SELECT 1 FROM ple_audit.course_roster_event AS event
                      WHERE event.course_id = v_course_id AND event.student_account_id = v_student_account_id
                        AND event.event_kind = 'student_access_revoked')
       OR EXISTS (SELECT 1 FROM ple_data.assignment WHERE course_id = v_course_id)
    THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Roster atomic evidence is incomplete';
    END IF;
END
\$\$;
SELECT 'course_roster_authority';"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^course_roster_authority$/p')" != "course_roster_authority" ]; then
		echo "Course Roster atomic persistence evidence was not recorded" >&2
		exit 1
	fi
}

prove_import() {
	local instructor_cookie student_cookie course_reference initial imported imported_again claimed listed revoked
	instructor_cookie="$(persona_cookie elenaInstructor)"
	student_cookie="$(persona_cookie maryStudent)"
	course_reference="$(new_course_reference "$instructor_cookie")"
	assert_concealed "$(request "/api/course-instances/$course_reference/roster")"
	assert_concealed "$(request "/api/course-instances/$course_reference/roster" "$student_cookie")"
	initial="$(request "/api/course-instances/$course_reference/roster" "$instructor_cookie")"
	if [ "$(response_status "$initial")" != "200" ] || [ "$(response_body "$initial")" != "[]" ]; then
		echo "Course Roster did not begin empty for its exact Course Instance" >&2
		exit 1
	fi
	imported="$(request "/api/course-instances/$course_reference/roster" "$instructor_cookie" POST '{"entries":[{"email":"mary.student@live-demo.invalid","rosterId":"m9-seeded"},{"email":"m9-created@live-demo.invalid","rosterId":"m9-created"}]}')"
	if [ "$(response_status "$imported")" != "201" ]; then
		echo "Instructor could not commit Course Roster Import" >&2
		exit 1
	fi
	assert_import_projection "$(response_body "$imported")"
	imported_again="$(request "/api/course-instances/$course_reference/roster" "$instructor_cookie" POST '{"entries":[{"email":"mary.student@live-demo.invalid","rosterId":"m9-seeded"},{"email":"m9-created@live-demo.invalid","rosterId":"m9-created"}]}')"
	if [ "$(response_status "$imported_again")" != "201" ]; then
		echo "Course Roster Import was not idempotent" >&2
		exit 1
	fi
	assert_import_projection "$(response_body "$imported_again")"
	claimed="$(request "/api/course-instances/$course_reference/roster/claim" "$student_cookie" POST '{}')"
	if [ "$(response_status "$claimed")" != "200" ] || [ "$(response_body "$claimed")" != '{"activeStudentMembership":true}' ]; then
		echo "Authenticated Student could not claim the exact Course Invitation" >&2
		exit 1
	fi
	listed="$(request "/api/course-instances/$course_reference/roster" "$instructor_cookie")"
	if [ "$(response_status "$listed")" != "200" ]; then
		echo "Instructor could not load the authorized Course Roster" >&2
		exit 1
	fi
	assert_active_projection "$(response_body "$listed")"
	revoked="$(request "/api/course-instances/$course_reference/roster/m9-seeded/revoke" "$instructor_cookie" POST '{}')"
	if [ "$(response_status "$revoked")" != "204" ]; then
		echo "Instructor could not revoke exact Student course access" >&2
		exit 1
	fi
	assert_concealed "$(request "/api/course-instances/$course_reference/roster/claim" "$student_cookie" POST '{}')"
	assert_database_evidence "$course_reference"
	echo "Course Roster authority: idempotent import, exact Student Record claim, and immediate revocation complete"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	node tests/playwright/e2e_live_demo_roster_browser.mjs "$port"
	echo "Course Roster browser: visible Instructor import and protected roster projection complete"
}

require_live_demo
case "$mode" in
	import) prove_import ;;
	browser) prove_browser ;;
	all) prove_import; prove_browser ;;
esac

echo "Live Demo Course Roster: PASS"
