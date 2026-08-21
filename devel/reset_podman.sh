#!/usr/bin/env bash
# Reset the ordinary local Podman demo to newly created simulated data volumes.
#
# This wrapper deliberately delegates ownership checks, Compose selection, and
# host-record cleanup to local_stack.py. Running it without --dry-run deletes:
#
# - containers_ple_pgdata
# - containers_ple_miniodata
# - containers_ple_identity_runtime
# - containers/local-chapter-one-pilot.json
set -euo pipefail

usage() {
	cat <<'EOF'
Usage: ./devel/reset_podman.sh [--dry-run]

With no option, irreversibly reset the fixed local Podman project "containers".
Use --dry-run to print the exact selected resources without deleting them.
EOF
}

dry_run=0
case "${1:-}" in
	"")
		;;
	--dry-run)
		dry_run=1
		;;
	-h|--help)
		usage
		exit 0
		;;
	*)
		usage >&2
		exit 2
		;;
esac

if [ "$#" -gt 1 ]; then
	usage >&2
	exit 2
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd -- "${script_dir}/.." && pwd -P)"
cd "$repo_root"

# Repository Python commands use source_me.sh so bytecode stays disabled and
# the supported developer environment is selected.
# repo_root is resolved above before sourcing.
# shellcheck disable=SC1091
source "$repo_root/source_me.sh"

set +e
preview_output="$(python3 "$repo_root/local_stack.py" reset --dry-run 2>&1)"
preview_status=$?
set -e

if [ "$preview_status" -ne 0 ]; then
	empty_message='ERROR: no labelled project resources were found; refusing an empty cleanup mutation'
	pilot_record="$repo_root/containers/local-chapter-one-pilot.json"
	if [ "$preview_status" -eq 2 ] && [ "$preview_output" = "$empty_message" ] && [ ! -e "$pilot_record" ]; then
		printf '%s\n' 'Local Podman project "containers" is already reset; nothing to delete.'
		exit 0
	fi
	printf '%s\n' "$preview_output" >&2
	exit "$preview_status"
fi

if [ "$dry_run" -eq 1 ]; then
	printf '%s\n' "$preview_output"
	exit 0
fi

printf '%s\n' \
	'Resetting the fixed local Podman project "containers".' \
	'This deletes its three simulated named volumes and generated Chapter One pilot record.'
exec python3 "$repo_root/local_stack.py" reset --confirm-project containers
