"""Shared factories for local-stack lifecycle unit tests."""

import pathlib

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
def live_demo_target(
	tmp_path: pathlib.Path,
	profile: local_stack_control.models.LiveDemoProfile,
) -> local_stack_control.models.DisposableComposeTarget:
	"""Build one fixed live-demo target with closed profile metadata."""
	selected = lifecycle_target(
		tmp_path,
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		"live/env.local",
	)
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=selected,
		owner_policy=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		capability_file=tmp_path / "capability",
		project_prefix=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		private_environment_file=selected.env_file,
		live_demo_profile=profile,
	)
	return disposable
