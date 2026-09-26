"""Local Stack Controller lifecycle core behavioral contracts."""

import os
import pathlib

import pytest

import local_stack_control.lifecycle
import local_stack_control.local_environment
import local_stack_control.lifecycle_profiles
import local_stack_control.models
import local_stack_control.process
import local_stack_control.renderer
import local_stack_control.env_file
import local_stack_control.status
import local_stack_control.service_singletons
import local_stack_lifecycle_helpers


UnexpectedRunner = local_stack_lifecycle_helpers.UnexpectedRunner
lifecycle_target = local_stack_lifecycle_helpers.lifecycle_target
live_demo_target = local_stack_lifecycle_helpers.live_demo_target


#============================================
class GatewayPortRunner(local_stack_control.process.CommandRunner):
	"""Return exact injected listener and default-gateway observations."""

	def __init__(self, listening: tuple[str, ...], gateway_running: bool) -> None:
		"""Store fixed non-network observations for one port selection decision."""
		self.listening = listening
		self.gateway_running = gateway_running

	#============================================
	def run(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None, stdin: str | None = None) -> local_stack_control.models.CommandResult:
		"""Answer only the expected lsof and Podman name requests."""
		if argv[:2] == ["lsof", "-nP"]:
			port = argv[2].split(":")[1]
			return local_stack_control.models.CommandResult(tuple(argv), 0 if port in self.listening else 1, "1" if port in self.listening else "", "")
		if argv[:3] == ["podman", "ps", "--format"]:
			stdout = "containers_gateway_1\n" if self.gateway_running else ""
			return local_stack_control.models.CommandResult(tuple(argv), 0, stdout, "")
		raise AssertionError(f"unexpected port decision command: {argv}")

	#============================================
	def stream(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None) -> int:
		"""Keep port decisions captured and deterministic."""
		raise AssertionError("gateway port selection does not stream commands")


#============================================
def readiness_container(
	service: str,
	identifier: int,
	*,
	healthy: bool = True,
) -> local_stack_control.models.ContainerResource:
	"""Build one deterministic required-service readiness observation."""
	one_shot = service in local_stack_control.models.BASE_ONE_SHOT_SERVICES
	running = healthy and not one_shot
	state = "exited" if one_shot else "running"
	health: str | None = "healthy"
	if one_shot:
		health = None
	if not healthy:
		state = "exited"
		health = None
	container = local_stack_control.models.ContainerResource(
		id=f"{service}-{identifier}",
		names=(),
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		service=service,
		state=state,
		running=running,
		exit_code=0 if healthy else 1,
		health=health,
		image="local/image",
		ports=(),
	)
	return container


#============================================
def replica_readiness_snapshot(
	api_instances: int,
	*,
	unhealthy_api: int | None = None,
	postgres_instances: int = 1,
) -> local_stack_control.models.ProjectSnapshot:
	"""Build the complete fixed replica topology without invoking Compose."""
	containers: list[local_stack_control.models.ContainerResource] = []
	for service in local_stack_control.models.BASE_ONE_SHOT_SERVICES:
		containers.append(readiness_container(service, 0))
	for service in local_stack_control.models.BASE_LONG_RUNNING_SERVICES:
		instances = 1
		if service == "api":
			instances = api_instances
		elif service == "postgres":
			instances = postgres_instances
		for identifier in range(instances):
			containers.append(
				readiness_container(
					service,
					identifier,
					healthy=service != "api" or identifier != unhealthy_api,
				)
			)
	snapshot = local_stack_control.models.ProjectSnapshot(
		local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		tuple(containers),
		(),
		(),
	)
	return snapshot


#============================================
@pytest.mark.parametrize(
	("build", "expected_first"),
	((True, "application-image-build"), (False, None)),
)
def test_teaching_stack_verifies_application_schema_before_application_start(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
	build: bool,
	expected_first: str | None,
) -> None:
	"""Every teaching stack verifies its generated app login before API processes run."""
	selected = lifecycle_target(tmp_path, "baseline", "baseline/env.local")
	target = local_stack_control.models.DisposableComposeTarget(
		target=selected,
		owner_policy="live-demo-baseline",
		capability_file=tmp_path / "capability",
		project_prefix="ple-live-demo-baseline",
		private_environment_file=selected.env_file,
	)
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, build, False, False)
	events: list[str] = []
	database_events: list[str] = []
	values = {"PLE_WEBWORK_RENDERER_IMAGE": "localhost/renderer:tag"}
	(tmp_path / "containers").mkdir()

	monkeypatch.setattr(local_stack_control.lifecycle, "require_lifecycle_inputs", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "require_disposable_ownership", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "bootstrap_default_state", lambda *args: None)
	monkeypatch.setattr(local_stack_control.env_file, "require_mutation_env_file", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "validate_static", lambda *args: values)
	monkeypatch.setattr(local_stack_control.lifecycle_validation, "require_mutation_engine", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "validate_compose", lambda *args: None)
	monkeypatch.setattr(local_stack_control.service_singletons, "require_for_target", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "child_environment", lambda *args: {})
	monkeypatch.setattr(local_stack_control.lifecycle, "build_artifacts", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "build_object_storage", lambda *args: None)
	monkeypatch.setattr(local_stack_control.renderer, "ensure_renderer_oci_id", lambda *args: "sha256:" + "a" * 64)
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_postgres", lambda *args: None)
	monkeypatch.setattr(
		local_stack_control.lifecycle,
		"synchronize_database",
		lambda *args: database_events.append("synchronize"),
	)
	monkeypatch.setattr(
		local_stack_control.lifecycle_migrations,
		"database_operation_for",
		lambda *args: database_events.append("operation") or "initialize",
	)
	monkeypatch.setattr(local_stack_control.lifecycle, "run_migrations", lambda *args: None)
	monkeypatch.setattr(local_stack_control.process_logins, "setup_service_logins", lambda *args: events.append("logins"))
	monkeypatch.setattr(local_stack_control.lifecycle, "verify_migrated_application_schema", lambda *args: events.append("verify"))
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_one_shot", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_renderer_ready", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "attest_renderer", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "run_api_initializers", lambda *args: None)
	monkeypatch.setattr(
		local_stack_control.lifecycle,
		"should_provision_installation_data",
		lambda *args: True,
	)
	monkeypatch.setattr(local_stack_control.lifecycle, "remove_live_demo_persona_configuration", lambda *args: None)
	monkeypatch.setattr(local_stack_control.lifecycle, "wait_for_complete_ready", lambda *args: "http://127.0.0.1:8080/")
	monkeypatch.setattr(local_stack_control.lifecycle, "provision_ready_installation_data", lambda *args, **kwargs: events.append("provision"))
	monkeypatch.setattr(local_stack_control.live_demo_gateway, "is_tls_target", lambda *args: False)

	def record_compose(
		_target: local_stack_control.models.ComposeTarget,
		_runner: local_stack_control.process.CommandRunner,
		arguments: list[str],
	) -> None:
		with pytest.raises(local_stack_control.models.ControllerError, match="active"):
			with local_stack_control.image_cleanup.image_build_lease(tmp_path):
				raise AssertionError("lifecycle released its image lease before attachment")
		if arguments == ["build", "api"]:
			events.append("application-image-build")
		elif arguments == ["build", "gateway"]:
			events.append("gateway-image-build")
		elif "api" in arguments:
			events.append("application-start")

	monkeypatch.setattr(local_stack_control.lifecycle, "compose_run", record_compose)

	local_stack_control.lifecycle.start_lifecycle(target, UnexpectedRunner(), tmp_path, options)

	expected = ["logins", "verify", "gateway-image-build", "application-start", "provision"]
	if expected_first is not None:
		expected.insert(0, expected_first)
	assert events == expected
	assert database_events[:2] == ["operation", "synchronize"]


#============================================
def test_default_bootstrap_preserves_existing_selected_value(tmp_path: pathlib.Path) -> None:
	"""Default bootstrap fills first-run settings without replacing a configured value."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	target.env_file.parent.mkdir()
	target.env_file.write_text("KEEP=value\n", encoding="ascii")
	target.env_file.chmod(0o600)
	local_stack_control.lifecycle.bootstrap_default_state(target)
	assert "KEEP=value\n" in target.env_file.read_text(encoding="ascii")


#============================================
@pytest.mark.parametrize("unsafe_mode", (0o644, 0o640))
def test_existing_default_environment_is_refused_before_default_generation(
	tmp_path: pathlib.Path,
	unsafe_mode: int,
) -> None:
	"""An unsafe preexisting default environment remains unchanged and starts no process."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	target.env_file.parent.mkdir()
	content = b"KEEP=unchanged\n"
	target.env_file.write_bytes(content)
	target.env_file.chmod(unsafe_mode)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.bootstrap_default_state(target, UnexpectedRunner())
	assert target.env_file.read_bytes() == content


#============================================
def test_existing_default_environment_with_foreign_owner_is_refused_before_generation(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Current-user ownership is required before reading an existing default environment."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	target.env_file.parent.mkdir()
	content = b"KEEP=unchanged\n"
	target.env_file.write_bytes(content)
	target.env_file.chmod(0o600)
	owner_id = os.getuid()
	monkeypatch.setattr(local_stack_control.env_file.os, "getuid", lambda: owner_id + 1)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.bootstrap_default_state(target, UnexpectedRunner())
	assert target.env_file.read_bytes() == content


#============================================
def test_failed_one_shot_refuses_before_polling_or_later_lifecycle_work(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A named initializer exit failure is terminal rather than a readiness timeout."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	failed = local_stack_control.models.StackServiceStatus(
		service="createbuckets", instances=1, present=True, running=False,
		healthy=False, complete=False, state="exited", health=None, exit_code=1,
	)
	report = local_stack_control.models.StatusReport(
		project="containers", with_smtp=False,
		snapshot=local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
		services=(failed,), ok=False, state="failed", message="a required service failed",
	)
	monkeypatch.setattr(local_stack_control.lifecycle, "status_report", lambda target, runner: report)
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.wait_for_one_shot(target, UnexpectedRunner(), options, "createbuckets")


#============================================
def test_completed_requested_one_shot_does_not_wait_for_later_initializers(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Storage setup advances after its own successful result, before API-only initialization."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	complete = local_stack_control.models.StackServiceStatus(
		service="createbuckets", instances=1, present=True, running=False,
		healthy=True, complete=True, state="exited", health=None, exit_code=0,
	)
	report = local_stack_control.models.StatusReport(
		project="containers", with_smtp=False,
		snapshot=local_stack_control.models.ProjectSnapshot("containers", (), (), ()),
		services=(complete,), ok=False, state="starting", message="later initializers are pending",
	)
	monkeypatch.setattr(local_stack_control.lifecycle, "status_report", lambda target, runner: report)
	options = local_stack_control.lifecycle.LifecycleOptions(1.0, False, False, False)
	local_stack_control.lifecycle.wait_for_one_shot(target, UnexpectedRunner(), options, "createbuckets")


#============================================
@pytest.mark.parametrize(
	("profile", "expected_api_instances"),
	(
		(local_stack_control.models.LiveDemoProfile.BROWSER, 1),
		(local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC, 1),
		(local_stack_control.models.LiveDemoProfile.REPLICA_RESTART, 2),
	),
)
def test_live_demo_profiles_own_their_expected_api_cardinality(
	tmp_path: pathlib.Path,
	profile: local_stack_control.models.LiveDemoProfile,
	expected_api_instances: int,
) -> None:
	"""Each closed profile determines readiness cardinality without a caller knob."""
	target = live_demo_target(tmp_path, profile)
	count = local_stack_control.lifecycle_profiles.expected_long_running_count(
		target, "api"
	)

	assert count == expected_api_instances


#============================================
def test_replica_profile_accepts_exactly_two_healthy_api_instances(
	tmp_path: pathlib.Path,
) -> None:
	"""The complete fixed replica topology is ready with two healthy APIs."""
	target = live_demo_target(
		tmp_path, local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	report = local_stack_control.status.build_target_report(
		target, replica_readiness_snapshot(2)
	)
	api = next(service for service in report.services if service.service == "api")

	assert report.ok and api.instances == 2 and api.healthy


#============================================
@pytest.mark.parametrize(
	("api_instances", "expected_state", "expected_service_state"),
	((1, "partially-active", "missing"), (3, "failed", "ambiguous")),
)
def test_replica_profile_rejects_api_cardinality_below_or_above_two(
	tmp_path: pathlib.Path,
	api_instances: int,
	expected_state: str,
	expected_service_state: str,
) -> None:
	"""Replica readiness distinguishes missing and unexpected extra API instances."""
	target = live_demo_target(
		tmp_path, local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	report = local_stack_control.status.build_target_report(
		target, replica_readiness_snapshot(api_instances)
	)
	api = next(service for service in report.services if service.service == "api")

	assert report.state == expected_state and api.state == expected_service_state


#============================================
def test_replica_profile_requires_every_api_instance_healthy(
	tmp_path: pathlib.Path,
) -> None:
	"""One unhealthy API keeps an exact two-instance observation unready."""
	target = live_demo_target(
		tmp_path, local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	report = local_stack_control.status.build_target_report(
		target, replica_readiness_snapshot(2, unhealthy_api=1)
	)
	api = next(service for service in report.services if service.service == "api")

	assert not report.ok and not api.healthy


#============================================
def test_replica_profile_still_requires_one_postgres_instance(
	tmp_path: pathlib.Path,
) -> None:
	"""The API exception does not weaken duplicate protection for PostgreSQL."""
	target = live_demo_target(
		tmp_path, local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	report = local_stack_control.status.build_target_report(
		target, replica_readiness_snapshot(2, postgres_instances=2)
	)
	postgres = next(
		service for service in report.services if service.service == "postgres"
	)

	assert report.state == "failed" and postgres.state == "ambiguous"


#============================================
def test_question_renderer_version_is_replaceable_private_attestation(tmp_path: pathlib.Path) -> None:
	"""A verified Question Renderer Version atomically replaces the prior version record."""
	first = local_stack_control.models.QuestionRendererVersion("localhost/renderer:one", "sha256:" + "a" * 64)
	second = local_stack_control.models.QuestionRendererVersion("localhost/renderer:two", "sha256:" + "b" * 64)
	local_stack_control.renderer.write_question_renderer_version(tmp_path, first)
	local_stack_control.renderer.write_question_renderer_version(tmp_path, second)
	assert local_stack_control.renderer.load_question_renderer_version(tmp_path) == second


#============================================
def test_closed_teaching_owner_is_a_teaching_profile_but_custom_target_is_not(
	tmp_path: pathlib.Path,
) -> None:
	"""Only the declared disposable teaching owner receives bootstrap authority."""
	target = lifecycle_target(tmp_path, "walk", "walk.env")
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=target, owner_policy="live-demo-baseline", capability_file=tmp_path / "capability",
		project_prefix="ple_live_demo_baseline_", private_environment_file=target.env_file,
	)
	assert local_stack_control.lifecycle_profiles.uses_local_teaching_state(disposable)
	assert not local_stack_control.lifecycle_profiles.uses_local_teaching_state(target)


#============================================
def test_live_teaching_bootstrap_keeps_seed_inputs_without_local_auth_files(
	tmp_path: pathlib.Path,
) -> None:
	"""The TLS owner creates seed inputs without introducing local-file credentials."""
	target = lifecycle_target(tmp_path, "ple-live-demo-browser", "live/env.local")
	target.env_file.parent.mkdir()
	target.env_file.write_text("\n", encoding="ascii")
	target.env_file.chmod(0o600)
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy="live-demo-browser",
		capability_file=tmp_path / "capability",
		project_prefix="ple-live-demo-browser",
		private_environment_file=target.env_file,
		live_demo_profile=local_stack_control.models.LiveDemoProfile.BROWSER,
	)

	local_stack_control.lifecycle.bootstrap_default_state(disposable, GatewayPortRunner((), False))
	values = local_stack_control.env_file.env_settings(target.env_file)
	secret_directory = target.env_file.parent / ".secrets"
	invitation_path = secret_directory / "invitation_token_secret"

	assert "PLE_LOCAL_AUTH_HOST_FILE" not in values
	assert not (target.env_file.parent / "local-login.txt").exists()
	assert not (target.env_file.parent / "local-identities.json").exists()
	assert invitation_path.is_file()


#============================================
#============================================
def test_minio_bootstrap_replaces_a_retained_publisher_policy_before_attachment() -> None:
	"""The generic publisher receives only Question-object access after policy replacement."""
	compose = pathlib.Path("containers/compose.yaml").read_text(encoding="utf-8")
	remove = "mc admin policy remove local ple-public-asset-publisher"
	create = "mc admin policy create local ple-public-asset-publisher \"$$policy_file\""
	attach = "mc admin policy attach local ple-public-asset-publisher --user"
	assert "if ! mc admin policy info local ple-public-asset-publisher" not in compose
	assert compose.index(remove) < compose.index(create) < compose.index(attach)
	assert 'arn:aws:s3:::private-content/questions/*' in compose
	assert 'arn:aws:s3:::public-assets/questions/*' in compose
	assert "PLE_PUBLISHER_RESTRICTED_ASSET_PATH" not in compose
	assert "PLE_PUBLISHER_PUBLIC_ASSET_PATH" not in compose


#============================================
def test_database_migrator_projects_only_the_existing_installation_data_capabilities() -> None:
	"""The one-shot installer receives ordinary publication inputs without an API image change."""
	compose = pathlib.Path("containers/compose.yaml").read_text(encoding="utf-8")
	migrator = compose[compose.index("  database-migrator:"):compose.index("  postgres-major-guard:")]
	for value in (
		"PLE_MIGRATION_DATABASE_URL:",
		"DATABASE_URL: ${PLE_API_DATABASE_URL:-postgres://ple_api_login:service-login-setup-required",
		"PLE_STORAGE_TOPOLOGY:",
		"PLE_BROWSER_ORIGIN: ${PLE_BROWSER_ORIGIN:-}",
		"PLE_S3_ENDPOINT: http://minio:9000",
		"PLE_PUBLIC_ASSETS_BUCKET: public-assets",
		"PLE_PRIVATE_CONTENT_BUCKET: private-content",
		"PLE_STUDENT_RECORDS_BUCKET: student-records",
		"PLE_TEMP_PROCESSING_BUCKET: temp-processing",
		"AWS_ACCESS_KEY_ID: ${MINIO_ROOT_USER}",
		"AWS_SECRET_ACCESS_KEY: ${MINIO_ROOT_PASSWORD}",
		"source: ple_identity_runtime",
		"target: /run/ple-secrets",
		"read_only: true",
	):
		assert value in migrator
	assert "PLE_INVITATION_TOKEN_SECRET_FILE" not in migrator
	assert "PLE_PUBLISHER_DATABASE_URL" not in migrator


#============================================
def test_busy_default_port_selects_first_free_teaching_port_or_keeps_running_gateway(tmp_path: pathlib.Path) -> None:
	"""First startup avoids an unrelated 8080 listener while retaining its own active gateway."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	values = {"PLE_GATEWAY_HOST_PORT": "8080"}
	available = GatewayPortRunner(("8080",), False)
	running = GatewayPortRunner(("8080",), True)
	assert local_stack_control.lifecycle.choose_default_gateway_port(target, values, available) == "8000"
	assert local_stack_control.lifecycle.choose_default_gateway_port(target, values, running) == "8080"
	custom = lifecycle_target(
		tmp_path,
		"custom-project",
		"walk/env.local",
	)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle.choose_default_gateway_port(custom, values, available)


#============================================
def test_default_environment_symlink_is_not_a_bootstrap_target(tmp_path: pathlib.Path) -> None:
	"""Lexical default-path matching rejects a symlink before a private bootstrap can follow it."""
	default_path = tmp_path / "containers/env.local"
	default_path.parent.mkdir()
	other = tmp_path / "other.env"
	other.write_text("SAFE=value\n", encoding="ascii")
	default_path.symlink_to(other)
	assert not local_stack_control.local_environment.is_default_local_environment(tmp_path, default_path)


#============================================
def test_compose_failures_retain_redacted_bounded_child_diagnostics(tmp_path: pathlib.Path) -> None:
	"""Both Compose boundaries retain only bounded diagnostic detail after redaction."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	target.env_file.parent.mkdir()
	target.env_file.write_text("PLE_TEST_SECRET=private-value\n", encoding="ascii")
	target.env_file.chmod(0o600)
	test_footer = "test result: FAILED. rerun with cargo test --test grading\n" * 12
	result = local_stack_control.models.CommandResult(
		("podman", "compose"),
		1,
		"thread 'grading' panicked: Rust failure section private-value\n" + test_footer,
		"SQL ERROR private-value\n" + test_footer,
	)
	detail = local_stack_control.lifecycle_diagnostics.redacted_failure_detail(
		result, ("private-value",)
	)
	assert "Rust failure section [private]" in detail
	assert "SQL ERROR [private]" in detail
	assert detail.index("SQL ERROR [private]") < detail.index("Rust failure section [private]")
	class FailureRunner(UnexpectedRunner):
		def run(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None, stdin: str | None = None) -> local_stack_control.models.CommandResult:
			return result
	with pytest.raises(local_stack_control.models.ControllerError) as compose_error:
		local_stack_control.lifecycle.compose_run(target, FailureRunner(), ["up", "-d"])
	with pytest.raises(local_stack_control.models.ControllerError) as validation_error:
		local_stack_control.lifecycle.validate_compose(target, FailureRunner(), tmp_path)
	messages = (str(compose_error.value), str(validation_error.value))
	assert all("Rust failure section [private]" in message for message in messages)
	assert all("SQL ERROR [private]" in message for message in messages)
	assert all("private-value" not in message for message in messages)


#============================================
def test_unspecified_private_values_keep_failure_detail_generic() -> None:
	"""Non-Compose callers retain the safe generic message without a redaction authority."""
	result = local_stack_control.models.CommandResult(("command",), 1, "useful child output", "useful child error")
	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.lifecycle.require_command(result, "other operation")
	assert "child reported a failure" in str(error.value)
	assert "useful child" not in str(error.value)


#============================================
def test_postgres_readiness_failure_retains_redacted_last_probe_detail(

	monkeypatch: pytest.MonkeyPatch, tmp_path: pathlib.Path,
) -> None:
	"""A failed PostgreSQL probe retains bounded redacted operator detail."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	secret = "private-postgres-password"
	result = local_stack_control.models.CommandResult(
		("podman", "compose"), 2, f"database rejected {secret}", "",
	)
	class ProbeFailureRunner(UnexpectedRunner):
		def run(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None, stdin: str | None = None) -> local_stack_control.models.CommandResult:
			return result
	def timeout(read_report: object, timeout_seconds: float) -> None:
		read_report()  # type: ignore[operator]
		raise local_stack_control.models.ControllerError("selected stack did not become ready: PostgreSQL is starting")
	monkeypatch.setattr(local_stack_control.lifecycle_wait, "poll_ready", timeout)
	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.lifecycle.wait_for_postgres(
			target, ProbeFailureRunner(),
			{"POSTGRES_USER": "ple", "POSTGRES_DB": "postgres", "POSTGRES_PASSWORD": secret},
			local_stack_control.lifecycle.LifecycleOptions(1, False, False, False),
		)
	assert "PostgreSQL readiness detail: database rejected [private]" in str(error.value)
	assert secret not in str(error.value)


#============================================
def test_postgres_readiness_failure_uses_redacted_service_log_when_probe_is_empty(
	monkeypatch: pytest.MonkeyPatch, tmp_path: pathlib.Path,
) -> None:
	"""An empty probe falls back to filtered, redacted PostgreSQL lifecycle evidence."""
	target = lifecycle_target(tmp_path, "containers", "containers/env.local")
	secret = "private-postgres-password"
	class ProbeThenLogRunner(UnexpectedRunner):
		def run(self, argv: list[str], environment: dict[str, str] | None = None, cwd: pathlib.Path | None = None, stdin: str | None = None) -> local_stack_control.models.CommandResult:
			if "pg_isready" in argv:
				return local_stack_control.models.CommandResult(tuple(argv), 2, "", "")
			return local_stack_control.models.CommandResult(
				tuple(argv), 1, f"database system is starting {secret}\nSELECT secret", "",
			)
	def timeout(read_report: object, timeout_seconds: float) -> None:
		read_report()  # type: ignore[operator]
		raise local_stack_control.models.ControllerError("selected stack did not become ready: PostgreSQL is starting")
	monkeypatch.setattr(local_stack_control.lifecycle_wait, "poll_ready", timeout)
	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.lifecycle.wait_for_postgres(
			target, ProbeThenLogRunner(),
			{"POSTGRES_USER": "ple", "POSTGRES_DB": "postgres", "POSTGRES_PASSWORD": secret},
			local_stack_control.lifecycle.LifecycleOptions(1, False, False, False),
		)
	assert "PostgreSQL readiness detail: database system is starting [private]" in str(error.value)
	assert secret not in str(error.value) and "SELECT" not in str(error.value)


#============================================
def test_teaching_environment_paths_follow_the_selected_private_environment(tmp_path: pathlib.Path) -> None:
	"""Teaching-profile defaults keep secrets and identity projection beside its selected env file."""
	target = lifecycle_target(
		tmp_path,
		"ple_live_demo_baseline_test",
		"walk/env.local",
	)
	target.env_file.parent.mkdir()
	target.env_file.write_text("PLE_GATEWAY_HOST_PORT=8123\n", encoding="ascii")
	target.env_file.chmod(0o600)
	local_stack_control.lifecycle.configure_default_environment(target, None)
	values = target.env_file.read_text(encoding="ascii")
	assert str(target.env_file.parent / ".secrets/invitation_token_secret") in values
	assert "PLE_LOCAL_AUTH_HOST_FILE" not in values
