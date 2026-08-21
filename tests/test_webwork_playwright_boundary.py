"""Regression contract for the private WebWork browser owner command."""

import pathlib

import file_utils


REPOSITORY_ROOT = pathlib.Path(file_utils.get_repo_root())
WEBWORK_E2E_PATH = REPOSITORY_ROOT / "tests" / "e2e" / "e2e_webwork_render_rpc.sh"


#============================================
def test_webwork_owner_runs_its_fixed_private_config_and_spec_directly() -> None:
	"""The WebWork lifecycle owns its browser child without reopening the canonical suite selector."""
	contents = WEBWORK_E2E_PATH.read_text(encoding="ascii")
	assert "npx playwright test --config playwright.config.ts tests/playwright/webwork_run.spec.ts" in contents
	assert "run_playwright_tests.sh" not in contents
