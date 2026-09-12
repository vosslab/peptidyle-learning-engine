"""Database-migrator lifecycle contracts for local and disposable stacks."""

import pathlib

import pytest

import local_stack_control.lifecycle_database
import local_stack_control.lifecycle_migrations
import local_stack_control.models
import local_stack_control.process


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Record closed child commands and return one selected result."""

	def __init__(self, returncode: int = 0, stderr: str = "") -> None:
		self.returncode = returncode
		self.stderr = stderr
		self.calls: list[tuple[list[str], dict[str, str] | None, pathlib.Path | None, str | None]] = []

	#============================================
	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		self.calls.append((argv, environment, cwd, stdin))
		return local_stack_control.models.CommandResult(tuple(argv), self.returncode, "", self.stderr)

	#============================================
	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		raise AssertionError("database migration does not stream child commands")


#============================================
def test_fresh_ordinary_database_uses_one_canonical_initialize_operation(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""A fresh ordinary stack builds then initializes through the migrator."""
	target = compose_target(tmp_path)
	runner = RecordingRunner()
	monkeypatch.setattr(
		local_stack_control.lifecycle_migrations,
		"migration_database_url_for",
		lambda selected, selected_runner, values: "postgres://private",
	)

	local_stack_control.lifecycle_migrations.run_migrations(
		target, runner, tmp_path, migration_values(), {}
	)

	assert runner.calls[0][0][-2:] == ["build", "database-migrator"]
	assert runner.calls[1][0][-3:] == ["database-migrator", "database", "initialize"]
	assert runner.calls[1][1] is not None
	assert runner.calls[1][1]["PLE_MIGRATION_DATABASE_URL"] == "postgres://private"
	assert local_stack_control.lifecycle_migrations.database_operation_for(target) == "migrate"


#============================================
def test_initialized_ordinary_database_uses_one_canonical_migrate_operation(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""An ordinary stack with its existing lifecycle fact applies forwards."""
	target = compose_target(tmp_path)
	target.env_file.write_text(
		"POSTGRES_PASSWORD=very-secret\n"
		"PLE_MIGRATION_DATABASE_URL=postgres://previous-private-url\n",
		encoding="utf-8",
	)
	runner = RecordingRunner()
	monkeypatch.setattr(
		local_stack_control.lifecycle_migrations,
		"migration_database_url_for",
		lambda selected, selected_runner, values: "postgres://private",
	)

	local_stack_control.lifecycle_migrations.run_migrations(
		target, runner, tmp_path, migration_values(), {}
	)

	assert runner.calls[0][0][-2:] == ["build", "database-migrator"]
	assert runner.calls[1][0][-3:] == ["database-migrator", "database", "migrate"]


#============================================
def test_failed_first_initialize_keeps_the_next_attempt_on_initialize(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The first-start fact is committed only after the database action succeeds."""
	target = compose_target(tmp_path)
	runner = RecordingRunner(returncode=1, stderr="database initialize failed")
	monkeypatch.setattr(
		local_stack_control.lifecycle_migrations,
		"migration_database_url_for",
		lambda selected, selected_runner, values: "postgres://temporary-private-url",
	)
	monkeypatch.setattr(
		local_stack_control.lifecycle_migrations, "build_database_migrator", lambda *args: None
	)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.lifecycle_migrations.run_migrations(
			target, runner, tmp_path, migration_values(), {}
		)

	assert runner.calls[0][1] is not None
	assert runner.calls[0][1]["PLE_MIGRATION_DATABASE_URL"] == "postgres://temporary-private-url"
	assert "PLE_MIGRATION_DATABASE_URL" not in local_stack_control.env_file.env_settings(target.env_file)
	assert local_stack_control.lifecycle_migrations.database_operation_for(target) == "initialize"


#============================================
def test_failed_database_command_redacts_private_database_url(
	tmp_path: pathlib.Path,
) -> None:
	"""Child diagnostics never echo the private migration URL or password."""
	target = compose_target(tmp_path)
	secret_url = "postgres://ple_migrator:very-secret@postgres:5432/ple"
	target.env_file.write_text(
		f"PLE_MIGRATION_DATABASE_URL={secret_url}\nPOSTGRES_PASSWORD=very-secret\n",
		encoding="utf-8",
	)
	target.env_file.chmod(0o600)
	runner = RecordingRunner(returncode=1, stderr=f"connection failed for {secret_url}")

	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.lifecycle_migrations.run_database_command(
			target, runner, "migrate", "database migrate"
		)

	assert "very-secret" not in str(error.value)
	assert secret_url not in str(error.value)
	assert "[private]" in str(error.value)
	assert runner.calls[0][0][-3:] == ["database-migrator", "database", "migrate"]


#============================================
def test_application_schema_verify_uses_its_url_without_replacing_migrator_state(
	tmp_path: pathlib.Path,
) -> None:
	"""Application verification is a temporary capability, not lifecycle state."""
	target = compose_target(tmp_path)
	target.env_file.write_text(
		"POSTGRES_PASSWORD=very-secret\n"
		"PLE_MIGRATION_DATABASE_URL=postgres://migrator-url\n"
		"PLE_API_DATABASE_URL=postgres://application-url\n",
		encoding="utf-8",
	)
	runner = RecordingRunner()

	local_stack_control.lifecycle_migrations.verify_migrated_application_schema(target, runner)

	assert runner.calls[0][1] is not None
	assert runner.calls[0][1]["PLE_MIGRATION_DATABASE_URL"] == "postgres://application-url"
	assert "postgres://application-url" not in runner.calls[0][0]
	assert "postgres://migrator-url" not in runner.calls[0][0]
	assert local_stack_control.env_file.env_settings(target.env_file)[
		"PLE_MIGRATION_DATABASE_URL"
	] == "postgres://migrator-url"


#============================================
def test_failed_application_schema_verify_redacts_its_temporary_url(
	tmp_path: pathlib.Path,
) -> None:
	"""The API URL stays in its temporary child environment and diagnostics redact it."""
	target = compose_target(tmp_path)
	target.env_file.write_text(
		"POSTGRES_PASSWORD=very-secret\n"
		"PLE_MIGRATION_DATABASE_URL=postgres://migrator-url\n"
		"PLE_API_DATABASE_URL=postgres://application-url\n",
		encoding="utf-8",
	)
	runner = RecordingRunner(returncode=1, stderr="postgres://application-url")

	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.lifecycle_migrations.verify_migrated_application_schema(target, runner)

	assert "postgres://application-url" not in str(error.value)
	assert local_stack_control.env_file.env_settings(target.env_file)[
		"PLE_MIGRATION_DATABASE_URL"
	] == "postgres://migrator-url"


#============================================
def test_non_tls_target_bootstraps_and_uses_the_dedicated_migrator(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Every local target sends schema work through the restricted migrator URL."""
	target = compose_target(tmp_path)
	runner = RecordingRunner()
	monkeypatch.setattr(
		local_stack_control.lifecycle_migrations.secrets, "token_hex", lambda length: "migratorsecret"
	)

	url = local_stack_control.lifecycle_migrations.migration_database_url_for(
		target, runner, migration_values()
	)

	assert url == "postgres://ple_migrator:migratorsecret@postgres:5432/ple"
	assert "postgres:very-secret" not in url
	assert runner.calls[0][0][-8:] == [
		"psql", "-X", "-v", "ON_ERROR_STOP=1", "-U", "postgres", "-d", "ple",
	]
	assert runner.calls[0][3] is not None
	assert "CREATE ROLE ple_database_owner" in runner.calls[0][3]
	assert "\\gset" not in runner.calls[0][3]


#============================================
def test_failed_principal_bootstrap_redacts_both_platform_and_migrator_secrets(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Bootstrap diagnostics keep both credential classes out of operator output."""
	target = compose_target(tmp_path)
	runner = RecordingRunner(returncode=1, stderr="very-secret migratorsecret")
	monkeypatch.setattr(
		local_stack_control.lifecycle_migrations.secrets, "token_hex", lambda length: "migratorsecret"
	)

	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.lifecycle_migrations.migration_database_url_for(
			target, runner, migration_values()
		)

	assert "very-secret" not in str(error.value)
	assert "migratorsecret" not in str(error.value)
	assert "[private]" in str(error.value)


#============================================
def test_existing_principal_action_rotates_only_the_migrator_credential() -> None:
	"""An initialized stack has one bounded platform action before migration."""
	sql = local_stack_control.lifecycle_migrations.migration_principal_sql(
		"ple", "migratorsecret", initial_bootstrap=False
	)

	assert sql == "ALTER ROLE ple_migrator PASSWORD 'migratorsecret';\n"
	assert "GRANT" not in sql
	assert "REVOKE" not in sql


#============================================
def test_platform_bootstrap_accepts_the_runtime_migrator_credential_alphabet() -> None:
	"""The private URL generator may use URL-safe Base64 punctuation in a password."""
	sql = local_stack_control.lifecycle_database.migration_principal_bootstrap_sql(
		"ple", "Use_only-ASCII-safe_credential-characters"
	)

	assert "PASSWORD 'Use_only-ASCII-safe_credential-characters';" in sql


#============================================
@pytest.mark.parametrize(
	"password",
	("has space", "has\ttab", "line\nbreak", "nul\x00", "quote'", "semi;colon", "slash/"),
)
def test_platform_bootstrap_rejects_unsafe_migrator_passwords(password: str) -> None:
	"""Bootstrap credentials remain a positive ASCII allowlist before fixed SQL."""
	with pytest.raises(local_stack_control.models.ControllerError, match="settings are invalid"):
		local_stack_control.lifecycle_database.migration_principal_bootstrap_sql("ple", password)


#============================================
def compose_target(tmp_path: pathlib.Path) -> local_stack_control.models.ComposeTarget:
	"""Build one private ordinary Compose target for migration command tests."""
	env_file = tmp_path / "local.env"
	env_file.write_text("POSTGRES_PASSWORD=very-secret\n", encoding="utf-8")
	env_file.chmod(0o600)
	return local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="ple-test",
		env_file=env_file,
		compose_files=(),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman compose"),
		with_smtp=False,
		env_setting_names=("POSTGRES_PASSWORD",),
	)


#============================================
def migration_values() -> dict[str, str]:
	"""Return the private values accepted by the migration URL helper."""
	return {
		"POSTGRES_USER": "postgres",
		"POSTGRES_PASSWORD": "very-secret",
		"POSTGRES_DB": "ple",
	}
