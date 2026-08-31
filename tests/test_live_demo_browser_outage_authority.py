"""Offline authority contracts for the canonical browser gateway outage."""

import pathlib

import pytest

import local_stack_control.disposable_stack_command
import local_stack_control.disposable_stack_adapter
import local_stack_control.models
import local_stack_control.process


#============================================
def disposable(
	tmp_path: pathlib.Path,
	profile: local_stack_control.models.LiveDemoProfile = (
		local_stack_control.models.LiveDemoProfile.BROWSER
	),
) -> local_stack_control.models.DisposableComposeTarget:
	"""Build the fixed browser owner without consulting Podman."""
	env_file = tmp_path / "env.local"
	env_file.write_text("STACK_SECRET=private\n", encoding="ascii")
	primary = tmp_path / "containers" / "compose.yaml"
	overlay = tmp_path / "tests" / "e2e" / "compose.live-demo-browser.yaml"
	primary.parent.mkdir(exist_ok=True)
	overlay.parent.mkdir(parents=True, exist_ok=True)
	primary.write_text("services: {}\n", encoding="ascii")
	overlay.write_text("services: {}\n", encoding="ascii")
	target = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		env_file=env_file,
		compose_files=(primary, overlay),
		provider=local_stack_control.models.ComposeProvider(("podman-compose",), "podman-compose"),
		with_smtp=False,
		env_setting_names=("STACK_SECRET",),
	)
	result = local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		capability_file=tmp_path / "capability",
		project_prefix=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		private_environment_file=env_file,
		live_demo_profile=profile,
	)
	return result


#============================================
def container(
	identifier: str,
	service: str,
	running: bool,
	project: str | None = None,
	owner: str | None = local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
) -> local_stack_control.models.ContainerResource:
	"""Build one labelled container state for outage authority checks."""
	selected_project = project or local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	result = local_stack_control.models.ContainerResource(
		id=identifier,
		names=(service,),
		project=selected_project,
		service=service,
		state="running" if running else "exited",
		running=running,
		exit_code=None,
		health="healthy" if running else None,
		image="private-image",
		ports=(),
		owner=owner,
	)
	return result


#============================================
def snapshot(
	containers: tuple[local_stack_control.models.ContainerResource, ...],
	volumes: tuple[local_stack_control.models.VolumeResource, ...] | None = None,
	networks: tuple[local_stack_control.models.NetworkResource, ...] | None = None,
) -> local_stack_control.models.ProjectSnapshot:
	"""Build one complete labelled browser-project snapshot."""
	project = local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
	selected_volumes = volumes
	if selected_volumes is None:
		selected_volumes = (
			local_stack_control.models.VolumeResource(
				project + "_ple_pgdata",
				project,
				owner=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
			),
		)
	selected_networks = networks
	if selected_networks is None:
		selected_networks = (
			local_stack_control.models.NetworkResource(
				project + "_default",
				project,
				owner=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
			),
		)
	result = local_stack_control.models.ProjectSnapshot(
		project=project,
		containers=containers,
		volumes=selected_volumes,
		networks=selected_networks,
	)
	return result


#============================================
def test_fixed_owner_outage_authority_follows_only_its_closed_profile(
	tmp_path: pathlib.Path,
) -> None:
	"""The same fixed owner receives only its manifest-selected outage service."""
	browser = disposable(tmp_path)
	webwork = disposable(tmp_path, local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC)

	assert local_stack_control.disposable_stack_adapter.outage_service(browser) == "gateway"
	assert local_stack_control.disposable_stack_adapter.outage_service(webwork) == "webwork-renderer"


def test_gateway_outage_plan_is_closed_to_one_running_labelled_gateway(
	tmp_path: pathlib.Path,
) -> None:
	"""The stop command follows the owner policy and a single gateway selection."""
	selected = disposable(tmp_path)
	before = snapshot((container("gateway-id", "gateway", True), container("api-id", "api", True)))

	plan = local_stack_control.disposable_stack_adapter.declared_outage_stop_plan(selected, before)

	assert plan.service == "gateway"
	assert plan.argv[-2:] == ("stop", "gateway")


#============================================
@pytest.mark.parametrize(
	"before",
	(
		snapshot((container("gateway-id", "gateway", False),)),
		snapshot((container("gateway-one", "gateway", True), container("gateway-two", "gateway", True))),
		snapshot((container("gateway-id", "gateway", True, "foreign-project"),)),
	),
)
def test_gateway_outage_rejects_unavailable_ambiguous_or_foreign_selection(
	tmp_path: pathlib.Path,
	before: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Gateway selection rejects unavailable, duplicate, and foreign labelled resources."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.disposable_stack_adapter.declared_outage_stop_plan(disposable(tmp_path), before)


#============================================
def test_gateway_outage_postcondition_rejects_persistent_or_unrelated_change(
	tmp_path: pathlib.Path,
) -> None:
	"""Stopping the gateway cannot alter labelled persistence or another service."""
	selected = disposable(tmp_path)
	before = snapshot((container("gateway-id", "gateway", True), container("api-id", "api", True)))
	plan = local_stack_control.disposable_stack_adapter.declared_outage_stop_plan(selected, before)
	after_persistent_change = snapshot(
		(container("gateway-id", "gateway", False), container("api-id", "api", True)),
		(local_stack_control.models.VolumeResource("other-volume", before.project),),
	)
	after_unrelated_change = snapshot(
		(container("gateway-id", "gateway", False), container("api-id", "api", False)),
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.disposable_stack_adapter.require_declared_outage_stopped(
			selected, before, after_persistent_change, plan
		)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.disposable_stack_adapter.require_declared_outage_stopped(
			selected, before, after_unrelated_change, plan
		)


#============================================
def test_gateway_outage_postcondition_rejects_a_replaced_gateway(tmp_path: pathlib.Path) -> None:
	"""A stop proof remains bound to the exact gateway selected before mutation."""
	selected = disposable(tmp_path)
	before = snapshot((container("gateway-id", "gateway", True),))
	plan = local_stack_control.disposable_stack_adapter.declared_outage_stop_plan(selected, before)
	after = snapshot((container("replacement-id", "gateway", False),))

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.disposable_stack_adapter.require_declared_outage_stopped(selected, before, after, plan)


#============================================
@pytest.mark.parametrize(
	"after",
	(
		snapshot((container("gateway-id", "gateway", False), container("gateway-two", "gateway", False))),
		snapshot((container("api-id", "api", True),)),
		snapshot((container("gateway-id", "gateway", True),)),
	),
)
def test_gateway_outage_postcondition_rejects_duplicate_missing_or_restarted_gateway(
	tmp_path: pathlib.Path,
	after: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Post-stop proof requires one selected gateway to remain stopped."""
	selected = disposable(tmp_path)
	before = snapshot((container("gateway-id", "gateway", True),))
	plan = local_stack_control.disposable_stack_adapter.declared_outage_stop_plan(selected, before)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.disposable_stack_adapter.require_declared_outage_stopped(selected, before, after, plan)


#============================================
def test_gateway_outage_postcondition_rejects_a_forged_plan(tmp_path: pathlib.Path) -> None:
	"""Post-stop proof accepts only the closed gateway stop command it selected."""
	selected = disposable(tmp_path)
	before = snapshot((container("gateway-id", "gateway", True),))
	after = snapshot((container("gateway-id", "gateway", False),))
	forged = local_stack_control.models.ServiceStopPlan(
		before.project,
		"gateway",
		("podman-compose", "stop", "api"),
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.disposable_stack_adapter.require_declared_outage_stopped(selected, before, after, forged)


#============================================
def test_gateway_outage_postcondition_rejects_a_forged_preselection(tmp_path: pathlib.Path) -> None:
	"""A malformed caller cannot turn an unselected gateway into a stop receipt."""
	selected = disposable(tmp_path)
	before = snapshot((container("api-id", "api", True),))
	after = snapshot((container("gateway-id", "gateway", False), container("api-id", "api", True)))
	argv, _environment = local_stack_control.disposable_stack_adapter.outage_stop_command(selected)
	forged = local_stack_control.models.ServiceStopPlan(before.project, "gateway", tuple(argv))

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.disposable_stack_adapter.require_declared_outage_stopped(selected, before, after, forged)


#============================================
class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Record the one closed outage command without starting a subprocess."""

	def __init__(self) -> None:
		"""Start with no command records."""
		self.streamed: list[tuple[str, ...]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Reject process discovery because the test supplies typed snapshots."""
		raise AssertionError("outage authority receives its snapshot through the typed seam")

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Record the only allowed mutation as successful."""
		self.streamed.append(tuple(argv))
		return 0


#============================================
def test_gateway_outage_boundary_reinvents_and_proves_the_stopped_gateway(
	monkeypatch: pytest.MonkeyPatch,
	tmp_path: pathlib.Path,
) -> None:
	"""The typed boundary snapshots before and after its one closed stop mutation."""
	selected = disposable(tmp_path)
	before = snapshot((container("gateway-id", "gateway", True), container("api-id", "api", True)))
	after = snapshot((container("gateway-id", "gateway", False), container("api-id", "api", True)))
	values = iter((before, after))
	runner = RecordingRunner()
	monkeypatch.setattr(
		local_stack_control.disposable_stack_adapter,
		"require_current_resource_capability",
		lambda unused_runner, unused_disposable: next(values),
	)

	completed = local_stack_control.disposable_stack_adapter.stop_declared_outage_service(runner, selected)

	assert completed == local_stack_control.models.DeclaredOutageStop(before.project, "gateway")
	assert runner.streamed[0][-2:] == ("stop", "gateway")


#============================================
@pytest.mark.parametrize(
	"before",
	(
		snapshot((container("gateway-id", "gateway", True, owner=None),)),
		snapshot((container("gateway-id", "gateway", True, owner="foreign"),)),
		snapshot((container("gateway-id", "gateway", True), container("other-id", "unknown", True))),
		snapshot(
			(container("gateway-id", "gateway", True),),
			(local_stack_control.models.VolumeResource(
				"unexpected-volume",
				local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
				owner=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
			),),
		),
		snapshot(
			(container("gateway-id", "gateway", True),),
			networks=(local_stack_control.models.NetworkResource(
				"unexpected-network",
				local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
				owner=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
			),),
		),
	),
)
def test_gateway_outage_boundary_rejects_invalid_ownership_or_topology_before_mutation(
	monkeypatch: pytest.MonkeyPatch,
	tmp_path: pathlib.Path,
	before: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Missing/foreign labels and unknown topology fail before the closed stop streams."""
	selected = disposable(tmp_path)
	runner = RecordingRunner()
	monkeypatch.setattr(
		local_stack_control.disposable_stack_adapter,
		"require_current_resource_capability",
		lambda unused_runner, unused_disposable: before,
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.disposable_stack_adapter.stop_declared_outage_service(runner, selected)
	assert runner.streamed == []


#============================================
def test_outage_cli_derives_the_service_from_its_manifest_policy(tmp_path: pathlib.Path) -> None:
	"""The public action accepts its manifest only and exposes no service selector."""
	manifest = tmp_path / "manifest"
	args = local_stack_control.disposable_stack_command.parse_args([
		"stop-outage-service", "--manifest", str(manifest),
	])

	assert args.action == "stop-outage-service"
	assert not hasattr(args, "service")
