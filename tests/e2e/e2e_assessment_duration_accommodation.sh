#!/usr/bin/env bash
# Disposable PostgreSQL proof that assessment_effective_duration_seconds applies
# a Student multiplier after the stored Assessment time limit and stops at
# 86400 seconds. The oracle calls that shipped function. It does not restate
# the formula in another language.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
name="ple-duration-cap-$$"
database="ple_duration_$$"
password="duration-accommodation"
sql_file="$root/tests/e2e/assessment_duration_accommodation_oracle.sql"

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
proof="$(podman exec -i "$name" \
    psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" < "$sql_file" 2>&1)"
printf '%s\n' "$proof"
printf '%s\n' "$proof" | rg -q 'assessment_effective_duration_seconds standard 1800 one_and_half 2700 two_times 3600 capped 86400 fractional 152 fractional_cap 86400'
echo "assessment_effective_duration_seconds accommodation cap passed"
