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
WITH_WEBWORK=0
TIMEOUT_SECONDS="${PLE_LAUNCH_TIMEOUT_SECONDS:-180}"
LOCAL_ENV_FILE="containers/env.local"
LOCAL_IDENTITY_FILE="containers/local-identities.json"
LOCAL_CREDENTIAL_FILE="containers/local-login.txt"
LOCAL_DEMO_MANIFEST_FILE="containers/local-demo.json"
LOCAL_WEBWORK_DEMO_MANIFEST_FILE="containers/local-webwork-demo.json"
LOCAL_WEBWORK_SECRET_FILE="containers/.secrets/webwork_render_password"
LOCAL_WEBWORK_MOJO_SECRET_FILE="containers/.secrets/webwork_mojolicious_secret"
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
  --with-webwork     Build and start the private source-pinned WeBWorK renderer.
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
		--with-webwork)
			WITH_WEBWORK=1
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

first_available_gateway_port() {
	candidate_port=3000
	while [ "$candidate_port" -le 3099 ]; do
		if ! lsof -nP -iTCP:"$candidate_port" -sTCP:LISTEN >/dev/null 2>&1; then
			printf '%s\n' "$candidate_port"
			return 0
		fi
		candidate_port=$((candidate_port + 1))
	done
	die "no available local gateway port was found from 3000 through 3099"
}

ensure_default_gateway_port() {
	configured_gateway_port="$(env_value PLE_GATEWAY_HOST_PORT)"
	configured_gateway_port="${configured_gateway_port:-3000}"
	if lsof -nP -iTCP:"$configured_gateway_port" -sTCP:LISTEN >/dev/null 2>&1; then
		gateway_container_is_running && return 0
		available_gateway_port="$(first_available_gateway_port)"
		write_env_value PLE_GATEWAY_HOST_PORT "$available_gateway_port"
		echo "==> Port ${configured_gateway_port} is occupied; using gateway port ${available_gateway_port}"
	elif [ -z "$(env_value PLE_GATEWAY_HOST_PORT)" ]; then
		write_env_value PLE_GATEWAY_HOST_PORT "$configured_gateway_port"
	fi
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

bootstrap_local_identities() {
	[ -r "$LOCAL_IDENTITY_FILE" ] && [ -r "$LOCAL_CREDENTIAL_FILE" ] && return 0

	instructor_record="$(local_credential_record)"
	student_record="$(local_credential_record)"
	instructor_credential="${instructor_record%%	*}"
	instructor_hash="${instructor_record#*	}"
	student_credential="${student_record%%	*}"
	student_hash="${student_record#*	}"

	umask 077
	printf 'instructor=%s\nstudent=%s\n' \
		"$instructor_credential" "$student_credential" >"$LOCAL_CREDENTIAL_FILE"
	printf '{"credentials":[{"credential_sha256":"%s","tenant_id":"%s","user_id":"%s","display_name":"Local Instructor","roles":["instructor","administrator"]},{"credential_sha256":"%s","tenant_id":"%s","user_id":"%s","display_name":"Local Student","roles":["student"]}]}\n' \
		"$instructor_hash" "$LOCAL_TENANT_ID" "$LOCAL_INSTRUCTOR_ID" \
		"$student_hash" "$LOCAL_TENANT_ID" "$LOCAL_STUDENT_ID" >"$LOCAL_IDENTITY_FILE"
	# The container runs as a fixed non-root UID and needs to read hashes only;
	# the bearer credentials remain in the adjacent mode-0600 file.
	chmod 644 "$LOCAL_IDENTITY_FILE"
}

validate_webwork_secret_file() {
	secret_path="$1"
	[ -r "$secret_path" ] || die "WebWork secret $secret_path is missing or unreadable"
	if stat -f '%Lp' "$secret_path" >/dev/null 2>&1; then
		secret_mode="$(stat -f '%Lp' "$secret_path")"
	else
		secret_mode="$(stat -c '%a' "$secret_path")"
	fi
	[ "$secret_mode" = "600" ] || die "WebWork secret $secret_path must have mode 0600; fix it before launch"
	secret_value="$(cat "$secret_path")"
	printf '%s' "$secret_value" | grep -Eq '^[A-Za-z0-9_-]{43,}$' || die "WebWork secret $secret_path must be at least 32 random bytes encoded as base64url"
}

bootstrap_webwork_secret_file() {
	secret_path="$1"
	if [ -r "$secret_path" ]; then
		validate_webwork_secret_file "$secret_path"
		return 0
	fi
	secret_directory="$(dirname "$secret_path")"
	umask 077
	mkdir -p "$secret_directory"
	temporary_secret_file="$(mktemp "${secret_path}.XXXXXX")"
	openssl rand -base64 48 | tr '+/' '-_' | tr -d '\n=' >"$temporary_secret_file"
	chmod 600 "$temporary_secret_file"
	mv "$temporary_secret_file" "$secret_path"
	validate_webwork_secret_file "$secret_path"
}

bootstrap_default_local_configuration() {
	if [ ! -e "$ENV_FILE" ]; then
		cp containers/env.example "$ENV_FILE"
		chmod 600 "$ENV_FILE"
	fi
	[ -w "$ENV_FILE" ] || die "$ENV_FILE is not writable for first-run local bootstrap"
	bootstrap_local_identities
	bootstrap_webwork_secret_file "$LOCAL_WEBWORK_SECRET_FILE"
	bootstrap_webwork_secret_file "$LOCAL_WEBWORK_MOJO_SECRET_FILE"

	set_default_env_value POSTGRES_PASSWORD "$(random_hex 24)"
	set_default_env_value MINIO_ROOT_PASSWORD "$(random_hex 24)"
	set_default_env_value PLE_LOCAL_GRADER_PASSWORD "$(random_hex 24)"
	set_default_env_value PLE_WEBWORK_DATABASE_PASSWORD "$(random_hex 24)"
	set_default_env_value PLE_WEBWORK_DATABASE_ROOT_PASSWORD "$(random_hex 24)"
	set_default_env_value PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE "$REPO_ROOT/$LOCAL_WEBWORK_SECRET_FILE"
	set_default_env_value PLE_WEBWORK_MOJO_SECRET_HOST_FILE "$REPO_ROOT/$LOCAL_WEBWORK_MOJO_SECRET_FILE"
	set_default_env_value PLE_GATEWAY_IMAGE_SHA256 "$LOCAL_CADDY_IMAGE_SHA256"
	set_default_env_value PLE_POSTGRES_IMAGE_SHA256 "$LOCAL_POSTGRES_IMAGE_SHA256"
	set_default_env_value PLE_MINIO_IMAGE_SHA256 "$LOCAL_MINIO_IMAGE_SHA256"
	set_default_env_value PLE_MINIO_MC_IMAGE_SHA256 "$LOCAL_MINIO_MC_IMAGE_SHA256"
	set_default_env_value PLE_LOCAL_AUTH_HOST_FILE "$REPO_ROOT/$LOCAL_IDENTITY_FILE"
	set_default_env_value PLE_PUBLIC_ASSET_BASE_URL "http://127.0.0.1:9000/content"
	configured_renderer_url="$(env_value PLE_WEBWORK_RENDERER_BASE_URL)"
	if [ -z "$configured_renderer_url" ] || [ "$configured_renderer_url" = "http://webwork-renderer:8080" ]; then
		write_env_value PLE_WEBWORK_RENDERER_BASE_URL "http://webwork-renderer:8080/webwork2/"
	fi
	set_default_env_value PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS "15"
	set_default_env_value PLE_WEBWORK_MAX_RESPONSE_BYTES "1048576"
	set_default_env_value PLE_WEBWORK_RENDERER_ID "openwebwork-webwork2"
	set_default_env_value PLE_WEBWORK_RENDERER_VERSION "webwork2-c7060fe858cb-pg-726ff42840f9"
	set_default_env_value PLE_WEBWORK_BASE_IMAGE_SHA256 "561618e2c15bf2397621dd04f96926663a3b5616c189cf7e38db7e82f5c538ea"
	set_default_env_value PLE_SECRET_INIT_IMAGE_SHA256 "48b0309ca019d89d40f670aa1bc06e426dc0931948452e8491e3d65087abc07d"
	set_default_env_value PLE_WEBWORK2_GIT_URL "https://github.com/openwebwork/webwork2.git"
	set_default_env_value PLE_WEBWORK2_GIT_SHA "c7060fe858cb27b17aad5cf77574ff7d1ae3e1fa"
	set_default_env_value PLE_WEBWORK_PG_GIT_URL "https://github.com/openwebwork/pg.git"
	set_default_env_value PLE_WEBWORK_PG_GIT_SHA "726ff42840f968a1d6dfcc270c23c297e1d963f4"
	set_default_env_value PLE_WEBWORK_MARIADB_IMAGE_SHA256 "d9f7eb2637296652f24b484afd5d246f759f49f5babcadc6a9e344c9acb75fbf"
	set_default_env_value PLE_WEBWORK_DATABASE_NAME "webwork"
	set_default_env_value PLE_WEBWORK_DATABASE_USER "webwork"
	set_default_env_value PLE_WEBWORK_RENDER_COURSE_ID "ple-render"
	set_default_env_value PLE_WEBWORK_RENDER_USER "ple-renderer"
	ensure_default_gateway_port
}

if [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ] && [ "$CHECK_ONLY" -eq 0 ]; then
	bootstrap_default_local_configuration
fi
[ -r "$ENV_FILE" ] || die "$ENV_FILE is missing or unreadable; run without --check once to bootstrap containers/env.local"

if [ "$CHECK_ONLY" -eq 1 ] && [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ]; then
	for required_setting in PLE_POSTGRES_IMAGE_SHA256 PLE_MINIO_IMAGE_SHA256 PLE_MINIO_MC_IMAGE_SHA256 PLE_GATEWAY_IMAGE_SHA256; do
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
	if [ "$WITH_WEBWORK" -eq 1 ]; then
		"${COMPOSE_COMMAND[@]}" -f containers/compose.yaml -f containers/compose.webwork.yaml --env-file "$ENV_FILE" --profile webwork "$@"
	else
		"${COMPOSE_COMMAND[@]}" -f containers/compose.yaml --env-file "$ENV_FILE" "$@"
	fi
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

for required_setting in POSTGRES_USER POSTGRES_PASSWORD POSTGRES_DB MINIO_ROOT_USER MINIO_ROOT_PASSWORD PLE_LOCAL_GRADER_PASSWORD PLE_LOCAL_AUTH_HOST_FILE; do
	require_env_value "$required_setting"
done
for required_setting in PLE_POSTGRES_IMAGE_SHA256 PLE_MINIO_IMAGE_SHA256 PLE_MINIO_MC_IMAGE_SHA256 PLE_GATEWAY_IMAGE_SHA256; do
	require_sha256_env_value "$required_setting"
done

if [ "$WITH_WEBWORK" -eq 1 ] && [ "$CHECK_ONLY" -eq 1 ] && [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ]; then
	for required_setting in PLE_WEBWORK_BASE_IMAGE_SHA256 PLE_SECRET_INIT_IMAGE_SHA256 PLE_WEBWORK2_GIT_URL PLE_WEBWORK2_GIT_SHA PLE_WEBWORK_PG_GIT_URL PLE_WEBWORK_PG_GIT_SHA PLE_WEBWORK_MARIADB_IMAGE_SHA256 PLE_WEBWORK_DATABASE_PASSWORD PLE_WEBWORK_DATABASE_ROOT_PASSWORD PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE PLE_WEBWORK_MOJO_SECRET_HOST_FILE; do
		[ -n "$(env_value "$required_setting")" ] || die "--check --with-webwork cannot validate this pre-RC3 env.local; run './launch_local_stack.sh --with-webwork --no-open' once to safely add generated local settings"
	done
fi
if [ "$WITH_WEBWORK" -eq 1 ]; then
	for required_setting in PLE_WEBWORK_BASE_IMAGE_SHA256 PLE_SECRET_INIT_IMAGE_SHA256 PLE_WEBWORK2_GIT_URL PLE_WEBWORK2_GIT_SHA PLE_WEBWORK_PG_GIT_URL PLE_WEBWORK_PG_GIT_SHA PLE_WEBWORK_MARIADB_IMAGE_SHA256 PLE_WEBWORK_DATABASE_PASSWORD PLE_WEBWORK_DATABASE_ROOT_PASSWORD PLE_WEBWORK_RENDER_COURSE_ID PLE_WEBWORK_RENDER_USER PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE PLE_WEBWORK_MOJO_SECRET_HOST_FILE PLE_WEBWORK_RENDERER_ID PLE_WEBWORK_RENDERER_VERSION; do
		require_env_value "$required_setting"
	done
	[ -r "$(env_value PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE)" ] || die "--with-webwork render password file is missing or unreadable"
	[ -r "$(env_value PLE_WEBWORK_MOJO_SECRET_HOST_FILE)" ] || die "--with-webwork Mojolicious secret file is missing or unreadable"
	validate_webwork_secret_file "$(env_value PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE)"
	validate_webwork_secret_file "$(env_value PLE_WEBWORK_MOJO_SECRET_HOST_FILE)"
	[ "$(env_value PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE)" != "$(env_value PLE_WEBWORK_MOJO_SECRET_HOST_FILE)" ] || die "--with-webwork requires distinct render-password and Mojolicious-secret files"
	[ "$(env_value PLE_WEBWORK2_GIT_URL)" = "https://github.com/openwebwork/webwork2.git" ] || die "--with-webwork requires the official WebWork2 upstream URL"
	[ "$(env_value PLE_WEBWORK_PG_GIT_URL)" = "https://github.com/openwebwork/pg.git" ] || die "--with-webwork requires the official PG upstream URL"
	for source_ref in "$(env_value PLE_WEBWORK2_GIT_SHA)" "$(env_value PLE_WEBWORK_PG_GIT_SHA)"; do
		case "$source_ref" in
			????????????????????????????????????????) ;;
			*) die "--with-webwork requires full 40-character immutable upstream source revisions" ;;
		esac
		echo "$source_ref" | awk '/^[0-9a-f]{40}$/ { found = 1 } END { exit !found }' || die "--with-webwork source revisions must be lowercase hexadecimal SHA-1s"
	done
fi

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
if ! compose run --rm --no-deps postgres-major-guard; then
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

if [ "$WITH_WEBWORK" -eq 1 ]; then
	echo "==> Building and provisioning private upstream WeBWorK"
	# Refresh the API-owned runtime copy on every WebWork launch so a rotated
	# host password never leaves a stale secret in the named runtime volume.
	compose rm -f webwork-api-secret-init >/dev/null 2>&1 || true
	compose up -d webwork-api-secret-init
	compose up -d --build webwork-db webwork-renderer
	renderer_image_ref="localhost/ple-webwork-renderer:$(env_value PLE_WEBWORK_RENDERER_VERSION)"
	renderer_image_id="$(podman image inspect --format '{{.Id}}' "$renderer_image_ref")"
	[ -n "$renderer_image_id" ] || die "the built WebWork renderer image has no OCI image ID"
	umask 077
	printf 'image_ref=%s\nimage_id=%s\nwebwork2_sha=%s\npg_sha=%s\n' \
		"$renderer_image_ref" "$renderer_image_id" "$(env_value PLE_WEBWORK2_GIT_SHA)" "$(env_value PLE_WEBWORK_PG_GIT_SHA)" \
		>"$LOCAL_WEBWORK_PROVENANCE_FILE"
	chmod 600 "$LOCAL_WEBWORK_PROVENANCE_FILE"
	write_env_value PLE_WEBWORK_RENDERER_VERSION "${renderer_image_id#sha256:}-ww$(env_value PLE_WEBWORK2_GIT_SHA)-pg$(env_value PLE_WEBWORK_PG_GIT_SHA)"
	echo "==> Waiting for authenticated WeBWorK render_rpc readiness"
	started_at=$SECONDS
	until compose exec -T webwork-renderer /usr/local/bin/probe_render_rpc.sh >/dev/null 2>&1; do
		if [ $((SECONDS - started_at)) -ge "$TIMEOUT_SECONDS" ]; then
			echo "ERROR: WeBWorK did not pass its authenticated render_rpc probe; it has been left running for diagnosis" >&2
			compose ps webwork-db webwork-renderer >&2 || true
			compose logs --tail=80 webwork-db webwork-renderer >&2 || true
			exit 1
		fi
		sleep 2
	done
	if [ "$ENV_FILE" = "$LOCAL_ENV_FILE" ]; then
		# This host-only seed uses the production PostgreSQL and object-store
		# contracts.  It is intentionally after renderer identity finalization so
		# the API cannot be started with a pilot question for an unpinned renderer.
		echo "==> Seeding the opt-in WebWork pilot course and immutable PGML source"
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
	fi
fi

echo "==> Building images and starting API, worker, and browser gateway"
services=(api worker gateway)
compose up -d --build "${services[@]}"

gateway_port="$({
	awk -F= '
		$1 == "PLE_GATEWAY_HOST_PORT" {
			value = $2
			sub(/^[[:space:]]+/, "", value)
			sub(/[[:space:]]+$/, "", value)
			print value
		}
	' "$ENV_FILE"
} | tail -n 1)"
gateway_port="${gateway_port:-3000}"
gateway_port="${PLE_GATEWAY_HOST_PORT:-$gateway_port}"
case "$gateway_port" in
	''|*[!0-9]*) die "PLE_GATEWAY_HOST_PORT must be an unquoted integer" ;;
esac
[ "$gateway_port" -ge 1 ] && [ "$gateway_port" -le 65535 ] || die "PLE_GATEWAY_HOST_PORT must be between 1 and 65535"

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
if [ "$WITH_WEBWORK" -eq 1 ]; then
	echo "Stop it without deleting data: ${COMPOSE_COMMAND[*]} -f containers/compose.yaml -f containers/compose.webwork.yaml --env-file ${ENV_FILE} --profile webwork down"
else
	echo "Stop it without deleting data: ${COMPOSE_COMMAND[*]} -f containers/compose.yaml --env-file ${ENV_FILE} down"
fi

if [ "$OPEN_BROWSER" -eq 1 ]; then
	if command -v open >/dev/null 2>&1; then
		open "${base_url}/"
	elif command -v xdg-open >/dev/null 2>&1; then
		xdg-open "${base_url}/" >/dev/null 2>&1 &
	else
		echo "No browser opener was found; open ${base_url}/ manually."
	fi
fi
