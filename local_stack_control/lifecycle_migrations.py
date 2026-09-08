"""Network-scoped database migration jobs for the local stack."""

import pathlib
import secrets

import local_stack_control.compose
import local_stack_control.disposable_stack_adapter
import local_stack_control.env_file
import local_stack_control.lifecycle_commands
import local_stack_control.lifecycle_database
import local_stack_control.live_demo_gateway
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process


MIGRATION_SERVICE = "database-migrator"
MIGRATION_URL_SETTING = "PLE_MIGRATION_DATABASE_URL"


#============================================
def run_migrations(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	values: dict[str, str],
	environment: dict[str, str],
) -> None:
	"""Run the selected schema migration inside the Compose data network."""
	del repo_root, environment
	selected = target_of(target)
	url = migration_database_url_for(target, runner, values)
	write_migration_database_url(selected.env_file, url)
	build_database_migrator(selected, runner)
	operation = "migrate-schema" if local_stack_control.live_demo_gateway.is_tls_target(selected) else "migrate"
	run_database_command(selected, runner, operation, "database migration")


#============================================
def verify_migrated_application_schema(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Verify the browser profile through its API login on the Compose network."""
	settings = local_stack_control.env_file.env_settings(target.env_file)
	try:
		url = settings["PLE_API_DATABASE_URL"]
	except KeyError as error:
		raise local_stack_control.models.ControllerError(
			"live-demo application-schema verification credentials are unavailable"
		) from error
	write_migration_database_url(target.env_file, url)
	run_database_command(target, runner, "verify", "application-schema verification")


#============================================
def run_database_command(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	operation: str,
	description: str,
) -> None:
	"""Run one database administration action without using a host-published port."""
	result = runner.run(
		local_stack_control.compose.compose_argv(
			target,
			[
				"--profile", "migration", "run", "--rm", "--no-deps",
				MIGRATION_SERVICE, "database", operation,
			],
		),
		local_stack_control.lifecycle_commands.child_environment(target),
		target.repo_root,
	)
	private_values = local_stack_control.disposable_stack_adapter.private_environment_values(
		target.env_file
	)
	local_stack_control.lifecycle_commands.require_command(result, description, private_values)


#============================================
def build_database_migrator(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Build the one-shot migration image before its first network-scoped use."""
	result = runner.run(
		local_stack_control.compose.compose_argv(
			target, ["--profile", "migration", "build", MIGRATION_SERVICE]
		),
		local_stack_control.lifecycle_commands.child_environment(target),
		target.repo_root,
	)
	private_values = local_stack_control.disposable_stack_adapter.private_environment_values(
		target.env_file
	)
	local_stack_control.lifecycle_commands.require_command(
		result, "database migrator build", private_values
	)


#============================================
def migration_database_url_for(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	values: dict[str, str],
) -> str:
	"""Create the fresh-demo migration principal and return its private network URL."""
	selected = target_of(target)
	if not local_stack_control.live_demo_gateway.is_tls_target(selected):
		return database_url(values)

	migrator_password = secrets.token_hex(32)
	bootstrap_environment = local_stack_control.lifecycle_commands.child_environment(selected)
	bootstrap_environment["PGPASSWORD"] = values["POSTGRES_PASSWORD"]
	bootstrap_argv = local_stack_control.compose.compose_argv(
		selected,
		[
			"exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1",
			"-U", values["POSTGRES_USER"], "-d", values["POSTGRES_DB"],
		],
	)
	local_stack_control.lifecycle_commands.require_command(
		runner.run(
			bootstrap_argv,
			bootstrap_environment,
			selected.repo_root,
			migration_principal_bootstrap_sql(values["POSTGRES_DB"], migrator_password),
		),
		"live-demo migration-principal bootstrap",
		(values["POSTGRES_PASSWORD"], migrator_password),
	)
	migrator_values = dict(values)
	migrator_values["POSTGRES_USER"] = MIGRATION_ROLE
	migrator_values["POSTGRES_PASSWORD"] = migrator_password
	return database_url(migrator_values)


#============================================
def write_migration_database_url(env_file: pathlib.Path, url: str) -> None:
	"""Replace the private URL consumed by the migration job."""
	if not url.startswith("postgres://") or any(character in url for character in "\r\n\x00"):
		raise local_stack_control.models.ControllerError("migration database URL is invalid")
	local_stack_control.env_file.require_mutation_env_file(env_file)
	settings = local_stack_control.env_file.env_settings(env_file)
	settings[MIGRATION_URL_SETTING] = url
	content = "".join(f"{name}={value}\n" for name, value in settings.items()).encode("utf-8")
	local_stack_control.private_files.write_atomic_file(env_file, content, 0o600)


#============================================
def target_of(
	target: local_stack_control.models.ComposeTarget | local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.ComposeTarget:
	"""Return the shared Compose target from a lifecycle owner."""
	if isinstance(target, local_stack_control.models.DisposableComposeTarget):
		return target.target
	return target


#============================================
def migration_principal_bootstrap_sql(database_name: str, migrator_password: str) -> str:
	"""Build the closed fresh-database principal baseline."""
	return local_stack_control.lifecycle_database.migration_principal_bootstrap_sql(
		database_name, migrator_password
	)


#============================================
def postgres_role_sql(role: str, password: str) -> str:
	"""Build one bounded local role-password update."""
	return local_stack_control.lifecycle_database.postgres_role_sql(role, password)


#============================================
def database_url(values: dict[str, str]) -> str:
	"""Construct the private Compose-network PostgreSQL URL for one migration job."""
	return local_stack_control.lifecycle_database.database_url(values)


MIGRATION_DATABASE_OWNER = local_stack_control.lifecycle_database.MIGRATION_DATABASE_OWNER
MIGRATION_ROLE = local_stack_control.lifecycle_database.MIGRATION_ROLE
