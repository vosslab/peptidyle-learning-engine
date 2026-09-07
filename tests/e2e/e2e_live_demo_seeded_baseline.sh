#!/usr/bin/env bash
# Prove the fixed Live Demo seed installs and replays without duplicate records.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly manifest_path="local_stack_state/live_demo_browser/workspace/disposable.manifest"
readonly expected_inventory="5|4|4|4|4|1|1|1|1"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_seeded_baseline.sh [--install|--replay]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--install) mode="install" ;;
	--replay) mode="replay" ;;
	*)
		usage
		exit 2
		;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

start_fixed_demo() {
	python3 local_stack.py start --headless
}

read_inventory() {
	python3 -m local_stack_control.disposable_stack_command seed-inventory \
		--manifest "$manifest_path"
}

require_baseline() {
	local inventory
	if [ ! -f "$manifest_path" ]; then
		echo "seeded baseline requires the fixed Live Demo manifest" >&2
		exit 1
	fi
	python3 local_stack.py status --project "$project_name"
	inventory="$(read_inventory)"
	if [ "$inventory" != "$expected_inventory" ]; then
		echo "seeded baseline inventory was $inventory, not the expected answer-free manifest" >&2
		exit 1
	fi
	bash tests/e2e/e2e_live_demo_readiness.sh --healthy
	echo "Seeded baseline installed: Accounts Published Questions private sources and one pending Question Asset publication chain"
}

install_baseline() {
	start_fixed_demo
	require_baseline
}

replay_baseline() {
	local before
	local after
	before="$(read_inventory)"
	if [ "$before" != "$expected_inventory" ]; then
		echo "seeded baseline was not installed before replay" >&2
		exit 1
	fi
	start_fixed_demo
	require_baseline
	after="$(read_inventory)"
	if [ "$after" != "$before" ]; then
		echo "seeded baseline replay changed its immutable inventory" >&2
		exit 1
	fi
	echo "Seeded baseline replay: no duplicate Accounts or Published Questions"
}

case "$mode" in
	install)
		install_baseline
		;;
	replay|all)
		install_baseline
		replay_baseline
		;;
esac

echo "Live Demo seeded baseline: PASS"
