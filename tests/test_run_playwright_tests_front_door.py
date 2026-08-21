"""Shell-front-door contracts for the canonical production browser suite."""

import os
import pathlib
import subprocess

import file_utils


REPOSITORY_ROOT = pathlib.Path(file_utils.get_repo_root())
RUNNER_PATH = REPOSITORY_ROOT / "run_playwright_tests.sh"


#============================================
def run_front_door(*arguments: str) -> subprocess.CompletedProcess[str]:
	"""Run a selection that completes before any stack allocation is possible."""
	environment = dict(os.environ)
	result = subprocess.run(
		["bash", str(RUNNER_PATH), *arguments],
		cwd=REPOSITORY_ROOT,
		env=environment,
		capture_output=True,
		text=True,
		check=False,
	)
	return result


#============================================
def test_shell_front_door_has_valid_syntax_and_owner_help() -> None:
	"""The shell wrapper delegates its concise public help to the typed owner."""
	syntax = subprocess.run(
		["bash", "-n", str(RUNNER_PATH)],
		cwd=REPOSITORY_ROOT,
		capture_output=True,
		text=True,
		check=False,
	)
	help_result = run_front_door("--help")
	assert syntax.returncode == 0 and help_result.returncode == 0
	assert "fresh disposable stack" in help_result.stdout and "--scenario" in help_result.stdout


#============================================
def test_shell_front_door_rejects_unknown_file_before_lifecycle_allocation() -> None:
	"""An unsupported browser target returns a selection error before Podman or Chromium work."""
	result = run_front_door("tests/playwright/smoke.spec.ts")
	combined = result.stdout + result.stderr
	assert result.returncode == 2 and "focused file" in combined
	assert "Browser-suite: starting" not in combined and "npx playwright" not in combined
