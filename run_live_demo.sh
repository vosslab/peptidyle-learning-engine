#!/usr/bin/env bash
# run_live_demo.sh - concise front door for the Local Stack Controller.

set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

usage() {
  echo "Usage: ./run_live_demo.sh [--headless|start [--headless]|stop]"
}

command="start"
headless="false"

case "$#" in
  0)
    ;;
  1)
    case "$1" in
      --help|-h)
        usage
        exit 0
        ;;
      --headless)
        headless="true"
        ;;
      start|stop)
        command="$1"
        ;;
      *)
        usage >&2
        exit 2
        ;;
    esac
    ;;
  2)
    if [ "$1" = "start" ] && [ "$2" = "--headless" ]; then
      headless="true"
    else
      usage >&2
      exit 2
    fi
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

# shellcheck disable=SC1091
source "$script_directory/source_me.sh"

if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 is required. Follow docs/INSTALL.md." >&2
  exit 1
fi

python_version="$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')"
if [ "$python_version" != "3.12" ]; then
  echo "ERROR: python3 must be Python 3.12; found $python_version. Follow docs/INSTALL.md." >&2
  exit 1
fi

if ! python3 -c 'import yaml' >/dev/null 2>&1; then
  echo "ERROR: PyYAML is required. Follow docs/INSTALL.md." >&2
  exit 1
fi

if [ "$command" = "stop" ]; then
  # ASVS 1.2.5: fixed controller path and literal arguments avoid shell evaluation.
  exec python3 "$script_directory/local_stack.py" stop
fi

if [ ! -d "$script_directory/node_modules" ]; then
  echo "==> First launch: installing repository dependencies"
  "$script_directory/devel/setup_typescript.sh"
fi

if [ "$headless" = "true" ]; then
  # ASVS 1.2.5: fixed controller path and literal arguments avoid shell evaluation.
  exec python3 "$script_directory/local_stack.py" start --headless
fi

# ASVS 1.2.5: fixed controller path and literal arguments avoid shell evaluation.
exec python3 "$script_directory/local_stack.py" start
