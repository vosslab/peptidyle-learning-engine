"""Offline behavioral tests for explicit UI walkthrough child inputs."""

import importlib
import json
import pathlib
import sys

import pytest


WALKTHROUGH_DIRECTORY = pathlib.Path(__file__).resolve().parent / "walkthrough"
sys.path.insert(0, str(WALKTHROUGH_DIRECTORY))
walkthrough = importlib.import_module("walklib.runner")


class RecordingCommands:
	"""Record runner-owned process boundaries without starting child processes."""

	def __init__(self) -> None:
		self.calls: list[tuple[list[str], dict[str, str] | None]] = []

	def __call__(self, command: list[str], environ: dict[str, str] | None) -> object:
		self.calls.append((command, environ))
		return walkthrough.CommandResult(0, "", "")


class ResourceCommands:
	"""Record read-only Podman ownership queries and optionally expose one stale resource."""

	def __init__(self, stale_resource: str | None = None) -> None:
		self.stale_resource = stale_resource
		self.calls: list[list[str]] = []

	def __call__(self, command: list[str], _environ: dict[str, str] | None) -> object:
		self.calls.append(command)
		resource_type = "containers"
		if command[1:3] == ["volume", "ls"]:
			resource_type = "volumes"
		elif command[1:3] == ["network", "ls"]:
			resource_type = "networks"
		stdout = "stale\n" if resource_type == self.stale_resource else ""
		return walkthrough.CommandResult(0, stdout, "")


class StackCommands:
	"""Expose one bounded stack label or host listener without using Podman."""

	def __init__(self, *, project: str = "", listener_port: int | None = None) -> None:
		self.project = project
		self.listener_port = listener_port

	def __call__(self, command: list[str], _environ: dict[str, str] | None) -> object:
		if command[0] == "lsof":
			stdout = "123\n" if f"-iTCP:{self.listener_port}" in command else ""
			return walkthrough.CommandResult(0 if stdout else 1, stdout, "")
		if command[-1].startswith("label=") and self.project == "containers":
			return walkthrough.CommandResult(0, "container-id\n", "")
		if command[-1].startswith("label=") and self.project.startswith("ple-ui-walkthrough-"):
			return walkthrough.CommandResult(0, "container-id\n", "")
		if command[:3] == ["podman", "container", "inspect"]:
			return walkthrough.CommandResult(0, f"{self.project}\n", "")
		return walkthrough.CommandResult(0, "", "")


def write_env_file(repository: pathlib.Path, relative_path: str, port: int) -> pathlib.Path:
	"""Write one selected Compose environment file for an offline test."""
	path = repository / relative_path
	path.parent.mkdir(parents=True, exist_ok=True)
	path.write_text(f"PLE_GATEWAY_HOST_PORT={port}\n", encoding="ascii")
	return path


def resolved_inputs(repository: pathlib.Path, *arguments: str) -> object:
	"""Resolve a fixed seed plus the small command-line variation under test."""
	return walkthrough.resolve_inputs(
		walkthrough.parse_args(["--master-seed", "42", *arguments]), repository
	)


#============================================
def chapter_one_manifest(display_ids: tuple[str, str, str, str]) -> str:
	"""Build one answer-free Chapter 1 manifest accepted by the arrangement child."""
	question_slugs = (
		"genetics-disorders-webwork-mc",
		"genetics-disorders-webwork-matching",
		"genetics-disorders-flat-mc",
		"genetics-disorders-flat-matching",
	)
	questions = [
		{
			"slug": slug,
			"displayId": display_id,
			"problemId": f"00000000-0000-4000-8000-00000000000{index}",
			"versionId": f"10000000-0000-4000-8000-00000000000{index}",
		}
		for index, (slug, display_id) in enumerate(zip(question_slugs, display_ids), start=1)
	]
	manifest = {
		"chapters": [
			{
				"slug": "genetics-chapter-1",
				"courseId": "20000000-0000-4000-8000-000000000001",
				"assignmentId": "30000000-0000-4000-8000-000000000001",
				"enrollmentId": "40000000-0000-4000-8000-000000000001",
				"questions": questions,
			}
		]
	}
	contents = json.dumps(manifest)
	return contents


def test_selected_env_file_wins_and_ple_values_are_not_forwarded(tmp_path: pathlib.Path) -> None:
	"""Child configuration comes from the selected explicit file, not ambient PLE values."""
	repository = tmp_path / "repository"
	write_env_file(repository, "containers/env.local", 3010)
	write_env_file(repository, "config/walkthrough.env", 3011)
	runner = walkthrough.WalkthroughRunner(
		resolved_inputs(repository, "--env-file", "config/walkthrough.env"),
		repository,
		{"PLE_GATEWAY_HOST_PORT": "3999", "PLE_SECRET": "not-forwarded"},
		RecordingCommands(),
	)

	assert walkthrough.effective_gateway_port(runner.inputs) == 3011
	assert not any(key.startswith("PLE_") for key in runner.sanitized_child_environment())


#============================================
def test_private_stack_environment_preserves_source_and_owns_image(tmp_path: pathlib.Path) -> None:
	"""The launcher gets a 0600 copied env file with a project-specific application image."""
	repository = tmp_path / "repository"
	source = write_env_file(repository, "containers/env.local", 3010)
	source.write_text("PLE_GATEWAY_HOST_PORT=3010\nPOSTGRES_PASSWORD=private\n", encoding="ascii")
	runner = walkthrough.WalkthroughRunner(resolved_inputs(repository), repository, {}, RecordingCommands())
	runner.prepare_journey_state()
	runner.create_private_stack_environment()
	private_env = runner.stack_env_file()
	private_contents = private_env.read_text(encoding="ascii")
	runner.remove_private_state()

	assert source.read_text(encoding="ascii") == "PLE_GATEWAY_HOST_PORT=3010\nPOSTGRES_PASSWORD=private\n"
	assert f"PLE_APPLICATION_IMAGE=localhost/peptidyle-learning-engine:{runner.compose_project_name}" in private_contents
	assert f"PLE_LOCAL_AUTH_HOST_FILE={private_env.parent}/local-identities.json" in private_contents
	assert f"PLE_INVITATION_TOKEN_SECRET_HOST_FILE={private_env.parent}/.secrets/invitation_token_secret" in private_contents
	assert private_env.parent.name.startswith("ple-ui-walkthrough-") and not private_env.exists()


#============================================
def test_private_stack_preflight_refuses_other_ple_stack_and_listener() -> None:
	"""Fixed-port acceptance will not overlap an existing PLE project or arbitrary listener."""
	with pytest.raises(walkthrough.RunnerError, match="default PLE stack"):
		walkthrough.assert_no_active_ple_stack(StackCommands(project="containers"))
	with pytest.raises(walkthrough.RunnerError, match="local port 9000"):
		walkthrough.assert_ports_available((5432, 9000), StackCommands(listener_port=9000))


#============================================
def test_generated_compose_project_is_only_passed_to_stack_children(
	tmp_path: pathlib.Path,
) -> None:
	"""The runner reserves one bounded project name for its launcher and cleanup children."""
	repository = tmp_path / "repository"
	write_env_file(repository, "containers/env.local", 3010)
	runner = walkthrough.WalkthroughRunner(
		resolved_inputs(repository), repository, {"COMPOSE_PROJECT_NAME": "outside"}, RecordingCommands()
	)
	stack_environment = runner.compose_child_environment()

	assert "COMPOSE_PROJECT_NAME" not in runner.sanitized_child_environment()
	assert stack_environment["COMPOSE_PROJECT_NAME"] == runner.compose_project_name


#============================================
def test_external_compose_project_is_rejected_before_stack_ownership(
	tmp_path: pathlib.Path,
) -> None:
	"""An inherited or selected Compose project cannot redirect disposable cleanup."""
	repository = tmp_path / "repository"
	write_env_file(repository, "containers/env.local", 3010)
	inputs = resolved_inputs(repository)

	with pytest.raises(walkthrough.RunnerError, match="must be unset"):
		walkthrough.reject_external_compose_project_name(inputs, {"COMPOSE_PROJECT_NAME": "outside"})


#============================================
def test_selected_env_file_cannot_override_disposable_project_ownership(
	tmp_path: pathlib.Path,
) -> None:
	"""The selected Compose env file cannot redirect a runner-owned cleanup boundary."""
	repository = tmp_path / "repository"
	env_file = write_env_file(repository, "containers/env.local", 3010)
	env_file.write_text(
		"PLE_GATEWAY_HOST_PORT=3010\nCOMPOSE_PROJECT_NAME=outside\n", encoding="ascii"
	)
	inputs = resolved_inputs(repository)

	with pytest.raises(walkthrough.RunnerError, match="must be unset"):
		walkthrough.reject_external_compose_project_name(inputs, {})


#============================================
def test_project_ownership_inspects_every_resource_type_and_compose_label() -> None:
	"""Preflight queries containers, volumes, and networks for both Compose label conventions."""
	commands = ResourceCommands()
	project_name = "ple-ui-walkthrough-0123456789abcdef"
	walkthrough.assert_no_stale_project_resources(project_name, commands)
	resource_types = {
		"containers" if command[1] == "ps" else f"{command[1]}s"
		for command in commands.calls
	}
	labels = {command[-1] for command in commands.calls}

	assert resource_types == {"containers", "volumes", "networks"}
	assert labels == {
		f"label=io.podman.compose.project={project_name}",
		f"label=com.docker.compose.project={project_name}",
	}


@pytest.mark.parametrize("stale_resource", ["containers", "volumes", "networks"])
def test_stale_compose_project_resource_fails_closed(stale_resource: str) -> None:
	"""Any stale resource prevents a runner from claiming disposable cleanup ownership."""
	with pytest.raises(walkthrough.RunnerError, match=f"stale {stale_resource}"):
		walkthrough.assert_no_stale_project_resources(
			"ple-ui-walkthrough-0123456789abcdef", ResourceCommands(stale_resource)
		)


#============================================
def test_lifecycle_scopes_stack_children_and_keep_cleanup(
	tmp_path: pathlib.Path,
	capsys: pytest.CaptureFixture[str],
) -> None:
	"""Launcher stages share ownership while browser children do not and keep retains volumes."""
	repository = tmp_path / "repository"
	write_env_file(repository, "containers/env.local", 3010)
	commands = RecordingCommands()
	runner = walkthrough.WalkthroughRunner(
		resolved_inputs(repository, "--keep"), repository, {"PLE_SECRET": "not-forwarded"}, commands
	)
	runner.compose_command = ["podman", "compose"]
	runner.prepare_report_directory()
	runner.prepare_journey_state()
	runner.create_private_stack_environment()
	runner.run_required(["launcher", "--check"], runner.compose_child_environment())
	runner.run_required(["launcher", "--no-open"], runner.compose_child_environment())
	runner.run_required(["node", "arrange"], runner.sanitized_child_environment())
	runner.run_required(["browser", "journey"], runner.sanitized_child_environment())
	runner.stack_launch_attempted = True
	private_env = runner.stack_env_file()
	runner.finish(True)
	output = capsys.readouterr().out
	report = runner.report_path.read_text(encoding="ascii")
	launcher_environments = [
		environment for command, environment in commands.calls if command[0] == "launcher"
	]
	child_environments = [
		environment for command, environment in commands.calls if command[0] != "launcher"
	]

	assert all(
		env is not None and env["COMPOSE_PROJECT_NAME"] == runner.compose_project_name
		for env in launcher_environments
	)
	assert all("COMPOSE_PROJECT_NAME" not in (env or {}) for env in child_environments) and not any(
		"down" in command for command, _environment in commands.calls
	)
	assert (
		runner.compose_project_name in output
		and "volumes are retained" in output
		and "Inspect without changing it: podman ps --all --filter" in output
		and f"label=io.podman.compose.project={runner.compose_project_name}" in output
		and "not-forwarded" not in output
		and "not-forwarded" not in report
		and private_env.exists()
	)
	runner.remove_private_state()


#============================================
def test_instructor_arrangement_uses_only_private_launcher_manifest(tmp_path: pathlib.Path) -> None:
	"""Canonical setup must not consult a stale manifest beside the selected source env."""
	repository = tmp_path / "repository"
	write_env_file(repository, "containers/env.local", 3010)
	stale_manifest = repository / "containers/local-chapter-one-pilot.json"
	stale_contents = chapter_one_manifest(("P-10-v1", "P-11-v1", "P-12-v1", "P-13-v1"))
	stale_manifest.write_text(stale_contents, encoding="ascii")
	stale_manifest.chmod(0o600)
	runner = walkthrough.WalkthroughRunner(
		resolved_inputs(repository), repository, {}, RecordingCommands()
	)
	runner.prepare_journey_state()
	runner.create_private_stack_environment()
	private_manifest = runner.stack_env_file().parent / "local-chapter-one-pilot.json"
	private_contents = chapter_one_manifest(("P-1-v1", "P-2-v1", "P-3-v1", "P-4-v1"))
	private_manifest.write_text(private_contents, encoding="ascii")
	private_manifest.chmod(0o600)
	runner.write_private_child_inputs(
		walkthrough.walklib.models.ArrangementChildInputs(private_manifest)
	)
	observed: dict[str, str] = {}

	def record_private_arrangement() -> None:
		child_inputs = runner.child_inputs_file
		if child_inputs is None:
			raise walkthrough.RunnerError("private arrangement inputs are unavailable")
		payload = json.loads(child_inputs.read_text(encoding="ascii"))
		manifest_path = pathlib.Path(payload["chapterOneManifestFile"])
		observed["path"] = str(manifest_path)
		observed["contents"] = manifest_path.read_text(encoding="ascii")
		runner.arrangements = [{"label": "launcher-chapter-one-genetics"}]
		runner.instructor_catalog_display_ids = ("P-1-v1", "P-2-v1", "P-3-v1", "P-4-v1")

	runner.arrange = record_private_arrangement
	runner.arrange_instructor_setup()
	runner.remove_private_state()

	assert (
		observed == {"path": str(private_manifest), "contents": private_contents}
		and observed["contents"] != stale_contents
	)
	assert not private_manifest.exists()


def test_private_child_handoff_redacts_credentials_and_is_removed(tmp_path: pathlib.Path) -> None:
	"""The explicit private handoff contains no credential material and has a bounded lifetime."""
	repository = tmp_path / "repository"
	write_env_file(repository, "containers/env.local", 3010)
	manifest = repository / "containers/local-chapter-one-pilot.json"
	manifest.write_text("{}", encoding="ascii")
	runner = walkthrough.WalkthroughRunner(
		resolved_inputs(repository), repository, {}, RecordingCommands()
	)
	runner.prepare_journey_state()
	runner.write_private_child_inputs(
		walkthrough.walklib.models.ArrangementChildInputs(manifest)
	)
	input_path = runner.child_inputs_file
	assert input_path is not None
	assert "chapterOneManifestFile" in input_path.read_text(encoding="ascii")
	runner.remove_private_state()
	assert not input_path.exists()


def test_playwright_uses_standard_config_and_no_hidden_ple_protocol(tmp_path: pathlib.Path) -> None:
	"""The browser child receives its state at the standard config boundary."""
	repository = tmp_path / "repository"
	write_env_file(repository, "containers/env.local", 3010)
	commands = RecordingCommands()
	runner = walkthrough.WalkthroughRunner(
		resolved_inputs(repository),
		repository,
		{"PLE_UI_WALKTHROUGH_MASTER_SEED": "999"},
		commands,
	)
	runner.prepare_journey_state()
	runner.run_playwright("tests/playwright/ui_walkthrough_keyboard_j1.spec.ts")
	command, environment = commands.calls[0]
	config_path = runner.playwright_config_file
	assert config_path is not None
	runner.remove_private_state()

	assert "--config" in command
	assert config_path.suffix == ".mts"
	assert not any(key.startswith("PLE_") for key in environment or {})


def test_unsafe_explicit_paths_are_rejected_before_child_actions(tmp_path: pathlib.Path) -> None:
	"""The runner rejects unsafe selected input paths during preflight."""
	repository = tmp_path / "repository"
	env_file = write_env_file(repository, "containers/env.local", 3010)
	env_file.unlink()
	env_file.symlink_to("outside.env")

	with pytest.raises(walkthrough.RunnerError, match="must not be a symlink"):
		resolved_inputs(repository)
