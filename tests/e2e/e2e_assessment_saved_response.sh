#!/usr/bin/env bash
# Disposable PostgreSQL proof that a Student save stays on the Attempt, a
# second save replaces it while the Attempt is open, and student finalization
# records the submission and grading result. The oracle calls those functions.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
name="ple-saved-response-$$"
database="ple_saved_response_$$"
password="saved-response"

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
proof="$(podman exec "$name" \
    psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" \
    -f /workspace/tests/e2e/assessment_saved_response_oracle.sql 2>&1)"
oracle_status="$?"
set -e
printf '%s\n' "$proof"
if [[ "$oracle_status" -ne 0 ]]; then
    exit "$oracle_status"
fi
echo "save_student_assessment_attempt_response lifecycle passed"
