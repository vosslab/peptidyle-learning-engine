"""Offline behavior contracts for the Python Chapter One browser owner."""

import importlib.util
import pathlib

import pytest

import local_stack_control.models
import local_stack_control.process


#============================================
def browser_runner_module() -> object:
	"""Load the excluded E2E owner as an importable offline test subject."""
	path = pathlib.Path(__file__).parent / "e2e/e2e_chapter_one_browser.py"
	specification = importlib.util.spec_from_file_location("chapter_one_browser_e2e", path)
	if specification is None or specification.loader is None:
		raise RuntimeError("Chapter One browser E2E module is unavailable")
	module = importlib.util.module_from_spec(specification)
	specification.loader.exec_module(module)
	return module


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Record one runner-owned command without starting a real process."""

	#============================================
	def __init__(self, returncode: int) -> None:
		"""Start with the requested child result."""
		self.returncode = returncode
		self.calls: list[tuple[list[str], dict[str, str] | None, pathlib.Path | None]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Reject captured process use from this stream-only contract."""
		if stdin is not None:
			raise AssertionError("browser stream boundary does not accept stdin")
		raise RuntimeError("captured execution is not expected")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Record the action and return the configured result."""
		self.calls.append((argv, environment, cwd))
		return self.returncode


#============================================
def test_browser_live_environment_replaces_inherited_ple_values(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The visible Playwright journey receives only its owned live endpoint and credential file."""
	module = browser_runner_module()
	login_path = tmp_path / "local-login.txt"
	login_path.write_text("student=credential\n", encoding="ascii")
	monkeypatch.setenv("PLE_WEBWORK_LIVE_BASE_URL", "http://inherited.invalid")
	monkeypatch.setenv("AWS_SECRET_ACCESS_KEY", "unrelated-secret")
	environment = module.playwright_environment(53123, login_path)

	assert environment["PLE_WEBWORK_LIVE_BASE_URL"] == "http://127.0.0.1:53123"
	assert environment["PLE_WEBWORK_LIVE_STUDENT_CREDENTIAL_FILE"] == str(login_path)
	assert "AWS_SECRET_ACCESS_KEY" not in environment


#============================================
def test_browser_runner_keeps_a_child_failure_red(tmp_path: pathlib.Path) -> None:
	"""A failed child cannot be reported as a successful browser journey."""
	module = browser_runner_module()
	runner = RecordingRunner(1)

	with pytest.raises(local_stack_control.models.ControllerError):
		module.run(runner, ["npx", "playwright"], tmp_path)


#============================================
def test_browser_private_environment_forwards_selected_renderer_name(tmp_path: pathlib.Path) -> None:
	"""The disposable browser stack receives its renderer selection from configuration."""
	module = browser_runner_module()
	selections = {
		"PLE_GATEWAY_IMAGE_SHA256": "gateway",
		"PLE_POSTGRES_IMAGE_SHA256": "postgres",
		"PLE_MINIO_IMAGE_SHA256": "minio",
		"PLE_MINIO_MC_IMAGE_SHA256": "minio-mc",
		"PLE_SECRET_INIT_IMAGE_SHA256": "initializer",
		"PLE_WEBWORK_RENDERER_IMAGE": "localhost/reviewed-renderer:chosen",
		"PLE_WEBWORK_RENDERER_BASE_URL": "http://webwork-renderer:3000/",
		"PLE_WEBWORK_RENDERER_ID": "reviewed-renderer",
		"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS": "12",
		"PLE_WEBWORK_MAX_RESPONSE_BYTES": "4096",
	}
	module.write_private_target(tmp_path, 51001, 52001, 53001, 54001, selections)
	environment = (tmp_path / "env.local").read_text(encoding="ascii")

	assert "PLE_WEBWORK_RENDERER_IMAGE=localhost/reviewed-renderer:chosen\n" in environment
	assert "PLE_WEBWORK_RENDERER_BASE_URL=http://webwork-renderer:3000/\n" in environment
