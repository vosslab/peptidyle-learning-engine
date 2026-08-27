"""Offline contracts for disposable API and worker PostgreSQL logins."""

import pathlib

import pytest
import yaml

import file_utils
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


class RecordingRunner(local_stack_control.process.CommandRunner):
	"""Record provisioning input without starting a database or Compose."""

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
		raise AssertionError("process login provisioning does not stream commands")


#============================================
def target(tmp_path: pathlib.Path) -> local_stack_control.models.ComposeTarget:
	"""Build the smallest private Compose target used by the provisioner."""
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
def test_process_login_profiles_have_exact_set_only_memberships() -> None:
	"""Role reset grants only the API or ordinary worker capability set."""
	sql = local_stack_control.process_logins.login_sql(
		local_stack_control.process_logins.WORKER_LOGIN,
		local_stack_control.process_logins.WORKER_ROLES,
		"a" * 64,
	)
	assert "GRANT ple_accepted_submission_execution TO ple_worker_login WITH INHERIT FALSE, SET TRUE, ADMIN FALSE" in sql
	assert "NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS" in sql


#============================================
def test_process_login_profiles_reject_unallowlisted_capability() -> None:
	"""Callers cannot assemble an arbitrary database authority profile."""
	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.process_logins.login_sql(
			local_stack_control.process_logins.API_LOGIN,
			("ple_app", "ple_accepted_submission_execution"),
			"a" * 64,
		)


#============================================
def test_provision_writes_separate_service_urls_without_password_child_environment(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""Compose reads only role-specific URLs from its selected private file."""
	runner = RecordingRunner()
	passwords = iter(("a" * 64, "b" * 64))
	monkeypatch.setattr(
		local_stack_control.process_logins.secrets,
		"token_hex",
		lambda size: next(passwords),
	)
	selected = target(tmp_path)
	local_stack_control.process_logins.provision(
		selected, runner, values(), {"PATH": "/bin"}
	)
	content = selected.env_file.read_text(encoding="utf-8")
	assert "PLE_API_DATABASE_URL=postgres://ple_api_login:" + "a" * 64 in content
	assert "PLE_WORKER_DATABASE_URL=postgres://ple_worker_login:" + "b" * 64 in content
	assert runner.environment == {"PATH": "/bin", "PGPASSWORD": "admin-private"}


#============================================
def test_provisioning_failure_redacts_ephemeral_process_passwords() -> None:
	"""Failed provisioning provides an actionable bounded diagnostic."""
	result = local_stack_control.models.CommandResult(
		("psql",), 1, "failed private-api private-worker", "database refused",
	)
	with pytest.raises(local_stack_control.models.ControllerError) as error:
		local_stack_control.process_logins.require_provision_success(
			result, ("private-api", "private-worker")
		)
	assert "private-api" not in str(error.value)


#============================================
def test_compose_assigns_distinct_database_variables_to_api_and_worker() -> None:
	"""The shipped service topology preserves the two process authority paths."""
	compose_path = pathlib.Path(file_utils.get_repo_root()) / "containers" / "compose.yaml"
	services = yaml.load(compose_path.read_text(encoding="utf-8"), Loader=ComposeLoader)["services"]
	api_environment = services["api"]["environment"]
	worker_environment = services["worker"]["environment"]
	assert "PLE_API_DATABASE_URL" in api_environment["DATABASE_URL"]
	assert "DATABASE_URL" not in worker_environment and "PLE_WORKER_DATABASE_URL" in worker_environment
	assert worker_environment["PLE_GRADER_DATABASE_URL"] == api_environment["PLE_GRADER_DATABASE_URL"]
	assert {
		name: worker_environment[name]
		for name in (
			"PLE_WEBWORK_RENDERER_BASE_URL",
			"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS",
			"PLE_WEBWORK_MAX_RESPONSE_BYTES",
			"PLE_WEBWORK_RENDERER_ID",
			"PLE_WEBWORK_RENDERER_VERSION",
		)
	} == {
		name: api_environment[name]
		for name in (
			"PLE_WEBWORK_RENDERER_BASE_URL",
			"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS",
			"PLE_WEBWORK_MAX_RESPONSE_BYTES",
			"PLE_WEBWORK_RENDERER_ID",
			"PLE_WEBWORK_RENDERER_VERSION",
		)
	}
	assert services["worker"]["depends_on"]["webwork-renderer"] == {
		"condition": "service_healthy"
	}
	assert services["worker"]["networks"] == ["default", "renderer_private"]

	overlay_path = pathlib.Path(file_utils.get_repo_root()) / "tests/e2e/compose.live-demo-browser.yaml"
	overlay = yaml.load(overlay_path.read_text(encoding="utf-8"), Loader=ComposeLoader)["services"]
	assert overlay["api"]["environment"]["PLE_QTI_RUNTIME_ENABLED"] == "1"
	assert overlay["worker"]["environment"]["PLE_QTI_RUNTIME_ENABLED"] == "1"
