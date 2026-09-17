#!/usr/bin/env bash
# Permanent C853 acceptance: Question Star names are a narrow vetted-Instructor
# disclosure. A regression could expose an Account identity or let an anonymous,
# Student, inactive, or non-Published caller discover Star activity. If this fails,
# repair the authorization/projection boundary; do not weaken concealment or add
# substitute identity fields to the browser response.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
postgres_name="ple-c853-star-privacy-$$"
minio_name="${postgres_name}-minio"
work="$(mktemp -d /private/tmp/ple-c853-star-privacy.XXXXXX)"
server_pid=""

cleanup() {
    local status="$?"
    if [[ -n "$server_pid" ]]; then
        kill "$server_pid" >/dev/null 2>&1 || true
        wait "$server_pid" 2>/dev/null || true
    fi
    podman rm --force --volumes "$postgres_name" "$minio_name" >/dev/null 2>&1 || true
    rm -rf "$work"
    return "$status"
}
trap cleanup EXIT

fail() { echo "Question Star name privacy: $*" >&2; exit 1; }
status() { printf '%s' "${1##*$'\n'}"; }
body() { printf '%s' "${1%$'\n'*}"; }

free_port() {
    python3 -c 'import socket; sock=socket.socket(); sock.bind(("127.0.0.1", 0)); print(sock.getsockname()[1]); sock.close()'
}

postgres_port="$(free_port)"
api_port="$(free_port)"
minio_port="$(free_port)"
database="ple_c853_${$}"
database_password="temporary-c853-password"
api_password="$(python3 -c 'import secrets; print(secrets.token_hex(32))')"
minio_password="$(python3 -c 'import secrets; print(secrets.token_hex(24))')"
origin="https://localhost:${api_port}"
endpoint="http://localhost:${api_port}"
readonly postgres_port api_port minio_port database database_password api_password minio_password origin endpoint

source "$root/source_me.sh"
cd "$root"

podman run --detach --name "$postgres_name" --label org.peptidyle.e2e=question-star-name-privacy \
    --mount "type=bind,src=$root,dst=/workspace,ro=true" \
    --env POSTGRES_PASSWORD=c853 --env POSTGRES_DB="$database" \
    -p "127.0.0.1:${postgres_port}:5432" docker.io/library/postgres:17 >/dev/null
for _ in $(seq 1 30); do
    podman exec "$postgres_name" pg_isready -U postgres -d "$database" >/dev/null && break
    sleep 1
done
podman exec "$postgres_name" pg_isready -U postgres -d "$database" >/dev/null || fail "PostgreSQL 17 did not start"

python3 -c "import local_stack_control.lifecycle_database as d; print(d.migration_principal_bootstrap_sql('$database', '$database_password'), end='')" \
    | podman exec -i "$postgres_name" psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" >/dev/null
podman exec --env PGPASSWORD="$database_password" "$postgres_name" \
    psql -X -v ON_ERROR_STOP=1 --single-transaction -U ple_migrator -d "$database" \
    -f /workspace/schemas/base_schema/install.sql >/dev/null

# The Sysadmin, Student, and inactive Instructor are role fixtures. The sole
# active Star actor is created through the C17/C18 HTTP vetting/account flow.
podman exec "$postgres_name" psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" -c "
    SET ROLE ple_private_owner;
    INSERT INTO ple_private.account(account_id, product_role, created_at) VALUES
      ('00000000-0000-0000-0000-00000000c831', 'sysadmin', clock_timestamp()),
      ('00000000-0000-0000-0000-00000000c832', 'student', clock_timestamp()),
      ('00000000-0000-0000-0000-00000000c833', 'instructor', clock_timestamp());
" >/dev/null

podman run --detach --name "$minio_name" --label org.peptidyle.e2e=question-star-name-privacy \
    -p "127.0.0.1:${minio_port}:9000" -e MINIO_ROOT_USER=c853root \
    -e MINIO_ROOT_PASSWORD="$minio_password" quay.io/minio/minio server /data >/dev/null
for _ in $(seq 1 30); do
    podman exec "$minio_name" mc ready local >/dev/null 2>&1 && break
    sleep 1
done
podman exec "$minio_name" mc ready local >/dev/null || fail "MinIO did not start"
podman exec "$minio_name" mc alias set local http://127.0.0.1:9000 c853root "$minio_password" >/dev/null
for bucket in public-assets private-content student-records temp-processing; do
    podman exec "$minio_name" mc mb --ignore-existing "local/$bucket" >/dev/null
done

python3 -c 'import sys; from local_stack_control.process_logins import login_sql; print("BEGIN;\n" + login_sql("ple_api_login", ("ple_app", "ple_auth"), sys.argv[1]) + "COMMIT;")' "$api_password" \
    | podman exec -i "$postgres_name" psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" >/dev/null
printf 'oci_id=sha256:%064d\n' 0 > "$work/question-renderer-version"
chmod 600 "$work/question-renderer-version"
(
    export DATABASE_URL="postgres://ple_api_login:${api_password}@127.0.0.1:${postgres_port}/${database}"
    export PLE_STORAGE_TOPOLOGY=disposable-local PLE_S3_ENDPOINT="http://127.0.0.1:${minio_port}" PLE_S3_REGION=us-east-1
    export AWS_ACCESS_KEY_ID=c853root AWS_SECRET_ACCESS_KEY="$minio_password" PLE_PUBLIC_ASSETS_BUCKET=public-assets PLE_PRIVATE_CONTENT_BUCKET=private-content
    export PLE_STUDENT_RECORDS_BUCKET=student-records PLE_TEMP_PROCESSING_BUCKET=temp-processing PLE_PUBLIC_ASSET_BASE_URL="$origin/public-assets"
    export PLE_BROWSER_ORIGIN="$origin" PLE_BIND_ADDR="127.0.0.1:${api_port}"
    export PLE_WEBWORK_RENDERER_VERSION_FILE="$work/question-renderer-version" PLE_WEBWORK_RENDERER_BASE_URL=http://127.0.0.1:9/
    export PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS=1 PLE_WEBWORK_MAX_RESPONSE_BYTES=1024 PLE_WEBWORK_RENDERER_ID=c853
    export PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID=00000000-0000-0000-0000-00000000c833 PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID=00000000-0000-0000-0000-00000000c831
    cargo run --quiet -p server_core >"$work/server.log" 2>&1
) &
server_pid=$!
for _ in $(seq 1 45); do
    curl --silent --max-time 1 -H "Host: localhost:${api_port}" "$endpoint/health" >/dev/null 2>&1 && break
    sleep 1
done
curl --silent --max-time 2 -H "Host: localhost:${api_port}" "$endpoint/health" >/dev/null || {
    cat "$work/server.log" >&2
    fail "host server did not start"
}

request() {
    local method="$1" path="$2" cookie="${3:-}" payload="${4:-}"
    local -a args=(--silent --show-error --max-time 12 --write-out $'\n%{http_code}' --request "$method" -H "Host: localhost:${api_port}")
    [[ "$method" != GET ]] && args+=(-H "Origin: $origin" -H 'Content-Type: application/json')
    [[ -z "$cookie" ]] || args+=(-H "Cookie: $cookie")
    [[ -z "$payload" ]] || args+=(--data "$payload")
    curl "${args[@]}" "$endpoint$path"
}

new_session() {
    local account_id="$1" token hash session_id
    read -r token hash session_id < <(python3 -c 'import base64, hashlib, os, uuid; token=os.urandom(32); print(base64.urlsafe_b64encode(token).decode().rstrip("="), hashlib.sha256(token).hexdigest(), uuid.uuid4())')
    podman exec "$postgres_name" psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" -c \
        "SELECT ple_private.create_authenticated_session('$session_id'::uuid, '$account_id'::uuid, decode('$hash', 'hex'), 28800);" >/dev/null
    printf '%s\n' "__Host-ple_session=$token"
}

new_sysadmin_session() {
    local account_id="$1" token hash session_id attestation_id binding_hash
    read -r token hash session_id attestation_id binding_hash < <(python3 -c 'import base64, hashlib, os, uuid; token=os.urandom(32); print(base64.urlsafe_b64encode(token).decode().rstrip("="), hashlib.sha256(token).hexdigest(), uuid.uuid4(), uuid.uuid4(), hashlib.sha256(os.urandom(32)).hexdigest())')
    # This database fixture supplies the trusted primary-authentication fact;
    # it uses the ordinary bound, one-use transition rather than generic issuance.
    # TOTP verification itself is covered by the authentication acceptance lane.
    podman exec "$postgres_name" psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" -c "
        BEGIN;
        SET LOCAL ROLE ple_private_owner;
        INSERT INTO ple_private.sysadmin_totp_attestation (
            attestation_id, account_id, browser_binding_hash, created_at, expires_at
        ) VALUES ('$attestation_id'::uuid, '$account_id'::uuid, decode('$binding_hash', 'hex'),
                  transaction_timestamp(), transaction_timestamp() + interval '5 minutes');
        SET LOCAL ROLE ple_auth;
        SELECT ple_api.consume_sysadmin_totp_attestation_into_session(
            '$attestation_id'::uuid, decode('$binding_hash', 'hex'), 0,
            '$session_id'::uuid, decode('$hash', 'hex'), 28800);
        COMMIT;
    " >/dev/null
    printf '%s\n' "__Host-ple_session=$token"
}

require_status() {
    local label="$1" response="$2" expected="$3"
    [[ "$(status "$response")" == "$expected" ]] || {
        printf '%s\n' "$response" >&2
        fail "$label returned HTTP $(status "$response"), expected $expected"
    }
}
assert_concealed() { require_status "$1" "$2" 404; }

sysadmin_cookie="$(new_sysadmin_session 00000000-0000-0000-0000-00000000c831)"
student_cookie="$(new_session 00000000-0000-0000-0000-00000000c832)"
inactive_cookie="$(new_session 00000000-0000-0000-0000-00000000c833)"
podman exec "$postgres_name" psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" -c "
    SET ROLE ple_private_owner;
    INSERT INTO ple_private.account_state_event(event_id, account_id, state, occurred_at, reason) VALUES
      ('00000000-0000-0000-0000-00000000c834', '00000000-0000-0000-0000-00000000c833', 'deactivated', clock_timestamp(), 'C853 isolated inactive fixture');
" >/dev/null

# C17: the Sysadmin independently vets an exact bounded display name.
email="c853-${$}@example.edu"
vetted="$(request POST /api/instructor-identity-vetting-decisions "$sysadmin_cookie" "{\"normalizedEmail\":\"$email\",\"verifiedInstructorDisplayName\":\"C853 Verified Instructor\"}")"
require_status "Instructor identity vetting" "$vetted" 201
vetting_reference="$(python3 -c 'import json, re, sys; value=json.loads(sys.argv[1]); assert set(value)=={"vettingDecisionReference"}; reference=value["vettingDecisionReference"]; assert re.fullmatch(r"[0-9a-f-]{36}", reference); print(reference)' "$(body "$vetted")")"

# C18: the vetted identity becomes the active Instructor Account that acts.
created="$(request POST /api/instructor-accounts "$sysadmin_cookie" "{\"normalizedEmail\":\"$email\",\"vettingDecisionReference\":\"$vetting_reference\"}")"
require_status "Instructor Account creation" "$created" 201
active_account_id="$(podman exec "$postgres_name" psql -XAt -U postgres -d "$database" -c "SELECT account_id FROM ple_private.account_authentication_email WHERE normalized_email = '$email'")"
[[ "$active_account_id" =~ ^[0-9a-f-]{36}$ ]] || fail "C18 did not create one Instructor Account"
active_cookie="$(new_session "$active_account_id")"

# The only Question fixtures are source state. The HMAC-valid non-Published
# identifier proves that route concealment occurs after identity validation.
read -r published_question nonpublished_question < <(python3 - <<'PY'
import hashlib
import hmac

alphabet = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ"
secret = b"\0" * 32  # The unpadded base64url `A` fixture decodes to 32 zero bytes.
def question(identifier):
    check = alphabet[hmac.new(secret, identifier, hashlib.sha256).digest()[0] >> 3:][0:1]
    compact = identifier[:4] + check + identifier[4:]
    print(compact[:4].decode() + "-" + compact[4:].decode(), end=" ")
question(b"C853ABC")
question(b"C853ABD")
print()
PY
)
podman exec "$postgres_name" psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" -c "
    SET ROLE ple_data_owner;
    INSERT INTO ple_data.published_question(question_id, created_at)
    VALUES ('${published_question/-/}', clock_timestamp());
" >/dev/null

# C370 action must travel over authenticated HTTP; no SQL shortcut creates a Star.
starred="$(request PUT "/api/questions/by-id/$published_question/stewardship/star" "$active_cookie" '{"starred":true}')"
require_status "active vetted Instructor Star" "$starred" 200
python3 -c 'import json, sys; value=json.loads(sys.argv[1]); expected={"starCount":1,"viewerHasStarred":True,"starredInstructors":[{"displayName":"C853 Verified Instructor"}]}; assert value == expected, value' "$(body "$starred")" || fail "Star action did not return the closed vetted-name projection"

visible="$(request GET "/api/questions/by-id/$published_question/stewardship/star" "$active_cookie")"
require_status "active Instructor Star disclosure" "$visible" 200
python3 -c 'import json, sys; value=json.loads(sys.argv[1]); expected={"starCount":1,"viewerHasStarred":True,"starredInstructors":[{"displayName":"C853 Verified Instructor"}]}; assert value == expected, value' "$(body "$visible")" || fail "Star disclosure was not the exact closed projection"

anonymous="$(request GET "/api/questions/by-id/$published_question/stewardship/star")"
student="$(request GET "/api/questions/by-id/$published_question/stewardship/star" "$student_cookie")"
inactive="$(request GET "/api/questions/by-id/$published_question/stewardship/star" "$inactive_cookie")"
nonpublished="$(request GET "/api/questions/by-id/$nonpublished_question/stewardship/star" "$active_cookie")"
assert_concealed "anonymous Star disclosure" "$anonymous"
assert_concealed "Student Star disclosure" "$student"
assert_concealed "inactive Instructor Star disclosure" "$inactive"
assert_concealed "HMAC-valid non-Published Star disclosure" "$nonpublished"

echo "Question Star name privacy E2E: PASS"
