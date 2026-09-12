"""Lease-owned PostgreSQL and MinIO course-appearance acceptance lifecycle."""

from __future__ import annotations

import os
import pathlib
import socket
import subprocess
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
import local_stack_control.process
import local_stack_control.process_logins
import local_stack_control.runtime_manifest


#============================================
def _select_loopback_port() -> int:
	"""Ask the kernel for one loopback port while the suite lease is held."""
	with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
		listener.bind(("127.0.0.1", 0))
		result = int(listener.getsockname()[1])
	return result


#============================================
def _select_loopback_ports() -> tuple[int, int]:
	"""Select distinct PostgreSQL and MinIO loopback ports for one private runtime."""
	postgres_port = _select_loopback_port()
	minio_port = _select_loopback_port()
	if postgres_port == minio_port:
		raise local_stack_control.models.ControllerError("course appearance cross-store ports are invalid")
	result = postgres_port, minio_port
	return result


#============================================
def _password_from_private_url(url_path: pathlib.Path, expected_login: str) -> str:
	"""Read one validated private PostgreSQL password for its owning child."""
	# ASVS 1.2.5 and 2.2.1: a closed locator supplies a password only to psql
	# stdin or a child environment, never to a command argument or diagnostic.
	content = local_stack_control.private_files.read_current_user_private_file(url_path, 4_096)
	try:
		parsed = urllib.parse.urlsplit(content.decode("ascii").strip())
	except UnicodeDecodeError as error:
		raise local_stack_control.models.ControllerError("cross-store credential is invalid") from error
	if parsed.username != expected_login or parsed.password is None:
		raise local_stack_control.models.ControllerError("cross-store credential is invalid")
	return parsed.password


#============================================
def _network_migrator_url(migrator_password: str) -> str:
	"""Construct the owner-generated private network URL for Compose interpolation."""
	return local_stack_control.lifecycle_database.database_url(
		{
			"POSTGRES_USER": local_stack_control.runtime_manifest.POSTGRES_MIGRATOR_ROLE,
			"POSTGRES_PASSWORD": migrator_password,
			"POSTGRES_DB": local_stack_control.runtime_manifest.DATABASE_NAME,
		}
	)


#============================================
def _compose_environment_with_migrator(
	disposable: local_stack_control.models.DisposableComposeTarget,
	migration_url: str,
) -> dict[str, str]:
	"""Keep the migrator URL in the child Compose environment, never its env file."""
	if not migration_url.startswith("postgres://") or any(
		character in migration_url for character in "\r\n\x00"
	):
		raise local_stack_control.models.ControllerError("course appearance migrator URL is invalid")
	environment = local_stack_control.disposable_stack_adapter.compose_environment(disposable)
	environment[local_stack_control.lifecycle_migrations.MIGRATION_URL_SETTING] = migration_url
	return environment


#============================================
def _e2e_child_environment(
	application_environment: dict[str, str],
	manifest_path: pathlib.Path,
) -> dict[str, str]:
	"""Give the cross-store child its application role and runtime locator."""
	environment = {
		name: value
		for name, value in application_environment.items()
		if not name.startswith("PLE_") and not name.startswith("COMPOSE_")
	}
	environment["PLE_ACCEPTANCE_RUNTIME_MANIFEST"] = str(manifest_path)
	return environment


#============================================
def _run_oracle(repository_root: pathlib.Path, workspace: pathlib.Path, ports: tuple[int, ...]) -> None:
	"""Prepare the ordinary database boundary before the focused cross-store checks."""
	if len(ports) != 2:
		raise local_stack_control.models.ControllerError("course appearance cross-store ports are invalid")
	runtime = local_stack_control.runtime_manifest.write_course_appearance_cross_store_runtime(
		workspace, ports[0], ports[1]
	)
	runner = local_stack_control.process.SubprocessRunner()
	manifest = local_stack_control.disposable_stack_adapter.load_manifest(
		repository_root, runtime.manifest_path
	)
	disposable = local_stack_control.disposable_stack_adapter.disposable_target(
		runner, repository_root, manifest
	)
	private_values = local_stack_control.disposable_stack_adapter.private_environment_values(
		disposable.target.env_file
	)
	admin_password = _password_from_private_url(runtime.admin_url_path, "ple_e2e_migrator")
	migrator_password = _password_from_private_url(runtime.migrator_url_path, "ple_migrator")
	migration_url = _network_migrator_url(migrator_password)
	compose_environment = _compose_environment_with_migrator(disposable, migration_url)
	private_values = private_values + (migration_url,)

	def require(result: local_stack_control.models.CommandResult, description: str, secrets: tuple[str, ...]) -> str:
		local_stack_control.lifecycle_commands.require_command(result, description, secrets)
		return result.stdout

	def compose_command(arguments: list[str]) -> tuple[list[str], dict[str, str]]:
		argv, environment = local_stack_control.disposable_stack_adapter.compose_command(
			disposable, arguments
		)
		environment[local_stack_control.lifecycle_migrations.MIGRATION_URL_SETTING] = migration_url
		return argv, environment

	def compose(arguments: list[str], description: str, stdin: str | None = None) -> str:
		argv, environment = compose_command(arguments)
		return require(runner.run(argv, environment, repository_root, stdin), description, private_values)

	compose(["up", "-d", "postgres", "minio"], "course appearance service startup")
	for _ in range(30):
		argv, environment = compose_command(
			["exec", "-T", "postgres", "pg_isready", "-U", "ple_e2e_migrator", "-d", "postgres"],
		)
		if runner.run(argv, environment, repository_root).ok():
			break
		time.sleep(1)
	else:
		raise local_stack_control.models.ControllerError("course appearance PostgreSQL did not become ready")

	admin_environment = dict(compose_environment)
	admin_environment["PGPASSWORD"] = admin_password
	create_database_argv, _ = compose_command([
		"exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1",
		"-U", "ple_e2e_migrator", "-d", "postgres",
	])
	require(
		runner.run(create_database_argv, admin_environment, repository_root, "CREATE DATABASE ple_e2e_baseline;\n"),
		"course appearance database creation",
		private_values + (admin_password, migrator_password),
	)
	bootstrap_argv, _ = compose_command([
		"exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1",
		"-U", "ple_e2e_migrator", "-d", "ple_e2e_baseline",
	])
	require(
		runner.run(
			bootstrap_argv,
			admin_environment,
			repository_root,
			local_stack_control.lifecycle_database.migration_principal_bootstrap_sql(
				"ple_e2e_baseline", migrator_password
			),
		),
		"course appearance principal bootstrap",
		private_values + (admin_password, migrator_password),
	)
	child_environment = {
		name: value
		for name, value in os.environ.items()
		if not name.startswith("PLE_") and not name.startswith("COMPOSE_")
	}
	child_environment["PLE_ACCEPTANCE_RUNTIME_MANIFEST"] = str(runtime.manifest_path)
	compose(
		["--profile", "migration", "build", "database-migrator"],
		"course appearance database migrator build",
	)
	compose(
		[
			"--profile", "migration", "run", "--rm", "--no-deps",
			"database-migrator", "database", "initialize",
		],
		"course appearance database initialize",
	)
	replay = compose(
		[
			"--profile", "migration", "run", "--rm", "--no-deps",
			"database-migrator", "database", "initialize",
		],
		"course appearance database initialize replay",
	)
	if replay.strip() != "database initialize: complete and compatible":
		raise local_stack_control.models.ControllerError("course appearance database initialize replay did not converge")
	service_values = local_stack_control.env_file.env_settings(disposable.target.env_file)
	service_values["POSTGRES_DB"] = "ple_e2e_baseline"
	service_values["POSTGRES_PASSWORD"] = admin_password
	service_urls = local_stack_control.process_logins.setup_service_logins(
		disposable.target, runner, service_values, compose_environment
	)
	application_environment = dict(child_environment)
	application_environment["DATABASE_URL"] = service_urls[0]
	require(
		runner.run(
			[
				"cargo", "run", "--manifest-path", str(repository_root / "Cargo.toml"), "--quiet",
				"-p", "project-tools", "--", "database", "verify",
			],
			application_environment,
			workspace,
		),
		"course appearance application database verification",
		private_values + (admin_password, migrator_password, service_urls[0]),
	)
	environment = _e2e_child_environment(application_environment, runtime.manifest_path)
	result = subprocess.run(
		[
			"bash",
			str(repository_root / "tests/e2e/e2e_course_appearance.sh"),
			"--owned-child",
			"--runtime-manifest",
			local_stack_control.runtime_manifest.MANIFEST_NAME,
		],
		cwd=workspace,
		env=environment,
		check=False,
	)
	if result.returncode != 0:
		raise local_stack_control.models.ControllerError("course appearance cross-store oracle failed")


#============================================
def run_owned_course_appearance_cross_store(
	repository_root: pathlib.Path,
	oracle_runner: Callable[[pathlib.Path, pathlib.Path, tuple[int, ...]], None] = _run_oracle,
	acquire_browser_suite_lease: Callable[[pathlib.Path], local_stack_control.browser_suite_lease.BrowserSuiteLease] = local_stack_control.browser_suite_lease.BrowserSuiteLease.acquire,
	create_command_runner: Callable[[], local_stack_control.process.CommandRunner] = local_stack_control.process.SubprocessRunner,
	ports_selector: Callable[[], tuple[int, int]] = _select_loopback_ports,
	port_checker: Callable[[tuple[int, ...], local_stack_control.process.CommandRunner, pathlib.Path], None] = local_stack_control.process.require_available_loopback_ports,
) -> None:
	"""Run the fixed cross-store oracle with one serial lease and two final resets."""
	local_stack_control.acceptance_profile_owner.run_owned_acceptance_profile(
		repository_root,
		"course appearance cross-store",
		oracle_runner,
		acquire_browser_suite_lease,
		create_command_runner,
		ports_selector,
		port_checker,
	)


#============================================
def main() -> None:
	"""Run the public cross-store profile entry point."""
	repository_root = local_stack_control.compose.repo_root_from_entrypoint(pathlib.Path(__file__))
	run_owned_course_appearance_cross_store(repository_root)
	print("Course-appearance cross-store: PASS")


if __name__ == "__main__":
	main()
