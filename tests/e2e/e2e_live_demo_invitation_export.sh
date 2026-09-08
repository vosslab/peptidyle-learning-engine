#!/usr/bin/env bash
# Disposable acceptance: protected pending-invitation export and attended-mailer dry run.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_invitation_export.sh [--route|--dry-run]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--route) mode="route" ;;
	--dry-run) mode="dry-run" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

umask 077
temporary_directory="$(mktemp -d "${TMPDIR:-/tmp}/ple-m18-export.XXXXXX")"
readonly temporary_directory
chmod 700 "$temporary_directory"
output_email_directory="$repository_root/output-email"
created_output_email_directory=false

cleanup() {
	if [ "$created_output_email_directory" = true ]; then
		rm -f -- "$output_email_directory"/m18-export.* "$output_email_directory/invitation_status.json" "$output_email_directory/sent_log.csv"
		rmdir "$output_email_directory" 2>/dev/null || true
	fi
	rm -rf -- "$temporary_directory"
}
trap cleanup EXIT

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Invitation export evidence requires the fixed Live Demo to be running" >&2
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
		echo "Invitation export evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Invitation export evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request_to_files() {
	local path="$1"
	local cookie="$2"
	local method="$3"
	local body="$4"
	local response_path="$5"
	local headers_path="$6"
	local gateway port
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	local -a curl_args=(--silent --show-error --insecure --max-time 12 --output - --dump-header -
		--write-out $'\n__PLE_M18_STATUS__%{http_code}' --header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then
		curl_args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
	fi
	if [ -n "$cookie" ]; then curl_args+=(--header "Cookie: $cookie"); fi
	if [ -n "$body" ]; then curl_args+=(--data "$body"); fi
	podman exec "$gateway" curl "${curl_args[@]}" "https://localhost:8080$path" | python3 -c '
import pathlib, sys
raw = sys.stdin.buffer.read()
prefix, marker, status = raw.rpartition(b"\n__PLE_M18_STATUS__")
if not marker or len(status) != 3 or not status.isdigit():
    raise SystemExit("Invitation export response did not carry a status")
parts = prefix.split(b"\r\n\r\n")
if len(parts) < 2:
    raise SystemExit("Invitation export response did not carry headers")
headers, body = parts[-2], parts[-1]
pathlib.Path(sys.argv[1]).write_bytes(body)
pathlib.Path(sys.argv[2]).write_bytes(headers + b"\r\n")
print(status.decode("ascii"))
' "$response_path" "$headers_path"
}

persona_cookie() {
	local persona="$1"
	local gateway port headers cookie status
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 \
		--dump-header - --output /dev/null --header "Host: localhost:$port" \
		--header "Origin: https://localhost:$port" --header 'Content-Type: application/json' \
		--request POST --data "{\"persona\":\"$persona\"}" \
		--write-out $'\n__PLE_M18_STATUS__%{http_code}' 'https://localhost:8080/api/auth/live-demo/accounts')"
	status="${headers##*$'\n__PLE_M18_STATUS__'}"
	cookie="$(printf '%s\n' "${headers%$'\n__PLE_M18_STATUS__'*}" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"
	if [ "$status" != "200" ] || [ -z "$cookie" ]; then
		echo "seeded demo did not issue an Authenticated Session" >&2
		exit 1
	fi
	printf '%s\n' "$cookie"
}

assert_status() {
	if [ "$1" != "$2" ]; then
		echo "Invitation export returned an unexpected HTTP status" >&2
		exit 1
	fi
}

assert_concealed() { assert_status "$1" "404"; }

assert_header() {
	local headers_path="$1"
	local name="$2"
	local expected="$3"
	if ! python3 -c '
import sys
path, name, expected = sys.argv[1:]
headers = {}
for line in open(path, encoding="iso-8859-1"):
    if ":" in line:
        key, value = line.split(":", 1)
        headers[key.strip().lower()] = value.strip()
raise SystemExit(0 if headers.get(name) == expected else 1)
' "$headers_path" "$name" "$expected"; then
		echo "Invitation export response headers were not the protected fixed shape" >&2
		exit 1
	fi
}

latest_course_reference() {
	local instructor_cookie="$1"
	local body_path="$temporary_directory/course-list.json"
	local headers_path="$temporary_directory/course-list.headers"
	local status
	status="$(request_to_files '/api/course-instances' "$instructor_cookie" GET '' "$body_path" "$headers_path")"
	assert_status "$status" "200"
	python3 -c '
import json, re, sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
items = value.get("items") if isinstance(value, dict) else None
references = [item.get("reference") for item in items or [] if isinstance(item, dict)]
if not references or any(not isinstance(ref, str) or not re.fullmatch(r"C-[1-9][0-9]{0,9}", ref) for ref in references):
    raise SystemExit("Course Instance list lacks public identities")
print(max(references, key=lambda ref: int(ref[2:])))
' "$body_path"
}

create_empty_course() {
	local instructor_cookie="$1"
	bash "$repository_root/tests/e2e/e2e_live_demo_course_instance.sh" --authority >/dev/null
	latest_course_reference "$instructor_cookie"
}

import_pending_invitation() {
	local course="$1"
	local instructor_cookie="$2"
	local body_path="$temporary_directory/roster-import.json"
	local headers_path="$temporary_directory/roster-import.headers"
	local status
	status="$(request_to_files "/api/course-instances/$course/roster" "$instructor_cookie" POST \
		'{"entries":[{"email":"m18-recipient@mail.roosevelt.edu","rosterId":"m18-pending"}]}' \
		"$body_path" "$headers_path")"
	assert_status "$status" "201"
	if ! python3 -c '
import json, sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
if not isinstance(value, list) or len(value) != 1:
    raise SystemExit(1)
row = value[0]
raise SystemExit(0 if isinstance(row, dict) and set(row) == {"rosterId", "rosterEmail", "state"} and row.get("state") == "invitationPending" else 1)
' "$body_path"; then
		echo "Course Roster did not create the required pending Course Invitation" >&2
		exit 1
	fi
}

assert_mailer_export_shape() {
	local body_path="$1"
	local expected_count="$2"
	if ! python3 -c '
import json, sys, urllib.parse
value = json.load(open(sys.argv[1], encoding="utf-8"))
if not isinstance(value, dict) or set(value) != {"course_name", "students"}:
    raise SystemExit(1)
if not isinstance(value["course_name"], str) or not value["course_name"].strip() or not isinstance(value["students"], list) or len(value["students"]) != int(sys.argv[2]):
    raise SystemExit(1)
for row in value["students"]:
    if not isinstance(row, dict) or set(row) != {"email", "signup_url", "roster_id"}:
        raise SystemExit(1)
    if not isinstance(row["email"], str) or not isinstance(row["roster_id"], str):
        raise SystemExit(1)
    signup = urllib.parse.urlsplit(row["signup_url"])
    if signup.scheme != "https" or not signup.hostname or signup.username or signup.password:
        raise SystemExit(1)
' "$body_path" "$expected_count"; then
		echo "Invitation export body was not the closed attended-mailer shape" >&2
		exit 1
	fi
}

assert_export_procedure_boundary() {
	local postgres output sql
	postgres="$(service_id postgres)"
	sql="DO \$\$
DECLARE
    v_export oid;
    v_course oid;
BEGIN
    SELECT proc.oid INTO v_export
      FROM pg_proc AS proc
      JOIN pg_namespace AS namespace ON namespace.oid = proc.pronamespace
     WHERE namespace.nspname = 'ple_api'
       AND proc.proname = 'export_live_demo_pending_course_invitations'
       AND pg_get_function_identity_arguments(proc.oid) = 'p_course_reference_number bigint';
    SELECT proc.oid INTO v_course
      FROM pg_proc AS proc
      JOIN pg_namespace AS namespace ON namespace.oid = proc.pronamespace
     WHERE namespace.nspname = 'ple_api'
       AND proc.proname = 'load_live_demo_invitation_export_course'
       AND pg_get_function_identity_arguments(proc.oid) = 'p_course_reference_number bigint';
    IF v_export IS NULL OR v_course IS NULL
       OR NOT has_function_privilege('ple_app', v_export, 'EXECUTE')
       OR NOT has_function_privilege('ple_app', v_course, 'EXECUTE')
       OR has_function_privilege('public', v_export, 'EXECUTE')
       OR has_function_privilege('public', v_course, 'EXECUTE')
       OR has_table_privilege('ple_app', 'ple_private.course_invitation', 'SELECT')
       OR has_table_privilege('ple_app', 'ple_private.course_roster_profile', 'SELECT')
       OR position('current_session_account_is_course_instructor' IN pg_get_functiondef(v_export)) = 0
       OR position('current_session_account_is_course_instructor' IN pg_get_functiondef(v_course)) = 0
    THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Invitation export membership procedure boundary is incomplete';
    END IF;
END
\$\$;
SELECT 'invitation_export_membership_authority';"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	if [ "$(printf '%s\n' "$output" | sed -n '/^invitation_export_membership_authority$/p')" != "invitation_export_membership_authority" ]; then
		echo "Invitation export membership procedure boundary was not present" >&2
		exit 1
	fi
}

download_export() {
	local course="$1"
	local cookie="$2"
	local body_path="$3"
	local headers_path="$4"
	request_to_files "/api/course-instances/$course/invitation-export" "$cookie" GET '' "$body_path" "$headers_path"
}

prove_route() {
	local instructor_cookie student_cookie sysadmin_cookie course empty_course body_path headers_path status
	instructor_cookie="$(persona_cookie elenaInstructor)"
	student_cookie="$(persona_cookie maryStudent)"
	sysadmin_cookie="$(persona_cookie morganSysadmin)"
	course="$(create_empty_course "$instructor_cookie")"
	import_pending_invitation "$course" "$instructor_cookie"

	body_path="$temporary_directory/authorized-export.json"
	headers_path="$temporary_directory/authorized-export.headers"
	status="$(download_export "$course" "$instructor_cookie" "$body_path" "$headers_path")"
	assert_status "$status" "200"
	assert_header "$headers_path" cache-control no-store
	assert_header "$headers_path" x-content-type-options nosniff
	assert_header "$headers_path" content-type application/json
	assert_header "$headers_path" content-disposition 'attachment; filename=ple-invitations.json'
	assert_mailer_export_shape "$body_path" 1
	assert_export_procedure_boundary

	for access in anonymous student sysadmin; do
		body_path="$temporary_directory/concealed-$access.json"
		headers_path="$temporary_directory/concealed-$access.headers"
		case "$access" in
			anonymous) status="$(download_export "$course" '' "$body_path" "$headers_path")" ;;
			student) status="$(download_export "$course" "$student_cookie" "$body_path" "$headers_path")" ;;
			sysadmin) status="$(download_export "$course" "$sysadmin_cookie" "$body_path" "$headers_path")" ;;
		esac
		assert_concealed "$status"
	done

	empty_course="$(create_empty_course "$instructor_cookie")"
	body_path="$temporary_directory/empty-export.json"
	headers_path="$temporary_directory/empty-export.headers"
	status="$(download_export "$empty_course" "$instructor_cookie" "$body_path" "$headers_path")"
	assert_status "$status" "200"
	assert_header "$headers_path" cache-control no-store
	assert_header "$headers_path" x-content-type-options nosniff
	assert_header "$headers_path" content-type application/json
	assert_header "$headers_path" content-disposition 'attachment; filename=ple-invitations.json'
	assert_mailer_export_shape "$body_path" 0
	echo "Invitation export route: direct Instructor attachment and concealed non-Instructor access complete"
}

prove_dry_run() {
	local instructor_cookie course export_path headers_path status mailer_output
	instructor_cookie="$(persona_cookie elenaInstructor)"
	course="$(create_empty_course "$instructor_cookie")"
	import_pending_invitation "$course" "$instructor_cookie"
	if [ -e "$output_email_directory" ]; then
		echo "Invitation export dry run requires no pre-existing output-email directory" >&2
		exit 2
	fi
	mkdir -m 700 "$output_email_directory"
	created_output_email_directory=true
	export_path="$(mktemp "$output_email_directory/m18-export.XXXXXX.json")"
	chmod 600 "$export_path"
	if ! python3 -c 'import os, stat, sys; raise SystemExit(0 if stat.S_IMODE(os.stat(sys.argv[1]).st_mode) == 0o600 else 1)' "$export_path"; then
		echo "Invitation export dry run did not create an owner-private export file" >&2
		exit 1
	fi
	headers_path="$temporary_directory/dry-run-export.headers"
	status="$(download_export "$course" "$instructor_cookie" "$export_path" "$headers_path")"
	assert_status "$status" "200"
	assert_header "$headers_path" cache-control no-store
	assert_header "$headers_path" x-content-type-options nosniff
	assert_header "$headers_path" content-type application/json
	assert_header "$headers_path" content-disposition 'attachment; filename=ple-invitations.json'
	assert_mailer_export_shape "$export_path" 1
	mailer_output="$temporary_directory/mailer-output.txt"
	if ! (source "$repository_root/source_me.sh" && python3 launchers/send_invitations.py "output-email/${export_path##*/}" --dry-run) >"$mailer_output" 2>&1; then
		if rg -q '^ERROR: recipient domain is not allowed:' "$mailer_output"; then
			echo "Invitation export did not satisfy the existing mailer recipient-domain policy" >&2
		fi
		echo "Invitation export was rejected by the existing dry-run mailer" >&2
		exit 1
	fi
	set +e
	python3 -c '
import json, re, sys
output = open(sys.argv[1], encoding="utf-8").read()
status = json.load(open(sys.argv[2], encoding="utf-8"))
if not re.fullmatch(r"dry-run 1/1 [^\s]+\nSummary: sent=0 failed=0 dry_run=1 already_sent=0 held_indeterminate=0\n", output):
    raise SystemExit(2)
if not isinstance(status, dict) or status.get("version") != 1 or not isinstance(status.get("cells"), list) or len(status["cells"]) != 1 or not isinstance(status["cells"][0], dict) or status["cells"][0].get("status") != "dry_run":
    raise SystemExit(3)

' "$mailer_output" "$output_email_directory/invitation_status.json"
	validation_status=$?
	set -e
	if [ "$validation_status" != "0" ]; then
		case "$validation_status" in
			2) echo "Invitation export dry run did not produce the fixed safe mailer summary" >&2 ;;
			3) echo "Invitation export dry run did not retain the expected no-delivery status" >&2 ;;
			*) echo "Invitation export dry run verification failed" >&2 ;;
		esac
		exit 1
	fi
	set -e
	echo "Invitation export mailer: owner-private dry run completed without Mail delivery"
}

require_live_demo
case "$mode" in
	route) prove_route ;;
	dry-run) prove_dry_run ;;
	all) prove_route; prove_dry_run ;;
esac

echo "Live Demo Invitation Export: PASS"
