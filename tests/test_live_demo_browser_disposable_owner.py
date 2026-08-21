"""Offline ownership and TLS-target contracts for the connected live-demo lane."""

import pathlib

import pytest

import local_stack_control.compose
import local_stack_control.consumer
import local_stack_control.live_demo_gateway
import local_stack_control.lifecycle
import local_stack_control.models


#============================================
def live_demo_files(tmp_path: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
	"""Create the policy-declared topology without starting a Compose provider."""
	primary = tmp_path / "containers" / "compose.yaml"
	overlay = tmp_path / "tests" / "e2e" / "compose.live-demo-browser.yaml"
	primary.parent.mkdir()
	overlay.parent.mkdir(parents=True)
	primary.write_text("services: {}\n", encoding="ascii")
	overlay.write_text("services: {}\n", encoding="ascii")
	return primary.resolve(), overlay.resolve()


#============================================
def target(
	tmp_path: pathlib.Path,
	compose_files: tuple[pathlib.Path, ...],
) -> local_stack_control.models.ComposeTarget:
	"""Build a non-mutating target with the live-demo project grammar."""
	env_file = tmp_path / "env.local"
	env_file.write_text("PRIVATE_VALUE=private\n", encoding="ascii")
	result = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="ple-live-demo-browser-0123456789ab",
		env_file=env_file,
		compose_files=compose_files,
		provider=local_stack_control.models.ComposeProvider(("podman-compose",), "podman-compose"),
		with_smtp=False,
		env_setting_names=("PRIVATE_VALUE",),
	)
	return result


#============================================
def disposable(
	tmp_path: pathlib.Path,
) -> local_stack_control.models.DisposableComposeTarget:
	"""Build the typed live owner without container or capability mutation."""
	files = live_demo_files(tmp_path)
	selected = target(tmp_path, files)
	result = local_stack_control.models.DisposableComposeTarget(
		target=selected,
		owner_policy="live-demo-browser",
		capability_file=tmp_path / "capability",
		project_prefix="ple-live-demo-browser-",
		private_environment_file=selected.env_file,
	)
	return result


#============================================
def test_live_demo_browser_policy_requires_exact_primary_then_tls_overlay(
	tmp_path: pathlib.Path,
) -> None:
	"""The connected owner cannot launch production auth with another Compose shape."""
	primary, overlay = live_demo_files(tmp_path)
	policy = local_stack_control.models.disposable_owner_policy("live-demo-browser")
	assert policy.compose_relative_paths == (
		local_stack_control.models.PRIMARY_COMPOSE_FILE,
		"tests/e2e/compose.live-demo-browser.yaml",
	)
	expected = local_stack_control.compose.disposable_policy_compose_files(
		tmp_path, "live-demo-browser"
	)
	assert expected == (primary, overlay)
	assert local_stack_control.live_demo_gateway.is_tls_target(target(tmp_path, expected))
	for files in ((primary,), (overlay, primary)):
		with pytest.raises(local_stack_control.models.ControllerError, match="Compose files"):
			local_stack_control.compose.require_disposable_target_policy(
				target(tmp_path, files), "live-demo-browser"
			)


#============================================
def test_live_demo_browser_owns_only_tls_launch_and_worker_readiness_evidence(
	tmp_path: pathlib.Path,
) -> None:
	"""TLS behavior and bounded worker evidence follow the closed owner identity."""
	selected = disposable(tmp_path)
	options = local_stack_control.consumer.lifecycle_options(selected, 240)
	assert options.build and not options.open_browser
	assert local_stack_control.consumer.owned_project_images(selected) == (
		"localhost/ple-live-demo-browser-0123456789ab_gateway:latest",
	)
	argv, environment = local_stack_control.consumer.evidence_log_command(selected)
	assert argv[-1] == "worker"
	assert environment["COMPOSE_PROJECT_NAME"] == "ple-live-demo-browser-0123456789ab"
	with pytest.raises(local_stack_control.models.ControllerError, match="generic Compose"):
		local_stack_control.consumer.compose_command(selected, ["exec", "api", "env"])


#============================================
def test_live_demo_browser_builds_ordinary_browser_artifacts_and_uses_tls_loopback(
	tmp_path: pathlib.Path,
) -> None:
	"""The production-auth topology cannot accidentally compile the local login form."""
	selected = disposable(tmp_path)
	selected.target.env_file.write_text(
		"PRIVATE_VALUE=private\nPLE_GATEWAY_HOST_PORT=8443\n", encoding="ascii"
	)
	captured: list[dict[str, str] | None] = []

	class CaptureRunner:
		def run(
			self,
			argv: list[str],
			environment: dict[str, str] | None = None,
			cwd: pathlib.Path | None = None,
			stdin: str | None = None,
		) -> local_stack_control.models.CommandResult:
			captured.append(environment)
			return local_stack_control.models.CommandResult(tuple(argv), 0, "", "")

	options = local_stack_control.lifecycle.LifecycleOptions(
		60.0, True, False, False, local_development_browser_auth=False
	)
	local_stack_control.lifecycle.build_artifacts(CaptureRunner(), tmp_path, options)

	assert len(captured) == 1 and captured[0] is not None
	assert captured[0]["PLE_BROWSER_LOCAL_DEVELOPMENT_AUTH"] == "0"
	assert local_stack_control.live_demo_gateway.gateway_url(selected.target) == "https://localhost:8443/"
