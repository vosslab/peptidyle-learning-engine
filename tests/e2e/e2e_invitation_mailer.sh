#!/usr/bin/env bash
# e2e_invitation_mailer.sh - run the disposable invitation-mailer workflow oracle.

set -uo pipefail
script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "$script_directory" rev-parse --show-toplevel)"
cd "$repo_root" || exit 1

source source_me.sh
exec python3 tests/e2e/e2e_invitation_mailer.py
