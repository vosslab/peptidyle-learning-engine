#!/usr/bin/env bash
# Prove bounded public readiness across every fixed Live Demo dependency.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly manifest_path="local_stack_state/live_demo_browser/workspace/disposable.manifest"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_readiness.sh [--healthy|--outages]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--healthy) mode="healthy" ;;
	--outages) mode="outages" ;;
	*)
		usage
		exit 2
		;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

active_fault=""

recover_active_fault() {
	if [ -n "$active_fault" ]; then
		python3 -m local_stack_control.disposable_stack_command recover-readiness-dependency \
			--manifest "$manifest_path" --service "$active_fault" >/dev/null 2>&1 || true
	fi
}
trap recover_active_fault EXIT

require_live_demo_manifest() {
	if [ ! -f "$manifest_path" ] || [ ! -f "$runtime_environment_path" ]; then
		echo "readiness requires the fixed Live Demo to be running" >&2
		exit 2
	fi
}

gateway_id() {
	local identifiers
	local line_count
	identifiers="$(podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=gateway")"
	if [ -z "$identifiers" ]; then
		echo "readiness requires one running gateway" >&2
		exit 1
	fi
	line_count="$(printf '%s\n' "$identifiers" | wc -l | tr -d '[:space:]')"
	if [ "$line_count" != "1" ]; then
		echo "readiness found an ambiguous gateway service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	local line_count
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	line_count="$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')"
	if [ "$line_count" != "1" ]; then
		echo "readiness requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

readiness_response() {
	local gateway
	local port
	gateway="$(gateway_id)"
	port="$(gateway_port)"
	podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 \
		--write-out $'\n%{http_code}' --header "Host: localhost:$port" \
		https://localhost:8080/health
}

readiness_matches() {
	local expected_status="$1"
	local expected_body="$2"
	local response
	local body
	local status
	response="$(readiness_response)" || return 1
	status="${response##*$'\n'}"
	body="${response%$'\n'*}"
	[ "$status" = "$expected_status" ] && [ "$body" = "$expected_body" ]
}

require_healthy() {
	python3 local_stack.py status --project "$project_name"
	if ! readiness_matches "200" '{"status":"ready","unavailable":[]}'; then
		echo "readiness did not report the expected healthy body" >&2
		exit 1
	fi
	echo "Readiness healthy: api database object-store renderer worker"
}

wait_for_healthy() {
	local attempt
	for attempt in $(seq 1 45); do
		if python3 local_stack.py status --project "$project_name" >/dev/null 2>&1 \
			&& readiness_matches "200" '{"status":"ready","unavailable":[]}'; then
			return 0
		fi
		sleep 1
	done
	echo "readiness did not recover within 45 seconds" >&2
	return 1
}

prove_outage() {
	local service="$1"
	local unavailable="$2"
	local expected_body
	expected_body="{\"status\":\"degraded\",\"unavailable\":[\"$unavailable\"]}"
	python3 -m local_stack_control.disposable_stack_command stop-readiness-dependency \
		--manifest "$manifest_path" --service "$service"
	active_fault="$service"
	if ! readiness_matches "503" "$expected_body"; then
		echo "readiness outage did not report only $unavailable for $service" >&2
		return 1
	fi
	echo "Readiness outage: $service -> $unavailable"
	python3 -m local_stack_control.disposable_stack_command recover-readiness-dependency \
		--manifest "$manifest_path" --service "$service"
	if ! wait_for_healthy; then
		return 1
	fi
	active_fault=""
}

require_live_demo_manifest
require_healthy
case "$mode" in
	healthy) ;;
	outages|all)
		prove_outage api api
		prove_outage postgres database
		prove_outage minio object-store
		prove_outage webwork-renderer renderer
		prove_outage worker worker
		;;
esac

echo "Live Demo readiness: PASS"
