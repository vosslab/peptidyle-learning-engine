#!/usr/bin/env bash
# setup_python.sh - create or refresh the fixed repo-local Python environment.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
VENV_DIRECTORY="$REPO_ROOT/.venv"
VENV_PYTHON="$VENV_DIRECTORY/bin/python"
RECEIPT_PATH="$VENV_DIRECTORY/.setup_python_receipt"

if command -v python3.12 >/dev/null 2>&1; then
	PYTHON_312="python3.12"
elif command -v python3 >/dev/null 2>&1 \
	&& [ "$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')" = "3.12" ]; then
	PYTHON_312="python3"
else
	echo "ERROR: Python 3.12 is required. Install python3.12, then rerun ./run_live_demo.sh." >&2
	exit 1
fi

if [ "$("$PYTHON_312" -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')" != "3.12" ]; then
	echo "ERROR: $PYTHON_312 must be Python 3.12." >&2
	exit 1
fi

requirements_digest="$("$PYTHON_312" -c '
import hashlib
import pathlib
import sys

digest = hashlib.sha256()
for requirements_path in sys.argv[1:]:
    digest.update(pathlib.Path(requirements_path).read_bytes())
    digest.update(b"\0")
print(digest.hexdigest())
' "$REPO_ROOT/pip_requirements.txt" "$REPO_ROOT/pip_requirements-dev.txt")"
interpreter_identity="$("$PYTHON_312" -c 'import sys; print(sys.implementation.name + " " + sys.version)')"
expected_receipt="python=$interpreter_identity
requirements_sha256=$requirements_digest"

environment_is_current() {
	[ -x "$VENV_PYTHON" ] \
		&& [ "$("$VENV_PYTHON" -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')" = "3.12" ] \
		&& [ -f "$RECEIPT_PATH" ] \
		&& [ "$(cat "$RECEIPT_PATH")" = "$expected_receipt" ]
}

if environment_is_current; then
	echo "Python environment is current."
else
	echo "==> Preparing the repo-local Python environment"
	if [ -d "$VENV_DIRECTORY" ]; then
		rm -rf "$VENV_DIRECTORY"
	fi
	"$PYTHON_312" -m venv "$VENV_DIRECTORY"
	"$VENV_PYTHON" -m pip install --requirement "$REPO_ROOT/pip_requirements-dev.txt"
	printf '%s\n' "$expected_receipt" > "$RECEIPT_PATH"
	echo "Python environment is ready."
fi

"$VENV_PYTHON" -c 'import yaml; print("PyYAML import verified.")'
