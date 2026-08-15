"""Offline contracts for the Chapter One browser disposable owner."""

import pathlib
import hashlib

import pytest

import local_stack_control.consumer
import local_stack_control.models
import local_stack_control.process


class ProviderRunner(local_stack_control.process.CommandRunner):
	"""Report the preferred provider without invoking Podman."""

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Provide an available provider response for target construction."""
		if stdin is not None:
			raise AssertionError("provider discovery does not accept stdin")
		result = local_stack_control.models.CommandResult(tuple(argv), 0, "", "")
		return result

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Keep the pure target tests away from subprocess execution."""
		raise RuntimeError("stream is not used by this test")


#============================================
def test_chapter_one_owner_rejects_an_invalid_project_shape(tmp_path: pathlib.Path) -> None:
	"""A project prefix alone cannot obtain browser lifecycle authority."""
	env_file = tmp_path / "env.local"
	raw_capability = b"c" * 32
	env_file.write_text(
		"POSTGRES_PASSWORD=private\nPLE_DISPOSABLE_CAPABILITY_SHA256="
		+ hashlib.sha256(raw_capability).hexdigest()
		+ "\n",
		encoding="ascii",
	)
	env_file.chmod(0o600)
	capability_file = tmp_path / "cleanup.capability"
	capability_file.write_bytes(raw_capability)
	capability_file.chmod(0o600)
	manifest = local_stack_control.consumer.DisposableManifest(
		owner="chapter-one-browser",
		project="ple-chapter-one-browser-not-a-token",
		env_file=env_file,
		capability_file=capability_file,
	)
	repo_root = pathlib.Path(__file__).resolve().parents[1]

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.consumer.disposable_target(ProviderRunner(), repo_root, manifest)


#============================================
def test_post_cleanup_requires_no_labelled_resources() -> None:
	"""Image cleanup is withheld if Compose leaves project ownership evidence."""
	snapshot = local_stack_control.models.ProjectSnapshot(
		project="ple-chapter-one-browser-0123456789ab",
		containers=(),
		volumes=(local_stack_control.models.VolumeResource("leftover", None),),
		networks=(),
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.consumer.require_empty_post_cleanup_snapshot(snapshot)


#============================================
def test_chapter_one_policy_removes_only_its_generated_gateway_tag(tmp_path: pathlib.Path) -> None:
	"""An empty verified target cannot select a shared image ID or default tag."""
	env_file = tmp_path / "env.local"
	env_file.write_text("POSTGRES_PASSWORD=private\n", encoding="ascii")
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=local_stack_control.models.ComposeTarget(
			repo_root=tmp_path,
			project="ple-chapter-one-browser-0123456789ab",
			env_file=env_file,
			compose_files=(),
			provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "test"),
			with_smtp=False,
			env_setting_names=(),
		),
		owner_policy="chapter-one-browser",
		capability_file=tmp_path / "capability",
		project_prefix="ple-chapter-one-browser-",
		private_environment_file=env_file,
	)

	assert local_stack_control.consumer.owned_project_images(disposable) == (
		"localhost/ple-chapter-one-browser-0123456789ab_gateway:latest",
	)
