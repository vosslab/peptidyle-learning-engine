#!/usr/bin/env bash
# Disposable PostgreSQL proof that an expired Assessment Attempt is submitted
# by the expiry worker and that its saved response is finalized. The oracle
# calls those functions after the Attempt time limit elapses.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
name="ple-attempt-expiry-$$"
database="ple_attempt_expiry_$$"
password="attempt-expiry"
sql_file="$root/tests/e2e/assessment_attempt_expiry_oracle.sql"

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
printf '%s\n' "$proof" | rg -q 'commit_expired_student_assessment_attempt_finalization finalized_saved_response'
echo "commit_expired_student_assessment_attempt_finalization lifecycle passed"
