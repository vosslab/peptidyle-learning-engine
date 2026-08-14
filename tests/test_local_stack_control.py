"""Offline behavior tests for local Compose lifecycle decisions."""

import dataclasses
import hashlib
import pathlib

import pytest

import local_stack_control.cleanup
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
	) -> local_stack_control.models.CommandResult:
		"""Return only the selected project from value-scoped label queries."""
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
	) -> local_stack_control.models.CommandResult:
		"""Report a reachable but rootful active connection."""
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
def ready_snapshot(with_smtp: bool = False) -> local_stack_control.models.ProjectSnapshot:
	"""Build a snapshot whose declared topology is semantically ready."""
	containers: list[local_stack_control.models.ContainerResource] = []
	for service in local_stack_control.status.required_one_shots(with_smtp):
		containers.append(
			container(service, running=False, health=None, state="exited", exit_code=0)
		)
	for service in local_stack_control.models.BASE_LONG_RUNNING_SERVICES:
		health = None if service == "worker" else "healthy"
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
def test_public_container_identity_is_limited_to_a_short_prefix() -> None:
	"""Human diagnostics expose a compact Podman correlation prefix only."""
	container_value = container(
		"api", running=True, health="healthy", state="running", exit_code=0
	)
	container_value = dataclasses.replace(container_value, id="0123456789abcdef")

	serialized = local_stack_control.commands.asdict_for_json(container_value)

	assert serialized["id"] == "0123456789ab"


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
	"""Aggregate acceptance cannot inherit another stack's lifecycle controls."""
	environment = local_stack_control.env_file.sanitized_acceptance_environment({
		"COMPOSE_PROJECT_NAME": "foreign",
		"PLE_E2E_PROJECT": "foreign",
		"PLE_WEBWORK_LIVE_PORT": "9999",
		"PLE_LAUNCH_TIMEOUT_SECONDS": "1",
		"SAFE_VALUE": "kept",
	})

	assert environment.get("SAFE_VALUE") == "kept" and all(
		name not in environment for name in ("COMPOSE_PROJECT_NAME", "PLE_E2E_PROJECT")
	)


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
def disposable_target(tmp_path: pathlib.Path) -> local_stack_control.models.DisposableComposeTarget:
	"""Build a private target with an opaque runner-held capability."""
	selected_target = target(tmp_path, project="ple-ui-walkthrough-0123456789abcdef")
	compose_file = tmp_path / "containers" / "compose.yaml"
	compose_file.parent.mkdir()
	compose_file.write_text("services: {}\n", encoding="ascii")
	selected_target = dataclasses.replace(
		selected_target,
		compose_files=(compose_file.resolve(strict=True),),
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
	expected_compose_file.parent.mkdir()
	expected_compose_file.write_text("services: {}\n", encoding="ascii")

	with pytest.raises(local_stack_control.models.ControllerError, match="Compose files"):
		local_stack_control.compose.new_disposable_target(
			selected_target, capability_file, "ui-walkthrough"
		)
