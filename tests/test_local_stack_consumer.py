"""Offline contracts for the closed disposable E2E adapter."""

import pathlib
import hashlib

import pytest

import local_stack_control.consumer
import local_stack_control.models


#============================================
def private_environment(tmp_path: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
	"""Create the one private Compose interpolation file used by a test."""
	path = tmp_path / "private.env"
	raw_capability = b"b" * 32
	path.write_text(
		"POSTGRES_PASSWORD=private\nPLE_DISPOSABLE_CAPABILITY_SHA256="
		+ hashlib.sha256(raw_capability).hexdigest()
		+ "\n",
		encoding="ascii",
	)
	path.chmod(0o600)
	capability = tmp_path / "cleanup.capability"
	capability.write_bytes(raw_capability)
	capability.chmod(0o600)
	return path, capability


#============================================
@pytest.mark.parametrize("resource_digest", (None, "foreign-capability-digest"))
def test_resource_capability_rejects_missing_or_foreign_digest(
	tmp_path: pathlib.Path,
	resource_digest: str | None,
) -> None:
	"""A manifest cannot remove a labelled resource without its exact digest."""
	env_file, capability_file = private_environment(tmp_path)
	compose_file = tmp_path / "containers" / "compose.yaml"
	compose_file.parent.mkdir()
	compose_file.write_text("services: {}\n", encoding="ascii")
	local_development_compose_file = compose_file.with_name("compose.local-development.yaml")
	local_development_compose_file.write_text("services: {}\n", encoding="ascii")
	target = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="ple-chapter-one-browser-0123456789ab",
		env_file=env_file,
		compose_files=(
			compose_file.resolve(strict=True),
			local_development_compose_file.resolve(strict=True),
		),
		provider=local_stack_control.models.ComposeProvider(
			("podman-compose", "--in-pod", "false"), "podman-compose"
		),
		with_smtp=False,
		env_setting_names=("POSTGRES_PASSWORD", "PLE_DISPOSABLE_CAPABILITY_SHA256"),
	)
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy="chapter-one-browser",
		capability_file=capability_file,
		project_prefix="ple-chapter-one-browser-",
		private_environment_file=env_file,
	)
	snapshot = local_stack_control.models.ProjectSnapshot(
		project=target.project,
		containers=(),
		volumes=(local_stack_control.models.VolumeResource("retained", target.project, resource_digest),),
		networks=(),
	)

	with pytest.raises(local_stack_control.models.ControllerError, match="do not all carry"):
		local_stack_control.consumer.require_capability_snapshot(disposable, snapshot)


#============================================
def test_adapter_compose_action_cannot_bypass_its_cleanup_preview() -> None:
	"""A runner cannot turn the generic action into an unpreviewed teardown."""

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.consumer.require_safe_compose_arguments(["down"])


#============================================
def test_full_stack_lifecycle_request_is_closed_and_has_a_bounded_timeout(
	tmp_path: pathlib.Path,
) -> None:
	"""Only approved disposable owners receive an explicit non-browser lifecycle request."""
	env_file, capability_file = private_environment(tmp_path)
	target = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="ple-chapter-one-browser-0123456789ab",
		env_file=env_file,
		compose_files=(),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman compose"),
		with_smtp=False,
		env_setting_names=(),
	)
	disposable = local_stack_control.models.DisposableComposeTarget(
		target, "chapter-one-browser", capability_file,
		"ple-chapter-one-browser-", env_file,
	)

	options = local_stack_control.consumer.lifecycle_options(disposable, 60)

	assert options.timeout_seconds == 60 and options.build and not options.open_browser
	with pytest.raises(local_stack_control.models.ControllerError, match="between 1 and 600"):
		local_stack_control.consumer.lifecycle_options(disposable, 601)
