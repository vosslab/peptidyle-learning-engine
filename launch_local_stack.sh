#!/usr/bin/env bash
# launch_local_stack.sh - build and open the complete local Podman stack.
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
TIMEOUT_SECONDS="${PLE_LAUNCH_TIMEOUT_SECONDS:-180}"
LOCAL_ENV_FILE="containers/env.local"
LOCAL_IDENTITY_FILE="containers/local-identities.json"
LOCAL_CREDENTIAL_FILE="containers/local-login.txt"
LOCAL_DEMO_MANIFEST_FILE="containers/local-demo.json"
LOCAL_WEBWORK_DEMO_MANIFEST_FILE="containers/local-webwork-demo.json"
LOCAL_CHAPTER_ONE_MANIFEST_FILE="containers/local-chapter-one-pilot.json"
LOCAL_INVITATION_SECRET_FILE="containers/.secrets/invitation_token_secret"
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

usage() {
	cat <<'EOF'
Usage: ./launch_local_stack.sh [options]

Build the repository, bootstrap the private local configuration, migrate the
database, start the supported local stack, wait for gateway health, and open
the browser application.

Options:
  --release          Build optimized host artifacts instead of debug artifacts.
  --skip-build       Reuse an existing dist/ bundle and skip ./build.sh.
  --no-open          Start the stack without opening a browser.
  --with-smtp        Connect the API to an operator-selected external SMTP provider.
  --check            Validate tools and Compose configuration without changing state.
  --env-file PATH    Use a different Compose environment file.
  -h, --help         Show this help.

PLE_LAUNCH_TIMEOUT_SECONDS controls the readiness timeout (default: 180).
EOF
}

die() {
	echo "ERROR: $*" >&2
	exit 1
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		--release)
			BUILD_PROFILE="--release"
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
		--check)
			CHECK_ONLY=1
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
	inherited_gateway_port="${PLE_GATEWAY_HOST_PORT:-}"
	gateway_port="${inherited_gateway_port:-$configured_gateway_port}"
	gateway_port="${gateway_port:-8080}"
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
		gateway_container_is_running && return 0
		available_gateway_port="$(first_available_gateway_port)"
		write_env_value PLE_GATEWAY_HOST_PORT "$available_gateway_port"
		export PLE_GATEWAY_HOST_PORT="$available_gateway_port"
		echo "==> Port ${configured_gateway_port} is occupied; using gateway port ${available_gateway_port}"
	elif [ -z "$(env_value PLE_GATEWAY_HOST_PORT)" ] && [ -z "${PLE_GATEWAY_HOST_PORT:-}" ]; then
		write_env_value PLE_GATEWAY_HOST_PORT "$configured_gateway_port"
	fi
}

configure_local_webauthn_origin() {
	effective_gateway_port_value="$(effective_gateway_port)"
	configured_webauthn_origin="$(env_value PLE_WEBAUTHN_ORIGIN)"
	case "$configured_webauthn_origin" in
	"" | http://localhost:*)
		if [ -z "${PLE_GATEWAY_HOST_PORT:-}" ] || \
			[ "$(env_value PLE_GATEWAY_HOST_PORT)" = "$effective_gateway_port_value" ]; then
			write_env_value PLE_WEBAUTHN_ORIGIN "http://localhost:${effective_gateway_port_value}"
		fi
		export PLE_WEBAUTHN_ORIGIN="http://localhost:${effective_gateway_port_value}"
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

validate_invitation_secret_file() {
	secret_path="$1"
	[ -r "$secret_path" ] || die "Invitation issuer secret $secret_path is missing or unreadable"
	if stat -f '%Lp' "$secret_path" >/dev/null 2>&1; then
		secret_mode="$(stat -f '%Lp' "$secret_path")"
	else
		secret_mode="$(stat -c '%a' "$secret_path")"
	fi
	[ "$secret_mode" = "600" ] || die "Invitation issuer secret $secret_path must have mode 0600; fix it before launch"
	secret_value="$(cat "$secret_path")"
	printf '%s' "$secret_value" | grep -Eq '^[A-Za-z0-9_-]{43}$' || die "Invitation issuer secret $secret_path must be exactly 32 random bytes encoded as canonical base64url"
}

bootstrap_invitation_secret_file() {
	secret_path="$1"
	if [ -r "$secret_path" ]; then
		validate_invitation_secret_file "$secret_path"
		return 0
	fi
	secret_directory="$(dirname "$secret_path")"
	umask 077
	mkdir -p "$secret_directory"
	temporary_secret_file="$(mktemp "${secret_path}.XXXXXX")"
	openssl rand 32 | openssl base64 -A | tr '+/' '-_' | tr -d '=' >"$temporary_secret_file"
	chmod 600 "$temporary_secret_file"
	mv "$temporary_secret_file" "$secret_path"
	validate_invitation_secret_file "$secret_path"
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
	bootstrap_invitation_secret_file "$LOCAL_INVITATION_SECRET_FILE"

	set_default_env_value POSTGRES_PASSWORD "$(random_hex 24)"
	set_default_env_value MINIO_ROOT_PASSWORD "$(random_hex 24)"
	set_default_env_value PLE_LOCAL_GRADER_PASSWORD "$(random_hex 24)"
	set_default_env_value PLE_INVITATION_TOKEN_SECRET_HOST_FILE "$REPO_ROOT/$LOCAL_INVITATION_SECRET_FILE"
	set_default_env_value PLE_GATEWAY_IMAGE_SHA256 "$LOCAL_CADDY_IMAGE_SHA256"
	set_default_env_value PLE_POSTGRES_IMAGE_SHA256 "$LOCAL_POSTGRES_IMAGE_SHA256"
	set_default_env_value PLE_MINIO_IMAGE_SHA256 "$LOCAL_MINIO_IMAGE_SHA256"
	set_default_env_value PLE_MINIO_MC_IMAGE_SHA256 "$LOCAL_MINIO_MC_IMAGE_SHA256"
	set_default_env_value PLE_LOCAL_AUTH_HOST_FILE "$REPO_ROOT/$LOCAL_IDENTITY_FILE"
	set_default_env_value PLE_PUBLIC_ASSET_BASE_URL "http://127.0.0.1:9000/content"
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
	set_default_env_value PLE_WEBWORK_RENDERER_IMAGE "localhost/pg-renderer:latest"
	set_default_env_value PLE_SECRET_INIT_IMAGE_SHA256 "$LOCAL_SECRET_INIT_IMAGE_SHA256"
	ensure_default_gateway_port
	configure_local_webauthn_origin
}

if [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ] && [ "$CHECK_ONLY" -eq 0 ]; then
	bootstrap_default_local_configuration
fi
[ -r "$ENV_FILE" ] || die "$ENV_FILE is missing or unreadable; run without --check once to bootstrap containers/env.local"

if [ "$CHECK_ONLY" -eq 1 ] && [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ]; then
	for required_setting in PLE_POSTGRES_IMAGE_SHA256 PLE_MINIO_IMAGE_SHA256 PLE_MINIO_MC_IMAGE_SHA256 PLE_GATEWAY_IMAGE_SHA256 PLE_SECRET_INIT_IMAGE_SHA256; do
		[ -n "$(env_value "$required_setting")" ] || die "--check cannot validate this pre-image-pin env.local; run './launch_local_stack.sh --no-open' once to safely add immutable local image settings"
	done
fi

COMPOSE_COMMAND=()
if podman compose version >/dev/null 2>&1; then
	COMPOSE_COMMAND=(podman compose)
elif command -v podman-compose >/dev/null 2>&1 && podman-compose version >/dev/null 2>&1; then
	COMPOSE_COMMAND=(podman-compose)
else
	die "neither 'podman compose' nor 'podman-compose' is usable"
fi

compose() {
	compose_arguments=(-f containers/compose.yaml)
	if [ "$WITH_SMTP" -eq 1 ]; then
		compose_arguments+=(-f containers/compose.smtp.yaml)
	fi
	compose_arguments+=(--env-file "$ENV_FILE")
	"${COMPOSE_COMMAND[@]}" "${compose_arguments[@]}" "$@"
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

for required_setting in POSTGRES_USER POSTGRES_PASSWORD POSTGRES_DB MINIO_ROOT_USER MINIO_ROOT_PASSWORD PLE_LOCAL_GRADER_PASSWORD PLE_LOCAL_AUTH_HOST_FILE PLE_INVITATION_TOKEN_SECRET_HOST_FILE; do
	require_env_value "$required_setting"
done
for required_setting in PLE_POSTGRES_IMAGE_SHA256 PLE_MINIO_IMAGE_SHA256 PLE_MINIO_MC_IMAGE_SHA256 PLE_GATEWAY_IMAGE_SHA256 PLE_SECRET_INIT_IMAGE_SHA256; do
	require_sha256_env_value "$required_setting"
done
[ -r "$(env_value PLE_INVITATION_TOKEN_SECRET_HOST_FILE)" ] || die "invitation issuer secret file is missing or unreadable"
validate_invitation_secret_file "$(env_value PLE_INVITATION_TOKEN_SECRET_HOST_FILE)"

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

if [ "$BUILD_ENABLED" -eq 1 ]; then
	echo "==> Building Rust, Wasm, generated contracts, fixtures, and the browser client"
	./build.sh "$BUILD_PROFILE"
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

echo "==> Waiting for PostgreSQL"
started_at=$SECONDS
until compose exec -T postgres pg_isready -U "$postgres_user" -d "$postgres_database" >/dev/null 2>&1; do
	if [ $((SECONDS - started_at)) -ge "$TIMEOUT_SECONDS" ]; then
		die "PostgreSQL did not become ready within ${TIMEOUT_SECONDS}s"
	fi
	sleep 2
done

database_url="postgres://${postgres_user}:${postgres_password}@127.0.0.1:${postgres_port}/${postgres_database}"
echo "==> Applying and verifying database migrations"
PLE_MIGRATION_DATABASE_URL="$database_url" cargo tools database migrate

if [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ]; then
	demo_course_exists="$(compose exec -T postgres psql -v ON_ERROR_STOP=1 -U "$postgres_user" -d "$postgres_database" -Atc "SELECT EXISTS (SELECT 1 FROM course WHERE tenant_id = '$LOCAL_TENANT_ID' AND title = 'PLE replica E2E course');")"
	if [ "$demo_course_exists" = "f" ]; then
		echo "==> Seeding one local course, assignment, and native question"
		cargo tools e2e-seed \
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
compose exec -T postgres psql -v ON_ERROR_STOP=1 -U "$postgres_user" -d "$postgres_database" \
	-c "ALTER ROLE ple_grading_reader PASSWORD '$grader_password';" >/dev/null

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
renderer_container_id="$(podman ps \
	--filter label=io.podman.compose.project=containers \
	--filter label=io.podman.compose.service=webwork-renderer \
	--format '{{.ID}}')"
case "$renderer_container_id" in
""|*$'\n'*) die "expected exactly one running PLE webwork-renderer container" ;;
esac
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
if [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ]; then
	# This host-only seed uses the production PostgreSQL and object-store
	# contracts.  It is intentionally after renderer identity finalization so
	# the API cannot be started with a pilot question for an unpinned renderer.
	echo "==> Seeding the legacy walkthrough WeBWorK course and immutable PGML source"
	minio_port="$(env_value PLE_MINIO_API_HOST_PORT)"
	minio_port="${minio_port:-9000}"
	AWS_ACCESS_KEY_ID="$(env_value MINIO_ROOT_USER)" \
	AWS_SECRET_ACCESS_KEY="$(env_value MINIO_ROOT_PASSWORD)" \
	cargo tools e2e-seed --webwork-pilot \
		--database-url "$database_url" \
		--apply-migrations \
		--tenant "$LOCAL_TENANT_ID" \
		--instructor "$LOCAL_INSTRUCTOR_ID" \
		--student "$LOCAL_STUDENT_ID" \
		--s3-endpoint "http://127.0.0.1:${minio_port}" \
		--s3-region "us-east-1" \
		--content-bucket "content" >"$LOCAL_WEBWORK_DEMO_MANIFEST_FILE"
	chmod 600 "$LOCAL_WEBWORK_DEMO_MANIFEST_FILE"
	echo "==> Publishing the Genetics and Biochemistry Chapter 1 pilot corpus"
	AWS_ACCESS_KEY_ID="$(env_value MINIO_ROOT_USER)" \
	AWS_SECRET_ACCESS_KEY="$(env_value MINIO_ROOT_PASSWORD)" \
	cargo tools e2e-seed --chapter-one-pilot \
		--database-url "$database_url" \
		--apply-migrations \
		--tenant "$LOCAL_TENANT_ID" \
		--instructor "$LOCAL_INSTRUCTOR_ID" \
		--student "$LOCAL_STUDENT_ID" \
		--s3-endpoint "http://127.0.0.1:${minio_port}" \
		--s3-region "us-east-1" \
		--content-bucket "content" >"$LOCAL_CHAPTER_ONE_MANIFEST_FILE"
	chmod 600 "$LOCAL_CHAPTER_ONE_MANIFEST_FILE"
fi

if [ "$WITH_SMTP" -eq 1 ]; then
	echo "==> Installing external SMTP credential for the API"
	compose rm -f smtp-secret-init >/dev/null 2>&1 || true
	compose up -d smtp-secret-init
fi

echo "==> Building images and starting API, worker, and browser gateway"
services=(api worker gateway)
# Refresh the API-owned runtime copy on every launch so a rotated invitation
# issuer secret cannot leave a stale value in the named runtime volume.
compose rm -f identity-secret-init >/dev/null 2>&1 || true
compose up -d identity-secret-init
# podman-compose can leave a stopped container attached to the previous image
# ID even after rebuilding the same local tag. Recreate only the stateless
# application services so their running binaries always match this build;
# PostgreSQL, MinIO, and their named volumes remain untouched.
compose up -d --build --force-recreate --no-deps "${services[@]}"

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

echo "Local stack is ready: ${base_url}/"
if [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ]; then
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
