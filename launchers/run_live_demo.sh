#!/usr/bin/env bash
# run_live_demo.sh - concise front door for the Local Stack Controller.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

usage() {
  echo "Usage: ./launchers/run_live_demo.sh [--headless|--open|start [--open|--headless]|open|stop]"
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
        headless="false"
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

# Only an explicit `stop` clears a running suite; `start` reports its URL. A launch receipt
# counts as still starting only while the supervisor holds the lease lock.
suite_state="$repository_root/local_stack_state/live_demo_browser"
control_receipt="$suite_state/developer-control.json"
if [[ -f "$suite_state/developer-launch.json" && ! -f "$control_receipt" ]] \
  && lsof "$suite_state/browser-suite.lock" >/dev/null 2>&1; then
  echo "Live Demo is still starting; wait for its ready URL or run ./launchers/run_live_demo.sh stop." >&2
  exit 1
fi
if [[ -f "$control_receipt" ]]; then
  entry_url="$(jq -r '.origin' "$control_receipt")sign-in"
  echo "Live demo entry: $entry_url"
  echo "Live Demo is already running; use ./launchers/run_live_demo.sh stop to clear it."
  case "$headless" in
    false)
      exec python3 "$repository_root/local_stack.py" open
      ;;
  esac
  exit 0
fi

npm ls --depth=0 --json >/dev/null 2>&1 || "$repository_root/devel/setup_typescript.sh"

case "$headless" in
  true)
    exec python3 "$repository_root/local_stack.py" start --headless
    ;;
  false)
    exec python3 "$repository_root/local_stack.py" start
    ;;
esac
