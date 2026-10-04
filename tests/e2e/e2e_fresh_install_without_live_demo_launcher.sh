#!/usr/bin/env bash
# Fresh installation-data provision without the Live Demo launcher.
# The shipped migrator command is `installation-data provision`.

set -euo pipefail

fresh_install_includes_live_demo_and_genetics() {
	local repository_root
	repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
	local runtime="${INSTALL_RUNTIME_DIR:-$(mktemp -d)}"
	local log="${INSTALL_LOG:-$runtime/install.log}"
	local network="ple-hg-install-$$"
	local postgres_name="ple-hg-install-db-$$"
	local minio_name="ple-hg-install-minio-$$"
	local bucket_name="ple-hg-install-buckets-$$"
	local renderer_name="ple-hg-install-renderer-$$"
	local worker_name="ple-hg-install-worker-$$"
	local api_name="ple-hg-install-api-$$"
	local migrator_name="ple-hg-install-migrator-$$"
	local secrets_volume="ple-hg-install-secrets-$$"
	local migrator_image="localhost/peptidyle-learning-engine:local-database-migrator"
	local api_image="localhost/peptidyle-learning-engine:local"
	local minio_image="localhost/ple-object-storage:reviewed"

	cleanup() {
		set +u
		podman rm --force --volumes \
			"$migrator_name" "$api_name" "$renderer_name" "$worker_name" "$bucket_name" \
			"$minio_name" "$postgres_name" >>"$log" 2>&1 || true
		podman network rm "$network" >>"$log" 2>&1 || true
		podman volume rm --force "$secrets_volume" >>"$log" 2>&1 || true
	}
	trap cleanup EXIT
	: >"$log"
	mkdir -p "$runtime"
	chmod 700 "$runtime"
	rm -f "$runtime/bootstrap.sql" "$runtime/api_login.sql" "$runtime/child.env"

	{
		echo "preparing credentials"
		cd "$repository_root"
		# shellcheck disable=SC1091
		source ./source_me.sh
		export PYTHONPATH="$repository_root${PYTHONPATH:+:$PYTHONPATH}"
		export INSTALL_RUNTIME="$runtime"
		python3 - <<'PY'
import os
import pathlib
import secrets
from urllib.parse import urlsplit

import local_stack_control.lifecycle_database as lifecycle_database

runtime = pathlib.Path(os.environ["INSTALL_RUNTIME"])
admin_password = secrets.token_hex(16)
migrator_password = secrets.token_hex(16)
api_password = secrets.token_hex(16)
minio_password = secrets.token_hex(16)
bootstrap = lifecycle_database.migration_principal_bootstrap_sql(
    "ple_e2e_baseline", migrator_password
)
api_sql = f"""
CREATE ROLE ple_api_login LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT
    NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 8 PASSWORD '{api_password}';
GRANT CONNECT ON DATABASE ple_e2e_baseline TO ple_api_login;
GRANT ple_app TO ple_api_login WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
GRANT ple_auth TO ple_api_login WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
"""

def write_private(path: pathlib.Path, content: str) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "w", encoding="ascii") as handle:
        handle.write(content)
    os.chmod(path, 0o600)

write_private(runtime / "bootstrap.sql", bootstrap)
write_private(runtime / "api_login.sql", api_sql)
write_private(
    runtime / "child.env",
    "\n".join([
        f"ADMIN_PASSWORD={admin_password}",
        f"MIGRATOR_PASSWORD={migrator_password}",
        f"API_PASSWORD={api_password}",
        f"MINIO_PASSWORD={minio_password}",
    ]) + "\n",
)
print("credentials ready")
PY
	} >>"$log" 2>&1

	# shellcheck disable=SC1091
	source "$runtime/child.env"
	echo "starting disposable postgres and object storage" | tee -a "$log"
	podman network create "$network" >>"$log" 2>&1
	podman run --detach --name "$postgres_name" --network "$network" --network-alias postgres \
		--env POSTGRES_USER=ple_e2e_migrator \
		--env POSTGRES_PASSWORD="$ADMIN_PASSWORD" \
		--env POSTGRES_DB=ple_e2e_baseline \
		--volume "$repository_root:/workspace:ro" \
		docker.io/library/postgres:17 >>"$log" 2>&1
	podman run --detach --name "$minio_name" --network "$network" --network-alias minio \
		--user 0:0 \
		--env MINIO_ROOT_USER=pleinstall \
		--env MINIO_ROOT_PASSWORD="$MINIO_PASSWORD" \
		"$minio_image" server /data >>"$log" 2>&1

	local ready=0
	local attempt
	for attempt in $(seq 1 90); do
		if podman exec "$postgres_name" pg_isready -U ple_e2e_migrator -d ple_e2e_baseline >>"$log" 2>&1; then
			ready=1
			break
		fi
		sleep 1
	done
	if [[ "$ready" -ne 1 ]]; then
		echo "FAILED: postgres did not become ready"
		exit 1
	fi

	echo "bootstrapping roles and installing the current schema" | tee -a "$log"
	podman exec --interactive --env PGPASSWORD="$ADMIN_PASSWORD" "$postgres_name" \
		psql -X -v ON_ERROR_STOP=1 -U ple_e2e_migrator -d ple_e2e_baseline \
		<"$runtime/bootstrap.sql" >>"$log" 2>&1
	podman exec --env PGPASSWORD="$MIGRATOR_PASSWORD" "$postgres_name" \
		psql -X -v ON_ERROR_STOP=1 --single-transaction \
		-U ple_migrator -d ple_e2e_baseline \
		-f /workspace/schemas/base_schema/install.sql >>"$log" 2>&1
	podman exec --interactive --env PGPASSWORD="$ADMIN_PASSWORD" "$postgres_name" \
		psql -X -v ON_ERROR_STOP=1 -U ple_e2e_migrator -d ple_e2e_baseline \
		<"$runtime/api_login.sql" >>"$log" 2>&1

	echo "creating object-storage buckets" | tee -a "$log"
	podman run --name "$bucket_name" --network "$network" \
		--entrypoint /bin/sh \
		--env MINIO_PASSWORD="$MINIO_PASSWORD" \
		"$minio_image" \
		-ec 'mc alias set local http://minio:9000 pleinstall "$MINIO_PASSWORD" && mc mb --ignore-existing local/public-assets && mc mb --ignore-existing local/private-content && mc mb --ignore-existing local/student-records && mc mb --ignore-existing local/temp-processing' \
		>>"$log" 2>&1

	echo "starting readiness listeners and API without the Live Demo launcher" | tee -a "$log"
	podman volume create "$secrets_volume" >>"$log" 2>&1
	podman run --rm --volume "$secrets_volume:/secrets" \
		docker.io/library/alpine:latest \
		sh -ec 'printf "%s\n" "oci_id=sha256:abababababababababababababababababababababababababababababababab" > /secrets/question-renderer-version && chmod 644 /secrets/question-renderer-version' \
		>>"$log" 2>&1
	podman run --detach --name "$renderer_name" --network "$network" \
		--network-alias webwork-renderer docker.io/library/alpine:latest \
		sh -c 'while true; do nc -l -p 3000; done' >>"$log" 2>&1
	podman run --detach --name "$worker_name" --network "$network" \
		--network-alias worker docker.io/library/alpine:latest \
		sh -c 'while true; do nc -l -p 3001; done' >>"$log" 2>&1
	podman run --detach --name "$api_name" --network "$network" --network-alias api \
		--user 10001:10001 \
		--read-only \
		--tmpfs /tmp:rw,noexec,nosuid,size=64m,mode=1777 \
		--volume "$secrets_volume:/run/ple-secrets:ro" \
		--env PLE_BIND_ADDR=0.0.0.0:3000 \
		--env PLE_BROWSER_ORIGIN=https://localhost \
		--env PLE_STORAGE_TOPOLOGY=disposable-local \
		--env DATABASE_URL="postgres://ple_api_login:${API_PASSWORD}@postgres:5432/ple_e2e_baseline" \
		--env PLE_S3_ENDPOINT=http://minio:9000 \
		--env PLE_S3_REGION=us-east-1 \
		--env PLE_PUBLIC_ASSETS_BUCKET=public-assets \
		--env PLE_PRIVATE_CONTENT_BUCKET=private-content \
		--env PLE_STUDENT_RECORDS_BUCKET=student-records \
		--env PLE_TEMP_PROCESSING_BUCKET=temp-processing \
		--env PLE_PUBLIC_ASSET_BASE_URL=http://minio:9000/public-assets/ \
		--env PLE_WEBWORK_RENDERER_BASE_URL=http://webwork-renderer:3000/ \
		--env PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS=15 \
		--env PLE_WEBWORK_MAX_RESPONSE_BYTES=1048576 \
		--env PLE_WEBWORK_RENDERER_ID=vosslab-webwork-pg-renderer \
		--env PLE_WEBWORK_RENDERER_VERSION_FILE=/run/ple-secrets/question-renderer-version \
		--env AWS_ACCESS_KEY_ID=pleinstall \
		--env AWS_SECRET_ACCESS_KEY="$MINIO_PASSWORD" \
		"$api_image" >>"$log" 2>&1

	ready=0
	for attempt in $(seq 1 60); do
		if podman exec "$api_name" /usr/local/bin/peptidyle-api --health-probe >>"$log" 2>&1; then
			ready=1
			break
		fi
		sleep 2
	done
	if [[ "$ready" -ne 1 ]]; then
		podman run --rm --network "$network" docker.io/library/alpine:latest \
			sh -c "printf 'GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n' | nc api 3000" \
			>>"$log" 2>&1 || true
		echo "FAILED: API did not become ready"
		exit 1
	fi

	echo "running default installation-data provision" | tee -a "$log"
	podman run --name "$migrator_name" --network "$network" \
		--env PLE_MIGRATION_DATABASE_URL="postgres://ple_migrator:${MIGRATOR_PASSWORD}@postgres:5432/ple_e2e_baseline" \
		--env DATABASE_URL="postgres://ple_api_login:${API_PASSWORD}@postgres:5432/ple_e2e_baseline" \
		--env PLE_STORAGE_TOPOLOGY=disposable-local \
		--env PLE_BROWSER_ORIGIN=https://localhost \
		--env PLE_S3_ENDPOINT=http://minio:9000 \
		--env PLE_S3_REGION=us-east-1 \
		--env PLE_PUBLIC_ASSETS_BUCKET=public-assets \
		--env PLE_PRIVATE_CONTENT_BUCKET=private-content \
		--env PLE_STUDENT_RECORDS_BUCKET=student-records \
		--env PLE_TEMP_PROCESSING_BUCKET=temp-processing \
		--env AWS_ACCESS_KEY_ID=pleinstall \
		--env AWS_SECRET_ACCESS_KEY="$MINIO_PASSWORD" \
		"$migrator_image" installation-data provision >>"$log" 2>&1 \
		|| { echo "FAILED: installation-data provision"; exit 1; }

	echo "checking Live Demo course and Genetics example" | tee -a "$log"
	local report
	report="$(podman exec --interactive --env PGPASSWORD="$ADMIN_PASSWORD" "$postgres_name" \
		psql -X -v ON_ERROR_STOP=1 -U ple_e2e_migrator -d ple_e2e_baseline -At <<'SQL'
SELECT course_short_name || '|' || course_long_name
  FROM ple_data.course_instance
 WHERE course_short_name = 'BCHM 301';
SELECT short_name || '|' || long_name || '|' || availability::text
  FROM ple_data.blueprint_course
 WHERE short_name = 'Genetics';
SQL
)"
	printf '%s\n' "$report" >>"$log"
	if [[ "$report" != *$'BCHM 301|Biochemistry 301: Proteins and Peptides'* ]]; then
		echo "FAILED: default provision did not include the Live Demo course"
		exit 1
	fi
	if [[ "$report" != *$'Genetics|Fall Genetics|public'* ]]; then
		echo "FAILED: default provision did not include the public Genetics example"
		exit 1
	fi
	cleanup
	trap - EXIT
	echo "DONE: fresh install included the Live Demo course and the public Genetics example"
}

fresh_install_includes_live_demo_and_genetics
