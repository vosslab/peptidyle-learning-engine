#!/usr/bin/env bash
# Run the Store integration oracle against the lease-owned staged PostgreSQL 17 stack.
set -euo pipefail

workspace="${1:?workspace is required}"
compose_environment="$workspace/secrets/compose.env"
admin_url_file="$workspace/secrets/postgres-admin.url"
[ -f "$compose_environment" ] && [ -f "$admin_url_file" ] || {
	echo "iMathAS Question Backend Session oracle: private staged runtime is unavailable" >&2
	exit 2
}
postgres_port="$(sed -n 's/^PLE_POSTGRES_HOST_PORT=//p' "$compose_environment")"
case "$postgres_port" in
	''|*[!0-9]*)
		echo "iMathAS Question Backend Session oracle: staged PostgreSQL port is invalid" >&2
		exit 2
		;;
esac
admin_url="$(tr -d '\n' < "$admin_url_file")"
app_url="postgres://ple_api_login:imathasquestionbackendoracle@127.0.0.1:${postgres_port}/ple_e2e_baseline"
PLE_IMATHAS_QUESTION_BACKEND_SESSION_DATABASE_URL="$app_url" \
	PLE_IMATHAS_QUESTION_BACKEND_SESSION_ADMIN_DATABASE_URL="$admin_url" \
	cargo test -p learning-data-access --features postgres \
		--test imathas_question_backend_session_postgres -- --ignored --test-threads=1
