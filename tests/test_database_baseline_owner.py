"""Focused contracts for the canonical PostgreSQL baseline owner."""

import pathlib

import pytest

import local_stack_control.database_baseline_owner
import local_stack_control.models


#============================================
def test_baseline_application_child_receives_its_api_database_url_without_inherited_ple_state(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The Draft stores use the ordinary API capability supplied by the owner."""
	monkeypatch.setattr(
		local_stack_control.database_baseline_owner.os,
		"environ",
		{
			"PATH": "/usr/bin",
			"PLE_MIGRATION_DATABASE_URL": "postgres://ple_migrator:secret@postgres/ple",
			"COMPOSE_PROJECT_NAME": "private-project",
		},
	)

	environment = local_stack_control.database_baseline_owner._application_child_environment(
		"postgres://ple_api_login:service-secret@127.0.0.1:5432/ple_e2e_baseline"
	)

	assert environment == {
		"PATH": "/usr/bin",
		"DATABASE_URL": "postgres://ple_api_login:service-secret@127.0.0.1:5432/ple_e2e_baseline",
	}


#============================================
@pytest.mark.parametrize("url", ("", "http://wrong", "postgres://ple_migrator:secret@postgres/ple", "postgres://ple_api_login:bad\nurl"))
def test_baseline_application_child_rejects_an_unexpected_database_url(url: str) -> None:
	"""Only the generated API login can cross into application acceptance."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.database_baseline_owner._application_child_environment(url)


#============================================
def _baseline_target(root: pathlib.Path) -> local_stack_control.models.DisposableComposeTarget:
	"""Build the smallest owner-scoped target for container selection."""
	target = local_stack_control.models.ComposeTarget(
		repo_root=root,
		project="ple-live-demo-browser",
		env_file=root / "env.local",
		compose_files=(),
		provider=local_stack_control.models.ComposeProvider(("podman",), "podman"),
		with_smtp=False,
		env_setting_names=(),
	)
	return local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
		capability_file=root / "capability",
		project_prefix="ple-live-demo-browser",
		private_environment_file=root / "env.local",
	)


#============================================
def _postgres(identifier: str, running: bool = True) -> local_stack_control.models.ContainerResource:
	"""Build a label-resolved PostgreSQL record from a trusted snapshot."""
	return local_stack_control.models.ContainerResource(
		id=identifier,
		names=(),
		project="ple-live-demo-browser",
		service="postgres",
		state="running" if running else "exited",
		running=running,
		exit_code=0,
		health="healthy",
		image="postgres",
		ports=(),
	)


#============================================
def test_unrelease_receives_one_owner_resolved_postgres_container(tmp_path: pathlib.Path) -> None:
	"""The child receives only one full opaque container identity."""
	identifier = "a" * 64
	snapshot = local_stack_control.models.ProjectSnapshot(
		project="ple-live-demo-browser",
		containers=(_postgres(identifier),),
		volumes=(),
		networks=(),
	)
	assert local_stack_control.database_baseline_owner._postgres_container_id(
		_baseline_target(tmp_path), snapshot
	) == identifier


#============================================
@pytest.mark.parametrize("identifier", ("a" * 12, "A" * 64, "z" * 64))
def test_unrelease_rejects_nonopaque_postgres_container_identity(
	tmp_path: pathlib.Path, identifier: str
) -> None:
	"""The acceptance child cannot receive a partial or malformed target."""
	snapshot = local_stack_control.models.ProjectSnapshot(
		project="ple-live-demo-browser",
		containers=(_postgres(identifier),),
		volumes=(),
		networks=(),
	)
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.database_baseline_owner._postgres_container_id(
			_baseline_target(tmp_path), snapshot
		)


#============================================
