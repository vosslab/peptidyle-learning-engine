#!/usr/bin/env bash
# setup_python.sh - create or refresh the fixed repo-local Python environment.

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
venv_directory="$repo_root/.venv"
venv_python="$venv_directory/bin/python"
receipt_path="$venv_directory/.setup_python_receipt"

if command -v python3.12 >/dev/null 2>&1; then
	python_312="python3.12"
elif command -v python3 >/dev/null 2>&1 \
	&& [ "$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')" = "3.12" ]; then
	python_312="python3"
else
	echo "ERROR: Python 3.12 is required. Install python3.12, then rerun ./run_live_demo.sh." >&2
	exit 1
fi

if [ "$("$python_312" -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')" != "3.12" ]; then
	echo "ERROR: $python_312 must be Python 3.12." >&2
	exit 1
fi

requirements_digest="$("$python_312" -c '
import hashlib
import pathlib
import sys

digest = hashlib.sha256()
for requirements_path in sys.argv[1:]:
    digest.update(pathlib.Path(requirements_path).read_bytes())
    digest.update(b"\0")
print(digest.hexdigest())
' "$repo_root/pip_requirements.txt" "$repo_root/pip_requirements-dev.txt")"
interpreter_identity="$("$python_312" -c 'import sys; print(sys.implementation.name + " " + sys.version)')"
expected_receipt="python=$interpreter_identity
requirements_sha256=$requirements_digest"

environment_is_current() {
	[ -x "$venv_python" ] \
		&& [ "$("$venv_python" -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')" = "3.12" ] \
		&& [ -f "$receipt_path" ] \
		&& [ "$(cat "$receipt_path")" = "$expected_receipt" ]
}

if environment_is_current; then
	echo "Python environment is current."
else
	echo "==> Preparing the repo-local Python environment"
	if [ -d "$venv_directory" ]; then
		rm -rf "$venv_directory"
	fi
	"$python_312" -m venv "$venv_directory"
	"$venv_python" -m pip install --requirement "$repo_root/pip_requirements-dev.txt"
	printf '%s\n' "$expected_receipt" > "$receipt_path"
	echo "Python environment is ready."
fi

"$venv_python" -c 'import yaml; print("PyYAML import verified.")'
