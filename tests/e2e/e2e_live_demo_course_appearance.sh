#!/usr/bin/env bash
# Disposable acceptance: current Course Appearance read/write authorization.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly workspace="local_stack_state/live_demo_browser/workspace"
readonly runtime_environment_path="$workspace/env.local"
readonly live_demo_course_long_name="Biochemistry 301: Proteins and Peptides"

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
value=json.loads(sys.argv[1]); course_instance_id, role=sys.argv[2:]
if set(value)!={"classification","id","shortName","longName","term","role"}:
    raise SystemExit("Course Summary response is not closed")
if value.get("id") != course_instance_id or value.get("role") != role:
    raise SystemExit("Course Summary did not retain exact Course Instance ID and caller membership role")
for key in ("shortName", "longName"):
    if not isinstance(value.get(key),str) or not value[key].strip():
        raise SystemExit(f"Course Summary {key} is invalid")
term=value.get("term")
if not isinstance(term,dict) or set(term)!={"startDate","endDate"}:
    raise SystemExit("Course Summary term is invalid")
if not all(isinstance(term.get(key),str) and term[key] for key in term):
    raise SystemExit("Course Summary term fields are invalid")
' "$1" "$2" "$3"
}

assert_same_course_summary_identity() {
	python3 -c '
import json, sys
left=json.loads(sys.argv[1]); right=json.loads(sys.argv[2])
for key in ("id","shortName","longName","term","classification"):
    if left.get(key) != right.get(key):
        raise SystemExit("Instructor and enrolled Student Course Summaries differ")
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
	local cookie token token_hash postgres sql cleanup_failed=0
	if ! postgres="$(service_id postgres)"; then
		echo "Course Appearance evidence: synthetic fixture cleanup could not resolve PostgreSQL" >&2
		return 1
	fi
	for cookie in "${inactive_member_cookie:-}"; do
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

course_instance_id() {
	python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); long_name=sys.argv[2]
if not isinstance(value,dict) or set(value)!={"items","nextCursor"} or value["nextCursor"] is not None or not isinstance(value["items"],list):
    raise SystemExit("Course list is not the closed current projection")
matches=[item for item in value["items"] if isinstance(item,dict) and item.get("longName")==long_name]
if len(matches)!=1 or set(matches[0])!={"id","shortName","longName","term","theme"}:
    raise SystemExit("Live Demo Course is absent, duplicated, or malformed")
course_instance_id=matches[0]["id"]
if not isinstance(course_instance_id,str) or not re.fullmatch(r"CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}",course_instance_id):
    raise SystemExit("Live Demo Course has no canonical Course Instance ID")
print(course_instance_id)
' "$1" "$live_demo_course_long_name"
}

course_uuid() {
	local course_instance_id="$1" postgres sql output
	postgres="$(service_id postgres)"
	sql="SELECT course_instance_id FROM ple_data.course_instance WHERE course_instance_id = '$course_instance_id';"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	[ "$(printf '%s\n' "$output" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = "1" ] ||
		fail "could not resolve exactly one internal Course identity for accepted setup"
	printf '%s\n' "$output"
}

find_foreign_course_uuid() {
	local postgres sql output
	postgres="$(service_id postgres)"
	sql="SELECT course.course_instance_id FROM ple_data.course_instance AS course WHERE course.course_short_name = 'Foreign course' AND course.course_long_name = 'Foreign Instructor Course' AND NOT EXISTS (SELECT 1 FROM ple_data.course_membership AS membership JOIN ple_private.account_authentication_email AS email ON email.account_id = membership.account_id WHERE membership.course_instance_id = course.course_instance_id AND email.normalized_email = 'elena.rivera@live-demo.invalid' AND ple_data.course_membership_is_active(membership.course_membership_id)) ORDER BY course.course_instance_id DESC LIMIT 1;"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	[ "$(printf '%s\n' "$output" | sed '/^$/d' | wc -l | tr -d '[:space:]')" -le "1" ] ||
		fail "foreign-Instructor Course discovery was ambiguous"
	printf '%s\n' "$output"
}

foreign_course_instance_id() {
	local course="$1" postgres sql output
	postgres="$(service_id postgres)"
	sql="SELECT course_instance_id FROM ple_data.course_instance WHERE course_instance_id = '$course';"
	output="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "$sql")"
	[ "$(printf '%s\n' "$output" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = "1" ] ||
		fail "could not resolve exactly one foreign Course Instance ID"
	printf '%s\n' "$output"
}

synthetic_concealed_cookie() {
	local kind="$1" course="$2" postgres membership_id session_id token_pair token token_hash inactive_membership_end_event_id
	case "$kind" in
		inactive_member)
			read -r membership_id inactive_membership_end_event_id <<<"$(python3 -c 'import uuid; print(uuid.uuid4(), uuid.uuid4())')"
			;;
		*) fail "unknown synthetic concealed persona" ;;
	esac
	token_pair="$(python3 -c 'import base64, hashlib, secrets, uuid; raw=secrets.token_bytes(32); print(base64.urlsafe_b64encode(raw).decode().rstrip("="), hashlib.sha256(raw).hexdigest(), uuid.uuid4())')"
	read -r token token_hash session_id <<<"$token_pair"
	postgres="$(service_id postgres)"
	podman exec -i "$postgres" sh -lc 'exec psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB"' >/dev/null <<SQL
INSERT INTO ple_private.account (account_id, product_role, created_at)
VALUES ('U00000009', 'instructor', pg_catalog.clock_timestamp())
RETURNING account_id AS minted_account_id \\gset
INSERT INTO ple_data.course_membership (course_membership_id, course_instance_id, account_id, role, joined_at)
VALUES ('$membership_id', '$course', :'minted_account_id', 'instructor', pg_catalog.clock_timestamp());
INSERT INTO ple_data.course_membership_event (course_membership_event_id, course_membership_id, event_kind, occurred_at, reason)
VALUES ('$inactive_membership_end_event_id', '$membership_id', 'ended', pg_catalog.clock_timestamp(), 'disposable Course Summary refusal fixture');
INSERT INTO ple_private.authenticated_session (session_id, account_id, product_role, token_hash, created_at, expires_at)
VALUES ('$session_id', :'minted_account_id', 'instructor', decode('$token_hash', 'hex'), pg_catalog.clock_timestamp(), '2100-01-01 00:00:00+00')
ON CONFLICT (token_hash) DO NOTHING;
SQL
	printf '__Host-ple_session=%s\n' "$token"
}

require_live_demo
# The seeded Course is discovered through the Instructor's ordinary current projection.
bash "$repository_root/tests/e2e/e2e_live_demo_course_seed.sh" --state >/dev/null
# This establishes an exact foreign Course without relying on a hidden session
# or direct data mutation; the later appearance calls remain ordinary HTTP.
foreign_course="$(find_foreign_course_uuid)"
if [ -z "$foreign_course" ]; then
	bash "$repository_root/tests/e2e/e2e_live_demo_assignment_list.sh" >/dev/null
	foreign_course="$(find_foreign_course_uuid)"
	[ -n "$foreign_course" ] || fail "foreign-Instructor Course prerequisite was not persisted"
fi
foreign_course_id="$(foreign_course_instance_id "$foreign_course")"

instructor_cookie="$(persona_cookie elenaInstructor)"
student_cookie="$(persona_cookie maryStudent)"
sysadmin_cookie="$(persona_cookie morganSysadmin)"
course_list="$(request '/api/course-instances' "$instructor_cookie")"
assert_no_store "$course_list" 200
course_instance_id="$(course_instance_id "$(body "$course_list")")"
course="$(course_uuid "$course_instance_id")"
trap restore_original_theme_on_exit EXIT
inactive_member_cookie="$(synthetic_concealed_cookie inactive_member "$course")"
path="/api/course-instances/$course_instance_id/appearance"
summary_path="/api/course-instances/$course_instance_id/summary"
course_instance_path="/api/course-instances/$course_instance_id"

instructor_course_instance="$(request "$course_instance_path" "$instructor_cookie")"
student_course_instance="$(request "$course_instance_path" "$student_cookie")"
assert_no_store "$instructor_course_instance" 200
assert_no_store "$student_course_instance" 404

instructor_summary="$(request "$summary_path" "$instructor_cookie")"
student_summary="$(request "$summary_path" "$student_cookie")"
assert_no_store "$instructor_summary" 200
assert_no_store "$student_summary" 200
assert_course_summary "$(body "$instructor_summary")" "$course_instance_id" instructor
assert_course_summary "$(body "$student_summary")" "$course_instance_id" student
assert_same_course_summary_identity "$(body "$instructor_summary")" "$(body "$student_summary")"

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
foreign="$(request "/api/course-instances/$foreign_course_id/appearance" "$instructor_cookie")"
blueprint="$(request "/api/course-instances/BP7K3M2QAF/appearance" "$instructor_cookie")"
assert_no_store "$anonymous" 404
assert_no_store "$sysadmin" 404
assert_no_store "$foreign" 404
assert_no_store "$blueprint" 404
[ "$(body "$anonymous")" = "$(body "$sysadmin")" ] && \
	[ "$(body "$anonymous")" = "$(body "$foreign")" ] && \
	[ "$(body "$anonymous")" = "$(body "$blueprint")" ] ||
	fail "anonymous, nonmember, foreign-Instructor, and Blueprint responses differ"

summary_anonymous="$(request "$summary_path")"
summary_sysadmin="$(request "$summary_path" "$sysadmin_cookie")"
summary_foreign="$(request "/api/course-instances/$foreign_course_id/summary" "$instructor_cookie")"
summary_inactive_member="$(request "$summary_path" "$inactive_member_cookie")"
assert_no_store "$summary_anonymous" 404
assert_no_store "$summary_sysadmin" 404
assert_no_store "$summary_foreign" 404
assert_no_store "$summary_inactive_member" 404
[ "$(body "$summary_anonymous")" = "$(body "$summary_sysadmin")" ] && \
	[ "$(body "$summary_anonymous")" = "$(body "$summary_foreign")" ] && \
	[ "$(body "$summary_anonymous")" = "$(body "$summary_inactive_member")" ] ||
fail "Course Summary concealed caller responses differ"

updated="$(request "$path" "$instructor_cookie" PUT "$(theme_payload "$target_theme")")"
assert_no_store "$updated" 200
appearance_cleanup_armed=1
assert_appearance "$(body "$updated")" "$target_theme"
reloaded="$(request "$path" "$student_cookie")"
assert_no_store "$reloaded" 200
assert_appearance "$(body "$reloaded")" "$target_theme"

student_write="$(request "$path" "$student_cookie" PUT "$(theme_payload ocean)")"
assert_no_store "$student_write" 404
# This is the durable C32 authorization boundary: a Student can bypass every
# browser affordance and call the binary Course Banner staging route directly.
# The handler authorizes before it reads the body. This route outcome protects
# that server boundary; the PostgreSQL Course Banner saga independently
# protects exact Instructor Course Membership in the Store. Failure repairs the
# server role gate before release.
student_banner_upload="$(request "$path/banner-uploads" "$student_cookie" POST '{}')"
assert_no_store "$student_banner_upload" 404
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
