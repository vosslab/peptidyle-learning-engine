#!/usr/bin/env bash
# Disposable PostgreSQL proof that start_assessment_attempt resumes an open
# Attempt and keeps the stored expires_at. The oracle calls that function.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
name="ple-resume-expiration-$$"
database="ple_resume_expiration_$$"
password="resume-expiration"
sql_file="$root/tests/e2e/assessment_resume_expiration_oracle.sql"

cleanup() {
    status="$?"
    podman rm --force --volumes "$name" >/dev/null 2>&1 || true
    return "$status"
}
trap cleanup EXIT

source "$root/source_me.sh"
cd "$root"

podman run --detach --name "$name" \
    --mount "type=bind,src=$root,dst=/workspace,ro=true" \
    --env POSTGRES_PASSWORD=postgres --env POSTGRES_DB="$database" \
    docker.io/library/postgres:17 >/dev/null
for _ in $(seq 1 40); do
    if podman exec "$name" pg_isready -U postgres -d "$database" >/dev/null; then
        break
    fi
    sleep 1
done
podman exec "$name" pg_isready -U postgres -d "$database" >/dev/null

python3 -c "import local_stack_control.lifecycle_database as database; print(database.migration_principal_bootstrap_sql('${database}', '${password}'), end='')" \
    | podman exec -i "$name" psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" >/dev/null
podman exec --env PGPASSWORD="$password" "$name" \
    psql -X -v ON_ERROR_STOP=1 --single-transaction -U ple_migrator -d "$database" \
    -f /workspace/schemas/base_schema/install.sql >/dev/null
echo "schema install finished"
set +e
proof="$(podman exec -i "$name" \
    psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" < "$sql_file" 2>&1)"
oracle_status="$?"
set -e
printf '%s\n' "$proof"
if [[ "$oracle_status" -ne 0 ]]; then
    exit "$oracle_status"
fi
printf '%s\n' "$proof" | rg -q 'start_assessment_attempt resume same_attempt kept_expires_at'
printf '%s\n' "$proof" | rg -q 'start_assessment_attempt resume same_attempt from_distinct_browser_sessions'
echo "start_assessment_attempt resume kept the original expires_at"
echo "start_assessment_attempt resume kept the Attempt across two browser sessions"
