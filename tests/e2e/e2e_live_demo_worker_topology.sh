#!/usr/bin/env bash
# Prove the one worker Service Identity in the fixed disposable Live Demo.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly manifest_path="local_stack_state/live_demo_browser/workspace/disposable.manifest"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_worker_topology.sh [--service|--lifecycle]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--service) mode="service" ;;
	--lifecycle) mode="lifecycle" ;;
	*)
		usage
		exit 2
		;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo_manifest() {
	if [ ! -f "$manifest_path" ]; then
		echo "worker topology requires the fixed Live Demo to be running" >&2
		exit 2
	fi
}

worker_id() {
	local ids
	local line_count
	ids="$(podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=worker")"
	if [ -z "$ids" ]; then
		echo "worker topology requires one running worker" >&2
		exit 1
	fi
	line_count="$(printf '%s\n' "$ids" | wc -l | tr -d '[:space:]')"
	if [ "$line_count" != "1" ]; then
		echo "worker topology found an ambiguous worker service" >&2
		exit 1
	fi
	printf '%s\n' "$ids"
}

require_worker_service() {
	local identifier
	local health
	local networks
	local default_network_internal
	local host_ports
	local command
	identifier="$(worker_id)"
	health="$(podman inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{end}}' "$identifier")"
	if [ "$health" != "healthy" ]; then
		echo "worker topology requires a healthy worker" >&2
		exit 1
	fi
	networks="$(podman inspect --format '{{range $name, $_ := .NetworkSettings.Networks}}{{$name}}{{"\n"}}{{end}}' "$identifier")"
	if [ "$networks" != "$project_name"_default ]; then
		echo "worker topology grants an unexpected network" >&2
		exit 1
	fi
	default_network_internal="$(podman network inspect --format '{{.Internal}}' "$project_name"_default)"
	if [ "$default_network_internal" != "true" ]; then
		echo "worker topology requires an internal dependency network" >&2
		exit 1
	fi
	host_ports="$(podman port "$identifier" 2>/dev/null || true)"
	if [ -n "$host_ports" ]; then
		echo "worker topology must not publish a host port" >&2
		exit 1
	fi
	command="$(podman inspect --format '{{json .Config.Cmd}}' "$identifier")"
	if [ "$command" != '["--worker"]' ]; then
		echo "worker topology has an unexpected process mode" >&2
		exit 1
	fi
}

replace_worker() {
	local before
	local after
	local candidate
	local health
	before="$(worker_id)"
	python3 -m local_stack_control.disposable_stack_command stop-worker \
		--manifest "$manifest_path"
	if podman ps -q \
		--filter "label=com.docker.compose.project=$project_name" \
		--filter "label=com.docker.compose.service=worker" | rg -q .; then
		echo "worker topology stop did not stop the worker" >&2
		exit 1
	fi
	python3 -m local_stack_control.disposable_stack_command replace-worker \
		--manifest "$manifest_path"
	for _ in $(seq 1 30); do
		candidate="$(podman ps -q \
			--filter "label=com.docker.compose.project=$project_name" \
			--filter "label=com.docker.compose.service=worker")"
		if [ -n "$candidate" ]; then
			health="$(podman inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{end}}' "$candidate")"
			if [ "$health" = "healthy" ]; then
				require_worker_service
				break
			fi
		fi
		if [ "$_" = "30" ]; then
			echo "worker topology replacement did not become healthy" >&2
			exit 1
		fi
		sleep 1
	done
	after="$(worker_id)"
	if [ "$after" = "$before" ]; then
		echo "worker topology replacement retained the previous container" >&2
		exit 1
	fi
}

require_live_demo_manifest
case "$mode" in
	service)
		require_worker_service
		;;
	lifecycle)
		require_worker_service
		replace_worker
		;;
	all)
		require_worker_service
		replace_worker
		;;
esac

echo "Live Demo worker topology: PASS"
