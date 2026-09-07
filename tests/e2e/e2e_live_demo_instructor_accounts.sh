#!/usr/bin/env bash
# Disposable M16 acceptance: Sysadmin-only Instructor Account lifecycle.

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
items=json.loads(sys.argv[1])
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
items=json.loads(sys.argv[1])
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

prove_service() {
	local sysadmin_cookie instructor_cookie student_cookie listed existing_reference created created_reference deactivated reactivated
	sysadmin_cookie="$(persona_cookie morganSysadmin)"
	instructor_cookie="$(persona_cookie elenaInstructor)"
	student_cookie="$(persona_cookie maryStudent)"

	assert_concealed "$(request '/api/instructor-accounts')"
	assert_concealed "$(request '/api/instructor-accounts' "$student_cookie")"
	assert_concealed "$(request '/api/instructor-accounts' "$instructor_cookie")"
	assert_concealed "$(request '/api/instructor-accounts/U-0/deactivate' "$sysadmin_cookie" POST '{"reason":"bounded"}')"
	assert_concealed "$(request '/api/instructor-accounts/not-a-reference/reactivate' "$sysadmin_cookie" POST '{}')"
	assert_concealed "$(request '/api/instructor-accounts/U-2147483647/reactivate' "$sysadmin_cookie" POST '{}')"

	listed="$(request '/api/instructor-accounts' "$sysadmin_cookie")"
	if [ "$(response_status "$listed")" != "200" ]; then
		echo "Active Sysadmin could not list Instructor Accounts" >&2
		exit 1
	fi
	assert_list "$(response_body "$listed")"
	existing_reference="$(active_signed_in_instructor_reference "$(response_body "$listed")")"

	created="$(request '/api/instructor-accounts' "$sysadmin_cookie" POST "{\"normalizedEmail\":\"m16-live-demo-${RANDOM}${RANDOM}@example.invalid\"}")"
	if [ "$(response_status "$created")" != "201" ]; then
		echo "Active Sysadmin could not create an Instructor Account" >&2
		exit 1
	fi
	created_reference="$(python3 -c 'import json, sys; print(json.loads(sys.argv[1])["reference"])' "$(response_body "$created")")"
	assert_summary "$(response_body "$created")" "$created_reference" active

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
	reactivated="$(request "/api/instructor-accounts/$existing_reference/reactivate" "$sysadmin_cookie" POST '{}')"
	if [ "$(response_status "$reactivated")" != "200" ]; then
		echo "Instructor session revocation cleanup failed" >&2
		exit 1
	fi
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
