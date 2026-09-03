"""Focused contracts for the shared fixed production-auth live-demo target."""

import pathlib

import pytest

import local_stack_control.compose
import local_stack_control.disposable_stack_adapter
import local_stack_control.env_file
import local_stack_control.live_demo_target
import local_stack_control.models
import local_stack_control.private_files


#============================================
def selections() -> dict[str, str]:
	"""Return the required target-writer selection shape."""
	return {
		"PLE_WEBWORK_RENDERER_IMAGE": "renderer",
		"PLE_WEBWORK_RENDERER_BASE_URL": "http://webwork-renderer:3000/",
		"PLE_WEBWORK_RENDERER_ID": "renderer-id",
		"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS": "15",
		"PLE_WEBWORK_MAX_RESPONSE_BYTES": "1048576",
		"PLE_GATEWAY_IMAGE_SHA256": "gateway",
		"PLE_POSTGRES_IMAGE_SHA256": "postgres",
		"PLE_MINIO_IMAGE_SHA256": "minio",
		"PLE_MINIO_MC_IMAGE_SHA256": "minio-mc",
		"PLE_SECRET_INIT_IMAGE_SHA256": "secret-init",
	}


#============================================
def test_random_secret32_is_accepted_by_the_strict_private_secret_reader(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Generated private secrets satisfy the persisted secret representation."""
	expected_secret = bytes(range(32))
	monkeypatch.setattr(
		local_stack_control.live_demo_target.secrets,
		"token_bytes",
		lambda _requested_bytes: expected_secret,
	)
	encoded_secret = local_stack_control.live_demo_target.random_secret32()
	decoded_secret = local_stack_control.private_files.decode_base64url_secret32(
		encoded_secret.encode("ascii")
	)
	assert decoded_secret == expected_secret


#============================================
def create_topology(root: pathlib.Path) -> None:
	"""Create the three canonical Compose files used by profile policy tests."""
	primary = root / "containers" / "compose.yaml"
	live = root / "tests" / "e2e" / "compose.live-demo-browser.yaml"
	replica = root / "tests" / "e2e" / "compose.replica-e2e.yaml"
	primary.parent.mkdir(parents=True)
	live.parent.mkdir(parents=True)
	for path in (primary, live, replica):
		path.write_text("services: {}\n", encoding="ascii")


#============================================
def disposable_profile(
	root: pathlib.Path,
	profile: local_stack_control.models.LiveDemoProfile,
) -> local_stack_control.models.DisposableComposeTarget:
	"""Build one profile-selected target without reading external state."""
	create_topology(root)
	policy = local_stack_control.models.live_demo_profile_policy(profile)
	compose_files = tuple((root / path).resolve() for path in policy.compose_relative_paths)
	environment = root / "env.local"
	environment.write_text("SAFE=value\n", encoding="ascii")
	target = local_stack_control.models.ComposeTarget(
		repo_root=root,
		project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		env_file=environment,
		compose_files=compose_files,
		provider=local_stack_control.models.ComposeProvider(
			("podman-compose", "--in-pod", "false"), "podman-compose"
		),
		with_smtp=False,
		env_setting_names=("SAFE",),
	)
	return local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		capability_file=root / "capability",
		project_prefix=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
		private_environment_file=environment,
		live_demo_profile=profile,
	)


#============================================
def test_closed_profiles_have_exact_topology_and_child_capabilities() -> None:
	"""Each profile selects only its named overlays and bounded child authority."""
	browser = local_stack_control.models.live_demo_profile_policy(
		local_stack_control.models.LiveDemoProfile.BROWSER
	)
	webwork = local_stack_control.models.live_demo_profile_policy(
		local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC
	)
	replica = local_stack_control.models.live_demo_profile_policy(
		local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	assert browser.compose_relative_paths == webwork.compose_relative_paths == (
		"containers/compose.yaml",
		"tests/e2e/compose.live-demo-browser.yaml",
	)
	assert browser.child_capabilities == ("browser_lifecycle",)
	assert webwork.child_capabilities == (
		"bounded_renderer_log", "webwork_service_client",
	)
	assert webwork.outage_service == "webwork-renderer"
	assert browser.outage_service == "gateway"
	assert replica.compose_relative_paths[-1] == "tests/e2e/compose.replica-e2e.yaml"
	assert replica.outage_service is None
	assert replica.child_capabilities == (
		"bounded_replica_restart", "postgresql_count", "replica_service_client",
	)
	assert browser.application_image is None and webwork.application_image is None
	assert browser.service_replica_counts == webwork.service_replica_counts == ()
	assert (
		replica.application_image
		== local_stack_control.models.LIVE_DEMO_REPLICA_APPLICATION_IMAGE
	)
	assert replica.service_replica_counts == (("api", 2),)


#============================================
@pytest.mark.parametrize("profile", tuple(local_stack_control.models.LiveDemoProfile))
def test_writer_emits_fixed_production_auth_manifest(
	tmp_path: pathlib.Path,
	profile: local_stack_control.models.LiveDemoProfile,
) -> None:
	"""Every profile shares one owner/project writer without local-auth settings."""
	target = local_stack_control.live_demo_target.write_private_target(
		tmp_path,
		profile,
		local_stack_control.live_demo_target.LiveDemoPorts(53501, 54001, 54501, 55001),
		selections(),
	)
	manifest = local_stack_control.disposable_stack_adapter.load_manifest(tmp_path, target.manifest_path)
	values = local_stack_control.env_file.env_settings(target.environment_path)
	assert (
		manifest.owner,
		manifest.project,
		manifest.live_demo_profile,
		target.origin,
	) == (
		"live-demo-browser",
		"ple-live-demo-browser",
		profile,
		"https://localhost:55001/",
	)
	assert values["PLE_E2E_OWNER"] == "live-demo-browser"
	assert values["PLE_WEBAUTHN_ORIGIN"] == "https://localhost:55001"
	assert {
		name: values[name]
		for name in (
			"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID",
			"PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID",
			"PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID",
			"PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID",
			"PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID",
		)
	} == {
		"PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID": "00000000-0000-0000-0000-000000000101",
		"PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID": "00000000-0000-0000-0000-000000000102",
		"PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID": "00000000-0000-0000-0000-000000000103",
		"PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID": "00000000-0000-0000-0000-000000000104",
		"PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID": "00000000-0000-0000-0000-000000000105",
	}
	assert "PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_HOST_FILE" not in values
	assert all(
		name not in values
		for name in local_stack_control.live_demo_target.FORBIDDEN_LOCAL_AUTH_SETTINGS
	)


#============================================
def test_replica_application_image_exactly_matches_cleanup_authority(
	tmp_path: pathlib.Path,
) -> None:
	"""The private replica target and cleanup own the same explicit image tag."""
	target_directory = tmp_path / "target"
	target_directory.mkdir()
	target = local_stack_control.live_demo_target.write_private_target(
		target_directory,
		local_stack_control.models.LiveDemoProfile.REPLICA_RESTART,
		local_stack_control.live_demo_target.LiveDemoPorts(53501, 54001, 54501, 55001),
		selections(),
	)
	values = local_stack_control.env_file.env_settings(target.environment_path)
	replica = disposable_profile(
		tmp_path / "profile",
		local_stack_control.models.LiveDemoProfile.REPLICA_RESTART,
	)
	images = local_stack_control.disposable_stack_adapter.owned_project_images(replica)
	assert values["PLE_APPLICATION_IMAGE"] == images[0]
	assert images[0] == local_stack_control.models.LIVE_DEMO_REPLICA_APPLICATION_IMAGE


#============================================
@pytest.mark.parametrize(
	"profile",
	(
		local_stack_control.models.LiveDemoProfile.BROWSER,
		local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC,
	),
)
def test_non_replica_targets_retain_shared_application_image_behavior(
	tmp_path: pathlib.Path,
	profile: local_stack_control.models.LiveDemoProfile,
) -> None:
	"""Browser and WebWork profiles neither select nor remove an application tag."""
	target_directory = tmp_path / "target"
	target_directory.mkdir()
	target = local_stack_control.live_demo_target.write_private_target(
		target_directory,
		profile,
		local_stack_control.live_demo_target.LiveDemoPorts(53501, 54001, 54501, 55001),
		selections(),
	)
	values = local_stack_control.env_file.env_settings(target.environment_path)
	selected = disposable_profile(tmp_path / "profile", profile)
	assert "PLE_APPLICATION_IMAGE" not in values
	assert local_stack_control.disposable_stack_adapter.owned_project_images(selected) == (
		"localhost/ple-live-demo-browser_gateway:latest",
	)


#============================================
def test_manifest_rejects_missing_or_foreign_profile(tmp_path: pathlib.Path) -> None:
	"""The fixed owner cannot fall back to an implicit or caller-invented topology."""
	manifest = tmp_path / "disposable.manifest"
	manifest.write_text(
		"OWNER=live-demo-browser\nPROJECT=ple-live-demo-browser\n"
		"ENV_FILE=env.local\nCAPABILITY_FILE=capability\n",
		encoding="ascii",
	)
	manifest.chmod(0o600)
	with pytest.raises(local_stack_control.models.ControllerError, match="complete ownership"):
		local_stack_control.disposable_stack_adapter.load_manifest(tmp_path, manifest)
	manifest.write_text(
		"OWNER=live-demo-browser\nPROJECT=ple-live-demo-browser\nPROFILE=foreign\n"
		"ENV_FILE=env.local\nCAPABILITY_FILE=capability\n",
		encoding="ascii",
	)
	with pytest.raises(local_stack_control.models.ControllerError, match="supported profile"):
		local_stack_control.disposable_stack_adapter.load_manifest(tmp_path, manifest)


#============================================
def test_profile_rejects_foreign_compose_files(tmp_path: pathlib.Path) -> None:
	"""A profile value cannot be paired with a caller-selected Compose topology."""
	create_topology(tmp_path)
	environment = tmp_path / "env.local"
	environment.write_text("SAFE=value\n", encoding="ascii")
	foreign = tmp_path / "foreign.yaml"
	foreign.write_text("services: {}\n", encoding="ascii")
	target = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="ple-live-demo-browser",
		env_file=environment,
		compose_files=(foreign,),
		provider=local_stack_control.models.ComposeProvider(("podman-compose",), "podman-compose"),
		with_smtp=False,
		env_setting_names=("SAFE",),
	)
	with pytest.raises(local_stack_control.models.ControllerError, match="Compose files"):
		local_stack_control.compose.require_disposable_target_policy(
			target,
			local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
			local_stack_control.models.LiveDemoProfile.BROWSER,
		)


#============================================
def test_selected_adapter_actions_follow_the_closed_profile(tmp_path: pathlib.Path) -> None:
	"""Renderer logs and replica diagnostics cannot cross their profile boundary."""
	webwork = disposable_profile(
		tmp_path / "webwork", local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC
	)
	replica = disposable_profile(
		tmp_path / "replica", local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	browser = disposable_profile(
		tmp_path / "browser", local_stack_control.models.LiveDemoProfile.BROWSER
	)
	assert local_stack_control.disposable_stack_adapter.evidence_log_service(
		webwork, "renderer_delivery"
	) == "api"
	status, logs = local_stack_control.disposable_stack_adapter.diagnostic_commands(
		replica, ("api", "gateway")
	)
	assert status[-1] == "ps" and logs[-2:] == ["api", "gateway"]
	with pytest.raises(local_stack_control.models.ControllerError, match="diagnostics"):
		local_stack_control.disposable_stack_adapter.diagnostic_commands(browser, ("api",))
	with pytest.raises(local_stack_control.models.ControllerError, match="requested evidence"):
		local_stack_control.disposable_stack_adapter.evidence_log_service(replica, "renderer_delivery")


#============================================
def test_webwork_profile_has_only_its_exact_renderer_outage_authority(
	tmp_path: pathlib.Path,
) -> None:
	"""WebWork can stop/restart its renderer while replica and generic Compose stay denied."""
	webwork = disposable_profile(
		tmp_path / "webwork", local_stack_control.models.LiveDemoProfile.WEBWORK_RENDER_RPC
	)
	replica = disposable_profile(
		tmp_path / "replica", local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
	)
	assert local_stack_control.disposable_stack_adapter.outage_service(webwork) == "webwork-renderer"
	argv, _environment = local_stack_control.disposable_stack_adapter.outage_stop_command(webwork)
	assert argv[-2:] == ["stop", "webwork-renderer"]
	with pytest.raises(local_stack_control.models.ControllerError, match="generic Compose"):
		local_stack_control.disposable_stack_adapter.compose_command(webwork, ["ps"])
	with pytest.raises(local_stack_control.models.ControllerError, match="service outage"):
		local_stack_control.disposable_stack_adapter.outage_service(replica)


#============================================
def test_rendered_topology_accepts_production_auth_overlay() -> None:
	"""The production-auth overlay renders without local-auth declarations."""
	rendered = """services:
  api:
    environment:
    volumes:
      - type: volume
        source: ple_identity_runtime
        target: /run/ple-secrets
  gateway:
    labels:
      production-auth-note: PLE_AUTH_PROVIDER
"""
	local_stack_control.live_demo_target.require_production_auth_topology(rendered)


#============================================
def test_rendered_topology_retains_only_the_compose_reset_tag() -> None:
	"""The safe parser accepts Compose's reset tag and maps it to absence."""
	rendered = "services:\n  api:\n    environment: !reset\n"
	local_stack_control.live_demo_target.require_production_auth_topology(rendered)


#============================================
def test_rendered_topology_rejects_python_object_constructors() -> None:
	"""The topology boundary never constructs caller-selected Python objects."""
	rendered = (
		"services:\n  api:\n    environment: !!python/object/apply:builtins.str\n"
		"      - unsafe\n"
	)
	with pytest.raises(
		local_stack_control.models.ControllerError,
		match="not valid YAML",
	):
		local_stack_control.live_demo_target.require_production_auth_topology(rendered)


#============================================
@pytest.mark.parametrize(
	"rendered",
	(
		"services:\n  api:\n    environment:\n      PLE_AUTH_PROVIDER: local-file\n",
		(
			"services:\n  api:\n    environment:\n"
			"      - PLE_LOCAL_AUTH_FILE=/run/ple/local-identities.json\n"
		),
		"services:\n  api:\n    volumes:\n      - /run/ple/local-identities.json\n",
		(
			"services:\n  api:\n    volumes:\n      - type: bind\n"
			"        source: /tmp/identities\n"
			"        target: /run/ple/local-identities.json\n"
		),
	),
)
def test_rendered_topology_rejects_semantic_local_auth_fields(rendered: str) -> None:
	"""Only active environment declarations and the exact container mount are forbidden."""
	with pytest.raises(local_stack_control.models.ControllerError, match="local-auth setting"):
		local_stack_control.live_demo_target.require_production_auth_topology(rendered)
