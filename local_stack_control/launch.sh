#!/usr/bin/env bash
# local_stack_control/launch.sh - build and open the complete local Podman stack.
#
# The gateway is the one browser origin. It serves the freshly built Solid/Wasm
# bundle and proxies same-origin API requests, so this script never starts a
# second static-file server beside Compose.

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
REPO_ROOT="$PWD"

ENV_FILE="containers/env.local"
BUILD_PROFILE="--debug"
BUILD_ENABLED=1
OPEN_BROWSER=1
CHECK_ONLY=0
WITH_SMTP=0
CANONICAL_WALKTHROUGH=0
RELEASE_SELECTED=0
RESTART_SERVICE=""
TIMEOUT_SECONDS="${PLE_LAUNCH_TIMEOUT_SECONDS:-180}"
LOCAL_ENV_FILE="containers/env.local"
LOCAL_IDENTITY_FILE="containers/local-identities.json"
LOCAL_CREDENTIAL_FILE="containers/local-login.txt"
LOCAL_DEMO_MANIFEST_FILE="containers/local-demo.json"
LOCAL_CHAPTER_ONE_MANIFEST_FILE="containers/local-chapter-one-pilot.json"
LOCAL_INVITATION_SECRET_FILE="containers/.secrets/invitation_token_secret"
LOCAL_QUESTION_ID_SECRET_FILE="containers/.secrets/question_id_secret"
LOCAL_WEBWORK_PROVENANCE_FILE="containers/.secrets/webwork_renderer_provenance"
LOCAL_TENANT_ID="00000000-0000-0000-0000-000000000100"
LOCAL_INSTRUCTOR_ID="00000000-0000-0000-0000-000000000101"
LOCAL_STUDENT_ID="00000000-0000-0000-0000-000000000102"
# Official Caddy Linux manifest already reviewed for the non-root gateway image.
LOCAL_CADDY_IMAGE_SHA256="844f60b64e4724a5aa8245e019dace0d3f199f7433ce6c57676cb30a920dbad9"
# Public multi-architecture manifests used by the native local stack.  They
# are pinned here as well as env.example so a first-run bootstrap has no
# mutable image tags.
LOCAL_POSTGRES_IMAGE_SHA256="7958605b474b3d264a969cb3a123d6aa00ad1e1fe9da8a69984dabb704d93317"
LOCAL_MINIO_IMAGE_SHA256="14cea493d9a34af32f524e538b8346cf79f3321eff8e708c1e2960462bd8936e"
LOCAL_MINIO_MC_IMAGE_SHA256="a7fe349ef4bd8521fb8497f55c6042871b2ae640607cf99d9bede5e9bdf11727"
LOCAL_SECRET_INIT_IMAGE_SHA256="48b0309ca019d89d40f670aa1bc06e426dc0931948452e8491e3d65087abc07d"
# Reviewed local OCI identity for the separately owned, stateless PG renderer.
# This is deliberately a repository-and-digest reference rather than a mutable tag.
LOCAL_WEBWORK_RENDERER_IMAGE="localhost/pg-renderer@sha256:d606c4b5d82d425729643c4f36d093d549759a416d0527f0340ae0a7319a8456"

usage() {
	cat <<'EOF'
Usage: ./local_stack_control/launch.sh [options]

Build the repository, bootstrap the private local configuration, migrate the
database, start the supported local stack, wait for gateway health, and open
the browser application.

Options:
  --release          Build optimized host artifacts instead of debug artifacts.
  --skip-build       Reuse an existing dist/ bundle and skip ./build.sh.
  --no-open          Start the stack without opening a browser.
  --with-smtp        Connect the API to an operator-selected external SMTP provider.
  --canonical-walkthrough
                    Run the repository-owned disposable teaching walkthrough only.
  --check            Validate tools and Compose configuration without changing state.
  --restart SERVICE  Recreate one verified stateless service in an already-ready stack.
                     SERVICE is api, worker, gateway, or webwork-renderer.
  --env-file PATH    Use a different Compose environment file.
  -h, --help         Show this help.

PLE_LAUNCH_TIMEOUT_SECONDS controls the readiness timeout (default: 180).
EOF
}

die() {
	echo "ERROR: $*" >&2
	exit 1
}

require_rootless_podman() {
	local operation="$1"
	[ "$(podman info --format '{{.Host.Security.Rootless}}')" = "true" ] \
		|| die "${operation} requires the rootless Podman engine"
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		--release)
			BUILD_PROFILE="--release"
			RELEASE_SELECTED=1
			;;
		--skip-build)
			BUILD_ENABLED=0
			;;
		--no-open)
			OPEN_BROWSER=0
			;;
		--with-smtp)
			WITH_SMTP=1
			;;
		--canonical-walkthrough)
			CANONICAL_WALKTHROUGH=1
			;;
		--check)
			CHECK_ONLY=1
			;;
		--restart)
			[ "$#" -ge 2 ] || die "--restart requires a service"
			[ -z "$RESTART_SERVICE" ] || die "--restart may be specified only once"
			shift
			RESTART_SERVICE="$1"
			;;
		--env-file)
			[ "$#" -ge 2 ] || die "--env-file requires a path"
			shift
			ENV_FILE="$1"
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			echo "ERROR: unknown option: $1" >&2
			usage >&2
			exit 2
			;;
	esac
	shift
done

if [ -n "$RESTART_SERVICE" ]; then
	case "$RESTART_SERVICE" in
		api|worker|gateway|webwork-renderer) ;;
		*) die "--restart service must be api, worker, gateway, or webwork-renderer" ;;
	esac
	[ "$CHECK_ONLY" -eq 0 ] || die "--restart cannot be combined with --check"
	[ "$RELEASE_SELECTED" -eq 0 ] || die "--restart cannot be combined with --release"
	[ "$BUILD_ENABLED" -eq 1 ] || die "--restart cannot be combined with --skip-build"
	[ "$CANONICAL_WALKTHROUGH" -eq 0 ] || die "--restart cannot be combined with --canonical-walkthrough"
	OPEN_BROWSER=0
fi

case "$TIMEOUT_SECONDS" in
	''|*[!0-9]*) die "PLE_LAUNCH_TIMEOUT_SECONDS must be a positive integer" ;;
esac
[ "$TIMEOUT_SECONDS" -gt 0 ] || die "PLE_LAUNCH_TIMEOUT_SECONDS must be a positive integer"

for command_name in git podman curl awk openssl xxd lsof; do
	command -v "$command_name" >/dev/null 2>&1 || die "$command_name not found on PATH"
done

env_value() {
	setting_name="$1"
	awk -F= -v setting_name="$setting_name" '
		$1 == setting_name {
			value = substr($0, index($0, "=") + 1)
			sub(/^[[:space:]]+/, "", value)
			sub(/[[:space:]]+$/, "", value)
			found = value
		}
		END { print found }
	' "$ENV_FILE"
}

env_declares_setting() {
	setting_name="$1"
	awk -F= -v setting_name="$setting_name" '
		$1 == setting_name { found = 1 }
		END { exit !found }
	' "$ENV_FILE"
}

write_env_value() {
	setting_name="$1"
	setting_value="$2"

	temporary_env_file="$(mktemp "${TMPDIR:-/tmp}/ple-env-local.XXXXXX")"
	awk -v setting_name="$setting_name" -v setting_value="$setting_value" '
		BEGIN { replaced = 0 }
		index($0, setting_name "=") == 1 {
			if (!replaced) print setting_name "=" setting_value
			replaced = 1
			next
		}
		{ print }
		END {
			if (!replaced) print setting_name "=" setting_value
		}
	' "$ENV_FILE" >"$temporary_env_file"
	mv "$temporary_env_file" "$ENV_FILE"
	chmod 600 "$ENV_FILE"
}

set_default_env_value() {
	setting_name="$1"
	setting_value="$2"
	current_setting_value="$(env_value "$setting_name")"
	if [ -n "$current_setting_value" ] && [ "$current_setting_value" != "change-me-before-first-run" ]; then
		return 0
	fi
	write_env_value "$setting_name" "$setting_value"
}

gateway_container_is_running() {
	podman ps --format '{{.Names}}' 2>/dev/null | awk '$0 == "containers_gateway_1" { found = 1 } END { exit !found }'
}

effective_gateway_port() {
	configured_gateway_port="$(env_value PLE_GATEWAY_HOST_PORT)"
	gateway_port="${configured_gateway_port:-8080}"
	case "$gateway_port" in
	''|*[!0-9]*) die "PLE_GATEWAY_HOST_PORT must be an unquoted integer" ;;
	esac
	[ "$gateway_port" -ge 1 ] && [ "$gateway_port" -le 65535 ] \
		|| die "PLE_GATEWAY_HOST_PORT must be between 1 and 65535"
	printf '%s\n' "$gateway_port"
}

first_available_gateway_port() {
	candidate_port=8000
	while [ "$candidate_port" -le 8099 ]; do
		if ! lsof -nP -iTCP:"$candidate_port" -sTCP:LISTEN >/dev/null 2>&1; then
			printf '%s\n' "$candidate_port"
			return 0
		fi
		candidate_port=$((candidate_port + 1))
	done
	die "no available local gateway port was found from 8000 through 8099"
}

ensure_default_gateway_port() {
	configured_gateway_port="$(effective_gateway_port)"
	if lsof -nP -iTCP:"$configured_gateway_port" -sTCP:LISTEN >/dev/null 2>&1; then
		[ "$CANONICAL_WALKTHROUGH" -eq 0 ] || die "canonical walkthrough gateway port ${configured_gateway_port} is occupied"
		gateway_container_is_running && return 0
		available_gateway_port="$(first_available_gateway_port)"
		write_env_value PLE_GATEWAY_HOST_PORT "$available_gateway_port"
		echo "==> Port ${configured_gateway_port} is occupied; using gateway port ${available_gateway_port}"
	elif [ -z "$(env_value PLE_GATEWAY_HOST_PORT)" ]; then
		write_env_value PLE_GATEWAY_HOST_PORT "$configured_gateway_port"
	fi
}

configure_local_webauthn_origin() {
	effective_gateway_port_value="$(effective_gateway_port)"
	configured_webauthn_origin="$(env_value PLE_WEBAUTHN_ORIGIN)"
	case "$configured_webauthn_origin" in
	"" | http://localhost:*)
		write_env_value PLE_WEBAUTHN_ORIGIN "http://localhost:${effective_gateway_port_value}"
		;;
	esac
}

random_hex() {
	openssl rand -hex "$1"
}

local_credential_record() {
	raw_hex="$(random_hex 32)"
	credential="$(printf '%s' "$raw_hex" | xxd -r -p | openssl base64 -A | tr '+/' '-_' | tr -d '=')"
	credential_hash="$(printf '%s' "$raw_hex" | xxd -r -p | openssl dgst -sha256 -r | awk '{print $1}')"
	printf '%s\t%s\n' "$credential" "$credential_hash"
}

source "$REPO_ROOT/containers/local_identity_bootstrap.sh"
ENV_FILE="$(normalize_default_local_env_file "$ENV_FILE")"

if [ "$CANONICAL_WALKTHROUGH" -eq 1 ]; then
	LOCAL_RUNTIME_DIRECTORY="$(dirname "$ENV_FILE")"
	LOCAL_IDENTITY_FILE="$LOCAL_RUNTIME_DIRECTORY/local-identities.json"
	LOCAL_CREDENTIAL_FILE="$LOCAL_RUNTIME_DIRECTORY/local-login.txt"
	LOCAL_DEMO_MANIFEST_FILE="$LOCAL_RUNTIME_DIRECTORY/local-demo.json"
	LOCAL_CHAPTER_ONE_MANIFEST_FILE="$LOCAL_RUNTIME_DIRECTORY/local-chapter-one-pilot.json"
	LOCAL_INVITATION_SECRET_FILE="$LOCAL_RUNTIME_DIRECTORY/.secrets/invitation_token_secret"
	LOCAL_QUESTION_ID_SECRET_FILE="$LOCAL_RUNTIME_DIRECTORY/.secrets/question_id_secret"
	LOCAL_WEBWORK_PROVENANCE_FILE="$LOCAL_RUNTIME_DIRECTORY/.secrets/webwork_renderer_provenance"
fi

validate_secret32_file() {
	secret_path="$1"
	secret_label="$2"
	case "$secret_path" in
		/*) ;;
		*) die "$secret_label file must use an absolute host path" ;;
	esac
	[ ! -L "$secret_path" ] || die "$secret_label file must not be a symbolic link"
	[ -f "$secret_path" ] && [ -r "$secret_path" ] || die "$secret_label file is missing, unreadable, or not regular"
	if stat -f '%Lp' "$secret_path" >/dev/null 2>&1; then
		secret_mode="$(stat -f '%Lp' "$secret_path")"
	else
		secret_mode="$(stat -c '%a' "$secret_path")"
	fi
	[ "$secret_mode" = "600" ] || die "$secret_label file must have mode 0600; fix it before launch"
	secret_size="$(wc -c <"$secret_path" | tr -d '[:space:]')"
	[ "$secret_size" = "43" ] || die "$secret_label file must contain exactly one canonical 32-byte base64url secret"
	secret_value="$(cat "$secret_path")"
	printf '%s' "$secret_value" | grep -Eq '^[A-Za-z0-9_-]{43}$' || die "$secret_label file must contain exactly one canonical 32-byte base64url secret"
	secret_hex="$(printf '%s=' "$secret_value" | tr '_-' '/+' | openssl base64 -d -A 2>/dev/null | xxd -p -c 999)"
	printf '%s' "$secret_hex" | grep -Eq '^[0-9a-f]{64}$' || die "$secret_label file must contain exactly one canonical 32-byte base64url secret"
	canonical_secret_value="$(printf '%s' "$secret_hex" | xxd -r -p | openssl base64 -A | tr '+/' '-_' | tr -d '=')"
	[ "$secret_value" = "$canonical_secret_value" ] || die "$secret_label file must contain exactly one canonical 32-byte base64url secret"
}

bootstrap_secret32_file() {
	secret_path="$1"
	secret_label="$2"
	case "$secret_path" in
		/*) ;;
		*) secret_path="$REPO_ROOT/$secret_path" ;;
	esac
	if [ -e "$secret_path" ] || [ -L "$secret_path" ]; then
		validate_secret32_file "$secret_path" "$secret_label"
		return 0
	fi
	secret_directory="$(dirname "$secret_path")"
	umask 077
	mkdir -p "$secret_directory"
	temporary_secret_file="$(mktemp "${secret_path}.XXXXXX")"
	openssl rand 32 | openssl base64 -A | tr '+/' '-_' | tr -d '=' >"$temporary_secret_file"
	chmod 600 "$temporary_secret_file"
	mv "$temporary_secret_file" "$secret_path"
	validate_secret32_file "$secret_path" "$secret_label"
}

validate_smtp_password_file() {
	secret_path="$1"
	case "$secret_path" in
		/*) ;;
		*) die "SMTP password file must use an absolute host path" ;;
	esac
	[ ! -L "$secret_path" ] || die "SMTP password file must not be a symbolic link"
	[ -f "$secret_path" ] && [ -r "$secret_path" ] || die "SMTP password file is missing, unreadable, or not regular"
	if stat -f '%Lp' "$secret_path" >/dev/null 2>&1; then
		secret_mode="$(stat -f '%Lp' "$secret_path")"
	else
		secret_mode="$(stat -c '%a' "$secret_path")"
	fi
	[ "$secret_mode" = "600" ] || die "SMTP password file must have mode 0600"
	secret_size="$(wc -c <"$secret_path" | tr -d '[:space:]')"
	[ "$secret_size" -ge 1 ] && [ "$secret_size" -le 4096 ] || die "SMTP password file must contain 1 through 4096 bytes"
}

bootstrap_default_local_configuration() {
	if [ ! -e "$ENV_FILE" ]; then
		cp containers/env.example "$ENV_FILE"
		chmod 600 "$ENV_FILE"
	fi
	[ -w "$ENV_FILE" ] || die "$ENV_FILE is not writable for first-run local bootstrap"
	bootstrap_local_identities
	bootstrap_secret32_file "$LOCAL_INVITATION_SECRET_FILE" "Invitation issuer secret"
	bootstrap_secret32_file "$LOCAL_QUESTION_ID_SECRET_FILE" "Question ID secret"

	set_default_env_value POSTGRES_PASSWORD "$(random_hex 24)"
	set_default_env_value MINIO_ROOT_PASSWORD "$(random_hex 24)"
	set_default_env_value PLE_LOCAL_GRADER_PASSWORD "$(random_hex 24)"
	set_default_env_value PLE_INVITATION_TOKEN_SECRET_HOST_FILE "$REPO_ROOT/$LOCAL_INVITATION_SECRET_FILE"
	set_default_env_value PLE_QUESTION_ID_SECRET_HOST_FILE "$REPO_ROOT/$LOCAL_QUESTION_ID_SECRET_FILE"
	set_default_env_value PLE_GATEWAY_IMAGE_SHA256 "$LOCAL_CADDY_IMAGE_SHA256"
	set_default_env_value PLE_POSTGRES_IMAGE_SHA256 "$LOCAL_POSTGRES_IMAGE_SHA256"
	set_default_env_value PLE_MINIO_IMAGE_SHA256 "$LOCAL_MINIO_IMAGE_SHA256"
	set_default_env_value PLE_MINIO_MC_IMAGE_SHA256 "$LOCAL_MINIO_MC_IMAGE_SHA256"
	set_default_env_value PLE_LOCAL_AUTH_HOST_FILE "$REPO_ROOT/$LOCAL_IDENTITY_FILE"
	set_default_env_value PLE_PUBLIC_ASSET_BASE_URL "http://127.0.0.1:9000/public-assets"
	set_default_env_value PLE_WEBAUTHN_RP_ID "localhost"
	set_default_env_value PLE_WEBAUTHN_RP_NAME "Peptidyle Learning Engine"
	configured_renderer_url="$(env_value PLE_WEBWORK_RENDERER_BASE_URL)"
	case "$configured_renderer_url" in
	""|http://webwork-renderer:8080|http://webwork-renderer:8080/webwork2/)
		write_env_value PLE_WEBWORK_RENDERER_BASE_URL "http://webwork-renderer:3000/"
		;;
	esac
	set_default_env_value PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS "15"
	set_default_env_value PLE_WEBWORK_MAX_RESPONSE_BYTES "1048576"
	set_default_env_value PLE_WEBWORK_PROBLEM_JWT_SECRET "$(random_hex 32)"
	set_default_env_value PLE_WEBWORK_SESSION_JWT_SECRET "$(random_hex 32)"
	case "$(env_value PLE_WEBWORK_RENDERER_ID)" in
	""|openwebwork-webwork2) write_env_value PLE_WEBWORK_RENDERER_ID "vosslab-webwork-pg-renderer" ;;
	esac
	set_default_env_value PLE_WEBWORK_RENDERER_IMAGE "$LOCAL_WEBWORK_RENDERER_IMAGE"
	set_default_env_value PLE_SECRET_INIT_IMAGE_SHA256 "$LOCAL_SECRET_INIT_IMAGE_SHA256"
	ensure_default_gateway_port
	configure_local_webauthn_origin
}

if { [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ] || [ "$CANONICAL_WALKTHROUGH" -eq 1 ]; } \
	&& [ "$CHECK_ONLY" -eq 0 ] && [ -z "$RESTART_SERVICE" ]; then
	bootstrap_default_local_configuration
fi
[ -r "$ENV_FILE" ] || die "$ENV_FILE is missing or unreadable; run 'source source_me.sh && python3 local_stack.py start --no-open' to initialize it"
if [ -n "$RESTART_SERVICE" ]; then
	[ ! -L "$ENV_FILE" ] && [ -f "$ENV_FILE" ] \
		|| die "restart requires a regular, non-symbolic-link environment file"
	[ "$(local_file_mode "$ENV_FILE")" = "600" ] || die "restart requires an environment file with mode 0600"
fi

# Compose project naming is deliberately process-only: the selected env file
# owns application configuration, while callers such as the isolated browser
# E2E runner use COMPOSE_PROJECT_NAME to isolate a whole stack.
if env_declares_setting COMPOSE_PROJECT_NAME; then
	die "COMPOSE_PROJECT_NAME must not be declared in $ENV_FILE; set it in the calling environment to isolate a Compose project"
fi
if env_declares_setting PLE_DISPOSABLE_CAPABILITY_FILE; then
	die "PLE_DISPOSABLE_CAPABILITY_FILE must be supplied only by the typed disposable owner"
fi

if [ "$CHECK_ONLY" -eq 1 ] && [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ]; then
	for required_setting in PLE_POSTGRES_IMAGE_SHA256 PLE_MINIO_IMAGE_SHA256 PLE_MINIO_MC_IMAGE_SHA256 PLE_GATEWAY_IMAGE_SHA256 PLE_SECRET_INIT_IMAGE_SHA256; do
		[ -n "$(env_value "$required_setting")" ] || die "--check cannot validate this pre-image-pin env.local; run './local_stack_control/launch.sh --no-open' once to safely add immutable local image settings"
	done
fi

COMPOSE_PROJECT_NAME_VALUE="${COMPOSE_PROJECT_NAME:-containers}"
COMPOSE_COMMAND=()
if [ "$COMPOSE_PROJECT_NAME_VALUE" != "containers" ]; then
	if [ "$CHECK_ONLY" -eq 0 ]; then
		capability_file="${PLE_DISPOSABLE_CAPABILITY_FILE:-}"
		[ -n "$capability_file" ] \
			|| die "disposable launch requires the runner-held cleanup capability"
		case "$capability_file" in
			/*) ;;
			*) die "disposable capability file must use an absolute path" ;;
		esac
		[ ! -L "$capability_file" ] && [ -f "$capability_file" ] && [ -r "$capability_file" ] \
			|| die "disposable capability file must be a readable regular file"
		[ -O "$capability_file" ] \
			|| die "disposable capability file must be owned by the current user"
		[ "$(local_file_mode "$capability_file")" = "600" ] \
			|| die "disposable capability file must have mode 0600"
		[ "$(wc -c <"$capability_file" | tr -d '[:space:]')" = "32" ] \
			|| die "disposable capability file must contain exactly 32 bytes"
		expected_capability_digest="$(env_value PLE_DISPOSABLE_CAPABILITY_SHA256)"
		printf '%s' "$expected_capability_digest" | grep -Eq '^[0-9a-f]{64}$' \
			|| die "disposable environment must declare a SHA-256 capability commitment"
		actual_capability_digest="$(openssl dgst -sha256 -r "$capability_file" | awk '{print $1}')"
		[ "$actual_capability_digest" = "$expected_capability_digest" ] \
			|| die "disposable capability does not match its environment commitment"
	fi
	unset PLE_DISPOSABLE_CAPABILITY_FILE
	if command -v podman-compose >/dev/null 2>&1 && podman-compose version >/dev/null 2>&1; then
		COMPOSE_COMMAND=(podman-compose --in-pod false)
	else
		die "disposable stacks require 'podman-compose --in-pod false'"
	fi
elif podman compose version >/dev/null 2>&1; then
	unset PLE_DISPOSABLE_CAPABILITY_FILE
	COMPOSE_COMMAND=(podman compose)
elif command -v podman-compose >/dev/null 2>&1 && podman-compose version >/dev/null 2>&1; then
	unset PLE_DISPOSABLE_CAPABILITY_FILE
	COMPOSE_COMMAND=(podman-compose)
else
	unset PLE_DISPOSABLE_CAPABILITY_FILE
	die "neither 'podman compose' nor 'podman-compose' is usable"
fi

# Compose gives inherited shell variables precedence over --env-file. Remove
# only names owned by the selected file so one explicit configuration supplies
# both container interpolation and the host-side migration URL.
COMPOSE_ENVIRONMENT_ARGUMENTS=()
while IFS= read -r compose_setting_name; do
	COMPOSE_ENVIRONMENT_ARGUMENTS+=(-u "$compose_setting_name")
done < <(awk -F= '/^[A-Za-z_][A-Za-z0-9_]*=/ { print $1 }' "$ENV_FILE")
if [ -n "$RESTART_SERVICE" ] && [ "${COMPOSE_PROJECT_NAME+x}" = x ] \
	&& [ "$COMPOSE_PROJECT_NAME" != "containers" ]; then
	die "--restart is limited to the default containers project; use the typed disposable local-stack adapter for an owned stack"
fi
compose() {
	compose_arguments=(-p "$COMPOSE_PROJECT_NAME_VALUE" -f containers/compose.yaml)
	if [ "$WITH_SMTP" -eq 1 ]; then
		compose_arguments+=(-f containers/compose.smtp.yaml)
	fi
	compose_arguments+=(--env-file "$ENV_FILE")
	env "${COMPOSE_ENVIRONMENT_ARGUMENTS[@]}" \
		"${COMPOSE_COMMAND[@]}" "${compose_arguments[@]}" "$@"
}

compose_service_container_id() {
	service_name="$1"
	compose_container_ids="$(compose ps -q)"
	service_container_ids=()
	while IFS= read -r compose_container_id; do
		[ -n "$compose_container_id" ] || continue
		podman_service_name="$(podman container inspect \
			--format '{{ index .Config.Labels "io.podman.compose.service" }}' \
			"$compose_container_id")"
		docker_service_name="$(podman container inspect \
			--format '{{ index .Config.Labels "com.docker.compose.service" }}' \
			"$compose_container_id")"
		if [ "$podman_service_name" = "$service_name" ] || [ "$docker_service_name" = "$service_name" ]; then
			assert_compose_container_labels "$compose_container_id" "$service_name"
			service_container_ids+=("$compose_container_id")
		fi
	done <<<"$compose_container_ids"
	case "${#service_container_ids[@]}" in
	1) printf '%s\n' "${service_container_ids[0]}" ;;
	*) die "expected exactly one running PLE ${service_name} container" ;;
	esac
}

assert_compose_container_labels() {
	local container_id="$1" service_name="$2"
	local podman_project docker_project podman_service docker_service
	podman_project="$(podman container inspect --format '{{with index .Config.Labels "io.podman.compose.project"}}{{.}}{{end}}' "$container_id")"
	docker_project="$(podman container inspect --format '{{with index .Config.Labels "com.docker.compose.project"}}{{.}}{{end}}' "$container_id")"
	podman_service="$(podman container inspect --format '{{with index .Config.Labels "io.podman.compose.service"}}{{.}}{{end}}' "$container_id")"
	docker_service="$(podman container inspect --format '{{with index .Config.Labels "com.docker.compose.service"}}{{.}}{{end}}' "$container_id")"
	if ! { [ -z "$podman_project" ] || [ "$podman_project" = "$COMPOSE_PROJECT_NAME_VALUE" ]; } \
		|| ! { [ -z "$docker_project" ] || [ "$docker_project" = "$COMPOSE_PROJECT_NAME_VALUE" ]; } \
		|| ! { [ -z "$podman_service" ] || [ "$podman_service" = "$service_name" ]; } \
		|| ! { [ -z "$docker_service" ] || [ "$docker_service" = "$service_name" ]; }; then
		die "Compose label aliases conflict for ${container_id}"
	fi
}

compose_service_container_id_any_state() {
	local service_name="$1"
	local compose_container_ids

	compose_container_ids="$(
		{
			podman ps -a -q \
				--filter "label=io.podman.compose.project=${COMPOSE_PROJECT_NAME_VALUE}" \
				--filter "label=io.podman.compose.service=${service_name}"
			podman ps -a -q \
				--filter "label=com.docker.compose.project=${COMPOSE_PROJECT_NAME_VALUE}" \
				--filter "label=com.docker.compose.service=${service_name}"
		} | awk 'NF && !seen[$0]++'
	)"
	valid_container_ids=()
	while IFS= read -r compose_container_id; do
		[ -n "$compose_container_id" ] || continue
		assert_compose_container_labels "$compose_container_id" "$service_name"
		valid_container_ids+=("$compose_container_id")
	done <<<"$compose_container_ids"
	case "${#valid_container_ids[@]}" in
	1) printf '%s\n' "${valid_container_ids[0]}" ;;
	*) die "expected exactly one PLE ${service_name} container" ;;
	esac
}

compose_service_container_count_any_state() {
	local service_name="$1"
	{
		podman ps -a -q \
			--filter "label=io.podman.compose.project=${COMPOSE_PROJECT_NAME_VALUE}" \
			--filter "label=io.podman.compose.service=${service_name}"
		podman ps -a -q \
			--filter "label=com.docker.compose.project=${COMPOSE_PROJECT_NAME_VALUE}" \
			--filter "label=com.docker.compose.service=${service_name}"
	} | awk 'NF && !seen[$0]++' | while IFS= read -r compose_container_id; do
		assert_compose_container_labels "$compose_container_id" "$service_name"
		echo "$compose_container_id"
	done | awk 'NF { count += 1 } END { print count + 0 }'
}

wait_for_one_shot_service() {
	local service_name="$1"
	local container_id
	local container_status
	local container_exit_code
	local started_at

	container_id="$(compose_service_container_id_any_state "$service_name")"
	started_at=$SECONDS
	while true; do
		container_status="$(podman container inspect --format '{{.State.Status}}' "$container_id")"
		container_exit_code="$(podman container inspect --format '{{.State.ExitCode}}' "$container_id")"
		if [ "$container_status" = "exited" ]; then
			if [ "$container_exit_code" = "0" ]; then
				return 0
			fi
			echo "ERROR: the ${service_name} one-shot service failed with exit code ${container_exit_code}" >&2
			compose logs --tail 80 "$service_name" >&2 || true
			exit 1
		fi
		if [ $((SECONDS - started_at)) -ge "$TIMEOUT_SECONDS" ]; then
			echo "ERROR: the ${service_name} one-shot service did not finish within ${TIMEOUT_SECONDS}s" >&2
			compose ps >&2 || true
			compose logs --tail 80 "$service_name" >&2 || true
			exit 1
		fi
		sleep 1
	done
}

set_local_postgres_role_password() {
	local role_name="$1"
	local role_password="$2"

	printf 'ALTER ROLE "%s" PASSWORD '\''%s'\'';\n' "$role_name" "$role_password" \
		| compose exec -T postgres psql -v ON_ERROR_STOP=1 \
			-U "$postgres_user" -d "$postgres_database" >/dev/null
}

wait_for_required_stack_services() {
	local service_name
	local container_id
	local running
	local health_status
	local started_at
	local required_services=(postgres minio webwork-renderer api worker gateway)

	for service_name in "${required_services[@]}"; do
		container_id="$(compose_service_container_id "$service_name")"
		started_at=$SECONDS
		while true; do
			running="$(podman container inspect --format '{{.State.Running}}' "$container_id")"
			if [ "$service_name" = "worker" ]; then
				health_status="disabled"
			else
				health_status="$(podman container inspect --format '{{.State.Health.Status}}' "$container_id")"
			fi
			if [ "$running" = "true" ] && { [ "$health_status" = "healthy" ] || [ "$health_status" = "disabled" ]; }; then
				break
			fi
			if [ $((SECONDS - started_at)) -ge "$TIMEOUT_SECONDS" ]; then
				echo "ERROR: required service ${service_name} did not become active and healthy within ${TIMEOUT_SECONDS}s" >&2
				compose ps >&2 || true
				compose logs --tail 80 "$service_name" >&2 || true
				exit 1
			fi
			sleep 1
		done
	done
}


source "$REPO_ROOT/local_stack_control/_restart.sh"

print_compose_action() {
	local compose_arguments=(-p "$COMPOSE_PROJECT_NAME_VALUE" -f containers/compose.yaml)
	[ "$WITH_SMTP" -eq 0 ] || compose_arguments+=(-f containers/compose.smtp.yaml)
	compose_arguments+=(--env-file "$ENV_FILE" "$@")
	printf 'Resolved Compose action:'
	printf ' %q' "${COMPOSE_COMMAND[@]}" "${compose_arguments[@]}"
	printf '\n'
}

probe_renderer() {
	local renderer_container_id="$1"
	local started_at=$SECONDS
	until podman exec -i "$renderer_container_id" bash -s -- --exercise \
		<containers/webwork/probe_render_api.sh >/dev/null 2>&1; do
		[ $((SECONDS - started_at)) -lt "$TIMEOUT_SECONDS" ] \
			|| restart_refusal "the renderer failed its render and grade probe"
		sleep 2
	done
}

wait_for_gateway_health() {
	gateway_port="$(effective_gateway_port)"
	base_url="http://127.0.0.1:${gateway_port}"
	echo "==> Waiting up to ${TIMEOUT_SECONDS}s for ${base_url}/health"
	started_at=$SECONDS
	until curl --fail --silent --show-error --max-time 2 --output /dev/null "${base_url}/health" 2>/dev/null; do
		[ $((SECONDS - started_at)) -lt "$TIMEOUT_SECONDS" ] \
			|| restart_refusal "the published gateway health route did not recover"
		sleep 2
	done
}

record_renderer_provenance() {
	local temporary_provenance_file
	[ ! -L "$renderer_provenance_path" ] \
		|| restart_refusal "the renderer provenance record must not be a symbolic link"
	temporary_provenance_file="$(mktemp "${renderer_provenance_path}.XXXXXX")"
	chmod 600 "$temporary_provenance_file"
	printf 'image_ref=%s\nimage_id=%s\n' "$renderer_image_ref" "$renderer_image_id" \
		>"$temporary_provenance_file"
	mv "$temporary_provenance_file" "$renderer_provenance_path"
}

restart_stateless_service() {
	local renderer_container_id restarted_renderer_image_id
	assert_ready_for_restart
	prepare_renderer_identity_for_restart
	echo "==> Restarting verified service ${RESTART_SERVICE} in Compose project ${COMPOSE_PROJECT_NAME_VALUE}"
	if [ "$RESTART_SERVICE" = "api" ]; then
		probe_renderer "$(compose_service_container_id webwork-renderer)"
		print_compose_action up -d --force-recreate --no-deps identity-secret-init
		compose up -d --force-recreate --no-deps identity-secret-init
		wait_for_one_shot_service identity-secret-init
		if [ "$WITH_SMTP" -eq 1 ]; then
			print_compose_action up -d --force-recreate --no-deps smtp-secret-init
			compose up -d --force-recreate --no-deps smtp-secret-init
			wait_for_one_shot_service smtp-secret-init
		fi
	fi
	print_compose_action up -d --force-recreate --no-deps "$RESTART_SERVICE"
	compose up -d --force-recreate --no-deps "$RESTART_SERVICE"
	if [ "$RESTART_SERVICE" = "webwork-renderer" ]; then
		renderer_container_id="$(compose_service_container_id webwork-renderer)"
		restarted_renderer_image_id="$(podman container inspect --format '{{.Image}}' "$renderer_container_id")"
		[ "$restarted_renderer_image_id" = "$renderer_image_id" ] \
			|| restart_refusal "the recreated renderer does not use the attested image"
		record_renderer_provenance
		probe_renderer "$renderer_container_id"
	fi
	echo "==> Confirming every required long-running service is active"
	wait_for_required_stack_services
	case "$RESTART_SERVICE" in
		api|gateway) wait_for_gateway_health ;;
	esac
	echo "Verified restart complete: ${RESTART_SERVICE} in project ${COMPOSE_PROJECT_NAME_VALUE}."
}

require_env_value() {
	setting_name="$1"
	[ -n "$(env_value "$setting_name")" ] || die "$setting_name is missing from $ENV_FILE"
}

require_sha256_env_value() {
	setting_name="$1"
	require_env_value "$setting_name"
	setting_value="$(env_value "$setting_name")"
	printf '%s' "$setting_value" | awk '/^[0-9a-f]{64}$/ { valid = 1 } END { exit !valid }' || die "$setting_name must be a 64-character lowercase hexadecimal SHA-256 manifest digest"
}

require_renderer_image_reference() {
	renderer_reference="$1"
	printf '%s' "$renderer_reference" | awk '/^.+@sha256:[0-9a-f]{64}$/ { valid = 1 } END { exit !valid }' \
		|| die "PLE_WEBWORK_RENDERER_IMAGE must be an immutable repository@sha256: digest reference"
}

for required_setting in POSTGRES_USER POSTGRES_PASSWORD POSTGRES_DB MINIO_ROOT_USER MINIO_ROOT_PASSWORD PLE_LOCAL_GRADER_PASSWORD PLE_LOCAL_AUTH_HOST_FILE PLE_INVITATION_TOKEN_SECRET_HOST_FILE PLE_QUESTION_ID_SECRET_HOST_FILE; do
	require_env_value "$required_setting"
done
for required_setting in PLE_POSTGRES_IMAGE_SHA256 PLE_MINIO_IMAGE_SHA256 PLE_MINIO_MC_IMAGE_SHA256 PLE_GATEWAY_IMAGE_SHA256 PLE_SECRET_INIT_IMAGE_SHA256; do
	require_sha256_env_value "$required_setting"
done
validate_secret32_file "$(env_value PLE_INVITATION_TOKEN_SECRET_HOST_FILE)" "Invitation issuer secret"
validate_secret32_file "$(env_value PLE_QUESTION_ID_SECRET_HOST_FILE)" "Question ID secret"

if [ "$WITH_SMTP" -eq 1 ]; then
	for required_setting in PLE_SMTP_RELAY PLE_SMTP_PORT PLE_SMTP_TLS_MODE PLE_SMTP_USERNAME PLE_SMTP_PASSWORD_HOST_FILE PLE_SMTP_FROM PLE_PUBLIC_APP_BASE_URL; do
		require_env_value "$required_setting"
	done
	smtp_port="$(env_value PLE_SMTP_PORT)"
	case "$smtp_port" in
		''|*[!0-9]*) die "PLE_SMTP_PORT must be an unquoted integer" ;;
	esac
	[ "$smtp_port" -ge 1 ] && [ "$smtp_port" -le 65535 ] || die "PLE_SMTP_PORT must be between 1 and 65535"
	case "$(env_value PLE_SMTP_TLS_MODE)" in
		starttls|implicit-tls) ;;
		*) die "PLE_SMTP_TLS_MODE must be exactly starttls or implicit-tls" ;;
	esac
	case "$(env_value PLE_SMTP_RELAY)" in
		*://*) die "PLE_SMTP_RELAY must be a hostname without a URL scheme" ;;
	esac
	case "$(env_value PLE_PUBLIC_APP_BASE_URL)" in
		https://?*) ;;
		*) die "PLE_PUBLIC_APP_BASE_URL must be a public HTTPS origin" ;;
	esac
	validate_smtp_password_file "$(env_value PLE_SMTP_PASSWORD_HOST_FILE)"
fi

for required_setting in PLE_WEBWORK_RENDERER_IMAGE PLE_WEBWORK_RENDERER_ID PLE_WEBWORK_PROBLEM_JWT_SECRET PLE_WEBWORK_SESSION_JWT_SECRET; do
	require_env_value "$required_setting"
done
renderer_image_ref="$(env_value PLE_WEBWORK_RENDERER_IMAGE)"
require_renderer_image_reference "$renderer_image_ref"
podman image inspect "$renderer_image_ref" >/dev/null 2>&1 || die "Build or pull the standalone webwork-pg-renderer image '$renderer_image_ref', then rerun the launcher"

echo "==> Checking Compose configuration"
compose_error_file="$(mktemp "${TMPDIR:-/tmp}/ple-compose-config.XXXXXX")"
if ! compose config >/dev/null 2>"$compose_error_file"; then
	compose_error="$(tail -n 1 "$compose_error_file")"
	rm -f "$compose_error_file"
	compose_error="${compose_error#ValueError: }"
	die "Compose configuration is incomplete: ${compose_error:-the provider did not report a reason}"
fi
rm -f "$compose_error_file"

if [ "$CHECK_ONLY" -eq 1 ]; then
	echo "Local stack configuration is ready. No build or containers were changed."
	exit 0
fi

if ! podman info >/dev/null 2>&1; then
	if [ "$(uname -s)" = "Darwin" ]; then
		echo "==> Starting the Podman machine"
		podman machine start || die "the Podman machine could not be started; see docs/MACOS_PODMAN.md"
		podman info >/dev/null 2>&1 || die "Podman is still unavailable after starting its machine"
	else
		die "Podman is unavailable; start its service and try again"
	fi
fi

if [ -n "$RESTART_SERVICE" ]; then
	require_rootless_podman "stateless restart"
else
	require_rootless_podman "local stack"
fi

if [ -n "$RESTART_SERVICE" ]; then
	restart_stateless_service
	exit 0
fi

if [ "$BUILD_ENABLED" -eq 1 ]; then
	echo "==> Building Rust, Wasm, generated contracts, fixtures, and the browser client"
	PLE_BROWSER_LOCAL_DEVELOPMENT_AUTH=1 ./build.sh "$BUILD_PROFILE"
elif [ ! -f dist/index.html ] || [ ! -f dist/main.js ]; then
	die "--skip-build requires dist/index.html and dist/main.js; run ./build.sh first"
fi

postgres_user="$(env_value POSTGRES_USER)"
postgres_password="$(env_value POSTGRES_PASSWORD)"
postgres_database="$(env_value POSTGRES_DB)"
postgres_port="$(env_value PLE_POSTGRES_HOST_PORT)"
postgres_port="${postgres_port:-5432}"
grader_password="$(env_value PLE_LOCAL_GRADER_PASSWORD)"
for identifier_value in "$postgres_user" "$postgres_database"; do
	case "$identifier_value" in
		''|*[!A-Za-z0-9_]*) die "local PostgreSQL names must contain only letters, numbers, and underscores" ;;
	esac
done
for password_value in "$postgres_password" "$grader_password"; do
	case "$password_value" in
		''|*[!A-Za-z0-9_-]*) die "local PostgreSQL passwords must contain only letters, numbers, underscores, and hyphens" ;;
	esac
done
case "$postgres_port" in
	''|*[!0-9]*) die "PLE_POSTGRES_HOST_PORT must be an unquoted integer" ;;
esac

echo "==> Starting PostgreSQL and object storage"
echo "==> Verifying PostgreSQL data-volume major"
if ! compose --profile maintenance run --rm --no-deps -T postgres-major-guard; then
	die "the existing PostgreSQL data volume is not compatible with the pinned PostgreSQL 17 image; preserve it and migrate it with an explicit major-version procedure"
fi
compose up -d postgres minio createbuckets
wait_for_one_shot_service createbuckets

echo "==> Waiting for PostgreSQL"
started_at=$SECONDS
until compose exec -T postgres pg_isready -U "$postgres_user" -d "$postgres_database" >/dev/null 2>&1; do
	if [ $((SECONDS - started_at)) -ge "$TIMEOUT_SECONDS" ]; then
		die "PostgreSQL did not become ready within ${TIMEOUT_SECONDS}s"
	fi
	sleep 2
done

if [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ] || [ "$CANONICAL_WALKTHROUGH" -eq 1 ]; then
	echo "==> Synchronizing the retained local PostgreSQL login"
	set_local_postgres_role_password "$postgres_user" "$postgres_password"
fi

database_url="postgres://${postgres_user}:${postgres_password}@127.0.0.1:${postgres_port}/${postgres_database}"
echo "==> Applying and verifying database migrations"
PLE_MIGRATION_DATABASE_URL="$database_url" cargo tools database migrate

if [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ] || [ "$CANONICAL_WALKTHROUGH" -eq 1 ]; then
	demo_course_exists="$(compose exec -T postgres psql -v ON_ERROR_STOP=1 -U "$postgres_user" -d "$postgres_database" -Atc "SELECT EXISTS (SELECT 1 FROM course WHERE tenant_id = '$LOCAL_TENANT_ID' AND title = 'PLE replica E2E course');")"
	if [ "$demo_course_exists" = "f" ]; then
		echo "==> Seeding one local course, assignment, and native question"
		PLE_QUESTION_ID_SECRET_FILE="$(env_value PLE_QUESTION_ID_SECRET_HOST_FILE)" cargo tools e2e-seed \
			--database-url "$database_url" \
			--apply-migrations \
			--tenant "$LOCAL_TENANT_ID" \
			--instructor "$LOCAL_INSTRUCTOR_ID" \
			--student "$LOCAL_STUDENT_ID" >"$LOCAL_DEMO_MANIFEST_FILE"
		chmod 600 "$LOCAL_DEMO_MANIFEST_FILE"
	elif [ "$demo_course_exists" != "t" ]; then
		die "could not determine whether the local demonstration course exists"
	fi
fi

echo "==> Provisioning the isolated local grading login"
set_local_postgres_role_password ple_grading_reader "$grader_password"

echo "==> Starting the external stateless PG renderer image"
compose up -d --force-recreate --no-deps webwork-renderer
renderer_image_id="$(podman image inspect --format '{{.Id}}' "$renderer_image_ref")"
[ -n "$renderer_image_id" ] || die "the WebWork renderer image has no OCI image ID"
umask 077
printf 'image_ref=%s\nimage_id=%s\n' \
	"$renderer_image_ref" "$renderer_image_id" \
	>"$LOCAL_WEBWORK_PROVENANCE_FILE"
chmod 600 "$LOCAL_WEBWORK_PROVENANCE_FILE"
# Bind this process to the inspected external image. The renderer repository
# owns the image contents and PG compatibility; PLE owns only this integration.
PLE_WEBWORK_RENDERER_VERSION="${renderer_image_id#sha256:}"
export PLE_WEBWORK_RENDERER_VERSION
renderer_container_id="$(compose_service_container_id webwork-renderer)"
echo "==> Verifying standalone PG render and grade behavior"
started_at=$SECONDS
until podman exec -i "$renderer_container_id" bash -s -- --exercise <containers/webwork/probe_render_api.sh >/dev/null 2>&1; do
	if [ $((SECONDS - started_at)) -ge "$TIMEOUT_SECONDS" ]; then
		echo "ERROR: the standalone PG renderer did not pass its render/grade probe; it has been left running for diagnosis" >&2
		compose ps >&2 || true
		compose logs --tail 80 webwork-renderer >&2 || true
		exit 1
	fi
	sleep 2
done
if [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ] || [ "$CANONICAL_WALKTHROUGH" -eq 1 ]; then
	# This host-only seed uses the production PostgreSQL and object-store
	# contracts. It is intentionally after renderer identity finalization so the
	# canonical teaching corpus cannot be published for an unpinned renderer.
	minio_port="$(env_value PLE_MINIO_API_HOST_PORT)"
	minio_port="${minio_port:-9000}"
	echo "==> Publishing the Genetics and Biochemistry Chapter 1 pilot corpus"
	AWS_ACCESS_KEY_ID="$(env_value MINIO_ROOT_USER)" \
	AWS_SECRET_ACCESS_KEY="$(env_value MINIO_ROOT_PASSWORD)" \
	PLE_QUESTION_ID_SECRET_FILE="$(env_value PLE_QUESTION_ID_SECRET_HOST_FILE)" \
	cargo tools e2e-seed --chapter-one-pilot \
		--database-url "$database_url" \
		--apply-migrations \
		--tenant "$LOCAL_TENANT_ID" \
		--instructor "$LOCAL_INSTRUCTOR_ID" \
		--student "$LOCAL_STUDENT_ID" \
		--s3-endpoint "http://127.0.0.1:${minio_port}" \
		--s3-region "us-east-1" \
		--private-content-bucket "private-content" >"$LOCAL_CHAPTER_ONE_MANIFEST_FILE"
	chmod 600 "$LOCAL_CHAPTER_ONE_MANIFEST_FILE"
fi

if [ "$WITH_SMTP" -eq 1 ]; then
	echo "==> Installing external SMTP credential for the API"
	compose rm -f smtp-secret-init >/dev/null 2>&1 || true
	compose up -d smtp-secret-init
	wait_for_one_shot_service smtp-secret-init
fi

echo "==> Building the shared application image and browser gateway"
services=(api worker gateway)
# Refresh the API-owned runtime copy on every launch so a rotated invitation
# issuer secret cannot leave a stale value in the named runtime volume.
compose rm -f identity-secret-init >/dev/null 2>&1 || true
compose up -d identity-secret-init
wait_for_one_shot_service identity-secret-init
# podman-compose can leave a stopped container attached to the previous image
# ID even after rebuilding the same local tag. Recreate only the stateless
# application services so their running binaries always match this build;
# PostgreSQL, MinIO, and their named volumes remain untouched.
# Build the Rust application once through its single Compose owner before any
# stateless service starts. `worker` deliberately has no build declaration: it
# must consume the same image tag as `api`. Keeping build and start separate
# is portable across `podman compose` and standalone `podman-compose`, and a
# failed build stops the launcher before it can start a stale image.
compose build api gateway
echo "==> Starting API, worker, and browser gateway from the built images"
compose up -d --force-recreate --no-deps "${services[@]}"

gateway_port="$(effective_gateway_port)"

base_url="http://127.0.0.1:${gateway_port}"
echo "==> Waiting up to ${TIMEOUT_SECONDS}s for ${base_url}/health"
started_at=$SECONDS
while ! curl --fail --silent --show-error --max-time 2 --output /dev/null "${base_url}/health" 2>/dev/null; do
	if [ $((SECONDS - started_at)) -ge "$TIMEOUT_SECONDS" ]; then
		echo "ERROR: the stack did not become ready; it has been left running for diagnosis" >&2
		compose ps >&2 || true
		compose logs --tail=80 gateway api worker >&2 || true
		exit 1
	fi
	sleep 2
done

echo "==> Confirming every required long-running service is active"
wait_for_required_stack_services

echo "Local stack is ready: ${base_url}/"
if [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ] || [ "$CANONICAL_WALKTHROUGH" -eq 1 ]; then
	echo "Local sign-in credentials: $LOCAL_CREDENTIAL_FILE"
fi
stop_command=("${COMPOSE_COMMAND[@]}" -f containers/compose.yaml)
if [ "$WITH_SMTP" -eq 1 ]; then
	stop_command+=(-f containers/compose.smtp.yaml)
fi
stop_command+=(--env-file "$ENV_FILE")
stop_command+=(down --remove-orphans)
printf 'Stop it without deleting data:'
printf ' %q' "${stop_command[@]}"
printf '\n'

if [ "$OPEN_BROWSER" -eq 1 ]; then
	if command -v open >/dev/null 2>&1; then
		open "${base_url}/"
	elif command -v xdg-open >/dev/null 2>&1; then
		xdg-open "${base_url}/" >/dev/null 2>&1 &
	else
		echo "No browser opener was found; open ${base_url}/ manually."
	fi
fi
