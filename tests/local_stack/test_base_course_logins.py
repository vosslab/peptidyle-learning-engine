"""Offline contracts for the closed Base Course PostgreSQL login boundary."""

import pathlib

import pytest

import local_stack_control.base_course_logins
import local_stack_control.models
import local_stack_control.process


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Record one provisioning command without starting PostgreSQL."""

	def __init__(
		self, result: local_stack_control.models.CommandResult | None = None,
	) -> None:
		self.calls: list[tuple[list[str], dict[str, str], str | None]] = []
		self.result = result

	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		del cwd
		self.calls.append((argv, {} if environment is None else environment, stdin))
		if self.result is not None:
			return self.result
		return local_stack_control.models.CommandResult(tuple(argv), 0, "", "")

	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		raise AssertionError("login provisioning does not stream commands")


#============================================
def target(tmp_path: pathlib.Path) -> local_stack_control.models.ComposeTarget:
	"""Build the minimum closed Compose target required by login provisioning."""
	return local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="ple_test",
		env_file=tmp_path / "private.env",
		compose_files=(),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman"),
		with_smtp=False,
		env_setting_names=(),
	)


#============================================
def values() -> dict[str, str]:
	"""Return minimal private settings without an actual credential artifact."""
	return {
		"POSTGRES_USER": "ple_admin",
		"POSTGRES_PASSWORD": "admin-private",
		"POSTGRES_DB": "ple",
		"PLE_POSTGRES_HOST_PORT": "55432",
		"PLE_QUESTION_ID_SECRET_HOST_FILE": "/private/question-secret",
	}


#============================================
def test_provision_resets_both_exact_memberships_without_argv_or_env_secrets(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""One transactional stdin script supplies two independently bounded pools."""
	runner = RecordingRunner()
	passwords = iter(("a" * 64, "b" * 64))
	monkeypatch.setattr(
		local_stack_control.base_course_logins.secrets,
		"token_hex",
		lambda size: next(passwords),
	)

	installer_url, app_url = local_stack_control.base_course_logins.provision(
		target(tmp_path), runner, values(), {"PATH": "/bin"}
	)

	assert installer_url.endswith("@127.0.0.1:55432/ple")
	assert app_url.endswith("@127.0.0.1:55432/ple")
	argv, environment, sql = runner.calls[0]
	assert "a" * 64 not in " ".join(argv)
	assert "b" * 64 not in " ".join(argv)
	assert "a" * 64 not in environment.values()
	assert "b" * 64 not in environment.values()
	assert environment == {"PATH": "/bin", "PGPASSWORD": "admin-private"}
	assert sql is not None and sql.startswith("BEGIN;\n") and sql.endswith("COMMIT;\n")
	assert "GRANT ple_base_course_installer TO ple_base_course_installer_login" in sql
	assert "GRANT ple_app TO ple_base_course_app_login" in sql
	assert "WITH INHERIT FALSE, SET TRUE, ADMIN FALSE" in sql
	assert "REVOKE %I FROM %I" in sql


#============================================
def test_reset_sql_is_idempotent_and_refuses_cross_role_profile() -> None:
	"""Repeated reset uses the same closed script and cannot swap pool authority."""
	password = "c" * 64
	first = local_stack_control.base_course_logins.login_sql(
		local_stack_control.base_course_logins.BASE_COURSE_APP_LOGIN,
		local_stack_control.base_course_logins.BASE_COURSE_APP_ROLE,
		password,
	)
	second = local_stack_control.base_course_logins.login_sql(
		local_stack_control.base_course_logins.BASE_COURSE_APP_LOGIN,
		local_stack_control.base_course_logins.BASE_COURSE_APP_ROLE,
		password,
	)
	assert first == second
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.base_course_logins.login_sql(
			local_stack_control.base_course_logins.BASE_COURSE_APP_LOGIN,
			local_stack_control.base_course_logins.BASE_COURSE_INSTALLER_ROLE,
			password,
		)


#============================================
def test_base_course_child_gets_two_urls_but_never_migration_authority() -> None:
	"""The two pools are the entire private Base Course database authority."""
	installer_url = "postgres://installer:" + "d" * 64 + "@127.0.0.1:5432/ple"
	app_url = "postgres://app:" + "e" * 64 + "@127.0.0.1:5432/ple"
	child = local_stack_control.base_course_logins.child_environment(
		{
			"PATH": "/bin",
			"PLE_MIGRATION_DATABASE_URL": "postgres://admin:private@localhost/ple",
			"PLE_OTHER_SECRET": "unrelated",
			"COMPOSE_PROJECT_NAME": "not-for-cargo",
			"PGPASSWORD": "not-for-cargo",
		},
		values(),
		installer_url,
		app_url,
	)
	assert child == {
		"PATH": "/bin",
		"PLE_BASE_COURSE_INSTALLER_DATABASE_URL": installer_url,
		"PLE_BASE_COURSE_APP_DATABASE_URL": app_url,
		"PLE_BASE_COURSE_DEPLOYMENT_MODE": "local",
		"PLE_QUESTION_ID_SECRET_FILE": "/private/question-secret",
	}


#============================================
def test_provisioning_failure_redacts_both_passwords() -> None:
	"""Retained diagnostics do not disclose either process-local secret."""
	result = local_stack_control.models.CommandResult(
		("psql",), 1, "failed private-one private-two", "database refused",
	)
	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.base_course_logins.require_provision_success(
			result, ("private-one", "private-two")
		)
	message = str(error.value)
	assert "private-one" not in message and "private-two" not in message
	assert "[private]" in message
