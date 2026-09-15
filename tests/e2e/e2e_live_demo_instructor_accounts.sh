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
if not isinstance(page,dict) or set(page)!={"accounts","displayTimeZone"}:
	raise SystemExit("Instructor Account list envelope is not closed")
if not isinstance(page["displayTimeZone"],str) or not page["displayTimeZone"]:
	raise SystemExit("Instructor Account list lacks its Sysadmin viewer zone")
items=page["accounts"]
if not isinstance(items,list) or not items:
    raise SystemExit("Instructor Account list is not a nonempty array")
for item in items:
    if not isinstance(item,dict) or set(item)!={"reference","state","lastSuccessfulSignIn"}:
        raise SystemExit("Instructor Account list projection is not closed")
    if not isinstance(item["reference"],str) or not re.fullmatch(r"U-[1-9][0-9]{0,9}",item["reference"]):
        raise SystemExit("Instructor Account reference is malformed")
    if item["state"] not in {"active","deactivated","closed"}:
        raise SystemExit("Instructor Account state is malformed")
    if item["lastSuccessfulSignIn"] is not None and (not isinstance(item["lastSuccessfulSignIn"],int) or isinstance(item["lastSuccessfulSignIn"],bool)):
        raise SystemExit("Instructor Account sign-in projection is malformed")
' "$1"
}

active_signed_in_instructor_reference() {
	python3 -c '
import json, re, sys
page=json.loads(sys.argv[1]); items=page["accounts"]
matches=[item.get("reference") for item in items if isinstance(item,dict) and item.get("state")=="active" and isinstance(item.get("lastSuccessfulSignIn"),int) and isinstance(item.get("reference"),str) and re.fullmatch(r"U-[1-9][0-9]{0,9}",item["reference"])]
if len(matches) != 1:
    raise SystemExit("Live Demo did not retain one observable active Instructor session")
print(matches[0])
' "$1"
}

assert_summary() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); expected_reference=sys.argv[2]; expected_state=sys.argv[3]
if not isinstance(value,dict) or set(value)!={"reference","state","lastSuccessfulSignIn"}:
    raise SystemExit("Instructor Account lifecycle projection is not closed")
if value.get("reference") != expected_reference or not re.fullmatch(r"U-[1-9][0-9]{0,9}", expected_reference):
    raise SystemExit("Instructor Account lifecycle changed its public identity")
if value.get("state") != expected_state or value.get("lastSuccessfulSignIn") is not None:
    raise SystemExit("Instructor Account lifecycle did not retain the expected state")
' "$1" "$2" "$3"
}

assert_vetting_receipt() {
	python3 -c '
import json, re, sys
receipt=json.loads(sys.argv[1])
if not isinstance(receipt, dict) or set(receipt) != {"vettingDecisionReference"}:
    raise SystemExit("Instructor identity vetting receipt is not closed")
reference=receipt["vettingDecisionReference"]
if not isinstance(reference, str) or not re.fullmatch(
    r"[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}", reference
):
    raise SystemExit("Instructor identity vetting receipt is not an opaque UUID")
print(reference)
' "$1"
}

instructor_account_count_for_email() {
	local email="$1" postgres
	postgres="$(service_id postgres)"
	podman exec "$postgres" sh -lc \
		'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh \
		"SELECT count(*) FROM ple_private.account_authentication_email AS email
          JOIN ple_private.account AS account ON account.account_id = email.account_id
         WHERE email.normalized_email = '$email' AND account.product_role = 'instructor'"
}

assert_no_instructor_account_for_email() {
	if [ "$(instructor_account_count_for_email "$1")" != "0" ]; then
		echo "Rejected vetting did not prevent Instructor capability creation" >&2
		exit 1
	fi
}

assert_creation_audit_link() {
	local email="$1" decision="$2" postgres output
	postgres="$(service_id postgres)"
	output="$(podman exec "$postgres" sh -lc \
		'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh \
		"SELECT CASE WHEN count(*) = 1 THEN 'instructor_creation_vetting_audit_link' END
           FROM ple_audit.instructor_account_creation_event AS event
           JOIN ple_private.account_authentication_email AS email
             ON email.account_id = event.created_instructor_account_id
          WHERE email.normalized_email = '$email'
            AND event.vetting_decision_id = '$decision'::uuid")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^instructor_creation_vetting_audit_link$/p')" != "instructor_creation_vetting_audit_link" ]; then
		echo "Instructor Account creation did not retain its completed vetting audit link" >&2
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
	local reference="$1" reference_number postgres sql
	if [[ ! "$reference" =~ ^U-([1-9][0-9]{0,9})$ ]]; then
		echo "Instructor preservation evidence received an invalid reference" >&2
		exit 1
	fi
	reference_number="${reference#U-}"
	postgres="$(service_id postgres)"
	sql="WITH instructor AS (
    SELECT account_id FROM ple_private.account
     WHERE reference_number = $reference_number AND product_role = 'instructor'
), snapshot AS (
    SELECT
        (SELECT count(*) FROM ple_private.authoring_workspace AS workspace
          JOIN instructor ON instructor.account_id = workspace.owner_account_id) AS authored_workspaces,
        (SELECT count(*) FROM ple_data.blueprint_course AS blueprint
          JOIN instructor ON instructor.account_id = blueprint.owner_account_id) AS authored_blueprints,
        (SELECT count(*) FROM ple_data.question_ownership_event AS ownership
          JOIN instructor ON instructor.account_id = ownership.owner_account_id) AS authored_questions,
        (SELECT count(*) FROM ple_data.course_instance AS course
          JOIN instructor ON instructor.account_id = course.assigned_instructor_account_id) AS assigned_courses,
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
    'assignedCourses', assigned_courses,
    'instructorMemberships', instructor_memberships,
    'courseCreationHistory', course_creation_history,
    'rosterHistory', roster_history,
    'stateEvents', state_events
) FROM snapshot;"
	podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql"
}

assert_deactivation_preserves_records() {
	python3 -c '
import json, sys
before, after = (json.loads(value) for value in sys.argv[1:])
required = (
    "authoredWorkspaces", "authoredBlueprints", "authoredQuestions",
    "assignedCourses", "instructorMemberships", "courseCreationHistory", "rosterHistory",
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
	local sysadmin_cookie instructor_cookie student_cookie listed existing_reference created created_reference deactivated reactivated preservation_before preservation_after missing_email invalid_email mismatched_email approved_email mismatched_decision approved_decision repeated_decision vetting
	sysadmin_cookie="$(persona_cookie morganSysadmin)"
	instructor_cookie="$(persona_cookie elenaInstructor)"
	student_cookie="$(persona_cookie maryStudent)"

	assert_concealed "$(request '/api/instructor-accounts')"
	assert_concealed "$(request '/api/instructor-accounts' "$student_cookie")"
	assert_concealed "$(request '/api/instructor-accounts' "$instructor_cookie")"
	assert_concealed "$(request '/api/instructor-accounts/U-0/deactivate' "$sysadmin_cookie" POST '{"reason":"bounded"}')"
	assert_concealed "$(request '/api/instructor-accounts/not-a-reference/reactivate' "$sysadmin_cookie" POST '{}')"
	assert_concealed "$(request '/api/instructor-accounts/U-2147483647/reactivate' "$sysadmin_cookie" POST '{}')"
	assert_concealed "$(request '/api/instructor-identity-vetting-decisions' "$student_cookie" POST '{"normalizedEmail":"student-cannot-vet@example.invalid","verifiedInstructorDisplayName":"Student Cannot Vet"}')"
	# C10: platform account administration is Sysadmin-only. These are durable
	# authorization boundaries, not a proxy for the internal approval workflow.
	assert_concealed "$(request '/api/instructor-identity-vetting-decisions' "$instructor_cookie" POST '{"normalizedEmail":"instructor-cannot-vet@example.invalid","verifiedInstructorDisplayName":"Instructor Cannot Vet"}')"
	assert_concealed "$(request '/api/instructor-accounts' "$instructor_cookie" POST '{"normalizedEmail":"instructor-cannot-create@example.invalid","vettingDecisionReference":"00000000-0000-4000-8000-000000000001"}')"

	listed="$(request '/api/instructor-accounts' "$sysadmin_cookie")"
	if [ "$(response_status "$listed")" != "200" ]; then
		echo "Active Sysadmin could not list Instructor Accounts" >&2
		exit 1
	fi
	assert_list "$(response_body "$listed")"
	existing_reference="$(active_signed_in_instructor_reference "$(response_body "$listed")")"

	# Permanent C18 authorization contract: a rejected request must leave no
	# Instructor capability behind. If this fails, repair the creation
	# validation/transaction boundary; do not loosen these denial assertions.
	missing_email="m18-missing-${RANDOM}${RANDOM}@example.invalid"
	if [ "$(response_status "$(request '/api/instructor-accounts' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"$missing_email\"}")")" != "422" ]; then
		echo "Instructor creation without completed vetting was not denied" >&2
		exit 1
	fi
	assert_no_instructor_account_for_email "$missing_email"

	invalid_email="m18-invalid-${RANDOM}${RANDOM}@example.invalid"
	if [ "$(response_status "$(request '/api/instructor-accounts' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"$invalid_email\",\"vettingDecisionReference\":\"00000000-0000-4000-8000-000000000001\"}")")" != "422" ]; then
		echo "Instructor creation with an invalid vetting decision was not denied" >&2
		exit 1
	fi
	assert_no_instructor_account_for_email "$invalid_email"

	mismatched_email="m18-mismatch-${RANDOM}${RANDOM}@example.invalid"
	vetting="$(request '/api/instructor-identity-vetting-decisions' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"m18-vetted-${RANDOM}${RANDOM}@example.invalid\",\"verifiedInstructorDisplayName\":\"M18 Vetted Instructor\"}")"
	if [ "$(response_status "$vetting")" != "201" ]; then
		echo "Active Sysadmin could not record completed Instructor identity vetting" >&2
		exit 1
	fi
	mismatched_decision="$(assert_vetting_receipt "$(response_body "$vetting")")"
	if [ "$(response_status "$(request '/api/instructor-accounts' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"$mismatched_email\",\"vettingDecisionReference\":\"$mismatched_decision\"}")")" != "422" ]; then
		echo "Instructor creation with a mismatched vetting decision was not denied" >&2
		exit 1
	fi
	assert_no_instructor_account_for_email "$mismatched_email"

	approved_email="m18-approved-${RANDOM}${RANDOM}@example.invalid"
	vetting="$(request '/api/instructor-identity-vetting-decisions' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"$approved_email\",\"verifiedInstructorDisplayName\":\"M18 Approved Instructor\"}")"
	if [ "$(response_status "$vetting")" != "201" ]; then
		echo "Active Sysadmin could not record completed Instructor identity vetting" >&2
		exit 1
	fi
	approved_decision="$(assert_vetting_receipt "$(response_body "$vetting")")"
	vetting="$(request '/api/instructor-identity-vetting-decisions' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"$approved_email\",\"verifiedInstructorDisplayName\":\"M18 Approved Instructor\"}")"
	if [ "$(response_status "$vetting")" != "201" ]; then
		echo "Repeated completed Instructor identity vetting was not idempotent" >&2
		exit 1
	fi
	repeated_decision="$(assert_vetting_receipt "$(response_body "$vetting")")"
	if [ "$approved_decision" != "$repeated_decision" ]; then
		echo "Completed Instructor identity vetting did not retain one immutable decision" >&2
		exit 1
	fi

	created="$(request '/api/instructor-accounts' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"$approved_email\",\"vettingDecisionReference\":\"$approved_decision\"}")"
	if [ "$(response_status "$created")" != "201" ]; then
		echo "Active Sysadmin could not create an Instructor Account from completed vetting" >&2
		exit 1
	fi
	created_reference="$(python3 -c 'import json, sys; print(json.loads(sys.argv[1])["reference"])' "$(response_body "$created")")"
	assert_summary "$(response_body "$created")" "$created_reference" active
	assert_creation_audit_link "$approved_email" "$approved_decision"

	deactivated="$(request "/api/instructor-accounts/$created_reference/deactivate" "$sysadmin_cookie" POST '{"reason":"Live demo access review"}')"
	if [ "$(response_status "$deactivated")" != "200" ]; then
		echo "Active Sysadmin could not deactivate an Instructor Account" >&2
		exit 1
	fi
	assert_summary "$(response_body "$deactivated")" "$created_reference" deactivated
	reactivated="$(request "/api/instructor-accounts/$created_reference/reactivate" "$sysadmin_cookie" POST '{}')"
	if [ "$(response_status "$reactivated")" != "200" ]; then
		echo "Active Sysadmin could not reactivate an Instructor Account" >&2
		exit 1
	fi
	assert_summary "$(response_body "$reactivated")" "$created_reference" active

	# The current seeded configuration has no deactivated Sysadmin persona.  Do
	# not mutate private state merely to fabricate one; deactivated-Sysadmin
	# concealment is therefore structurally unobservable in this disposable run.
	preservation_before="$(instructor_preservation_snapshot "$existing_reference")"
	deactivated="$(request "/api/instructor-accounts/$existing_reference/deactivate" "$sysadmin_cookie" POST '{"reason":"Live demo session revocation check"}')"
	if [ "$(response_status "$deactivated")" != "200" ]; then
		echo "Instructor session revocation setup failed" >&2
		exit 1
	fi
	if [ "$(response_status "$(request '/api/course-instances' "$instructor_cookie")")" = "200" ]; then
		echo "Deactivated Instructor retained an Authenticated Session" >&2
		# This is intentionally a fixed sentinel: no response body or identity is emitted.
		request "/api/instructor-accounts/$existing_reference/reactivate" "$sysadmin_cookie" POST '{}' >/dev/null || true
		exit 1
	fi
	if ! preservation_after="$(instructor_preservation_snapshot "$existing_reference")"; then
		request "/api/instructor-accounts/$existing_reference/reactivate" "$sysadmin_cookie" POST '{}' >/dev/null || true
		exit 1
	fi
	reactivated="$(request "/api/instructor-accounts/$existing_reference/reactivate" "$sysadmin_cookie" POST '{}')"
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
	node tests/playwright/e2e_live_demo_instructor_accounts_browser.mjs "$port"
	echo "Instructor Account browser: visible Sysadmin account lifecycle complete"
}

require_live_demo
case "$mode" in
	service) prove_service ;;
	browser) prove_browser ;;
	all) prove_service; prove_browser ;;
esac
echo "Live Demo Instructor Accounts: PASS"
