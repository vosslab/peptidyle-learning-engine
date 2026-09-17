#!/usr/bin/env bash
# Disposable acceptance: self-only Account avatar authorization and storage identity.
#
# This is permanent because a route or composition regression could let one Account
# obtain another Account's Profile image, or restore the deprecated Instructor-only
# thumbnail boundary. If it fails, repair the authorization/storage contract rather
# than weakening the concealment or exact-object assertions.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root project_name="ple-live-demo-browser"
readonly environment_path="local_stack_state/live_demo_browser/workspace/env.local"
readonly image_path="/tmp/ple-profile-avatar-e2e.png"

source "$repository_root/source_me.sh"
cd "$repository_root"

fail() { echo "Profile avatar acceptance: $*" >&2; exit 1; }
require_live_demo() { [ -f "$environment_path" ] || fail "requires the fixed Live Demo"; }
service_id() {
    local ids
    ids="$(podman ps -q --filter "label=com.docker.compose.project=$project_name" --filter "label=com.docker.compose.service=$1")"
    [ "$(printf '%s\n' "$ids" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = 1 ] || fail "expected one $1 service"
    printf '%s\n' "$ids"
}
gateway_port() {
    local value
    value="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$environment_path" || true)"
    [ "$(printf '%s\n' "$value" | sed '/^$/d' | wc -l | tr -d '[:space:]')" = 1 ] || fail "expected one gateway port"
    printf '%s\n' "${value#PLE_GATEWAY_HOST_PORT=}"
}
status() { printf '%s' "${1##*$'\n'}"; }
body() { printf '%s' "${1%$'\n'*}"; }
require_status() { [ "$(status "$2")" = "$3" ] || fail "$1 returned HTTP $(status "$2"), expected $3"; }
concealed() { require_status "concealed request" "$1" 404; }
request() {
    local path="$1" cookie="${2:-}" method="${3:-GET}" payload="${4:-}" gateway port
    gateway="$(service_id gateway)"; port="$(gateway_port)"
    local -a args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}'
        --header "Host: localhost:$port" --request "$method")
    [ "$method" = GET ] || args+=(--header "Origin: https://localhost:$port" --header 'Content-Type: application/json')
    [ -z "$cookie" ] || args+=(--header "Cookie: $cookie")
    [ -z "$payload" ] || args+=(--data "$payload")
    podman exec "$gateway" curl "${args[@]}" "https://localhost:8080$path"
}
upload() {
    local cookie="$1" gateway port
    gateway="$(service_id gateway)"; port="$(gateway_port)"
    podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}' \
        --header "Host: localhost:$port" --header "Origin: https://localhost:$port" \
        --header 'Content-Type: application/octet-stream' --header "Cookie: $cookie" \
        --header 'X-PLE-Profile-Crop: {"sourceWidth":128,"sourceHeight":128,"horizontal":50,"vertical":50,"zoomPercent":100}' \
        --request POST --data-binary "@$image_path" \
        'https://localhost:8080/api/profile/avatar/profile-image'
}
persona_cookie() {
    local persona="$1" gateway port headers cookie
    gateway="$(service_id gateway)"; port="$(gateway_port)"
    headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 --dump-header - --output /dev/null \
        --header "Host: localhost:$port" --header "Origin: https://localhost:$port" --header 'Content-Type: application/json' \
        --request POST --data "{\"persona\":\"$persona\"}" 'https://localhost:8080/api/auth/live-demo/accounts')"
    cookie="$(printf '%s\n' "$headers" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"
    [ -n "$cookie" ] || fail "Live Demo did not issue an authenticated session for $persona"
    printf '%s\n' "$cookie"
}
provided_avatar() {
    local postgres
    postgres="$(service_id postgres)"
    podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "SELECT provided_avatar_id FROM ple_data.provided_avatar ORDER BY provided_avatar_id LIMIT 1"' |
        rg -x '[a-z][a-z0-9-]{0,63}' || fail "fixture has no closed provided avatar"
}
image_reference() {
    python3 -c '
import json, re, sys
value=json.loads(sys.argv[1]); avatar=value.get("avatar")
if set(value) != {"avatar"} or not isinstance(avatar, dict) or set(avatar) != {"kind","profileImageId"}:
    raise SystemExit("Profile-image response has an invalid public shape")
reference=avatar["profileImageId"]
if avatar["kind"] != "profileImage" or not isinstance(reference,str) or not re.fullmatch(r"[0-9a-f-]{36}", reference):
    raise SystemExit("Profile-image response lacks an opaque image reference")
print(reference)
' "$1"
}
assert_exact_record() {
    local persona="$1" image="$2" postgres result
    postgres="$(service_id postgres)"
    result="$(podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "
SELECT record.object_address::text
  FROM ple_private.account AS account
  JOIN ple_private.account_avatar AS avatar ON avatar.account_id = account.account_id
  JOIN ple_data.profile_image_delivery AS delivery ON delivery.profile_image_id = avatar.profile_image_id
  JOIN ple_private.object_record AS record ON record.object_id = delivery.object_id
 WHERE account.display_name = '$persona' AND avatar.profile_image_id = '$image'")"
    python3 -c '
import json, sys
value=json.loads(sys.argv[1]); image=sys.argv[2]
if value != {"kind":"profileImage", "image":image, "object":value.get("object")} or not isinstance(value["object"], str):
    raise SystemExit("Profile image did not retain its exact typed object address")
' "$result" "$image"
}
assert_retired_exact_cleanup() {
    local image="$1" postgres
    postgres="$(service_id postgres)"
    podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "$1"' sh "
SELECT CASE WHEN count(*) = 1 AND min(state) = 'completed' THEN 'retired-exact-cleanup' END
  FROM ple_private.profile_image_work
 WHERE profile_image_id = '$image' AND operation_kind = 'delete';" |
        rg -qx 'retired-exact-cleanup' || fail "replacement did not finish exact retired-image cleanup"
}
prepare_image() {
    local gateway image_base64="${1:-iVBORw0KGgoAAAANSUhEUgAAAIAAAACACAIAAABMXPacAAAARElEQVR4nO3BAQEAAACAkP6v7ggKAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAYwIAAAWMWdQAAAAAASUVORK5CYII=}"
    gateway="$(service_id gateway)"
    podman exec "$gateway" sh -lc "printf '%s' '$image_base64' | base64 -d > '$image_path'"
}

require_live_demo
prepare_image
trap 'podman exec "$(service_id gateway)" rm -f "$image_path" >/dev/null 2>&1 || true' EXIT

instructor_cookie="$(persona_cookie elenaInstructor)"
student_cookie="$(persona_cookie maryStudent)"
sysadmin_cookie="$(persona_cookie morganSysadmin)"
provided="$(provided_avatar)"

concealed "$(request '/api/profile/avatar')"
student_selected="$(request '/api/profile/avatar' "$student_cookie" PUT "{\"providedAvatarId\":\"$provided\"}")"
require_status "Student provided-avatar selection" "$student_selected" 204
student_upload="$(upload "$student_cookie")"
require_status "Student Profile-image upload denial" "$student_upload" 403

prepare_image 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScL6AAAAAElFTkSuQmCC'
small_upload="$(upload "$instructor_cookie")"
require_status "Undersized Profile-image upload" "$small_upload" 422
prepare_image

first_upload="$(upload "$instructor_cookie")"; require_status "Instructor image upload" "$first_upload" 200
first_image="$(image_reference "$(body "$first_upload")")"
assert_exact_record 'Elena Rivera' "$first_image"
instructor_delivery="$(request "/api/profile/avatar/profile-images/$first_image/delivery" "$instructor_cookie" POST '{}')"
require_status "Instructor self delivery" "$instructor_delivery" 200
concealed "$(request "/api/profile/avatar/profile-images/$first_image/delivery" "$sysadmin_cookie" POST '{}')"

second_upload="$(upload "$instructor_cookie")"; require_status "Instructor replacement upload" "$second_upload" 200
second_image="$(image_reference "$(body "$second_upload")")"
[ "$first_image" != "$second_image" ] || fail "replacement reused an image reference"
assert_retired_exact_cleanup "$first_image"

sysadmin_upload="$(upload "$sysadmin_cookie")"; require_status "Sysadmin image upload" "$sysadmin_upload" 200
sysadmin_image="$(image_reference "$(body "$sysadmin_upload")")"
sysadmin_delivery="$(request "/api/profile/avatar/profile-images/$sysadmin_image/delivery" "$sysadmin_cookie" POST '{}')"
require_status "Sysadmin self delivery" "$sysadmin_delivery" 200
concealed "$(request '/api/instructor-profile/thumbnail' "$instructor_cookie")"
concealed "$(request "/api/instructor-profile/thumbnails/$first_image/delivery" "$instructor_cookie" POST '{}')"

echo "Live Demo Profile Avatar: PASS"
