"""Offline behavior tests for local Compose lifecycle decisions."""

import dataclasses
import hashlib
import pathlib

import pytest

import local_stack_control.cleanup
import local_stack_control.acceptance_lanes
import local_stack_control.compose
import local_stack_control.commands
import local_stack_control.discovery
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process
import local_stack_control.status


#============================================
class ProjectScopedInventoryRunner(local_stack_control.process.CommandRunner):
	"""Offline Podman inventory where a foreign container is no longer inspectable."""

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Return only the selected project from value-scoped label queries."""
		if stdin is not None:
			raise AssertionError("inventory discovery does not accept stdin")
		selected_label = next(
			(value for value in argv if value.endswith("=target-project")),
			None,
		)
		if argv[:3] == ["podman", "ps", "-a"]:
			if selected_label is None:
				raise AssertionError("single-project discovery must not list every Compose container")
			return self.result(argv, self.container_json())
		if argv[:3] == ["podman", "inspect", "target-container"]:
			return self.result(argv, '[{"Id":"target-container","State":{"Status":"running","Running":true,"ExitCode":0}}]')
		if argv[:3] == ["podman", "volume", "ls"]:
			return self.result(argv, self.volume_json() if selected_label is not None else "[]")
		if argv[:3] == ["podman", "network", "ls"]:
			return self.result(argv, self.network_json() if selected_label is not None else "[]")
		raise AssertionError(f"unexpected discovery command: {argv}")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Keep this pure discovery fixture from starting a subprocess."""
		raise AssertionError("discovery does not stream commands")

	#============================================
	@staticmethod
	def result(argv: list[str], stdout: str) -> local_stack_control.models.CommandResult:
		"""Return one successful offline command result."""
		return local_stack_control.models.CommandResult(tuple(argv), 0, stdout, "")

	#============================================
	@staticmethod
	def container_json() -> str:
		"""Represent the target container through either Compose label alias."""
		return '[{"Id":"target-container","Names":["target-api"],"Labels":{"io.podman.compose.project":"target-project","com.docker.compose.project":"target-project","io.podman.compose.service":"api"},"Image":"target-image","Ports":[]}]'

	#============================================
	@staticmethod
	def volume_json() -> str:
		"""Represent the target volume through the selected label query."""
		return '[{"Name":"target-data","Labels":{"io.podman.compose.project":"target-project","com.docker.compose.project":"target-project"}}]'

	#============================================
	@staticmethod
	def network_json() -> str:
		"""Represent the target network through the selected label query."""
		return '[{"name":"target-default","Labels":{"io.podman.compose.project":"target-project","com.docker.compose.project":"target-project"}}]'


#============================================
class NonRootlessInfoRunner(local_stack_control.process.CommandRunner):
	"""Provide typed engine metadata without allowing a real process call."""

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Report a reachable but rootful active connection."""
		if stdin is not None:
			raise AssertionError("engine metadata does not accept stdin")
		return local_stack_control.models.CommandResult(
			tuple(argv), 0, '{"host":{"security":{"rootless":false}}}', ""
		)

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Keep the engine-proof test away from subprocess execution."""
		raise RuntimeError("stream is not used by the rootless engine proof")


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
def test_start_authorization_allows_only_missing_exact_default_environment(
	tmp_path: pathlib.Path,
) -> None:
	"""First start reaches lifecycle bootstrap only for the declared default environment."""
	compose_file = tmp_path / "containers" / "compose.yaml"
	local_compose_file = tmp_path / "containers" / "compose.local-development.yaml"
	compose_file.parent.mkdir()
	compose_file.write_text("services: {}\n", encoding="ascii")
	local_compose_file.write_text("services: {}\n", encoding="ascii")
	default_env = tmp_path / "containers" / "env.local"
	selected = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="containers",
		env_file=default_env,
		compose_files=(compose_file, local_compose_file),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman compose"),
		with_smtp=False,
		env_setting_names=(),
	)

	local_stack_control.commands.authorize_start_target(selected)


#============================================
def test_start_authorization_refuses_missing_custom_environment(tmp_path: pathlib.Path) -> None:
	"""A missing custom environment cannot borrow default bootstrap authority."""
	compose_file = tmp_path / "containers" / "compose.yaml"
	compose_file.parent.mkdir()
	compose_file.write_text("services: {}\n", encoding="ascii")
	selected = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="containers",
		env_file=tmp_path / "private" / "custom.env",
		compose_files=(compose_file,),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman compose"),
		with_smtp=False,
		env_setting_names=(),
	)

	with pytest.raises(local_stack_control.models.ControllerError, match="custom mutating env"):
		local_stack_control.commands.authorize_start_target(selected)


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
def ready_snapshot(with_smtp: bool = False) -> local_stack_control.models.ProjectSnapshot:
	"""Build a snapshot whose declared topology is semantically ready."""
	containers: list[local_stack_control.models.ContainerResource] = []
	for service in local_stack_control.status.required_one_shots(with_smtp):
		containers.append(
			container(service, running=False, health=None, state="exited", exit_code=0)
		)
	for service in local_stack_control.status.required_long_running(with_smtp):
		health = None if service in ("worker", "invitation-delivery-worker") else "healthy"
		containers.append(
			container(service, running=True, health=health, state="running", exit_code=0)
		)
	return local_stack_control.models.ProjectSnapshot(
		project="containers",
		containers=tuple(containers),
		volumes=(),
		networks=(),
	)


#============================================
#============================================
def test_conflicting_compose_aliases_are_rejected() -> None:
	"""A resource cannot claim incompatible Podman and Compose project labels."""
	raw = {
		"Id": "container-id",
		"Names": ["untrusted-generated-name"],
		"Labels": {
			"io.podman.compose.project": "containers",
			"com.docker.compose.project": "other-project",
		},
		"Image": "local-image",
		"Ports": [],
	}
	inspection = {
		"State": {"Status": "running", "Running": True, "ExitCode": 0},
	}

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.discovery.container_from_json(raw, inspection)


#============================================
def test_discovery_normalizes_podman_bare_oci_image_id() -> None:
	"""A bare Podman configuration ID compares with canonical renderer provenance."""
	assert local_stack_control.discovery.canonical_oci_image_id("a" * 64) == "sha256:" + "a" * 64


#============================================
@pytest.mark.parametrize(
	"content",
	(
		"MISSING_SEPARATOR\n",
		"PLE_WEBWORK_RENDERER_IMAGE=localhost/pg-renderer:reviewed\nPLE_WEBWORK_RENDERER_IMAGE=other\n",
	),
)
def test_environment_parser_refuses_malformed_or_duplicate_declarations(
	tmp_path: pathlib.Path,
	content: str,
) -> None:
	"""A selected environment must have one valid declaration per setting."""
	env_file = tmp_path / "env.local"
	env_file.write_text(content, encoding="ascii")

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.env_file.env_settings(env_file)


#============================================
def test_project_snapshot_ignores_a_foreign_disappearing_container(tmp_path: pathlib.Path) -> None:
	"""Target discovery needs no inventory or inspection authority over another project."""
	snapshot = local_stack_control.discovery.discover_snapshot(
		ProjectScopedInventoryRunner(),
		tmp_path,
		"target-project",
	)

	assert snapshot == local_stack_control.models.ProjectSnapshot(
		project="target-project",
		containers=(local_stack_control.models.ContainerResource(
			id="target-container",
			names=("target-api",),
			project="target-project",
			service="api",
			state="running",
			running=True,
			exit_code=0,
			health=None,
			image="target-image",
			ports=(),
		),),
		volumes=(local_stack_control.models.VolumeResource("target-data", "target-project"),),
		networks=(local_stack_control.models.NetworkResource("target-default", "target-project"),),
	)


#============================================
def test_volume_only_project_remains_discoverable() -> None:
	"""A stopped project is still found from its labelled persistent volume."""
	volume = local_stack_control.models.VolumeResource(
		name="retained-data",
		project="retained-project",
	)

	projects = local_stack_control.discovery.all_projects((), (volume,), ())

	assert projects == ("retained-project",)


#============================================
def test_default_target_overrides_ambient_compose_project(tmp_path: pathlib.Path) -> None:
	"""A first start clears inherited lifecycle inputs before setting its project."""
	selected_target = target(tmp_path)

	environment = local_stack_control.compose.target_environment(
		selected_target,
		{
			"COMPOSE_PROJECT_NAME": "ambient-project",
			"PLE_E2E_PROJECT": "ambient-project",
			"PLE_WEBWORK_LIVE_PORT": "9999",
			"CONTAINER_HOST": "ssh://elsewhere",
			"PODMAN_CONNECTION": "other-machine",
			"DOCKER_CONTEXT": "remote-docker",
			"DOCKER_TLS_VERIFY": "1",
			"SAFE_VALUE": "kept",
			"STACK_SECRET": "ambient",
		},
	)

	assert environment == {"COMPOSE_PROJECT_NAME": "containers", "SAFE_VALUE": "kept"}


#============================================
def test_rootful_engine_metadata_cannot_authorize_a_local_stack_mutation(
	tmp_path: pathlib.Path,
) -> None:
	"""The active default engine must prove it is rootless before mutation."""
	with pytest.raises(local_stack_control.models.ControllerError, match="not rootless"):
		local_stack_control.process.require_rootless_local_engine(
			NonRootlessInfoRunner(), tmp_path
		)


#============================================
def test_explicit_read_only_project_has_its_own_semantic_status(tmp_path: pathlib.Path) -> None:
	"""Inspection can classify a named project without granting mutation authority."""
	selected_target = target(tmp_path, project="inspection-only")
	snapshot = local_stack_control.models.ProjectSnapshot(
		project="inspection-only",
		containers=(),
		volumes=(
			local_stack_control.models.VolumeResource(
				name="inspection-only-data",
				project="inspection-only",
			),
		),
		networks=(),
	)

	report = local_stack_control.status.build_report(
		selected_target.project,
		selected_target.with_smtp,
		snapshot,
	)

	assert report.state == "stopped-with-data"


#============================================
@pytest.mark.parametrize("mode", (0o644, 0o640))
def test_mutating_target_requires_a_private_env_file(
	tmp_path: pathlib.Path,
	mode: int,
) -> None:
	"""Cleanup planning refuses an environment file readable by another user."""
	selected_target = target(tmp_path)
	selected_target.env_file.chmod(mode)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.stop_plan(
			selected_target,
			local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
		)


#============================================
def test_mutating_target_rejects_a_symbolic_link_env_file(tmp_path: pathlib.Path) -> None:
	"""Cleanup planning rejects a symlink before it can form a mutation command."""
	selected_target = target(tmp_path)
	target_file = tmp_path / "target.env"
	target_file.write_text("STACK_SECRET=private\n", encoding="ascii")
	target_file.chmod(0o600)
	selected_target.env_file.unlink()
	selected_target.env_file.symlink_to(target_file)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.stop_plan(
			selected_target,
			local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
		)


#============================================
def test_foreign_project_cannot_form_a_reset_plan(tmp_path: pathlib.Path) -> None:
	"""A user-selected non-default project never receives reset authority."""
	foreign_target = target(tmp_path, project="another-project")

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.reset_plan(
			foreign_target,
			local_stack_control.models.ProjectSnapshot("another-project", (), (), ()),
			"another-project",
			False,
		)


#============================================
def test_duplicate_required_service_is_not_ready() -> None:
	"""Two labelled instances of one required service fail closed."""
	snapshot = ready_snapshot()
	duplicated = (*snapshot.containers, container(
		"gateway", running=True, health="healthy", state="running", exit_code=0
	))
	with_duplicate = local_stack_control.models.ProjectSnapshot(
		project="containers",
		containers=duplicated,
		volumes=(),
		networks=(),
	)

	report = local_stack_control.status.build_report("containers", False, with_duplicate)

	assert report.state == "failed"


#============================================
def test_smtp_topology_requires_its_setup_service() -> None:
	"""Selecting SMTP keeps the stack unready until its one-shot setup completes."""
	report = local_stack_control.status.build_report("containers", True, ready_snapshot(False))

	assert not report.ok


#============================================
def test_smtp_resources_infer_the_required_overlay() -> None:
	"""Status cannot call an SMTP-backed stack ready as a base stack."""
	report = local_stack_control.status.build_report("containers", False, ready_snapshot(True))

	assert report.ok and report.with_smtp


#============================================
def test_persisted_smtp_topology_requires_invitation_delivery_worker() -> None:
	"""An inferred SMTP topology cannot be ready without its delivery worker."""
	snapshot = ready_snapshot(True)
	without_delivery_worker = tuple(
		container_value
		for container_value in snapshot.containers
		if container_value.service != "invitation-delivery-worker"
	)
	report = local_stack_control.status.build_report(
		"containers",
		False,
		local_stack_control.models.ProjectSnapshot("containers", without_delivery_worker, (), ()),
	)

	assert report.with_smtp and not report.ok


#============================================
def test_smtp_delivery_worker_requires_one_running_instance() -> None:
	"""SMTP readiness rejects duplicate delivery workers even if each is running."""
	snapshot = ready_snapshot(True)
	delivery_worker = next(
		item for item in snapshot.containers if item.service == "invitation-delivery-worker"
	)
	with_duplicate = local_stack_control.models.ProjectSnapshot(
		"containers",
		(*snapshot.containers, dataclasses.replace(delivery_worker, id="delivery-worker-duplicate")),
		(),
		(),
	)
	report = local_stack_control.status.build_report("containers", True, with_duplicate)

	assert not report.ok and report.state == "failed"


#============================================
def test_invitation_delivery_restart_is_limited_to_smtp_topology() -> None:
	"""The optional delivery worker has no base-stack restart authority."""
	assert "invitation-delivery-worker" not in local_stack_control.models.restartable_services(False)
	assert "invitation-delivery-worker" in local_stack_control.models.restartable_services(True)


#============================================
def test_reset_requires_visible_default_project_acknowledgement(tmp_path: pathlib.Path) -> None:
	"""A reset plan cannot be created until the operator names its target project."""
	selected_target = target(tmp_path)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.reset_plan(
			selected_target,
			local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
			None,
			False,
		)


#============================================
def test_stop_refuses_an_empty_snapshot_before_compose_mutation(tmp_path: pathlib.Path) -> None:
	"""Stopping an absent project must not turn into a broad Compose action."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.stop_plan(
			target(tmp_path),
			local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
		)


#============================================
def test_reset_refuses_an_empty_snapshot_before_compose_mutation(tmp_path: pathlib.Path) -> None:
	"""Resetting an absent project must not manufacture a destructive action."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.reset_plan(
			target(tmp_path),
			local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
			"containers",
			False,
		)


#============================================
def test_stop_keeps_a_volume_only_project_actionable(tmp_path: pathlib.Path) -> None:
	"""Retained named data remains a valid normal-stack stop target."""
	snapshot = local_stack_control.models.ProjectSnapshot(
		"containers",
		(),
		(local_stack_control.models.VolumeResource("containers_ple_pgdata", "containers"),),
		(),
	)

	plan = local_stack_control.cleanup.stop_plan(target(tmp_path), snapshot)

	assert not plan.removes_volumes


#============================================
def test_confirmed_reset_removes_manifest_after_empty_cleanup_proof(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A confirmed reset clears its replay record after label discovery is empty."""
	selected_target = target(tmp_path)
	base_course_manifest = (
		selected_target.env_file.parent
		/ local_stack_control.models.DEFAULT_BASE_COURSE_MANIFEST_FILE
	)
	chapter_manifest = tmp_path / local_stack_control.models.DEFAULT_CHAPTER_ONE_MANIFEST_FILE
	base_course_manifest.parent.mkdir(parents=True)
	chapter_manifest.parent.mkdir(parents=True, exist_ok=True)
	for manifest_path in (base_course_manifest, chapter_manifest):
		manifest_path.write_text("{}", encoding="ascii")
		manifest_path.chmod(0o600)
	snapshot = local_stack_control.models.ProjectSnapshot(
		"containers",
		(),
		(local_stack_control.models.VolumeResource("containers_ple_pgdata", "containers"),),
		(),
	)
	plan = local_stack_control.cleanup.reset_plan(selected_target, snapshot, "containers", False)
	runner = ValidationLaneRunner((0,))
	empty_snapshot = local_stack_control.models.ProjectSnapshot("containers", (), (), ())
	monkeypatch.setattr(local_stack_control.process, "require_rootless_local_engine", lambda *_: None)
	monkeypatch.setattr(local_stack_control.discovery, "discover_snapshot", lambda *_: empty_snapshot)

	local_stack_control.commands.execute_cleanup(plan, selected_target, runner, False)

	assert not base_course_manifest.exists()
	assert not chapter_manifest.exists()


#============================================
def test_confirmed_reset_keeps_manifest_when_labelled_data_remains(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The replay record remains available when Compose cleanup leaves owned data."""
	selected_target = target(tmp_path)
	base_course_manifest = (
		selected_target.env_file.parent
		/ local_stack_control.models.DEFAULT_BASE_COURSE_MANIFEST_FILE
	)
	chapter_manifest = tmp_path / local_stack_control.models.DEFAULT_CHAPTER_ONE_MANIFEST_FILE
	base_course_manifest.parent.mkdir(parents=True)
	chapter_manifest.parent.mkdir(parents=True, exist_ok=True)
	for manifest_path in (base_course_manifest, chapter_manifest):
		manifest_path.write_text("{}", encoding="ascii")
		manifest_path.chmod(0o600)
	snapshot = local_stack_control.models.ProjectSnapshot(
		"containers",
		(),
		(local_stack_control.models.VolumeResource("containers_ple_pgdata", "containers"),),
		(),
	)
	plan = local_stack_control.cleanup.reset_plan(selected_target, snapshot, "containers", False)
	runner = ValidationLaneRunner((0,))
	monkeypatch.setattr(local_stack_control.process, "require_rootless_local_engine", lambda *_: None)
	monkeypatch.setattr(local_stack_control.discovery, "discover_snapshot", lambda *_: snapshot)

	with pytest.raises(local_stack_control.models.ControllerError, match="resources remain"):
		local_stack_control.commands.execute_cleanup(plan, selected_target, runner, False)

	assert base_course_manifest.exists()
	assert chapter_manifest.exists()


#============================================
def test_reset_preview_owns_live_demo_private_records(tmp_path: pathlib.Path) -> None:
	"""A reset preview names each local live-demo record before any mutation."""
	selected_target = target(tmp_path)
	snapshot = local_stack_control.models.ProjectSnapshot(
		"containers",
		(),
		(local_stack_control.models.VolumeResource("containers_ple_pgdata", "containers"),),
		(),
	)

	plan = local_stack_control.cleanup.reset_plan(selected_target, snapshot, "containers", True)

	assert plan.host_paths_to_remove == (
		selected_target.env_file.parent
		/ local_stack_control.models.DEFAULT_BASE_COURSE_MANIFEST_FILE,
		tmp_path / local_stack_control.models.DEFAULT_CHAPTER_ONE_MANIFEST_FILE,
	)


#============================================
def test_cleanup_allows_distinct_replicas_but_rejects_repeated_container_identity(
	tmp_path: pathlib.Path,
) -> None:
	"""Cleanup accepts named replicas and rejects a duplicated engine identity."""
	snapshot = ready_snapshot()
	api = next(item for item in snapshot.containers if item.service == "api")
	replica = dataclasses.replace(api, id="api-running-replica", names=("api-replica",))
	replicated = dataclasses.replace(snapshot, containers=(*snapshot.containers, replica))

	plan = local_stack_control.cleanup.stop_plan(target(tmp_path), replicated)

	assert plan.project == "containers"

	repeated = dataclasses.replace(replica, names=api.names)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.stop_plan(
			target(tmp_path), dataclasses.replace(replicated, containers=(*replicated.containers, repeated))
		)


#============================================
def test_cleanup_rejects_an_undeclared_labelled_network(tmp_path: pathlib.Path) -> None:
	"""A normal cleanup cannot reconcile a network outside selected topology."""
	snapshot = local_stack_control.models.ProjectSnapshot(
		"containers",
		(),
		(local_stack_control.models.VolumeResource("containers_ple_pgdata", "containers"),),
		(local_stack_control.models.NetworkResource("containers_unowned", "containers"),),
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.stop_plan(target(tmp_path), snapshot)


#============================================
def test_ambiguous_snapshot_cannot_form_a_cleanup_plan(tmp_path: pathlib.Path) -> None:
	"""A duplicate labelled service stops cleanup before a mutation argv exists."""
	snapshot = ready_snapshot()
	ambiguous = local_stack_control.models.ProjectSnapshot(
		project="containers",
		containers=(*snapshot.containers, container(
			"gateway", running=True, health="healthy", state="running", exit_code=0
		)),
		volumes=(),
		networks=(),
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.stop_plan(target(tmp_path), ambiguous)


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
def test_acceptance_preflight_blocks_retained_walkthrough_containers() -> None:
	"""The aggregate cannot reuse or remove a prior walkthrough container set."""
	snapshot = local_stack_control.models.ProjectSnapshot(
		project="ple-ui-walkthrough-owned",
		containers=(container(
			"gateway", running=False, health=None, state="exited", exit_code=0
		),),
		volumes=(),
		networks=(),
	)

	preflight = local_stack_control.cleanup.aggregate_acceptance_preflight((snapshot,))

	assert preflight.conflicting_projects == ("ple-ui-walkthrough-owned",)


#============================================
def test_acceptance_lanes_stop_after_the_first_nonzero_child(tmp_path: pathlib.Path) -> None:
	"""A failed lane keeps its result and prevents later live state from starting."""
	runner = ValidationLaneRunner((0, 0, 0, 17))
	result = local_stack_control.acceptance_lanes.run(runner, tmp_path, {})

	assert result == 17


#============================================
def test_acceptance_lanes_keep_live_demo_final_and_dispatch_in_order(tmp_path: pathlib.Path) -> None:
	"""The connected live-demo lane remains final in the fully successful aggregate."""
	declared_lanes = local_stack_control.acceptance_lanes.lanes("test-python")
	live_demo_lane = declared_lanes[-1]

	assert (live_demo_lane.name, live_demo_lane.argv) == (
		"connected ordinary-site live-demo browser journey",
		("test-python", "tests/e2e/e2e_live_demo_browser.py"),
	)
	runner = ValidationLaneRunner(tuple(0 for _ in declared_lanes))
	local_stack_control.acceptance_lanes.run(runner, tmp_path, {})

	assert runner.streamed == [list(lane.argv) for lane in local_stack_control.acceptance_lanes.lanes()]


#============================================
def disposable_target(tmp_path: pathlib.Path) -> local_stack_control.models.DisposableComposeTarget:
	"""Build a private target with an opaque runner-held capability."""
	selected_target = target(tmp_path, project="ple-ui-walkthrough-0123456789abcdef")
	compose_file = tmp_path / "containers" / "compose.yaml"
	local_compose_file = tmp_path / "containers" / "compose.local-development.yaml"
	compose_file.parent.mkdir()
	compose_file.write_text("services: {}\n", encoding="ascii")
	local_compose_file.write_text("services: {}\n", encoding="ascii")
	selected_target = dataclasses.replace(
		selected_target,
		compose_files=(
			compose_file.resolve(strict=True),
			local_compose_file.resolve(strict=True),
		),
		provider=local_stack_control.models.ComposeProvider(
			("podman-compose",), "podman-compose"
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
		"ui-walkthrough",
	)


#============================================
def test_default_compose_order_keeps_local_auth_before_optional_smtp(tmp_path: pathlib.Path) -> None:
	"""The ordinary target layers SMTP after the local-development behavior."""
	base, local = local_stack_control.compose.compose_files(tmp_path, False)
	with_smtp = local_stack_control.compose.compose_files(tmp_path, True)

	assert (base, local) == (
		tmp_path / local_stack_control.models.PRIMARY_COMPOSE_FILE,
		tmp_path / local_stack_control.models.LOCAL_DEVELOPMENT_COMPOSE_FILE,
	)
	assert with_smtp == (
		base,
		local,
		tmp_path / local_stack_control.models.SMTP_COMPOSE_FILE,
	)


#============================================
def test_replica_owner_keeps_its_override_after_local_development() -> None:
	"""The replica image target remains the final declared Compose override."""
	policy = local_stack_control.models.disposable_owner_policy("replica-restart")

	assert policy.compose_relative_paths == (
		local_stack_control.models.PRIMARY_COMPOSE_FILE,
		local_stack_control.models.LOCAL_DEVELOPMENT_COMPOSE_FILE,
		"tests/e2e/compose.replica-e2e.yaml",
	)


#============================================
def test_disposable_cleanup_rejects_project_mismatch(tmp_path: pathlib.Path) -> None:
	"""Cleanup cannot be redirected from the typed disposable project snapshot."""
	disposable = disposable_target(tmp_path)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.disposable_cleanup_plan(
			disposable,
			local_stack_control.models.ProjectSnapshot("ple-ui-walkthrough-fedcba9876543210", (), (), ()),
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
def test_forged_disposable_provider_cannot_form_cleanup_authority(
	tmp_path: pathlib.Path,
) -> None:
	"""A hand-built target cannot omit the mandatory no-pod provider arguments."""
	disposable = disposable_target(tmp_path)
	forged_target = dataclasses.replace(
		disposable.target,
		provider=local_stack_control.models.ComposeProvider(
			("podman-compose",), "podman-compose"
		),
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
		project="ple-ui-walkthrough-0123456789abcdef",
		containers=(),
		volumes=(local_stack_control.models.VolumeResource(
			"owned-data", "ple-ui-walkthrough-0123456789abcdef"
		),),
		networks=(),
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.cleanup.disposable_cleanup_plan(disposable, snapshot)


#============================================
def test_walkthrough_owner_rejects_a_target_with_noncanonical_compose_files(
	tmp_path: pathlib.Path,
) -> None:
	"""A walkthrough capability cannot authorize an arbitrary Compose definition."""
	selected_target = target(tmp_path, project="ple-ui-walkthrough-0123456789abcdef")
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
	local_compose_file = tmp_path / "containers" / "compose.local-development.yaml"
	expected_compose_file.parent.mkdir()
	expected_compose_file.write_text("services: {}\n", encoding="ascii")
	local_compose_file.write_text("services: {}\n", encoding="ascii")

	with pytest.raises(local_stack_control.models.ControllerError, match="Compose files"):
		local_stack_control.compose.new_disposable_target(
			selected_target, capability_file, "ui-walkthrough"
		)
