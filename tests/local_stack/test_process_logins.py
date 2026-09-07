"""Offline contracts for disposable PostgreSQL service logins."""

import pathlib

import pytest
import yaml

import local_stack_control.models
import local_stack_control.process
import local_stack_control.process_logins


class ComposeLoader(yaml.SafeLoader):
	"""Parse the small Compose extension tag used by the live-demo overlay."""


#============================================
def _compose_extension(loader: ComposeLoader, node: yaml.Node) -> object:
	"""Construct a tagged Compose value without enabling arbitrary YAML objects."""
	if isinstance(node, yaml.SequenceNode):
		return loader.construct_sequence(node)
	if isinstance(node, yaml.MappingNode):
		return loader.construct_mapping(node)
	return loader.construct_scalar(node)


ComposeLoader.add_constructor("!override", _compose_extension)


#============================================
def _load_compose_mapping(path: pathlib.Path) -> dict[str, object]:
	"""Decode one Compose document with the bounded SafeLoader extension."""
	loader = ComposeLoader(path.read_text(encoding="utf-8"))
	try:
		decoded = loader.get_single_data()
	finally:
		loader.dispose()
	if not isinstance(decoded, dict) or not all(isinstance(key, str) for key in decoded):
		raise AssertionError("Compose fixture must decode to a string-keyed mapping")
	return decoded


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Record service-login setup input without starting a database or Compose."""

	def __init__(self) -> None:
		self.environment: dict[str, str] | None = None
		self.sql: str | None = None

	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		del argv, cwd
		self.environment = environment
		self.sql = stdin
		return local_stack_control.models.CommandResult(("psql",), 0, "", "")

	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		raise AssertionError("service-login setup does not stream commands")


#============================================
def target(tmp_path: pathlib.Path) -> local_stack_control.models.ComposeTarget:
	"""Build the smallest private Compose target used by service-login setup."""
	env_file = tmp_path / "env.local"
	env_file.write_text("POSTGRES_DB=ple\nPLE_POSTGRES_HOST_PORT=55432\n", encoding="utf-8")
	env_file.chmod(0o600)
	return local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="ple_test",
		env_file=env_file,
		compose_files=(),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman"),
		with_smtp=False,
		env_setting_names=(),
	)


#============================================
def values() -> dict[str, str]:
	"""Return the bounded administrator input that is never service state."""
	return {
		"POSTGRES_USER": "ple_admin",
		"POSTGRES_PASSWORD": "admin-private",
		"POSTGRES_DB": "ple",
		"PLE_POSTGRES_HOST_PORT": "55432",
	}


#============================================
def test_service_login_profiles_have_exact_set_only_memberships() -> None:
	"""Each closed profile first loses authority, then receives only its scope."""
	for login, roles, _ in local_stack_control.process_logins.LOGIN_PROFILES:
		sql = local_stack_control.process_logins.login_sql(login, roles, "a" * 64)
		grant_lines = [
			line
			for line in sql.splitlines()
			if line.startswith("GRANT ") and f" TO {login} " in line
		]
		expected_grants = [
			f"GRANT {role} TO {login} WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;"
			for role in roles
		]
		assert grant_lines == expected_grants
		assert "REVOKE %I FROM %I" in sql
		assert f"GRANT CONNECT ON DATABASE %I TO {login}" in sql
		assert "NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS" in sql


#============================================
def test_service_login_profiles_reject_unallowlisted_capability() -> None:
	"""Callers cannot assemble an arbitrary database authority profile."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.process_logins.login_sql(
			local_stack_control.process_logins.API_LOGIN,
			("ple_app", "ple_unexpected_capability"),
			"a" * 64,
		)


#============================================
def test_service_login_setup_writes_separate_service_urls_without_service_credentials_in_child(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Compose reads service credentials only from its selected private file."""
	runner = RecordingRunner()
	profile_passwords = tuple(
		chr(ord("a") + index) * 64
		for index, _ in enumerate(local_stack_control.process_logins.LOGIN_PROFILES)
	)
	passwords = iter(profile_passwords)
	monkeypatch.setattr(
		local_stack_control.process_logins.secrets,
		"token_hex",
		lambda size: next(passwords),
	)
	selected = target(tmp_path)
	host_urls = local_stack_control.process_logins.setup_service_logins(
		selected,
		runner,
		values(),
		{
			"PATH": "/bin",
			**{
				setting_name: "postgres://ambient-service-credential@postgres/ple"
				for _, _, setting_name in local_stack_control.process_logins.LOGIN_PROFILES
			},
		},
	)
	content = selected.env_file.read_text(encoding="utf-8")
	assert runner.environment == {"PATH": "/bin", "PGPASSWORD": "admin-private"}
	assert all(password not in runner.environment.values() for password in profile_passwords)
	expected_urls = tuple(
		f"postgres://{login}:{password}@postgres:5432/ple"
		for (login, _, _), password in zip(
			local_stack_control.process_logins.LOGIN_PROFILES,
			profile_passwords,
			strict=True,
		)
	)
	for url, (_, _, setting_name) in zip(
		expected_urls,
		local_stack_control.process_logins.LOGIN_PROFILES,
		strict=True,
	):
		assert f"{setting_name}={url}" in content
	assert host_urls == tuple(
		f"postgres://{login}:{password}@127.0.0.1:55432/ple"
		for (login, _, _), password in zip(
			local_stack_control.process_logins.LOGIN_PROFILES,
			profile_passwords,
			strict=True,
		)
	)


#============================================
def test_service_login_setup_failure_redacts_ephemeral_process_passwords() -> None:
	"""Failed service-login setup provides an actionable bounded diagnostic."""
	private_values = (
		"admin-private",
		"private-api",
		"private-worker",
		"private-recovery",
		"private-fast-path",
	)
	result = local_stack_control.models.CommandResult(
		("psql",), 1, "failed " + " ".join(private_values), "database refused",
	)
	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.process_logins.require_service_login_setup_success(
			result, private_values
		)
	assert all(value not in str(error.value) for value in private_values)


#============================================
