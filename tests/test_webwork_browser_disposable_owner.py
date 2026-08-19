"""Offline ownership contracts for the isolated WebWork browser E2E."""

import pathlib

import pytest

import local_stack_control.consumer
import local_stack_control.models


#============================================
def disposable(tmp_path: pathlib.Path) -> local_stack_control.models.DisposableComposeTarget:
	"""Build the closed WebWork target without starting Podman."""
	env_file = tmp_path / "env.local"
	env_file.write_text("STACK_SECRET=private\n", encoding="ascii")
	compose_file = tmp_path / "containers" / "compose.yaml"
	compose_file.parent.mkdir()
	compose_file.write_text("services: {}\n", encoding="ascii")
	target = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="ple-webwork-browser-0123456789ab",
		env_file=env_file,
		compose_files=(compose_file,),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "test"),
		with_smtp=False,
		env_setting_names=("STACK_SECRET",),
	)
	return local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy="webwork-browser",
		capability_file=tmp_path / "capability",
		project_prefix="ple-webwork-browser-",
		private_environment_file=env_file,
	)


#============================================
def test_webwork_browser_owner_is_full_stack_private_and_exact(tmp_path: pathlib.Path) -> None:
	"""The WebWork lane owns one tokenized primary-compose project and gateway tag."""
	policy = local_stack_control.models.disposable_owner_policy("webwork-browser")

	assert policy.project_pattern.fullmatch("ple-webwork-browser-0123456789ab") is not None
	assert policy.compose_relative_paths == (local_stack_control.models.PRIMARY_COMPOSE_FILE,)
	assert policy.removes_gateway_image is True
	assert policy.outage_service == "webwork-renderer"
	assert policy.evidence_log_service == "api"
	assert policy.allows_generic_compose is False
	assert local_stack_control.consumer.CANONICAL_IMAGE_SELECTIONS_BY_OWNER[policy.owner] == ()
	assert local_stack_control.consumer.owned_project_images(disposable(tmp_path)) == (
		"localhost/ple-webwork-browser-0123456789ab_gateway:latest",
	)


#============================================
def test_webwork_browser_outage_and_lifecycle_are_closed_to_declared_renderer(tmp_path: pathlib.Path) -> None:
	"""Only the policy-owned renderer can be stopped or restarted during the outage oracle."""
	selected = disposable(tmp_path)

	assert local_stack_control.consumer.outage_service(selected) == "webwork-renderer"
	options = local_stack_control.consumer.lifecycle_options(selected, 240)
	assert options.timeout_seconds == 240.0
	assert options.build is True
	restart = local_stack_control.consumer.restart_options(selected, 240)
	assert restart.build is False
	argv, environment = local_stack_control.consumer.outage_stop_command(selected)
	assert argv[-2:] == ["stop", "webwork-renderer"]
	assert environment["COMPOSE_PROJECT_NAME"] == "ple-webwork-browser-0123456789ab"


#============================================
def test_webwork_browser_has_only_one_bounded_redacted_log_read(
	tmp_path: pathlib.Path,
) -> None:
	"""The owner cannot run generic Compose or select a different log service."""
	selected = disposable(tmp_path)

	with pytest.raises(local_stack_control.models.ControllerError, match="generic Compose"):
		local_stack_control.consumer.compose_command(selected, ["exec", "api", "env"])
	argv, environment = local_stack_control.consumer.evidence_log_command(selected)
	assert argv[-5:] == ["logs", "--no-color", "--tail", "5000", "api"]
	assert environment["COMPOSE_PROJECT_NAME"] == "ple-webwork-browser-0123456789ab"
	redacted = local_stack_control.consumer.redact_evidence_logs(
		"renderer private postgres://owner:private@example/log\n",
		("private",),
	)
	assert "private" not in redacted
	assert "[redacted]" in redacted
