#!/usr/bin/env bash
# capture_screenshots.sh - publish or live-verify the manifest screenshot corpus.

set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repository_root="$(dirname "$script_directory")"
# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
exec python3 "$script_directory/capture_screenshots.py" "$@"
