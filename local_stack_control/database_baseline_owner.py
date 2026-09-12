"""Lease-owned canonical PostgreSQL baseline lifecycle."""

from __future__ import annotations

import os
import pathlib
import re
import socket
import time
import urllib.parse
from collections.abc import Callable

import local_stack_control.acceptance_profile_owner
import local_stack_control.browser_suite_lease
import local_stack_control.compose
import local_stack_control.disposable_stack_adapter
import local_stack_control.env_file
import local_stack_control.lifecycle_commands
import local_stack_control.lifecycle_database
import local_stack_control.lifecycle_migrations
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process_logins
import local_stack_control.process
import local_stack_control.runtime_manifest


POSTGRES_CONTAINER_ID_PATTERN = re.compile(r"^[a-f0-9]{64}$")


#============================================
def _select_loopback_port() -> int:
	"""Ask the kernel for one loopback port while the fixed lease is held."""
	with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
		listener.bind(("127.0.0.1", 0))
		return int(listener.getsockname()[1])


#============================================
def _password_from_private_url(url_path: pathlib.Path, expected_login: str) -> str:
	"""Read one already-validated private PostgreSQL password for its owning child."""
	# ASVS 1.2.5 and 2.2.1: parse the closed locator before its password is sent
	# only over psql stdin; never place it in an argv or diagnostic.
	content = local_stack_control.private_files.read_current_user_private_file(url_path, 4_096)
	try:
		parsed = urllib.parse.urlsplit(content.decode("ascii").strip())
	except UnicodeDecodeError as error:
		raise local_stack_control.models.ControllerError("baseline credential is invalid") from error
	if parsed.username != expected_login or parsed.password is None:
		raise local_stack_control.models.ControllerError("baseline credential is invalid")
	return parsed.password


#============================================
def _baseline_target(
	repository_root: pathlib.Path,
	workspace: pathlib.Path,
	runner: local_stack_control.process.CommandRunner,
) -> local_stack_control.models.DisposableComposeTarget:
	"""Load the one owner-created target through the closed runtime manifest."""
	manifest = local_stack_control.disposable_stack_adapter.load_manifest(
		repository_root,
		workspace / local_stack_control.runtime_manifest.MANIFEST_NAME,
	)
	return local_stack_control.disposable_stack_adapter.disposable_target(
		runner, repository_root, manifest
	)


#============================================
def _require_command(
	runner: local_stack_control.process.CommandRunner,
	argv: list[str],
	environment: dict[str, str],
	workspace: pathlib.Path,
	description: str,
	private_values: tuple[str, ...],
	stdin: str | None = None,
) -> str:
	"""Run one fixed child and retain only its successful standard output."""
	result = runner.run(argv, environment, workspace, stdin)
	local_stack_control.lifecycle_commands.require_command(result, description, private_values)
	return result.stdout


#============================================
def _network_migrator_url(migrator_password: str) -> str:
	"""Construct the private in-network URL used only by the migrator job."""
	return local_stack_control.lifecycle_database.database_url(
		{
			"POSTGRES_USER": local_stack_control.runtime_manifest.POSTGRES_MIGRATOR_ROLE,
			"POSTGRES_PASSWORD": migrator_password,
			"POSTGRES_DB": local_stack_control.runtime_manifest.DATABASE_NAME,
		}
	)


#============================================
def _application_child_environment(service_database_url: str) -> dict[str, str]:
	"""Give one bounded child the API login it needs, without inherited PLE state."""
	if not service_database_url.startswith("postgres://ple_api_login:") or any(
		character in service_database_url for character in "\r\n\x00"
	):
		raise local_stack_control.models.ControllerError("application service database URL is invalid")
	environment = {
		name: value
		for name, value in os.environ.items()
		if not name.startswith("PLE_") and not name.startswith("COMPOSE_")
	}
	environment["DATABASE_URL"] = service_database_url
	return environment


#============================================
def _postgres_container_id(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> str:
	"""Select the one running PostgreSQL container from the owned snapshot."""
	if snapshot.project != disposable.target.project:
		raise local_stack_control.models.ControllerError(
			"database baseline snapshot does not match its owned project"
		)
	selected = tuple(
		container
		for container in snapshot.containers
		if container.service == "postgres" and container.running
	)
	if len(selected) != 1 or POSTGRES_CONTAINER_ID_PATTERN.fullmatch(selected[0].id) is None:
		raise local_stack_control.models.ControllerError(
			"database baseline requires exactly one running PostgreSQL container"
		)
	return selected[0].id


#============================================
def _run_oracle(repository_root: pathlib.Path, workspace: pathlib.Path, port: int) -> None:
	"""Build, replay, and verify the one ordinary disposable database lifecycle."""
	runtime = local_stack_control.runtime_manifest.write_database_baseline_runtime(workspace, port)
	runner = local_stack_control.process.SubprocessRunner()
	disposable = _baseline_target(repository_root, workspace, runner)
	compose_environment = local_stack_control.disposable_stack_adapter.compose_environment(disposable)
	private_values = local_stack_control.disposable_stack_adapter.private_environment_values(
		disposable.target.env_file
	)
	admin_password = _password_from_private_url(runtime.admin_url_path, "ple_e2e_migrator")
	migrator_password = _password_from_private_url(runtime.migrator_url_path, "ple_migrator")
	migration_url = _network_migrator_url(migrator_password)
	# Compose resolves required variables before it selects a profile, so the
	# network-only locator must exist before PostgreSQL itself starts.
	local_stack_control.lifecycle_migrations.write_migration_database_url(
		disposable.target.env_file, migration_url
	)
	private_values = private_values + (migration_url,)

	def compose(arguments: list[str], description: str, stdin: str | None = None) -> str:
		argv, environment = local_stack_control.disposable_stack_adapter.compose_command(
			disposable, arguments
		)
		return _require_command(
			runner, argv, environment, repository_root, description, private_values, stdin
		)

	compose(["up", "-d", "postgres"], "database baseline PostgreSQL startup")
	for _ in range(30):
		ready_argv, ready_environment = local_stack_control.disposable_stack_adapter.compose_command(
			disposable,
			["exec", "-T", "postgres", "pg_isready", "-U", "ple_e2e_migrator", "-d", "postgres"],
		)
		ready = runner.run(ready_argv, ready_environment, repository_root)
		if ready.ok():
			break
		time.sleep(1)
	else:
		raise local_stack_control.models.ControllerError("database baseline PostgreSQL did not become ready")

	admin_environment = dict(compose_environment)
	admin_environment["PGPASSWORD"] = admin_password
	create_database_argv, _ = local_stack_control.disposable_stack_adapter.compose_command(
		disposable,
		["exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1", "-U", "ple_e2e_migrator", "-d", "postgres"],
	)
	_require_command(
		runner,
		create_database_argv,
		admin_environment,
		repository_root,
		"database baseline database creation",
		private_values + (admin_password, migrator_password),
		"CREATE DATABASE ple_e2e_baseline;\n",
	)
	bootstrap_argv, _ = local_stack_control.disposable_stack_adapter.compose_command(
		disposable,
		["exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1", "-U", "ple_e2e_migrator", "-d", "ple_e2e_baseline"],
	)
	_require_command(
		runner,
		bootstrap_argv,
		admin_environment,
		repository_root,
		"database baseline principal bootstrap",
		private_values + (admin_password, migrator_password),
		local_stack_control.lifecycle_database.migration_principal_bootstrap_sql(
			"ple_e2e_baseline", migrator_password
		),
	)
	compose(["--profile", "migration", "build", "database-migrator"], "database migrator build")
	_require_command(
		runner,
		bootstrap_argv,
		admin_environment,
		repository_root,
		"database baseline unrelated public table",
		private_values + (admin_password, migrator_password),
		"CREATE TABLE public.ple_baseline_unrelated_structure (identifier integer);\n",
	)
	contaminated_initialize_argv, contaminated_initialize_environment = (
		local_stack_control.disposable_stack_adapter.compose_command(
			disposable,
			[
				"--profile", "migration", "run", "--rm", "--no-deps",
				"database-migrator", "database", "initialize",
			],
		)
	)
	if runner.run(
		contaminated_initialize_argv,
		contaminated_initialize_environment,
		repository_root,
	).ok():
		raise local_stack_control.models.ControllerError(
			"database initialize accepted unrelated public structure"
		)
	no_ple_schemas_argv = [*bootstrap_argv, "-tA"]
	no_ple_schemas = _require_command(
		runner,
		no_ple_schemas_argv,
		admin_environment,
		repository_root,
		"database baseline failed-install rollback check",
		private_values + (admin_password, migrator_password),
		"SELECT count(*) FROM pg_catalog.pg_namespace WHERE nspname LIKE 'ple\\_%';\n",
	)
	if no_ple_schemas.strip() != "0":
		raise local_stack_control.models.ControllerError(
			"database initialize left PLE schemas after rejecting unrelated structure"
		)
	_require_command(
		runner,
		bootstrap_argv,
		admin_environment,
		repository_root,
		"database baseline unrelated public table cleanup",
		private_values + (admin_password, migrator_password),
		"DROP TABLE public.ple_baseline_unrelated_structure;\n",
	)
	compose(
		[
			"--profile", "migration", "run", "--rm", "--no-deps",
			"database-migrator", "database", "initialize",
		],
		"database initialize",
	)
	replay = compose(
		[
			"--profile", "migration", "run", "--rm", "--no-deps",
			"database-migrator", "database", "initialize",
		],
		"database initialize replay",
	)
	if replay.strip() != "database initialize: complete and compatible":
		raise local_stack_control.models.ControllerError("database initialize replay did not converge")
	service_values = local_stack_control.env_file.env_settings(disposable.target.env_file)
	service_values["POSTGRES_DB"] = "ple_e2e_baseline"
	service_values["POSTGRES_PASSWORD"] = admin_password
	service_urls = local_stack_control.process_logins.setup_service_logins(
		disposable.target, runner, service_values, compose_environment
	)
	application_environment = _application_child_environment(service_urls[0])
	authoring_environment = dict(application_environment)
	authoring_environment["PLE_ACCEPTANCE_RUNTIME_MANIFEST"] = str(runtime.manifest_path)
	_require_command(
		runner,
		[
			"cargo", "test", "--manifest-path", str(repository_root / "Cargo.toml"),
			"-p", "learning-data-access", "--features", "postgres",
			"--test", "authoring_draft_source_postgres",
			"webwork_draft_creation_keeps_the_initial_source_binding_on_confirmation",
			"--", "--ignored", "--exact", "--test-threads=1",
		],
		authoring_environment,
		workspace,
		"authoring Draft Question source-binding acceptance",
		private_values + (admin_password, migrator_password, service_urls[0]),
	)
	verification_environment = dict(application_environment)
	tool_argv = [
		"cargo", "run", "--manifest-path", str(repository_root / "Cargo.toml"), "--quiet",
		"-p", "project-tools", "--", "database",
	]
	_require_command(
		runner, [*tool_argv, "verify"], verification_environment, workspace,
		"application database verification", private_values + (admin_password, migrator_password, service_urls[0]),
	)
	migrator_environment = dict(compose_environment)
	migrator_environment["PGPASSWORD"] = migrator_password
	security_argv, _ = local_stack_control.disposable_stack_adapter.compose_command(
		disposable,
		["exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1", "-U", "ple_migrator", "-d", "ple_e2e_baseline"],
	)
	security_sql = (repository_root / "tests/e2e/database_baseline_security_catalog.sql").read_text(encoding="utf-8")
	_require_command(
		runner, security_argv, migrator_environment, repository_root, "database baseline security catalog",
		private_values + (admin_password, migrator_password, service_urls[0]), security_sql,
	)
	owned_snapshot = local_stack_control.disposable_stack_adapter.require_current_resource_capability(
		runner, disposable
	)
	postgres_container = _postgres_container_id(disposable, owned_snapshot)
	# Unrelease exercises the destructive Student Work closure against the same
	# initialized database. Its fixed oracle receives the owner-resolved opaque
	# PostgreSQL container identity; it cannot choose a database target.
	_require_command(
		runner,
		[
			"bash",
			str(repository_root / "tests/e2e/e2e_unrelease_connected.sh"),
			postgres_container,
		],
		application_environment,
		repository_root,
		"connected Unrelease acceptance",
		private_values + (admin_password, migrator_password, service_urls[0]),
	)


#============================================
def run_owned_database_baseline(
	repository_root: pathlib.Path,
	oracle_runner: Callable[[pathlib.Path, pathlib.Path, int], None] = _run_oracle,
	acquire_browser_suite_lease: Callable[[pathlib.Path], local_stack_control.browser_suite_lease.BrowserSuiteLease] = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire,
	create_command_runner: Callable[[], local_stack_control.process.CommandRunner] = local_stack_control.process.SubprocessRunner,
	port_selector: Callable[[], int] = _select_loopback_port,
	port_checker: Callable[[tuple[int, ...], local_stack_control.process.CommandRunner, pathlib.Path], None] = local_stack_control.process.require_available_loopback_ports,
) -> None:
	"""Run one serial canonical PostgreSQL baseline oracle."""
	def profile_oracle(
		root: pathlib.Path, workspace: pathlib.Path, ports: tuple[int, ...]
	) -> None:
		oracle_runner(root, workspace, ports[0])

	local_stack_control.acceptance_profile_owner.run_owned_acceptance_profile(
		repository_root,
		"canonical PostgreSQL baseline",
		profile_oracle,
		acquire_browser_suite_lease,
		create_command_runner,
		lambda: (port_selector(),),
		port_checker,
	)


#============================================
def main() -> None:
	"""Run the canonical PostgreSQL baseline lifecycle entry point."""
	repository_root = local_stack_control.compose.repo_root_from_entrypoint(
		pathlib.Path(__file__)
	)
	run_owned_database_baseline(repository_root)
	print("database baseline E2E: PASS")


if __name__ == "__main__":
	main()
