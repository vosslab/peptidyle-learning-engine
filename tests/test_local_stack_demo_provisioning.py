"""Local Stack Controller Live Demo provisioning contracts."""

import pathlib

import local_stack_control.lifecycle
import local_stack_control.models
import local_stack_control.process


class UnexpectedRunner(local_stack_control.process.CommandRunner):
	"""Reject every child process unless a test explicitly supplies an expectation."""

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		raise AssertionError(f"unexpected lifecycle command: {argv}")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		raise AssertionError(f"unexpected lifecycle stream: {argv}")


#============================================
def lifecycle_target(tmp_path: pathlib.Path, project: str, env_name: str) -> local_stack_control.models.ComposeTarget:
	"""Build one selected target without reading a tracked configuration file."""
	env_file = tmp_path / env_name
	return local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project=project,
		env_file=env_file,
		compose_files=(),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman compose"),
		with_smtp=False,
		env_setting_names=(),
	)


#============================================
def test_ready_live_demo_uses_one_canonical_migrator_command_after_readiness(
	tmp_path: pathlib.Path,
) -> None:
	"""The controller delegates cross-system activity to the Rust owner once."""
	selected = lifecycle_target(tmp_path, "ple-local", "workspace/env.local")
	selected.env_file.parent.mkdir()
	selected.env_file.write_text("PLE_GATEWAY_HOST_PORT=8181\n", encoding="ascii")
	selected.env_file.chmod(0o600)

	class Runner(UnexpectedRunner):
		def __init__(self) -> None:
			self.calls: list[list[str]] = []

		def run(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None, stdin: str | None = None) -> local_stack_control.models.CommandResult:
			self.calls.append(argv)
			return local_stack_control.models.CommandResult(tuple(argv), 0, "", "")

	runner = Runner()
	local_stack_control.lifecycle.provision_ready_live_demo(selected, runner)

	assert len(runner.calls) == 1
	assert runner.calls[0][-8:] == [
		"--profile", "migration", "run", "--rm", "--no-deps",
		"database-migrator", "installation-data", "provision",
	]
	assert "apply" not in runner.calls[0]


#============================================
def test_explicit_demo_opt_out_skips_all_demo_data_effects(
	tmp_path: pathlib.Path,
) -> None:
	"""The opt-out leaves canonical lifecycle work intact but starts no Demo phase."""
	selected = lifecycle_target(tmp_path, "ple-live-demo-browser", "workspace/env.local")
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=selected,
		owner_policy="live-demo-browser",
		capability_file=tmp_path / "capability",
		project_prefix="ple-live-demo-browser",
		private_environment_file=selected.env_file,
		live_demo_profile=local_stack_control.models.LiveDemoProfile.BROWSER,
	)
	options = local_stack_control.lifecycle.LifecycleOptions(
		1.0, False, False, False, without_live_demo=True
	)

	assert not local_stack_control.lifecycle.should_provision_live_demo(
		disposable, options, initial_database_install=True
	)


#============================================
def test_later_lifecycle_replay_never_reprovisions_ordinary_demo_data(
	tmp_path: pathlib.Path,
) -> None:
	"""Only the absence of the preexisting migration URL authorizes Demo installation."""
	selected = lifecycle_target(tmp_path, "ple-live-demo-browser", "workspace/env.local")
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=selected,
		owner_policy="live-demo-browser",
		capability_file=tmp_path / "capability",
		project_prefix="ple-live-demo-browser",
		private_environment_file=selected.env_file,
		live_demo_profile=local_stack_control.models.LiveDemoProfile.BROWSER,
	)
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)

	assert not local_stack_control.lifecycle.should_provision_live_demo(
		disposable, options, initial_database_install=False
	)


def test_only_the_browser_demo_keeps_its_closed_persona_selector(
	tmp_path: pathlib.Path,
) -> None:
	"""The ordinary browser Demo retains its personas; other paths omit them."""
	selected = lifecycle_target(tmp_path, "ple-live-demo-browser", "workspace/env.local")
	browser = local_stack_control.models.DisposableComposeTarget(
		target=selected,
		owner_policy=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		capability_file=tmp_path / "capability",
		project_prefix="ple-live-demo-browser",
		private_environment_file=selected.env_file,
		live_demo_profile=local_stack_control.models.LiveDemoProfile.BROWSER,
	)
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)

	assert local_stack_control.lifecycle.retains_live_demo_persona_configuration(
		browser, options
	)
	assert not local_stack_control.lifecycle.retains_live_demo_persona_configuration(
		browser,
		local_stack_control.lifecycle.LifecycleOptions(
			1.0, False, False, False, without_live_demo=True
		),
	)
	assert local_stack_control.lifecycle.retains_live_demo_persona_configuration(
		browser, options
	)
	non_browser = local_stack_control.models.DisposableComposeTarget(
		target=selected,
		owner_policy=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		capability_file=tmp_path / "capability-two",
		project_prefix="ple-live-demo-browser",
		private_environment_file=selected.env_file,
		live_demo_profile=local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC,
	)
	assert not local_stack_control.lifecycle.retains_live_demo_persona_configuration(
		non_browser, options
	)
