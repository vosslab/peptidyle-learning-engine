"""Offline safety contracts for the disposable replica adapter."""

import pathlib

import pytest

import local_stack_control.consumer
import local_stack_control.models


#============================================
def disposable_target() -> local_stack_control.models.DisposableComposeTarget:
	"""Build typed replica ownership without consulting the current checkout."""
	target = local_stack_control.models.ComposeTarget(
		repo_root=pathlib.Path("/repository"),
		project="ple-replica-e2e-0123456789",
		env_file=pathlib.Path("/private/compose.env"),
		compose_files=(pathlib.Path("/repository/compose.yaml"),),
		provider=local_stack_control.models.ComposeProvider(("compose",), "compose"),
		with_smtp=False,
		env_setting_names=(),
	)
	return local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy="replica-restart",
		capability_file=pathlib.Path("/private/cleanup.capability"),
		project_prefix="ple-replica-e2e-",
		private_environment_file=target.env_file,
	)


#============================================
def api_container(identifier: str) -> local_stack_control.models.ContainerResource:
	"""Build one running, label-resolved API replica."""
	return local_stack_control.models.ContainerResource(
		id=identifier,
		names=(),
		project="ple-replica-e2e-0123456789",
		service="api",
		state="running",
		running=True,
		exit_code=0,
		health="healthy",
		image="private-image",
		ports=(),
	)


#============================================
def test_replica_stop_refuses_a_single_api_instance() -> None:
	"""The outage cannot remove the only running API in its project."""
	snapshot = local_stack_control.models.ProjectSnapshot(
		project="ple-replica-e2e-0123456789",
		containers=(api_container("0123456789ab" + "0" * 52),),
		volumes=(),
		networks=(),
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.consumer.replica_stop_container(
			disposable_target(), snapshot, "api", "0123456789ab"
		)


#============================================
def test_replica_diagnostics_redact_private_values() -> None:
	"""Captured diagnostics cannot return an env secret or URL credential."""
	redacted = local_stack_control.consumer.redact_diagnostics(
		"private-value postgres://user:password@postgres/database",
		("private-value",),
	)

	assert "private-value" not in redacted and "user:password" not in redacted
