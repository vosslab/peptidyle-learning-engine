#!/usr/bin/env bash
# Disposable acceptance: current Course Appearance read/write authorization.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly workspace="local_stack_state/live_demo_browser/workspace"
readonly runtime_environment_path="$workspace/env.local"
readonly report_path="$workspace/live_demo_course_report.json"

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

fail() { echo "Course Appearance evidence: $*" >&2; exit 1; }

require_live_demo() {
	[ -f "$runtime_environment_path" ] || fail "fixed Live Demo environment is unavailable"
}

service_id() {
	local service="$1" identifiers
	identifiers="$(podman ps -q --filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=$service")"
	[ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = "1" ] ||
		fail "expected one running $service service"
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entry
	entry="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	[ "$(printf '%s\n' "$entry" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = "1" ] ||
		fail "expected one validated gateway port"
	printf '%s\n' "${entry#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1" cookie="${2:-}" method="${3:-GET}" body="${4:-}"
	local gateway port
	gateway="$(service_id gateway)"
	port="$(gateway_port)"
	local -a arguments=(--silent --show-error --insecure --max-time 12
		--write-out $'\n__PLE_STATUS__%{http_code}\n__PLE_CACHE__%header{cache-control}'
		--header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then
		arguments+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
	fi
	[ -z "$cookie" ] || arguments+=(--header "Cookie: $cookie")
	[ -z "$body" ] || arguments+=(--data "$body")
	podman exec "$gateway" curl "${arguments[@]}" "https://localhost:8080$path"
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
	[ -n "$cookie" ] || fail "seeded demo did not issue an Authenticated Session"
	printf '%s\n' "$cookie"
}

status() { printf '%s\n' "$1" | sed -n 's/^__PLE_STATUS__//p' | tail -n 1; }
cache_control() { printf '%s\n' "$1" | sed -n 's/^__PLE_CACHE__//p' | tail -n 1; }
body() { printf '%s\n' "$1" | sed '/^__PLE_STATUS__/,$d'; }

assert_no_store() {
	local response="$1" expected="$2"
	[ "$(status "$response")" = "$expected" ] || fail "expected HTTP $expected, received $(status "$response")"
	[ "$(cache_control "$response")" = "no-store" ] ||
		fail "response did not carry exact Cache-Control: no-store"
}

assert_appearance() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1]); expected=sys.argv[2]
if set(value)!={"theme","banner"} or value.get("theme") != expected or value.get("banner") is not None:
    raise SystemExit("Course Appearance response is not the closed expected aggregate")
' "$1" "$2"
}

assert_course_summary() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); course_id, reference, role=sys.argv[2:]
if set(value)!={"id","reference","title","term","role"}:
    raise SystemExit("Course Summary response is not closed")
if value.get("id") != course_id or value.get("reference") != reference or value.get("role") != role:
    raise SystemExit("Course Summary did not retain exact identity and caller membership role")
if not isinstance(value.get("title"),str) or not value["title"].strip():
    raise SystemExit("Course Summary title is invalid")
term=value.get("term")
if not isinstance(term,dict) or set(term)!={"startDate","endDate"}:
    raise SystemExit("Course Summary term is invalid")
if not all(isinstance(term.get(key),str) and term[key] for key in term):
    raise SystemExit("Course Summary term fields are invalid")
' "$1" "$2" "$3" "$4"
}

assert_same_course_summary_identity() {
	python3 -c '
import json, sys
left=json.loads(sys.argv[1]); right=json.loads(sys.argv[2])
for key in ("id","reference","title","term"):
    if left.get(key) != right.get(key):
        raise SystemExit("Instructor and enrolled Student Course Summaries differ")
' "$1" "$2"
}

assert_course_navigation() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1]); expected=sys.argv[2]
if value != {"kind":"course", "courseId":expected}:
    raise SystemExit("Course navigation response is not the exact Course resolution")
' "$1" "$2"
}

appearance_theme() {
	python3 -c '
import json, sys
value=json.loads(sys.argv[1])
if set(value)!={"theme","banner"} or value.get("banner") is not None or not isinstance(value.get("theme"),str):
    raise SystemExit("Course Appearance response is not a closed current theme aggregate")
print(value["theme"])
' "$1"
}

theme_payload() {
	python3 -c 'import json, sys; print(json.dumps({"theme":sys.argv[1]}, separators=(",",":")))' "$1"
}

cleanup_synthetic_sessions() {
	local cookie token token_hash postgres sql observer_end_event_id cleanup_failed=0
	if ! postgres="$(service_id postgres)"; then
		echo "Course Appearance evidence: synthetic fixture cleanup could not resolve PostgreSQL" >&2
		return 1
	fi
	for cookie in "${observer_cookie:-}" "${inactive_member_cookie:-}"; do
		[ -n "$cookie" ] || continue
		token="${cookie#*=}"
		if ! token_hash="$(python3 -c 'import base64, hashlib, sys; print(hashlib.sha256(base64.urlsafe_b64decode(sys.argv[1] + "=" * (-len(sys.argv[1]) % 4))).hexdigest())' "$token")"; then
			echo "Course Appearance evidence: synthetic session hash cleanup failed" >&2
			cleanup_failed=1
			continue
		fi
		sql="UPDATE ple_private.authenticated_session SET revoked_at = pg_catalog.clock_timestamp() WHERE token_hash = decode('$token_hash', 'hex') AND revoked_at IS NULL;"
		if ! podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "$1"' sh "$sql" >/dev/null; then
			echo "Course Appearance evidence: synthetic session revocation failed" >&2
			cleanup_failed=1
		fi
	done
	if [ -n "${observer_account_id:-}" ] && [ -n "${course:-}" ]; then
		if ! observer_end_event_id="$(python3 -c 'import uuid; print(uuid.uuid4())')"; then
			echo "Course Appearance evidence: observer relationship cleanup identity failed" >&2
			return 1
		fi
		sql="INSERT INTO ple_private.course_observer_relationship_event (course_observer_relationship_event_id, course_id, course_observer_account_id, recorded_by_account_id, event_kind, occurred_at) SELECT '$observer_end_event_id', '$course', '$observer_account_id', email.account_id, 'ended', pg_catalog.clock_timestamp() FROM ple_private.account_authentication_email AS email WHERE email.normalized_email = 'elena.rivera@live-demo.invalid' ON CONFLICT (course_id, course_observer_account_id, event_kind) DO NOTHING;"
		if ! podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "$1"' sh "$sql" >/dev/null; then
			echo "Course Appearance evidence: observer relationship end event failed" >&2
			cleanup_failed=1
		fi
	fi
	return "$cleanup_failed"
}

restore_original_theme_on_exit() {
	local exit_status=$? cleanup_status=0
	trap - EXIT
	if [ "${appearance_cleanup_armed:-0}" = "1" ]; then
		# Theme restoration protects the shared fixture but cannot mask the
		# original test result or replace the authoritative fixture cleanup.
		if ! request "$path" "$instructor_cookie" PUT "$(theme_payload "$original_theme")" >/dev/null; then
			echo "Course Appearance evidence: theme restoration was unavailable" >&2
		fi
	fi
	if ! cleanup_synthetic_sessions; then
		cleanup_status=1
	fi
	[ "$exit_status" -ne 0 ] && exit "$exit_status"
	exit "$cleanup_status"
}

course_reference() {
	python3 -c '
import json, re, sys
value=json.load(open(sys.argv[1], encoding="utf-8")); reference=value.get("course_reference")
if not isinstance(reference,str) or not re.fullmatch(r"C-[1-9][0-9]{0,9}",reference):
    raise SystemExit("seeded Course report has no public Course reference")
print(reference)
' "$report_path"
}

course_uuid() {
	local reference="$1" postgres sql output
	postgres="$(service_id postgres)"
	sql="SELECT course_id FROM ple_data.course_instance WHERE reference_number = ${reference#C-};"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	[ "$(printf '%s\n' "$output" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = "1" ] ||
		fail "could not resolve exactly one internal Course identity for accepted setup"
	printf '%s\n' "$output"
}

find_foreign_course_uuid() {
	local postgres sql output
	postgres="$(service_id postgres)"
	sql="SELECT course.course_id FROM ple_data.course_instance AS course WHERE course.course_title = 'M3 foreign Instructor Course' AND NOT EXISTS (SELECT 1 FROM ple_data.course_membership AS membership JOIN ple_private.account_authentication_email AS email ON email.account_id = membership.account_id WHERE membership.course_id = course.course_id AND email.normalized_email = 'elena.rivera@live-demo.invalid' AND ple_data.course_membership_is_active(membership.membership_id)) ORDER BY course.reference_number DESC LIMIT 1;"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	[ "$(printf '%s\n' "$output" | sed '/^$/d' | wc -l | tr -d '[:space:]')" -le "1" ] ||
		fail "foreign-Instructor Course discovery was ambiguous"
	printf '%s\n' "$output"
}

foreign_course_reference() {
	local course="$1" postgres sql output
	postgres="$(service_id postgres)"
	sql="SELECT 'C-' || reference_number FROM ple_data.course_instance WHERE course_id = '$course';"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	[ "$(printf '%s\n' "$output" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = "1" ] ||
		fail "could not resolve exactly one foreign Course reference"
	printf '%s\n' "$output"
}

synthetic_concealed_cookie() {
	local kind="$1" course="$2" supplied_account_id="${3:-}" postgres account_id membership_id session_id token_pair token token_hash observer_start_event_id inactive_membership_end_event_id sql
	case "$kind" in
		observer)
			[ -n "$supplied_account_id" ] || fail "observer fixture needs one Account identity"
			account_id="$supplied_account_id"
			membership_id=""
			;;
		inactive_member)
			read -r account_id membership_id inactive_membership_end_event_id <<<"$(python3 -c 'import uuid; print(uuid.uuid4(), uuid.uuid4(), uuid.uuid4())')"
			;;
		*) fail "unknown synthetic concealed persona" ;;
	esac
	token_pair="$(python3 -c 'import base64, hashlib, secrets, uuid; raw=secrets.token_bytes(32); print(base64.urlsafe_b64encode(raw).decode().rstrip("="), hashlib.sha256(raw).hexdigest(), uuid.uuid4(), uuid.uuid4())')"
	read -r token token_hash session_id observer_start_event_id <<<"$token_pair"
	postgres="$(service_id postgres)"
	if [ "$kind" = observer ]; then
		sql="INSERT INTO ple_private.account (account_id, product_role, created_at) VALUES ('$account_id', 'instructor', pg_catalog.clock_timestamp()) ON CONFLICT (account_id) DO NOTHING; INSERT INTO ple_private.course_observer_relationship_event (course_observer_relationship_event_id, course_id, course_observer_account_id, recorded_by_account_id, event_kind, occurred_at) SELECT '$observer_start_event_id', '$course', '$account_id', email.account_id, 'started', pg_catalog.clock_timestamp() FROM ple_private.account_authentication_email AS email WHERE email.normalized_email = 'elena.rivera@live-demo.invalid' ON CONFLICT (course_id, course_observer_account_id, event_kind) DO NOTHING;"
	else
		sql="INSERT INTO ple_private.account (account_id, product_role, created_at) VALUES ('$account_id', 'instructor', pg_catalog.clock_timestamp()) ON CONFLICT (account_id) DO NOTHING; INSERT INTO ple_data.course_membership (membership_id, course_id, account_id, role, joined_at, student_record_id) VALUES ('$membership_id', '$course', '$account_id', 'instructor', pg_catalog.clock_timestamp(), NULL) ON CONFLICT (membership_id) DO NOTHING; INSERT INTO ple_data.course_membership_event (course_membership_event_id, membership_id, event_kind, occurred_at, reason) VALUES ('$inactive_membership_end_event_id', '$membership_id', 'ended', pg_catalog.clock_timestamp(), 'disposable Course Summary refusal fixture');"
	fi
	sql="$sql INSERT INTO ple_private.authenticated_session (session_id, account_id, product_role, token_hash, created_at, expires_at) VALUES ('$session_id', '$account_id', 'instructor', decode('$token_hash', 'hex'), pg_catalog.clock_timestamp(), '2100-01-01 00:00:00+00') ON CONFLICT (token_hash) DO NOTHING;"
	podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "$1"' sh "$sql" >/dev/null
	printf '__Host-ple_session=%s\n' "$token"
}

require_live_demo
# The seeded Course is the required Instructor-plus-enrolled-Student fixture.
bash "$repository_root/tests/e2e/e2e_live_demo_course_seed.sh" --state >/dev/null
# Course-state convergence writes the report on a first use; only then is it a
# valid setup identity for the read-only UUID lookup below.
[ -f "$report_path" ] || fail "seeded Course report was not produced"
# This establishes an exact foreign Course without relying on a hidden session
# or direct data mutation; the later appearance calls remain ordinary HTTP.
foreign_course="$(find_foreign_course_uuid)"
if [ -z "$foreign_course" ]; then
	bash "$repository_root/tests/e2e/e2e_live_demo_assignment_list.sh" >/dev/null
	foreign_course="$(find_foreign_course_uuid)"
	[ -n "$foreign_course" ] || fail "foreign-Instructor Course prerequisite was not persisted"
fi
foreign_reference="$(foreign_course_reference "$foreign_course")"

instructor_cookie="$(persona_cookie elenaInstructor)"
student_cookie="$(persona_cookie maryStudent)"
sysadmin_cookie="$(persona_cookie morganSysadmin)"
course="$(course_uuid "$(course_reference)")"
trap restore_original_theme_on_exit EXIT
observer_account_id="$(python3 -c 'import uuid; print(uuid.uuid4())')"
observer_cookie="$(synthetic_concealed_cookie observer "$course" "$observer_account_id")"
inactive_member_cookie="$(synthetic_concealed_cookie inactive_member "$course")"
path="/api/courses/$course/appearance"
summary_path="/api/courses/$course"
navigation_path="/api/navigation/$(course_reference)"

instructor_summary="$(request "$summary_path" "$instructor_cookie")"
student_summary="$(request "$summary_path" "$student_cookie")"
assert_no_store "$instructor_summary" 200
assert_no_store "$student_summary" 200
assert_course_summary "$(body "$instructor_summary")" "$course" "$(course_reference)" instructor
assert_course_summary "$(body "$student_summary")" "$course" "$(course_reference)" student
assert_same_course_summary_identity "$(body "$instructor_summary")" "$(body "$student_summary")"

instructor_navigation="$(request "$navigation_path" "$instructor_cookie")"
student_navigation="$(request "$navigation_path" "$student_cookie")"
assert_no_store "$instructor_navigation" 200
assert_no_store "$student_navigation" 200
assert_course_navigation "$(body "$instructor_navigation")" "$course"
assert_course_navigation "$(body "$student_navigation")" "$course"

instructor_read="$(request "$path" "$instructor_cookie")"
student_read="$(request "$path" "$student_cookie")"
assert_no_store "$instructor_read" 200
assert_no_store "$student_read" 200
original_theme="$(appearance_theme "$(body "$instructor_read")")"
assert_appearance "$(body "$student_read")" "$original_theme"
if [ "$original_theme" = "forest" ]; then
	target_theme="ocean"
else
	target_theme="forest"
fi

anonymous="$(request "$path")"
sysadmin="$(request "$path" "$sysadmin_cookie")"
foreign="$(request "/api/courses/$foreign_course/appearance" "$instructor_cookie")"
assert_no_store "$anonymous" 404
assert_no_store "$sysadmin" 404
assert_no_store "$foreign" 404
[ "$(body "$anonymous")" = "$(body "$sysadmin")" ] && [ "$(body "$anonymous")" = "$(body "$foreign")" ] ||
	fail "anonymous, nonmember, and foreign-Instructor responses differ"

summary_anonymous="$(request "$summary_path")"
summary_sysadmin="$(request "$summary_path" "$sysadmin_cookie")"
summary_foreign="$(request "/api/courses/$foreign_course" "$instructor_cookie")"
summary_observer="$(request "$summary_path" "$observer_cookie")"
summary_inactive_member="$(request "$summary_path" "$inactive_member_cookie")"
assert_no_store "$summary_anonymous" 404
assert_no_store "$summary_sysadmin" 404
assert_no_store "$summary_foreign" 404
assert_no_store "$summary_observer" 404
assert_no_store "$summary_inactive_member" 404
[ "$(body "$summary_anonymous")" = "$(body "$summary_sysadmin")" ] && \
	[ "$(body "$summary_anonymous")" = "$(body "$summary_foreign")" ] && \
	[ "$(body "$summary_anonymous")" = "$(body "$summary_observer")" ] && \
	[ "$(body "$summary_anonymous")" = "$(body "$summary_inactive_member")" ] ||
fail "Course Summary concealed caller responses differ"

navigation_anonymous="$(request "$navigation_path")"
navigation_sysadmin="$(request "$navigation_path" "$sysadmin_cookie")"
navigation_foreign="$(request "/api/navigation/$foreign_reference" "$instructor_cookie")"
navigation_observer="$(request "$navigation_path" "$observer_cookie")"
navigation_inactive_member="$(request "$navigation_path" "$inactive_member_cookie")"
navigation_malformed="$(request "/api/navigation/C-0" "$instructor_cookie")"
navigation_non_course="$(request "/api/navigation/R-1" "$instructor_cookie")"
for response in "$navigation_anonymous" "$navigation_sysadmin" "$navigation_foreign" \
	"$navigation_observer" "$navigation_inactive_member" "$navigation_malformed" "$navigation_non_course"; do
	assert_no_store "$response" 404
done
[ "$(body "$navigation_anonymous")" = "$(body "$navigation_sysadmin")" ] && \
	[ "$(body "$navigation_anonymous")" = "$(body "$navigation_foreign")" ] && \
	[ "$(body "$navigation_anonymous")" = "$(body "$navigation_observer")" ] && \
	[ "$(body "$navigation_anonymous")" = "$(body "$navigation_inactive_member")" ] && \
	[ "$(body "$navigation_anonymous")" = "$(body "$navigation_malformed")" ] && \
	[ "$(body "$navigation_anonymous")" = "$(body "$navigation_non_course")" ] ||
	fail "Course navigation concealed caller responses differ"

updated="$(request "$path" "$instructor_cookie" PUT "$(theme_payload "$target_theme")")"
assert_no_store "$updated" 200
appearance_cleanup_armed=1
assert_appearance "$(body "$updated")" "$target_theme"
reloaded="$(request "$path" "$student_cookie")"
assert_no_store "$reloaded" 200
assert_appearance "$(body "$reloaded")" "$target_theme"

student_write="$(request "$path" "$student_cookie" PUT "$(theme_payload ocean)")"
assert_no_store "$student_write" 404
unknown_theme="$(request "$path" "$instructor_cookie" PUT "$(theme_payload unreviewed)")"
assert_no_store "$unknown_theme" 422
after_refusal="$(request "$path" "$instructor_cookie")"
assert_no_store "$after_refusal" 200
assert_appearance "$(body "$after_refusal")" "$target_theme"

restored="$(request "$path" "$instructor_cookie" PUT "$(theme_payload "$original_theme")")"
assert_no_store "$restored" 200
assert_appearance "$(body "$restored")" "$original_theme"
student_restored="$(request "$path" "$student_cookie")"
assert_no_store "$student_restored" 200
assert_appearance "$(body "$student_restored")" "$original_theme"
appearance_cleanup_armed=0

echo "Course Summary: authorized Instructor and enrolled Student reads with caller roles and concealed refusal boundaries complete"
echo "Course Appearance: authorized Instructor persistence, enrolled Student visibility, and concealed refusal boundaries complete"
echo "Live Demo Course Appearance: PASS"
