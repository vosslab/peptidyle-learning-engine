"""Aggregate Validation shell-front-door contract."""

import pathlib
import file_utils


REPOSITORY_ROOT = pathlib.Path(file_utils.get_repo_root())
ALL_TEST_PATH = REPOSITORY_ROOT / "all_test.sh"


#============================================
def executable_lines() -> tuple[str, ...]:
	"""Return the stable executable lines from the small aggregate shell contract."""
	return tuple(
		line.strip()
		for line in ALL_TEST_PATH.read_text(encoding="ascii").splitlines()
		if line.strip() and not line.lstrip().startswith("#") and not line.startswith("#!")
	)


#============================================
def test_all_test_shell_is_fail_fast() -> None:
	"""The aggregate shell boundary stops at a failed receipt."""
	assert "set -euo pipefail" in executable_lines()


#============================================
def test_all_test_runs_one_ordered_aggregate_acceptance_receipt() -> None:
	"""The aggregate makes one real-stack handoff after local build and check receipts."""
	lines = executable_lines()
	expected = (
		"set -euo pipefail",
		"source source_me.sh",
		"pytest tests/",
		"./build.sh",
		"./check_rust.sh",
		"./check_codebase.sh",
		"source source_me.sh && python3 local_stack.py acceptance",
		"git diff --check",
		"git diff --cached --check",
	)

	assert lines == expected
	assert sum("local_stack.py acceptance" in line for line in lines) == 1
	assert all("run_playwright_tests.sh" not in line for line in lines)
