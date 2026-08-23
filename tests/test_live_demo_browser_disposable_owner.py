"""Offline ownership and TLS-target contracts for the connected live-demo lane."""

import dataclasses
import hashlib
import pathlib

import pytest

import local_stack_control.cleanup
import local_stack_control.compose
import local_stack_control.consumer
import local_stack_control._consumer_cli
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
		project="ple-live-demo-browser",
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
		project_prefix="ple-live-demo-browser",
		private_environment_file=selected.env_file,
		live_demo_profile=local_stack_control.models.LiveDemoProfile.BROWSER,
	)
	return result


#============================================
def database_baseline_disposable(
	tmp_path: pathlib.Path,
) -> local_stack_control.models.DisposableComposeTarget:
	"""Build the fixed owner with only its PostgreSQL baseline profile."""
	compose_file = tmp_path / "tests" / "e2e" / "compose.database-baseline.yaml"
	compose_file.parent.mkdir(parents=True, exist_ok=True)
	compose_file.write_text("services: {}\n", encoding="ascii")
	selected = target(tmp_path, (compose_file.resolve(),))
	return local_stack_control.models.DisposableComposeTarget(
		target=selected,
		owner_policy=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		capability_file=tmp_path / "capability",
		project_prefix=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		private_environment_file=selected.env_file,
		live_demo_profile=local_stack_control.models.LiveDemoProfile.DATABASE_BASELINE,
	)


#============================================
def cleanup_disposable(
	tmp_path: pathlib.Path,
) -> local_stack_control.models.DisposableComposeTarget:
	"""Build one capability-backed fixed owner for cleanup authorization tests."""
	selected = target(tmp_path, live_demo_files(tmp_path))
	raw_capability = b"a" * 32
	capability_file = tmp_path / "capability"
	capability_file.write_bytes(raw_capability)
	capability_file.chmod(0o600)
	selected.env_file.write_text(
		"PRIVATE_VALUE=private\nPLE_DISPOSABLE_CAPABILITY_SHA256="
		+ hashlib.sha256(raw_capability).hexdigest()
		+ "\n",
		encoding="ascii",
	)
	selected.env_file.chmod(0o600)
	return local_stack_control.compose.new_disposable_target(
		selected,
		capability_file,
		local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		local_stack_control.models.LiveDemoProfile.BROWSER,
	)


#============================================
def cleanup_snapshot(
	selected: local_stack_control.models.DisposableComposeTarget,
	owner: str | None = local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
	service: str = "gateway",
) -> local_stack_control.models.ProjectSnapshot:
	"""Build one capability-bound fixed-owner cleanup inventory."""
	digest = local_stack_control.compose.disposable_capability_digest(
		selected.capability_file
	)
	container = local_stack_control.models.ContainerResource(
		id="owned-container",
		names=("owned-container",),
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		service=service,
		state="running",
		running=True,
		exit_code=None,
		health="healthy",
		image="owned-image",
		ports=(),
		capability_digest=digest,
		owner=owner,
	)
	return local_stack_control.models.ProjectSnapshot(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		(container,),
		(),
		(),
	)


def evidence_snapshot(
	selected: local_stack_control.models.DisposableComposeTarget,
	service: str,
) -> local_stack_control.models.ProjectSnapshot:
	"""Return one label-resolved running service fixture for evidence policy tests."""
	container = local_stack_control.models.ContainerResource(
		"a" * 64, ("owned",), selected.target.project, service, "running", True,
		None, None, "owned", (), owner=selected.owner_policy,
	)
	return local_stack_control.models.ProjectSnapshot(selected.target.project, (container,), (), ())


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
		tmp_path, "live-demo-browser", local_stack_control.models.LiveDemoProfile.BROWSER
	)
	assert expected == (primary, overlay)
	assert local_stack_control.live_demo_gateway.is_tls_target(target(tmp_path, expected))
	for files in ((primary,), (overlay, primary)):
		with pytest.raises(local_stack_control.models.ControllerError, match="Compose files"):
			local_stack_control.compose.require_disposable_target_policy(
				target(tmp_path, files),
				"live-demo-browser",
				local_stack_control.models.LiveDemoProfile.BROWSER,
			)


#============================================
@pytest.mark.parametrize(
	"project",
	("ple-live-demo-browser-0123456789ab", "ple-live-demo-browserx", "other"),
)
def test_live_demo_browser_policy_accepts_only_the_canonical_project(
	tmp_path: pathlib.Path,
	project: str,
) -> None:
	"""The one browser owner exposes no random-suffix compatibility project grammar."""
	policy = local_stack_control.models.disposable_owner_policy("live-demo-browser")
	assert policy.project_pattern.fullmatch("ple-live-demo-browser") is not None
	assert policy.project_pattern.fullmatch(project) is None


#============================================
def test_live_demo_browser_owns_only_typed_tls_launch_and_evidence_receipts(
	tmp_path: pathlib.Path,
) -> None:
	"""TLS behavior and bounded worker evidence follow the closed owner identity."""
	selected = disposable(tmp_path)
	options = local_stack_control.consumer.lifecycle_options(selected, 240)
	assert options.build and not options.open_browser
	assert local_stack_control.consumer.owned_project_images(selected) == (
		"localhost/ple-live-demo-browser_gateway:latest",
	)
	profile = local_stack_control.consumer.live_demo_profile_policy(selected)
	assert profile.evidence_log_services == (
		("worker_completion", "worker"), ("renderer_delivery", "api"),
	)
	argv, environment = local_stack_control.consumer.evidence_log_command(
		selected, "worker_completion", evidence_snapshot(selected, "worker")
	)
	assert argv[-1] == "a" * 64
	assert environment["COMPOSE_PROJECT_NAME"] == "ple-live-demo-browser"
	with pytest.raises(local_stack_control.models.ControllerError, match="generic Compose"):
		local_stack_control.consumer.compose_command(selected, ["exec", "api", "env"])


#============================================
def test_database_baseline_profile_allows_only_its_postgres_oracle_commands(
	tmp_path: pathlib.Path,
) -> None:
	"""The PostgreSQL-only profile cannot become an arbitrary fixed-stack shell."""
	selected = database_baseline_disposable(tmp_path)
	argv, environment = local_stack_control.consumer.compose_command(
		selected, ["up", "-d", "postgres"]
	)
	assert argv[-3:] == ["up", "-d", "postgres"]
	assert environment["COMPOSE_PROJECT_NAME"] == "ple-live-demo-browser"
	argv, _ = local_stack_control.consumer.compose_command(
		selected, ["exec", "-T", "postgres", "psql", "-d", "postgres", "-c", "SELECT 1"]
	)
	assert argv[-8:] == ["exec", "-T", "postgres", "psql", "-d", "postgres", "-c", "SELECT 1"]
	for arguments in (
		["up", "-d", "api"],
		["restart", "postgres"],
		["exec", "-T", "api", "psql", "-d", "postgres"],
		["exec", "-T", "postgres", "sh"],
	):
		with pytest.raises(local_stack_control.models.ControllerError, match="database baseline Compose"):
			local_stack_control.consumer.compose_command(selected, arguments)


#============================================
def test_fixed_cleanup_rejects_foreign_owner_label(tmp_path: pathlib.Path) -> None:
	"""A valid capability cannot authorize a foreign fixed-project owner label."""
	selected = cleanup_disposable(tmp_path)
	snapshot = cleanup_snapshot(selected, owner="foreign-owner")

	with pytest.raises(local_stack_control.models.ControllerError, match="foreign resource"):
		local_stack_control.cleanup.disposable_cleanup_plan(selected, snapshot)


#============================================
def test_fixed_cleanup_rejects_foreign_service_topology(tmp_path: pathlib.Path) -> None:
	"""A valid project and owner label cannot enlarge the selected service graph."""
	selected = cleanup_disposable(tmp_path)
	snapshot = cleanup_snapshot(selected, service="foreign-service")

	with pytest.raises(local_stack_control.models.ControllerError, match="foreign resource"):
		local_stack_control.cleanup.disposable_cleanup_plan(selected, snapshot)


#============================================
def test_fixed_cleanup_rejects_foreign_volume_topology(tmp_path: pathlib.Path) -> None:
	"""A capability-bound volume must have one exact declared project-prefixed name."""
	selected = cleanup_disposable(tmp_path)
	digest = local_stack_control.compose.disposable_capability_digest(
		selected.capability_file
	)
	volume = local_stack_control.models.VolumeResource(
		"ple-live-demo-browser_foreign",
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		capability_digest=digest,
		owner=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
	)
	snapshot = local_stack_control.models.ProjectSnapshot(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (volume,), ()
	)

	with pytest.raises(local_stack_control.models.ControllerError, match="foreign resource"):
		local_stack_control.cleanup.disposable_cleanup_plan(selected, snapshot)


#============================================
def test_fixed_cleanup_rejects_foreign_network_topology(tmp_path: pathlib.Path) -> None:
	"""A capability-bound network must have one exact declared project-prefixed name."""
	selected = cleanup_disposable(tmp_path)
	digest = local_stack_control.compose.disposable_capability_digest(
		selected.capability_file
	)
	network = local_stack_control.models.NetworkResource(
		"ple-live-demo-browser_foreign",
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		capability_digest=digest,
		owner=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
	)
	snapshot = local_stack_control.models.ProjectSnapshot(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT, (), (), (network,)
	)

	with pytest.raises(local_stack_control.models.ControllerError, match="foreign resource"):
		local_stack_control.cleanup.disposable_cleanup_plan(selected, snapshot)


#============================================
def test_fixed_cleanup_rejects_ambiguous_resource_identity(tmp_path: pathlib.Path) -> None:
	"""Repeated fixed-project engine identities fail before a cleanup command is formed."""
	selected = cleanup_disposable(tmp_path)
	first = cleanup_snapshot(selected).containers[0]
	second = dataclasses.replace(first, names=("other-name",))
	snapshot = local_stack_control.models.ProjectSnapshot(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		(first, second),
		(),
		(),
	)

	with pytest.raises(local_stack_control.models.ControllerError, match="ambiguous"):
		local_stack_control.cleanup.disposable_cleanup_plan(selected, snapshot)


#============================================
def test_fixed_cleanup_accepts_partial_owned_profile_topology(tmp_path: pathlib.Path) -> None:
	"""An interrupted launch may clean the valid subset already created by Compose."""
	selected = cleanup_disposable(tmp_path)
	snapshot = cleanup_snapshot(selected)
	plan = local_stack_control.cleanup.disposable_cleanup_plan(selected, snapshot)

	assert plan.snapshot == snapshot
	assert plan.argv[-3:] == ("down", "--volumes", "--remove-orphans")


#============================================
def test_live_demo_renderer_claim_resolves_api_and_rejects_a_worker_snapshot(
	tmp_path: pathlib.Path,
) -> None:
	"""A renderer receipt cannot silently read the worker's unrelated evidence stream."""
	selected = disposable(tmp_path)
	argv, _environment = local_stack_control.consumer.evidence_log_command(
		selected, "renderer_delivery", evidence_snapshot(selected, "api")
	)
	assert argv[:4] == ["podman", "logs", "--tail", "5000"]
	with pytest.raises(local_stack_control.models.ControllerError, match="exactly one"):
		local_stack_control.consumer.evidence_log_command(
			selected, "renderer_delivery", evidence_snapshot(selected, "worker")
		)
	with pytest.raises(local_stack_control.models.ControllerError, match="requested evidence"):
		local_stack_control.consumer.evidence_log_command(
			selected, "provider_source", evidence_snapshot(selected, "worker")
		)


#============================================
def test_evidence_log_cli_accepts_only_closed_receipt_claims(tmp_path: pathlib.Path) -> None:
	"""The adapter exposes typed claims rather than a generic service selector."""
	manifest = tmp_path / "disposable.manifest"
	args = local_stack_control._consumer_cli.parse_args([
		"read-evidence-logs", "--manifest", str(manifest), "--claim", "renderer_delivery",
	])
	assert args.claim == "renderer_delivery"
	with pytest.raises(SystemExit):
		local_stack_control._consumer_cli.parse_args([
			"read-evidence-logs", "--manifest", str(manifest), "--claim", "api",
		])


#============================================
def test_lifecycle_build_uses_the_one_production_browser_artifact_and_tls_loopback(
	tmp_path: pathlib.Path,
) -> None:
	"""Every lifecycle build uses the production browser build without an auth variant."""
	selected = disposable(tmp_path)
	selected.target.env_file.write_text(
		"PRIVATE_VALUE=private\nPLE_GATEWAY_HOST_PORT=8443\n", encoding="ascii"
	)
	captured: list[tuple[list[str], dict[str, str] | None]] = []

	class CaptureRunner:
		def run(
			self,
			argv: list[str],
			environment: dict[str, str] | None = None,
			cwd: pathlib.Path | None = None,
			stdin: str | None = None,
		) -> local_stack_control.models.CommandResult:
			captured.append((argv, environment))
			return local_stack_control.models.CommandResult(tuple(argv), 0, "", "")

	options = local_stack_control.lifecycle.LifecycleOptions(60.0, True, False, False)
	local_stack_control.lifecycle.build_artifacts(CaptureRunner(), tmp_path, options)

	assert len(captured) == 1
	assert captured[0][0] == ["./build.sh", "--debug"]
	assert local_stack_control.live_demo_gateway.gateway_url(selected.target) == "https://localhost:8443/"
