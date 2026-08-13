#!/usr/bin/env bash
# e2e_chapter_one_browser.sh - isolated real-browser journey through the eight pilot questions.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly REPO_ROOT
readonly PROJECT_NAME="ple_chapter_one_browser_$$"
readonly GATEWAY_IMAGE="localhost/${PROJECT_NAME}_gateway:latest"
readonly TENANT_ID="00000000-0000-0000-0000-000000000100"
readonly INSTRUCTOR_ID="00000000-0000-0000-0000-000000000101"
readonly STUDENT_ID="00000000-0000-0000-0000-000000000102"
readonly POSTGRES_PORT="${PLE_CHAPTER_ONE_BROWSER_POSTGRES_PORT:-$((51500 + RANDOM % 400))}"
readonly MINIO_PORT="${PLE_CHAPTER_ONE_BROWSER_MINIO_PORT:-$((52000 + RANDOM % 400))}"
readonly MINIO_CONSOLE_PORT="${PLE_CHAPTER_ONE_BROWSER_MINIO_CONSOLE_PORT:-$((52500 + RANDOM % 400))}"
readonly GATEWAY_PORT="${PLE_CHAPTER_ONE_BROWSER_GATEWAY_PORT:-$((53000 + RANDOM % 400))}"

TEMP_DIRECTORY=""
STACK_OWNED=0
COMPOSE=()

fail() {
	echo "Chapter 1 browser E2E: $*" >&2
	exit 1
}

require_command() {
	command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

configure_compose() {
	if podman compose version >/dev/null 2>&1; then
		COMPOSE=(podman compose)
	elif command -v podman-compose >/dev/null 2>&1 && podman-compose version >/dev/null 2>&1; then
		COMPOSE=(podman-compose)
	else
		fail "no usable Podman Compose provider is available"
	fi
}

compose() {
	COMPOSE_PROJECT_NAME="$PROJECT_NAME" "${COMPOSE[@]}" \
		-f containers/compose.yaml --env-file "$TEMP_DIRECTORY/env.local" "$@"
}

cleanup() {
	local status="$?"
	local cleanup_failed=0
	local image_status=0
	if [ "${PLE_E2E_KEEP:-0}" = "1" ]; then
		echo "Chapter 1 browser E2E: preserving $PROJECT_NAME and $TEMP_DIRECTORY"
	else
		if [ "$STACK_OWNED" = "1" ]; then
			compose down --volumes --remove-orphans >/dev/null 2>&1 || cleanup_failed=1
			if podman image exists "$GATEWAY_IMAGE" >/dev/null 2>&1; then
				podman image rm "$GATEWAY_IMAGE" >/dev/null 2>&1 || cleanup_failed=1
			else
				image_status="$?"
				if [ "$image_status" -ne 1 ]; then
					cleanup_failed=1
				fi
			fi
		fi
		if [ -n "$TEMP_DIRECTORY" ] && [ -d "$TEMP_DIRECTORY" ]; then
			rm -rf -- "$TEMP_DIRECTORY"
		fi
		if [ "$cleanup_failed" -ne 0 ]; then
			echo "Chapter 1 browser E2E: exact disposable cleanup failed" >&2
			if [ "$status" -eq 0 ]; then
				status=1
			fi
		fi
	fi
	exit "$status"
}
trap cleanup EXIT

base64url_random() {
	openssl rand "$1" | openssl base64 -A | tr '+/' '-_' | tr -d '='
}

credential_hash() {
	printf '%s=' "$1" | tr '_-' '/+' | openssl base64 -d -A 2>/dev/null | \
		openssl dgst -sha256 -r | awk '{print $1}'
}

write_private_inputs() {
	local instructor_credential="$1"
	local student_credential="$2"
	local instructor_hash="$3"
	local student_hash="$4"
	local postgres_password="$5"
	local minio_password="$6"
	local grader_password="$7"
	local invitation_secret="$8"

	umask 077
	printf 'instructor=%s\nstudent=%s\n' \
		"$instructor_credential" "$student_credential" >"$TEMP_DIRECTORY/local-login.txt"
	printf '{"credentials":[{"credential_sha256":"%s","learner_alias":"instructor-local","tenant_id":"%s","user_id":"%s","display_name":"Dr. Fake Professor","roles":["instructor","administrator"]},{"credential_sha256":"%s","learner_alias":"student-local","tenant_id":"%s","user_id":"%s","display_name":"Mary Fake Student","roles":["student"]}]}\n' \
		"$instructor_hash" "$TENANT_ID" "$INSTRUCTOR_ID" \
		"$student_hash" "$TENANT_ID" "$STUDENT_ID" >"$TEMP_DIRECTORY/local-identities.json"
	printf '%s' "$invitation_secret" >"$TEMP_DIRECTORY/invitation-secret"
	chmod 600 "$TEMP_DIRECTORY/local-login.txt" "$TEMP_DIRECTORY/invitation-secret"
	chmod 644 "$TEMP_DIRECTORY/local-identities.json"

	printf '%s\n' \
		"POSTGRES_USER=ple_chapter_browser" \
		"POSTGRES_PASSWORD=$postgres_password" \
		"POSTGRES_DB=ple_chapter_browser" \
		"PLE_POSTGRES_IMAGE_SHA256=7958605b474b3d264a969cb3a123d6aa00ad1e1fe9da8a69984dabb704d93317" \
		"PLE_LOCAL_GRADER_PASSWORD=$grader_password" \
		"PLE_POSTGRES_HOST_PORT=$POSTGRES_PORT" \
		"MINIO_ROOT_USER=ple-chapter-browser" \
		"MINIO_ROOT_PASSWORD=$minio_password" \
		"PLE_MINIO_API_HOST_PORT=$MINIO_PORT" \
		"PLE_MINIO_CONSOLE_HOST_PORT=$MINIO_CONSOLE_PORT" \
		"PLE_MINIO_IMAGE_SHA256=14cea493d9a34af32f524e538b8346cf79f3321eff8e708c1e2960462bd8936e" \
		"PLE_MINIO_MC_IMAGE_SHA256=a7fe349ef4bd8521fb8497f55c6042871b2ae640607cf99d9bede5e9bdf11727" \
		"PLE_GATEWAY_HOST_PORT=$GATEWAY_PORT" \
		"PLE_GATEWAY_IMAGE_SHA256=844f60b64e4724a5aa8245e019dace0d3f199f7433ce6c57676cb30a920dbad9" \
		"PLE_LOCAL_AUTH_HOST_FILE=$TEMP_DIRECTORY/local-identities.json" \
		"PLE_PUBLIC_ASSET_BASE_URL=http://127.0.0.1:$MINIO_PORT/content" \
		"PLE_WEBAUTHN_RP_ID=localhost" \
		"PLE_WEBAUTHN_ORIGIN=http://localhost:$GATEWAY_PORT" \
		"PLE_WEBAUTHN_RP_NAME=Peptidyle Learning Engine" \
		"PLE_INVITATION_TOKEN_SECRET_HOST_FILE=$TEMP_DIRECTORY/invitation-secret" \
		"PLE_WEBWORK_RENDERER_IMAGE=localhost/pg-renderer:latest" \
		"PLE_WEBWORK_RENDERER_ID=vosslab-webwork-pg-renderer" \
		"PLE_WEBWORK_PROBLEM_JWT_SECRET=$(openssl rand -hex 32)" \
		"PLE_WEBWORK_SESSION_JWT_SECRET=$(openssl rand -hex 32)" \
		"PLE_SECRET_INIT_IMAGE_SHA256=48b0309ca019d89d40f670aa1bc06e426dc0931948452e8491e3d65087abc07d" \
		>"$TEMP_DIRECTORY/env.local"
	chmod 600 "$TEMP_DIRECTORY/env.local"
}

cd "$REPO_ROOT"
for command_name in awk cargo curl git npx openssl podman python3; do
	require_command "$command_name"
done
configure_compose
TEMP_DIRECTORY="$(mktemp -d)"

instructor_credential="$(base64url_random 32)"
student_credential="$(base64url_random 32)"
postgres_password="$(openssl rand -hex 24)"
minio_password="$(openssl rand -hex 24)"
grader_password="$(openssl rand -hex 24)"
invitation_secret="$(base64url_random 32)"
write_private_inputs \
	"$instructor_credential" "$student_credential" \
	"$(credential_hash "$instructor_credential")" "$(credential_hash "$student_credential")" \
	"$postgres_password" "$minio_password" "$grader_password" "$invitation_secret"

echo "Chapter 1 browser E2E: building the browser and host artifacts"
./build.sh

echo "Chapter 1 browser E2E: starting an isolated complete PLE stack"
STACK_OWNED=1
COMPOSE_PROJECT_NAME="$PROJECT_NAME" PLE_LAUNCH_TIMEOUT_SECONDS=240 \
	./launch_local_stack.sh --env-file "$TEMP_DIRECTORY/env.local" --skip-build --no-open

database_url="postgres://ple_chapter_browser:${postgres_password}@127.0.0.1:${POSTGRES_PORT}/ple_chapter_browser"
echo "Chapter 1 browser E2E: publishing the exact two-by-four teaching corpus"
AWS_ACCESS_KEY_ID=ple-chapter-browser AWS_SECRET_ACCESS_KEY="$minio_password" \
	cargo tools e2e-seed --chapter-one-pilot \
	--database-url "$database_url" \
	--apply-migrations \
	--tenant "$TENANT_ID" \
	--instructor "$INSTRUCTOR_ID" \
	--student "$STUDENT_ID" \
	--s3-endpoint "http://127.0.0.1:$MINIO_PORT" \
	--s3-region us-east-1 \
	--content-bucket content >"$TEMP_DIRECTORY/chapter-one.json"
chmod 600 "$TEMP_DIRECTORY/chapter-one.json"

echo "Chapter 1 browser E2E: completing all eight questions through visible keyboard controls"
PLE_WEBWORK_LIVE_REQUIRED=1 \
PLE_WEBWORK_LIVE_BASE_URL="http://127.0.0.1:$GATEWAY_PORT" \
PLE_WEBWORK_LIVE_STUDENT_CREDENTIAL_FILE="$TEMP_DIRECTORY/local-login.txt" \
	npx playwright test tests/playwright/chapter_one_run.spec.ts

echo "Chapter 1 browser E2E: PASS"
