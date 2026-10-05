#!/usr/bin/env bash
# Disposable acceptance: Sysadmin-only Instructor Account lifecycle.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_instructor_accounts.sh [--service|--browser]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--service) mode="service" ;;
	--browser) mode="browser" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Instructor Account evidence requires the fixed Live Demo to be running" >&2
		exit 2
	fi
}

service_id() {
	local service="$1" identifiers
	identifiers="$(podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=$service")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Instructor Account evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Instructor Account evidence requires one validated gateway port setting" >&2
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
	if [ "$persona" = morganSysadmin ]; then
		local setup_file="${PLE_LOCAL_DEMO_TOTP_SETUP_FILE:-$repository_root/local_stack_state/live_demo_browser/workspace/morgan-totp-setup-uri}"
		(
			set -e
			local ca_file
			ca_file="$(mktemp "${TMPDIR:-/tmp}/ple-morgan-ca.XXXXXX")"
			trap 'rm -f -- "$ca_file"' EXIT
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
		echo "Instructor Account authority was not concealed" >&2
		exit 1
	fi
}

assert_list() {
	python3 -c '
import json, re, sys
page=json.loads(sys.argv[1])
if not isinstance(page,dict) or set(page)!={"accounts","displayTimeZone","nextCursor"}:
	raise SystemExit("Instructor Account list envelope is not closed")
if not isinstance(page["displayTimeZone"],str) or not page["displayTimeZone"]:
	raise SystemExit("Instructor Account list lacks its Sysadmin viewer zone")
if page["nextCursor"] is not None and (not isinstance(page["nextCursor"],str) or not re.fullmatch(r"U[0-9A-HJKMNP-TV-Z]{8}",page["nextCursor"])):
    raise SystemExit("Instructor Account list cursor is malformed")
items=page["accounts"]
if not isinstance(items,list) or not items:
    raise SystemExit("Instructor Account list is not a nonempty array")
for item in items:
    if not isinstance(item,dict) or set(item)!={"id","state","lastSuccessfulSignIn","providedAvatarId"}:
        raise SystemExit("Instructor Account list projection is not closed")
    if not isinstance(item["id"],str) or not re.fullmatch(r"U[0-9A-HJKMNP-TV-Z]{8}",item["id"]):
        raise SystemExit("Instructor Account ID is malformed")
    if item["state"] not in {"active","deactivated","closed"}:
        raise SystemExit("Instructor Account state is malformed")
    if item["lastSuccessfulSignIn"] is not None and (not isinstance(item["lastSuccessfulSignIn"],int) or isinstance(item["lastSuccessfulSignIn"],bool)):
        raise SystemExit("Instructor Account sign-in projection is malformed")
' "$1"
}

seeded_elena_instructor_account_id() {
	local entry
	entry="$(rg --no-messages '^PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID=U[0-9A-HJKMNP-TV-Z]{8}$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entry" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Live Demo does not declare one seeded Elena Instructor Account" >&2
		exit 1
	fi
	printf '%s\n' "${entry#PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID=}"
}

assert_active_listed_account() {
	python3 -c '
import json, sys

page = json.loads(sys.argv[1])
account_id = sys.argv[2]
if not any(item.get("id") == account_id and item.get("state") == "active" for item in page["accounts"]):
    raise SystemExit("Live Demo did not list the active seeded Elena Instructor Account")
' "$1" "$2"
}

assert_summary() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); expected_account_id=sys.argv[2]; expected_state=sys.argv[3]
if not isinstance(value,dict) or set(value)!={"id","state","lastSuccessfulSignIn","providedAvatarId"}:
    raise SystemExit("Instructor Account lifecycle projection is not closed")
if value.get("id") != expected_account_id or not re.fullmatch(r"U[0-9A-HJKMNP-TV-Z]{8}", expected_account_id):
    raise SystemExit("Instructor Account lifecycle changed its public identity")
if value.get("state") != expected_state or value.get("lastSuccessfulSignIn") is not None:
    raise SystemExit("Instructor Account lifecycle did not retain the expected state")
' "$1" "$2" "$3"
}

assert_created_account() {
	python3 -c '
import json, sys

value = json.loads(sys.argv[1])
if set(value) != {"account", "setupEmailSent"}:
    raise SystemExit("Instructor Account creation response is not closed")
if value["setupEmailSent"] is not False:
    raise SystemExit("unconfigured Live Demo unexpectedly reported setup-email delivery")
print(json.dumps(value["account"]))
' "$1"
}

instructor_account_count_for_email() {
	local email="$1" postgres
	postgres="$(service_id postgres)"
	podman exec "$postgres" sh -lc \
		'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh \
		"SELECT count(*) FROM ple_private.account_authentication_email AS email
          JOIN ple_private.account AS account ON account.account_id = email.account_id
         WHERE email.normalized_email = '$email' AND account.user_role = 'instructor'"
}

assert_no_instructor_account_for_email() {
	if [ "$(instructor_account_count_for_email "$1")" != "0" ]; then
		echo "Invalid Instructor setup created an Account" >&2
		exit 1
	fi
}

assert_catalog_least_privilege() {
	local postgres sql output
	postgres="$(service_id postgres)"
	sql="DO \$\$
BEGIN
    IF has_table_privilege('ple_app', 'ple_private.account', 'SELECT')
       OR has_table_privilege('ple_app', 'ple_private.account_state_event', 'SELECT')
       OR has_table_privilege('ple_app', 'ple_private.authenticated_session', 'SELECT')
    THEN RAISE EXCEPTION USING ERRCODE='42501', MESSAGE='ple_app received direct private account reads'; END IF;
END
\$\$;
SELECT 'instructor_account_catalog_authority';"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^instructor_account_catalog_authority$/p')" != "instructor_account_catalog_authority" ]; then
		echo "Instructor Account catalog least-privilege evidence was not recorded" >&2
		exit 1
	fi
}

instructor_preservation_snapshot() {
	local account_id="$1" postgres sql
	if [[ ! "$account_id" =~ ^U[0-9A-HJKMNP-TV-Z]{8}$ ]]; then
		echo "Instructor preservation evidence received an invalid Account ID" >&2
		exit 1
	fi
	postgres="$(service_id postgres)"
	sql="WITH instructor AS (
    SELECT account_id FROM ple_private.account
     WHERE account_id = :'account_id' AND user_role = 'instructor'
), snapshot AS (
    SELECT
        (SELECT count(*) FROM ple_private.authoring_workspace AS workspace
          JOIN instructor ON instructor.account_id = workspace.owner_account_id) AS authored_workspaces,
        (SELECT count(*) FROM ple_data.blueprint_course AS blueprint
          JOIN instructor ON instructor.account_id = blueprint.owner_account_id) AS authored_blueprints,
        (SELECT count(*) FROM ple_data.question_ownership_event AS ownership
          JOIN instructor ON instructor.account_id = ownership.owner_account_id) AS authored_questions,
        (SELECT count(*) FROM ple_data.course_membership AS membership
          JOIN instructor ON instructor.account_id = membership.account_id
         WHERE membership.role = 'instructor') AS instructor_memberships,
        (SELECT count(*) FROM ple_audit.course_instance_creation_event AS event
          JOIN instructor ON instructor.account_id IN (
              event.assigned_instructor_account_id, event.created_by_account_id
          )) AS course_creation_history,
        (SELECT count(*) FROM ple_audit.course_roster_event AS event
          JOIN instructor ON instructor.account_id = event.acting_account_id) AS roster_history,
        (SELECT count(*) FROM ple_private.account_state_event AS event
          JOIN instructor ON instructor.account_id = event.account_id) AS state_events
)
SELECT json_build_object(
    'authoredWorkspaces', authored_workspaces,
    'authoredBlueprints', authored_blueprints,
    'authoredQuestions', authored_questions,
    'instructorMemberships', instructor_memberships,
    'courseCreationHistory', course_creation_history,
    'rosterHistory', roster_history,
    'stateEvents', state_events
) FROM snapshot;"
	printf '%s\n' "$sql" | podman exec -i "$postgres" sh -lc 'exec psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At "$@"' sh -v account_id="$account_id"
}

assert_deactivation_preserves_records() {
	python3 -c '
import json, sys
before, after = (json.loads(value) for value in sys.argv[1:])
required = (
    "authoredWorkspaces", "authoredBlueprints", "authoredQuestions",
    "instructorMemberships", "courseCreationHistory", "rosterHistory",
)
if set(before) != set(after) or set(before) != set(required) | {"stateEvents"}:
    raise SystemExit("Instructor preservation snapshot shape changed")
if any(not isinstance(before[key], int) or before[key] < 1 for key in required):
    raise SystemExit("Live Demo lacks the authored, Course, and historical records required for preservation evidence")
if any(after[key] != before[key] for key in required):
    raise SystemExit("Instructor deactivation deleted authored content, a Course relationship, or historical records")
if not isinstance(before["stateEvents"], int) or after["stateEvents"] != before["stateEvents"] + 1:
    raise SystemExit("Instructor deactivation did not append exactly one Account state history record")
' "$1" "$2"
}

prove_service() {
	local sysadmin_cookie instructor_cookie student_cookie listed existing_account_id created created_account_id deactivated reactivated preservation_before preservation_after missing_email approved_email
	sysadmin_cookie="$(persona_cookie morganSysadmin)"
	instructor_cookie="$(persona_cookie elenaInstructor)"
	student_cookie="$(persona_cookie maryStudent)"

	assert_concealed "$(request '/api/instructor-accounts')"
	assert_concealed "$(request '/api/instructor-accounts' "$student_cookie")"
	assert_concealed "$(request '/api/instructor-accounts' "$instructor_cookie")"
	assert_concealed "$(request '/api/instructor-accounts/U0/deactivate' "$sysadmin_cookie" POST '{"reason":"bounded"}')"
	assert_concealed "$(request '/api/instructor-accounts/not-an-id/reactivate' "$sysadmin_cookie" POST '{}')"
	assert_concealed "$(request '/api/instructor-accounts/UZZZZZZZZ/reactivate' "$sysadmin_cookie" POST '{}')"
	assert_concealed "$(request '/api/instructor-accounts' "$instructor_cookie" POST '{"normalizedEmail":"instructor-cannot-create@example.invalid","firstName":"Instructor","lastName":"Cannot Create","affiliation":"Example"}')"

	listed="$(request '/api/instructor-accounts' "$sysadmin_cookie")"
	if [ "$(response_status "$listed")" != "200" ]; then
		echo "Active Sysadmin could not list Instructor Accounts" >&2
		exit 1
	fi
	assert_list "$(response_body "$listed")"
	existing_account_id="$(seeded_elena_instructor_account_id)"
	assert_active_listed_account "$(response_body "$listed")" "$existing_account_id"

	# Required setup fields are validated atomically before Account creation.
	missing_email="m18-missing-${RANDOM}${RANDOM}@example.invalid"
	if [ "$(response_status "$(request '/api/instructor-accounts' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"$missing_email\"}")")" != "422" ]; then
		echo "Instructor creation without required setup fields was not denied" >&2
		exit 1
	fi
	assert_no_instructor_account_for_email "$missing_email"

	approved_email="m18-approved-${RANDOM}${RANDOM}@example.invalid"
	created="$(request '/api/instructor-accounts' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"$approved_email\",\"firstName\":\"M18\",\"lastName\":\"Instructor\",\"affiliation\":\"Example University\"}")"
	if [ "$(response_status "$created")" != "201" ]; then
		echo "Active Sysadmin could not create an Instructor Account" >&2
		exit 1
	fi
	created_account_id="$(python3 -c 'import json, sys; print(json.loads(sys.argv[1])["account"]["id"])' "$(response_body "$created")")"
	assert_summary "$(assert_created_account "$(response_body "$created")")" "$created_account_id" active

	deactivated="$(request "/api/instructor-accounts/$created_account_id/deactivate" "$sysadmin_cookie" POST '{"reason":"Live demo access review"}')"
	if [ "$(response_status "$deactivated")" != "200" ]; then
		echo "Active Sysadmin could not deactivate an Instructor Account" >&2
		exit 1
	fi
	assert_summary "$(response_body "$deactivated")" "$created_account_id" deactivated
	reactivated="$(request "/api/instructor-accounts/$created_account_id/reactivate" "$sysadmin_cookie" POST '{}')"
	if [ "$(response_status "$reactivated")" != "200" ]; then
		echo "Active Sysadmin could not reactivate an Instructor Account" >&2
		exit 1
	fi
	assert_summary "$(response_body "$reactivated")" "$created_account_id" active

	# The current seeded configuration has no deactivated Sysadmin persona.  Do
	# not mutate private state merely to fabricate one; deactivated-Sysadmin
	# concealment is therefore structurally unobservable in this disposable run.
	preservation_before="$(instructor_preservation_snapshot "$existing_account_id")"
	deactivated="$(request "/api/instructor-accounts/$existing_account_id/deactivate" "$sysadmin_cookie" POST '{"reason":"Live demo session revocation check"}')"
	if [ "$(response_status "$deactivated")" != "200" ]; then
		echo "Instructor session revocation setup failed" >&2
		exit 1
	fi
	if [ "$(response_status "$(request '/api/course-instances' "$instructor_cookie")")" = "200" ]; then
		echo "Deactivated Instructor retained an Authenticated Session" >&2
		# This is intentionally a fixed sentinel: no response body or identity is emitted.
		request "/api/instructor-accounts/$existing_account_id/reactivate" "$sysadmin_cookie" POST '{}' >/dev/null || true
		exit 1
	fi
	if ! preservation_after="$(instructor_preservation_snapshot "$existing_account_id")"; then
		request "/api/instructor-accounts/$existing_account_id/reactivate" "$sysadmin_cookie" POST '{}' >/dev/null || true
		exit 1
	fi
	reactivated="$(request "/api/instructor-accounts/$existing_account_id/reactivate" "$sysadmin_cookie" POST '{}')"
	if [ "$(response_status "$reactivated")" != "200" ]; then
		echo "Instructor session revocation cleanup failed" >&2
		exit 1
	fi
	assert_deactivation_preserves_records "$preservation_before" "$preservation_after"
	assert_catalog_least_privilege
	echo "Instructor Account authority: Sysadmin lifecycle, concealment, and session revocation complete"
}

prove_browser() {
	local port
	port="$(gateway_port)"
	NODE_EXTRA_CA_CERTS="$repository_root/local_stack_state/live_demo_browser/workspace/gateway-root.crt" \
	PLE_LOCAL_DEMO_TOTP_SETUP_FILE="${PLE_LOCAL_DEMO_TOTP_SETUP_FILE:-$repository_root/local_stack_state/live_demo_browser/workspace/morgan-totp-setup-uri}" \
		node --import tsx tests/playwright/e2e_live_demo_instructor_accounts_browser.mjs "$port"
	echo "Instructor Account browser: visible Sysadmin account lifecycle complete"
}

require_live_demo
case "$mode" in
	service) prove_service ;;
	browser) prove_browser ;;
	all) prove_service; prove_browser ;;
esac
echo "Live Demo Instructor Accounts: PASS"
