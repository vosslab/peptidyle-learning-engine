"""Permanent policy checks for the root local-stack launcher."""

# Standard Library
import os
import pathlib
import subprocess

# Local
import file_utils


REPO_ROOT = pathlib.Path(file_utils.get_repo_root())
LAUNCHER_PATH = REPO_ROOT / "launch_local_stack.sh"


#============================================
def test_launcher_has_a_safe_complete_default_workflow() -> None:
	"""The front door bootstraps, migrates, starts supported services, and opens one origin."""
	launcher = LAUNCHER_PATH.read_text()

	for requirement in (
		'set -euo pipefail',
		'./build.sh "$BUILD_PROFILE"',
		'compose up -d --build --force-recreate --no-deps "${services[@]}"',
		'cargo tools database migrate',
		'cargo tools e2e-seed',
		'PLE_LOCAL_GRADER_PASSWORD',
		'Local sign-in credentials:',
		'"${base_url}/health"',
		'open "${base_url}/"',
		'xdg-open "${base_url}/"',
		'compose config >/dev/null',
		'Compose configuration is incomplete:',
		'podman machine start',
		'PLE_POSTGRES_IMAGE_SHA256',
		'PLE_MINIO_IMAGE_SHA256',
		'PLE_MINIO_MC_IMAGE_SHA256',
		'compose --profile maintenance run --rm --no-deps postgres-major-guard',
	):
		assert requirement in launcher, f"launcher workflow is missing {requirement!r}"

	assert "down -v" not in launcher
	assert "rm -rf" not in launcher
	assert "containers/env.local" in launcher
	assert "mktemp" in launcher
	assert launcher.index('if [ "$CHECK_ONLY" -eq 1 ]') < launcher.index(
		"if ! podman info"
	), "--check must return before any attempt to start the Podman machine"


#============================================
def test_launcher_requires_immutable_native_service_image_digests() -> None:
	"""A custom env file cannot quietly restore mutable native infrastructure tags."""
	launcher = LAUNCHER_PATH.read_text()

	assert "require_sha256_env_value()" in launcher
	for setting in (
		"PLE_POSTGRES_IMAGE_SHA256",
		"PLE_MINIO_IMAGE_SHA256",
		"PLE_MINIO_MC_IMAGE_SHA256",
	):
		assert setting in launcher

	assert "^[0-9a-f]{64}$" in launcher
	assert "--check cannot validate this pre-image-pin env.local" in launcher
	assert "./launch_local_stack.sh --no-open" in launcher


#============================================
def test_launcher_is_executable_and_has_valid_bash_syntax() -> None:
	"""The root front door can be run directly and parsed by the macOS Bash baseline."""
	assert os.access(LAUNCHER_PATH, os.X_OK)
	subprocess.run(["bash", "-n", str(LAUNCHER_PATH)], cwd=REPO_ROOT, check=True)


#============================================
def test_launcher_help_is_available_without_podman_or_configuration() -> None:
	"""A newcomer can inspect the contract before installing or starting anything."""
	result = subprocess.run(
		["bash", str(LAUNCHER_PATH), "--help"],
		cwd=REPO_ROOT,
		check=True,
		capture_output=True,
		text=True,
	)

	assert "Build the repository" in result.stdout
	assert "--check" in result.stdout
	assert "--skip-build" in result.stdout
	assert "--with-webwork" in result.stdout
	assert result.stderr == ""


#============================================
def test_webwork_live_e2e_is_a_required_gateway_acceptance_not_a_readiness_probe() -> None:
	"""The live acceptance uses the all-in-one path and hands only safe inputs to Playwright."""
	e2e = (REPO_ROOT / "tests" / "e2e" / "e2e_webwork_render_rpc.sh").read_text()

	for requirement in (
		'./launch_local_stack.sh --with-webwork --no-open',
		'PLE_WEBWORK_LIVE_REQUIRED=1',
		'PLE_WEBWORK_LIVE_BASE_URL="$BASE_URL"',
		'PLE_WEBWORK_LIVE_STUDENT_CREDENTIAL_FILE="$CREDENTIAL_FILE"',
		'PLE_WEBWORK_LIVE_ASSIGNMENT_ID="$ASSIGNMENT_ID"',
		'npx playwright test tests/playwright/webwork_run.spec.ts',
		'[ -f tests/playwright/webwork_run.spec.ts ]',
		'first PLE question GET did not produce exactly one WebWork cache_hit event',
		'renderer outage did not fail WebWork run issuance closed with HTTP 503',
		'assert_summary_score "$summary_one" 1.0',
		'assert_summary_score "$summary_two" 0.0',
		'ple.webwork.cache',
		'renderer_call',
		'cache_hit',
	):
		assert requirement in e2e

	assert 'echo "SKIP:' not in e2e
	assert 'probe_render_rpc.sh --exercise' not in e2e
	assert "--data-urlencode" not in e2e
	assert "containers_webwork-renderer_1" not in e2e
	assert "docker.io/library/alpine:3.21" not in e2e
	assert 'cargo tools e2e-seed --webwork-pilot' in LAUNCHER_PATH.read_text()
