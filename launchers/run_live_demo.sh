#!/usr/bin/env bash
# run_live_demo.sh - concise front door for the Local Stack Controller.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

usage() {
  echo "Usage: ./launchers/run_live_demo.sh [--headless|start [--open|--headless]|open|--open|stop]"
}

command="start"
headless="true"

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
      --open)
        command="open"
        ;;
      start|open|stop)
        command="$1"
        ;;
      *)
        usage >&2
        exit 2
        ;;
    esac
    ;;
  2)
    case "$1:$2" in
      start:--headless)
        ;;
      start:--open)
        headless="false"
        ;;
      *)
        usage >&2
        exit 2
        ;;
    esac
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"

case "$command" in
  open|stop)
    # ASVS 1.2.5: fixed controller path and literal arguments avoid shell evaluation.
    exec python3 "$repository_root/local_stack.py" "$command"
    ;;
esac

"$repository_root/devel/setup_typescript.sh"

case "$headless" in
  true)
    exec python3 "$repository_root/local_stack.py" start --headless
    ;;
  false)
    exec python3 "$repository_root/local_stack.py" start
    ;;
esac
