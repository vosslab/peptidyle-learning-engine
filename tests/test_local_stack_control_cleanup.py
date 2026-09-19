"""Offline acceptance-lane and disposable cleanup contracts."""

import dataclasses
import hashlib
import pathlib

import pytest

import local_stack_control.cleanup
import local_stack_control.acceptance_lanes
import local_stack_control.compose
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process


#============================================
class ValidationLaneRunner(local_stack_control.process.CommandRunner):
	"""Capture aggregate lane handoffs without starting an external process."""

	def __init__(self, result_codes: tuple[int, ...]) -> None:
		"""Store the fixed child results used by one offline lane test."""
		self.result_codes = iter(result_codes)
		self.failed = False
		self.streamed: list[list[str]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Reject unexpected captured process calls from lane sequencing."""
		if stdin is not None:
			raise AssertionError("lane sequencing does not accept stdin")
		raise AssertionError(f"lane sequencing must stream child argv, not capture {argv}")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Return one child result and reject work after the first failure."""
		if self.failed:
			raise AssertionError(f"lane sequencing continued after failure with {argv}")
		self.streamed.append(argv)
		result = next(self.result_codes)
		self.failed = result != 0
		return result


#============================================
def target(
	repo_root: pathlib.Path,
	project: str = "containers",
	with_smtp: bool = False,
) -> local_stack_control.models.ComposeTarget:
	"""Build one explicit target for an offline controller decision."""
	env_file = repo_root / "env.local"
	env_file.write_text("STACK_SECRET=private\n", encoding="ascii")
	env_file.chmod(0o600)
	compose_file = repo_root / "compose.yaml"
	compose_file.write_text("services: {}\n", encoding="ascii")
	return local_stack_control.models.ComposeTarget(
		repo_root=repo_root,
		project=project,
		env_file=env_file,
		compose_files=(compose_file,),
		provider=local_stack_control.models.ComposeProvider(
			argv=("podman", "compose"),
			name="podman compose",
		),
		with_smtp=with_smtp,
		env_setting_names=("STACK_SECRET",),
	)


#============================================
def container(
	service: str,
	*,
	running: bool,
	health: str | None,
	state: str,
	exit_code: int | None,
) -> local_stack_control.models.ContainerResource:
	"""Build one labelled inspected container for readiness decisions."""
	return local_stack_control.models.ContainerResource(
		id=f"{service}-{state}-{exit_code}",
		names=(service,),
		project="containers",
		service=service,
		state=state,
		running=running,
		exit_code=exit_code,
		health=health,
		image="local-image",
		ports=(),
	)


#============================================
def test_acceptance_environment_discards_lifecycle_overrides() -> None:
	"""Aggregate acceptance owns lifecycle and child color configuration."""
	environment = local_stack_control.env_file.sanitized_acceptance_environment({
		"COMPOSE_PROJECT_NAME": "foreign",
		"FORCE_COLOR": "1",
		"NO_COLOR": "1",
		"PLE_E2E_PROJECT": "foreign",
		"PLE_WEBWORK_LIVE_PORT": "9999",
		"PLE_LAUNCH_TIMEOUT_SECONDS": "1",
		"SAFE_VALUE": "kept",
	})

	assert environment.get("SAFE_VALUE") == "kept"
	assert environment.get("FORCE_COLOR") == "1"
	assert "NO_COLOR" not in environment
	assert all(name not in environment for name in ("COMPOSE_PROJECT_NAME", "PLE_E2E_PROJECT"))


#============================================
def test_acceptance_preflight_blocks_retained_fixed_owner_containers() -> None:
	"""The aggregate cannot reuse or remove a prior fixed-owner container set."""
	snapshot = local_stack_control.models.ProjectSnapshot(
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		containers=(container(
			"gateway", running=False, health=None, state="exited", exit_code=0
		),),
		volumes=(),
		networks=(),
	)

	preflight = local_stack_control.cleanup.aggregate_acceptance_preflight((snapshot,))

	assert preflight.conflicting_projects == (
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
	)


#============================================
def test_acceptance_lanes_stop_after_the_first_nonzero_child(tmp_path: pathlib.Path) -> None:
	"""A failed lane keeps its result and prevents later live state from starting."""
	runner = ValidationLaneRunner((0, 17))
	result = local_stack_control.acceptance_lanes.run(runner, tmp_path, {})

	assert result == 17


#============================================
def disposable_target(
	tmp_path: pathlib.Path,
	provider_argv: tuple[str, ...] | None = None,
) -> local_stack_control.models.DisposableComposeTarget:
	"""Build a private target with an opaque runner-held capability."""
	selected_target = target(
		tmp_path,
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
	)
	compose_file = tmp_path / "containers" / "compose.yaml"
	live_demo_compose_file = tmp_path / "tests" / "e2e" / "compose.live-demo-browser.yaml"
	compose_file.parent.mkdir()
	live_demo_compose_file.parent.mkdir(parents=True)
	compose_file.write_text("services: {}\n", encoding="ascii")
	live_demo_compose_file.write_text("services: {}\n", encoding="ascii")
	selected_provider_argv = local_stack_control.models.podman_compose_argv()
	if provider_argv is not None:
		selected_provider_argv = provider_argv
	selected_provider_name = "podman-compose"
	if selected_provider_argv == ("podman", "compose"):
		selected_provider_name = "podman compose"
	selected_target = dataclasses.replace(
		selected_target,
		compose_files=(
			compose_file.resolve(strict=True),
			live_demo_compose_file.resolve(strict=True),
		),
		provider=local_stack_control.models.ComposeProvider(
			selected_provider_argv, selected_provider_name
		),
	)
	raw_capability = b"a" * 32
	capability_file = tmp_path / "cleanup.capability"
	capability_file.write_bytes(raw_capability)
	capability_file.chmod(0o600)
	selected_target.env_file.write_text(
		"STACK_SECRET=private\nPLE_DISPOSABLE_CAPABILITY_SHA256="
		+ hashlib.sha256(raw_capability).hexdigest()
		+ "\n",
		encoding="ascii",
	)
	return local_stack_control.compose.new_disposable_target(
		selected_target,
		capability_file,
		local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		local_stack_control.models.LiveDemoProfile.BROWSER,
	)


#============================================
@pytest.mark.parametrize(
	"provider_argv",
	(("podman", "compose"), ("podman-compose",)),
)
def test_compose_provider_retains_no_pod_lifecycle_boundary(
	tmp_path: pathlib.Path,
	provider_argv: tuple[str, ...],
) -> None:
	"""Every selected adapter receives the same explicit no-pod behavior."""
	disposable = disposable_target(tmp_path, provider_argv)

	assert disposable.target.provider.argv == (
		*provider_argv, "--in-pod", "false"
	)


#============================================
def test_default_compose_order_keeps_base_before_optional_smtp(tmp_path: pathlib.Path) -> None:
	"""The ordinary target uses one explicit Compose topology."""
	assert local_stack_control.compose.compose_files(tmp_path) == (
		tmp_path / local_stack_control.models.PRIMARY_COMPOSE_FILE,
	)


#============================================
@pytest.mark.parametrize(
	"owner",
	("chapter-one-pilot", "webwork-renderer-oracle", "replica-restart"),
)
def test_transitional_full_stack_owner_identities_are_rejected(owner: str) -> None:
	"""Retired service identities cannot recover disposable policy authority."""
	with pytest.raises(local_stack_control.models.ControllerError, match="supported owner policy"):
		local_stack_control.models.disposable_owner_policy(owner)


#============================================
@pytest.mark.parametrize(
	"project",
	(
		"ple_chapter_one_pilot_0123456789",
		"ple-webwork-renderer-oracle",
		"ple-replica-e2e-0123456789",
	),
)
def test_fixed_full_stack_policy_rejects_transitional_projects(project: str) -> None:
	"""The fixed owner cannot be redirected to either retired project grammar."""
	policy = local_stack_control.models.disposable_owner_policy(
		local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
	)
	assert policy.project_pattern.fullmatch(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	) is not None
	assert policy.project_pattern.fullmatch(project) is None


#============================================
def test_disposable_cleanup_rejects_project_mismatch(tmp_path: pathlib.Path) -> None:
	"""Cleanup cannot be redirected from the typed disposable project snapshot."""
	disposable = disposable_target(tmp_path)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.disposable_cleanup_plan(
			disposable,
			local_stack_control.models.ProjectSnapshot(
				"ple-live-demo-browser-other", (), (), ()
			),
		)


#============================================
def test_forged_disposable_owner_policy_cannot_form_cleanup_authority(
	tmp_path: pathlib.Path,
) -> None:
	"""A capability alone cannot turn an undeclared policy name into cleanup authority."""
	forged = dataclasses.replace(disposable_target(tmp_path), owner_policy="forged-owner")

	with pytest.raises(local_stack_control.models.ControllerError, match="supported owner policy"):
		local_stack_control.compose.require_disposable_ownership(forged)


#============================================
@pytest.mark.parametrize(
	"provider",
	(
		local_stack_control.models.ComposeProvider(
			("podman-compose",), "podman-compose"
		),
		local_stack_control.models.ComposeProvider(
			("podman-compose", "--in-pod", "false"), "forged-provider"
		),
	),
)
def test_forged_disposable_provider_cannot_form_cleanup_authority(
	tmp_path: pathlib.Path,
	provider: local_stack_control.models.ComposeProvider,
) -> None:
	"""A hand-built target cannot forge provider arguments or provenance."""
	disposable = disposable_target(tmp_path)
	forged_target = dataclasses.replace(
		disposable.target,
		provider=provider,
	)
	forged = dataclasses.replace(disposable, target=forged_target)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.compose.require_disposable_ownership(forged)


#============================================
def test_disposable_cleanup_rejects_resource_without_runner_capability(
	tmp_path: pathlib.Path,
) -> None:
	"""A project label alone cannot authorize removal of an extant resource."""
	disposable = disposable_target(tmp_path)
	snapshot = local_stack_control.models.ProjectSnapshot(
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		containers=(),
		volumes=(local_stack_control.models.VolumeResource(
			"owned-data", local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
		),),
		networks=(),
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.disposable_cleanup_plan(disposable, snapshot)


#============================================
def test_fixed_full_stack_owner_rejects_noncanonical_compose_files(
	tmp_path: pathlib.Path,
) -> None:
	"""The fixed full-stack capability cannot authorize arbitrary Compose files."""
	selected_target = target(
		tmp_path,
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
	)
	raw_capability = b"w" * 32
	capability_file = tmp_path / "cleanup.capability"
	capability_file.write_bytes(raw_capability)
	capability_file.chmod(0o600)
	selected_target.env_file.write_text(
		"STACK_SECRET=private\nPLE_DISPOSABLE_CAPABILITY_SHA256="
		+ hashlib.sha256(raw_capability).hexdigest()
		+ "\n",
		encoding="ascii",
	)
	expected_compose_file = tmp_path / "containers" / "compose.yaml"
	live_demo_compose_file = tmp_path / "tests" / "e2e" / "compose.live-demo-browser.yaml"
	expected_compose_file.parent.mkdir()
	live_demo_compose_file.parent.mkdir(parents=True)
	expected_compose_file.write_text("services: {}\n", encoding="ascii")
	live_demo_compose_file.write_text("services: {}\n", encoding="ascii")

	with pytest.raises(local_stack_control.models.ControllerError, match="Compose files"):
		local_stack_control.compose.new_disposable_target(
			selected_target,
			capability_file,
			local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
			local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC,
		)
