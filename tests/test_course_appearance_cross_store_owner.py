"""Focused private-environment contracts for the course-appearance oracle."""

import pathlib

import pytest

import local_stack_control.course_appearance_cross_store_owner
import local_stack_control.disposable_stack_adapter
import local_stack_control.lifecycle_migrations
import local_stack_control.models


#============================================
def test_cross_store_compose_environment_receives_only_a_temporary_migrator_url(
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Required Compose interpolation receives the owner URL without an env-file write."""
	base_environment = {"PATH": "/usr/bin"}
	monkeypatch.setattr(
		local_stack_control.disposable_stack_adapter,
		"compose_environment",
		lambda disposable: dict(base_environment),
	)

	environment = (
		local_stack_control.course_appearance_cross_store_owner._compose_environment_with_migrator(
			object(), "postgres://ple_migrator:private-password@postgres:5432/ple_e2e_baseline"
		)
	)

	assert environment[local_stack_control.lifecycle_migrations.MIGRATION_URL_SETTING].startswith(
		"postgres://ple_migrator:"
	)
	assert base_environment == {"PATH": "/usr/bin"}


#============================================
def test_cross_store_e2e_child_receives_its_application_role_and_manifest() -> None:
	"""The direct E2E child uses the ordinary application role for Store checks."""
	environment = local_stack_control.course_appearance_cross_store_owner._e2e_child_environment(
		{
			"PATH": "/usr/bin",
			"DATABASE_URL": "postgres://ple_api_login:service-secret@postgres:5432/ple",
			"PLE_UNRELATED": "not-for-child",
		},
		pathlib.Path("/private/runtime.yaml"),
	)

	assert environment == {
		"PATH": "/usr/bin",
		"DATABASE_URL": "postgres://ple_api_login:service-secret@postgres:5432/ple",
		"PLE_ACCEPTANCE_RUNTIME_MANIFEST": "/private/runtime.yaml",
	}


#============================================
@pytest.mark.parametrize("url", ("", "http://wrong", "postgres://bad\nurl"))
def test_cross_store_compose_environment_refuses_invalid_migrator_urls(
	monkeypatch: pytest.MonkeyPatch,
	url: str,
) -> None:
	"""Invalid URL bytes cannot reach Compose interpolation."""
	monkeypatch.setattr(
		local_stack_control.disposable_stack_adapter,
		"compose_environment",
		lambda disposable: (_ for _ in ()).throw(AssertionError("must not build environment")),
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.course_appearance_cross_store_owner._compose_environment_with_migrator(
			object(), url
		)
